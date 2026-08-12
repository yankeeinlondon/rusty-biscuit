---
ready: false
agent: codex/default
created: 2026-06-29T20:19:54
implemented: true
---

# Review 1 - Eager Files

## Findings

### High - `prompts/implement-plan.md` now requires `spec`, breaking the documented "plan or spec" workflow

`prompts/implement-plan.md:5-16` changes both schema and behavior:

- `plan` changed from `file(required)` to `file(required;match(**/*plan*.md))`.
- `spec` changed from optional `file` to `file(required;match(**/*spec*.md))`.
- The prompt description still says callers may provide either `plan` or `spec`.
- The computed `spec` value is `null` when a caller supplies an existing plan that has no sibling `spec.md`.

Because `required` still means "present and non-null", a valid plan-only invocation without a sibling spec now fails before the implementation run. This is outside the spec's migration boundary: the spec explicitly says prompt audit is owner-managed and out of scope except for tests needed to prove this feature. It also does not add `eager`, so the edit does not preserve pre-change existence validation for inputs that need it.

Recommendation: revert this prompt edit for this feature. If/when the prompt audit is done, preserve the "either plan or spec" contract explicitly rather than marking both required.

### Medium - Public docs say eager `file` resolves from CWD, but implementation resolves document-first with launch fallback

The updated docs state that eager file validation resolves from the current working directory:

- `darkmatter/docs/inline/schema-validation.md:104`
- `darkmatter/docs/topics/schema-definition.md:97`
- `darkmatter/docs/topics/schema-definition.md:165-167`

The implementation and tests use the newer anchored resolution contract: document directory first, then launch-area fallback, and only legacy/no-anchor callers fall back to ambient CWD (`darkmatter/lib/src/markdown/schemas/format.rs:80-90`, `darkmatter/lib/src/markdown/schemas/format.rs:200-226`, `darkmatter/lib/src/markdown/schemas/mod.rs:216-219`).

This is user-facing documentation drift on a behavior this feature specifically touches. It should be corrected before release so authors know where `file(eager)` paths resolve.

## Verification Levels

This feature is schema-validation behavior, not terminal rendering or keyboard UX. Level 1 is the appropriate minimum for most requirements; the Cli compose regression also runs through the compiled binary.

- Bare `file` is syntax-only and accepts missing paths: Level 1 present.
- `file(eager)` preserves existence validation: Level 1 present.
- `required` remains presence-only and orthogonal to `eager`: Level 1 present.
- `file(eager)[]` applies eagerness per item: Level 1 present.
- `match(...)` is metadata only and completion still receives patterns: Level 1 present for schema/completion logic; existing Level 2/Level 3 completion tests are not required for this spec because it does not change terminal input encoding or rendering behavior.
- Motivating Claudine compose shape succeeds/fails correctly: Level 1 compiled-binary test present.

## Verification Run

- `cargo check --color=never -p darkmatter -p claudine -p claudine-cli` passed.
- `cargo nextest run --color=never -p darkmatter -p claudine-cli -E 'test(/file|eager|motivating|schema_completion|compose_lazy_plan|compose_missing_eager/)'` passed: 603 run, 603 passed.

## Decision

Not production ready until the `implement-plan` prompt regression is removed and the docs are corrected to match the anchored eager-file resolution behavior.
