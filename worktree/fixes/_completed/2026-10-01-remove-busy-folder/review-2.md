---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-11T17:18:40-07:00
spec: 2026-10-01-remove-busy-folder/spec.md
implemented: false
description: "A **fix** review of `2026-10-01-remove-busy-folder/spec.md`"
fix: 2026-10-01-remove-busy-folder/review-2.md
previous: 2026-10-01-remove-busy-folder/review-1.md
---

# Review of Remove Busy Folder — Iteration 2

The fix is production ready. The previous review's blocking coverage gap is
closed, its documentation observation is addressed, and this review found no
new defects within the specification's scope.

## Previous review and repair log

The previous review used a single Findings section rather than separate
Unblocked and Blocked sections. Its one high finding, **Automatic recovery
success lacks the specified public-path tests**, was unblocked and is resolved.
There were no blocked findings to reconsider. The repair log records no
deferred or disputed findings.

In the worktree library, the new tests call
[`remove_worktree`](../../lib/src/remove/mod.rs:100), the public operation that
removes a registered checkout. The test-only
[`fail_after_running`](../../lib/src/git.rs:494) helper lets the real Git command
remove and unregister the checkout before reporting an injected error. One
test leaves the folder absent; the other recreates `target/flycheck0/x`. Both
assert successful return, absent folder and registration, and a retained local
branch. These checks exercise the recovery decision through the public entry
point rather than calling the cleanup helper directly.

The new worktree CLI
[`recovery test`](../../cli/tests/remove.rs:501) puts a Git stand-in on the child
process's PATH. It runs real removal, recreates build-output remnants, and
returns failure. The test asserts exit 0, folder and registration gone, and
branch deleted, proving that successful cleanup reaches the later branch step.
This CLI fixture is Unix-only; the two library success tests are not
platform-gated. All new tests are compiled by live test targets and selected
in Level 1.

The observation **Current removal documentation contradicts the implemented
recovery** is also resolved. The current-behavior sections of
[`remove.md`](../../docs/cli/remove.md:126) now describe the all-OS process
refusal, its visibility limits, all three post-failure outcomes, and the
manual cleanup instructions when recovery fails. The separately marked
planned behavior does not enlarge this fix's contract.

## Findings

None.

## Verification

The recovery coverage sweep includes the following outcomes. All executed
cases passed; implementation inspection confirmed the shared continuation
used by ordinary removal and the handoff's second run.

| Requirement or recovery outcome | Verification present and checked | Result |
| --- | --- | --- |
| Find a child in a descendant directory; exclude a sibling prefix and the caller | Sniff Level-1 process-discovery tests | Passed |
| Refuse before the report, name the process, exit 4 despite force flags, retain files and branch | CLI Level-1 subprocess test | Passed |
| Recheck process usage in the library before deletion, retaining files and registration | Library Level-1 held-directory test | Passed |
| Keep Windows' open-file rename probe after the process check | Implementation inspection; existing probe tests remain | Preserved |
| Failed Git removal leaves target registered | Library Level-1 dirty-worktree refusal test | Error returned; files and registration retained |
| Failed Git removal unregisters target and leaves folder absent | New library Level-1 test through public removal | Success; branch retained for the caller |
| Failed Git removal unregisters target and leaves deletable remnants | New library Level-1 test through public removal | Remnants deleted; success; branch retained for the caller |
| Unregistered remnants cannot be deleted | Existing library Level-1 permissions test | `FolderNotFullyRemoved`; remnants and branch retained |
| Ordinary folder was never registered | Existing library Level-1 test | Refusal; contents retained |
| Successful recovery continues later removal steps | New CLI Level-1 test plus inspection of shared execution path | Branch deletion passed; copy-record cleanup and remote steps remain after successful removal |

Level 1 is appropriate for these filesystem, process-table, exit-code, and
workflow requirements. The spec introduces no keyboard encoding, terminal
rendering, or scrolling requirement that needs Level 2 or Level 3. This fix
adds no configuration or file-format reader, so the input robustness matrix
does not apply. Cross-OS execution evidence remains with CI and does not
block this review.

Executed on macOS:

- `just test remove::tests` in `worktree/`: 14 passed.
- `just test a_folder_refilled` in `worktree/`: 1 passed.
- `just test a_process_standing_in_the_worktree` in `worktree/`: 1 passed.
- `just test working_processes` in `sniff/`: 3 passed.
- `just lint` in `worktree/`: passed for both packages.
- `just check-tier-coverage worktree`: no stranded tests.

Every executed test finished below the five-second Level-1 budget. The repair
log additionally reports a passing full worktree Level-1 run and a mutation
check; this review did not repeat those checks.

## Recurrence

No finding recurs. The previous review's recovery coverage class is closed by
the public-entry-point tests and the CLI continuation test described above.

## Observations

None.
