# Recursive Schema Golden Corpus

This corpus guards one promise: each frozen input keeps lowering to exactly the
same JSON Schema bytes, key order included, unless resolver code intentionally
changes the representation. The test is
`darkmatter/lib/tests/l1/recursive_schema_golden.rs`.

Acyclic output is not promised to stay byte-stable across such a change. The
resolver stores a cyclic named type, and a structured named type used at two
or more authored sites, once in `$defs` and points to it with a local `$ref`
at each use:

```json
{
  "properties": {
    "primary": { "$ref": "#/$defs/endpoint" },
    "backup": { "$ref": "#/$defs/endpoint" }
  },
  "$defs": {
    "endpoint": { "type": "object", "properties": { "host": { "type": "string" } } }
  }
}
```

Changing what is shared moves these goldens without changing what any schema
accepts. The behavior tests prove that the meaning is preserved; the goldens
only pin the bytes.

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

Regenerate only for an intentional representation change, after the behavior
tests pass, and record in that change which entries moved and why. A mismatch
from any other change is a regression:
the failure names the entry, the first differing line, and a temporary file
holding the actual output.

To add an entry, copy its input (and every file it imports) under `inputs/` at
its repository path, add it to `manifest.yaml`, and regenerate. The test fails
on an input file the manifest does not name, a golden no entry owns, and a
dependency outside `inputs/`.
