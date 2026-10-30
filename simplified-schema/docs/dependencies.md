# simplified-schema Dependencies

| Crate | Why |
| --- | --- |
| `biscuit-file` | `SourceSpan`, the YAML and JSON5 parsers used by number admission, and the passive file-reference and glob grammar (`file-reference-syntax`); full resolution with the `file` feature. |
| `serde`, `serde_json` | The value model. `preserve_order`, `raw_value`, and `float_roundtrip` match Darkmatter: key order, number spellings, and exact floats are contracts. |
| `base64` | The literal-token payload codec. |
| `thiserror` | Error types. |
| `tracing` | Debug events from the expression scan. |
| `sniff` (optional, `file`) | Repository discovery for path-based schema factories. |
| `jsonschema` | Compiles and runs the lowered JSON Schema (default features off, as in Darkmatter). |
| `fancy-regex`, `regex` | Pattern-key precedence needs lookaround (`fancy-regex`, already compiled through `jsonschema`); `regex` escapes and checks emitted patterns. |
| `indexmap` | Declaration-ordered maps in the AST and source maps. |
| `serde_yaml_ng` | The YAML value model schemas are parsed from. |
| `biscuit-hash` (`json`) | Schema-content hashes that key the validator and plan caches. |
| `chrono` (`std` only) | The date and time format checks; no clock. |
| `url` | The URL-scheme keyword check. |
| `renderable` | Markdown code spans in `SchemaError` messages. |
| `biscuit-terminal` (optional, `terminal`) | `SchemaError`'s terminal presentation. |

Dev-only: `proptest` (codec and grammar round trips), `biscuit-test-harness`
and `test-toolkit` (the test-layout gate, `manifest_dir!`), `tempfile`
(schema-file fixtures), `serial_test` (tests that read process environment),
`gix` (repository fixtures for resolution tests).
