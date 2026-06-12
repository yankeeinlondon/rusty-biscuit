---
ready: true
agent: codex
model: ""
---

# Review: Poor Error Report Fix

## Findings

No blocking findings.

## Test Rigor

All user-observable requirements in this spec are schema-validation diagnostics and Markdown error rendering. Level 1 verification is appropriate; no Level 2 or Level 3 terminal coverage is required because the feature does not depend on terminal emulator rendering, input encoding, key handling, paste, IME, mouse behavior, or scroll behavior.

The implementation now has Level 1 coverage for the required behavior:

- parse failures produce `is not a valid file reference`;
- resolution failures, including unset env vars and unconfigured vault roots, produce `could not resolve file reference`;
- missing relative, absolute, magic, package, and recursive references produce `no existing file matched reference` without fabricated candidate paths;
- existing files validate successfully;
- `file(match(...))` emits one glob diagnostic for existing mismatches and one file-reference diagnostic for missing files;
- `x-darkmatter-match` without `format: darkmatter-file` fails schema construction;
- direct JSON Schema `format: darkmatter-file` receives the improved message;
- unrelated format failures retain the upstream `jsonschema` message;
- nested paths, array paths, and root-union arm attribution are retained;
- rendered messages escape markup-sensitive text.

The previous CWD-race concern has been addressed by using the shared `serial_test` key for CWD-mutating schema tests in both `format.rs` and `validate.rs`.

## Notes

The implementation matches the spec shape: `format::resolve_file_reference` centralizes parse, resolution, and no-match classification; `validate::build_problem` substitutes only `darkmatter-file` format failures and falls back to the upstream message otherwise; `x-darkmatter-match` now validates only glob constraints after a file has resolved.

## Verification

I ran:

```text
cargo test -p darkmatter markdown::schemas --color=never
cargo test -p darkmatter --test error_snapshots schema_validation_format_failure --color=never
cargo test -p darkmatter --test schemas_validate_table validate_table --color=never
```

Results: all passed.
