---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-17T20:41:28-07:00
spec: 2026-10-17-logging-improvements/spec.md
implemented: false
description: "A **fix** review of `2026-10-17-logging-improvements/spec.md`"
fix: 2026-10-17-logging-improvements/review-3.md
previous: 2026-10-17-logging-improvements/review-2.md
---

# Logging improvements: review 3

The fix is **production ready** under the specification's acceptance criteria.
The remaining finding, R2-F1, is verified fixed across all eleven repair-ledger
children. No new findings were identified. No human decision is required.

## Previous Finding Verification

I read both earlier reviews, the specification, the original implementation
log, and both repair Finding Ledgers in `log.md`. Verification below uses
source inspection and this review's test execution, rather than completion
flags or the implementer's reported runs.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Diagnostics tests use a response as a background-work barrier | R2-F1.1–R2-F1.11: both external-payload file-list tests, other file-list readers, referenced-target readers, existing context/parity/session readers, held-worker assertions, shared flush, versioned envelope publication, save publication drain, and remaining publication waits | **verified fixed** | Every child is checked below. The shared helper waits for outstanding registry jobs and makes another receiving round trip after their counts change. The server reports completion after publishing held refreshes. The deterministic held-worker test and all affected sessions passed in this review's area run. | None. The additional stale-publication sites were repaired in this cycle; they are not remaining defects or new findings. |

| Ledger child | Verification evidence |
| --- | --- |
| R2-F1.1 | In the dmls package, [`workspace_file_list::latest`](../../dmls/tests/l1/workspace_file_list.rs:75) calls `flush_server_with` before taking the latest buffered diagnostics and discarding older ones. `without_folder_watches_a_save_rechecks_external_schema_inputs` passed, including initial open and the external payload's required-to-optional edit followed by save. |
| R2-F1.2 | The same helper protects `schema_inputs_outside_the_workspace_are_watched_by_folder`; that test passed with folder watches enabled and its watch-registration assertion intact. |
| R2-F1.3 | All other file-list helper consumers passed, including startup, list-readiness, unchanged-target reuse, buffer override/close, and save-rescan parse counts. Its round trip remains a document-symbol request on the open document, so synchronization does not build a full index. |
| R2-F1.4 | [`referenced_targets::latest`](../../dmls/tests/l1/referenced_targets.rs:151) now uses the same background-aware document-symbol flush. All 33 referenced-target sessions passed, including excluded/outside-folder save refresh and parse reuse. This strengthens a site review 2 called clean: save rescans can queue preparation even for a source without frontmatter. The repair addressed that sibling; it is not a partial repair. |
| R2-F1.5 | Repository-context and entry-point parity readers still call `flush_server` before taking buffered diagnostics. It delegates to the same loop with a workspace-symbol round trip. Their session tests passed. |
| R2-F1.6 | Session readers and strict-recovery drains retain their explicit flush/drain or publication wait. Their tests passed. Direct publication waits receive a notification rather than treating an unrelated response as completion. The additional stale-publication repair is checked separately below. |
| R2-F1.7 | The remaining `round_trip` calls in `registry_preparation.rs` intentionally run while a preparation is held, to assert absence or responsiveness. They do not claim to complete it. These tests passed. |
| R2-F1.8 | [`LspFixture::flush_server_with`](../../dmls/tests/common/mod.rs:609) retains the outstanding-job loop, waits for job completion, and makes another round trip whenever counts moved. In the dmls server, [`finish_registry_job`](../../dmls/src/router.rs:1708) publishes held diagnostics and transitions and proceeds with waiters before reporting the job finished. [`a_flush_through_any_request_buffers_a_held_refresh`](../../dmls/tests/l1/registry_preparation.rs:239) passed: an unrelated document-symbol response arrives while the refresh is held, then the flush buffers the required-property diagnostic after release. No sleep or retry was added. |
| R2-F1.9 | [`schema_roots_parity`](../../dmls/tests/l1/schema_roots_parity.rs:450) now waits for a versioned publication from the opened envelope instead of accepting startup's closed-file publication. The forbidden-path-prefix parity test passed. |
| R2-F1.10 | [`trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope`](../../dmls/tests/l1/lsp_session.rs:625) flushes and drains envelope publications before deleting the payload and saving. Its subsequent wait therefore observes post-save diagnostics. The test passed, asserting both the envelope failure and retained effective schema. |
| R2-F1.11 | Rechecked buffered readers and direct waits across the DMLS tests. Remaining readers use the barriers above, wait for the first publication of a newly opened URI, or intentionally inspect equivalent publications. In the suggestion tests, repeated opens use equivalent content; strict-recovery transitions retain their existing drains. The area run exercised these sessions successfully. |

Review 2 had already closed all five findings from review 1. No earlier finding
was left blocked or open. I also rechecked their principal contracts and ran
their regression suites:

| Earlier finding | Outcome | Current verification |
| --- | --- | --- |
| R1-F1: Startup omits package and package-area schema roots | **verified fixed** | Workspace-scope and root-parity sessions passed. Startup preparation still derives scopes through document contexts and includes repositories discovered by the walk. |
| R1-F2: Unlisted Markdown targets never receive on-demand analysis | **verified fixed** | All referenced-target sessions passed. Dependency targets remain separate from listed documents and full-index projections. |
| R1-F3: A ready full index stays ready after losing a readable document | **verified fixed** | Full-index failure/recovery sessions passed, including the five loss transitions against references, heading rename, file rename, and workspace symbols. Completeness still requires current parses for listed files. |
| R1-F4: Trigger preparation still blocks message dispatch | **verified fixed** | Registry-preparation sessions passed, including worker placement, held-worker responsiveness, cancellation, shutdown, and lookup work counters. The server still performs preparation outside the overlay lock and dispatch thread. The accepted rescan and lazy-parsing boundaries are unchanged. |
| R1-F5: Trigger payload failures are missing from failed-trigger counts | **verified fixed** | Startup log and registry tests passed. The summary still combines passive classifications with context-bound trigger-load failures, counting a failed trigger once across contexts. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding is open or newly reported, so `recurrence` is false. The repair
expanded R2-F1's sweep to cover the referenced-target helper and two stale
publication readers. Those repairs are verified above; they do not require
another fix cycle. Earlier accepted scope decisions are unchanged.

## Verification and coverage

- `just test` in `darkmatter/`: **10,349 passed, 10 skipped, 0 failed**, across
  12 binaries, in **35.425 seconds** (nextest run
  `9e6f9f01-dae9-4810-9536-2740d6887d84`).
- `just check-tier-coverage darkmatter`: **zero stranded tests**.
- The DMLS manifest declares the consolidated L1 target, its module root
  includes the repaired suites, and the local/CI feature lists enable
  `effects-instrumentation`. The new ordering test ran as L1.

The test run is corroborating evidence, not the sole proof that the race is
fixed: the outstanding-job reporting order and receiving round trip establish
the synchronization contract. Repeating tests until they pass is unnecessary.

| User-facing requirement | Appropriate verification level | Evidence checked in this review |
| --- | --- | --- |
| Ignore unrelated malformed YAML; report claimed invalid YAML with a located syntax error | L1 public parser/discovery, CLI and protocol | Classifier, incident fixture, shipped-schema corpus, CLI trigger cases, and position-encoding tests passed. |
| Prepare applicable schema roots, preserve context/shadowing, and count successful/failed triggers accurately | L1 filesystem fixtures, protocol and log capture | Workspace-scope, root-parity, registry-summary, and startup-log suites passed. |
| Cached lookups avoid rediscovery; events and save fallback refresh external dependencies; dispatch remains responsive | L1 work counters, protocol and controlled worker gates | Registry cache, preparation, external payload, held-refresh, cancellation, and shutdown tests passed. The repaired diagnostic readers now wait for the relevant background work. |
| Discover Markdown/YAML with ignore rules, skip `.git`, and avoid parsing unopened Markdown at startup | L1 filesystem fixtures and read/parse counters | Discovery and workspace-file-list suites passed, including non-repository ignore rules and startup parse counts. |
| Parse referenced Markdown on demand, reuse unchanged parses, refresh dependencies, and preserve buffer precedence | L1 public protocol and counters | Workspace-file-list and referenced-target suites passed for listed, excluded, outside-folder, and non-Markdown controls. |
| Return complete references, rename edits, file-rename edits, and workspace symbols; handle failures and concurrent changes | L1 public protocol and controlled build gates | Full-index prerequisite, reconciliation, cancellation, failure, and recovery tests passed. |
| Emit startup/rescan/build counts and slow-request timing with correct outcomes and routing | L1 captured logs, injected clock and headless subprocess | Startup log, registry-transition, timing, router logging, and stdio subprocess suites passed, including the 500 ms boundary and error/cancellation paths. |
| Explain configuration, startup, caching, and logging | Documentation/source comparison | The configuration and indexing pages describe the current defaults, scope rules, prerequisites, watch fallback, log fields, and timing-stage nesting. |

These requirements concern protocol results, disk discovery, and log content.
They do not promise terminal rendering or keyboard behavior, so Level 2 or
Level 3 is not needed for this change. Cross-OS evidence remains CI's concern
and does not affect this review's readiness decision.

### Input robustness

This repair changes test synchronization, not a configuration loader. For the
original fix's changed reader, I rechecked the shared trigger-envelope
classifier and its public-API tests. A parseable `kind` that is absent, null
(explicit or empty YAML value), wrong-type, or empty does not claim the
envelope; the exact trigger claim is the positive control. Claimed invalid
YAML produces the dedicated located error; unclaimed invalid YAML is ignored.
Duplicate keys, trailing invalid content, a second YAML document, quoted/JSON
spellings, BOM/CRLF, and multiline/nested nonclaims have passing tests.

The classifier has no `serde(default)`, `unwrap_or_default()`, `.ok()`, or
element-filtering fallback. No new loader or field-validator gap was found.
No stronger null/absence policy is imposed on unchanged payload parsing.

## Observations

None newly identified. Observations from earlier reviews are not carried as
findings or repair requirements.

Only this review and the requested review/spec frontmatter were edited.
Production sources and tests were left unchanged. No formatter, commit,
terminal emulator, or editor was run.
