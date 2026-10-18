---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Diagnostics tests use a response as a background-work barrier"
observations:
    - Rescans still perform synchronous disk work
    - Existing tests exceed the Level 1 time budget
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-17T20:25:19-07:00
spec: 2026-10-17-logging-improvements/spec.md
implemented: true
description: "A **fix** review of `2026-10-17-logging-improvements/spec.md`"
fix: 2026-10-17-logging-improvements/review-2.md
previous: 2026-10-17-logging-improvements/review-1.md
next: 2026-10-17-logging-improvements/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/log.md
implemented_by: claude/opus
---

# Logging improvements: review 2

The fix is **not production ready** because one acceptance-test synchronization
race remains. All five previous findings are verified fixed. The new finding
concerns tests made unreliable by background registry preparation; it is not
an unresolved instance of the previous dispatch-thread defect.

## Previous Finding Verification

I read review 1, the original implementation log, the repair log's Finding
Ledger, and the approved plan. I checked the ledger's child inventory against
source and the tests run in this review. The ledger's completion labels were
not used as verification evidence. There is no earlier review in this directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Startup omits package and package-area schema roots | R1-F1.1–R1-F1.18: package/area roots, startup counts, tree/home/environment controls, workspace subfolders/multiple folders/nested areas, manifest/rescan/config/save/watch refresh, watch registration, per-document lookup, fallback areas, nested repositories, and new repositories | **verified fixed** | The DMLS [scope preparation](../../dmls/src/overlay/mod.rs:540) obtains catalog and fallback probes through each document's context. Discovery records nested repositories; startup and refresh prepare their scopes. All `workspace_schema_scopes` session tests, startup-log tests, root-parity tests, repository-context tests, and discovery tests passed in the DMLS run. Detailed checks below include every ledger child. | None from this finding. |
| R1-F2: Unlisted Markdown targets never receive on-demand analysis | R1-F2.1–R1-F2.20: eligibility, diagnostics, completion, hover, definition, cycles, watch/save refresh, listed/wiki controls, full-index separation, direct target consumers, watch registration, graph projections, references/rename, diagnostics/listing, and buffer/list transitions | **verified fixed** | The DMLS [invalidation engine](../../dmls/src/graph/invalidate.rs:339) admits resolved Markdown independently of listing. The [graph builder](../../dmls/src/graph/arena.rs:301) retains dependency headings/transclusions while excluding unrelated references. All 33 `referenced_targets` sessions and the graph/full-index controls passed. | None from this finding. |
| R1-F3: A ready full index stays ready after losing a readable document | R1-F3.1–R1-F3.21: five loss transitions × four consumers, recovery, initial-build/deletion controls, failed-path retries, listing replacement, open buffers, file rename, configuration, completeness callers, and stale-heading prevention | **verified fixed** | The DMLS [completeness check](../../dmls/src/graph/invalidate.rs:630) requires a current parse for every listed file. The [protocol matrix](../../dmls/tests/l1/full_index.rs:540) exercises all 20 transition/consumer combinations; each requires an error naming the unreadable file and no result. Recovery, deletion, build reconciliation, and graph controls also passed. | None from this finding. |
| R1-F4: Trigger preparation still blocks message dispatch | R1-F4.1–R1-F4.9 and R1-F4.16; inspect R1-F4.10–R1-F4.15 for their stated boundary/contract | **verified fixed** | The DMLS [worker](../../dmls/src/router.rs:472) performs preparation; the cache lock covers copying/installing rather than preparation I/O. All 24 `registry_preparation` tests passed, including every schema-dependent method, held-worker edits/cancellation/shutdown, and thread placement. Registry unit tests verify lookup-only behavior and stale-result rejection. | No required instance remains. R1-F4.14 is the log's explicitly nonrequired rescan follow-up; see Observations. R2-F1 is a new test synchronization defect. |
| R1-F5: Trigger payload failures are missing from failed-trigger counts | R1-F5.1–R1-F5.10: valid/envelope controls, missing/malformed/import/example payload failures, startup projection, distinct-file counts across contexts, shadowing, and other projections | **verified fixed** | The DMLS [summary](../../dmls/src/overlay/registry.rs:682) combines passive classes with context-bound failed-load outcomes. Each [payload logging case](../../dmls/tests/l1/startup_log_lines.rs:177) passed with the expected message and counts. Cross-context and shadowing unit controls passed. | None from this finding. |

The child checks were as follows. A range includes every child in that range;
these are grouped only where the same test matrix or code invariant supplies
the evidence.

| Ledger children | Verification performed |
| --- | --- |
| R1-F1.1–R1-F1.3 | Unopened package failure, unopened area named-schema count, and no-open-document startup summary each have a passing session assertion; startup parse count stays zero. |
| R1-F1.4–R1-F1.5 | Existing tree/home startup totals and direct-library discovery parity, including environment roots and shadowing, passed. |
| R1-F1.6–R1-F1.8 | Subfolder roots above the editor folder, multiple workspace folders, and nested package areas each have passing scope-session tests. |
| R1-F1.9–R1-F1.13 | Manifest change, rescan, configuration reload, watched trigger repair, and saved trigger repair each have passing sessions. |
| R1-F1.14 | Schema watch folders come from prepared scope dependencies; widening preparation therefore includes unopened package/area roots without another root calculation. |
| R1-F1.15 | Per-document roots remain context-derived; file-list/full-index lines count their own listed documents rather than prepared scopes. Root-parity and startup-log controls passed. |
| R1-F1.16 | Fallback-area startup and watched schema creation passed; the control proves directories without schema roots add no scans. |
| R1-F1.17 | Nested-repository startup and rescan discovery passed; the walk's repository-recording unit test covers both directory and file `.git` entries. |
| R1-F1.18 | Watched `.git/config` creation prepares a newly created nested repository; its session passed. |
| R1-F2.1–R1-F2.6 | Both excluded and outside-folder fixtures passed missing-anchor, completion, hover, heading-location, and cycle assertions. Graph unit coverage confirms unlisted targets become dependencies. |
| R1-F2.7 | Both target forms passed watched, server-rescan, and folderless-save refresh with modification, deletion, and recreation. |
| R1-F2.8–R1-F2.9 | Listed target controls and wiki-heading controls passed; unlisted dependencies do not broaden wiki-name discovery. |
| R1-F2.10–R1-F2.13 | Full-build inputs still come from the list. Hover/definition and incomplete-link completion reach the new file-target parsing paths; corresponding sessions passed. |
| R1-F2.14–R1-F2.16 | Unlisted watch changes, dependency-folder registration, and folderless save checks are exercised by the refresh/watch sessions above. |
| R1-F2.17–R1-F2.18 | Inspected path/frontmatter/wiki completions, symbols, key index, portability, transclusion references, log counts, references, and rename. Dependency records omit ordinary link/file-use nodes and are excluded from list-based projections; symbol-session and graph controls passed. |
| R1-F2.19–R1-F2.20 | Document links and diagnostics use the same resolved edges. The buffer/list transition unit control and protocol diagnostics passed, including a dependency joining the list. |
| R1-F3.1–R1-F3.9 | All 20 named tests cover watched modification, watched creation, save rescan, buffer close, and relist against references, heading rename, file rename, and workspace symbols. All passed. |
| R1-F3.10–R1-F3.12 | Watched repair restores references/rename; initial unreadable builds fail and retry; deletion/edit during a build discards obsolete work. All passed. |
| R1-F3.13–R1-F3.15 | Watched changes, rescan, and list installation retry unparsed listed files after readiness. Inspected all three implementations and passing recovery/unit controls. |
| R1-F3.16–R1-F3.19 | Deletion removes the failed input; open buffers supply current parses; file rename uses the common prerequisite; configuration relisting uses the checked installation path. Deletion/open-buffer/reconciliation controls passed. |
| R1-F3.20–R1-F3.21 | All four complete-index methods use the common readiness check. Failed lazy reads remove prior parses; the existing stale-heading unit control passed. |
| R1-F4.1–R1-F4.5 | First-document, watched YAML, saved YAML, configuration, and save-rescan preparations each ran on a thread distinct from dispatch in passing tests. |
| R1-F4.6 | The declared method table tests symbols, definition, document links, references, highlights, folding, hover, both semantic-token methods, completion, code actions, formatting, and trigger authoring. Every row passed. |
| R1-F4.7–R1-F4.9 | Startup/post-refresh worker placement, atomic installation, later-result protection, concurrent invalidation, and unchanged-lookup work counters passed. |
| R1-F4.10–R1-F4.13 | Serverless callers intentionally prepare inline. Named-schema freshness remains outside this cache promise under the spec's “Reader's note — cache scope.” Changed-path hash reads are event-bounded. Watch folders, summaries, transitions, and error retrieval are lookups. No synchronous registry preparation remains in these server consumers. |
| R1-F4.14 | Confirmed synchronous rescan/list/manifest reads remain. The repair log explicitly excludes this row from required children; the startup/registry off-dispatch requirement does not require background rescans. Recorded as an observation, not silently marked repaired. |
| R1-F4.15–R1-F4.16 | Lazy Markdown parsing remains on dispatch by the scoped design. Held registry preparation permits edits, other requests, cancellation, and shutdown; both responsiveness sessions passed. |
| R1-F5.1–R1-F5.7 | Valid payload: 1 trigger/0 failures; malformed envelope and missing, malformed, import-failing, example-failing payloads: 0 triggers/1 failure. Startup failure-line selection also passed. |
| R1-F5.8–R1-F5.9 | Cross-context failure counts the trigger once as failed. A shadowed trigger is not resolved merely to classify it; the failure is counted when a preparation reaches it. Both unit controls passed. |
| R1-F5.10 | Startup is the class-count consumer. Rescan/file-list logs and authoring diagnostics do not independently project these classes into counts. |

## Unblocked Findings

### R2-F1 (high): Diagnostics tests use a response as a background-work barrier

**Authority — regression:** tests introduced by this fix to prove schema-input
watching and save fallback were reliable with synchronous document preparation;
the repair's asynchronous preparation makes their response-only barrier racy.
The spec's R3/R5 test requirements include external payload changes and save
fallback. Running the normal `just test` command reached this failure without
changing any input or manufacturing a timing fault.

The defect class is **assuming that a request response also completes earlier
background diagnostic work**. In the dmls package,
[the file-list tests' diagnostics helper](../../dmls/tests/l1/workspace_file_list.rs:75)
requests document symbols and immediately requires buffered diagnostics.
However, [the worker](../../dmls/src/router.rs:491) installs the registry
before dispatch handles its completion message, and
[the request prerequisite](../../dmls/src/router.rs:1784) checks registry
freshness rather than whether the held diagnostic refresh has been published.
A symbol request can therefore answer while that refresh is still pending.
The helper then panics, or could inspect an older publication.

**Reproduction:** run `just test` in `darkmatter/`. This review's run failed
`workspace_file_list::without_folder_watches_a_save_rechecks_external_schema_inputs`
at `workspace_file_list.rs:80` with `diagnostics were published`. Its unchanged
fixture opens a frontmatter document governed by a trigger with an external
payload, changes the payload's required property to optional, sends save, and
checks the resulting diagnostics. The output does not identify whether the
initial-open or save assertion reached the helper failure. Both use the same
unsafe barrier. A subsequent isolated test passed, as did 12 diagnostic runs
of the file-list module and a later full DMLS run; those passes establish
intermittency, not a repair.

The bounded sweep covered every buffered-diagnostics reader in DMLS tests and
the code deciding when registry-dependent requests proceed. Existing fixtures
were exercised in the full runs and the focused module runs. Clean sites below
have an explicit synchronization contract or inputs that cannot queue registry
work; they are not declared clean merely because two implementations agree.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | File-list helper: external payload, no folder watches | Existing frontmatter fixture: open, then required-to-optional payload edit and save | Normal area run panicked because no diagnostics were buffered; both calls share the response-only race | Observe the completed refresh before reading diagnostics |
| R2-F1.2 | Same helper: external payload with folder watches | Same trigger/payload fixture and frontmatter open, relative folder-watch support enabled | Tests passed, but the helper uses the identical response-only barrier; the worker/dispatch ordering also applies here | Synchronize the refresh before checking diagnostics and resulting watch registration |
| R2-F1.3 | Other file-list helper consumers | Plain Markdown links, transclusions, listing/ignore changes, buffer changes | Clean for this class: their fixtures have no frontmatter/claimed envelope, so no document registry job delays their diagnostics | Preserve synchronous path/parse-counter assertions |
| R2-F1.4 | Referenced-target diagnostics helper | Listed, excluded, outside-folder targets; watch/save modification, deletion, recreation | Clean for this class: source fixtures contain only links/transclusions and queue no document registry work; all 33 sessions passed | Preserve lazy parse counts; do not force a full index merely to synchronize |
| R2-F1.5 | Repository-context and entry-point parity readers | Their existing frontmatter/context fixtures | Clean: both use the common background-aware flush before reading buffered diagnostics; sessions passed | Preserve the background-aware barrier |
| R2-F1.6 | LSP session diagnostic readers and strict-recovery buffer drains | Existing trigger failure/recovery and sidecar fixtures | Clean: flush before buffered reads/drains, or wait directly for the diagnostic publication; sessions passed | Preserve explicit publication/idle synchronization |
| R2-F1.7 | Held-registry negative assertion | Intentionally paused preparation, then unrelated round trip | Clean: the assertion requires no diagnostics before release, rather than assuming a response completes preparation; held-job tests passed | Continue proving the absence while the worker is held |
| R2-F1.8 | Common fixture publication wait and registry-aware flush | All sessions using these helpers | Clean contract: publication waits receive the event; flush waits for outstanding jobs and performs another response round trip after job counts move | Reuse a suitable existing synchronization primitive |

Repair the affected file-list test barrier and its comment. The repair need
not change the server's response ordering: LSP does not require a symbol
response to wait for unrelated diagnostic publication. Prefer waiting for the
specific refresh, or the fixture's existing registry-job barrier plus a
receiving round trip. Avoid sleeps/retries, and avoid adding a full-index build
to parse-counter tests just to make the assertion settle. Add a controlled
ordering check if a small existing seam can express it; no new scheduler or
large test matrix is warranted.

## Blocked Findings

None. No human decision is needed for the remaining test repair.

## Recurrence

No finding repeats an earlier finding's defect class. R1-F4's required
background execution and responsiveness are now verified; R2-F1 is a newly
introduced test synchronization regression from that repair. It is not an
unchanged inline-preparation instance, a partial repair of R1-F4, or another
unlisted-target sibling of R1-F2.

## Verification and coverage

- `just test` in `darkmatter/`: **10,301 passed, 1 failed, 10 skipped;
  46 not run after fail-fast**, 56.367 seconds. The failure is R2-F1.
- Focused nextest run of that failing test: passed. Twelve diagnostic runs of
  the unchanged `workspace_file_list` module also passed. No retry policy or
  test source was changed.
- Full DMLS nextest run with `effects-instrumentation`: **1,096 passed,
  6 skipped**, 12.352 seconds. This exercised the tests omitted after the area
  run stopped, including all repair suites. Its success does not erase the
  earlier race.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. The new
  modules are declared in the DMLS L1 target; its manifest enables the required
  instrumentation feature in local/CI test configurations.

The root `just test dmls` recipe failed before execution because its generated
Cargo invocation supplied an empty `--package` argument. I used direct nextest
for the scoped DMLS follow-up; no runner configuration was changed. I did not
run a formatter, commit, terminal emulator, or editor. Production sources and
tests were left unchanged by this review.

| User-facing requirement | Required verification level | Evidence and assessment |
| --- | --- | --- |
| Unrelated malformed YAML is ignored; claimed invalid YAML has a located syntax error | L1 public parser, discovery, CLI/protocol | Existing classifier, corpus, incident, CLI, and position-encoding tests passed in the area run. |
| All applicable roots are prepared, counts reflect loaded trigger outcomes, scopes retain discovery parity | L1 isolated filesystem/protocol/log capture | Scope, root-parity, registry, and startup-line suites passed. R1-F1/R1-F5 verified fixed. |
| Ready registry lookups avoid rediscovery; changes refresh consumers; dispatch stays responsive | L1 counters and deterministic background gates | Cache and held-worker tests passed. External-input diagnostic acceptance tests retain R2-F1's synchronization race. |
| File discovery honors filters, skips `.git`, lists YAML, and avoids unopened Markdown parsing | L1 filesystem fixtures and work counters | Discovery/file-list controls passed; no missing terminal-rendering evidence is implied. |
| Referenced targets support anchors, navigation, hover, completion, cycles, invalidation | L1 public protocol | All referenced-target cases passed, including excluded/outside-folder targets and non-Markdown code controls. |
| Full references, rename, file rename, symbols, cancellation, reconciliation, failure/recovery | L1 public protocol with deterministic worker gates | Full-index suite passed, including all post-readiness failure consumers. |
| Startup/rescan/build log fields, 500 ms threshold, outcomes, stderr/file routing | L1 capture, injected clock, headless subprocess | Timing/log/startup/stdio suites passed. Registry wait timing is recorded alongside other prerequisites. |

No requirement changed here needs real-terminal rendering or keyboard input;
Level 2/3 would not strengthen these protocol, disk, and log assertions.
Cross-OS execution evidence is left to CI and does not decide readiness.

### Input robustness

The repair does not add a configuration loader or change its field validators.
The original fix's relevant reader remains the shared trigger-envelope
classifier. Its tests and source were rechecked: absent, explicit null,
wrong-type, and empty `kind` values are unclaimed; an exact trigger claim is
the control. Claimed invalid YAML yields the dedicated located error, while
unclaimed invalid YAML stays harmless. Existing cases cover duplicate keys,
trailing invalid content, additional YAML documents, quoted/JSON spellings,
BOM/CRLF, and multiline/nested nonclaims. These tests passed in the area run.
The requested smell search found no `serde(default)`, `unwrap_or_default()`,
`.ok()`, or element-filtering fallback in the classifier. No new input-matrix
finding was identified; no stronger absent/null contract is inferred for
unchanged payload configuration.

## Observations

### Rescans still perform synchronous disk work

The DMLS [rescan path](../../dmls/src/router.rs:893) still lists files, rechecks
parsed documents, and scans manifests on dispatch. This is the repair ledger's
R1-F4.14 follow-up. It can delay a save on a large workspace, but this spec's
background requirement concerns startup and trigger preparation, not all
rescans. Schedule separately if needed; it does not keep R1-F4 open.

### Existing tests exceed the Level 1 time budget

Seven existing tests exceeded five seconds in the area run: five Claudine
schema-discovery tests took 10.303–11.439 seconds and two nested-span tests took
5.648 and 7.399 seconds. The scoped DMLS run still reported five slow tests.
These tests were not added or edited by this repair, and I did not establish a
before/after performance regression. Inspect their existing schema assembly
and fixture costs separately; this observation does not request another
performance layer in the logging fix.
