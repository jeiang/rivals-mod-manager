use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_walkdir::WalkDir;
use chrono::{DateTime, Local};
use futures_util::StreamExt as _;
use lazy_regex::regex_captures;

use crate::categories::{CategoryMatcher, CategoryMatchers, match_category};
use crate::nexusmods::{CachedModInfo, fetch_mod_info};

#[derive(Clone)]
pub struct ModsRefreshResult {
    pub mods: Vec<ModInfo>,
    pub nexusmods_mod_cache: HashMap<u32, CachedModInfo>,
    pub toast_errors: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ModList {
    mods: Vec<ModInfo>,
}

impl ModList {
    pub fn new(mods: Vec<ModInfo>) -> Self {
        Self { mods }
    }

    pub fn set_mods(&mut self, mods: Vec<ModInfo>) {
        self.mods = mods;
    }

    pub fn mods(&self) -> &[ModInfo] {
        &self.mods
    }

    pub fn mods_mut(&mut self) -> &mut Vec<ModInfo> {
        &mut self.mods
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ModInfo {
    path: PathBuf,
    name: String,
    mod_id: Option<u32>,
    author: String,
    category: String,
    last_modified: String,
    enabled: bool,
    files: Vec<ModFileInfo>,
}

impl ModInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn set_author(&mut self, author: String) {
        self.author = author;
    }

    pub fn mod_id(&self) -> Option<u32> {
        self.mod_id
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn set_category(&mut self, category: String) {
        self.category = category;
    }

    pub fn last_modified(&self) -> &str {
        &self.last_modified
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ModFileInfo {
    subpath: PathBuf,
    enabled: bool,
}

pub async fn refresh_mod_list(
    dir: &Path,
    matchers: &[CategoryMatcher],
) -> io::Result<Vec<ModInfo>> {
    let mut list = vec![];
    let mut subdirs = tokio::fs::read_dir(dir).await?;
    while let Some(folder) = subdirs.next_entry().await? {
        if !folder.file_type().await.is_ok_and(|x| x.is_dir()) {
            continue;
        }
        let files = {
            let mut files = vec![];
            let mut entries = WalkDir::new(folder.path());
            loop {
                match entries.next().await {
                    None => break,
                    Some(Ok(entry)) => {
                        let Ok(file_type) = entry.file_type().await else {
                            continue;
                        };
                        let path = entry.path();
                        let extension_matches =
                            path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pak"));
                        if file_type.is_file() && extension_matches {
                            let Ok(subpath) = path.strip_prefix(folder.path()) else {
                                continue;
                            };
                            let subpath = subpath.to_path_buf();
                            files.push(ModFileInfo { subpath, enabled: true });
                        }
                    }
                    Some(Err(_)) => continue,
                }
            }
            files
        };

        let pathinfo = try_parse_name(&folder.path());
        let last_modified = get_last_modified(&folder.path())?;
        let category = match_category(matchers, &pathinfo.name);

        let modinfo = ModInfo {
            path: folder.path().to_path_buf(),
            name: pathinfo.name,
            mod_id: pathinfo.mod_id,
            author: pathinfo.author.unwrap_or("Unknown".to_string()),
            category,
            last_modified,
            enabled: true,
            files,
        };
        list.push(modinfo);
    }
    Ok(list)
}

pub async fn refresh_mods_with_nexusmods(
    input_folder: PathBuf,
    matchers: CategoryMatchers,
    api_key: String,
    mut nexusmods_mod_cache: HashMap<u32, CachedModInfo>,
) -> Result<ModsRefreshResult, String> {
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut mods =
        refresh_mod_list(&input_folder, &matchers).await.map_err(|err| err.to_string())?;
    let mut toast_errors = vec![];

    let api_key = api_key.trim();
    if api_key.is_empty() {
        toast_errors.push("NexusMods API key is not set.".to_string());
    }

    for mod_info in &mut mods {
        let Some(mod_id) = mod_info.mod_id() else {
            continue;
        };

        if !api_key.is_empty() {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                nexusmods_mod_cache.entry(mod_id)
            {
                match fetch_mod_info(&api_key, "marvelrivals", mod_id).await {
                    Ok(fetched_mod_info) => {
                        entry.insert(fetched_mod_info);
                    }
                    Err(err) => {
                        if matches!(
                            err.status(),
                            Some(
                                reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
                            )
                        ) {
                            toast_errors.push(
                                "NexusMods API key is invalid. Check the key in Settings."
                                    .to_string(),
                            );
                            break;
                        }

                        toast_errors.push(format!(
                            "NexusMods metadata was not found for {} ({mod_id}): {err}",
                            mod_info.name()
                        ));
                    }
                }
            }
        }

        if let Some(fetched_mod_info) = nexusmods_mod_cache.get(&mod_id) {
            mod_info.set_name(fetched_mod_info.name().to_string());
            mod_info.set_author(fetched_mod_info.author().to_string());
            if mod_info.category() == "Uncategorized" {
                mod_info.set_category(match_category(&matchers, fetched_mod_info.name()));
            }
        }
    }

    Ok(ModsRefreshResult { mods, nexusmods_mod_cache, toast_errors })
}

// pub fn merge_and_sort_mods(prev: Vec<ModInfo>, next: Vec<ModInfo>) {}

struct NameParseResult {
    name: String,
    author: Option<String>,
    mod_id: Option<u32>,
}

fn try_parse_name(path: &Path) -> NameParseResult {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    if let Some((_, author, name, id)) =
        regex_captures!(r#"^([^-]+)\s*-\s*(.*)\s*-\s*(\d+)$"#i, &name)
    {
        let author = author.trim().to_string();
        let name = name.trim().to_string();
        let mod_id = id.trim().parse::<u32>().unwrap();
        return NameParseResult { name, author: Some(author), mod_id: Some(mod_id) };
    } else if let Some((_, name, id)) = regex_captures!(r#"^(.*)\s*-\s*(\d+)$"#i, &name) {
        let name = name.trim().to_string();
        let mod_id = id.trim().parse::<u32>().unwrap();
        return NameParseResult { name, author: None, mod_id: Some(mod_id) };
    } else if let Some((_, author, name)) = regex_captures!(r#"^([^-]+)\s*-\s*(.*)$"#i, &name) {
        let author = author.trim().to_string();
        let name = name.trim().to_string();
        return NameParseResult { name, author: Some(author), mod_id: None };
    } else {
        NameParseResult { name, author: None, mod_id: None }
    }
}

fn get_last_modified(path: &Path) -> io::Result<String> {
    let metadata = std::fs::metadata(path)?;
    let datetime: DateTime<Local> = metadata.modified()?.into();
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}
