# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands
- Build: `bun build`
- Lint: `bun lint`
- Format: `bun fmt`
- Run single test: `bun test -- --testId=yourTestId`

## Code Architecture
This is a Tauri-based application with:
- Frontend: `src/` (React/Vue framework)
- Backend: `src-tauri/` (Rust/Tauri integration)
- Shared logic: `src/lib/`
- Utilities: `src/utils/`

## Development Notes
- Use `bun dev` for hot-reloading
- Tauri plugins are configured in `src-tauri/src/main.rs`
- API endpoints defined in `src/api/endpoint.rs`

## Special Instructions
- Follow Cursor rules in `.cursor/rules/`
- Adhere to Copilot instructions in `.github/copilot-instructions.md`
- Always validate UI updates in `src-tauri/src/main.rs`

## Linting and Formatting
- Install: `bun install oxlint oxfmt`
- Lint: `bun lint`
- Format: `bun fmt`
- Configuration: `.oxlintconfig` and `.oxfmtconfig` files