# Copilot Instructions for Rivals Mod Manager

## Quick Start

**Install & Run:**
```bash
bun install           # Install Node dependencies
bun run tauri dev     # Start dev server (Vite + Tauri with hot reload)
bun run tauri build   # Build production bundle
```

**Single Lint/Format:**
```bash
oxlint src/           # Lint TypeScript/JSX only (fast)
oxlint --fix src/     # Auto-fix linting issues
oxfmt src/            # Format TypeScript/JSX
```

Linting uses **oxlint** (Rust-based, much faster than ESLint), configured in `.oxlintrc.json` with basic rules (`no-unused-vars`, `no-missing-imports`).

## Architecture

**Tech Stack:**
- **Frontend:** SolidJS 1.9.3 + TypeScript + Vite, styled with Tailwind CSS 4.2
- **Backend:** Tauri v2 (Rust) + SQLite (bundled via rusqlite)
- **CLI:** Bun (package manager/runtime)

**High-Level Flow:**
1. **Frontend** (`src/`) runs in Vite dev server (port 1420) and compiles to `dist/` for production
2. **Tauri** wraps the compiled frontend in a native desktop app (Windows/macOS/Linux)
3. **Rust backend** (`src-tauri/src/`) exposes commands invoked from frontend via IPC
4. **Data** stored in local SQLite database (initialized at `src-tauri/data.db`)

**Entry Points:**
- Frontend: `src/index.tsx` → `src/pages/Main.tsx`, `Settings.tsx`, `Categories.tsx`
- Backend: `src-tauri/src/lib.rs` (defines Tauri commands & app logic)
- Tauri config: `src-tauri/tauri.conf.json` (app name, build settings, security)

## Key Conventions

**Frontend (SolidJS/TypeScript):**
- Page components live in `src/pages/` (routed via `@solidjs/router`)
- Reusable components in `src/components/`
- Tailwind CSS only—no CSS files needed (configured in `vite.config.ts`)
- Call Rust commands via `@tauri-apps/api/core`: `invoke("command_name", args)` (async)
- No tests configured; linting is your safety net

**Backend (Rust):**
- Commands are Tauri invocations: `#[tauri::command]` decorated functions in `src/lib.rs`
- Database uses **rusqlite** directly; queries execute synchronously in `AppState` (Mutex-wrapped `Connection`)
- Serialization via `serde` for JSON IPC between frontend/backend
- HTTP calls via **reqwest** (used for API integrations like NexusMods)

**File Structure:**
```
src/                              # Frontend (SolidJS)
├── pages/                        # Page components (Main, Settings, Categories)
├── components/                   # Reusable components (Button, TopNav, etc.)
├── assets/                       # Static assets
├── index.tsx                     # App router & entry point
└── index.css                     # Global Tailwind imports

src-tauri/                        # Rust backend
├── src/
│   ├── lib.rs                    # Tauri command handlers & app logic
│   └── main.rs                   # Minimal entry point (calls lib.rs)
├── Cargo.toml                    # Rust dependencies
├── tauri.conf.json               # Tauri app config (name, icon, security)
└── data.db                       # SQLite database (created at runtime)

.github/workflows/release.yml     # GitHub Actions for release automation
```

## Development Workflow

**Hot Reload:**
- Running `bun run tauri dev` starts both Vite dev server and Tauri in dev mode
- Changes to `src/` (TypeScript/JSX) hot-reload automatically in the Tauri window
- Changes to `src-tauri/src/` require restarting `tauri dev`

**Adding a New Frontend Page:**
1. Create `src/pages/NewPage.tsx` as a SolidJS component
2. Add route in `src/index.tsx`: `<Route path="/newpage" component={NewPage} />`
3. Link to it from navigation components as needed

**Adding a New Tauri Command:**
1. Define an async Rust function in `src-tauri/src/lib.rs` with `#[tauri::command]` macro
2. Invoke from frontend: `invoke("command_name", { arg1, arg2, ... })`
3. Commands automatically serialize/deserialize JSON between frontend/backend

**Database Changes:**
- SQLite schema defined implicitly when Rust queries execute in `lib.rs`
- Queries use **rusqlite** params to prevent SQL injection
- Database persists in `src-tauri/data.db` (gitignored)

## Testing & Verification

No automated tests configured—verify manually:
- **Frontend:** Test UI interactions in `bun run tauri dev`
- **Backend:** Test Tauri commands by invoking them from UI or via Tauri CLI testing tools
- **Linting:** `oxlint` catches basic issues; run before commits

## Release & Deployment

Push a tag matching `release-*` (e.g., `v0.2.0` → `release-v0.2.0`) to trigger GitHub Actions CI/CD:
- Workflow: `.github/workflows/release.yml`
- Builds binaries for Windows, macOS, Linux via `tauri-apps/tauri-action`
- Publishes release artifacts to GitHub Releases

## Common Tasks

**Update a Tauri command signature:**
- Edit the Rust function in `src-tauri/src/lib.rs`
- Update the frontend call in `src/` to match new args
- The app recompiles on next `tauri dev` restart

**Modify database schema:**
- Tauri creates/initializes the SQLite database on startup
- Alter tables in the `lib.rs` initialization logic or in individual command handlers
- Schema changes are not versioned; tests must verify schema expectations

**Add a new dependency:**
- JavaScript/TypeScript: `bun add package-name` (updates `package.json` & `bun.lock`)
- Rust: Edit `src-tauri/Cargo.toml`, then `cargo check` to fetch and verify

## MCP Servers (Optional)

If you've configured MCP servers in your Copilot environment, these are useful for this project:

- **Filesystem:** Navigate project structure, read files in bulk
- **Shell:** Run build/test commands (`bun run tauri dev`, `oxlint`, etc.) and inspect output
- **SQLite:** Inspect the app's SQLite schema directly from `src-tauri/data.db` (useful for understanding data persistence)
