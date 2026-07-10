---
ready: false
agent: codex/default
created: 2026-07-09T07:15:10
implemented: true
---

# Schema Plus Review 1

Verdict: **not production ready**.

The core parser/converter work is in good shape for Level 1 schema behavior, and `just test` passes for `darkmatter`, `darkmatter-cli`, and `dmls`. I found two acceptance-criteria gaps that should be fixed before marking this ready.

## Findings

### High: `example()` validation does not use the annotated target or function signature

Spec requirement: `example(...)` must validate the common `example.yaml` envelope and then validate target-specific fields such as `parameters` against the annotated property or typed expression-function signature (`spec.md:61-65`, `spec.md:89-96`, `spec.md:426-429`).

Implementation currently validates only:

- the common envelope; and
- a hard-coded generic `parameter[]` shape: an array of one-key maps with `any` values (`darkmatter/lib/src/markdown/schemas/example.rs:61-80`, `darkmatter/lib/src/markdown/schemas/example.rs:142-180`).

That catches malformed parameter containers, but it cannot reject target drift. For example, an `example()` attached to a `date` property can still carry arbitrary `parameters`, and expression examples are not checked against a function's declared arity/parameter names/types or return shape. The plan entry calls this complete by equating "inherited target shape" with the generic `parameter[]` check, but the spec explicitly requires inheritance from the annotated target/signature.

Verification level: Level 1 is the correct tier for this requirement, but the present Level 1 tests assert only the generic parameter shape and fixture acceptance. Add negative tests where an example is well-formed at the envelope level but invalid for the annotated target/signature.

### High: DMLS effective-schema cache ignores imported/example file content

Spec requirement: `Name@fileref` / `Name@this` resolution must include cache invalidation on referenced-file changes (`spec.md:430-432`). The resolver does expose dependency edges for imports and examples (`darkmatter/lib/src/markdown/schemas/resolve.rs:59-64`, `darkmatter/lib/src/markdown/schemas/resolve.rs:378-380`, `darkmatter/lib/src/markdown/schemas/resolve.rs:821-823`), but DMLS's overlay schema cache keys only on the document text and schema config (`darkmatter/dmls/src/overlay/mod.rs:145-165`).

Result: in a live DMLS session, editing an imported type file or referenced example file can leave open documents using a stale `SchemaBundle` until the Markdown document text or config changes. That breaks the dependency-edge part of the schema-plus contract and can surface stale completions/diagnostics/hover in the editor.

Verification level: Level 1 is the correct tier. Add a DMLS overlay/cache test that opens a document whose `$schema` imports `Name@./types.yaml`, changes `types.yaml` without changing the document text/config, and verifies the next schema assembly sees the new type. Do the same for `example(...)` if example data is surfaced from the bundle.

## Test Rigor

Schema-plus has no terminal-rendered or OS-input behavior, so Level 1 is the appropriate verification level for the user-observable behavior in this feature. No Level 2 or Level 3 tests are required by the stated test taxonomy.

Current verification is broad for parsing, conversion, validation, and CLI smoke paths, but the strongest tests for the two findings above are missing or assert a weaker contract than the spec requires.

## Verification Run

Ran:

```sh
cd darkmatter
just test
```

Result:

- `darkmatter`: 5144 passed, 111 skipped
- `darkmatter-cli`: 541 passed, 71 skipped
- `dmls`: 268 passed, 30 skipped

