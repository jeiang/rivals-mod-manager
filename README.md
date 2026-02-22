# Rivals Mod Manager

Rivals Mod Manager is a desktop app for managing Marvel Rivals mods.

It is built with:
- Tauri v2 (Rust backend)
- SolidJS + TypeScript + Vite (frontend)
- SQLite (local app data)

## What It Does

- Scans and lists mods and their files.
- Enables or disables mods and individual files.
- Applies enabled mods into the game output folder.
- Clears generated mod output.
- Supports bulk editing for mod metadata:
  - Name
  - Author
  - Nexus Mod ID
  - Category
- Manages categories and category matchers.
- Stores local settings for paths and API keys.

## Settings

The app includes settings for:
- NexusMods API key (`tokens.nexusmods`)
- MarvelRivalsAPI API key (`tokens.marvelrivalsapi`)
- Game folder (`paths.game`)
- Input mods folder (`paths.mods`)
- Downloads folder (`paths.downloads`)

## Development

Prerequisites:
- Rust toolchain
- Bun
- System dependencies required by Tauri (platform-specific)

Install dependencies:

```bash
bun install
```

Run in development:

```bash
bun run tauri dev
```

Build production bundles:

```bash
bun run tauri build
```

## Project Structure

- `src/` - SolidJS frontend
- `src-tauri/` - Rust backend and Tauri config
- `src-tauri/tauri.conf.json` - Tauri app/build/bundle configuration

## Release Automation

GitHub Actions is configured to build releases with `tauri-apps/tauri-action` when a tag matching `release-*` is pushed.
