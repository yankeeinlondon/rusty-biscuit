# Output schema conformance inventory

Each row is a schema file under `darkmatter/cli/schemas/output/` and how many `include_str!` uses of it the CLI L1 suite holds. `every_leaf_has_a_schema_file_and_a_conformance_test` keeps this complete against the clap definitions.

| Schema | Uses in conformance tests |
|---|---|
| `clean.yaml` | 5 |
| `code-block.yaml` | 3 |
| `compose.yaml` | 13 |
| `delta.yaml` | 3 |
| `edit.yaml` | 5 |
| `get.yaml` | 6 |
| `graph.yaml` | 4 |
| `hash.yaml` | 9 |
| `render.yaml` | 5 |
| `rm.yaml` | 3 |
| `schema-about.yaml` | 3 |
| `schema-detect.yaml` | 6 |
| `schema-triggers.yaml` | 4 |
| `schema-validate.yaml` | 5 |
| `set.yaml` | 4 |
| `toc.yaml` | 5 |
| `validate-refs.yaml` | 6 |
