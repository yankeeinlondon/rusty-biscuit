---
name: biscuit-file
description: >
  Expert knowledge for the biscuit-file Rust library and CLI (`bf`) providing format conversion (TOML/YAML/JSON/JSON5), PDF extraction, file type detection, and file reference resolution. Use when working in the `biscuit-file/` package area, using biscuit-file types (Toml, Yaml, Json5, Pdf, FileReference, FileResolutionContext, PortablePath, PortabilityPreference, PortablePathError, BaseDirOrigin, PathPosition, MagicPathTier, LaunchMagicScope, GlobReference, GlobReferenceError, GlobListing, SkippedEntry, FileType, DataFormat, FileReferenceError), adding the biscuit-file dependency, implementing file resolution, resolving file references, writing portable links or cleaning up relative links, converting between data formats, extracting PDF content, reading markdown frontmatter, or detecting file types.
---

- Read [references/format-conversion.md](references/format-conversion.md) when converting between TOML, YAML, JSON, JSON5, or PDF formats
- Read [references/file-references.md](references/file-references.md) when resolving file references (`@`, `&`, `^`, `vault:`, `%`, `{{ENV}}`), including the local-before-home `@` tiers, launch `@` scope, and `GlobReference` (native order, `take_first`, `GlobReferenceError`, `SkippedEntry`)
- Read [references/cli.md](references/cli.md) when working with the `bf` CLI tool
- Read [references/detect.md](references/detect.md) when using file type detection (`detect_file_type`, `detect_file_type_from_bytes`, `FileType`)
- Read [references/api.md](references/api.md) for full API surface of all types (Toml, Yaml, Json5, Pdf, FileReference, FileType, DataFormat, detect, PdfConfig)
- Rendering a `Path` as text? Use `biscuit_file::to_portable_string` / `try_portable_string` (unfeatured) — never write another `.replace('\\', "/")`. See [references/api.md](references/api.md#portable-path-text)
- Canonicalizing a path whose result leaves a private comparison (returned, stored, shown, hashed into a shared key, reparsed)? Use `biscuit_file::canonicalize_simplified` (unfeatured), never `std::fs::canonicalize`, `.canonicalize()`, or `dunce::canonicalize`. In biscuit-file, Claudine, and Darkmatter (lib and CLI) a source guard fails on any other direct call unless it is listed, by file, enclosing item, and operation, as a private-comparison exception in that package's `tests/l1/path_lookup_guard.rs` (biscuit-file-cli: `tests/cli_tests.rs`); the engine is `darkmatter/cli/tests/common/path_lookup_guard.rs`
- Need the user's home? Call `biscuit_file::home_dir()` (`file-reference`), qualified: `HOME` on POSIX, `USERPROFILE` on native Windows (`HOME` alone does not relocate it), relative home → `None`, never canonicalized. Never `dirs::home_dir` / `std::env::home_dir`; Claudine's guard rejects them, and a bare `home_dir()` imported from elsewhere is an unresolved candidate. Request-scoped code captures it once and passes it down (`PolicyContext`, `SensitivePathChecker::home_dir()`); a new read also needs a `biscuit_file::home_dir` entry in that package's `context_construction_guard.rs`
- Comparing paths (`starts_with`, relative route)? Use `biscuit_file::PathIdentity` — the one lexical, lossless, Windows-aware identity. See [references/api.md](references/api.md#path-identity)
- Context has two directories: `cwd()` (where `./`, `../`, bare start) and `base_dir()` (tree root + relative boundary; read `base_dir_origin()` / `base_dir_is_boundary()`, never compare it with `cwd()`). Reader opt-in: `allow_external_relative()`. See [references/file-references.md](references/file-references.md)
- Matching a **set** of files with a prefix plus glob (`^**/*spec*.md`, `!&**/_completed/**`)? Use `biscuit_file::GlobReference` (`list_files`, `take_first`, `matches`, `roots`; `lists_file` for a caller that walks `roots` itself with its own filters and must offer exactly what `list_files` would — `matches` judges a file symlink where it sits, `lists_file` also drops one whose target leaves the tree; `matches_without_context` for a caller with no request, which judges an absolute path by bare and absolute patterns only) — the only prefix-plus-glob implementation; never split a sigil from a glob or call `globset` yourself. `FileReference` never reads glob syntax. See the "Glob References" section of [../docs/topics/file-references.md](../../biscuit-file/docs/topics/file-references.md)
- Writing a link to a file (portable `./`, `&`, `~`, `{{VAR}}`, absolute)? Use `biscuit_file::PortablePath` — never hand-roll a relative or env-rooted spelling. See [references/api.md](references/api.md#portable-references-portablepath)

- Read [references/architecture.md](references/architecture.md) for module layout, feature flags, and re-exports

- Read [../docs/topics/file-references.md](../../biscuit-file/docs/topics/file-references.md) for the authoritative file reference topic doc
- Read [../docs/dependencies.md](../docs/dependencies.md) for dependency versions reference

 |

## Package Area Structure

```
biscuit-file/
├── lib/                    # Core library (biscuit-file crate)
├── cli/                    # CLI binary `bf`
├── docs/                   # Documentation
```

Optimization info: effective when targeted: `biscuit-file`, `biscuit-file lib`, `bf`, `FileReference`, `Toml`, `Yaml`, `Json5`, `Pdf`, `FileType`, `DataFormat`, `detect_file_type`
 in code or docs

 | CLI help | installed binary

 | Key Commands: `just build`, `just lint`, `just test`, `just install` (run from package area root)
 | Test from anywhere in the package area via `just test` (no subcommand needed)
  | The `bf` binary is installed to `~/.cargo/bin/`
  Key source: `src/lib/mod.rs` (public API surface) and `src/file_reference/mod.rs` (internal + re-exports)
  Source file | Key Types Exported |
  ---|---|---|---|---|---|---|---|---|---|---|---|---|---|
  `lib.rs` | `Toml`, `TomlError`, `TomlSource` | `yaml/mod.rs` | `Yaml`, `YamlError`, `YamlSource` | `json5/mod.rs` | `Json5`, `Json5Error`, `Json5Source` | `json5/format.rs` | `to_json5_pretty`, `to_json5_compact` | `pdf/mod.rs` | `Pdf`, `PdfConfig`, `PdfError` | `pdf/types.rs` | `PdfMarkdown`, `PdfToc`, `PageRange`, etc. |
  `detect.rs` | `FileType`, `detect_file_type`, `detect_file_type_from_bytes` | `format.rs` | `DataFormat` | `error.rs` | `BiscuitFileError` | `file_reference/mod.rs` | `FileReference`, `PathPosition` | `FileReferenceError` |
  `file_reference/parse.rs` | Parsing logic (internal) | `file_reference/resolve.rs` | Resolution logic (internal) | `file_reference/context.rs` | `FileResolutionContext`, `BaseDirOrigin`, `LaunchMagicScope` | `file_reference/portable/` | `PortablePath`, `PortableReference`, `PortabilityPreference`, `PortablePathError`, `PathIdentity` |

## Feature Flags

See [Cargo.toml](../../Cargo.toml) for the full list. Key defaults: `toml`, `yaml`, `json5`, `extract`, `lopdf`, `file-reference`.

To add: `schema` (JSON Schema validation) or `pdfium` (high-fidelity PDF). Use `full` in enable everything.

In `Cargo.toml`:
```toml
[dependencies]
biscuit-file = { path = ".../biscuit-file/lib", features = ["toml", "yaml", "file-reference"] }
```

## Code Conventions
- **No `unwrap()` or `expect()` in production code paths**
- Library code uses `thiserror` for error types; CLI uses `color-eyre`
- Use `tracing` for instrumentation (already integrated)
- `assert!` for assertions in tests only
- Avoid explicit `# Heading` (H1) inside `///` docblocks; use `## Heading` (H2) for sections
- Recommended section order: summary, `## Examples`, `## Returns`, `## Errors`, `## Panics`, `## Safety`, `## Notes`
- When adding new deps: `biscuit-file` uses existing version of `infer` crate for file type detection
- When adding docs, update READMEs, Cargo.toml, docs/dependencies.md, and skill files in sync
- Do not create documentation files unless explicitly requested
- `just` runner commands: `just build`, `just lint`, `just test`, `just install` always from package area root.
- Test runner uses `just test` (or underlying `cargo test`/`cargo nextest`).
- Wiremock for HTTP mocking; tempfile for temp dirs; serial_test for test isolation. Re-exported types from `biscuit-file` (e.g., `biscuit_file::serde_yaml_ng::Value`) avoid direct crate dependencies.
- Check `docs/dependencies.md` before adding deps to confirm availability and version.
