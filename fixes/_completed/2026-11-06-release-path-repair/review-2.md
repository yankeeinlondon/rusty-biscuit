---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Release calculation remains unable to produce the approved proposal'
  - 'R2-F1 (high): Local HTTP fixtures depend on host name resolution and time out'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-08T20:28:59-08:00'
spec: 2026-11-06-release-path-repair/spec.md
implemented: true
description: 'A **fix** review of `2026-11-06-release-path-repair/spec.md`'
fix: 2026-11-06-release-path-repair/review-2.md
previous: 2026-11-06-release-path-repair/review-1.md
next: 2026-11-06-release-path-repair/review-3.md
log: /Volumes/coding/rusty-biscuit/fixes/2026-11-06-release-path-repair/log.md
implemented_by: claude/opus
human_review_items:
  - 'R1-F1: the real unchanged-tag calculation proposes sniff-detection-cli 0.1.0 -> 0.2.0 (a feat! commit since 907cd6e), not the 0.1.1 the re-tag diagnostic showed; confirm before approving the proposal'
  - 'R1-F1.7: chose an in-repo patch of release-plz 0.4.3 built from its SHA-pinned upstream archive and cached, over moving tags or waiting for upstream; confirm or reverse'
---

# Release path repair review, iteration 2

The fix is not production ready. The historical checkout repair now has a passing regression, but the real release calculation still fails with the existing tags. Two focused tests also timed out because their local HTTP server resolves a host name during startup. No publication, tag mutation, or credential change was performed during this review.

## Previous Finding Verification

Review-1.md contains one finding, expanded into eight rows in the repair's Finding Ledger. Each is checked below. There is no `Human rulings` section in review-1.md. Its informational request for a tag decision is an implementer's request, not an author ruling. The spec's two-package ruling remains authoritative.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Release calculation cannot produce the approved proposal | All ledger rows .1–.8 | partially fixed | Checked setup, both calculation callers, Git regressions, publication/recovery compatibility, reports, actual tags and retained Linux evidence. | Successful calculation with unchanged tags and exact approved edits. |
| R1-F1.1 — Dry run/shared setup | Working pin, successful Linux calculation, approved edits | partially fixed | Setup verifies 0.4.3; all three jobs use it; dry run calls shared `update`. Actual Linux replay exits 1 on historical package lookup. | Successful real-tool calculation. |
| R1-F1.2 — Proposal environment | Shared repaired environment, intact clean-tree checks, completed calculation | partially fixed | Preparation enters `calculation_environment` before adding credentials. Driver checks inherited config, credential ordering, cleanup and clean checkout; diagnostic control executes successfully. | Actual preparation inherits the lookup failure; its HTTP fixture also needs R2-F1. |
| R1-F1.3 — Historical checkout regression | Original failure plus repaired-path success | verified fixed | Both history-walk tests passed. Success uses production `calculation_env`, checks the repository's PowerShell rule and returns to a clean tree. | Nothing. |
| R1-F1.4 — Publication/recovery | Replacement-version compatibility, exact head, independent tag/Release switches | verified fixed | Read retained 0.4.3 `command/release.rs` and `git_cmd/src/lib.rs`: local-tag early return and independent switches support current recovery. Pin guard, exact-head, rerun, recovery and partial-failure contracts pass. | Nothing within this repair; live publication remains post-review work. |
| R1-F1.5 — Reports/candidates | Retain checks and record actual successful calculation | partially fixed | Candidate code requires exactly the approved set. Report fixture passes with the diagnostic name-resolution bypass. Retained successful diagnostic moved local tags. | Success and report using real unchanged tags, including generated files and no dependency bump. |
| R1-F1.6 — Porcelain status column | Preserve first line's leading index column | verified fixed | `calculation_report` uses `_git_lines`, preserving leading whitespace. Inspected report assertions and ran their case with the diagnostic bypass. | Nothing. |
| R1-F1.7 — Renamed-package tags | All three tags and historical names; remove lookup failure | unresolved | Independently peeled tags to `907cd6e4522ed57790588f3685d4c5d55a3a7455`; manifests contain old names. Upstream collects all Git-only packages and looks them up by current name. | Repair historical identity lookup without changing published tags. |
| R1-F1.8 — Remaining callers/environment builders | Dry run, preparation, publication/recovery, credentials, diagnostics | verified fixed | Searched callers: calculations share environment; publication checks out only final proposal commit; credentials append config; diagnostics remain read-only. | Nothing beyond the shared failure already carried above. |

## Unblocked Findings

### R1-F1 (high): Release calculation remains unable to produce the approved proposal

**Authority: spec.** Acceptance item 1 requires: “The Linux dry run succeeds and the historical checkout failure has a documented cause and focused regression check.” The first-release ruling also requires exactly `sniff-detection-cli` and `git-wt-cli`, without an unreleased dependency bump. The unchanged-tag calculation still exits unsuccessfully.

The defect class remains release-tool compatibility selected without a successful calculation against this repository's real history.

The packaging incompatibility and CRLF checkout failure have concrete repairs. The next failure is historical identity: release-plz 0.4.3 reconstructs a package's tagged workspace and searches it by the package's current name. Three current-name tags point to a commit before those names existed. Its `next_versions` partitions all workspace packages by Git-only mode before historical collection, so excluding `darkmatter-lsp` from release eligibility does not bypass its lookup.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | Dry run/shared setup | Actual tags, repair config/environment, 0.4.3 Linux replay | Exit 1 before edits | Successful calculation with approved edits |
| R1-F1.2 | Proposal preparation | Shared environment and command wiring | Environment repaired; actual calculation inherits lookup failure | Completed calculation and clean checkout |
| R1-F1.3 | Historical checkout | Same stale-stat history, without/with production environment | Original failure and repaired success reproduced | Clean: preserve both regressions |
| R1-F1.4 | Publication/recovery | Exact head, complete rerun, tag-only recovery, partial failure | Contracts pass; inspected source supports orchestration | Clean: preserve on tool replacement |
| R1-F1.5 | Reports/candidate decisions | Actual-tag failure, diagnostic local re-tag success, report fixture | Exact-set checks retained; successful diagnostic changes tags | Acceptance report with unchanged tags |
| R1-F1.6 | Tracked-change projection | First porcelain line starts with space | Column preserved and asserted | Clean |
| R1-F1.7a | Historical `git-wt-cli` lookup | `git-wt-cli-v0.1.0` → `907cd6e…`; `worktree/cli/Cargo.toml` | Historical name `worktree-cli`; actual run aborts on current name | Resolve historical identity, retain tag |
| R1-F1.7b | Historical `sniff-detection-cli` lookup | Same target; `sniff/cli/Cargo.toml` | Historical name `sniff-cli`; same lookup cannot match | Resolve historical identity, retain baseline |
| R1-F1.7c | Historical excluded `darkmatter-lsp` lookup | Same target; `darkmatter/dmls/Cargo.toml` | Historical name `dmls`; collection includes excluded package | Handle historical identity despite exclusion |
| R1-F1.8 | Other tool/environment consumers | Credentials, diagnosis, publication environment | No additional calculation caller bypasses repair | Clean |

The suffixes under .7 enumerate that ledger row's three instances. The actual replay stops at `git-wt-cli`; the other two are established from their historical manifests and the same unconditional upstream lookup, rather than claiming the aborted run reached them. Read the actual exit status and stderr under `/tmp/rpr/evidence-r1/r1-update-0.4.3.*`; another identical expensive failed run was unnecessary. The changed-tag diagnostic reaches exactly both 0.1.0→0.1.1 bumps, but cannot close acceptance.

**Recommended repair:** preserve published tags and repair the pinned release tool's historical-package lookup. Prefer a compatible upstream build demonstrated to work; otherwise a narrowly patched, reproducibly pinned release-plz build can resolve these renamed packages by their stable repository-relative manifest paths when their current names are absent. Release-plz still owns calculation and publication. Test all three names and the normal current-name path; then run one bounded unchanged-tag Linux calculation, retain its generated files, and recheck publication/recovery against the chosen build.

This is a reversible implementation choice within the spec's tool/configuration/regression scope, not a required change to published history or release policy. It does not claim an untested patch already works. The log's claim that every repair requires an author ruling is too strong. Correct the docs' statement that resolution requires “only” a decision about the tags alongside the repair. Do not move remote tags, change tag syntax, or enlarge the approved package set.

### R2-F1 (high): Local HTTP fixtures depend on host name resolution and time out

**Authority: defect.** The changed driver's `FakeApi` constructs `ThreadingHTTPServer` at line 891. With the ordinary `just test release` command on this host, two release contract tests exceeded Nextest's 30-second timeout before reaching HTTP assertions. The repository's testing policy treats a Level-1 test over five seconds as failing.

The defect class is local fixture startup performing unnecessary ambient host name resolution.

Python's `HTTPServer.server_bind` calls `socket.getfqdn(host)` even for numeric loopback. An eight-second bounded reproduction with a four-second `faulthandler` dump stopped in `socket.py:getfqdn`, called by `http/server.py:server_bind` and `FakeApi.__init__`. Proxy bypass does not prevent that startup lookup.

The sweep found one constructor and three consumers. Each existing case was launched first with `socket.getfqdn` raising a diagnostic error, then with it returning the supplied numeric name as a control. These experiments changed no repository files.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | `FakeApi.__init__` / `serving` | Numeric loopback binding with slow/unavailable name lookup | Native stack blocks before serving | Bind without host name lookup |
| R2-F1.2 | `github_forge_reads_release_state_over_http` | Existing positive API fixture | Diagnostic rejection fails at constructor; numeric-name control passes in 0.11 s | Positive assertions execute with isolated local setup |
| R2-F1.3 | `github_forge_never_reads_a_failure_or_malformed_response_as_absence` | Existing invalid/API-failure fixture | Shipped test times out at 30.026 s; same constructor dependency; control passes in 0.12 s | Error assertions finish within Level-1 budget |
| R2-F1.4 | `release_pr_refuses_incomplete_credentials_and_leaves_the_checkout_clean` | Existing proposal fixture | Shipped test times out at 30.015 s; same dependency; control passes in 0.64 s | Credential/environment/clean-tree assertions finish within budget |
| R2-F1.5 | Remaining changed release/integration boundaries | In-memory Forge, recording tool, numeric HTTP URLs, local Git, closed-port probe | No additional server constructor or startup host-name lookup | Clean for this class |

**Smallest repair:** use a fixture-local HTTP server subclass whose bind calls `socketserver.TCPServer.server_bind` and assigns the known numeric server name and actual port directly. Add a guard that rejects name resolution during construction, then rerun all three consumers. Keep production networking, test cases and timeouts intact; retries or a slower tier do not repair this dependency.

## Blocked Findings

None. Neither repair needs credentials, publication, or an author decision. Existing post-review authorization boundaries remain intact.

## Recurrence

R1-F1 repeats review-1.md's release-tool compatibility class: the same dry-run/preparation failure now exposes another upstream assumption after pin and CRLF repairs. It is an unchanged failing instance, not a regression of a successful real calculation. The previous sweep already included both callers and the report; this review follows the newly exposed identity cause through all three packages and the eligibility decision that includes the excluded package. The requirement has a concrete boundary: this repository's actual history and unchanged tags, with the approved two-package set.

R2-F1 is new and concerns test-server startup, not another release-calculation sibling.

## Verification and limits

- `cd tools && just test release`: 35 selected, 33 passed, two timed out at 30 seconds. One selected passing test belongs to audio-spool because its name contains `release`. No successful normal-run result is claimed for the timed-out tests.
- `cd tools && just test integration`: the integration workflow contract passed. Integration selection cases also passed within the release driver selection.
- actionlint 1.7.12 from `/tmp/actionlint.xrKp/actionlint` on both changed workflows: passed.
- Independently peeled each real tag and read its historical manifest. Read retained Linux exit/stderr, replay script and diagnostic calculation report under `/tmp/rpr/evidence-r1/`, and upstream 0.4.3 source under `/tmp/rpz-upstream-r1/0.4.3/`.
- Cargo automatic integration-test discovery compiles `ci_workflow_contracts.rs`; ordinary names place the tests in Level 1. The driver group-coverage guard and new historical-success regression passed.
- Initial compound-filter attempts failed on Just shell quoting before testing; simple positional filters produced the actual runs above. Cargo's unrelated cache-cleanup permission warning did not prevent compilation or execution.

| Requirement | Verification present | Assessment |
|---|---|---|
| Event eligibility, proposal identity, freshness, exact merge/head | Level-1 public functions/commands, event fixtures and isolated real Git | Appropriate; selected checks passed |
| Permissions, token wiring, serialization, retained diagnostics | Workflow contracts and actionlint | Appropriate wiring checks; live App remains explicitly unverified |
| Historical CRLF failure/repair | Level-1 real Git with production environment | Both regressions passed |
| Exact approved calculation and generated changelogs | Actual Linux tool replay and reporting fixture | Unmet with real tags: R1-F1 |
| Remote error handling and proposal command | Level-1 public API with local HTTP and recording release tool | Correct level; unstable startup: R2-F1 |
| Publication/recovery and integration routing | Level-1 isolated Git, recording boundaries, pinned-source inspection, workflow contract | Appropriate before release; live publication/event delivery remains separately authorized |

No terminal input or GUI behavior requires Level 2 or Level 3 here. Missing duplicate cross-OS proof is not a finding. The prerequisite's green run on main, hosted dry run, App installation and later approved proposal/merge/publication remain pending as the spec permits; none is silently marked complete.

### Input robustness

Default invalid-input behavior remains an error before publication. Shared JSON loading rejects duplicate keys, trailing content and nonstandard constants; TOML loading rejects invalid syntax and duplicate keys. Required validators distinguish absent from null, reject wrong scalar/list types and malformed label members, and preserve an explicitly empty label list as valid no-label selection. Optional carried settings default when absent. Exact release-set and tag-format checks remain enforced.

Reviewed existing scalar/list matrices, selector controls and malformed-field cases, TOML/configuration cases, manifest-version projection, release-output checks and HTTP response-shape cases. Normal validator/configuration groups passed. HTTP groups need R2-F1 to execute reliably; numeric-name diagnostic controls pass. The requested coercion-smell search found no `serde(default)`, `filter_map`, `unwrap_or_default`, or `.ok()` parse in the changed scripts/driver. The environment repair introduces no additional document loader requiring another matrix.
