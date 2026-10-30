# simplified-schema Dependencies

| Crate | Why |
| --- | --- |
| `biscuit-file` | `SourceSpan`, the YAML and JSON5 parsers used by number admission, and the passive file-reference and glob grammar (`file-reference-syntax`); full resolution with the `file` feature. |
| `serde`, `serde_json` | The value model. `preserve_order`, `raw_value`, and `float_roundtrip` match Darkmatter: key order, number spellings, and exact floats are contracts. |
| `base64` | The literal-token payload codec. |
| `thiserror` | Error types. |
| `tracing` | Debug events from the expression scan. |
| `sniff` (optional, `file`) | Repository discovery for path-based schema factories. |

Dev-only: `proptest` (codec round trips), `biscuit-test-harness` and
`test-toolkit` (the test-layout gate).
