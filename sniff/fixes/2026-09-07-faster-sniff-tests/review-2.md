---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-09T16:56:02-07:00
spec: 2026-09-07-faster-sniff-tests/spec.md
implemented: false
description: "A **fix** review of `2026-09-07-faster-sniff-tests/spec.md`"
fix: 2026-09-07-faster-sniff-tests/review-2.md
previous: 2026-09-07-faster-sniff-tests/review-1.md
---

# Review 2 — Faster Sniff Tests

## Verdict

The fix is **not ready for production**. Review 1's cross-platform correctness
gap is closed: the candidate now passes Level 1 on macOS, Linux, WSL2, and
native Windows, and the focused fixture/guard targets pass in this review.

The performance outcome is still open. The new matched local evidence finds no
measurable full-L1 improvement and a consistently slower `sanity` cohort. More
importantly, the implementation's own attribution identifies ordinary Level 1
tests that still walk and hash the live monorepo. Those tests violate the spec's
owned-input contract and make the fast-confidence path slower as unrelated
tracked files accumulate. The fluent CWD and PATH-escape guards also claim
stronger structural guarantees than they enforce.

No human review is required yet. Each finding has a mechanical remediation and
can be verified by an agent before any product or UX decision is needed.

## Findings

### High — Live-monorepo document tests remain in `sanity` and dominate its regression

Six ordinary library tests still construct `RepoDocuments` or call
`detect_docs` with `Path::new(".")`, causing them to discover, enumerate, read,
and hash the mutable rusty-biscuit checkout
([`docs.rs:1556`](../../lib/src/filesystem/docs.rs#L1556),
[`docs.rs:1566`](../../lib/src/filesystem/docs.rs#L1566),
[`docs.rs:1592`](../../lib/src/filesystem/docs.rs#L1592),
[`docs.rs:1602`](../../lib/src/filesystem/docs.rs#L1602),
[`docs.rs:1614`](../../lib/src/filesystem/docs.rs#L1614),
[`docs.rs:1626`](../../lib/src/filesystem/docs.rs#L1626)). The inventory labels
the entire `lib-unit-filesystem` family as using owned filesystem fixtures, so
the audit disposition is incorrect.

This is not theoretical. The implementation attributes the five largest
candidate `sanity` deltas to tests in this six-test group and explains that 34
additional Markdown files and roughly 9.4 MB of tracked fix artifacts increased
their work
([`summary.md:200`](./measurement/review1-alternating/summary.md#L200)). The
candidate was slower in all five paired `sanity` rounds. Adding unrelated
documentation therefore increases the cost of the fast-confidence suite and
can change the assertions' input without changing Sniff.

Replace these cases with a small, static disposable repository containing the
package and document shapes each assertion needs. If one real-checkout smoke
test provides distinct value, keep one explicitly classified native/integration
case outside `sanity`, assert a portable invariant, and measure it separately.
Update the family disposition and add a work assertion that fixture size—not
checkout size—bounds document discovery.

Strongest verification present: Level 1 execution against the live checkout.
Level 1 is the correct boundary, but the input ownership and work bound are
wrong; passing does not prove deterministic or repository-size-independent
behavior.

### High — The required faster-suite outcome is still not demonstrated

The clean alternating comparison reports full-L1 medians of 23.66 seconds for
baseline and 23.36 seconds for candidate, a 1.3% difference inside a 69.4%
drift bracket. `sanity` moved from 11.10 to 12.01 seconds and was slower in all
five pairs ([`summary.md:14`](./measurement/review1-alternating/summary.md#L14)).
The results file consequently states that faster tests were not demonstrated
and AC8 is only partially verified
([`results.md:10`](./results.md#L10), [`results.md:268`](./results.md#L268)).
Matched per-family CI budgets also remain deferred, although missing cross-OS
CI proof is not by itself a readiness gate under the review instructions.

The implementation usefully adds 19 tests without increasing measured full-L1
wall time, but that is not the specified outcome and cannot be reported as a
speedup. Complete the owned-input remediation above, remove or relocate bulky
measurement artifacts from paths traversed by tests, and repeat the same pinned,
warm, alternating protocol. Ratify the remaining per-family budgets from the
required compatible samples. Readiness requires evidence that the affected
cohorts became faster or an explicit revision of the specification's outcome;
the current negative result cannot close it.

Strongest verification present: Level 1 matched timing and work-counter
evidence. It proves selected incidental Git work was removed and proves the
current candidate is not measurably faster; it does not verify the headline
performance requirement.

### Medium — The new structural guards accept unowned context and semantically empty reasons

`assert_disposable_context` accepts any existing directory beneath the
process-wide temporary root
([`common/mod.rs:244`](../../cli/tests/common/mod.rs#L244)). Being under that
root does not prove that this test created, owns, or may delete the directory;
another process's temporary repository is accepted. The negative test uses a
system directory outside the temp root, so it does not distinguish the claimed
ownership contract from the weaker location check. This leaves Review 1's
fluent-CWD finding only partially resolved.

The PATH escape guard similarly treats every non-empty adjacent `//` comment as
a valid justification: `unjustified` checks only `reason.is_none()`
([`spawn_site_guard.rs:304`](../../cli/tests/spawn_site_guard.rs#L304)). A comment
such as `// temporary workaround` passes even though the failure message and
spec require the real tool observed or the absence proved. Its mutation test
only removes the detector, not the semantic check, so it cannot catch this
failure mode.

Make the fluent command retain ownership of the disposable directory, or
replace fluent CWD mutation with a fixture API that creates/owns the requested
repository. Add a negative case for an existing but separately owned directory
under the temp root. For PATH escapes, use typed APIs that require a non-empty
tool/absence value and render the explanatory comment/report from that value;
at minimum, validate structured reason kinds and add negative semantic cases.

Strongest verification present: Level 1 unit tests of path containment and
source scanning. The level is appropriate, but the assertions do not
distinguish the promised contracts from weaker implementations.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Deterministic CLI environment, CWD, PATH, Git plumbing, JSON/stdout/stderr, and exit behavior | Level 1 real-binary integration tests | Correct level; cross-platform execution is now green, but fluent CWD ownership remains incompletely enforced. |
| Deterministic repository/document tests use owned inputs and bounded work | Level 1 in-process/filesystem tests | Gap: six document cases consume the live monorepo, including five that enumerate and hash its documents, so results and cost vary with unrelated files. |
| Requested work, observation reuse, and worker counter propagation | Level 1 in-process counter tests | Correct level and useful proof for selected paths; it does not cover the live document-walk cost. |
| CI/CD and Git glyphs, SGR styles, links, layout, and scrolling | Level 2 tmux pane capture through `just test-l2` | Correct level; existing evidence records two required-tmux tests executed and passing. |
| Keyboard, hotkey, paste, IME, or mouse behavior | No such changed requirement | Level 3 is not applicable. |
| Faster full-L1 and `sanity` cohorts | Level 1 alternating timing artifacts | Correct measurement boundary, but the result is negative and per-family CI budgets remain pending. |

## Validation Performed

- `cargo test --color=never -p sniff-cli --test cli_process_fixture --test spawn_site_guard`
  passed: 22 tests, 0 failed.
- `cargo test --color=never -p sniff --features remote filesystem::docs::tests::integration:: -- --nocapture`
  passed: 6 tests, 0 failed; the six live-repository cases completed in 1.17
  seconds after compilation.
- Reviewed the specification, Review 1, staged implementation changes,
  reconciled inventory, results ledger, deferred measurement record, alternating
  run provenance/summary, and the affected test sources.

## Production Readiness

Not ready. Cross-platform functional verification is substantially improved,
but the suite still contains an identified live-repository cost that violates
the isolation contract, the new guards do not fully enforce their documented
invariants, and the candidate has not demonstrated that the affected Sniff test
cohorts are faster.
