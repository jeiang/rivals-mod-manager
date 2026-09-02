---
status: accepted
---

# SQLite for library state, no `profiles/` folder

The managed library needs to store Mods, their Variants and ModFiles, global Tags, and
per-Profile modlists (enabled state, active Variants, and flat Group/order structure per
Profile) — data with real many-to-many and per-context cross-references (Mod↔Tag,
Profile↔Mod↔Group). We chose a single SQLite database over MO2-style flat JSON/RON files.
Relational queries ("which Mods carry tag X", "what's enabled in Profile Y") come for free as
joins instead of hand-rolled in-memory scans, and SQLite gives atomic writes where flat-file
writes risk partial-write corruption on a crash mid-save. Nothing in this effort's scope calls
for hand-editable or git-diffable profile files, so JSON/RON's main advantage doesn't apply
here. Decision record: wayfinder ticket #8.

## Consequences

- MO2's `profiles/` folder (per-profile modlist files) is dropped entirely — a Profile is pure
  DB rows, with `downloads/` and `mods/` the only folders left in the managed library.
- Manual inspection or editing of profile/modlist state requires a DB tool or an in-app export,
  not a text editor.
