---
phase: 1
---

# Unchanged import and artifact reproduction

On 2026-10-14, the unchanged execution tree reproduced the committed OpenAI definition and the complete generated Rust, OpenAPI, and Postman artifact sets byte for byte. Two independent temporary output trees also matched each other. No generic import transform, importer rewrite, artifact overwrite, or drift exemption was necessary.

## Pinned input and options

The import used the repository's vendored `schematic/specs/openai/openapi.yaml`, version 2.3.0 (OpenAPI 3.1.0), as documented in `schematic/specs/openai/README.md`. No download or network provider call was made. Options exactly matched `import-openai`, including excluded Assistants paths, the request-id header, and the generic authored-pagination overlay hook. The generator was built from the execution tree with Cargo's existing locked dependency resolution; no tools or dependency versions were changed.

Commands ran from `schematic/`:

```sh
cargo run --color=never -p schematic-gen -- import \
  --input specs/openai/openapi.yaml \
  --api-name OpenAI --module-path openai \
  --definitions-out /tmp/schematic-phase1-reproduction/openai \
  --exclude-path /assistants --exclude-path /threads \
  --request-id-header x-request-id \
  --overlay crate::openai_pagination::apply

cargo run --color=never -p schematic-gen -- generate --api all \
  --output /tmp/schematic-phase1-reproduction/schema/src \
  --openapi-out /tmp/schematic-phase1-reproduction/openapi \
  --postman-out /tmp/schematic-phase1-reproduction/postman -v
```

Both commands exited 0. Import reported 265 endpoints, 1,394 models, three warnings, and zero errors. Generation reported 18 APIs and successful Rust/OpenAPI/Postman publication. The known nine query-bearing endpoint paths emitted validation warnings; these remain unchanged migration inputs, not reproduction failures.

The same commands then ran with each output path placed beneath `/tmp/schematic-phase1-reproduction/repeated/`; both exited 0. The second generation's full console output was retained locally at `/tmp/schematic-phase1-reproduction/repeated-generate.log`.

## Comparisons

Comparison enumerated every regular file recursively, compared relative-path sets in both directions, and compared raw bytes for every matching path. It did not parse or normalize generated content.

| Committed reference | Temporary output | Files | File-set difference | Byte difference | Independent repeat |
| --- | --- | ---: | --- | --- | --- |
| `definitions/src/openai/` | `openai/` | 2 | None | None | Identical |
| `schema/src/` | `schema/src/` | 42 | None | None | Identical |
| `schema/Cargo.toml` | `schema/Cargo.toml` | 1 | None | None | Identical |
| `openapi/` | `openapi/` | 15 | None | None | Identical |
| `postman/` | `postman/` | 15 | None | None | Identical |

The two OpenAI files are `mod.rs` and `types.rs`. The generated module contains its single `crate::openai_pagination::apply(api)` call and byte equality preserves it. The importer destinations are isolated module directories; neither invocation writes `definitions/src/openai_pagination.rs`. The authored overlay and all committed artifacts were preserved.

The Rust comparison covers the complete generator-owned source tree, including shared and WebSocket files, plus the generated manifest. It intentionally excludes `schema/Cargo.lock`, `schema/tests/`, and build outputs: these are not artifacts written by `generate`.

## Classification and later regression coverage

There were no tool/version/options differences, nondeterministic differences, stale artifacts, or authored differences. There is no blocker and no proposed transform. Later phases should continue importing with these options, preserve the overlay, and run the existing `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` regression test together with the migration and artifact drift comparisons. The one-time investigation is complete.
