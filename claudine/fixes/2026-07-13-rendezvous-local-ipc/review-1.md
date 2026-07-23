---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T18:36:01-07:00
spec: 2026-07-13-rendezvous-local-ipc/spec.md
implemented: false
description: A **fix** review of `2026-07-13-rendezvous-local-ipc/spec.md`
fix: 2026-07-13-rendezvous-local-ipc/review-1.md
previous: /
---

# Review 1: Rendezvous Local IPC

## Verdict

The fix is **not ready for production**. The macOS implementation is substantially present and the local package tests and lint checks pass, but the current design can retain daemon resources after endpoint startup fails, Windows accepts an insufficiently private existing data directory, and private-directory validation can follow an intermediate symlink when the final directory already exists. In addition, the required native Linux, Windows, and WSL runtime evidence has not been completed.

## Findings

### 1. Critical: Windows accepts a user-owned data root with a broadened DACL

`claudine/rendezvous/daemon/src/private_dir.rs:428-448` validates an existing Windows directory by checking that it is not the final symlink/reparse point and that its owner SID matches the current user. It does not inspect the directory's DACL. A directory owned by the current user can still grant another principal, such as `Everyone` or `Authenticated Users`, access to the directory. The daemon will accept that directory and use it for `node.key`, the database, and other identity-bearing state.

This does not satisfy the specification's requirement that the per-user data root use a current-user DACL and that explicit overrides preserve the authorization boundary. It also makes `verify_private()` weaker than the security policy applied when a new directory is created.

The existing Windows tests do not close this gap:

- `claudine/rendezvous/daemon/src/private_dir/tests.rs:202-250` checks the security descriptor constructed for a newly created directory, not a pre-existing user-owned directory with a widened DACL.
- `claudine/rendezvous/daemon/src/server/tests.rs:237-265` verifies rejection of a directory owned by a different principal.
- `claudine/rendezvous/daemon/src/server/tests.rs:302-321` calls the same ownership-only validator.

**Required change:** inspect and validate the DACL of every existing Windows data-root override. Define the principals and rights that are allowed, reject inherited or explicit access that broadens the boundary, and return a typed privacy error. Add a native Windows Level 1 integration test that creates a user-owned directory with an intentionally broadened DACL and proves daemon startup rejects it. Also assert the effective DACL of a directory created by the daemon, rather than only asserting the descriptor builder's output.

### 2. High: endpoint refusal happens after persistent daemon resources are started

`claudine/rendezvous/daemon/src/local_transport/mod.rs:54-70` calls `prepare_daemon()` before either Unix socket binding or Windows named-pipe creation. `prepare_daemon()` opens storage and starts background workers and discovery-related tasks (`claudine/rendezvous/daemon/src/server.rs:429-525`). The transport can then refuse startup for an existing endpoint, an ownership mismatch, a bind failure, or a Windows first-instance failure (`local_transport/unix.rs:44-53`, `local_transport/windows.rs:52-57`). `PreparedDaemon` has no coordinated asynchronous teardown path for this failure sequence.

This creates a nondeterministic lifetime: a rejected daemon can leave work in flight and storage handles alive long enough for an immediate restart to fail with `DatabaseAlreadyOpen`. The implementation plan itself records this behavior and the associated nextest flake at `claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md:714-730`.

GitNexus classifies the blast radius of `spawn_local_server` and `prepare_daemon` as critical: the startup path feeds the daemon entry point and numerous Unix, Windows, and client round-trip flows. That makes the ordering defect a production blocker even though the happy-path tests pass.

**Required change:** acquire the exclusive transport endpoint before preparing persistent daemon state. Represent the bound Unix listener or first Windows pipe instance as a transport-specific RAII value, then pass it into the serving loop after `prepare_daemon()` succeeds. Release the endpoint if preparation fails. Add Level 1 integration coverage that forces endpoint refusal and immediately starts a daemon with the same data root at a valid endpoint, proving storage can be reopened and no startup worker remains active.

### 3. High: an existing directory reached through an intermediate symlink is accepted

`missing_ancestors()` in `claudine/rendezvous/daemon/src/private_dir.rs:148-186` stops walking as soon as the requested path exists. `verify_private()` then checks only the final path with `symlink_metadata` (`private_dir.rs:352-368`, `private_dir.rs:393-448`). Because intermediate symlinks are followed while resolving the final path, a path such as `private-root/link/existing-data`, where `link` is a symlink and `existing-data` already exists at its destination, can pass validation.

The current symlink tests only cover a missing target below a symlink (`private_dir/tests.rs:141-158` and `local_transport/unix/tests.rs:159-185`). In those cases the upward missing-ancestor walk happens to encounter the symlink. They do not cover the existing-descendant case.

This violates the module's stated invariant that the private directory is not reachable through a component another user can re-point, and it weakens both the Unix socket-parent and data-root boundaries.

**Required change:** validate each relevant path component without following it. Prefer descriptor-relative or handle-relative traversal so validation and later use are tied to the same objects and do not introduce a check/use race. Preserve the documented macOS handling for shared system ancestors above the private boundary. Add native Unix Level 1 tests with an already-created descendant reached through an intermediate symlink for both the data root and runtime socket parent.

### 4. High: the required native runtime matrix and external authorization checks are incomplete

The specification requires runtime verification on macOS, Linux, and Windows and an actual WSL smoke test; cross-compilation or conditionally compiled test code is not sufficient. The recorded implementation evidence says:

- native macOS tests are green;
- Linux and Windows workflow execution has not been confirmed (`plan.md:657-690`);
- an actual WSL smoke test has not been run (`plan.md:629-633`);
- the second-principal authorization checks have no completed execution record (`plan.md:691-703`).

The workflow at `.github/workflows/rendezvous-tests.yml` provides the appropriate native jobs, but the existence of a workflow does not establish that those jobs pass. The Windows named-pipe round-trip, concurrency, busy-instance retry, remote-client rejection, and first-instance exclusion tests are appropriately Level 1 native integration tests; their strongest recorded execution evidence is still missing. The WSL marker test uses manufactured environment state and does not prove behavior in WSL. Same-user acceptance and different-user denial exercise native OS authorization and likewise require execution on the target OS.

**Required change:** obtain and record green native Linux and Windows workflow runs, run the documented WSL smoke test, and complete the second-principal checks wherever CI or a controlled host can provision another user. Any test that cannot be automated must have a reproducible command and captured result before this fix is marked ready.

## Requirement Verification Levels

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Stable user identity and endpoint derivation on macOS, Linux, Windows, and WSL | Level 1 unit/integration tests; macOS executed, other native runs not confirmed; WSL marker is manufactured | **Gap:** native Linux, Windows, and actual WSL evidence is required. |
| Default endpoint selection and override validation | Level 1 unit/integration tests; macOS executed | Appropriate level, but Windows and Linux execution remains unconfirmed. |
| Distinct users receive distinct endpoints | Level 1 deterministic derivation tests | Appropriate for derivation; native second-principal authorization remains incomplete. |
| Unix socket type, `0600` permissions, private parent, stale cleanup, and replacement resistance | Level 1 native Unix integration tests executed on macOS | Appropriate level. Add the intermediate-existing-symlink regression from Finding 3. |
| Real Unix daemon/client round trip | Level 1 native integration test executed on macOS | Appropriate level; Linux execution remains unconfirmed. |
| Windows named-pipe round trip, concurrent clients, busy retry, remote rejection, and second-daemon exclusion | Level 1 native integration tests are implemented | Appropriate level, but no confirmed native Windows execution. |
| Same-user access succeeds and other-user access is denied | Level 1 native authorization tests/manual check | **Gap:** native Windows execution and a recorded second-principal check are missing. |
| Per-user data root remains private | Level 1 unit/integration tests | **Gap:** the Windows DACL and intermediate-symlink cases are not implemented or tested. |
| Portable behavior at CLI and daemon call sites | Level 1 process/integration tests and static inspection | Appropriate level, subject to the missing Linux/Windows matrix run. |

Levels 2 and 3 are not applicable to this fix. It does not assert terminal glyph, styling, scrolling, paste, mouse, IME, keybinding, or physical-key behavior; running in a real terminal emulator or injecting OS keyboard events would not add coverage for the IPC transport or authorization boundary.

## Verification Performed

- `claudine/rendezvous`: `just test` — **271 passed, 2 skipped** on macOS.
- `claudine/rendezvous`: `just lint` — **passed** for `rendezvous-core`, `rendezvous-daemon`, and `rendezvous-client`.
- Focused `sniff` identity/endpoint tests — **9 passed** on macOS.
- Source inspection covered the core endpoint contract, Unix and Windows transports, client connectors, private-directory enforcement, daemon initialization, workflow, and implementation plan.

These results establish the macOS happy path and local static quality, but they do not resolve the security, lifecycle, or native-runtime gaps above.

## Closure Criteria

The fix can be reviewed again after all four findings are addressed, the new regressions pass, and native Linux, Windows, and WSL results are attached or otherwise recorded. Production readiness requires the authorization boundary and failure-path cleanup to be demonstrated, not only the successful transport path.
