---
kind: feature
name: cyclomatic-complexity
date: 2026-10-07
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-06-symbol-type-relations
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 2
completed: true
implemented: true
implemented_by: "claude/opus"
human_review: true
human_review_items:
  - |-
    **Accept (or change) how complexity is counted, so the feature can
    go to review.**

    *Why now:* all five phases are built. The counting rules for Rust,
    JavaScript and TypeScript are implemented, tested and now published in
    `tree-hugger/docs/cyclomatic-complexity.md`. They follow the convention
    the spec recommends and the drafted answers below, because nobody could
    approve them in this unattended run. The spec makes your acceptance a
    condition of completion, so the feature is "ready for review" only once
    you accept. Changing a rule now means new expected numbers in the tests,
    an edit to that one doc page, and a cache invalidation (an extractor
    revision bump).

    The basic convention: a function starts at 1. It adds 1 for each
    `if`/else-if, loop with a condition, ternary, `&&`/`||`/`??`,
    `&&=`/`||=`/`??=`, `match`/`switch` arm, guard and `catch`. Plain
    `else`, `default`, `finally`, `loop`/`for(;;)`, optional chaining `?.`
    and Rust `?` add 0. The full tables with worked examples are in this
    feature's `implementation-log.md` under "Phase 1". The numbers the code
    now produces are in `lib/tests/l1/symbols_metrics.rs`
    (`RUST_CYCLOMATIC`, `JAVASCRIPT_CYCLOMATIC`, `TYPESCRIPT_CYCLOMATIC`).

    Six smaller questions the spec left open, with the answers now built:

    - **R1. Rust `match`:** count every arm except the last, because a
      `match` must cover every case and its last arm acts as the default.
      A two-arm `match` then scores the same as `if`/`else`. The
      alternative counts every arm except a literal `_`.
    - **R2. Rust `let Some(x) = y else { return }`:** +1, like `if let`.
      The alternative is 0.
    - **R3. Rust `async { }`, labeled or `unsafe` blocks inside a
      function:** counted as part of the function. The alternative treats
      `async` blocks as separate, like closures.
    - **R4. A Rust `const`/`static` item inside a function:** not counted,
      because it is compile-time code.
    - **R5. JS/TS object literal values `{ p: a ? 1 : 2 }`:** counted (they
      run right there); methods inside the literal are separate.
    - **R6. TS `namespace N { }` inside a function:** not counted.

    Options:

    - **Accept as built (recommended).** Nothing more to change.
      Pros: no delay, and this follows the spec's recommendation. Cons: any
      later change means revised numbers, a cache invalidation and a docs
      update.
    - **Accept, flipping some of R1–R6.** Name the ones to flip. Pros: the
      numbers match your preferred reading before the docs are written.
      Cons: a short reply, and a few expected values change in the tests.
    - **Choose a different basic convention** (for example, do not count
      `&&`/`||`). Pros: simpler numbers. Cons: all three rule tables and
      their tests are redone, and complex conditions are hidden.

    I recommend accepting as built. R1 keeps Rust `match` in line with
    `if`/`else`. The other choices leave out only code that does not run as
    part of the call.
message_to_agent: |-
  All five phases are implemented (schema 4.3, EXTRACTOR_REVISION "28").
  Notes for the reviewer:

  1. Author acceptance of the counting convention and R1-R6 is still
     pending (`human_review_items`). If any rule is flipped, update the rule
     table in `lib/src/file/extract/complexity/<language>.rs`, the matching
     `*_CYCLOMATIC` rows in `lib/tests/l1/symbols_metrics.rs`, the tables in
     `tree-hugger/docs/cyclomatic-complexity.md`, and bump
     EXTRACTOR_REVISION (adding the stale revision to `cache_upgrade.rs`).
  2. Correction to the Phase 4 note: `--json`/`--jsonl` records DO carry
     `metrics.cyclomatic` when a selector already requests Metrics
     (`--undocumented`, `--min-params`). This was always the behavior and
     matches the spec. Only the prose was wrong. Phase 5 fixed the CLI
     README and the skill, and extended
     `reports::cyclomatic_is_reported_only_where_metrics_already_are` to
     pin it.
  3. The user-facing contract is `tree-hugger/docs/cyclomatic-complexity.md`.
     Every example total in it is an executable row in `symbols_metrics.rs`
     or `complexity::tests`.
---

# Spec: Per-Symbol Cyclomatic Complexity

## Problem and goal

Tree-hugger's symbol reports carry parameter counts and documentation status,
with callable body lines added by `2026-10-06-symbol-type-relations`. They do
not yet expose a measure of control-flow complexity. Add a per-callable
`cyclomatic` measurement so consumers can identify functions with many
independent decisions without running diagnostics or a compiler.

This is a syntax-based measurement of written code. It does not prove the
number of reachable execution paths, account for called functions, expand
macros, or evaluate conditional compilation. Documentation must state the
chosen counting convention so consumers can interpret and compare values.

## Origin, scheduling and prerequisites

Proposed in the pre-split draft of `2026-10-03-symbol-improvements`, and moved
here by the author on 2026-10-03. Scheduled for 2026-10-07 after:

- `2026-10-03-symbol-filters-and-reports`: the optional metrics facet,
  selective queries, and absent versus unavailable measurement semantics.
- `2026-10-06-symbol-type-relations`: callable body selection, `lines`, and
  the additional metrics-matrix column.

Read the accepted prerequisite designs before implementation. Body selection
is shared with `lines`; this feature must not introduce a competing reader.
The current working tree does not yet contain that prerequisite's `lines`
field. Examples below describe the expected combined result after it lands.

## Scope and public contract

Extend the tree-hugger library's
[MetricsFacet](../../lib/src/shared/schema_v2/facets.rs), which holds requested
per-symbol measurements, with `cyclomatic: Option<u32>`.

For a Function, Method or Constructor with a supported, valid implementation
body, the value is `1 + the sum of the body's decision-point contributions`.
An empty or straight-line body is `Some(1)`, never `Some(0)`. Other symbol kinds,
bodyless declarations, and languages or forms without reviewed rules return
`None`. An expression body is eligible just like a block body.

Metrics not requested means the whole facet is absent. When Metrics is
requested, every returned record has the facet, and every metric key is
required inside it. `cyclomatic = None` serializes as `null`, not an omitted
key. An illustrative supported function might emit:

```json
"metrics": {"parameters": 2, "has_docs": false, "lines": 4, "cyclomatic": 3}
```

`null` means a value cannot be provided for this declaration under the
supported rules; it never means the requested analysis was silently skipped.
Unavailable complexity must not erase other available metrics. Do not add a
public unavailability-reason field in this feature; describe language/form
limits in the current docs and the test matrix.

Existing JSON, JSONL and full-index formats expose the new key wherever they
already request Metrics. Do not make every CLI command request Metrics solely
to show it. No new filter, sort mode, complexity threshold, lint rule, text
column, or aggregate statistic is required. Existing counts, documentation
statistics and exit-code policy remain unchanged.

God-files keeps its declaration-span size and risk calculations. Missing
complexity is never converted to zero or used to change a score. This feature
does not change symbol capture coverage, qualified names or symbol IDs, and
does not upgrade grammars to obtain more constructs.

## Body ownership and traversal

Extend the tree-hugger library's
[symbol_metrics](../../lib/src/file/extract/metrics.rs), which computes the
requested facet during extraction, using the same callable body node selected
for `source.body_span` and `lines` by the shared
[body reader](../../lib/src/file/extract/classify.rs). Do not discover a body
by taking the first descendant block. Prototypes, abstract requirements and
C++ defaulted/deleted declarations without a body remain unavailable.

Count only inside that callable's own body. Exclude its parameter defaults,
decorators, attributes, return/type syntax and surrounding declaration
wrappers. Recognize decision points by grammar nodes, fields and operator
tokens rather than searching source strings for keywords.

Stop before entering a nested function, method, constructor, closure, lambda
or type declaration/expression, including anonymous owners. These boundaries
apply even when the nested construct has no extracted symbol or was excluded
by the query. A nested callable that is extracted receives its own metric.
For example, an outer function with only a nested function containing two
`if` statements has complexity 1; the inner function has complexity 3 under
the recommended convention. A query returning only the outer function must
still give 1. Excluding nested type bodies intentionally excludes their field
initializers too; this is an ownership convention, not a runtime execution
model.

A nested boundary is not counted as a decision itself. Ordinary blocks and
control-flow branches are traversed. Each decision is counted once, avoiding
both wrapper/child duplication and repeated operator counting. Use an
iterative traversal or tree cursor so deeply nested input does not consume a
Rust call frame per syntax node. Prune excluded subtrees rather than scanning
them for every enclosing callable; do no per-callable whole-file scans.

Return `None` if a visited body region contains a Tree-sitter error or missing
node that prevents reliable counting. Errors inside excluded nested bodies do
not invalidate the outer metric; errors elsewhere in the file do not invalidate
an intact body. A language rule may also mark a recognized unsupported
control-flow form unavailable rather than publish a partial count. Use checked
arithmetic; a result outside `u32` is unavailable rather than wrapped or capped.

Macro token trees and embedded languages are opaque: do not infer decisions
from their text. Ordinary calls, including immediate calls of lambdas, do not
add the callee's complexity to the caller. Document these limits rather than
claiming equivalence with expanded or compiler-analyzed code.

## Language rule inventory

The counting convention and initial language set require the author choices
under Open Questions. No rule list is declared reviewed merely by this spec
review. Before finalizing support, provide a compact rule table for each
selected language with its pinned grammar node/field/operator matches,
contribution, exclusions, and source examples with expected totals.

Each selected language's inventory must explicitly classify:

- `if`, chained alternatives, loops, unconditional loops and loop `else` forms;
- switch/match/select arms, grouped labels or patterns, defaults and guards;
- ternary expressions and short-circuit operators, including chained operators,
  null-coalescing, optional chaining and logical assignment where applicable;
- exception handlers, `finally`, error propagation and conditional early exits;
- comprehensions, generator filters, pattern alternatives and language-specific
  conditional constructs;
- named and anonymous callable/type boundaries, including expression bodies.

For every applicable construct, state whether it contributes, contributes
nothing by design, or makes the metric unavailable. Do not silently leave a
recognized branching construct unhandled while labeling the language supported.
Language constructs that do not exist need no artificial fixture. Demonstrated
pinned-grammar limitations must name the source form and observed limitation;
a missing reader is an implementation gap, not a grammar limitation.

The tree-hugger library's
[metrics matrix](../../lib/tests/l1/symbols_metrics.rs) gains a `cyclomatic`
support column and expected values. All 16 languages retain a row: reviewed
values for supported forms, or `None` with a reason for unavailable support.
Do not infer complexity support from parameter or documentation support.

## Selective queries, cache and schema

Keep complexity under the existing tree-hugger library
[Capability::Metrics](../../lib/src/file/query.rs), which requests the entire
metrics facet. No new capability or pass is needed. Compute complexity only
for selected records receiving full metrics metadata. A declarations-only
query performs no complexity traversal. Metrics never requires references,
semantic analysis, diagnostics, or a second definitions query.

The existing tree-hugger library
[cache projection](../../lib/src/analysis/coverage.rs) removes the entire
facet when Metrics is not requested. Preserve that behavior. Widening a cache
entry to Metrics reparses once and computes all metric fields together;
a warm entry already covering Metrics is served without parsing. A language
whose complexity is unavailable still completes Metrics and can be cached;
unavailability must not cause perpetual recomputation.

Adding a required metric key changes the serialized record shape. Follow the
minor-version rule in the area's
[schema design](../../docs/symbol-schema-design.md): use the next minor of the
version emitted after the body-lines prerequisite lands (normally 4.2 after
4.1), not a hard-coded reuse of 4.0 or the prerequisite's version. Update
records, indexes, CLI envelopes and fingerprints consistently. Bump the
library's [EXTRACTOR_REVISION](../../lib/src/file/mod.rs), which invalidates
cached extracted facts, as well. Neither change alters symbol identity.

The area [cache design](../../docs/cache-design.md) requires exact current
versions on entries and every record. Recompute older entries; do not migrate
or default an old metrics object to a fabricated `null`. Deserialization must
reject missing or wrongly typed `cyclomatic`, just as for existing required
nullable keys. Update version-sensitive and cache-upgrade tests using the
current revision at implementation time. Any later change to supported rules
must also bump the extractor revision and update examples and expectations.
Older-model adapters that cannot represent metrics remain explicitly lossy;
they must not invent complexity during conversion.

**Reader's note:** the original draft described only a metric and language
coverage. Required serialization, schema versioning and cache invalidation
are added here because otherwise a warm cache could publish old records as
if the new measurement had been computed. This follows the prerequisite's
schema decision and the current strict metrics contract.

## Validation and documentation

Extend existing metrics, selective-query, cache-projection and adapter tests,
plus CLI JSON/JSONL/full-index and cache-robustness checks. Require:

- Exact expected totals for every included rule and zero-contribution examples
  for excluded look-alike constructs; assert fixtures parse without errors.
- Empty/block/expression bodies, bodyless declarations, non-callable records,
  nested anonymous and named bodies, nested types, and a query that omits the
  inner callable. Include decisions in comments, strings and parameter defaults
  to prove they do not contribute.
- Unsupported language/form values, malformed counted bodies and a malformed
  excluded body; a valid sibling callable retains its measurement.
- Equal metrics for narrow and complete queries, LF/CRLF source, and cached and
  uncached results. Metrics absent when unrequested; other fields intact when
  complexity is unavailable.
- No optional passes or complexity traversal when Metrics is unrequested,
  one parse when widening to Metrics, and an actual warm `CacheLookup::Hit`
  without parsing. Use existing work counters and add a focused complexity
  counter only if needed to observe skipped traversal.
- Required nullable-key validation, previous schema/revision invalidation,
  and unchanged symbol IDs and god-files scores. Update golden records only
  for the intentional new key and schema version.

Run `just test` and `just lint` in `tree-hugger/` when implementing. Existing
macOS, Linux, native Windows and WSL2 requirements apply; add no CI environment,
gate, or performance workload for this feature. No performance spike is
required for traversal of the existing tree with nested bodies pruned. Skipped
work and parse counters address the concrete risk of repeated analysis.

Update affected public README/API descriptions, current schema, cache and
symbol-report docs, and the tree-hugger skill. Publish the accepted counting
rules with small source examples and unavailable-form limits. Docs describe
current behavior without naming or linking this feature. Review comments on
changed symbols for drift, including descriptions of how many metrics exist.
Record accepted open-question choices and departures in the implementation log;
do not revise closed prerequisite specs.

Implementation is complete, ready for review, when the author has accepted
the convention and initial language set, every selected rule has executable
expectations, all 16 matrix rows are explicit, query/cache results agree, and
docs explain how to interpret a number or `null`. Leave the feature active;
the author moves it after the review cycle closes.

## Open Questions

### Which counting convention should the metric use?

“Decision points + 1” is not enough to distinguish common complexity variants.
For example, `if (a && b)` can count as one decision or two. The choice affects
comparability and must be accepted before implementing language tables.

- **Count syntax decisions and short-circuit operators (recommended).** Each
  `if`/else-if, conditional loop, ternary and short-circuit Boolean operator
  contributes 1; plain `else` contributes 0. For multi-way branches, count
  each non-default arm once (grouped labels/patterns remain one arm); each
  separate guard contributes 1. Count each exception handler once; `finally`
  contributes 0. An unconditional loop and an unconditional return/break/throw
  contribute 0. Pros: exposes condition complexity using the existing tree,
  including expression bodies. Cons: syntax variants affect totals; remaining
  forms such as comprehensions, null-coalescing, Rust `?`, optional chaining
  and logical assignment still need explicit per-language treatment.
- **Count branch constructs only, excluding expression-level choices.** Pros:
  simpler tables and easier explanation; `if (a && b)` has one decision.
  Cons: hides complex Boolean expressions and underrepresents expression-heavy
  functions; still needs arm, guard and exception rules.
- **Match a named external analyzer's convention per language.** Pros: users
  can compare with that analyzer's documented numbers. Cons: conventions may
  differ between languages and analyzer versions; replicating compiler-aware
  behavior may exceed a syntax-only implementation. Name the analyzer/version
  and document deliberate mismatches before claiming compatibility.

Recommend the first option because it measures written conditions without
adding compiler dependencies. It is a proposal, not an assertion that its
language-specific matches have been reviewed. The final inventories must
resolve every applicable category above before a language reports values.

### Should support land incrementally or for every language together?

The library supports 16 languages, each with distinct grammar shapes. Shipping
numbers without reviewed rules would mislead users; waiting for all rules
could delay useful measurements.

- **Ship a named initial set of reviewed languages (recommended).** Pros:
  useful results sooner; `null` clearly preserves unavailable support. Cons:
  coverage is uneven, so consumers must handle unavailable values. Start by
  reviewing Rust, JavaScript and TypeScript, then include only the languages
  whose complete inventories pass review; document the accepted release set.
- **Require all 16 language inventories before shipping.** Pros: uniform
  coverage and one review of shared counting choices. Cons: larger review and
  later availability, especially where the pinned grammar has limitations.
- **Ship one language first, then expand.** Pros: smallest implementation and
  review scope; proves cache and schema behavior early. Cons: limited immediate
  usefulness and more follow-up work. Rust is a reasonable first candidate
  because it exercises nested items, match arms and guards in this workspace.

Recommend incremental reviewed support with an explicit accepted language
list: it preserves honest measurements without tying the whole feature to the
slowest rule review. At least one language must report reviewed non-null values
for this feature to be complete. Recommendations here require author acceptance;
unreviewed languages remain unavailable rather than receiving approximate
fallback rules.
