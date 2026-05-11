---
agent: codex
model: ""
ready: false
---

# Review: Good Errors

Assumption: the requested spec path, `darkmatter/features/features/2026-05-08-review-1.md`, does not exist in this worktree. I reviewed the nearest matching active feature spec, `darkmatter/features/2026-05-08-good-errors/spec.md`, because the implementation contains the `StatusBlock`, `SourceContext`, Prose fenced-code, error snapshot, and darkmatter error-skill changes described there.

## Findings

### High: fenced code blocks do not safely preserve literal source content

`SourceContext::frontmatter_prose()` and `SourceContext::excerpt_prose()` feed user document text into fenced Prose code blocks (`biscuit-terminal/lib/src/errors/source_context.rs:75` and `biscuit-terminal/lib/src/errors/source_context.rs:85`). The fenced-code preprocessor then restores those blocks by embedding the raw body inside a synthetic `<code-block>` tag without escaping the body (`biscuit-terminal/lib/src/components/prose/markdown.rs:109` through `biscuit-terminal/lib/src/components/prose/markdown.rs:112`).

That means source content containing `</code-block>` can terminate the synthetic tag early. Everything after that is parsed again as Prose instead of remaining literal source text. This violates the spec's output bar: source excerpts and frontmatter snapshots must show user content literally, and no markup should leak or be interpreted unless it is intentionally authored diagnostic prose.

Required fix: make fenced code blocks opaque all the way into rendering. Do not represent them by interpolating raw body text into a tag grammar that can be closed by user content. A token variant or escaped/sentinel-preserved body is safer. Add regression tests with source/frontmatter containing `</code-block>`, `<dim>`, `<cyan>`, underscores, asterisks, and backslashes.

Verification level: currently Level 1 unit/snapshot coverage exists for happy paths only. This requirement needs Level 1 adversarial tests because it is parser correctness; Level 2 is useful for final rendered shape but will not replace the parser regression.

### High: user-controlled fields are still interpolated into Prose without escaping

The new `StatusBlock::body` contract fixes the old markup-leak footgun by always parsing Prose, but it also means every interpolated user value must be escaped. Several migrated variants still place user-controlled values directly inside Prose markup:

- `StylesheetError` interpolates `declaration`, `name`, `property`, and `value` directly into `<cyan>...</cyan>` bodies (`darkmatter/lib/src/render/stylesheet.rs:75` through `darkmatter/lib/src/render/stylesheet.rs:156`).
- `LinkError::MalformedHtml` and `MalformedMarkdown` interpolate `message` directly (`darkmatter/lib/src/render/link.rs:93` and `darkmatter/lib/src/render/link.rs:105`).
- `ImageRefError::MalformedHtml`, `MalformedMarkdown`, and value variants interpolate `message`/`value` directly (`darkmatter/lib/src/render/image_ref.rs:120` through `darkmatter/lib/src/render/image_ref.rs:179`).

Concrete failure mode: a bad CSS value or parser message containing `<red>hidden</red>`, `</cyan>`, `_self_`, or `**bold**` is interpreted as Prose syntax instead of displayed literally. That is both an output-quality bug and a diagnostic-integrity issue.

Required fix: use `Prose::escape_text()` or a small local helper before embedding any user-supplied string in Prose. The existing `value.replace('_', "\\_")` in `LinkError::InvalidTarget` is too narrow; use one consistent escaping path. Add adversarial snapshot tests for each error family.

Verification level: strongest present is Level 1 snapshots with benign payloads. This is not sufficient for the spec's "No bare markup" and literal user-content requirements.

### High: Level 2 error-rendering tests skip silently, so the required verification is not enforced

`darkmatter/cli/tests/level2_errors.rs` has real-terminal checks for the OSC 8 hyperlink and dimmed excerpt rendering, but both tests return early when WezTerm is unavailable (`darkmatter/cli/tests/level2_errors.rs:13` through `darkmatter/cli/tests/level2_errors.rs:15`, and `darkmatter/cli/tests/level2_errors.rs:51` through `darkmatter/cli/tests/level2_errors.rs:53`).

The spec requires user-observable terminal behavior: linked file paths, styled body text, fenced source excerpts, gutters, and inverse hint tokens. Under the requested rigor rubric, this is at least Level 2 for rendered terminal output. A silently skipped Level 2 suite means production readiness depends on the developer's local terminal setup, not CI.

Required fix: add a required mode, for example `DARKMATTER_LEVEL2_REQUIRED=1`, that fails when WezTerm is unavailable, and ensure the production gate runs that mode in an environment with WezTerm. Keep the skip behavior only for optional local runs.

Verification level: Level 2 tests exist for two behaviors but are not enforced.

### High: Level 2 coverage is incomplete for the rendered error contract

The Level 2 suite verifies only OSC 8 presence and dimmed excerpt output for `PageBlockError::UnterminatedBlock` (`darkmatter/cli/tests/level2_errors.rs:8` through `darkmatter/cli/tests/level2_errors.rs:83`). It does not capture these user-observable requirements in a real terminal:

- frontmatter snapshot rendering as a fenced `yaml` block;
- the specific `>` gutter plus surrounding context lines in a multi-line excerpt with frontmatter present;
- the inverse-styled `::end-block` hint token;
- the "no literal diagnostic tags" invariant in pane text for representative error families;
- at least one migrated non-page-block error family, such as link/image/stylesheet errors.

The unit snapshot for `UnterminatedBlock` is useful, and `unterminated_block_emits_ansi_styling` checks OSC 8/inverse bytes in process, but that is Level 1. It does not prove the real terminal renders and captures the full diagnostic shape.

Required fix: extend Level 2 coverage to a frontmatter-bearing page-block file and one or two representative non-page-block errors, then assert pane text and raw capture for the listed requirements.

Verification level: incomplete Level 2.

### Medium: the requested spec path is missing

The prompt names `darkmatter/features/features/2026-05-08-review-1.md`, but that file is absent. The review above uses `darkmatter/features/2026-05-08-good-errors/spec.md` because it matches the implementation. If the missing path was intended to describe a different feature, this review may be scoped to the wrong artifact.

Required fix: either add the intended spec file or update the review prompt/path so future reviews are unambiguous.

## Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `StatusBlock::body` parses Prose instead of leaking tags | Level 1 unit tests and snapshots | Partial; benign cases only |
| Linked file path via OSC 8 | Level 2 test exists but skips silently | Gap |
| Frontmatter snapshot in body | Level 1 snapshot | Gap for real-terminal rendering |
| Source excerpt with gutter/context | Level 2 for simple excerpt, skipped silently | Partial |
| Hint rendered as Prose with inverse directive token | Level 1 ANSI assertion | Gap for real-terminal rendering |
| Literal rendering of user source/frontmatter content | Level 1 happy-path tests | Gap; adversarial content can break out of code block |
| Literal rendering of user-controlled error fields | Level 1 happy-path snapshots | Gap; values/messages are not consistently escaped |
| Snapshot test pattern for error variants | Level 1 snapshots across many variants | Mostly OK, but missing adversarial invariants |
| Documentation and skill update | File present | OK |

## Ergonomics / Performance Notes

- Add a single helper for diagnostic values, e.g. `diagnostic_value(value) -> String`, instead of repeating ad hoc escaping rules at call sites.
- Treat source excerpts as structured content in Prose rather than lowering through a fake HTML-like tag. It will be simpler to reason about and avoids special sentinel escaping.
- Add a reusable assertion helper for snapshots: no diagnostic tags like `<dim>`, `<cyan>`, `<inverse>`, or `<code-block>` appear in stripped output unless explicitly expected as quoted source content.

## Production Readiness

Not ready. The feature has the right architecture in broad strokes, but literal source rendering is not robust, user-controlled values are not consistently escaped, and the required real-terminal verification is incomplete and not enforced.

## Verification Attempt

I attempted `cargo test -p darkmatter --test error_snapshots --color=never`. It was still compiling after roughly 60 seconds, so I terminated it per the non-interactive session constraint. The resulting SIGTERM errors are from stopping the command, not from test failures.
