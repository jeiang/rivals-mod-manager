---
status: accepted
---

# Tauri v2 with a Svelte frontend and a Tauri-free Rust core

The v2 rewrite needs a Windows/macOS/Linux desktop UI with a rich mod table (collapsible groups, drag-reorder), image previews, an `nxm://` protocol handler, a workable self-update path, and the domain logic in Rust. We chose Tauri v2 with a plain Vite + Svelte 5 + TypeScript frontend, and a Cargo workspace of two crates: a library crate holding the domain core with no `tauri` dependency, and the `src-tauri` app crate holding only command handlers, state wiring, and plugin setup. Tauri is the only candidate that solves the two hardest cross-platform requirements, `nxm://` registration and self-update, with first-party plugins, and everything else on the list is commodity web UI. Survey and sources: `docs/research/ui-stacks.md` on the `research/ui-stacks` branch (wayfinder ticket #3); decision record on ticket #7.

## Considered options

- **iced, Slint, GPUI-CE** (all-Rust, no IPC boundary). Passed over: none has a first-party `nxm://` or updater story, drag-reorder is assembled from primitives or community crates, and each describes itself as experimental, pre-1.0, or still closing desktop gaps. A prior iced attempt lives on `refactor/iced`.
- **Electron, Avalonia**. Rejected: both put the UI in a second runtime with an FFI or sidecar boundary to the Rust core; Electron also bundles 117 to 150 MB of Chromium.
- **SvelteKit** (Tauri's scaffold default). Rejected in favor of plain Vite + Svelte: a single-window app has no routes, so file-based routing and SSR config are dead weight.
- **Rust WASM frontend** (Leptos, Dioxus, Yew). Rejected: a second Rust compile target while giving up the web ecosystem that motivated Tauri.
- **Single crate**. Rejected: every `cargo test` and `cargo clippy` would pull the webview build deps, and the core could not be exercised without them.

## Consequences

- `nxm://` uses the deep-link plugin plus the single-instance plugin to route a second launch's URL into the running app. macOS needs the scheme in the bundle config and only works from an app installed in `/Applications`. Moving a Linux AppImage breaks the registration.
- Self-update uses the updater plugin. Signature verification is mandatory. Linux updates only via AppImage. On Windows the app exits during install.
- Drag-reorder in the mod list must use a pointer-event library (for example `svelte-dnd-action`, which listens to mouse and touch events) rather than the HTML5 drag API, so Tauri's native drag-drop handler can stay enabled and OS file drops still arrive with full paths. Enabling HTML5 drag-and-drop on Windows would require `dragDropEnabled: false`, which removes those path-bearing drop events.
- Linux users need `webkit2gtk-4.1` from their distro. The nix devshell gains Node and the Tauri CLI. Package manager is npm.
- Packaging, signing, self-update hosting, and CI release builds per OS are designed separately (see the wayfinder map, issue #2).
