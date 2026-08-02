# OpenAI OpenAPI Specification

Vendored source for `schematic-definitions`'s `openai` module.

| Field | Value |
|-------|-------|
| Source | <https://github.com/openai/openai-openapi> (`master`, `openapi.yaml`) |
| Spec version | `2.3.0` (OpenAPI 3.1.0) |
| Retrieved | 2026-07-31 |

## Regenerating

```bash
just -f schematic/justfile import-openai
```

The Assistants family (`/assistants`, `/threads`) is excluded: it is superseded
by the Responses and Conversations APIs and slated for removal by OpenAI.

## Known spec quirks

`CreateChatCompletionRequest.seed` declares its bounds as `-9223372036854776000`
/ `9223372036854776000` — the `f64` roundings of `i64::MIN` / `i64::MAX`, which
no Rust integer type can hold. The importer clamps them back into range and
reports a warning; see `clamp_numeric_bounds` in
`schematic/define/src/openapi/import/normalize.rs`.
