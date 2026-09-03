# Rivals Mod Manager

A desktop manager for Marvel Rivals mods: a managed library of downloaded archives and
extracted mod content, deployed into the game's `~mods` folder per an active Profile.

## Language

**Download**:
A file in `downloads/` — the archive as retrieved (from NexusMods or elsewhere). Carries an
md5 for identification and, when known, its NexusMods `(mod_id, file_id)`. One Download can
produce more than one Mod (a multi-mod archive is split by hand into separate Mods), and a
Mod need not have one at all (loose folders and Imported v1 mods arrive without an archive).
_Avoid_: Archive (use only when specifically talking about the file format, not the entity).

**Mod**:
An extracted unit of moddable content, living under `mods/`. Holds its own metadata (name,
author, category, tags) and one or more Variants. The heroes and skins a Mod touches are the
union of what its ModFiles were Scanned to contain.
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
resolved at deploy time, not separate rows — a ModFile is never partially enabled. Each
ModFile carries the hero/skin pairs its pak index was Scanned to contain (possibly none).
_Avoid_: File (too generic — a ModFile may back onto several physical files).

**Tag**:
A global, freeform label, many-to-many with Mod. Seeded from NexusMods categories on import;
any session may add new ones. Tags and other Mod metadata are shared across all Profiles.
_Avoid_: Category (Category is the single NexusMods-sourced classification; Tag is the
open-ended many-per-mod label).

**Roster**:
The known mapping from the game's numeric hero and skin ids to display names. Seeded from a
snapshot bundled with the app and refreshed from a public source; an id the Roster does not
know is shown verbatim rather than hidden.
_Avoid_: Hero list, character table.

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

## Operations

**Install**:
Bringing new content into the library: an archive (which becomes a Download), a loose folder,
or a loose set of pak files, turned into one or more Mods after the user confirms the detected
Variant grouping and any multi-mod split.
_Avoid_: Add, unpack.

**Apply**:
Re-extracting an existing Mod from a source — its own Download (reinstall), a newer Download
(update), or its current folder when it has no Download — keeping the Mod's identity, metadata,
Tags, and Profile entries. Reinstall and update are the same operation.
_Avoid_: Reinstall/update as distinct concepts.

**Import**:
The one-time migration of a v1 watched folder into the library: each v1 mod folder becomes a
Download-less Mod, with v1's edits, category, and enabled state carried over.
_Avoid_: Migrate (reserved for database schema changes).

**Identify**:
Attaching a NexusMods identity to a Download (its `(mod_id, file_id)`, found by md5 lookup or
carried by the `nxm://` link) and pulling that mod's NexusMods metadata onto the resulting Mod.
Happens at Install when a credential is present, or later on demand.
_Avoid_: Link, match, lookup.

**Scan**:
Reading a ModFile's pak index for the hero/skin ids its asset paths name, run after
extraction at Install and Apply. A Scan that finds nothing (unreadable index, or no character
content) leaves the ModFile empty; the Mod's hero is then guessed from its NexusMods category
or its name, as a Tag only. Heroes found by Scan or guess become Tags by name; skins are
display text only.
_Avoid_: Identify (reserved for the NexusMods identity), detect.
