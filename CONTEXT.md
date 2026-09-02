# Rivals Mod Manager

A desktop manager for Marvel Rivals mods: a managed library of downloaded archives and
extracted mod content, deployed into the game's `~mods` folder per an active Profile.

## Language

**Download**:
A file in `downloads/` — the archive as retrieved (from NexusMods or elsewhere). Carries an
md5 for identification and, when known, its NexusMods `(mod_id, file_id)`. One Download can
produce more than one Mod (a multi-mod archive is split by hand into separate Mods).
_Avoid_: Archive (use only when specifically talking about the file format, not the entity).

**Mod**:
An extracted unit of moddable content, living under `mods/`. Holds its own metadata (name,
author, category, tags) and one or more Variants. Hero/skin identification (scanned from pak
asset paths) is cached at the Mod level.
_Avoid_: Package, entry.

**Variant**:
An alternative or additional piece of content within a Mod — e.g. a skin-color choice, or an
independent modifier like a mask removal. Variants are composable: more than one Variant of
the same Mod can be active at once (a color choice plus a modifier, say), with no exclusivity
enforced by the domain. Each Variant owns one or more ModFiles.
_Avoid_: Option, choice.

**ModFile**:
One mountable unit within a Variant: a `.pak`+`.ucas`+`.utoc` triple, or a lone legacy `.pak`.
Sidecar discovery (matching `.ucas`/`.utoc` to a `.pak`'s stem) is a filesystem-level detail
resolved at deploy time, not separate rows — a ModFile is never partially enabled.
_Avoid_: File (too generic — a ModFile may back onto several physical files).

**Tag**:
A global, freeform label, many-to-many with Mod. Seeded from NexusMods categories on import;
any session may add new ones. Tags and other Mod metadata are shared across all Profiles.
_Avoid_: Category (Category is the single NexusMods-sourced classification; Tag is the
open-ended many-per-mod label).

**Profile**:
A named, ordered modlist: which Mods are enabled, which Variants of each are active, their
order, and their Group structure — all scoped to that Profile alone. A newly imported Mod
auto-appears, disabled, in every existing Profile.

**Group**:
A named, collapsible, bulk-toggleable section of a Profile's modlist. Groups are flat (no
nesting) and exist only within a Profile — they are markers in that Profile's ordered
sequence, and a Mod's group is whichever marker sits above it (implicit by position, not an
explicit assignment). Group and separator are the same concept.
_Avoid_: Separator (kept as a synonym in UI copy only; the domain term is Group).
