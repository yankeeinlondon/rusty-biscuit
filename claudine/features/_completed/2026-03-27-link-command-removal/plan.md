# Link Command Removal Plan

**Context**: [non-portable-assets.md](../../docs/topics/non-portable-assets.md)
**Target**: `claudine/cli/src/`, `claudine/lib/src/linking/`

---

## Rationale

The `link` command's derived artifact workflow converts Markdown frontmatter into TOML/YAML for providers that use non-Markdown formats (Gemini commands, Goose/Kimi/Roo agents). However, this conversion is purely a serialization format change — it does not translate property names, generate required properties, or convert value formats. The resulting files fail schema validation for every YAML-based provider:

- **Goose** requires `title` (not `name`), `extensions` (not `tools`)
- **KimiCode** requires `system_prompt_path` (no Claude equivalent) and `tools` as a structured YAML mapping (not a string)
- **RooCode** requires `slug`, `roleDefinition`, `groups` (no Claude equivalents)

The `skills --apply`, `agents --apply`, and `commands --apply` commands correctly handle the safe path (symlinks for Markdown-compatible providers) and correctly skip format-incompatible providers with `FormatIncompatible`. The `link` command's additional functionality is broken by design.

See the "Format Incompatibilities" and "Why Automated Format Conversion Is Not Viable" sections in `non-portable-assets.md` for the full analysis.

---

## Dependency Analysis

### CLI Layer — Link-Only

| File | What to Remove |
|------|---------------|
| `claudine/cli/src/commands/link.rs` | Delete entire file (911 lines) |
| `claudine/cli/src/commands/mod.rs` | Remove `pub mod link;` |
| `claudine/cli/src/args.rs` | Remove `Link` variant from `Commands` enum |
| `claudine/cli/src/main.rs` | Remove `Some(Commands::Link(args))` match arm |
| `claudine/cli/src/commands/help.rs` | Remove `link` entry from Shared Resources group |
| `claudine/cli/tests/link_integration.rs` | Delete entire file |

### Library Layer — `execution.rs` (Delete Entirely)

The entire `execution.rs` module is consumed exclusively by the `link` CLI command. No other code calls `analyze_resource_links()` or `apply_fixable_resources()`.

| Item | Visibility | Consumers |
|------|-----------|-----------|
| `analyze_resource_links` | pub | link.rs only |
| `apply_fixable_resources` | pub | link.rs only |
| `ApplySummary` | pub struct | link.rs only |
| `detect_snapshot` | private | execution.rs internal |
| `classify_target_state` | private | execution.rs internal |
| `classify_direct_state` | private | execution.rs internal |
| `classify_derived_state` | private | execution.rs internal |
| `apply_direct_link` | private | execution.rs internal |
| `apply_derived_write` | private | execution.rs internal |
| `render_derived_content` | private | execution.rs internal |
| `render_toml_derived` | private | execution.rs internal |
| `render_yaml_derived` | private | execution.rs internal |
| `render_markdown` | private | execution.rs internal |
| `derived_hashes_match` | private | execution.rs internal |
| `read_derived_hashes` | private | execution.rs internal |
| `read_toml_hashes` | private | execution.rs internal |
| `read_yaml_hashes` | private | execution.rs internal |
| `conversion_for_formats` | private | execution.rs internal |
| `DERIVED_FM_HASH_KEY` | const | execution.rs internal |
| `DERIVED_BODY_HASH_KEY` | const | execution.rs internal |
| ~27 additional private helpers | private | execution.rs internal |

### Library Layer — `model.rs` (Partial Removal)

Some `model.rs` types are shared with `compatibility.rs`. After removing `execution.rs`, the remaining consumers determine what stays.

| Type | Used By After Removal | Action |
|------|----------------------|--------|
| `ResourceDefinition` | compatibility.rs | **Keep** |
| `ResourceReference` | compatibility.rs | **Keep** but remove `DerivedLink`, `DerivedStale`, `DerivedMissing` variants |
| `IncompleteCause` | compatibility.rs | **Keep** but remove `NoConversionPath` and `UnexpectedSymlink` variants (only used by classify_derived_state) |
| `ResourceScope` (model-level) | compatibility.rs | **Keep** |
| `ReferenceStatus` | link.rs only | **Remove** |
| `Resource` (struct) | link.rs only | **Remove** |
| `ResourceFormatConversion` | execution.rs only | **Remove** |

### Library Layer — `compatibility.rs` (No Changes Needed)

`compatibility.rs` defines `classify_canonical_resource()` and `classify_target_reference()` which return `ResourceReference` variants. These functions are called only from `execution.rs` today, but they represent correct classification logic (non-portable property checks, required property validation) that other code may want in the future. They also have their own test suite. **Leave as-is** — they become unused but are not unsafe. Dead code warnings will naturally flag them if they stay unused long-term.

### Library Layer — `mod.rs` Re-exports

Remove from `claudine/lib/src/linking/mod.rs`:

```rust
// Remove this line:
pub use execution::{ApplySummary, analyze_resource_links, apply_fixable_resources};
```

Remove `mod execution;` declaration.

### Library Layer — `symlink.rs`

`category_link_target()` is used by:
- `execution.rs` (being removed)
- `mod.rs` in `link_skills()` function (shared, keep)
- `link.rs` CLI (being removed)

**Keep** the function in `symlink.rs` — it still has a consumer in `mod.rs`.

### External Dependencies

| Crate | Used By After Removal | Action |
|-------|----------------------|--------|
| `toml_edit` | execution.rs only | **Remove** from `claudine/lib/Cargo.toml` |
| `serde_yaml_ng` | compatibility.rs, config, composition | **Keep** |

---

## Phases

### Phase 1: Remove CLI `link` command

1. Delete `claudine/cli/src/commands/link.rs`
2. Remove `pub mod link;` from `claudine/cli/src/commands/mod.rs`
3. Remove `Link(commands::link::LinkArgs)` variant from `Commands` in `args.rs`
4. Remove `Some(Commands::Link(args)) => commands::link::run(args)` from `main.rs`
5. Remove `link` entry from help groups in `help.rs`
6. Delete `claudine/cli/tests/link_integration.rs`

### Phase 2: Remove `execution.rs` from library

1. Delete `claudine/lib/src/linking/execution.rs`
2. Remove `mod execution;` from `claudine/lib/src/linking/mod.rs`
3. Remove `pub use execution::{ApplySummary, analyze_resource_links, apply_fixable_resources};` from `mod.rs`

### Phase 3: Clean up `model.rs`

1. Remove `Resource` struct and its impls
2. Remove `ReferenceStatus` enum and its impls
3. Remove `ResourceFormatConversion` enum
4. Remove `DerivedLink`, `DerivedStale`, `DerivedMissing` variants from `ResourceReference`
5. Remove `NoConversionPath`, `UnexpectedSymlink` variants from `IncompleteCause`
6. Update `Display` impls, `status()` method, and any test code that references removed variants

### Phase 4: Remove `toml_edit` dependency

1. Remove `toml_edit` from `claudine/lib/Cargo.toml`
2. Verify no other module in `claudine/lib/` uses `toml_edit`

### Phase 5: Clean up re-exports and imports

1. Update `claudine/lib/src/linking/mod.rs` — remove any re-exports of deleted items
2. Grep across `claudine/` for any remaining references to removed types/functions
3. Update `claudine/cli/README.md` — remove `link` command documentation
4. Update the snapshot `wrap_commands__help_lists_wrapper_subcommands.snap` (will auto-update via `INSTA_UPDATE=always`)

### Phase 6: Build, test, lint

1. `cargo build -p claudine -p claudine-cli` — verify clean compilation
2. `cargo test -p claudine -p claudine-cli` — run tests, expect link_integration test to be gone
3. `cargo clippy -p claudine-cli --no-deps` — verify no new warnings from our changes
4. Manual: `claudine` (no args) — verify help no longer shows `link`
5. Manual: `claudine --help` — verify clap output no longer shows `link`

---

## Execution Order

| Step | Phase | Depends On | Description |
|------|-------|------------|-------------|
| 1 | 1 | — | Remove CLI command, args, match arm, help entry |
| 2 | 1 | — | Delete link integration test |
| 3 | 2 | 1 | Delete execution.rs, remove mod/re-exports |
| 4 | 3 | 3 | Clean up model.rs (remove dead types/variants) |
| 5 | 4 | 3 | Remove toml_edit from Cargo.toml |
| 6 | 5 | 1–5 | Grep for stale references, update docs/snapshot |
| 7 | 6 | 6 | Build, test, lint, manual verification |

Steps 1–2 are independent. Steps 3–5 are sequential (each depends on the prior removal compiling). Step 6 is a sweep after all deletions. Step 7 is final verification.

---

## What Is NOT Removed

- **`fix_missing_skills/agents/commands()`** — the safe symlink-creation path used by `skills --apply`, `agents --apply`, `commands --apply`. These correctly skip format-incompatible providers.
- **`compatibility.rs`** — non-portable property checks, required property validation, and classification functions. These are correct logic with their own test suite. They may have no callers after execution.rs removal but are not unsafe.
- **`category_link_target()`** in `symlink.rs` — still used by `mod.rs` link_skills function.
- **`serde_yaml_ng`** — still used by compatibility.rs, config, and composition modules.
- **`ResourceReference`, `ResourceDefinition`, `IncompleteCause`** — shared types still used by compatibility.rs. Only the derived-artifact-specific variants are removed.
