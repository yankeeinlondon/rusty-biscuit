# Build features, setter values, and boundary guards

Loaded from the `simplified-schema` skill.

## Property paths and setter values

`PropertyPath` is the only parser of the dotted grammar. It keeps decoded text
segments, because `0` is an index or a key depending on the value it meets, so
`PathStep` and `JsonPointer` come only from `resolve`'s `Resolved`, never from text.
`set_at` and `RemovalPlan` decide before changing anything and work on a frontmatter
`IndexMap`, not on source text.

`SetterValue::parse` sees only text; `bind` takes the `Shape` the host looked up.
`Shape::Any` is a declared shape, so text stays text under it; only `None` guesses. A
quoted string binds through the declared shape like text, and stays a string under `None`.
Syntax rejection is text, but a parsed container that fails admission is an error,
never text: `biscuit_file::json5_scan` finds duplicates and number spellings that
`serde_json::Value` has already lost, `numeric::admit_json5_text_numbers` judges
them, and `json5_is_well_formed` keeps an integer beyond `u64` from reaching the YAML
step as a key. A YAML list under `$schema` is a union of object arms, not an array,
which is a trap when writing fixtures; an array of objects is a named type with
`item[]`.

## Features

| Feature | Default | Adds |
|---|---|---|
| `file` | yes | File factories, `SchemaSource`, `repository_scope_catalog`; pulls `biscuit-file/file-reference` and `sniff` |
| `terminal` | no | `biscuit-terminal` `BlockError` rendering of `SchemaError` (Darkmatter enables it) |
| `work-counters` | no | Test-only `work_counters` module for complexity regressions |

Without `file`, the crate depends on `biscuit-file` with
`file-reference-syntax` only (no `gix`/`walkdir`/`sniff`); file-backed checks
fail with `FileReferenceError::ResolutionUnavailable` or
`ValidationFailure::UnavailableCheck`, never pass.

```toml
[dependencies]
simplified-schema = { path = "../../simplified-schema/lib" }                            # default
simplified-schema = { path = "../../simplified-schema/lib", default-features = false } # text only
```

## Boundary Guards

| Guard | Fails when | Satisfy it by |
|---|---|---|
| `simplified-schema/lib/tests/l1/context_construction_guard.rs` | Production code outside `schema/capture.rs` calls `std::env::current_dir`, `biscuit_file::home_dir`, `capture_env`, or `FileResolutionContext::from_snapshot` (besides the one empty `content_context` in `schema/layer.rs`), or adds a second optional context besides `SchemaSource`'s | Take a `FileResolutionContext` parameter; only extend `ALLOWLIST` (exact file, identifier, count, reason) for a genuinely new capture |
| `simplified-schema/lib/tests/l1/dependency_direction_guard.rs` | Any manifest table (dev included, followed through path deps) reaches `darkmatter`, `dmls`, `content-policy`, `claudine*`, `rendezvous-*`, `zed-dmls`, a `*-cli`, or a crate under those dirs; or source names one of their crates | Move the host-specific code to the host; invert the dependency |
| `simplified-schema/lib/tests/l1/path_lookup_guard.rs` | Raw `canonicalize` or home lookup | `biscuit_file::canonicalize_simplified`, `biscuit_file::home_dir` |
| `darkmatter/lib/tests/l1/schema_language_ownership_guard.rs` | Darkmatter lib, CLI, or DMLS production source *defines* (`fn`/`struct`/`enum`/`trait`/`type`/`union`) an owned name (`to_json_schema`, `is_subtype`, `parse_yaml_schema`, `SchemaRoot`, `Lexer`, `format_number`, `ConversionError`, ...); or the listed library file no longer defines a name | Call the library instead of re-implementing; when renaming or moving an owned item in the library, update the guard's owner table and Darkmatter's `[package.metadata.ci.tests] source-inputs` |
| `darkmatter/lib/tests/l1/glob_implementation_guard.rs` | `simplified-schema/lib/src` reaches a glob library directly or its manifest names one | Match through `biscuit_file::GlobReference` only |

The guards reuse Darkmatter's shared engines by `#[path]`
(`darkmatter/cli/tests/common/{context_guard,path_lookup_guard,source_scan}.rs`,
`darkmatter/lib/tests/common/dependency_graph.rs`); they are declared in this
crate's `source-inputs`, so editing them schedules this crate's tests.

