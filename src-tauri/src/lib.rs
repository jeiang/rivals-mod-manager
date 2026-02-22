use std::{
    collections::{BTreeSet, HashSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

type AppState = Mutex<AppData>;

const HEROES_API_URL: &str = "https://marvelrivalsapi.com/api/v2/heroes";

fn default_input_mods_folder() -> String {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("mods")
        .to_string_lossy()
        .to_string()
}

struct AppData {
    db: rusqlite::Connection,
}

#[derive(Serialize)]
struct ModEntry {
    id: i64,
    name: String,
    author: String,
    path: String,
    category: String,
    files: Vec<ModFileEntry>,
    last_modified: Option<i64>,
}

#[derive(Serialize)]
struct ModFileEntry {
    id: i64,
    filename: String,
    has_signatures: bool,
    is_enabled: bool,
}

#[derive(Deserialize)]
struct HeroApiEntry {
    name: String,
}

fn format_category_name(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut capitalize_next = true;

    for ch in raw.chars() {
        if ch == ' ' || ch == '-' {
            capitalize_next = true;
            result.push(ch);
            continue;
        }

        if capitalize_next {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let mut stmt = state
        .db
        .prepare("SELECT category FROM categories ORDER BY category ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row.map_err(|e| e.to_string())?);
    }

    Ok(categories)
}

#[tauri::command]
fn get_setting(state: State<'_, AppState>, name: String) -> Result<Option<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    query_setting(&state.db, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(state: State<'_, AppState>, name: String, value: Option<String>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    write_setting(&state.db, &name, value.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_mods(state: State<'_, AppState>) -> Result<Vec<ModEntry>, String> {
    let (mods_folder, db_rows) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let mods_folder = resolve_mods_folder(&state.db).map_err(|e| e.to_string())?;
        let mut stmt = state
            .db
            .prepare("SELECT id, name, author, path, category FROM mods ORDER BY name ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut db_rows = Vec::new();
        for row in rows {
            db_rows.push(row.map_err(|e| e.to_string())?);
        }
        (mods_folder, db_rows)
    };

    let mut mods = Vec::with_capacity(db_rows.len());
    let state = state.lock().map_err(|e| e.to_string())?;
    let mut files_stmt = state
        .db
        .prepare(
            "SELECT id, filename, has_signatures, is_enabled FROM mod_files WHERE mod_id = ?1 ORDER BY filename ASC",
        )
        .map_err(|e| e.to_string())?;

    for (id, name, author, path, category) in db_rows {
        let file_rows = files_stmt
            .query_map([id], |row| {
                Ok(ModFileEntry {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    has_signatures: row.get(2)?,
                    is_enabled: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for file_row in file_rows {
            files.push(file_row.map_err(|e| e.to_string())?);
        }

        mods.push(ModEntry {
            id,
            name,
            author,
            files,
            path: make_relative_to_mods_folder(&path, &mods_folder),
            category,
            last_modified: find_latest_modified_unix(&path),
        });
    }

    Ok(mods)
}

#[tauri::command]
fn refresh_mods(state: State<'_, AppState>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let mods_folder = resolve_mods_folder(&state.db).map_err(|e| e.to_string())?;
    let discovered = discover_mods(&mods_folder)?;

    let tx = state.db.unchecked_transaction().map_err(|e| e.to_string())?;
    {
        let mut upsert = tx
            .prepare(
                r#"
                INSERT INTO mods (name, author, path, nexus_mod_id, category)
                VALUES (?1, ?2, ?3, ?4, 'Uncategorized')
                ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name,
                    author = excluded.author,
                    nexus_mod_id = excluded.nexus_mod_id
                "#,
            )
            .map_err(|e| e.to_string())?;
        let mut select_mod_id = tx
            .prepare("SELECT id FROM mods WHERE path = ?1 LIMIT 1")
            .map_err(|e| e.to_string())?;
        let mut upsert_file = tx
            .prepare(
                r#"
                INSERT INTO mod_files (mod_id, filename, has_signatures, is_enabled)
                VALUES (?1, ?2, ?3, 1)
                ON CONFLICT(mod_id, filename) DO UPDATE SET
                    has_signatures = excluded.has_signatures
                "#,
            )
            .map_err(|e| e.to_string())?;
        let mut delete_all_files = tx
            .prepare("DELETE FROM mod_files WHERE mod_id = ?1")
            .map_err(|e| e.to_string())?;

        for discovered_mod in discovered {
            let metadata = parse_mod_metadata(&discovered_mod.name);
            let path = discovered_mod.path;
            upsert
                .execute(params![metadata.name, metadata.author, path, metadata.nexus_mod_id])
                .map_err(|e| e.to_string())?;

            let mod_id: i64 = select_mod_id
                .query_row([path], |row| row.get(0))
                .map_err(|e| e.to_string())?;

            if discovered_mod.files.is_empty() {
                delete_all_files
                    .execute([mod_id])
                    .map_err(|e| e.to_string())?;
                continue;
            }

            for file in &discovered_mod.files {
                upsert_file
                    .execute(params![mod_id, file.filename, file.has_signatures])
                    .map_err(|e| e.to_string())?;
            }

            let placeholders = (1..=discovered_mod.files.len())
                .map(|idx| format!("?{}", idx + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let delete_sql = format!(
                "DELETE FROM mod_files WHERE mod_id = ?1 AND filename NOT IN ({placeholders})"
            );
            let mut delete_stmt = tx.prepare(&delete_sql).map_err(|e| e.to_string())?;
            let mut values = Vec::with_capacity(discovered_mod.files.len() + 1);
            values.push(rusqlite::types::Value::Integer(mod_id));
            for file in &discovered_mod.files {
                values.push(rusqlite::types::Value::Text(file.filename.clone()));
            }
            delete_stmt
                .execute(rusqlite::params_from_iter(values))
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn set_mod_file_enabled(state: State<'_, AppState>, file_id: i64, is_enabled: bool) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "UPDATE mod_files SET is_enabled = ?2 WHERE id = ?1",
            params![file_id, is_enabled],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mod_enabled(state: State<'_, AppState>, mod_id: i64, is_enabled: bool) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "UPDATE mod_files SET is_enabled = ?2 WHERE mod_id = ?1",
            params![mod_id, is_enabled],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn query_setting(conn: &Connection, name: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE name = ?1 LIMIT 1",
        [name],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .map(|value| value.flatten())
}

fn resolve_mods_folder(conn: &Connection) -> rusqlite::Result<String> {
    query_setting(conn, "paths.mods")?
        .or_else(|| query_setting(conn, "paths.mods_folder").ok().flatten())
        .filter(|path| !path.trim().is_empty())
        .map(|path| path.trim().to_string())
        .map(Ok)
        .unwrap_or_else(|| Ok(default_input_mods_folder()))
}

struct DiscoveredMod {
    name: String,
    path: String,
    files: Vec<DiscoveredModFile>,
}

struct DiscoveredModFile {
    filename: String,
    has_signatures: bool,
}

struct ParsedModMetadata {
    name: String,
    author: String,
    nexus_mod_id: Option<i64>,
}

fn parse_mod_metadata(raw_name: &str) -> ParsedModMetadata {
    let parts = raw_name
        .split(" - ")
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.len() < 2 {
        return ParsedModMetadata {
            name: raw_name.to_string(),
            author: "Unknown".to_string(),
            nexus_mod_id: None,
        };
    }

    let author = if parts[0].is_empty() {
        "Unknown".to_string()
    } else {
        parts[0].to_string()
    };

    if parts.len() >= 3 {
        let maybe_id = parts.last().and_then(|value| value.parse::<i64>().ok());
        if let Some(nexus_mod_id) = maybe_id {
            return ParsedModMetadata {
                name: parts[1..parts.len() - 1].join(" - "),
                author,
                nexus_mod_id: Some(nexus_mod_id),
            };
        }
    }

    ParsedModMetadata {
        name: parts[1..].join(" - "),
        author,
        nexus_mod_id: None,
    }
}

fn discover_mods(mods_folder: &str) -> Result<Vec<DiscoveredMod>, String> {
    let root = Path::new(mods_folder);
    if !root.exists() {
        return Err(format!("Mods folder does not exist: {mods_folder}"));
    }
    if !root.is_dir() {
        return Err(format!("Mods folder is not a directory: {mods_folder}"));
    }
    let entries = fs::read_dir(root)
        .map_err(|e| format!("Failed to read mods folder `{mods_folder}`: {e}"))?;

    let mut mods = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let signature_stems = find_signature_stems(&path);
            let files = find_mod_files(&path)
                .into_iter()
                .filter_map(|file_path| {
                    let file = PathBuf::from(file_path);
                    file.strip_prefix(&path).ok().map(|relative| DiscoveredModFile {
                        filename: relative.to_string_lossy().to_string(),
                        has_signatures: file
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .map(|stem| signature_stems.contains(&stem.to_ascii_lowercase()))
                            .unwrap_or(false),
                    })
                })
                .collect::<Vec<_>>();
            mods.push(DiscoveredMod {
                name,
                path: path.to_string_lossy().to_string(),
                files,
            });
        } else if metadata.is_file() && has_mod_extension(&path) {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Unnamed Mod")
                .to_string();
            mods.push(DiscoveredMod {
                name,
                path: path.to_string_lossy().to_string(),
                files: vec![DiscoveredModFile {
                    filename: path
                        .file_name()
                        .and_then(|filename| filename.to_str())
                        .unwrap_or_default()
                        .to_string(),
                    has_signatures: has_matching_signature_files(&path),
                }],
            });
        }
    }

    Ok(mods)
}

fn write_setting(conn: &Connection, name: &str, value: Option<&str>) -> rusqlite::Result<()> {
    let updated = conn.execute(
        "UPDATE settings SET value = ?2 WHERE name = ?1",
        params![name, value],
    )?;

    if updated == 0 {
        conn.execute(
            "INSERT INTO settings (name, value) VALUES (?1, ?2)",
            params![name, value],
        )?;
    }

    Ok(())
}

fn make_relative_to_mods_folder(path: &str, mods_folder: &str) -> String {
    let base = Path::new(mods_folder);
    let value = Path::new(path);

    value
        .strip_prefix(base)
        .ok()
        .and_then(|relative| {
            if relative.as_os_str().is_empty() {
                Some(".".to_string())
            } else {
                Some(relative.to_string_lossy().to_string())
            }
        })
        .unwrap_or_else(|| path.to_string())
}

fn find_mod_files(path: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect_mod_files(path, &mut files);
    files.sort_unstable();
    files
}

fn has_matching_signature_files(file_path: &Path) -> bool {
    let mut ucas = file_path.to_path_buf();
    ucas.set_extension("ucas");

    let mut utoc = file_path.to_path_buf();
    utoc.set_extension("utoc");

    ucas.exists() || utoc.exists()
}

fn find_signature_stems(path: &Path) -> HashSet<String> {
    let mut stems = HashSet::new();
    collect_signature_stems(path, &mut stems);
    stems
}

fn collect_signature_stems(path: &Path, stems: &mut HashSet<String>) {
    let Ok(metadata) = path.metadata() else {
        return;
    };

    if metadata.is_file() {
        if is_signature_extension(path) {
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                stems.insert(stem.to_ascii_lowercase());
            }
        }
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_signature_stems(&entry.path(), stems);
    }
}

fn find_latest_modified_unix(path: &str) -> Option<i64> {
    find_latest_modified_path(Path::new(path))
}

fn find_latest_modified_path(path: &Path) -> Option<i64> {
    let Ok(metadata) = path.metadata() else {
        return None;
    };

    let mut latest = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64);

    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return latest;
        };

        for entry in entries.flatten() {
            if let Some(candidate) = find_latest_modified_path(&entry.path()) {
                latest = Some(latest.map_or(candidate, |current| current.max(candidate)));
            }
        }
    }

    latest
}

fn collect_mod_files(path: &Path, files: &mut Vec<String>) {
    let Ok(metadata) = path.metadata() else {
        return;
    };

    if metadata.is_file() {
        if has_mod_extension(path) {
            files.push(path.to_string_lossy().to_string());
        }
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_mod_files(&entry.path(), files);
    }
}

fn has_mod_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "pak"
        })
        .unwrap_or(false)
}

fn is_signature_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "ucas" || ext == "utoc"
        })
        .unwrap_or(false)
}

#[tauri::command]
async fn refresh_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let token = {
        let state = state.lock().map_err(|e| e.to_string())?;
        query_setting(&state.db, "tokens.marvelrivalsapi")
            .map_err(|e| e.to_string())?
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing MarvelRivalsAPI token in settings".to_string())?
    };

    let heroes = reqwest::Client::new()
        .get(HEROES_API_URL)
        .header("x-api-key", token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Vec<HeroApiEntry>>()
        .await
        .map_err(|e| e.to_string())?;

    let mut unique_names = BTreeSet::new();
    for hero in heroes {
        let name = hero.name.trim();
        if !name.is_empty() {
            unique_names.insert(format_category_name(name));
        }
    }
    unique_names.insert("Uncategorized".to_string());
    let categories: Vec<String> = unique_names.iter().cloned().collect();

    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    {
        let placeholders = (1..=categories.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");

        tx.execute(
            r#"INSERT OR IGNORE INTO categories (category) VALUES ('Uncategorized')"#,
            [],
        )
        .map_err(|e| e.to_string())?;

        let remap_mods_sql = format!(
            "UPDATE mods SET category = 'Uncategorized' WHERE category NOT IN ({placeholders})"
        );
        tx.execute(&remap_mods_sql, params_from_iter(categories.iter()))
            .map_err(|e| e.to_string())?;

        let remove_old_sql =
            format!("DELETE FROM categories WHERE category NOT IN ({placeholders})");
        tx.execute(&remove_old_sql, params_from_iter(categories.iter()))
            .map_err(|e| e.to_string())?;

        let mut insert_category = tx
            .prepare(r#"INSERT OR IGNORE INTO categories (category) VALUES (?1)"#)
            .map_err(|e| e.to_string())?;
        for category in &categories {
            insert_category
                .execute([category])
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(categories)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    fn runner() -> Result<(), Box<dyn Error>> {
        let mut conn = Connection::open("./data.db")?;
        init_db(&mut conn)?;
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![
                greet,
                get_categories,
                get_setting,
                set_setting,
                get_mods,
                refresh_mods,
                set_mod_file_enabled,
                set_mod_enabled,
                refresh_categories
            ])
            .setup(|app| {
                app.manage(Mutex::new(AppData { db: conn }));
                Ok(())
            })
            .run(tauri::generate_context!())?;
        Ok(())
    }
    runner().expect("error while running tauri application");
}

fn init_db(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value TEXT
        );

        CREATE TABLE IF NOT EXISTS mods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT 'Unknown',
            path TEXT NOT NULL,
            nexus_mod_id INTEGER,
            category TEXT NOT NULL,
            FOREIGN KEY (category) REFERENCES categories(category)
        );

        CREATE TABLE IF NOT EXISTS mod_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mod_id INTEGER NOT NULL,
            filename TEXT NOT NULL,
            has_signatures INTEGER NOT NULL,
            is_enabled INTEGER NOT NULL,
            FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
        );
        "#,
    )?;

    conn.execute("ALTER TABLE mods ADD COLUMN nexus_mod_id INTEGER", [])
        .or_else(|err| {
            if err.to_string().contains("duplicate column name") {
                Ok(0)
            } else {
                Err(err)
            }
        })?;
    conn.execute(
        "ALTER TABLE mods ADD COLUMN author TEXT NOT NULL DEFAULT 'Unknown'",
        [],
    )
    .or_else(|err| {
        if err.to_string().contains("duplicate column name") {
            Ok(0)
        } else {
            Err(err)
        }
    })?;

    conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_path_unique ON mods(path)", [])
        .map_err(|e| e)?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_mod_files_mod_filename_unique ON mod_files(mod_id, filename)",
        [],
    )
    .map_err(|e| e)?;

    let default_groups = ["Uncategorized"];

    let tx = conn.transaction()?;
    {
        let mut insert_group =
            tx.prepare(r#"INSERT OR IGNORE INTO categories (category) VALUES (?1)"#)?;
        for group in default_groups {
            insert_group.execute([group])?;
        }

        let path = default_input_mods_folder();
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, ?2
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["paths.mods", path],
        )?;
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, value
            FROM settings
            WHERE name = ?2
            AND NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["paths.mods", "paths.mods_folder"],
        )?;
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, NULL
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["tokens.nexusmods"],
        )?;
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, NULL
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["tokens.marvelrivalsapi"],
        )?;
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, NULL
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["paths.game"],
        )?;
        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, NULL
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["paths.downloads"],
        )?;
    }
    tx.commit()?;

    Ok(())
}
