# Phase 5 Baseline Replay

The Phase 1 captures were replayed after the shipped base-schema migration.
The observed differences are the intentional semantic metadata changes from
the completed phases; representative unrelated behavior remains unchanged.

| Phase 1 capture | Phase 5 replay evidence | Result |
|---|---|---|
| Compiled representative schema (`title`, nested `metadata`) | `base_schema_end_to_end` (14/14) plus the Phase 3/4 exact conversion tests | The representative ordinary-type lowering remains unchanged. The shipped `$schema` property alone now adds the `x-darkmatter-schema` fragment. |
| DMLS hover on `$schema` | `lsp_session::meta_schema_phase1_schema_hover_uses_nominal_type` through a real initialize/open/hover/shutdown session | The intentional metadata correction is present: `Type: **schema**` replaces `Type: **any**`; the revised description no longer presents raw JSON Schema as an inline mapping form. |
| `md schema about` | `schema_about` integration binary (14/14) | Existing reference output remains valid, with the intentional Phase 3 addition of the two semantic meta-type rows. |
| `md schema validate` using `payload: any` | Full Darkmatter-area Level-1 gate, including CLI schema-validation suites | The unrelated `any` document-property contract remains valid; the migration is scoped to the base schema's reserved `$schema` property. |

Focused replay commands:

```text
cargo nextest run -p darkmatter --test meta_schema_phase1 -E 'test(=shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance)'
cargo nextest run -p dmls --test lsp_session -E 'test(=meta_schema_phase1_schema_hover_uses_nominal_type)'
cargo nextest run -p darkmatter --test base_schema_end_to_end
cargo nextest run -p darkmatter-cli --test schema_about
```

All focused replays passed. No unrelated baseline difference was observed.
