---
kind: implementation-log
---

# Documentation audit: 2026-10-24-sequence-containers

Updated **5 existing canonical documents**; created **0 documents**. The
completed spec now lists eight canonical documents through implicit
repository-relative paths. Every referenced document declares
`kind: canonical-documentation`; the specification's snapshot body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `claudine/docs/topics/flow-control/sequences.md` | Recursive primitives, grammar, group concurrency and atomicity, lexical roots, native results, preflight, recovery, cancellation | Added an introductory choice between ordered and concurrent work; corrected the stale static-empty-list prohibition; qualified retry mutation rules for looping containers. |
| `claudine/docs/cli/sequence.md` | Invocation, setters, provider selection, flags, exit codes, dry-run | Added canonical kind and corrected the output-root name in the introduction. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Shared container/document events, finalization exceptions, recovery, cancellation restrictions | Qualified the container retry summary: looping containers restore the current pass's checkpoint before applying retry values. |
| `claudine/docs/topics/schemas.md` | Recursive sequence/group schemas, event restrictions, parser-only rules, embedded deployment | Reviewed current; no edit needed in this step. |
| `claudine/docs/topics/state-management/context-variables.md` | Document, group, retry, and loop capture boundaries | Reviewed current; no edit needed in this step. |
| `claudine/docs/topics/pre-flight-checks.md` | Closed transitive shell inventory, exact/template approvals, frozen membership, container event shells | Reviewed current after the preceding audit; no edit needed in this step. |
| `claudine/docs/topics/signal-handling.md` | Ctrl-C, descendant settlement, cleanup, process ownership, platform delivery | Added canonical kind and corrected the introductory count of signal surfaces. |
| `messenger/docs/user-guide.md` | Messenger-owned fleet lock and process tree, runner observations, durable admissions, explicit stale recovery, legacy records, promotion checks | Added canonical kind, distinguished runner outcome from research status, and documented readable/resumable but non-promotable legacy exhausted records. |

No new page was necessary. The sequence topic is the obvious developer entry
point and already provides nested examples, result shapes, lifecycle and
cancellation diagrams, and links to CLI and related topics. Its existing
executable example retains `kind: sequence`, rather than being misclassified
as canonical documentation; it is not included in `canonical_docs`.

## Drift resolved

The old empty-list paragraph contradicted both the earlier container section
and current tests: authored and dynamic empty sequences are valid, run their
own lifecycle, and return `[]`. The replacement also explains why a nested
empty sequence contributes one empty array while a skipped child contributes
nothing. The source comment with the same obsolete prohibition was reported
to the test agent, who owns code changes.

Later loop work introduced a checkpoint exception to the original retry
rule. Both summaries now distinguish non-looping containers, whose runtime
writes survive retry, from looping containers, whose current-pass outputs and
writes return to the pass's entry. Evaluated retry values apply after either
reset. The original specification remains a snapshot.

Messenger's guide already describes the fleet lock, interruption and liveness
checks, explicit stale recovery, elapsed-time and invocation limits, and
cleanup/promotion refusals. The added paragraphs make two less obvious
contracts explicit: process exit zero is not research approval, and old
`exhausted` records remain subject to stage checks and the recovery limit.

## Validation

- Compared empty-sequence behavior with `sequence/tests.rs`, nested task
  results, and the root CLI's empty-plan lifecycle path.
- Compared retry descriptions with the sequence/group task runners,
  `RootRun::retry`, and the loop guide's checkpoint contract.
- Checked Messenger claims against `runner.rs`, `RunStatus`, `prepare::resume`,
  and promotion's `AwaitingReview` requirement.
- Verified all eight canonical paths exist beneath package-area `docs/`
  folders and have the required frontmatter kind; verified no referenced page
  contains a feature/fix snapshot reference.
- Verified the spec body is byte-for-byte unchanged from HEAD and the new
  retry link's target heading exists. `git diff --check` passed.

The documentation agent ran no test suites; the concurrent test agent owns
behavioral verification and timing. No skill change was needed: the existing
Claudine skill already distinguishes looping checkpoints from ordinary retry.
