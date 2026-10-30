# Cargo Features

| Feature | Default | What it adds |
| --- | --- | --- |
| `file` | yes | Full file-reference resolution (`biscuit-file/file-reference`) and repository discovery through `sniff`, for the file factories (`SimplifiedSchema::try_from_yaml`, `try_from_markdown`, `try_from_json_schema`, `resolve`), `SchemaSource`, and `repository_scope_catalog`. |
| `terminal` | no | The terminal presentation of `SchemaError` (its `biscuit-terminal` `BlockError` implementation). A host that renders schema errors in a terminal enables it; Darkmatter does. |
| `work-counters` | no | Test-only instrumentation: the process-wide `work_counters` module (for example, how many expressions were parsed), read by complexity regressions. Darkmatter's `work-counters` feature enables it and folds the counts into `darkmatter::work_counters`. |

With `default-features = false` the crate works on supplied text only:
`SimplifiedSchema::try_from(text)` parses and validates in-memory data, and
references are parsed but never resolved. Its
dependency graph then holds no `gix`, `walkdir`, or `sniff`; it still holds
`globset`, because glob patterns written in a schema are checked while the
schema is parsed, in every build. `globset` is a passive syntax dependency,
not a resolver dependency: it validates `match(...)` pattern text and
performs no I/O.

Without `file`, nothing that needs a file on disk is judged to pass. A
`$schema` file reference, a named-type import from a file, or an
`example(...)` file fails with `SchemaError::Unresolved` whose source is
`FileReferenceError::ResolutionUnavailable`; a `file(eager)` value and a
`match(...)` glob never accept a value, and `validate_strict` /
`validate_soft` report such a failure as
`ValidationFailure::UnavailableCheck` (see
[Strict and Soft Validation](./validation.md)):

```rust
// Without `file`: `./types.yaml` is not resolved, it fails.
let error = resolve_yaml_schema(&reference, dir, &context, ShippedSources::NONE).unwrap_err();
assert!(matches!(
    error,
    SchemaError::Unresolved { source: FileReferenceError::ResolutionUnavailable { .. }, .. }
));
```

```toml
# Text-only build
simplified-schema = { path = "../../simplified-schema/lib", default-features = false }
```

`just test` and `just lint` in this area run the default build with
`work-counters` and `terminal` and the text-only build; `just lint` also lints the default
build alone.
