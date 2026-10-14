# Schema YAML Spellings Fixtures

Standalone SimplifiedSchema documents used by `tests/l1/schema_yaml_spellings.rs`.
Each fixture name is `{subject}.{presentation}.yaml`; files sharing a subject
mean the same schema.

| Presentation | Provenance |
| --- | --- |
| `conventional` | Hand-authored in the indented, single-line spelling the strict source locator has always accepted. The robustness matrix edits these seeds. |
| `serialized` | Exact `serde_yaml_ng::to_string` output for the conventional seed's value, which writes block sequences indentless. `serializer_captures_are_serde_yaml_ng_output` regenerates and compares it byte for byte. |
| `indentless` | Hand-authored: a block sequence at the same column as its parent key, where the serializer has no way to produce it (a mapping-value anchor) or to match the specification's example exactly. |
| `continued` | Hand-authored: plain scalars continued onto more-indented physical lines, in the type expression, a constraint list, a description, and a sequence item; with a trailing comment, a blank line, and a final line without a newline. |

Every non-conventional fixture places a sibling key after the list or the
continued value, so a locator that consumes too much loses that sibling.
Do not reformat these files: the whitespace is the subject under test.
