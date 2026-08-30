# Archive extraction options for Rust (zip, 7z, rar)

Research for wayfinder ticket #6. Question: how should the mod manager extract mod
archives (zip, 7z, rar) cross-platform, including listing contents and extracting
selected files without full extraction (variant choice, preview images)?

Facts verified 2026-08-30 against crates.io metadata, docs.rs API docs, project
READMEs, and license texts. Sources linked per section.

## Requirements recap

- Formats: zip, 7z, rar (the three formats GameBanana mods actually ship in).
- Partial access: list entries, extract a single named entry (preview image, one
  variant's `.pak`) without unpacking the whole archive.
- Windows, macOS, Linux. Prefer no system-package prerequisites for end users.
- License compatible with this repo (MIT OR Apache-2.0, enforced via `deny.toml`).

## Per-format crates (pure Rust / vendored, no system deps)

### zip (zip-rs/zip2)

- Latest: 8.6.0 stable (Apr 2026), 9.0.0-pre3 (Aug 2026). Actively maintained;
  this is the continuation fork ("zip2") of the original zip-rs crate, published
  under the same `zip` crate name. ~65M recent downloads.
  Source: <https://crates.io/crates/zip>, repo <https://github.com/zip-rs/zip2>.
- License: MIT.
- Pure Rust. Optional features cover deflate, bzip2, zstd, LZMA/XZ, PPMd, and
  AES encryption — enough for anything mod sites produce.
- Partial access: first-class. `ZipArchive` reads the central directory, then
  `file_names()`, `len()`, `by_name()`, `by_index()` give random access to
  individual entries; extracting one entry never touches the others.
  Source: <https://docs.rs/zip/latest/zip/read/struct.ZipArchive.html>.
- Platforms: all Rust tier-1 targets, no native deps.

### sevenz-rust2 (7z)

- Latest: 0.22.2 (Aug 2026), actively maintained. Fork of the original
  `sevenz-rust`, which is formally unmaintained (RustSec advisory filed;
  marked unmaintained as of Aug 2026). Use `sevenz-rust2`, not `sevenz-rust`.
  Sources: <https://crates.io/crates/sevenz-rust2>,
  <https://github.com/hasenbanck/sevenz-rust2>, <https://rustsec.org/advisories/>.
- License: Apache-2.0.
- Pure Rust decompressor/compressor. Codecs: COPY, LZMA, LZMA2, bzip2, deflate,
  PPMd, brotli, LZ4, zstd (some behind features), BCJ/delta filters, AES
  encryption.
- Partial access: yes, with a caveat. `ArchiveReader::archive()` lists entries
  from metadata without decompressing; `read_file(name)` extracts one entry;
  `for_each_entries` iterates. Caveat: for **solid** 7z archives (the 7-Zip
  default), extracting entry N requires decompressing all data before it in the
  same solid block — docs.rs explicitly calls `read_file` "very inefficient" on
  solid archives. Listing is always cheap; selective extraction is only cheap
  for non-solid archives.
  Source: <https://docs.rs/sevenz-rust2/latest/sevenz_rust2/struct.ArchiveReader.html>.
- Platforms: pure Rust, all targets (WASM feature exists).

### unrar (rar)

- Latest: 0.5.8 (Feb 2025). Maintained at a slow cadence (bindings track the
  upstream unrar library; low churn is normal for it).
  Source: <https://crates.io/crates/unrar>, repo <https://github.com/muja/unrar.rs>.
- Extraction/listing only — cannot create archives (which is exactly the shape
  the UnRAR license allows).
- Linking: `unrar_sys` vendors RARLAB's unrar C++ source and builds it with `cc`
  at compile time — static, no system library or DLL to ship. Works on Windows,
  macOS, Linux.
- License: the Rust wrapper code is MIT OR Apache-2.0, but the vendored C++
  library carries RARLAB's **UnRAR license**: free of charge, "may be used in
  any software to handle RAR archives without limitations", redistribution
  permitted — but it may NOT be used to re-create the RAR compression
  algorithm or develop a RAR-compatible archiver, and the license paragraph
  must be reproduced in the shipping package's docs/license file. This is a
  source-available freeware license, not OSI open source; `cargo deny` will
  need an explicit exception, and the app's third-party-licenses output must
  include the UnRAR text. Extraction-only use in this app is squarely within
  the permitted use.
  Source: <https://github.com/muja/unrar.rs> README and
  `unrar_sys/vendor/unrar/license.txt`.
- Partial access: listing via iterator (`List` mode); extraction via `Process`
  mode, which walks entries sequentially and lets you skip entries, extract to
  disk, or read one entry's bytes into memory. No random access and no
  `Read`-stream input (RAR is inherently stream-oriented; solid RARs have the
  same preceding-data dependency as solid 7z). Skipping headers to reach one
  entry is still cheap for non-solid archives.

## Whole-family alternatives

### libarchive bindings: compress-tools

- `compress-tools` 0.16.1 (Apr 2026), MIT OR Apache-2.0, by OSSystems.
  Repo: <https://github.com/OSSystems/compress-tools-rs>.
- Wraps **system** libarchive (>= 3.2.0) via pkg-config (vcpkg on Windows
  MSVC). libarchive itself is BSD; it reads zip, 7z, and RAR/RAR5 (RAR is
  read-only in libarchive).
  Sources: README above; <https://github.com/libarchive/libarchive/wiki/LibarchiveFormats>.
- API covers listing, whole-archive extraction, and extracting one named file.
- Drawback that disqualifies it here: it does not vendor libarchive. End users
  (or the packaging step) must supply libarchive per platform — brew/apt are
  fine for CI, painful for a Windows-first game-mod audience. The older raw
  `libarchive` binding crates are stale. Only worth revisiting if we ever want
  one code path for many exotic formats.

### Bundling the 7-Zip binary (7zz / 7z.exe)

- Official 7-Zip console builds: `7zz` for Linux/macOS, `7z.exe` on Windows.
  One tool lists (`l`), tests, and extracts (`x`/`e`, with per-file selection
  and `-so` stdout streaming) zip, 7z, and rar — RAR extraction is built in.
- License: GNU LGPL 2.1+ for most code, BSD 3-clause/2-clause for LZFSE, zstd,
  and xxhash parts, plus the same unRAR restriction for the RAR engine. Binary
  redistribution is permitted provided the license text is reproduced; no
  payment or registration. Bundling the unmodified binary alongside the app
  (mere aggregation, invoked as a subprocess) does not LGPL-infect the app.
  Source: <https://www.7-zip.org/license.txt>.
- Drawbacks: ~3 per-platform binaries to fetch, verify, and ship; subprocess
  output parsing instead of typed APIs; per-entry partial reads mean spawning
  processes; antivirus/quarantine friction on Windows/macOS for a bundled
  executable. Works, but strictly worse ergonomics than in-process crates for
  an app that already accepts the unrar licensing note.

## Shortlist and recommendation

Recommended: **three format-specific crates behind one internal trait** —
`zip` (MIT) + `sevenz-rust2` (Apache-2.0) + `unrar` (MIT/Apache wrapper over
UnRAR-licensed vendored C++). Rationale:

1. Zero runtime prerequisites on all three OSes (everything static/pure Rust).
2. All three support listing without extraction and extracting selected
   entries — the variant-choice and preview-image flows work, with the known
   caveat that solid 7z/rar archives pay sequential decompression cost for a
   deep single entry (acceptable: previews are usually small archives, and
   full install extracts everything anyway).
3. Licensing is clean for MIT/Apache app code; the only action item is a
   `deny.toml` exception for the UnRAR license plus shipping its notice text.

Fallbacks: `compress-tools`/libarchive if we ever need many formats and accept
a system dependency; bundled `7zz` if a native-code CVE in unrar ever forces
dropping in-process RAR.

Open follow-ups (out of scope here): exact `deny.toml` exception syntax for
`unrar_sys`, and whether GameBanana 7z mods are commonly solid-compressed
(affects preview-image latency, not correctness).
