use std::{
    collections::BTreeSet,
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
    path: String,
    category: String,
    files: Vec<String>,
    last_modified: Option<i64>,
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
            .prepare("SELECT id, name, path, category FROM mods ORDER BY name ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
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
    for (id, name, path, category) in db_rows {
        mods.push(ModEntry {
            id,
            name,
            files: find_mod_files(&path),
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
                INSERT INTO mods (name, path, category)
                VALUES (?1, ?2, 'Uncategorized')
                ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name
                "#,
            )
            .map_err(|e| e.to_string())?;

        for (name, path) in discovered {
            upsert
                .execute(params![name, path])
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

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

fn discover_mods(mods_folder: &str) -> Result<Vec<(String, String)>, String> {
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
            mods.push((name, path.to_string_lossy().to_string()));
        } else if metadata.is_file() && has_mod_extension(&path) {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Unnamed Mod")
                .to_string();
            mods.push((name, path.to_string_lossy().to_string()));
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

fn find_mod_files(path: &str) -> Vec<String> {
    let mut files = Vec::new();
    collect_mod_files(Path::new(path), &mut files);
    files.sort_unstable();
    files
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
            ext == "pak" || ext == "ucas" || ext == "utoc"
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
            path TEXT NOT NULL,
            nexus_mod_id INTEGER,
            category TEXT NOT NULL,
            FOREIGN KEY (category) REFERENCES categories(category)
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

    conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_path_unique ON mods(path)", [])
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
