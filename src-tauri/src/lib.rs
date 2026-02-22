use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

use regex::RegexBuilder;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

type AppState = Mutex<AppData>;

const HEROES_API_URL: &str = "https://marvelrivalsapi.com/api/v2/heroes";
const NEXUS_API_BASE_URL: &str = "https://api.nexusmods.com/v1";
const NEXUS_GAME_DOMAIN_NAME: &str = "marvelrivals";

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
    nexus_mod_id: Option<i64>,
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
    #[serde(default)]
    real_name: String,
}

#[derive(Serialize)]
struct CategoryMatcher {
    id: i64,
    pattern: String,
    matcher_type: String,
    case_sensitive: bool,
}

#[derive(Serialize)]
struct CategoryWithMatchers {
    category: String,
    matchers: Vec<CategoryMatcher>,
}

#[derive(Serialize)]
struct CategoryManagementData {
    api_categories: Vec<CategoryWithMatchers>,
    custom_categories: Vec<CategoryWithMatchers>,
}

#[derive(Deserialize)]
struct CategoryMatcherInput {
    pattern: String,
    matcher_type: String,
    #[serde(default)]
    case_sensitive: Option<bool>,
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
fn get_category_management(state: State<'_, AppState>) -> Result<CategoryManagementData, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    load_category_management_data(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_custom_category(state: State<'_, AppState>, category: String) -> Result<(), String> {
    let category = category.trim();
    if category.is_empty() {
        return Err("Category cannot be empty".to_string());
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "INSERT OR IGNORE INTO categories (category, is_api) VALUES (?1, 0)",
            [category],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_category_matchers(
    state: State<'_, AppState>,
    category: String,
    matchers: Vec<CategoryMatcherInput>,
) -> Result<(), String> {
    let category = category.trim();
    if category.is_empty() {
        return Err("Category cannot be empty".to_string());
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM category_matchers WHERE category = ?1",
        [category],
    )
    .map_err(|e| e.to_string())?;

    {
        let mut insert_matcher = tx
            .prepare(
                r#"
                INSERT INTO category_matchers (category, pattern, matcher_type, case_sensitive)
                VALUES (?1, ?2, ?3, ?4)
                "#,
            )
            .map_err(|e| e.to_string())?;
        for matcher in matchers {
            let pattern = matcher.pattern.trim().to_string();
            if pattern.is_empty() {
                continue;
            }
            let matcher_type = normalize_matcher_type(&matcher.matcher_type)
                .ok_or_else(|| format!("Invalid matcher type: {}", matcher.matcher_type))?;
            let case_sensitive = matcher.case_sensitive.unwrap_or(false);
            insert_matcher
                .execute(params![category, pattern, matcher_type, case_sensitive])
                .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_setting(state: State<'_, AppState>, name: String) -> Result<Option<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    query_setting(&state.db, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(
    state: State<'_, AppState>,
    name: String,
    value: Option<String>,
) -> Result<(), String> {
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
            .prepare(
                r#"
                SELECT
                    id,
                    name,
                    author,
                    nexus_mod_id,
                    path,
                    CASE
                        WHEN category_is_manual = 1 THEN category
                        WHEN auto_category IS NOT NULL THEN auto_category
                        ELSE 'Uncategorized'
                    END AS effective_category
                FROM mods
                ORDER BY name ASC
                "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
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

    for (id, name, author, nexus_mod_id, path, category) in db_rows {
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
            nexus_mod_id,
            files,
            path: make_relative_to_mods_folder(&path, &mods_folder),
            category,
            last_modified: find_latest_modified_unix(&path),
        });
    }

    Ok(mods)
}

#[tauri::command]
async fn refresh_mods(state: State<'_, AppState>) -> Result<(), String> {
    let (mods_folder, nexus_token) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let mods_folder = resolve_mods_folder(&state.db).map_err(|e| e.to_string())?;
        let nexus_token = query_setting(&state.db, "tokens.nexusmods")
            .map_err(|e| e.to_string())?
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        (mods_folder, nexus_token)
    };
    let discovered = discover_mods(&mods_folder)?;

    let existing_mods = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let mut stmt = state
            .db
            .prepare(
                "SELECT id, path, nexus_mod_id, mod_id_changed, author, category_is_manual FROM mods",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row.map_err(|e| e.to_string())?);
        }
        values
    };
    let category_matchers = {
        let state = state.lock().map_err(|e| e.to_string())?;
        load_compiled_category_matchers(&state.db).map_err(|e| e.to_string())?
    };

    let client = reqwest::Client::new();
    let mut candidates = Vec::with_capacity(discovered.len());
    for discovered_mod in discovered {
        let metadata = parse_mod_metadata(&discovered_mod.name);
        let matching_existing = existing_mods
            .iter()
            .find(|(_, path, nexus_mod_id, _, _, _)| {
                (metadata.nexus_mod_id.is_some() && *nexus_mod_id == metadata.nexus_mod_id)
                    || *path == discovered_mod.path
            });
        let is_new = matching_existing.is_none();
        let existing_nexus_mod_id =
            matching_existing.and_then(|(_, _, nexus_mod_id, _, _, _)| *nexus_mod_id);
        let effective_nexus_mod_id = metadata.nexus_mod_id.or(existing_nexus_mod_id);
        let mut name = metadata.name;
        let mut author = metadata.author;
        let should_fetch_nexus = effective_nexus_mod_id
            .map(|mod_id| mod_id > 0)
            .unwrap_or(false)
            && matching_existing
                .map(|(_, _, _, mod_id_changed, _, _)| *mod_id_changed)
                .unwrap_or(true);
        let mut mod_id_changed = should_fetch_nexus;

        if should_fetch_nexus {
            if let (Some(token), Some(mod_id)) = (nexus_token.as_deref(), effective_nexus_mod_id) {
                if mod_id > 0 {
                    if let Ok(Some(api_details)) =
                        fetch_nexus_mod_details(&client, token, mod_id).await
                    {
                        name = api_details.name;
                        let existing_author = matching_existing
                            .map(|(_, _, _, _, existing_author, _)| existing_author.trim());
                        let uploader_matches_existing = api_details
                            .uploader_name
                            .as_deref()
                            .zip(existing_author)
                            .map(|(uploader, existing)| uploader.eq_ignore_ascii_case(existing))
                            .unwrap_or(false);

                        if !uploader_matches_existing {
                            author = api_details.author;
                        }
                    }
                    mod_id_changed = false;
                }
            }
        }
        let category_is_manual = matching_existing
            .map(|(_, _, _, _, _, is_manual)| *is_manual)
            .unwrap_or(false);
        let auto_category = if !is_new || category_is_manual {
            None
        } else {
            find_auto_category_for_mod(&name, &discovered_mod.path, &category_matchers)
        };

        candidates.push(RefreshModCandidate {
            discovered_mod,
            name,
            author,
            nexus_mod_id: effective_nexus_mod_id,
            mod_id_changed,
            auto_category,
            is_new,
        });
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    {
        let mut find_existing_mod = tx
            .prepare(
                r#"
                SELECT id
                FROM mods
                WHERE
                    (?1 IS NOT NULL AND nexus_mod_id = ?1)
                    OR path = ?2
                ORDER BY CASE WHEN (?1 IS NOT NULL AND nexus_mod_id = ?1) THEN 0 ELSE 1 END
                LIMIT 1
                "#,
            )
            .map_err(|e| e.to_string())?;
        let mut insert_mod = tx
            .prepare(
                r#"
                INSERT INTO mods (name, author, path, nexus_mod_id, mod_id_changed, category, auto_category, category_is_manual)
                VALUES (?1, ?2, ?3, ?4, ?5, 'Uncategorized', ?6, 0)
                "#,
            )
            .map_err(|e| e.to_string())?;
        let mut update_mod = tx
            .prepare(
                r#"
                UPDATE mods
                SET author = ?2, path = ?3, nexus_mod_id = ?4, mod_id_changed = ?5
                WHERE id = ?1
                "#,
            )
            .map_err(|e| e.to_string())?;
        let mut update_auto_category = tx
            .prepare(
                r#"
                UPDATE mods
                SET auto_category = ?2
                WHERE id = ?1 AND category_is_manual = 0
                "#,
            )
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
        let mut seen_mod_ids = Vec::new();

        for candidate in candidates {
            let path = candidate.discovered_mod.path.clone();

            let existing_mod_id = find_existing_mod
                .query_row(params![candidate.nexus_mod_id, path], |row| {
                    row.get::<_, i64>(0)
                })
                .optional()
                .map_err(|e| e.to_string())?;

            let mod_id = if let Some(existing_id) = existing_mod_id {
                update_mod
                    .execute(params![
                        existing_id,
                        candidate.author,
                        candidate.discovered_mod.path,
                        candidate.nexus_mod_id,
                        candidate.mod_id_changed
                    ])
                    .map_err(|e| e.to_string())?;
                existing_id
            } else {
                insert_mod
                    .execute(params![
                        candidate.name,
                        candidate.author,
                        candidate.discovered_mod.path,
                        candidate.nexus_mod_id,
                        candidate.mod_id_changed,
                        candidate.auto_category
                    ])
                    .map_err(|e| e.to_string())?;
                tx.last_insert_rowid()
            };
            if candidate.is_new {
                update_auto_category
                    .execute(params![mod_id, candidate.auto_category])
                    .map_err(|e| e.to_string())?;
            }
            seen_mod_ids.push(mod_id);

            if candidate.discovered_mod.files.is_empty() {
                delete_all_files
                    .execute([mod_id])
                    .map_err(|e| e.to_string())?;
                continue;
            }

            for file in &candidate.discovered_mod.files {
                upsert_file
                    .execute(params![mod_id, file.filename, file.has_signatures])
                    .map_err(|e| e.to_string())?;
            }

            let placeholders = (1..=candidate.discovered_mod.files.len())
                .map(|idx| format!("?{}", idx + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let delete_sql = format!(
                "DELETE FROM mod_files WHERE mod_id = ?1 AND filename NOT IN ({placeholders})"
            );
            let mut delete_stmt = tx.prepare(&delete_sql).map_err(|e| e.to_string())?;
            let mut values = Vec::with_capacity(candidate.discovered_mod.files.len() + 1);
            values.push(rusqlite::types::Value::Integer(mod_id));
            for file in &candidate.discovered_mod.files {
                values.push(rusqlite::types::Value::Text(file.filename.clone()));
            }
            delete_stmt
                .execute(rusqlite::params_from_iter(values))
                .map_err(|e| e.to_string())?;
        }

        if seen_mod_ids.is_empty() {
            tx.execute("DELETE FROM mods", [])
                .map_err(|e| e.to_string())?;
        } else {
            let placeholders = (1..=seen_mod_ids.len())
                .map(|idx| format!("?{idx}"))
                .collect::<Vec<_>>()
                .join(", ");
            let delete_missing_sql = format!("DELETE FROM mods WHERE id NOT IN ({placeholders})");
            tx.execute(
                &delete_missing_sql,
                rusqlite::params_from_iter(seen_mod_ids),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn set_mod_file_enabled(
    state: State<'_, AppState>,
    file_id: i64,
    is_enabled: bool,
) -> Result<(), String> {
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
fn set_mod_enabled(
    state: State<'_, AppState>,
    mod_id: i64,
    is_enabled: bool,
) -> Result<(), String> {
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

#[tauri::command]
fn set_mod_category(
    state: State<'_, AppState>,
    mod_id: i64,
    category: String,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "UPDATE mods SET category = ?2, category_is_manual = 1, auto_category = NULL WHERE id = ?1",
            params![mod_id, category],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mods_category(
    state: State<'_, AppState>,
    mod_ids: Vec<i64>,
    category: String,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    for mod_id in mod_ids {
        tx.execute(
            "UPDATE mods SET category = ?2, category_is_manual = 1, auto_category = NULL WHERE id = ?1",
            params![mod_id, category],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reset_mods_category_to_auto(
    state: State<'_, AppState>,
    mod_ids: Vec<i64>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let matchers = load_compiled_category_matchers(&state.db).map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;

    {
        let mut find_mod = tx
            .prepare("SELECT name, path FROM mods WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let mut update_mod = tx
            .prepare("UPDATE mods SET category_is_manual = 0, auto_category = ?2 WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        for mod_id in mod_ids {
            let row = find_mod
                .query_row([mod_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some((name, path)) = row {
                let auto_category = find_auto_category_for_mod(&name, &path, &matchers);
                update_mod
                    .execute(params![mod_id, auto_category])
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mod_name(state: State<'_, AppState>, mod_id: i64, name: String) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Mod name cannot be empty".to_string());
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "UPDATE mods SET name = ?2 WHERE id = ?1",
            params![mod_id, trimmed],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mod_author(state: State<'_, AppState>, mod_id: i64, author: String) -> Result<(), String> {
    let trimmed = author.trim();
    if trimmed.is_empty() {
        return Err("Author cannot be empty".to_string());
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    state
        .db
        .execute(
            "UPDATE mods SET author = ?2 WHERE id = ?1",
            params![mod_id, trimmed],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mods_author(
    state: State<'_, AppState>,
    mod_ids: Vec<i64>,
    author: String,
) -> Result<(), String> {
    let trimmed = author.trim();
    if trimmed.is_empty() {
        return Err("Author cannot be empty".to_string());
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    for mod_id in mod_ids {
        tx.execute(
            "UPDATE mods SET author = ?2 WHERE id = ?1",
            params![mod_id, trimmed],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_mod_nexus_id(
    state: State<'_, AppState>,
    mod_id: i64,
    nexus_mod_id: Option<i64>,
) -> Result<(), String> {
    if let Some(value) = nexus_mod_id {
        if value < 0 {
            return Err("Mod ID cannot be negative".to_string());
        }
    }

    let state = state.lock().map_err(|e| e.to_string())?;
    let current_value = state
        .db
        .query_row(
            "SELECT nexus_mod_id FROM mods WHERE id = ?1",
            [mod_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let changed = current_value != nexus_mod_id;
    state
        .db
        .execute(
            "UPDATE mods SET nexus_mod_id = ?2, mod_id_changed = ?3 WHERE id = ?1",
            params![mod_id, nexus_mod_id, changed],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn clear_mods_output(state: State<'_, AppState>) -> Result<(), String> {
    let output_folder = {
        let state = state.lock().map_err(|e| e.to_string())?;
        resolve_output_mods_folder(&state.db).map_err(|e| e.to_string())?
    };

    ensure_and_clear_output_folder(&output_folder).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn apply_mods(state: State<'_, AppState>) -> Result<(), String> {
    let (output_folder, enabled_files) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let output_folder = resolve_output_mods_folder(&state.db).map_err(|e| e.to_string())?;
        let mut stmt = state
            .db
            .prepare(
                r#"
                SELECT m.path, mf.filename, mf.has_signatures
                FROM mods m
                JOIN mod_files mf ON mf.mod_id = m.id
                WHERE mf.is_enabled = 1
                ORDER BY m.id ASC, mf.filename ASC
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for row in rows {
            files.push(row.map_err(|e| e.to_string())?);
        }
        (output_folder, files)
    };

    ensure_and_clear_output_folder(&output_folder).map_err(|e| e.to_string())?;

    for (mod_path, filename, has_signatures) in enabled_files {
        let source_pak = resolve_source_file_path(Path::new(&mod_path), &filename);
        if !source_pak.exists() {
            continue;
        }

        let destination_pak = output_folder.join(Path::new(&filename));
        if let Some(parent) = destination_pak.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        symlink_file(&source_pak, &destination_pak).map_err(|e| e.to_string())?;

        if has_signatures {
            for extension in ["utoc", "ucas"] {
                let source_companion = source_pak.with_extension(extension);
                if !source_companion.exists() {
                    continue;
                }
                let destination_companion = destination_pak.with_extension(extension);
                if let Some(parent) = destination_companion.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                symlink_file(&source_companion, &destination_companion)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

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

fn resolve_output_mods_folder(conn: &Connection) -> Result<PathBuf, String> {
    let game_path = query_setting(conn, "paths.game")
        .map_err(|e| e.to_string())?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "paths.game is not set in settings".to_string())?;

    Ok(Path::new(&game_path)
        .join("MarvelGame")
        .join("Marvel")
        .join("Content")
        .join("Paks")
        .join("~mods"))
}

fn ensure_and_clear_output_folder(output_folder: &Path) -> io::Result<()> {
    fs::create_dir_all(output_folder)?;
    clear_directory_contents(output_folder)
}

fn clear_directory_contents(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.file_type().is_dir() {
            fs::remove_dir_all(&entry_path)?;
        } else {
            fs::remove_file(&entry_path)?;
        }
    }
    Ok(())
}

fn resolve_source_file_path(mod_path: &Path, filename: &str) -> PathBuf {
    if mod_path.is_dir() {
        return mod_path.join(filename);
    }

    let filename_path = Path::new(filename);
    if filename_path.components().count() == 1 {
        let current_name = mod_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if current_name == filename {
            return mod_path.to_path_buf();
        }
    }

    mod_path
        .parent()
        .map(|parent| parent.join(filename_path))
        .unwrap_or_else(|| mod_path.to_path_buf())
}

#[cfg(unix)]
fn symlink_file(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(windows)]
fn symlink_file(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(source, destination)
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

struct RefreshModCandidate {
    discovered_mod: DiscoveredMod,
    name: String,
    author: String,
    nexus_mod_id: Option<i64>,
    mod_id_changed: bool,
    auto_category: Option<String>,
    is_new: bool,
}

enum CompiledMatcher {
    Basic {
        category: String,
        pattern: String,
        case_sensitive: bool,
    },
    Regex {
        category: String,
        pattern: regex::Regex,
    },
}

struct NexusResolvedDetails {
    name: String,
    author: String,
    uploader_name: Option<String>,
}

#[derive(Deserialize)]
struct NexusModApiResponse {
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    uploader: Option<NexusUploader>,
}

#[derive(Deserialize)]
struct NexusUploader {
    name: Option<String>,
}

async fn fetch_nexus_mod_details(
    client: &reqwest::Client,
    token: &str,
    mod_id: i64,
) -> Result<Option<NexusResolvedDetails>, String> {
    let url = format!(
        "{}/games/{}/mods/{}.json",
        NEXUS_API_BASE_URL, NEXUS_GAME_DOMAIN_NAME, mod_id
    );
    let response = client
        .get(url)
        .header("apikey", token)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let body = response
        .json::<NexusModApiResponse>()
        .await
        .map_err(|e| e.to_string())?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Ok(None);
    }

    let author = if !body.author.trim().is_empty() {
        body.author.trim().to_string()
    } else {
        body.uploader
            .as_ref()
            .and_then(|uploader| uploader.name.as_deref())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Unknown".to_string())
    };
    let uploader_name = body
        .uploader
        .and_then(|uploader| uploader.name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(Some(NexusResolvedDetails {
        name,
        author,
        uploader_name,
    }))
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
                    file.strip_prefix(&path)
                        .ok()
                        .map(|relative| DiscoveredModFile {
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

fn normalize_matcher_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "string" => Some("string"),
        "regex" => Some("regex"),
        _ => None,
    }
}

fn load_category_management_data(conn: &Connection) -> rusqlite::Result<CategoryManagementData> {
    let mut categories_stmt =
        conn.prepare("SELECT category, is_api FROM categories ORDER BY category ASC")?;
    let category_rows = categories_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))
    })?;

    let mut categories = Vec::new();
    for row in category_rows {
        let (category, is_api) = row?;
        categories.push((category, is_api));
    }

    let mut matcher_map: BTreeMap<String, Vec<CategoryMatcher>> = BTreeMap::new();
    let mut matcher_stmt = conn.prepare(
        r#"
        SELECT id, category, pattern, matcher_type, case_sensitive
        FROM category_matchers
        ORDER BY category ASC, id ASC
        "#,
    )?;
    let matcher_rows = matcher_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;
    for row in matcher_rows {
        let (id, category, pattern, matcher_type, case_sensitive) = row?;
        matcher_map
            .entry(category)
            .or_default()
            .push(CategoryMatcher {
                id,
                pattern,
                matcher_type,
                case_sensitive,
            });
    }

    let mut api_categories = Vec::new();
    let mut custom_categories = Vec::new();
    for (category, is_api) in categories {
        let entry = CategoryWithMatchers {
            matchers: matcher_map.remove(&category).unwrap_or_default(),
            category: category.clone(),
        };
        if is_api {
            api_categories.push(entry);
        } else {
            custom_categories.push(entry);
        }
    }

    Ok(CategoryManagementData {
        api_categories,
        custom_categories,
    })
}

fn load_compiled_category_matchers(conn: &Connection) -> rusqlite::Result<Vec<CompiledMatcher>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT category, pattern, matcher_type, case_sensitive
        FROM category_matchers
        ORDER BY category ASC, id ASC
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, bool>(3)?,
        ))
    })?;

    let mut matchers = Vec::new();
    for row in rows {
        let (category, pattern, matcher_type, case_sensitive) = row?;
        if pattern.trim().is_empty() {
            continue;
        }

        match normalize_matcher_type(&matcher_type) {
            Some("regex") => {
                let regex = RegexBuilder::new(pattern.trim())
                    .case_insensitive(!case_sensitive)
                    .build();
                if let Ok(regex) = regex {
                    matchers.push(CompiledMatcher::Regex {
                        category,
                        pattern: regex,
                    });
                }
            }
            _ => matchers.push(CompiledMatcher::Basic {
                category,
                pattern: pattern.trim().to_string(),
                case_sensitive,
            }),
        }
    }

    Ok(matchers)
}

fn find_auto_category_for_mod(
    name: &str,
    path: &str,
    matchers: &[CompiledMatcher],
) -> Option<String> {
    for matcher in matchers {
        if matcher_matches_text(matcher, name) {
            return Some(match matcher {
                CompiledMatcher::Basic { category, .. } => category.clone(),
                CompiledMatcher::Regex { category, .. } => category.clone(),
            });
        }
    }

    for matcher in matchers {
        if matcher_matches_text(matcher, path) {
            return Some(match matcher {
                CompiledMatcher::Basic { category, .. } => category.clone(),
                CompiledMatcher::Regex { category, .. } => category.clone(),
            });
        }
    }

    None
}

fn matcher_matches_text(matcher: &CompiledMatcher, text: &str) -> bool {
    match matcher {
        CompiledMatcher::Basic {
            pattern,
            case_sensitive,
            ..
        } => {
            if *case_sensitive {
                text.contains(pattern)
            } else {
                text.to_ascii_lowercase()
                    .contains(&pattern.to_ascii_lowercase())
            }
        }
        CompiledMatcher::Regex { pattern, .. } => pattern.is_match(text),
    }
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
    let mut default_matchers: Vec<(String, String)> = Vec::new();
    for hero in heroes {
        let name = hero.name.trim();
        if !name.is_empty() {
            let category = format_category_name(name);
            unique_names.insert(category.clone());
            default_matchers.push((category.clone(), name.to_string()));
            let real_name = hero.real_name.trim();
            if !real_name.is_empty() {
                default_matchers.push((category, real_name.to_string()));
            }
        }
    }
    let api_categories: Vec<String> = unique_names.iter().cloned().collect();

    let state = state.lock().map_err(|e| e.to_string())?;
    let tx = state
        .db
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    {
        tx.execute(
            r#"INSERT OR IGNORE INTO categories (category, is_api) VALUES ('Uncategorized', 0)"#,
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE categories SET is_api = 0 WHERE category = 'Uncategorized'",
            [],
        )
        .map_err(|e| e.to_string())?;

        let mut upsert_category = tx
            .prepare(
                r#"
                INSERT INTO categories (category, is_api)
                VALUES (?1, 1)
                ON CONFLICT(category) DO UPDATE SET is_api = 1
                "#,
            )
            .map_err(|e| e.to_string())?;
        for category in &api_categories {
            upsert_category
                .execute([category])
                .map_err(|e| e.to_string())?;
        }

        let stale_api_categories = if api_categories.is_empty() {
            let mut stmt = tx
                .prepare("SELECT category FROM categories WHERE is_api = 1")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            let mut values = Vec::new();
            for row in rows {
                values.push(row.map_err(|e| e.to_string())?);
            }
            values
        } else {
            let placeholders = (1..=api_categories.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "SELECT category FROM categories WHERE is_api = 1 AND category NOT IN ({placeholders})"
            );
            let mut stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params_from_iter(api_categories.iter()), |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| e.to_string())?;
            let mut values = Vec::new();
            for row in rows {
                values.push(row.map_err(|e| e.to_string())?);
            }
            values
        };

        for category in stale_api_categories {
            tx.execute(
                "UPDATE mods SET auto_category = NULL WHERE auto_category = ?1",
                [category.as_str()],
            )
            .map_err(|e| e.to_string())?;

            let manual_count: i64 = tx
                .query_row(
                    "SELECT COUNT(1) FROM mods WHERE category_is_manual = 1 AND category = ?1",
                    [category.as_str()],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if manual_count > 0 {
                tx.execute(
                    "UPDATE categories SET is_api = 0 WHERE category = ?1",
                    [category.as_str()],
                )
                .map_err(|e| e.to_string())?;
            } else {
                tx.execute(
                    "DELETE FROM category_matchers WHERE category = ?1",
                    [category.as_str()],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "DELETE FROM categories WHERE category = ?1 AND is_api = 1",
                    [category.as_str()],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        let mut insert_default_matcher = tx
            .prepare(
                r#"
                INSERT OR IGNORE INTO category_matchers (category, pattern, matcher_type, case_sensitive)
                VALUES (?1, ?2, 'string', 0)
                "#,
            )
            .map_err(|e| e.to_string())?;
        for (category, pattern) in default_matchers {
            if pattern.trim().is_empty() {
                continue;
            }
            insert_default_matcher
                .execute(params![category, pattern])
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

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
                get_category_management,
                create_custom_category,
                set_category_matchers,
                get_setting,
                set_setting,
                get_mods,
                refresh_mods,
                set_mod_file_enabled,
                set_mod_enabled,
                set_mod_category,
                set_mods_category,
                reset_mods_category_to_auto,
                set_mod_name,
                set_mod_author,
                set_mods_author,
                set_mod_nexus_id,
                apply_mods,
                clear_mods_output,
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
            category TEXT NOT NULL UNIQUE,
            is_api INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS category_matchers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category TEXT NOT NULL,
            pattern TEXT NOT NULL,
            matcher_type TEXT NOT NULL,
            case_sensitive INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (category) REFERENCES categories(category) ON DELETE CASCADE
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
            mod_id_changed INTEGER NOT NULL DEFAULT 0,
            category TEXT NOT NULL,
            auto_category TEXT,
            category_is_manual INTEGER NOT NULL DEFAULT 0,
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
        "ALTER TABLE mods ADD COLUMN mod_id_changed INTEGER NOT NULL DEFAULT 0",
        [],
    )
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
    conn.execute(
        "ALTER TABLE categories ADD COLUMN is_api INTEGER NOT NULL DEFAULT 0",
        [],
    )
    .or_else(|err| {
        if err.to_string().contains("duplicate column name") {
            Ok(0)
        } else {
            Err(err)
        }
    })?;
    conn.execute("ALTER TABLE mods ADD COLUMN auto_category TEXT", [])
        .or_else(|err| {
            if err.to_string().contains("duplicate column name") {
                Ok(0)
            } else {
                Err(err)
            }
        })?;
    conn.execute(
        "ALTER TABLE mods ADD COLUMN category_is_manual INTEGER NOT NULL DEFAULT 0",
        [],
    )
    .or_else(|err| {
        if err.to_string().contains("duplicate column name") {
            Ok(0)
        } else {
            Err(err)
        }
    })?;

    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_path_unique ON mods(path)",
        [],
    )
    .map_err(|e| e)?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_mod_files_mod_filename_unique ON mod_files(mod_id, filename)",
        [],
    )
    .map_err(|e| e)?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_category_matchers_unique ON category_matchers(category, matcher_type, pattern, case_sensitive)",
        [],
    )
    .map_err(|e| e)?;
    conn.execute(
        "UPDATE mods SET auto_category = NULL WHERE category_is_manual = 1",
        [],
    )
    .map_err(|e| e)?;

    let default_groups = ["Uncategorized"];

    let tx = conn.transaction()?;
    {
        let mut insert_group =
            tx.prepare(r#"INSERT OR IGNORE INTO categories (category, is_api) VALUES (?1, 0)"#)?;
        for group in default_groups {
            insert_group.execute([group])?;
        }
        tx.execute(
            "UPDATE categories SET is_api = 0 WHERE category = 'Uncategorized'",
            [],
        )?;

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
