---
kind: feature
created: 2026-10-27
status: draft-spec
area: claudine
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
human_review: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-27
review_iterations: 2
completed: true
related:
    - 2026-10-25-pipeline-work-reduction
implemented: true
implemented_by: "claude/sonnet"
---

# Parse each shell placeholder once while preparing a sequence shell site

## Why this exists

Sequence preflight discovers possible commands before any task starts.
In the Claudine package,
[`SequencePreflight::prepare_site`](../../lib/src/composition/sequence/preflight/mod.rs)
prepares one authored command at a time for approval. On the measured input,
five analysis passes each parse the same `{{ … }}` placeholder text:

| Pass | Where the parse happens |
| ---- | ----------------------- |
| template grammar | `parse_authored` (Darkmatter `template/grammar.rs`) projects each hole's untyped type through `project_hole` |
| approval scan | `refuse_unapprovable_reads` → `approval_diagnostics` (`lifecycle/bindings.rs`) → Darkmatter `prepare_value` → `scan_leaf` |
| target identity | `refuse_unapprovable_reads` → `first_target_identity_root` (`sequence/preflight/shape.rs`) |
| type projection | `CommandTemplate::project_types` → `project_hole` (Darkmatter `template/typing.rs`) |
| late-hole marking | `mark_late_holes` → `DeferredShellRoots::late_roots_of` (Darkmatter `template/candidate.rs`) |

The expression text is the same in all five passes and none of them changes
it, so four of the five parses repeat work. This is the placeholder-parsing item in the work table of
`2026-10-25-pipeline-work-reduction`. That feature requires a separate dated
spec when an item meets its measurement rule. The estimated time share below
meets its 5% share rule.

## Measured evidence (2026-10-27, macOS development host)

Revision `d7ea96a1f`, with `2026-10-26-sequence-quadratic-frontmatter` and
`2026-10-25-shared-acyclic-definitions` merged.

- **Anchor A6** (sequence preflight of 100 shell steps, each
  `echo step-N {{ arg_0 }} … {{ arg_9 }}`; `claudine/lib/benches/`):
  - `ExpressionParse` 10,000 (100 per step at 10 and at 100 steps, so the
    sequence fix made it linear);
  - 26,709,585 bytes allocated;
  - Criterion median **8.2117 ms** (interval 8.1460–8.2866 ms).
- **Where the 10 parses per placeholder come from.** One run of A6 at 10 and
  100 steps, with a temporary backtrace tally at the parse counter (removed
  afterward), shows exactly one parse per placeholder per step at each of ten
  sites:
  - the five shell-site passes above;
  - two frontmatter late-root scans (`value_roots`, counted as
    `DeferredRootScanParse`);
  - Claudine's removed-read check (`reject_removed_sequence_reads`);
  - two interpolations that evaluate the command text.
- **Unit cost.** One parse costs **132.18 ns** (Criterion
  `pipeline_work/D8/single_parse`, interval 130.89–133.39 ns) and 215 bytes
  (`pipeline_work_alloc`).
- **Share of A6.** Removing four of the five shell-site parses saves
  4 × 1,000 = 4,000 parses: 4,000 × 132.18 ns = 0.529 ms, which is **6.4% of
  A6's time** (clause 1). In bytes it is 4,000 × 215 B = 0.86 MB, or 3.2%.
  - This is count × isolated unit cost, the report's method, not a
    before-and-after differential.
  - The timing pass ran at a 1-minute load average of about 7–8 on 16 cores.
    That is under the bench preflight's limit (half the cores) but above the
    "about 2" the spike report asked for. Both the per-parse and A6 medians
    came from that one pass.
- If only the two repeats that Darkmatter's isolated
  `CandidateScope::build` sequence shows are counted (3 parses per hole
  there), the share is 3.2% of time. The higher figure applies because all
  five passes run inside one `prepare_site` call.
- Clauses 2 and 3 are not met. The repeat grows linearly with placeholders
  (about 1 MB and 1 ms at ten times the largest real sequence), and the
  peak does not grow.

The five-pass count describes this benchmark's valid, present, early-value
placeholders. It is not a count for every command: failed template grammar
uses a fallback scan, and late or missing placeholders can also trigger
presence-guard analysis. Those paths belong to the same preparation boundary.

## Behavior to build

Within one shell-site preparation, parse each placeholder **occurrence** once
for static analysis, and let every analysis pass read that retained syntax.
For example, `echo {{ name }} {{ name }}` has two occurrences with separate
source locations. Sharing syntax by expression text is unnecessary here.

- Scope includes every command that Claudine's
  [`SequencePreflight::prepare_site`](../../lib/src/composition/sequence/preflight/mod.rs)
  prepares: task `shell:` commands, collected setup/teardown shell commands
  (including `on_error`), and the inner command of whole-value `set` shell
  values. It does not migrate separate prompt or container discovery paths
  that do not call this method.
- Retained syntax lives only until that call returns. No syntax cache is
  added across sites, steps, preflights, retries, or launches. Stored approval
  templates keep their current representation and lifetime.
- Static analysis remains passive: parsing, approval checks, type projection,
  target-identity checks, late-root discovery, and presence-guard lookup do
  not evaluate expressions, read files, or invoke lazy providers.
- Evaluation remains unchanged. Each existing interpolation, presence
  evaluation, and runtime fill still reads the state appropriate to that
  operation, and file functions run whenever that evaluation reaches them.
  A command approved as resolved bytes still executes those fixed bytes;
  a command approved as a template still fills against runtime values.
- Frontmatter late-root scans, the removed-read check, and evaluation-time
  parsing are outside this optimization. Some of those paths also encounter
  the command text; their work is excluded because they are separate
  operations, not because they never read placeholders.
- Existing public text entry points retain their behavior. Add parsed-input
  counterparts only where needed to share the existing implementation;
  callers outside this preparation path need not migrate.

## Design decisions

### Keep syntax separate from the approval template

In Darkmatter, use one preparation-local collection of occurrences produced
by the existing interpolation scanner. Each occurrence carries its trimmed
text, its whole-placeholder byte range, its expression offset within the
command, and either parsed syntax or the original parse failure. Retain a
spanned syntax tree so approval diagnostics keep their source locations. The
existing Darkmatter
[`SpannedExpr::erase`](../../../darkmatter/lib/src/markdown/compose/expression/ast.rs)
can derive the unspanned form needed by type and root analysis without parsing
again; derive it at most once per occurrence if those consumers need it.

Darkmatter's
[`PreparedValue`](../../../darkmatter/lib/src/markdown/compose/expression/prepared.rs)
already separates passive syntax preparation from validation. Reuse its
scanner, parsing, and diagnostic machinery where applicable. Its current
all-or-error constructor cannot by itself retain valid occurrences beside a
malformed one; provide the smallest shared internal helper needed for the
site collection rather than changing that public constructor's contract or
introducing another expression parser.

> **Reader's note:** Retaining syntax only inside a successful command
> template would miss commands that fail template grammar but can still be
> approved as resolved bytes. Keeping the collection separate preserves that
> existing approval route and lets fallback analysis reuse the same syntax.

### Feed every static consumer from that collection

Darkmatter owns scanning, expression parsing, template grammar, type
projection, and late-root analysis. Claudine keeps its host-specific approval
scope and target-identity policy. Connect the collection to these consumers:

| Consumer | Required behavior with retained syntax |
| --- | --- |
| Darkmatter [authored grammar](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs) | Use the same scanner spans and initial untyped projection without parsing text again; preserve quoting, literal tokens, and unsupported-grammar errors. |
| Claudine [approval diagnostics](../../lib/src/composition/lifecycle/bindings.rs) | Validate retained spanned expressions through Darkmatter's existing validator, including unknown functions and unavailable globals in dead branches; preserve diagnostic ordering and relocation into the command. |
| Claudine [target-identity policy](../../lib/src/composition/sequence/preflight/shape.rs) | Use the same static-access traversal; preserve dotted/static-bracket equivalence and refusal of computed or numeric indexing that could select agent/model identity. |
| Darkmatter [type projection](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs) | Project against the current site's schema and guard, retaining validators and refinements; share the projection implementation with the text entry point. |
| Darkmatter [late-root discovery](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs) | Use current declared and derived roots, preserving their order and duplicates policy, including when template grammar failed. |
| Darkmatter [presence-guard lookup](../../../darkmatter/lib/src/lifecycle/shell_template.rs), used by Claudine | Use retained syntax for late and missing placeholders; a guard proves presence only for the expression it currently proves. |

Do not attach schema-derived types, guard facts, resolved values, or approval
verdicts to reusable syntax. These remain specific to the current site.
For `set` shell values, preserve the distinction between the raw wrapper used
by diagnostics and the extracted authored command used by template spans.
Map occurrence offsets between these views without reparsing, including any
trimming and accepted suffix syntax.

### Retain failures without reporting them early

Collecting a parse failure does not make it a new preflight error. Preserve
the existing decisions in order:

1. A malformed whole-value `set` wrapper still fails at extraction.
2. If any placeholder fails to parse, the approval-diagnostics pass still
   contributes no diagnostics, as its current all-or-error preparation does.
   Target-identity analysis still examines the other, valid occurrences.
3. Initial type projection still falls back for an expression that does not
   parse. Failed template grammar still permits fallback late-root analysis
   over valid occurrences; it must not hide a late value from the mode choice.
4. The existing choice between resolved bytes and required template mode
   decides whether and when to report a grammar failure. An authored
   program-word hole still removes template approval while permitting exact
   approval where it does today.
5. Resolution and missing-value handling still report their own typed errors
   at the existing point. Do not replace them with a preparation error merely
   because its syntax was collected earlier.

In particular, a command containing both an invalid expression and an
unavailable read must keep today's winning error. Preserve error variants,
messages, source chains, source locations, and approval withdrawals, not just
whether preflight fails.

## Contracts that must not change

- Approval membership and template shapes, including authored-program-hole
  withdrawal and exact/template approval of the first task.
- Late, deferred, and guarded hole timing (`HoleTiming`) and the late roots
  each hole reports.
- `MissingValue` and the unapprovable-read refusals, with their messages and
  sources.

## Acceptance

1. Extend Claudine's existing L1 work-count tests with `work-counters` enabled.
   For the measured shape (S steps, each `echo step-N` followed by H distinct,
   valid, present placeholders, with matching string schema and values),
   preflight records exactly `S × H × 6` `ExpressionParse`, down from
   `S × H × 10`. Each site's five analysis parses fall to one; the other five
   operations stay unchanged. Count parser invocations at the existing
   counter, rather than bypassing it through a different parser entry point.
   This formula applies to this fixture, not malformed or guarded commands.
2. At the cheapest in-process boundary, verify one static parse per
   occurrence for repeated text, late and guarded holes, and a command that
   fails template grammar. Include a command with no placeholders and literal
   interpolation syntax to prove scanner behavior is unchanged. Extend
   existing tests rather than creating a second test framework.
3. Behavior coverage through sequence preflight preserves:
   - malformed expressions alone and beside unavailable/target-identity
     reads, including which error wins and its typed source/location;
   - an authored program-word hole on a first task that permits exact
     approval, and on a later task that requires template approval;
   - late and guard-proven placeholders, including derived frontmatter
     roots, unchanged timing, validators, refinements, and reported roots;
   - missing values with and without a presence guard, plus present empty
     strings, zero, and false;
   - target-identity refusal for dotted, static-bracket, and computed access,
     and unavailable reads or unknown functions inside dead branches;
   - a representative collected stack command and whole-value `set` shell
     command, including preserved diagnostic offsets.
   Reuse existing cases where they already prove these contracts. Keep one
   representative normal CLI invocation using the area's isolated
   `CliProcessFixture`; no real provider, terminal, or shell approval prompt
   is needed.
4. A hermetic evaluation test changes a fixture file between evaluations of
   a file-reading placeholder and observes the changed result. A separate
   assertion preserves resolved-byte execution versus fresh template fills.
   This proves syntax sharing did not become value or verdict caching.
5. Re-run only the existing A6 allocation row and
   `A6_shell_preparation` Criterion case as a quick sample on one host.
   Record counts, bytes, and timing beside the figures above in the
   implementation log, with revision and build configuration. The earlier
   6.4% is an estimate, not a required measured speedup; account for retained
   syntax and conversion allocations in the actual result. These figures
   already justify implementation, so no further qualification spike,
   additional hosts, workload expansion, or statistical threshold is required.
6. `just test` and `just lint` pass in `claudine/` and `darkmatter/`, using
   their existing nextest recipes. Required counter tests must actually run
   with `work-counters` enabled. Do not run `cargo fmt` without an explicit
   author instruction.
7. Update Claudine's [pre-flight checks](../../docs/topics/pre-flight-checks.md)
   and the relevant Darkmatter template API docs/comments when implementation
   lands. Describe parsing once **for static analysis during one sequence
   shell-site preparation**, with evaluation and approval-mode behavior
   unchanged. Update applicable skills if new shared API usage needs guidance;
   do not link current documentation back to this dated spec.

## Open Questions

None. The scope is preparation-local syntax reuse with unchanged observable
behavior. The exact names of parsed-input helpers are implementation choices;
changing approval rules, error precedence, or evaluation timing would require
a separate design decision.
