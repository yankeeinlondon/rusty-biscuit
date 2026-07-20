---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T01:26:26-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-6.md
previous: 2026-07-13-meta-schema/review-5.md
---

# Review 6 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 5's two implementation
findings are fixed: standalone scalar `$schema` references now receive the
required completion, and acyclic reference-depth exhaustion now has a distinct
structured error, accurate recovery advice, and boundary coverage. The
meta-schema implementation otherwise satisfies the reviewed parser, resolver,
validation, serialization, and DMLS requirements.

Production readiness is blocked by acceptance criterion 13. A fresh canonical
Level-2 run passed the library tier 18/18 but passed only 58/69 CLI tests and
stopped before DMLS. Three failures are the known code-block staging tests in
the specification. Eight additional error-rendering tests invoke bare `md`
instead of the Cargo-built binary and therefore capture `bash: md: command not
found`. The proposed exception covers exactly the original three tests, remains
unratified, and cannot cover these eight additional failures.

## Findings

### High: Eight Level-2 error-rendering tests do not execute the product under review

The Level-2 error suite claims to verify user-visible OSC 8 links, diagnostic
gutters, schema-validation blocks, and file-reference suggestions in WezTerm.
Its three command builders instead invoke the bare host command `md compose`
([level2_errors.rs:98](../../cli/tests/level2_errors.rs#L98),
[level2_errors.rs:135](../../cli/tests/level2_errors.rs#L135),
[level2_errors.rs:180](../../cli/tests/level2_errors.rs#L180)). This bypasses the
workspace binary. The shared Level-2 helper explicitly documents that tests
must use `md_shim()` so they exercise `CARGO_BIN_EXE_md` rather than an absent or
stale host installation
([level2.rs:30](../../cli/tests/common/level2.rs#L30),
[level2.rs:46](../../cli/tests/common/level2.rs#L46)).

On this review host, all eight tests in `level2_errors.rs` fail after the real
terminal reports `bash: md: command not found`. The assertions consequently
inspect shell error text, not Darkmatter output. These tests currently provide
no valid Level-2 verification of the user-observable rendering they name. Under
the required rigor model, that is a verification-level gap and therefore a
high-severity finding, not merely a red test count.

Route all three error-suite command builders through the same Cargo-built shim
used by the other CLI Level-2 tests, retaining the existing cross-platform
link/hard-link/copy fallback. Then rerun the canonical gate and confirm each of
the eight assertions captures actual `md` diagnostics through WezTerm.

### Medium: AC13 remains unmet and the proposed exception is insufficient

Acceptance criterion 13 requires both `just test` and `just test-l2` to pass
([spec.md:750](spec.md#L750)). The specification says its exception is proposed,
awaiting ratification, and does not yet relax the criterion
([spec.md:752](spec.md#L752)). It also limits that proposal to exactly three
named code-block tests ([spec.md:759](spec.md#L759),
[spec.md:766](spec.md#L766)).

The fresh canonical run produced library **18/18** and CLI **58/69**. The three
listed code-block failures remain reproducible, and the eight bare-`md` failures
above are outside the proposed scope. The recipe exits on the CLI failure before
starting DMLS; an independent continuation of the same scoped Level-2 recipe
passed DMLS **3/3**. Thus neither ratifying the proposal as currently written
nor citing the passing feature slices would make the declared gate green.

Repair the eight invalid tests, then either restore the three code-block tests
or obtain the documented ratification for precisely those remaining failures.

## Prior Review Closure

- **Standalone scalar `$schema` reference completion — closed.** The provider
  recognizes the top-level scalar declaration, and Level-1 in-memory LSP tests
  cover empty, partial, and complete values across LF and CRLF, replacement
  ranges, and last-good completion during a malformed edit
  ([lsp_session.rs:3460](../../dmls/tests/lsp_session.rs#L3460)).
- **Reference-depth diagnosis and boundary coverage — closed.** An acyclic
  32-file chain resolves, the 33rd file yields
  `SchemaError::ReferenceDepthExceeded`, the displayed message and rendered
  advice distinguish depth exhaustion from a cycle, and a root-union entry
  route is bounded by the same cap
  ([meta_schema_reference_graph.rs:217](../../lib/tests/meta_schema_reference_graph.rs#L217),
  [meta_schema_reference_graph.rs:237](../../lib/tests/meta_schema_reference_graph.rs#L237),
  [meta_schema_reference_graph.rs:278](../../lib/tests/meta_schema_reference_graph.rs#L278)).
- **Canonical Level-2 release gate — open and broader than recorded.** The three
  proposed-exception failures remain, and eight additional tests fail before
  invoking Darkmatter.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Library/CLI unit and integration tests, property tests, and filesystem round trips | Appropriate and passing. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity, source-projection, structural-path, and malformed-buffer tests | Appropriate and passing. |
| AC8: base schema and referenced declaration preparation | Level 1 | Base-schema, multi-hop, cycle, dependency, origin, exact-depth, over-depth, and cache-invalidation tests | Appropriate and passing. The explicit chain and recovery advice are asserted in process. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory LSP tests, including scalar-reference LF/CRLF and malformed-edit cases | Appropriate and passing for protocol behavior. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests | Appropriate and passing. |
| AC11–12: grammar recursion bounds and compatibility | Level 1 | Grammar-depth, baseline replay, import-name, and downstream tests | Appropriate and passing. |
| AC13: terminal/editor presentation and area release gate | Level 2 for rendering; Level 1 for protocol semantics | Library 18/18; CLI 58/69; independently continued DMLS 3/3 | Not satisfied. Eight CLI rendering tests do not execute the product, three more fail their real-terminal assertions, and the canonical recipe exits red. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion is
LSP protocol behavior and is correctly verified at Level 1; terminal rendering
is correctly assigned to Level 2, but the broken command staging prevents eight
tests from providing that evidence.

## Verification Performed

- The requested `@prompts/./_reviews/.../review-5.md` reference does not resolve
  in this worktree. The canonical previous review is the colocated
  `darkmatter/features/2026-07-13-meta-schema/review-5.md`, which was used for
  the review-chain update.
- `sniff` identified `darkmatter`, `darkmatter-cli`, and `dmls` as the affected
  package-area crates and confirmed the repository consumers of the public
  library.
- GitNexus reports HIGH upstream impact for `resolve_reference` (59 affected
  symbols), LOW impact for `meta_schema_kinds_for_line` (24), and LOW impact for
  `ReferenceStack::enter` (6). The HIGH-risk resolver was inspected without
  modification; its two recursive entry routes share the new cap.
- `just test` passed: Darkmatter **5,929/5,929**, Darkmatter CLI **561/561**, and
  DMLS **616/616**. One unrelated reference-graph test passed on nextest retry.
- `just build` and `just lint` passed for all three affected crates. No
  formatting command was run.
- `just test-l2 --no-fail-fast` passed Darkmatter **18/18** and failed the CLI at
  **58/69**, so the canonical recipe did not reach DMLS. A scoped continuation
  passed DMLS **3/3**. The CLI failures are the three named code-block tests and
  all eight tests from `level2_errors.rs`.
- The new public `SchemaError` variant has no exhaustive downstream match in
  the repository; the identified `claudine` translation path retains a wildcard
  arm.
- `md schema validate` accepts the updated specification. Review 6 cannot be
  validated against `feature-review.yaml` because that existing tagged schema
  envelope contains unsupported `description` and `$schema` keys, the same
  schema-definition drift recorded by earlier reviews.
- No Rust source was modified during this review. The pre-existing unrelated
  `CLAUDE.md` worktree change was preserved.

## Production Readiness

**Not ready.** Fix the eight Level-2 error tests so they run the Cargo-built
binary, then restore the remaining three code-block tests or ratify their exact
scoped exception. The parser, resolver, validation, serialization, and DMLS
implementation itself requires no additional functional or performance change
from this review.
