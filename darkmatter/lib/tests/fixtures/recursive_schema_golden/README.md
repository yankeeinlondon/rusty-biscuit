# Recursive Schema Golden Corpus

This corpus guards one promise: a schema with no cyclic named types lowers to
exactly the same JSON Schema bytes, key order included, before and after the
resolver learns to lower cyclic named types to `$defs`/`$ref`. The test is
`darkmatter/lib/tests/l1/recursive_schema_golden.rs`.

| Path | Holds |
| --- | --- |
| `manifest.yaml` | Every entry (`id`, `kind`, `input`, `source`), the `imports_only` files an entry reads, and each `skipped` input with its reason |
| `inputs/` | The frozen inputs, at their repository-relative paths, so relative references still resolve; `inputs/authored/` holds inputs written for this corpus |
| `golden/<id>.json` | The lowered schema, `serde_json::to_string_pretty` plus a newline |
| `golden/<id>.digest` | For a schema whose compact serialization exceeds 128 KiB: the length and BLAKE3 digest of that compact text |

The inputs are byte-for-byte copies taken on 2026-10-23 at commit `e5976e3f0`.
They are deliberately not kept in step with the files they were copied from:
the goldens must move only when resolver code changes, never when a shipped
schema is edited. Darkmatter's compiled-in baseline and the reserved
`@darkmatter:` partials are the exception; they are read live from the library.

## Regenerating

```sh
DARKMATTER_BLESS_RECURSIVE_GOLDEN=1 cargo nextest run -p darkmatter \
    --features effects-instrumentation --test l1 -E 'test(recursive_schema_golden)'
```

Regenerate only for a change that is meant to alter how acyclic schemas lower,
and say so in that change. A mismatch from any other change is a regression:
the failure names the entry, the first differing line, and a temporary file
holding the actual output.

To add an entry, copy its input (and every file it imports) under `inputs/` at
its repository path, add it to `manifest.yaml`, and regenerate. The test fails
on an input file the manifest does not name, a golden no entry owns, and a
dependency outside `inputs/`.
