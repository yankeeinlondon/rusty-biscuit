# schematic-gen

Generate typed Rust REST and WebSocket clients, OpenAPI documents, Postman
collections, and AsyncAPI documents from Schematic definitions. REST clients send through
`schematic-runtime`; provider models and ordered registries come from
`schematic-definitions`.

```sh
cargo install --path schematic/gen
schematic-gen generate --api all --output schematic/schema/src
schematic-gen validate --api github
schematic-gen export-asyncapi --output schematic/asyncapi
```

Use [definition builders](../docs/definition-builders.md) and
[the compiling authoring example](../docs/authoring-examples.md) for new APIs.
Build endpoints before the API borrows their shared `DefinitionContext`.

Read [the generator and client reference](../docs/generation/generated-clients.md),
[the generation workflow](../docs/generation/workflow.md),
[OpenAPI import](../docs/io/import-openapi.md),
[OpenAPI export](../docs/io/export-openapi.md),
[Postman export](../docs/io/export-postman.md),
[AsyncAPI export and import](../docs/io/asyncapi.md), and
[definition bundles](../docs/io/definition-bundles.md) for commands and output contracts.
Every API the generator knows is one entry in the definitions crate's
registration inventory; see [Register an API](../docs/generation/workflow.md#register-an-api).
Generated artifacts are rebuilt through the generator and never hand-edited.

Run `just test`, `just lint`, and `just doctest` in `schematic/`.
Use `just test-schema` and `just lint-schema` for the generated crate.
