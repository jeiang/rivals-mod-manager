# Marvel Rivals pak internals: readability, hero IDs, load order

Research for wayfinder ticket #5. Facts verified 2026-08-30 against tool source code,
UE engine source mirrors, and Marvel Rivals modding community primary sources.

Game baseline: Marvel Rivals is Unreal Engine **5.3.2** (the community usmap file is named
`5.3.2-1525091+++depot_marvel+S1_1_release-Marvel.usmap`) and ships its content as IoStore
containers (`.pak`/`.ucas`/`.utoc`) under `MarvelRivals/MarvelGame/Marvel/Content/Paks/`.
[Source: picarica/Marvel-modding-guide README](https://github.com/picarica/Marvel-modding-guide)

## 1. Are mod pak/utoc indexes readable without game AES keys?

**Yes, in all practical cases.** Three situations:

1. **Unencrypted mod containers (the normal case).** Encryption is a per-container flag.
   In the IoStore TOC, the directory index is AES-decrypted only when
   `EIoContainerFlags::Encrypted` is set in the container header; otherwise it is read as
   plain data — no key involved. Verified in retoc source:
   [`read_directory_index` in retoc/src/lib.rs](https://github.com/trumank/retoc/blob/master/retoc/src/lib.rs)
   (`if header.container_flags.contains(EIoContainerFlags::Encrypted)`), same for chunk data
   (`container is encrypted but no AES key ... supplied` branch). Legacy `.pak` works the same
   way in repak: the index is parsed directly; `--aes-key` is only needed for encrypted paks
   ([repak README](https://github.com/trumank/repak)). Mods built by community tooling are
   written unencrypted — repak does not even support writing encrypted paks.

2. **"Obfuscated" mods.** Some Rivals mods are deliberately packed with the Encrypted flag
   set to deter asset ripping — but they are encrypted with the *game's own public AES key*
   (otherwise the game could not load them). The community mod manager detects this by
   checking the Encrypted bit in the `.utoc` header
   ([`is_iostore_obfuscated` in repak-rivals/repak-gui/src/utoc_utils.rs](https://github.com/natimerry/repak-rivals/blob/master/repak-gui/src/utoc_utils.rs))
   and then reads them with the hardcoded key.

3. **The game's own containers** are AES-encrypted, but the key has been public since launch:
   `0x0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74`
   ([picarica/Marvel-modding-guide](https://github.com/picarica/Marvel-modding-guide);
   hardcoded in [repak-rivals source](https://github.com/natimerry/repak-rivals/blob/master/repak-gui/src/utoc_utils.rs)).

**Conclusion for the mod manager:** read the `.utoc` directory index (or legacy `.pak` index)
directly; carry the public key as a fallback for the Encrypted-flag case. No user-supplied
secret needed.

## 2. Rust crates for reading mod paks

| Crate | Formats | UE5/IoStore | Status |
| --- | --- | --- | --- |
| [repak](https://github.com/trumank/repak) (trumank) | legacy `.pak` read+write, pak versions 2–11 (UE 4.0–5.3); zlib/gzip/zstd, oodle feature | no IoStore | Active. crates.io has only a 0.1.0 name-holder (Feb 2025); real releases are 0.2.x — **consume as git dependency** |
| [retoc](https://github.com/trumank/retoc) (trumank) | IoStore `.utoc`/`.ucas` pack/unpack, Zen↔legacy asset conversion | UE **5.3+ well supported** (README compatibility note) | Active. Workspace = `retoc` lib + `retoc_cli`; **not on crates.io** (git deps on jmap/ser-hex/repak) — consume as git dependency. AES key optional (`Option<String>` in CLI) |
| [repak-rivals / retoc-rivals](https://github.com/natimerry/repak-rivals) (natimerry) | Marvel Rivals fork of both: GUI + `retoc-rivals-cli` + libs | Rivals-specific: default MR AES key, obfuscation support, KawaiiPhysics/hidden-material patching, oodle default, auto `_9999999_P` suffix | Very active (v3.5.1, Aug 2026), MIT OR Apache-2.0, ships a nix flake. **Best reference implementation** for Rivals-specific edge cases |
| [unpak](https://github.com/bananaturtlesandwich/unpak) | legacy `.pak` read-only, UE4 versions | none | last release Mar 2024, lib.rs lists it "minimal maintenance" — not suitable |

Practical split: **repak** for legacy single-`.pak` mods (older mods still circulate),
**retoc** (or the retoc-rivals fork) for the modern `.pak/.utoc/.ucas` trios. Listing a mod's
contents needs only index parsing — retoc's `list`/`manifest` path, repak's index read — which
is cheap (repak "only parses index initially").

## 3. Asset-path conventions identifying hero/skin

Character content lives under a fixed tree inside the container index:

```
Marvel/Content/Marvel/Characters/<HeroID>/<SkinID>/{Meshes,Materials,Textures,...}
e.g. Marvel/Content/Marvel/Characters/1020/1020001/Meshes/SK_1020_1020001.uasset
```

- **HeroID** = 4 digits (playable heroes are `10xx`: 1011 Hulk, 1014 The Punisher,
  1015 Storm, 1016 Loki, 1037 Magneto, … 1066 The Hood). NPCs use other ranges
  (e.g. 4011 Spider Zero, 8053 Knull).
- **SkinID** = 7 digits = HeroID + 3-digit skin suffix (`1011001` style; base skin `001`,
  store/battle-pass skins in `1xx`/`3xx`/`5xx`/`8xx` bands, e.g. 1014100 "Camo",
  1011300 "Maestro").
- Asset names embed both: meshes `SK_<HeroID>_<SkinID>`, textures `T_<SkinID>_<part>_<D|N|ORM|S>`.

So hero/skin detection for a mod = scan its file index for
`Characters/(\d{4})/(\d{7})/` and map the captures.
[Sources: picarica/Marvel-modding-guide](https://github.com/picarica/Marvel-modding-guide),
[donutman07/MarvelRivalsCharacterIDs](https://github.com/donutman07/MarvelRivalsCharacterIDs/blob/main/MarvelRivalsCharacterIDs.md),
[RegularLunar/MR-Character-IDS](https://github.com/RegularLunar/MR-Character-IDS).
Other asset classes have parallel trees (UI, `Wwise` audio, `Movies`); a mod touching no
`Characters/` path is not a skin mod.

## 4. Sourcing the hero-ID → name roster dynamically

Options, best first:

1. **Liquipedia's `Hero_ID` page via the MediaWiki API** — live, structured (ID, name, role,
   release date; 53 heroes as of Aug 2026, through 1066 The Hood):
   `https://liquipedia.net/marvelrivals/api.php?action=parse&page=Hero_ID&format=json&prop=text`.
   Verified working; requires gzip `Accept-Encoding` and a descriptive User-Agent, and is
   subject to [Liquipedia's API terms](https://liquipedia.net/api-terms-of-use)
   (rate limits + attribution). [Page: liquipedia.net/marvelrivals/Hero_ID](https://liquipedia.net/marvelrivals/Hero_ID)
2. **Community GitHub datasets** —
   [donutman07/MarvelRivalsCharacterIDs](https://github.com/donutman07/MarvelRivalsCharacterIDs)
   (hero + per-skin names, markdown table) and
   [RegularLunar/MR-Character-IDS](https://github.com/RegularLunar/MR-Character-IDS)
   (heroes + NPCs by season; archived June 2026 — snapshot only). Markdown, not JSON;
   fine as a vendored snapshot, unreliable as a live feed.
3. **The game's own files** — hero DataTables/localization are extractable with the public
   AES key plus a usmap mapping file (repak-rivals even auto-downloads "the latest Rivals
   depot mapping"). Fully offline-correct but heavyweight; needs Zen asset parsing, not just
   index reads.

Recommended: ship a built-in snapshot (option 2/3 derived), refresh from option 1 at runtime,
and render unknown IDs verbatim so new heroes degrade gracefully instead of breaking.

## 5. `~mods` load-order semantics

Engine mechanics, verified in UE source (public mirrors of
`Engine/Source/Runtime/PakFile/Private/IPlatformFilePak.cpp` and `FilePackageStore.cpp`,
[UE 4.27](https://github.com/AlexMercer-MA/UnrealEngine-4.27) and
[UE 5.5](https://github.com/Pekyyyyyy/Toon-UE) — logic identical in both; Rivals is 5.3):

- **`~mods` is convention, not magic.** `FindPakFilesInDirectory` iterates `Content/Paks`
  **recursively**, so any subfolder works; the community standardized on
  `...\Marvel\Content\Paks\~mods`
  ([repak-rivals README](https://github.com/natimerry/repak-rivals)).
- **Mods ship as a same-stem trio** `Mod_9999999_P.pak` + `.utoc` + `.ucas`. Mounting the
  `.pak` also mounts the companion `.utoc`/`.ucas` into the IoDispatcher and package store
  *with the same priority* (`IPlatformFilePak.cpp`: `IoDispatcherFileBackend->Mount(*UtocPath,
  PakOrder, ...)`; `PackageStoreBackend->Mount(..., PakOrder)`). That is why retoc generates a
  stub "fake pak containing chunknames" for every IoStore mod
  ([retoc-rivals-cli docs](https://github.com/natimerry/repak-rivals/blob/master/docs/retoc-rivals-cli.md)).
- **`_P` suffix = patch priority, and the number before it is the knob.** If the filename ends
  `_P.pak`, the engine parses the numeric segment right before `_P` as a chunk version `N` and
  adds `100 * (N + 1)` to the pak's mount order (`Mount()` in `IPlatformFilePak.cpp`; base
  order for paks under the project dir is 0–4). Epic documents `_P` as "priority over other
  PAK files" ([UE docs: How to Create a Patch](https://dev.epicgames.com/documentation/en-us/unreal-engine/how-to-create-a-patch-in-unreal-engine)).
  Hence the community-standard `_9999999_P`: +1,000,000,000 priority, beating every game chunk.
  A bare `Mod_P.pak` gets only +100 — still above the base game.
- **Two paks touching the same asset: highest priority wins.** For equal priority (all mods
  use `_9999999_P`), the tie-break is subtle and *differs by lookup path*:
  - The scan mounts found paks in **reverse-alphabetical order**
    (`FoundPakFiles.Sort(TGreater<FString>())`).
  - **Zen packages (what skin mods are):** `FFilePackageStoreBackend` stable-sorts containers
    by order, then by mount sequence with **later-mounted first** — and later-mounted =
    alphabetically **earlier** filename. So among equal-priority IoStore mods, the
    **alphabetically first name wins**. This matches Rivals community guidance ("rename the
    files to something earlier alphabetically while keeping the `_9999999_P`",
    [Nexus: Beginners Tutorial — Load Order and BOSS](https://www.nexusmods.com/marvelrivals/news/11184)).
  - **Legacy loose-file lookups** resolve the opposite way (`FindFileInPakFiles` returns the
    first entry in a list where first-mounted — alphabetically **last** — sits first).
  - **Design consequence:** don't encode mod ordering in alphabetical ties. If the manager
    must arbitrate conflicts deterministically, give conflicting mods distinct numeric
    priorities (`_<N>_P` with different `N`) or renumber filenames, and verify in-game once.
- **Anti-mod history.** Season 1 (Jan 2025) added asset hash checking that temporarily broke
  mods ([The Game Post](https://thegamepost.com/how-to-mod-marvel-rivals-season-1-update-guide/));
  the game also validated container signatures, which the community defeated with an ASI
  `dsound.dll` patch ("UTOC Signature Bypass",
  [Nexus mod 2940 mirror](https://megagames.com/mods/marvel-rivals-utoc-signature-bypass-enable-mod-loading-mod-v1-0)).
  Current tooling (repak-rivals, Aug 2026) documents plain drag-into-`~mods` with no bypass
  step, but this cat-and-mouse can resume any patch — treat "mods load at all" as a moving
  target outside the manager's control. NetEase officially warns mods may be bannable.

## Key takeaways for the mod manager

1. Read mod contents from the `.utoc` directory index (retoc as git dep, or port the
   retoc-rivals fork); fall back to repak for single legacy `.pak` mods. Keep the public AES
   key on hand for "obfuscated" mods; never require user key input.
2. Identify hero/skin by matching `Characters/(\d{4})/(\d{7})/` in index paths.
3. Roster = bundled snapshot + optional Liquipedia API refresh.
4. Install = copy trio into `Paks/~mods`, enforce same-stem trio integrity and a `_<N>_P`
   suffix; manage conflict order via the numeric priority, not alphabetical accident.
