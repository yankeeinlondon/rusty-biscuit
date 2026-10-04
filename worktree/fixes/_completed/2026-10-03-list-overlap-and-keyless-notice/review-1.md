---
$schema: feature-review.yaml
ready: false
findings:
    - title: Local gathering still waits for the remote worker
      priority: high
    - title: Successful API credential evidence and the keyless notice are missing
      priority: high
    - title: The performance report still adds overlapping work
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T14:42:48-07:00
spec: 2026-10-03-list-overlap-and-keyless-notice/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-10-03-list-overlap-and-keyless-notice/implementation-log.md
description: "A **fix** review of `2026-10-03-list-overlap-and-keyless-notice/spec.md`"
fix: 2026-10-03-list-overlap-and-keyless-notice/review-1.md
next: 2026-10-03-list-overlap-and-keyless-notice/review-2.md
---

# Review 1

The fix is **not production ready**. The reviewed checkout still implements the behavior the specification proposes replacing. Its two main changes have not landed. The specification itself remains marked `implemented: false`; the fix directory contains no implementation log or plan. This review evaluates the source present in this checkout rather than assuming an implementation exists elsewhere.

No human decision is needed to resolve these findings: the specification already defines the required behavior. This is the first implementation review, so no finding repeats an earlier implementation-review finding.

## Findings

### High — Local gathering still waits for the remote worker

**Defect class:** expensive local consumers run after remote waiting, and the pipeline has no speculative-result acceptance boundary.

In `worktree-cli`, [run_pipeline](../../cli/src/commands/list.rs:311) calls `gather_remote` synchronously, performs any fast-forward, rereads refs, and only then enters the scope containing the graph and list gathers. This preserves the latency problem described in the specification: network waiting and local computation add to each other.

The sibling sweep below follows the same pipeline for every local consumer. These are source observations, not claims that a held-worker runtime reproduction was run for each consumer. The existing bounded overlap test uses a repository without an origin and proves only list-versus-graph overlap.

| Site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| `worktree-cli` list gather | Remote work with unchanged tips | `fill_worktree_statuses` starts after `gather_remote` returns | Gather during the wait and accept the first result when both successful ref reads match |
| `worktree-cli` image graph gather | Image-capable path | Graph thread starts after waiting and ref rereading | Graph and list work overlap the remote wait |
| `worktree-cli` verbose gather | Verbose data applicable | Shares the same post-wait graph thread | History gathering overlaps the wait and participates in acceptance |
| `worktree` dirty-status walks | Ordinary listing or fetch | All status walks occur in the post-wait fill operation | Measure once during the wait and retain across a fetch |
| `worktree-cli` fast-forward path | `--ff` requested | Move precedes every local status walk; no use of the successful result's checkout path for a targeted refresh | Join speculative work before moving; refresh only the moved checkout afterward |
| `worktree` ref validation | Remote work followed or fast-forward attempted | `reread_refs` replaces the snapshot; no initial/final equality decision | Compare complete maps, including additions/deletions, and reject reuse if either read failed |
| `worktree` cache and record persistence | Filling listing results | Cache loading/saving, fork pruning, and copy-record pruning are coupled to the fill | Separate speculative computation from one accepted persistence pass; retain successful speculative SHA-pair entries |
| `worktree-cli` verbose labels | Commit history with branch decorations | [commit_details](../../cli/src/commands/git_graph.rs:867) and `commit_details_since` use live `%D` labels | Bind affected branch labels to the accepted tips while preserving tags and HEAD labels |
| `worktree-cli` no-origin and non-image controls | No origin; no image or verbose data | No worker without origin; conditional graph gathering remains present — clean existing controls | Preserve these conditions in the new pipeline |
| `worktree` failed-ref pruning guard | Ref read failed | Fork pruning requires `refs_read` — clean existing guard | Preserve the guard through both reads and final acceptance |

The affected library operation is [fill_worktree_statuses](../../lib/src/worktree.rs:286), which builds the table facts and commits persistent effects. Simply moving its current call earlier would introduce premature saves and pruning and would not implement safe discard/regather behavior. Separate those responsibilities before changing the scheduling.

Implement the specified acceptance decision once, then apply it to caption, target, tree, comparisons, graph, and verbose history together. Add the L1 bounded worker-versus-local-gather synchronization test; the existing [run_pipeline_gathers_the_graph_while_list_gather_is_unfinished](../../cli/src/commands/list/tests.rs:352) has `NO_PRS` and cannot establish this requirement. Also verify changed-tip advancement, rewind, addition/deletion, failed reads, targeted checkout refresh, and persistence counts through public results. Existing post-wait tests do not prove reuse or discard behavior.

Current pipeline descriptions in `worktree/README.md`, `worktree/docs/cli/list.md`, `worktree/docs/git-graph.md`, `worktree/docs/performance-testing.md`, and the worktree skill topic pages need to be updated when the implementation lands. Do not describe the overlap as implemented before that change.

### High — Successful API credential evidence and the keyless notice are missing

**Defect class:** successful requests lose authentication evidence before publication, so the listing cannot distinguish observed anonymous success from keyed or unknown success.

In `sniff`, [branch_head_with](../../../sniff/lib/src/remote/blocking.rs:299) and [open_pull_requests_with](../../../sniff/lib/src/remote/blocking.rs:241) return their payloads without successful-request credential metadata. In `worktree`, [the successful head check](../../lib/src/remote_update.rs:348) explicitly creates a report with `api: None`, and the PR writer stores no successful authentication state. In `worktree-cli`, [credential_line](../../cli/src/commands/list.rs:220) can render only confirmed failure conditions.

I reproduced the output defect through the shipped `wt list --perf` binary using a temporary L1 probe copied from the existing `MixedFixture`/`FakeGitea` setup. The fixture removes provider tokens, isolates configuration, and sends requests only to the local stand-in. I changed only which API half succeeded: empty PR success plus head HTTP 500, head success plus PR HTTP 500, then both successes. Head-success cases served the fixture's real bare repository so the worker could fetch. Each run completed with one PR request and one head request, and all three omitted `answered without an API key`. The probe was removed after execution.

| Site | Shape tested or inspected | Observed result | Expected result |
| --- | --- | --- | --- |
| Shipped CLI, PR-only success | Anonymous empty PR answer; generic head failure | No keyless notice — reproduced | One Gitea authentication notice; generic failure must not suppress it |
| Shipped CLI, head-only success | Anonymous head answer and successful fetch; generic PR failure | No keyless notice — reproduced | Notice survives the API-to-fetch transition |
| Shipped CLI, both successful | Anonymous head and empty PR answers | No keyless notice — reproduced | Exactly one credentials line |
| `sniff` branch-head request path | Successful response | Payload-only blocking result — source inspection | Return the sending client's actual credential selection |
| `sniff` open-PR request path | Successful response, including pagination | Payload-only blocking result — source inspection | Return evidence covering every page; claim anonymous only when every request was anonymous |
| `worktree` head attempt writer/reader | Current successful API attempt | Format 2; success has no credential evidence — source inspection | Format 3 with independent valid-answer handling; format 2 answer retained but attempt dropped |
| `worktree` PR publication writer/reader | Successful publication, including empty answers | Format 5; publication ID but no authentication evidence — source inspection | Format 6 metadata atomically bound to publication; valid format 5 answers retain unknown credentials |
| `worktree-cli` wait projection | New publication or adopted head attempt | PR success is a bare `PrEnd::Published`; no successful PR metadata capture — source inspection | Capture metadata from exactly the accepted publication and followed head attempt without waiting for a receipt |
| `worktree-cli` credentials selection | Confirmed head and PR failures | Existing head-warning-first precedence — clean source control | Preserve warnings above the new success notice |
| `worktree-cli` suppression controls | No successful evidence; local-path provider lookup | Existing failure-only helper remains silent — clean L1 controls | Preserve unknown/cached/local-path/unsupported suppression; add ignored/changed-origin success suppression |

Implement this as one end-to-end change from `sniff` request selection through worker publication, wait capture, and rendering. Do not infer success credentials by inspecting the foreground environment: an adopted worker may have different credentials. The existing `worktree` [SniffBranchHeads::key_in_use](../../lib/src/remote_update.rs:64) duplicates token lookup and explicitly misses host-bound overrides; it cannot provide the specified request evidence.

The input-robustness extension is missing with the new representation. There are existing JSON matrix tests for the old stores, but neither new authentication field exists, and no outcome table covers its absent, null, wrong whole type, wrong element, all-wrong elements, empty, duplicate, or trailing-content shapes. Extend both real-writer fixture matrices through public cache/attempt selection results. For the head store, bad attempt metadata must leave a valid answer usable; for the new PR store it must reject the publication. Legacy credentials must be unknown rather than anonymous. This review does not assert that malformed new metadata is accepted: the new formats are not implemented at all.

Required L1 verification remains absent for keyed/anonymous mixing, host-bound selection, pagination, adoption with another environment, publication before receipt, lock contention, origin changes, warning precedence, fallback coexistence, and token-value exclusion for the new evidence. The new dim line also lacks its required **L2** verification. The existing [credentials-warning terminal test](../../cli/tests/level2_list_verbose.rs:1049) tests a failed head request, not successful anonymous output. Extend it to capture the new line below the caption after spinner cleanup, using the existing windowless tmux harness. No L3 keyboard test is needed for this output-only requirement.

`worktree/docs/cli/list.md` currently states that success is silent with or without a key. Update that contract, the README, the remote-store skill page, and relevant `sniff` documentation alongside implementation.

### Medium — The performance report still adds overlapping work

**Defect class:** the timing projection treats concurrent durations as additive stages and hides excess attribution with saturating subtraction.

In `worktree-cli`, [PerfCollector::build_perf_tree](../../cli/src/perf.rs:58) builds every recorded stage as a top-level leaf, sums all stage durations, and calculates `unattributed` with `saturating_sub`. List and graph gathering already overlap, so their combined attributed duration can exceed elapsed time. This violates both the requested grouping and reconciliation rules.

| Site | Shape tested or inspected | Observed result | Expected result |
| --- | --- | --- | --- |
| CLI remote-plus-local report | Each of the three anonymous-success probe runs | No `remote wait ‖ local gather` group — reproduced | One directly measured concurrent span, with individual child durations |
| CLI local-only pipeline | No origin | Records a separate zero-duration `remote wait` stage — source inspection | Local-only group with no remote-wait child |
| Collector accounting | Overlapping list and graph stages | Both added to top-level sum; underflow clipped — source inspection | Count only the group's elapsed duration at the top level |
| Metric projection | Non-root nodes | Every node gets a share of wall time — source inspection | Concurrent diagnostic children have durations without percentages |
| Post-wait PR read | `RemoteAnswers::pr_gather` | Combined with pre-wait lookup in one standalone stage — source inspection | Keep discoverable timing without double-counting an overlapping portion |
| Regather and checkout refresh | Tips change or `--ff` moves a checkout | No such timing groups/stages — source inspection | Conditional measured `regather` group and separate affected-checkout refresh |
| Stage parser | [stage_from_perf](../../cli/tests/perf_support/mod.rs:793) | First substring match — source inspection | Resolve intended nested stage; an enclosing group containing `remote wait` must not become the wait-child measurement |
| Existing reconciliation control | Two small sequential durations | Test checks this additive case — clean existing control | Also verify overlapping children, nested groups, and conditional rows |

Introduce measured groups in the collector rather than reconstructing them from child maxima. Update consumers before adding a parent name containing a child name, and verify that top-level children plus `unattributed` equal total elapsed time without clipping away double-counted duration. Preserve the existing graph/verbose conditions and stage names. Update the performance topic page and record the requested quick release sample once the overlap exists; a baseline sample cannot validate this unimplemented change.

## Requirement and verification coverage

| User-facing requirement | Appropriate level | Evidence currently present | Review result |
| --- | --- | --- | --- |
| Local work runs during remote waiting | L1 bounded synchronization | List-versus-graph rendezvous with no remote | Missing proof and implementation |
| Unchanged tips reuse work; changed/failed reads select final results | L1 with real Git fixtures and work counters | Existing post-wait behavior | Missing acceptance-path proof and implementation |
| Fast-forward refreshes only the moved checkout's status | L1 | Every checkout measured after fast-forward | Missing targeted-refresh proof and implementation |
| Persistent effects occur once after acceptance | L1 public state and operation counts | Effects coupled to the only gather | Missing speculative-discard proof and implementation |
| Timeout remains bounded and detached work continues | L1 | Existing wait/worker tests | Baseline mechanisms present; overlap interactions unverified |
| Nested timing reports reconcile and parsers select intended stages | L1 | Flat collector and sequential reconciliation control | Missing grouped-report proof and implementation |
| Anonymous success is recorded, migrated, selected, and announced correctly | L1 | Old-store matrices and failure-warning tests | Missing evidence and notice; CLI reproduction confirms absence |
| New credentials line is dim, below caption, after spinner cleanup | L2 real-terminal capture | Failure-warning capture only | Required new L2 coverage missing; readiness blocker |

The L2 target is declared with `required-features = ["terminal-tests"]`; `just test-l2` enables that feature and CI metadata includes it. The coverage gap is an absent assertion of the new behavior, not an undeclared or stranded new test. No new permanent tests were added by this review.

## Validation

Review scope: checked the local CLI pipeline, library status/comparison/persistence operation, graph and verbose history, timing collector and parsers, both blocking provider API result paths, both worker stores, wait projections, existing L1 and L2 tests, and the required documentation surfaces.

- `just test credential`: 8 passed. These prove existing failure-warning behavior, not the new notice.
- Temporary `just test review_probe --nocapture`: one probe passed while asserting the current defective behavior across all three successful-API combinations. It is reproduction evidence, not a fix regression test, and was removed.
- `just test input_robustness_matrix`: 3 passed, covering the existing head store, PR store, and receipt. These matrices do not cover the proposed success metadata.
- `just test run_pipeline_gathers`: 4 passed across the CLI library and binary targets. These cover local list/graph overlap, not overlap with remote waiting.
- `just test build_perf_tree`: 2 passed across the CLI library and binary targets. These exercise the existing sequential reconciliation control.
- `just check-tier-coverage worktree`: passed; no stranded tests.

Full L1, L2, performance, lint, and affected `sniff` gates were not run. The missing implementation is already established by source inspection and the network-free CLI reproduction. Passing baseline gates would not make this fix ready. Cross-platform proof is left to CI and does not contribute to the readiness decision.
