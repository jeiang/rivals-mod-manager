use std::{error::Error, sync::Mutex};

use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::{Manager, State};

type AppState = Mutex<AppData>;

// TODO: update with sensible defaults.
const DEFAULT_MODS_FOLDER: &str =
    "/home/aidanp/Games/Steam/MarvelRivals/MarvelGame/Marvel/Content/Paks/~mods";

struct AppData {
    db: rusqlite::Connection,
}

#[derive(Serialize)]
struct ModEntry {
    id: i64,
    name: String,
    path: String,
    category: String,
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
fn get_mods(state: State<'_, AppState>) -> Result<Vec<ModEntry>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let mut stmt = state
        .db
        .prepare("SELECT id, name, path, category FROM mods ORDER BY name ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ModEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                category: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut mods = Vec::new();
    for row in rows {
        mods.push(row.map_err(|e| e.to_string())?);
    }

    Ok(mods)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    fn runner() -> Result<(), Box<dyn Error>> {
        let mut conn = Connection::open("./data.db")?;
        init_db(&mut conn)?;
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![greet, get_categories, get_mods])
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
            category TEXT NOT NULL,
            FOREIGN KEY (category) REFERENCES categories(category)
        );
        "#,
    )?;

    let default_groups = [
        "Doctor Strange",
        "Hulk",
        "Iron Man",
        "Spider-Man",
        "Luna Snow",
        "Namor",
        "Loki",
        "Black Panther",
        "Magik",
        "Rocket Raccoon",
        "Groot",
        "Peni Parker",
        "Storm",
        "Magneto",
        "Star-Lord",
        "Mantis",
        "The Punisher",
        "Scarlet Witch",
        "Hela",
        "Venom",
        "Adam Warlock",
        "Thor",
        "Jeff the Land Shark",
        "Winter Soldier",
        "Captain America",
        "Psylocke",
        "Moon Knight",
        "Hawkeye",
        "Squirrel Girl",
        "Iron Fist",
        "Black Widow",
        "Cloak & Dagger",
        "Wolverine",
        "Mister Fantastic",
        "Invisible Woman",
        "Human Torch",
        "The Thing",
        "Emma Frost",
        "Ultron",
        "Phoenix",
        "Blade",
        "Angela",
        "Daredevil",
        "Gambit",
        "Rogue",
        "Deadpool",
        "Elsa Bloodstone",
        "Uncategorized",
    ];

    let tx = conn.transaction()?;
    {
        let mut insert_group =
            tx.prepare(r#"INSERT OR IGNORE INTO categories (category) VALUES (?1)"#)?;
        for group in default_groups {
            insert_group.execute([group])?;
        }

        tx.execute(
            r#"
            INSERT INTO settings (name, value)
            SELECT ?1, ?2
            WHERE NOT EXISTS (
                SELECT 1 FROM settings WHERE name = ?1
            )
            "#,
            params!["paths.mods_folder", DEFAULT_MODS_FOLDER],
        )?;
    }
    tx.commit()?;

    Ok(())
}
