---
kind: implementation-log
---

# Documentation audit: 2026-10-20-lifecycle-grammar

Updated **7 existing canonical documents**; created **0 documents**. The
completed specification now lists eight canonical documents using implicit
repository-relative references. Its snapshot body is unchanged.

| Canonical document | Requirements covered | Change in this step |
| --- | --- | --- |
| `claudine/docs/topics/flow-control/lifecycle.md` | Lists, dictionary bundles, conditions and nesting, action forms, typed operands, `no_error`, removed spellings, audio order, source paths, loop gate grammar and globals | Added canonical kind and clarified passive known-call rejection versus unevaluated operands. Existing complete examples, canonical-order table, branch rules, lifecycle diagram, and removed-spellings guide remain the main author entry point. |
| `claudine/docs/topics/flow-control/looping.md` | `loop.*`, configuration versus runtime values, controls, lifecycle gate, loose verbs versus ordered gate | Added canonical kind; reviewed the current examples and linked gate grammar. |
| `claudine/docs/topics/flow-control/flow-control.md` | Directive selection, short/long parameter inventory, returning `prep`, `break`, planned `defer` | Added canonical kind; retained current integrated directive behavior. |
| `claudine/docs/topics/flow-control/flow-control-reference.md` | Retry budgets, one-attempt `retry.with`, persistent proxy overlay, typed overlay evaluation and transient data | Added canonical kind; corrected a contradictory claim that `resume` does not reread the document. It continues the provider session while rereading and validating the document. |
| `claudine/docs/topics/schemas.md` | Generated verb shapes, recursive action/conditional schemas, parser-only rules, passive editor support | Changed its previous overview kind to canonical documentation and clarified that returning `prep` is exempt from the last-item rule. Recursive schemas, the five-level parser limit, and typed argument rules are already documented. |
| `darkmatter/docs/topics/lifecycle/index.md` | Shared grammar and host ownership after the later extraction | Added canonical kind and a compact, example-led guide to dictionary order, short and named forms, `no_error`, empty stacks, and nonempty explicit branches. This lets an unfamiliar `md` user understand the shared grammar without starting with the Claudine reference. |
| `darkmatter/dmls/docs/diagnostics.md` | Nested-span diagnostics, authored paths and editor checking | Corrected stale text claiming only shallow conditional predicates are checked. Recursive schema-typed predicates are checked at nested levels; the five-level limit remains the runtime parser's responsibility. Kind was already canonical. |
| `darkmatter/docs/inline/interpolation.md` | Literal and single-pass interpolation, lifecycle predicate and operand surfaces | Reviewed unchanged; already current and canonical. |

The implementation has evolved beyond the snapshot: Darkmatter owns the shared
parser, `cancelled` is present, recursive editor schemas cover arbitrary depth,
and `prep` returns rather than ending an event. The canonical pages retain
those integrated behaviors instead of restoring the snapshot's historical
ownership, six-event list, shallow editor typing, or planned-only directives.

## Validation

Compared the pages with the shared parser's canonical ranking, empty-stack,
branch, parameter, `no_error`, depth, and returning-directive rules; the
executor's evaluation/dispatch failure distinction; the lifecycle source-path
walker; and the recursive editor predicate classifier. Coordinated with the
test agent to confirm that proven incompatible known calls are rejected in
both branches, while an unknown function in an ordinary unselected operand is
not executed. Shell approval performs its separate passive checks.

Validated all eight references exist under package-area `docs/` folders and
have `kind: canonical-documentation`. Checked the new link target and anchor,
the absence of dated feature/fix references in these canonical pages, and the
byte-for-byte unchanged specification body. `git diff --check` passed. No test
suite was run by the documentation agent; test execution belongs to the
concurrent test agent. No skill change was needed: relevant skill summaries
already describe the recursive, shared lifecycle grammar.
