# Copilot Instructions for rivals-mod-manager

## Project Overview

**rivals-mod-manager** is a desktop GUI application built in Rust for organizing and managing mods for the game "Marvel Rivals". The application categorizes mods using regex pattern matching and displays them in an egui-based UI.

### Architecture

**Key modules:**

- **`app.rs`**: Main UI application using `eframe` (egui framework). Manages state, rendering pages (Mods, Categories, Settings), and persistence via serde.
- **`mods.rs`**: Handles filesystem scanning of mod directories using async `WalkDir`. Identifies `.pak` files, extracts mod metadata (ID, name, author), and stores in `ModList`.
- **`categories.rs`**: Manages regex-based category matching. Includes 40+ hardcoded character matchers (e.g., "Iron Man" matches "iron man", "tony", "stark").
- **`settings.rs`**: Stores user configuration (game folder path, input folder path, NexusMods API key).

**Technology Stack:**
- **egui**: Immediate-mode GUI framework for cross-platform desktop UI
- **tokio**: Async runtime (full features enabled)
- **regex/lazy-regex**: Pattern matching for mod categorization
- **serde**: Serialization for app state persistence
- **eframe**: Wrapper for egui window management

## Build and Test Commands

### Build
```bash
# Debug build
cargo build

# Release build (optimized for size and speed)
cargo build --release
```

### Run
```bash
# Run the application
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

### Testing
```bash
# Run all tests (currently empty test suite)
cargo test

# Run tests with output displayed
cargo test -- --nocapture

# Run a single test
cargo test test_name --
```

### Formatting and Linting

```bash
# Check formatting (doesn't modify files)
cargo fmt --check

# Format all Rust files
cargo fmt

# Run clippy (linter)
cargo clippy

# Run clippy with all warnings treated as errors
cargo clippy -- -D warnings

# Fix common clippy issues automatically
cargo clippy --fix --allow-dirty
```

### Documentation
```bash
# Generate and open documentation
cargo doc --open
```

## Key Conventions and Patterns

### State Management
- **App-level state** is stored in the `State` struct and persisted via serde to `eframe::Storage` (OS-specific location).
- The UI is immediate-mode: state is read, UI elements rendered, then mutations applied at the end of the frame.
- `egui_async::Bind<T, E>` is used for async operations (e.g., `refresh_mod_list`) that resolve over multiple frames.

### Mod Discovery
- Mods are directories under a configurable folder (set in Settings).
- Only `.pak` files are recognized as mod files (case-insensitive extension matching).
- **Async filesystem scanning** uses `async_walkdir::WalkDir` to avoid blocking the UI thread.
- Mod metadata is extracted from filename parsing using regex (e.g., mod ID and author from filename).

### Category Matching
- Categories are predefined via `default_matchers()` in `categories.rs` (40+ Marvel character names).
- Each category has a name and a list of `regex_helper::RegexProxy` patterns for case-insensitive matching.
- Matches are checked against the mod name; matches determine the category assignment.
- New categories can be added by editing the hardcoded list or via the UI (stored in settings).

### Error Handling
- Filesystem errors are surfaced via `io::Error` and displayed as toast notifications.
- Invalid inputs (e.g., unparseable mod IDs) use `.unwrap()` or `.expect()` in parsing code—these may panic on malformed files.

### UI Pages
- **Mods page**: Displays filtered/searched mod list with enable/disable toggles.
- **Categories page**: Create/edit regex matchers for categories.
- **Settings page**: Configure game folder, input folder, and NexusMods API key.

## Development Tips

### Debugging
- Use `RUST_LOG=debug` environment variable to enable debug logging via the `log` crate.
- The app logs to stderr by default (managed by `env_logger`).

### Workspace Lints
- All lints are configured in `Cargo.toml` under `[workspace.lints]`.
- `unsafe_code` is fully denied; unsafe blocks must have `// SAFETY:` comments (enforced by lints).
- Clippy is very strict (most warnings enabled); fix warnings before committing.

### Dependencies
- **egui ecosystem** (`eframe`, `egui`, `egui_async`, `egui_extras`, `egui_material_icons`, `egui-toast`): Core UI.
- **regex/lazy-regex**: Precompiled regex patterns for performance.
- **async runtime** (tokio): Full feature set enabled for maximum compatibility.
- **chrono**: Date/time handling (last modified timestamps).

### Nix Development Environment
- `flake.nix` is configured for development with:
  - Latest nightly Rust compiler
  - Targets for Windows MSVC and Linux musl (for cross-compilation)
  - Dependency caching via `crane`
  - Security vulnerability scanning via advisory-db
- Use `nix flake show` to view available dev environments and builds.

### Icon and Assets
- App icon is included as `src/assets/logo.png` and embedded via `include_bytes!()`.
- Image must be valid PNG; loading errors will cause the app to fail at startup.

## Common Issues and Solutions

**Issue: Clippy warnings when modifying code**
- Solution: Run `cargo clippy --fix --allow-dirty` to auto-fix common issues. Review changes carefully.

**Issue: UI doesn't respond when scanning large mod directories**
- Solution: Already solved in the codebase via async filesystem scanning. Ensure new filesystem operations use `async_walkdir` or similar non-blocking APIs.

**Issue: Mod metadata parsing fails on unusual mod names**
- Solution: Regex patterns in `mods.rs` may need adjustment. Test regex patterns separately using `regex` crate's online tools before committing.

## File Structure

```
src/
├── main.rs             # Entry point, window setup
├── lib.rs              # Library exports
├── app.rs              # Main UI application struct
├── mods.rs             # Mod discovery and metadata extraction
├── categories.rs       # Character category matchers
├── settings.rs         # Settings data model
└── assets/
    └── logo.png        # App icon (embedded at compile time)
```
