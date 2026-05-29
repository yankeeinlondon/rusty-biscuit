---
ready: false
agent: codex
model: ""
---

# Review: Schema Validation in the Compose Pipeline (Iteration #4)

## Summary

The implementation matches the functional shape of the spec: schema validation is always-on, runs after effective frontmatter preparation and before interpolation/shell expansion, reuses `DarkmatterSchemas::validate`, supports library baseline schemas, validates recursive compose children after `set=` overlays, and includes baseline schemas in compose cache option hashing.

I did not find a functional implementation bug in the core compose path. The remaining blocker is verification rigor for the styled terminal contract. The feature should not be marked production-ready until the Level 2 test actually proves the schema error block styling that the spec promises.

## Findings

### High: Level 2 schema-error styling test does not actually verify all required styling

The spec requires the schema-validation `BlockError` to render a document `description:` line as `<i><dim>...</dim></i>` and to render each problem category label as `<red>missing|type|invalid</red>`.

The strongest schema-specific real-terminal test is `level2_schema_validation_block_renders_styled_link_and_bullet` in `darkmatter/cli/tests/level2_errors.rs:146`. It does not exercise a frontmatter `description:` at all: the `MISSING_REQUIRED_SCHEMA` fixture at `darkmatter/cli/tests/level2_errors.rs:112` contains only `$schema` and `spec`. That means the dim/italic description-line requirement is currently only covered by in-process rendering/snapshots, not by Level 2 real-terminal capture.

The same test's red assertion at `darkmatter/cli/tests/level2_errors.rs:183` is also too broad:

```rust
let has_red = frame.raw.contains("\x1b[31m")
    || frame.raw.contains("\x1b[91m")
    || frame.raw.contains("\x1b[0;31m")
    || frame.raw.contains("\x1b[38;5;1")
    || frame.raw.contains("\x1b[38;2;");
```

The final `\x1b[38;2;` branch accepts any 24-bit foreground color, not specifically red. A blue hyperlink, themed header, or any unrelated true-color foreground sequence could satisfy this assertion even if the `<red>` problem label stopped rendering red.

Required fix: add a schema Level 2 fixture with `description: ...`, assert the captured plain text includes that description, and assert the raw pane capture has italic and dim SGR for that line. Tighten the red assertion so true-color matches are red-specific, or avoid the broad true-color fallback and verify one of the actual red encodings emitted by the renderer/harness.

## Test Rigor Verification

| Requirement | Strongest observed verification | Assessment |
|---|---:|---|
| No-op when no `$schema` and no baseline | Level 1 | Adequate. |
| Document `$schema` honored for valid, missing, and wrong-type frontmatter | Level 1 | Adequate for validation behavior. |
| Baseline schema applies and merges with document schema, with document wins | Level 1 | Adequate. |
| `--set` / external state effective frontmatter is validated before later stages | Level 1 | Adequate for behavior; covered by pipeline placement and focused tests. |
| Compose fails before frontmatter shell expansion | Level 1 CLI + sentinel side effect | Adequate; no real terminal encoder behavior involved. |
| Recursive compose validates child after parent `set=` overlay | Level 1 | Adequate. |
| Baseline schema participates in transclusion cache keys | Level 1 hash + behavioral persistent-cache regression | Adequate. |
| `md compose` and `md schema validate` agree for document-level schemas | Level 1 CLI | Adequate. |
| Styled error source link and inverse property label | Level 2 WezTerm capture | Adequate for OSC8 and inverse styling. |
| Styled error red problem label | Level 2 attempted | Gap: assertion can pass on any true-color foreground sequence. |
| Styled description line is dim + italic | Level 1 only | Gap: no schema-specific Level 2 fixture exercises `description:`. |

No requirement needs Level 3; this feature has no OS keyboard/mouse input surface.

## Conclusion

Not ready for production under the requested rigor bar. The implementation appears functionally complete, but the styled terminal requirements are not yet verified at the appropriate level.
