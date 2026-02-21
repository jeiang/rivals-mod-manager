use std::{collections::BTreeSet, error::Error, sync::Mutex};

use rusqlite::{params, params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

type AppState = Mutex<AppData>;

// TODO: update with sensible defaults.
const DEFAULT_MODS_FOLDER: &str =
    "/home/aidanp/Games/Steam/MarvelRivals/MarvelGame/Marvel/Content/Paks/~mods";
// TODO: remove this and use a configuration value.
const MARVEL_RIVALS_API_KEY: &str =
    "fa25e0685957097c542fd9472c3d5cda5f1dc1a511369de77fd49ce1d8c90315";
const HEROES_API_URL: &str = "https://marvelrivalsapi.com/api/v2/heroes";

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

#[tauri::command]
async fn refresh_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let heroes = reqwest::Client::new()
        .get(HEROES_API_URL)
        .header("x-api-key", MARVEL_RIVALS_API_KEY)
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
                get_mods,
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
