---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): graceful cleanup forgets descendants discovered before SIGTERM"
  - "R1-F2 (high): generated-log terminal fixtures still fail their path assertions"
  - "R1-F3 (high): concurrent fixture builds exceed the Level 1 time budget"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T11:49:35-08:00
spec: 2026-11-14-shared-process-tree/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-shared-process-tree/spec.md`"
fix: 2026-11-14-shared-process-tree/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-shared-process-tree/log.md
implemented_by: claude/opus
next: 2026-11-14-shared-process-tree/review-2.md
---

# Review 1: shared process-tree ownership

The fix is **not ready for production**. The shared owner and caller migrations
are present, but graceful shutdown can report successful cleanup while leaving
a helper running. Two terminal tests remain broken, and concurrent fixture
builds still fail the repository's test-time budget. All three findings can be
repaired without an author decision.

## Implementation decisions (informational)

Two agents independently read the specification and implementation log in
parallel, including `decisions_for_review`. Their inventories agree. The
following choices do not need a human ruling or later reaffirmation:

- Keep the hard cleanup after the root exits during graceful shutdown. This
  follows the specification's explicit requirement; the plan's shortcut is
  subordinate to that requirement. Keep this choice while repairing R1-F1.
- Use independent Windows subprocess fixtures because the older fixtures did
  not report descendant PIDs. Keep the fixtures.
- Omit the unused attachment timestamp, keeping the next-sample deadline as
  the sampling schedule. Keep this internal simplification.
- Put platform test modules at the ends of their source files to satisfy
  clippy. Keep the layout.
- Add positive controls, failure-backstop, group-error, armed-drop, and Windows
  graceful-cleanup tests beyond the plan's named cases. Keep them.
- Use `waitid` without reaping to establish the Unix fast-exit case. Keep this
  deterministic method rather than relying on a readiness sleep.
- Remove Claudine's private fast-exit attachment test after moving that
  contract into Sniff's shared-owner tests. Keep the shared coverage.
- Serialize five macOS CoreAudio callers, then add the existing small-bundle
  setup script after measurement identified directory enumeration as the
  dominant cost. Keep both: they are reversible test scheduling choices,
  and the final recorded audio tests run below one second each.
- Keep the serialization group after adding the setup script. Its modest cost
  does not justify an author interruption; removal remains a routine choice.
- Fix Windows lint issues in touched Darkmatter files, including the boxed
  test-helper error and platform-specific import cleanup. Keep these repairs.
- Lengthen the long-refusal fixture's directory name so the fixture's promised
  message length also holds with Linux's shorter temporary-directory prefix.
  Keep this repair.
- Correct source-comment drift encountered during the required comment sweep.
  Keep these corrections.
- Implement the closely coupled phases directly rather than delegate them,
  and record the actual implementing model instead of the plan's model name.
  These are execution records, not product decisions.
- Do not edit or publish caller skills after checking their canonical sources
  and finding no affected process-ownership material. This is consistent with
  the specification's conditional publication requirement.
- Leave the two generated-log terminal tests red pending a supposed product
  decision. Do **not** keep this deferral: R1-F2 gives a verified fixture-only
  repair that preserves the tests' intended behavior and current path rules.
- Leave the slow fixture-build tests unresolved because they predate the fix.
  Do **not** keep this deferral: R1-F3 covers the unresolved performance class.
  The earlier temporary deferrals of CoreAudio slowness and Windows lint were
  subsequently repaired.
- Record missing native-Windows runtime evidence after the storage preflight
  refused execution, and missing Darkmatter Linux evidence after the rig ran
  out of linker memory. Keep these explicit limitations; cross-OS evidence is
  left to CI and is not a readiness finding. WSL2 evidence is Linux-path
  evidence, not native-Windows evidence.
- Leave the specification in its active directory and create no commit.
  This review does not require either action, and moving it to `_completed`
  belongs to the author.

The specification already settles the successful-edit warning on Windows
release failure, the Unix foreground lifecycle-command exception, and the
exclusion of provider-session supervision and intentionally detached launches.
These are not new decisions. Its final section permits integration choices
within those rulings; no proposed repair changes them.

## Findings

### R1-F1 (high): graceful cleanup forgets descendants discovered before SIGTERM

**Authority: spec.** The shared API must retain observed descendants with their
start times, and graceful termination must hard-stop remaining owned processes
“even if the root exited first.” A real helper that starts after the most recent
sample, creates its own session, and ignores `SIGTERM` survives the current
public API's graceful cleanup even though the initial cleanup scan sees it.
This is distinct from the accepted escape of a descendant that was never seen
before reparenting.

**Defect class:** a process identity discovered during the first stage of
shutdown is discarded before the later stage uses it to decide what to kill.

In Sniff, [the initial graceful scan](../../lib/src/process/tree/unix.rs:298)
finds the helper through the previously sampled root. However,
[the shared scan](../../lib/src/process/tree/unix.rs:324) returns only PIDs and
does not retain the newly discovered `(PID, start time)` entries. `SIGTERM`
then kills the root; the helper ignores the signal and is reparented. The
[final hard cleanup](../../lib/src/process/tree/unix.rs:282) can no longer find
it through the root, and it is absent from the recorded-descendant map.
Cleanup returns `Ok(())` and disarms the owner while that helper remains alive.

I reproduced this on macOS using the **public** Sniff API and controlled Rust
children. The complete reproduction is preserved below. It samples the root,
then allows it to start a helper, waits for the helper's readiness marker, and
calls graceful termination. The helper calls `setsid` before reporting readiness
and ignores `SIGTERM`. Every helper was explicitly killed after observation.

| Child ID | Site / shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Unix graceful cleanup; session helper first seen by the graceful scan | `cleanup=Ok(())`, root exited from SIGTERM, helper alive | Helper hard-stopped before successful cleanup |
| R1-F1.2 | Unix immediate cleanup; same newly started session helper | `cleanup=Ok(())`, root and helper dead | Same; clean control |
| R1-F1.3 | Unix sampling followed by graceful cleanup; helper recorded by a periodic sample | Helper dead | Same; sampling correctly retains its identity |
| R1-F1.4 | Unix graceful cleanup; same new helper stays in the owned group | Helper dead | Same; group signaling covers it |
| R1-F1.5 | Windows graceful/immediate cleanup | No dependency on saved descendant identities: the Job remains the ownership boundary through termination | Clean by the Job invariant; source inspection, no Windows runtime claim |

The sweep followed the lost identity into every migrated caller. Darkmatter's
[editor cancellation and wait-failure path](../../../darkmatter/lib/src/editor/launch.rs:241)
uses this graceful operation with a two-second grace. Claudine's
[task timeout, cancellation, and runaway-output path](../../../claudine/lib/src/composition/sequence/task/shell.rs:592)
and [isolated lifecycle cancellation / wait-error path](../../../claudine/lib/src/composition/sequence/task/shell.rs:907)
use it with a 250 ms grace. All inherit R1-F1.1; none retains a second copy of
the descendant identities that could repair the loss. These caller effects
are established by the direct delegation, not claimed as separate end-to-end
reproductions. The caller sweep records those delegations separately:

| Child ID | Decision site | Shape | Observed / expected |
| --- | --- | --- | --- |
| R1-F1.6 | Darkmatter editor cancellation and wait failure | New helper from R1-F1.1 | Delegates to affected graceful operation / stop helper |
| R1-F1.7 | Claudine sequence task timeout, cancellation, runaway cap | Same | Delegates to affected graceful operation / stop helper |
| R1-F1.8 | Claudine isolated lifecycle cancellation and wait failure | Same | Delegates to affected graceful operation / stop helper |
| R1-F1.9 | Sniff probe/install cleanup | Same, immediate cleanup | Delegates to clean R1-F1.2 / stop helper |
| R1-F1.10 | Darkmatter login-shell query cleanup | Same, immediate cleanup | Delegates to clean R1-F1.2 / stop helper |
| R1-F1.11 | Armed drop after a periodic sample recorded the helper | Recorded identity survives reparenting | Delegates to immediate cleanup with recorded identities / stop helper; invariant and existing drop tests |
| R1-F1.12 | Release and Unix foreground lifecycle exception | Intentionally unowned survivors | No shared graceful cleanup decision / survivors allowed by contract |

**Smallest repair:** retain the identities of newly discovered descendants from
the graceful scan before signaling anything. Preserve their existing start-time
validation when the final scan uses them. Add a deterministic observation test
where the first shutdown scan discovers the helper and the second snapshot
shows the root gone and the helper reparented, plus a controlled public-API
regression. Keep the existing recycled-PID controls. No new supervisor or
stronger containment mechanism is needed.

### R1-F2 (high): generated-log terminal fixtures still fail their path assertions

**Authority: defect.** The existing Claudine terminal fixtures run a successful
composition and receive the expected provider body, then assert a generated
file exists at a different path from the one their relative input resolves to.
Both fail in the required affected-area terminal recipe. This leaves that gate
red and obscures whether subsequent ownership changes regress terminal behavior.

**Defect class:** a fixture supplies a relative path whose meaning changes with
the document location, while its assertion assumes a workspace-absolute meaning.

The affected code is Claudine's [generated-log fixture](../../../claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs:140)
and [shared assertion](../../../claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs:209).
`just test-l2 level2_initialize_generated_transclusion` reproduces two failures
and two passing controls. The generated body contains the log marker for phase
two; creation and transclusion both succeeded. The remaining failure is the
workspace-relative existence assertion.

I temporarily changed only the fixture's `spec` argument to the absolute path
`staged.fixture.cwd().join(SPEC_ARGUMENT)`, ran the same canonical recipe, and
restored the original source. All four tests passed, including both the
phase-one exclusion and phase-two inclusion assertions. This preserves the
fixture's purpose of creating the log beside its spec, without changing
production path resolution or simply weakening the expected file location.

| Child ID | Site / shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F2.1 | Proxied initialization, phase one, relative spec argument | File-existence assertion fails after successful composition | Full fixture passes, log beside the fixture spec, log excluded from body |
| R1-F2.2 | Proxied initialization, phase two, relative spec argument | Same assertion fails; provider body already includes log marker | Full fixture passes, log beside the fixture spec and included in body |
| R1-F2.3 | Direct initialization without shell approval | Pass; no shell effect or provider launch | Same; clean control |
| R1-F2.4 | Sequence static preflight for an initialization-generated include | Pass; include refused before effects | Same; clean control |
| R1-F2.5 | Both proxied rows with an absolute fixture spec argument | Both pass, including original content and ordering assertions | Same; verified repair control |

**Smallest repair:** pass the absolute spec path in the shared router fixture.
Keep its existing expected log path and every behavior assertion. These tests
are declared by the consolidated L2 target and selected by the live recipe;
their failures are not missing harness evidence. No author ruling is needed.

### R1-F3 (high): concurrent fixture builds exceed the Level 1 time budget

**Authority: defect.** The repository's review and testing contract treats an L1
test over five seconds on a host without substantial overload as failing,
including pre-existing tests in recorded runs. The implementation log records
five final archive tests at 5.73–7.41 seconds on a 16-core host at load 8–14,
and earlier fixture-build tests at 13–14 seconds even under the loaded-host
allowance. Deferring them because another package owns them does not clear
the gate.

**Defect class:** concurrent tests launch expensive child Cargo builds without
sufficient sharing or scheduling, making individually fast tests exceed their
budget when run together.

The main common boundary is repo-deps' [archive fixture producer](../../../scripts/ci-build-archive-tests.rs:1033).
I ran all 70 L1 archive tests through
`just _test repo-deps --bin ci-build archive::tests::`; five exceeded five
seconds. Running those five one at a time then measured 1.715–2.552 seconds.
A subsequent full `just _test repo-deps` ran 919 L1 tests and exposed the
canonical-tier fixture as another member of this class. All test assertions
passed, but the timing gate is red. Load readings during this review were
roughly 10–17 on 16 cores, not well above the core count. The earlier recorded
quiet-host failure independently establishes the first five rows.

All names below are in repo-deps. Archive rows are under `archive::tests` in
the `ci-build` binary; feature-attribution rows are under `tests` in that
binary's namesake executable. Each slow row must remain in the repair ledger.

| Child ID | Test / site | Concurrent measurement | Isolated measurement | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | `a_consumer_reports_its_extraction_apart_from_its_identity_checks` | 7.270 s (archive run) | 2.161 s | At most 5 s in the concurrent run |
| R1-F3.2 | `a_produced_fixture_verifies_including_its_own_archive_inventory` | 7.244 s | 1.715 s | At most 5 s |
| R1-F3.3 | `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` | 7.151 s; 5.266 s in full package | 2.552 s | At most 5 s |
| R1-F3.4 | `an_archive_of_more_programs_than_the_plan_resolved_is_refused` | 6.632 s | 1.913 s | At most 5 s |
| R1-F3.5 | `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts` | 7.209 s | 1.772 s | At most 5 s |
| R1-F3.6 | `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` | 4.618 s in archive run; **5.661 s** in full package | 3.055 s | At most 5 s in both scopes |
| R1-F3.7 | `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach` | 3.600 s in archive run; 4.607 s in package | Not needed: below bar | Clean control, same producer |
| R1-F3.8 | `a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart` | Earlier log: 14.1 s; below 5 s in this full package run | Earlier log: 3.5 s | Currently clean; preserve this sibling while repairing contention |
| R1-F3.9 | Feature attribution: `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations` | Earlier log: 13.4 s; now 3.243 s | Earlier log: 3.1 s | Currently clean; same requirement |
| R1-F3.10 | Feature attribution: `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared` | Earlier log: 13.6 s; now 3.839 s | Earlier log: 3.0 s | Currently clean; same requirement |

The sweep also ran every other L1 test in repo-deps, rather than checking only
the reported five. These additional archive-producer consumers were clean:

| Child ID | Test | Shape | Observed / expected |
| --- | --- | --- | --- |
| R1-F3.11 | `the_fixture_archive_carries_every_declared_payload_class` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.12 | `a_refused_consumer_still_reports_what_its_transfer_cost` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.13 | `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.14 | `a_missing_sidecar_is_refused_end_to_end` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.15 | `an_archive_offered_to_the_wrong_environment_is_refused_end_to_end` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.16 | `the_manifest_records_the_workspace_the_producer_compiled_at` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.17 | `consumer_refuses_a_different_source_tree_even_with_a_recomputed_manifest_digest` | Shared immutable fixture | Passed below 5 s / at most 5 s |
| R1-F3.18 | `failed_and_refused_keys_report_themselves_and_let_an_unrelated_key_finish` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.19 | `a_key_narrowed_leg_produces_only_its_own_record` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.20 | `every_record_attempt_is_sampled_and_a_failing_sampler_changes_no_outcome` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.21 | `the_hosted_wrapper_samples_each_record_through_the_shipped_sampler` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.22 | `the_owner_wrapper_leaves_owner_timing_to_the_coordinator_in_record_mode` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.23 | `a_sampler_that_cannot_start_leaves_a_successful_owner_successful` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.24 | `one_owner_tree_shares_a_dependency_compile_without_unifying_features` | Custom producer fixture | Passed below 5 s / at most 5 s |
| R1-F3.25 | `produce_accepts_the_planned_head_and_refuses_the_merge_revision` | Custom producer fixture | Passed below 5 s / at most 5 s |

Rejection-only producer tests do not build a successful fixture; they were
included in the full run as well. The deliberately separate
`slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`
is outside L1, rather than silently omitted from the L1 inventory.

**Smallest repair:** reuse the existing immutable fixture where a test merely
inspects an archive, and give the remaining fixture compiles a bounded nextest
concurrency group if needed. Keep fresh builds where compilation itself is the
asserted behavior, notably the external native-library fixture. Scope the group
to actual compiler consumers; serializing all 71 archive tests is unnecessary.
Verify both the archive scope and the full package so the fix removes contention
rather than merely moving which test is slow. Do not rename, ignore, retry,
raise deadlines, or drop cases.

## Requirement coverage and verification levels

The implementation's recorded full L1 runs cover Sniff, Darkmatter, and
Claudine. This review added focused execution and source inspection:

| Requirement | Appropriate level | Evidence and assessment |
| --- | --- | --- |
| Configure/attach, group/session setup, fast exit, foreign-group rejection | L1 | Public owner, Unix controlled-process tests and injected group checks; focused Sniff run passes |
| Deferred sampling, no attachment scan, identity checks before traversal and signaling | L1 | Scripted host snapshots and sample scheduling tests pass; R1-F1 identifies the missing shutdown-to-shutdown identity retention case |
| Immediate, graceful, release, armed drop, disarm after successful cleanup | L1 | Existing tests pass; public reproduction exposes R1-F1 despite them |
| Normal completion, timeout, inherited pipes, large output, detached descendants and known unobserved escape | L1 | `just test process::` in Sniff: 47 passed, none slow; intentional escape has fixture cleanup |
| Windows suspension, Job assignment before resume, setup/assignment/thread/resume/release failures | L1 on native Windows | Tests are compiled in the Windows platform module; source preserves ordering and handle ownership. Implementation has compile evidence but lacks runtime evidence due to storage refusal; CI must supply it |
| Editor arguments/streams, cancellation, ignoring SIGTERM, pre-existing GUI process survival | L1 | `just test editor_launcher` in Darkmatter: 8 passed, none slow |
| Terminal foreground transfer, editor input and restoration after interruption | L2 | `just test-l2 level2_edit_action` in Darkmatter: 2 relevant real-terminal tests passed; no focus-changing GUI launch |
| Claudine command timeout, cancellation, background cleanup, caps and error categories | L1 | `just test-library shell_tasks`: 26 passed, none slow; existing tests do not cover the helper shape in R1-F1 |
| Claudine affected terminal suite | L2 | Original generated-log fixture: 2 passed / 2 failed; temporary absolute-path control: 4 passed; R1-F2 remains until repair lands |
| Local API without HTTP dependencies | Compile / dependency inspection | Implementation records no-feature Sniff check and no reqwest graph, plus Darkmatter's remote-enabled consumer build; no new dependency added for the owner |
| Canonical documentation and duplicate removal | Source/document inspection | Shared topic and consumer links exist; duplicate library owners are removed; foreground and provider-supervision exceptions remain explicit |

No requirement here depends on the terminal's physical-key encoder, so L3
keyboard injection is not required. Higher terminal levels would not address
the process-identity loss. No new file-format or configuration reader was
introduced; the input robustness matrix does not apply to this consolidation
or to the nextest configuration edits.

## Implementation record corrections

The implementation log was corrected in place with a dated review note.
Earlier “all pass” / “green except” descriptions now distinguish nextest's
assertion result from the failed timing gate. The earlier CoreAudio cause is
identified as a superseded hypothesis rather than retained as a fact, and the
close-out's blanket statement of behavioral conformance now acknowledges the
public-API counterexample. The spec's behavioral text was left unchanged;
only its review iteration was updated. These record corrections are not
additional findings or human-review items.

## Public API reproduction for R1-F1

This temporary example was compiled and run twice, then removed from the
source tree. To repeat it on macOS, save the following as
`sniff/lib/examples/review_process_tree.rs` and run
`cargo run -p sniff --example review_process_tree`. It uses readiness files;
the 260 ms waits deliberately cross the public API's sampling interval.
Remove the example afterwards. The direct child is waited for, and every
reported helper is killed before its temporary directory is removed.

```rust
use sniff::process::{ProcessTree, TreeConfig};
use std::{path::Path, process::{Command, Stdio}, time::{Duration, Instant}};
fn marker(path: &Path) {
    let until = Instant::now() + Duration::from_secs(3);
    while !path.exists() {
        assert!(Instant::now() < until, "missing marker {}", path.display());
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("helper") {
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN); }
        std::fs::write(&args[2], std::process::id().to_string()).unwrap();
        loop { std::thread::sleep(Duration::from_secs(1)); }
    }
    if args.get(1).map(String::as_str) == Some("root") {
        let dir = Path::new(&args[2]);
        std::fs::write(dir.join("root-ready"), "").unwrap();
        marker(&dir.join("go"));
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.arg("helper").arg(dir.join("helper-ready"));
        if args[3] == "session" { TreeConfig::session().configure(&mut cmd); }
        let mut helper = cmd.spawn().unwrap();
        let _ = helper.wait();
        return;
    }
    for (label, session, sampled) in [("new detached helper", true, false), ("previously sampled helper", true, true), ("helper still in group", false, false), ("immediate new detached helper", true, false)] {
        let dir = std::env::temp_dir().join(format!("review-tree-{}-{}", std::process::id(), sampled as u8 + 2 * session as u8));
        std::fs::create_dir(&dir).unwrap();
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.arg("root").arg(&dir).arg(if session { "session" } else { "group" }).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        TreeConfig::process_group().configure(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let mut tree = ProcessTree::attach(&child).unwrap();
        marker(&dir.join("root-ready"));
        std::thread::sleep(Duration::from_millis(260));
        tree.sample();
        std::fs::write(dir.join("go"), "").unwrap();
        marker(&dir.join("helper-ready"));
        let pid: i32 = std::fs::read_to_string(dir.join("helper-ready")).unwrap().parse().unwrap();
        if sampled { std::thread::sleep(Duration::from_millis(260)); tree.sample(); }
        let result = if label.starts_with("immediate") {
            let cleanup = tree.terminate(sniff::process::CleanupScan::Always);
            sniff::process::TreeTermination { cleanup, status: child.wait() }
        } else { tree.terminate_gracefully(Duration::from_millis(80), &mut child) };
        let until = Instant::now() + Duration::from_millis(100);
        while Instant::now() < until && unsafe { libc::kill(pid, 0) } == 0 { std::thread::sleep(Duration::from_millis(2)); }
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        println!("{label}: cleanup={:?}, root={:?}, helper_alive={alive}", result.cleanup, result.status);
        unsafe { libc::kill(pid, libc::SIGKILL); }
        let _ = child.kill(); let _ = child.wait();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
```

Observed output on both runs (the immediate control was added on the second):

```text
new detached helper: cleanup=Ok(()), root=Ok(ExitStatus(unix_wait_status(15))), helper_alive=true
previously sampled helper: cleanup=Ok(()), root=Ok(ExitStatus(unix_wait_status(15))), helper_alive=false
helper still in group: cleanup=Ok(()), root=Ok(ExitStatus(unix_wait_status(15))), helper_alive=false
immediate new detached helper: cleanup=Ok(()), root=Ok(ExitStatus(unix_wait_status(9))), helper_alive=false
```
