# Schema Enhancements Matrix Fixtures

The unedited positive controls for `tests/l1/schema_enhancements_matrix/`.
Every cell in that matrix starts from one of these files and makes exactly
one edit. Do not reformat them: the bytes are what the named producer wrote.

| Fixture | Producer | How it was obtained |
| --- | --- | --- |
| `timing.shipped.yaml` | Shipped SimplifiedSchema (tagged `kind`/`types` envelope) | Byte-for-byte copy of `darkmatter/schemas/partials/timing.yaml` at `7e6270793`. |
| `slash-commands.shipped.yaml` | Shipped SimplifiedSchema (pure `$schema` envelope) | Byte-for-byte copy of `claudine/docs/research/slash-commands/_schema.yaml` at `7e6270793`. |
| `detected.schema.json` | `md schema detect --format json` | Run from `darkmatter/lib/tests/fixtures/detect/`: `md schema detect --format json --merge basic_scalars/inputs/doc.md arrays/inputs/doc.md`, stdout captured unchanged. It is also exactly `serde_json::to_string_pretty` output plus a newline; the test asserts this, so a decoded edit re-serializes to a one-value diff. |
| `detected.schema.yaml` | `serde_yaml_ng::to_string` of `detected.schema.json` | Serializer output. No tool emits raw JSON Schema as YAML, so the YAML raw-schema format reuses the detected value through the serializer the YAML reader pairs with. The test asserts the bytes. |
| `keywords.schema.json` | **Hand-written** | Covers the keywords `detect` does not emit (`$ref`, `$defs`, `definitions`, `$id`, `$anchor`, `$dynamicRef`, `$dynamicAnchor`, `$vocabulary`, the applicators, bounds, counts, `pattern`, `format`, and the rest), one property per keyword so each removal is observable. Written in `serde_json::to_string_pretty` form; the test asserts this. |
| `keywords.schema.yaml` | `serde_yaml_ng::to_string` of `keywords.schema.json` | Serializer output; the test asserts the bytes. |
| `frontmatter.md-get.yaml` | `md get --yaml` | `md get source.md '$schema' n maybe zip flag values meta --yaml` over a document declaring those properties; stdout captured unchanged. The producer quotes `'02134'` and leaves `On` plain. The test wraps it in `---` fences above a fixed body. |
| `frontmatter.schema.yaml` | **Hand-written** | The SimplifiedSchema the frontmatter control declares: one property per archetype (required and optional number, numberlike and boolish text, typed list, typed object). |
