# schematic-gen

Generate typed Rust REST and WebSocket clients, OpenAPI documents, and Postman
collections from Schematic definitions. REST clients send through
`schematic-runtime`; provider models and ordered registries come from
`schematic-definitions`.

```sh
cargo install --path schematic/gen
schematic-gen generate --api all --output schematic/schema/src
schematic-gen validate --api github
```

Use [definition builders](../docs/definition-builders.md) and
[the compiling authoring example](../docs/authoring-examples.md) for new APIs.
Build endpoints before the API borrows their shared `DefinitionContext`.

Read [the generator and client reference](../docs/generation/generated-clients.md),
[the generation workflow](../docs/generation/workflow.md),
[OpenAPI import](../docs/io/import-openapi.md),
[OpenAPI export](../docs/io/export-openapi.md), and
[Postman export](../docs/io/export-postman.md) for commands and output contracts.
Generated artifacts are rebuilt through the generator and never hand-edited.

Run `just test`, `just lint`, and `just doctest` in `schematic/`.
Use `just test-schema` and `just lint-schema` for the generated crate.
