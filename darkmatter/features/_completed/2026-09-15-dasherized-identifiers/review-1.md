---
$schema: feature-review.yaml
ready: false
findings:
  - "High — Typed interpolation errors discard the required authored expression span"
  - "Medium — Unknown-root candidate deduplication is quadratic"
  - "Medium — The permanent corpus gate does not discover schema-typed expressions"
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T07:50:37-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: true
next: 2026-09-15-dasherized-identifiers/review-2.md
implemented_by: claude/opus
log: darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-1.md
---

# Review 1: Dasherized Identifiers

## Verdict

The fix is **not ready for production**. The lexer, parser, compose behavior,
runtime unknown-root policy, DMLS diagnostics, and editor navigation are
implemented and pass the complete Level-1 package-area suite. However, the
typed interpolation error does not retain the source span required by
Requirement 2, and it can also lose the authored line after an earlier body
rewrite. Two additional gaps leave the specified complexity bound and permanent
shipped-artifact gate weaker than required.

Human review is not required. Each finding has a direct implementation and
Level-1 verification path; cross-OS CI remains external to this readiness
decision.

## Findings

### High — Typed interpolation errors discard the required authored expression span

Requirement 2 says a fatal expression error must carry its original source
path, line, and expression span, specifically requiring the scanner byte span
to be projected through the source context rather than reduced to a line
number (`spec.md:238-244`). The rewrite engine does return a byte range in
`LocatedInterpolationError`, but `anchor_authored_failure` converts it to a
one-based line and discards the range
(`lib/src/markdown/compose/inline/interpolation.rs:116-132`). The public
`MarkdownError::Interpolation` shape has an expression string and a
`SourceRef`; `SourceRef::OnDiskLine` stores only `context` and `line`, so no
caller can recover the authored byte or column range
(`lib/src/markdown/types.rs:22-51`, `lib/src/markdown/types.rs:121-142`).

The loss is broader after an earlier stage edits the body. The regression at
`lib/tests/compose_expression_failure_contract.rs:106-118` explicitly accepts
only `SourceRef::OnDisk` after a page-block rewrite, which omits even the
authored line although the specification requires it. The primary failure
helpers assert the expression text and line but never a typed span
(`lib/tests/compose_expression_failure_contract.rs:56-85`), so the tests pass
while the contract remains incomplete.

Preserve an authored document-relative range in the typed error/source model
and carry it through body and mixed-frontmatter projection. Earlier stages
need source-map-aware projection rather than prefix equality if they can shift
an otherwise authored expression. Add Level-1 library assertions for the exact
range on body and mixed-frontmatter parse and evaluation failures, including a
failure after an offset-changing stage; retain the CLI assertions for the
rendered file, line, empty stdout, and single error.

### Medium — Unknown-root candidate deduplication is quadratic

The performance contract requires O(1) accumulator work per variable read and
O(candidates) reconciliation (`spec.md:590-600`). Candidate observation itself
is append-only, but every call to `add_unknown_root_candidates` scans the entire
candidate vector before inserting each root
(`lib/src/markdown/compose/context/report.rs:388-412`). A document containing
`n` distinct unknown roots therefore performs O(n²) equality checks before the
linear reconciliation pass. The later warning insertion also linearly scans
the warning vector for every distinct coded warning
(`lib/src/markdown/compose/context/report.rs:286-302`).

Keep first-occurrence ordering while using a hash-backed identity set for
candidate/warning membership, or collect candidates without deduplication and
perform one stable O(n) deduplication during reconciliation. Add a stable work
counter or comparison-count regression rather than a wall-clock test, as the
repository testing guidance recommends for complexity contracts.

### Medium — The permanent corpus gate does not discover schema-typed expressions

The permanent compatibility audit must classify Expression-typed frontmatter
values through the library's own extraction in addition to interpolations,
directive conditions, and `$()` branches (`spec.md:74-88`). The implemented
walk recursively treats only the hard-coded key names `when`, `while`, and
`until` as condition expressions
(`lib/tests/dasherized_identifier_corpus.rs:75-79`,
`lib/tests/dasherized_identifier_corpus.rs:114-143`,
`lib/tests/dasherized_identifier_corpus.rs:164-170`). It never resolves a
document's effective schema or asks the schema machinery which arbitrary
properties have the `expression` content format. A newly shipped schema-typed
property can therefore contain an incompatible or malformed expression while
the permanent gate remains green.

Route frontmatter values through the same passive schema-aware expression-value
classification used by DMLS, or expose a shared library extractor suitable for
both consumers. Add a fixture whose Expression-typed property has a name other
than the three Claudine condition keys and prove that malformed content makes
the corpus gate fail. Retain the hard-coded Claudine-key drift check only for
the separate lifecycle semantics it protects.

## Requirement Verification Levels

All user-observable requirements in this fix concern deterministic parsing,
composition, diagnostics, CLI process results, or LSP protocol responses.
Level 1 is the correct verification tier for each. No requirement depends on a
real terminal emulator's rendering, a terminal input encoder, OS keyboard or
pointer injection, paste/IME behavior, mouse behavior, or scrolling; Level 2
and Level 3 are not applicable.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Dasherized names lex and parse without changing subtraction | Level 1 lexer/parser tables, library composition, spawned `md` CLI, and real LSP-session navigation | Correct level and behavior; all specified positive, subtraction, Unicode, dotted-path, object-key, and cursor cases are exercised. |
| 2. Invalid full-document expressions fail with typed provenance and no partial output | Level 1 library and spawned `md` CLI tests | Behavior, exit status, and empty stdout pass, but typed span provenance is missing; **high gap**. |
| 3. DMLS checks identifiers in every operand position | Level 1 provider tests and real in-memory LSP sessions | Correct level; body and Expression-typed frontmatter findings, ranges, severity, dash-key fixes, and the frontmatter-less exception are covered. |
| 4. Unknown roots warn only when absence is unhandled | Level 1 full-document composition and spawned CLI tests | Correct level; reachability, suppression rows, state/caller/schema sources, page blocks, transclusion, `$()` branches, and warning metadata are covered. |
| 5. Each compose issue is reported once | Level 1 library and CLI identity tests | Correct level; repeated reads, distinct roots, sibling documents, and rescan failures are covered. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus plus normal shipped-prompt composition | Correct level, but the passive corpus omits general schema-typed expression discovery; **medium gap**. |
| Qualitative performance bounds | Source inspection only | The DMLS walk and runtime observation are linear, but candidate deduplication violates the required bound; **medium gap**. |

## Implementation Assessment

The lexer keeps the dash decision inside `read_variable`, applies it to both
leading and dotted segments, and shares `identifier_prefix_start` with DMLS.
The AST-level absence classifier is reused by runtime and static analysis, and
the runtime observer preserves short-circuit reachability without adding a
cross-document lock. Unknown-root reconciliation correctly consults final
state, caller input records, and effective schema; warning identity remains
source-document scoped.

GitNexus reports exact upstream impact of MEDIUM for `read_variable` (75
symbols), LOW for `identifier_prefix_start` (1), CRITICAL for
`interpolate_text` (52 symbols across two processes and two modules), and LOW
for `static_variable_reads` (3). The high-impact interpolation path is why the
missing typed range and its under-asserting tests block readiness. Aggregate
dirty-worktree change detection is CRITICAL across 549 files, 974 symbols, and
77 processes because this worktree contains several concurrent feature areas;
that aggregate result is not treated as feature-specific evidence.

## Verification Performed

- `darkmatter/just test`: 8,269 passed, 0 failed, 14 intentional higher-tier
  skips.
- `darkmatter/just lint`: passed for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.
- GitNexus concept query and exact upstream-impact analysis covered the lexer,
  cursor helper, interpolation rewrite, and static variable walk.
- Source inspection mapped every specification requirement to its strongest
  verification boundary. No Level-2 or Level-3 execution was warranted.
