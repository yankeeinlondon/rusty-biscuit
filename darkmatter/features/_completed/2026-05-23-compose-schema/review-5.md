---
ready: false
agent: claude
model: ""
---

# Review: Schema Validation in the Compose Pipeline (Iteration #5)

## Summary

The compose-pipeline schema-validation stage continues to be functionally complete and faithful to the spec:

- Always-on stage placed after `--set`/`--state` overrides and before frontmatter interpolation / shell expansion (`darkmatter/lib/src/markdown/compose/mod.rs`).
- Reuses `DarkmatterSchemas::validate` via `schema_validation::run` (`darkmatter/lib/src/markdown/compose/schema_validation.rs`).
- `ComposeOptions::with_baseline_schema(...)` exposed and participates in `options_hash` (`darkmatter/lib/src/markdown/compose/cache/hashing.rs:195`).
- `MarkdownError::SchemaValidationFailed` carries `path`, `problems`, `summary`, and `description`; the styled block correctly renders `<i><dim>{description}</dim></i>`, OSC8 source link, and per-problem `<red>{kind}</red> <inverse>{target}</inverse>` bullets (`darkmatter/lib/src/markdown/errors/blocks.rs:138`).
- Six unit snapshot tests cover the missing/type/invalid/preparation/with-description/multi-problem rendering paths in plain text.
- Integration tests verify CLI parity with `md schema validate`, fail-fast before shell expansion (with sentinel side-effect check), child schemas under `::file set=...` overlays, and persistent-cache isolation across distinct baselines.

The blockers identified in review-4 have **not** been addressed in the current working tree. The functional implementation is unchanged from review-3 / review-4; the Level 2 styling test still has the same two gaps that prevented review-4 from passing.

## Findings

### High: Level 2 schema-error styling test still does not exercise the `description:` line

Review-4 required a Level 2 schema fixture with a `description:` field plus an assertion that the captured pane shows italic + dim SGR for that line. The current fixture at `darkmatter/cli/tests/level2_errors.rs:112` is unchanged:

```rust
const MISSING_REQUIRED_SCHEMA: &str =
    "---\n$schema:\n  spec: 'string(min(1); required)'\nspec: \"\"\n---\nBody\n";
```

There is no `description:` field, and `level2_schema_validation_block_renders_styled_link_and_bullet` (`darkmatter/cli/tests/level2_errors.rs:146`) makes no italic-SGR (`\x1b[3m`) or description-text assertion. The spec's `<i><dim>...</dim></i>` rendering contract for `description:` (`spec.md:118`) therefore remains verified only at Level 1 (plain-text snapshots), which cannot catch a regression that drops the italic SGR but keeps the text.

Required fix: extend the fixture (or add a sibling Level 2 test) with `description: <text>`, assert `frame.plain.contains(<text>)`, and assert `frame.raw.contains("\x1b[3m")` (italic) alongside the existing `\x1b[2m` (dim) check.

### High: Level 2 red-SGR assertion still matches any true-color foreground

The red-SGR assertion at `darkmatter/cli/tests/level2_errors.rs:186` is unchanged:

```rust
let has_red = frame.raw.contains("\x1b[31m")
    || frame.raw.contains("\x1b[91m")
    || frame.raw.contains("\x1b[0;31m")
    || frame.raw.contains("\x1b[38;5;1")
    || frame.raw.contains("\x1b[38;2;");
```

The final branch (`\x1b[38;2;`) accepts any 24-bit foreground color sequence. The OSC8 source link in the same block renders blue with a true-color SGR, so the assertion can pass even if the `<red>missing|type|invalid</red>` label stops rendering red entirely. This is the same defect review-4 flagged; it has not been tightened.

Required fix: either drop the `\x1b[38;2;` branch and rely on `31` / `91` / `38;5;1` / `38;5;9` (the 256-color red indices the renderer can plausibly emit), or extract the SGR triplets and assert the R component dominates (e.g., R ≥ 128 and R > G and R > B). The simpler fix is to remove the broad 24-bit branch — the in-process snapshot tests + the narrow 256-color/3-bit checks are sufficient to catch a regression.

### Medium: `arm_index` rendering path has no test

The spec requires "Root-union failures include `arm_index` when present, for example `schema arm 2`" (`spec.md:124`). The renderer implements this (`blocks.rs:208-211`), but no unit, snapshot, or integration test constructs a `ValidationProblem` with `arm_index: Some(_)`. All six snapshot fixtures and the schema-validation unit tests use `arm_index: None`. The rendering branch is dead-code-untested.

Required fix: add a snapshot test exercising a root-union `$schema` mismatch (or directly construct a `ValidationProblem { arm_index: Some(2), .. }` in `error_snapshots/markdown_error.rs`) so the `(schema arm N)` suffix is captured.

### Low: `MarkdownError::SchemaValidationFailed.path` is a `PathBuf` that may hold a non-path string

`schema_validation::source_path` (`schema_validation.rs:93-110`) wraps `ComposeSource::Url(...)` / `ComposeSource::Unknown.display()` strings inside a `PathBuf` as a "display carrier". This is documented in the doc-comment and was flagged informationally in review-3, but it is a typed-API smell: a future caller who interprets `path` as a real filesystem path (e.g. to read excerpt context) will silently treat a URL or `<stdin>` as a path. The compose renderer only uses `to_string_lossy()`, so the current code is safe, but the API contract is fragile.

Suggested follow-up (not blocking): change the error field to an enum (`SchemaErrorSource::File(PathBuf) | Url(String) | Stdin`) when a future variant needs to consume the source distinctly. Out of scope for this iteration's production-readiness gate.

## Test Rigor Verification

| Requirement | Strongest observed verification | Assessment |
|---|---|---|
| No-op when no `$schema` and no baseline | Level 1 (`no_schema_no_baseline_is_no_op`) | Adequate |
| Document `$schema` honored (valid / missing / wrong type) | Level 1 unit + Level 1 CLI integration | Adequate |
| Baseline merges with document `$schema`; document wins | Level 1 (`baseline_merges_with_document_schema`, `document_wins_when_both_declare_same_property`) | Adequate |
| `--set` / state override effective frontmatter is validated | Level 1 (`set_override_can_*`) + Level 1 CLI (`compose_and_schema_validate_agree_on_same_document`) | Adequate |
| Compose fails before frontmatter shell expansion | Level 1 + Level 1 CLI sentinel | Adequate |
| Recursive compose validates child after parent `set=` overlay | Level 1 (`parent_set_overlay_*`) | Adequate |
| Baseline participates in transclusion cache key | Level 1 hash + Level 1 persistent-cache behavioral | Adequate |
| `md compose` ↔ `md schema validate` parity | Level 1 CLI integration | Adequate |
| Styled block: OSC8 source link + inverse property | Level 2 WezTerm pane capture | Adequate |
| Styled block: red category label | Level 2 attempted | **Gap** — assertion can pass on any 24-bit fg color |
| Styled block: dim+italic description line | Level 1 only | **Gap** — no Level 2 fixture exercises `description:` |
| Schema-preparation failures surface summary | Level 1 snapshot + Level 1 CLI | Adequate |
| `arm_index` rendering for root-union failures | None | **Gap** — branch is exercised by no test |

No requirement needs Level 3; this feature has no OS keyboard/mouse input surface.

## Conclusion

Not ready for production. The implementation is functionally complete and would pass the spec under a less strict styling-test bar, but the two Level 2 gaps from review-4 remain unfixed in the current working tree, and the `arm_index` rendering path has no test at any level. The previous review's request was to harden the real-terminal verification of the styling contract before shipping; that work has not landed.
