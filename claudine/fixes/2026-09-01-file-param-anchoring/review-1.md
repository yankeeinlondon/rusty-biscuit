---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-02T01:20:11+01:00
spec: 2026-09-01-file-param-anchoring/spec.md
implemented: false
description: A **fix** review of `2026-09-01-file-param-anchoring/spec.md`
fix: 2026-09-01-file-param-anchoring/review-1.md
---

# Review 1: File Parameter Anchoring

## Verdict

The fix is **not ready for production**. The Claudine planning regression now
passes from both tested launch directories, the new diagnostic remains typed,
and the full Darkmatter and Claudine Level 1 and lint gates pass. However, the
same defect remains reachable through Darkmatter's shipped `md compose` CLI,
and the caller projection guesses across union arms instead of preserving the
existing exactly-one-applicable-arm contract. Several required Level 1 rows are
also absent, including post-shell classification drift and the actual shipped
planning document.

## Findings

### 1. High: `md compose` still evaluates caller file expressions from the raw override

`prepare_caller_projection` returns an empty artifact whenever
`file_ref_fallback_dir` is absent (`darkmatter/lib/src/markdown/compose/schema_validation.rs:468-470`).
The `md compose` route creates `ComposeOptions`, captures the invocation CWD for
`ComposeContext`, and later sets only the source file
(`darkmatter/cli/src/commands/compose.rs:218-233`, `323-326`). It does not install
the caller launch base as `file_ref_fallback_dir` or a request-scoped
`FileResolutionContext`. Consequently, the new pre-interpolation seam does
nothing for the public Darkmatter CLI.

The exact shipped planning workflow reproduces the gap from the `claudine/`
package area:

```console
$ ../target/debug/md compose ../prompts/plan.md \
    spec=fixes/2026-09-01-file-param-anchoring/spec.md \
    --no-baseline-schema --no-trigger-schemas
MarkdownError: schema validation failed
invalid spec: no existing file matched ... while resolving from `.../prompts`
```

A smaller stdin probe also showed the split state directly: schema processing
normalized `spec` to `claudine/fixes/.../spec.md`, while the frontmatter
expression had already fixed `plan` as `fixes/.../plan.md`.

This violates D1, acceptance criteria 1 and 3, and the Impact section's claim
that the shared pipeline fixes direct compose and every downstream
`ComposeOptions::set_overrides` caller. The fix currently works only for callers
that know to provide the undocumented companion fallback field.

**Required change:** make caller provenance available on every normal compose
route. The `md compose` invocation should capture one launch-anchored
`FileResolutionContext` and pass it through both reference validation and final
composition. At the library boundary, either make that context sufficient for
projection or enforce the required context/fallback pair in one constructor so
callers cannot silently opt into the old behavior. Add a process-level Level 1
test using the real `md compose` binary from repository-root and package-area
launch directories.

### 2. High: union projection guesses an eager arm before applicability is established

The prelude recursively collects property fragments from every root
`allOf`/`anyOf`/`oneOf` arm (`schema_validation.rs:572-590`).
`value_has_eager_schema` then classifies any string as eager when any nested arm
contains `format: darkmatter-file` (`schema_validation.rs:547-569`), and
`resolve_eager_caller_value` resolves the first eager arm it encounters. Neither
path validates or selects the applicable property/root-union arm.

That conflicts with Darkmatter's existing union contract. Property coercion and
eager-file normalization commit only when exactly one arm validates; ambiguous
or zero-match unions do not guess
(`darkmatter/lib/src/markdown/schemas/coerce.rs:446-477`,
`darkmatter/lib/src/markdown/schemas/rewrite.rs:267-304`). The new test avoids
this edge by using `[file(eager), number]`; it does not cover an eager-file arm
beside another string arm or a discriminated root union.

As implemented, a valid caller value for a plain-string arm can be rejected as
a missing file merely because a sibling arm is eager. For a root union, the
classification set also includes eager properties from inactive arms, so a
discriminant moving between eager and non-eager arms may not be recognized as
D3 drift at all.

This violates D1's “applicable union arms,” D3's fail-closed classification
rule, and acceptance criteria 5-7.

**Required change:** derive projection and stability classification from the
same selected/applicable union semantics used by coercion and normalization.
Do not project when zero or multiple eager candidates apply. Add Level 1
regressions for a property union with eager-file and plain-string arms, a
discriminated root union whose active arm is non-eager, and a discriminant that
changes between eager and non-eager arms after interpolation and after shell
expansion.

### 3. High: required post-shell and shipped-workflow Level 1 evidence is missing

The specification explicitly requires Level 1 verification for post-shell
revalidation, pass 2, root unions, baseline-plus-document schemas, stable
classification after shell expansion, projection idempotence, and one registry
discovery walk. The five new Darkmatter integration tests cover root/package
launches, arrays, a property union, unprojected categories, pre-shell trigger
drift, and typed file failures. They do not directly cover:

- a trigger changing eager classification after `$(...)` expansion and pass 2;
- retention of the same native/presentation projection through that pass;
- root-union selection or baseline-plus-document layering;
- idempotence and proof that trigger discovery runs once; or
- an absent optional caller property.

The drift test at `darkmatter/lib/src/markdown/compose/tests/schema.rs:790-848`
deliberately fails before the sentinel shell command executes, so it cannot
verify the post-shell branch. The Claudine CLI regression at
`claudine/cli/tests/compose_schema_cli.rs:1084-1155` recreates the planning
frontmatter in a temporary file rather than invoking the shipped
`prompts/plan.md`; drift in the real prompt can therefore escape it.

These are user-observable composition and diagnostic requirements, so Level 1
is the correct tier, but no test currently exercises them. Under the review's
test-rigor rule, that is a production-readiness gap rather than inferred
coverage from nearby tests.

**Required change:** add the missing focused Level 1 cases and make the
Claudine process test invoke the shipped planning document (or assert the
fixture is byte/structure-locked to it). Include a counter or injectable
registry loader to prove discovery occurs once without relying on timing.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Eager caller values are projected before frontmatter pass 1 from repository-root and package-area launches | Level 1 Darkmatter integration and Claudine process tests | Correct level for the covered Claudine/library route; **gap** for the shipped `md compose` route (Finding 1). |
| The planning workflow derives one plan beside the input specification | Level 1 Claudine process test using a recreated prompt | Correct level, but the required shipped `prompts/plan.md` route is not exercised. |
| Frontmatter semantic state and body presentation identify the same file | Level 1 integration assertions on native frontmatter and portable body output | Appropriate level for scalar and array cases. |
| Caller resolution uses captured launch context without prompt/CWD fallback | Level 1 library/Claudine tests | **Broken** through `md compose`; direct reproduction resolves from `prompts/`. |
| Lazy, ordinary-string, excluded, and document-owned values retain existing behavior | Level 1 integration test | Appropriate level for the covered shapes. |
| Scalar, array, property-union, and root-union shapes use only applicable eager declarations | Level 1 covers scalar, array, and eager-file/number property union | **Gap and broken:** string-compatible and root-union applicability are untested and guessed. |
| Dynamic eager typing fails closed after pass 1 and after shell/pass 2 | Level 1 covers interpolation-time drift before shell execution | **Gap:** no post-shell/pass-2 verification. |
| Malformed/missing eager caller values keep typed file diagnostics and launch provenance | Level 1 typed diagnostic tests | Appropriate level for syntax/no-match; no process assertion for launch provenance. |
| Projection is idempotent and trigger discovery occurs once | No direct test | **Gap:** required Level 1 evidence is absent. |
| Windows native semantic paths and portable presentation remain equivalent | Cross-platform Level 1 test with Windows-specific assertions | Appropriate level when run on Windows CI; no L2/L3 requirement. |

Levels 2 and 3 are not applicable. This fix changes composition state,
filesystem anchoring, and process output; it does not claim terminal-emulator
rendering, glyph width/style, scrolling, paste/IME/mouse behavior, keybindings,
or OS keyboard encoding. Level 1 process and integration tests are the correct
verification tier.

## Verification Performed

- `darkmatter`: focused new Level 1 tests — **5 passed**.
- `claudine`: focused planning CLI regression — **1 passed**.
- `darkmatter`: `just test` — **7,559 passed, 50 skipped**.
- `darkmatter`: `just lint` — **passed** for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `claudine`: `just test` — **6,666 passed, 11 skipped**.
- `claudine`: `just lint` — **passed** for all five package-area crates and the diagnostic guards.
- Direct `md compose` reproduction through the shipped planning prompt — **failed with prompt-directory anchoring**, as described in Finding 1.
- `git diff --check` — **passed** before the review artifacts were written.

The green suites establish that the implemented Claudine route and existing
behavior are stable on this macOS host. They do not cover or override the two
semantic failures above.

## Closure Criteria

Address Findings 1 and 2, add the missing Level 1 rows from Finding 3, and rerun
the Darkmatter and Claudine package-area test and lint gates. Production
readiness requires the public `md compose` route and all applicable union shapes
to obey the same one-parameter/one-value contract as the currently passing
Claudine workflow.
