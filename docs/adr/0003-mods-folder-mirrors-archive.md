---
status: accepted
---

# `mods/` mirrors the archive; the database describes structure

A Mod's folder under `mods/` holds the archive's contents exactly as extracted — wrapper
folders, readmes, preview images and all — and the Variant and ModFile rows describe the
grouping by holding paths relative to that folder. We chose this over a normalized layout
(`<mod>/<variant>/<pak units>`, with everything else dropped or relocated) because the
database already carries the grouping, so a rewritten tree would duplicate it; extraction
stays a plain unpack; and Apply (reinstall or update) becomes "unpack again and match Variants
by relative path" with nothing to reconcile. Every detected Variant is extracted, so the
folder is always the full archive and no Variant carries an extracted-or-not state.
Decision record: wayfinder ticket #9.

## Consequences

- Disk usage equals the full unpacked archive per Mod. A skin mod with many color Variants
  can run past a gigabyte. If that matters later, extracting only chosen Variants from the
  kept Download is an additive feature, not a layout change.
- Mod folders are named `<database id>-<slug>`; the id makes them unique and stable across
  renames, so nothing on disk moves when a Mod's name changes.
- The extractor's only rewrite is refusing absolute and `..` entry paths.
