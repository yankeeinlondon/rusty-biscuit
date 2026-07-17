---
total_phases: 8
created: 2026-07-16
phase: 7
agent: codex/default
yolo: true
packages:
    - sniff
    - rendezvous-core
    - rendezvous-client
    - rendezvous-daemon
    - claudine-cli
source_files_during_phase_1:
    - sniff/lib/src/os/user.rs
    - sniff/lib/src/os/mod.rs
    - sniff/lib/src/error.rs
    - sniff/lib/Cargo.toml
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/fixes/2026-07-13-rendezvous-local-ipc/change-notes.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/rendezvous/core/src/local_endpoint.rs
    - claudine/rendezvous/core/src/local_endpoint/tests.rs
    - claudine/rendezvous/core/src/lib.rs
    - claudine/rendezvous/core/Cargo.toml
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/rendezvous/client/src/connector/mod.rs
    - claudine/rendezvous/client/src/connector/unix.rs
    - claudine/rendezvous/client/src/connector/windows.rs
    - claudine/rendezvous/client/src/connector/tests.rs
    - claudine/rendezvous/client/src/lib.rs
    - claudine/rendezvous/client/src/main.rs
    - claudine/rendezvous/core/src/socket.rs
    - claudine/cli/src/commands/dashboard/mod.rs
    - claudine/cli/src/commands/dashboard/tests.rs
    - claudine/cli/src/commands/handle.rs
    - claudine/cli/src/commands/wrap/session_report.rs
    - claudine/cli/src/commands/wrap/session_report/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/rendezvous/daemon/src/private_dir.rs
    - claudine/rendezvous/daemon/src/private_dir/tests.rs
    - claudine/rendezvous/daemon/src/local_transport/mod.rs
    - claudine/rendezvous/daemon/src/local_transport/unix.rs
    - claudine/rendezvous/daemon/src/server.rs
    - claudine/rendezvous/daemon/src/server/tests.rs
    - claudine/rendezvous/daemon/src/lib.rs
    - claudine/rendezvous/daemon/src/main.rs
    - claudine/rendezvous/daemon/Cargo.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/rendezvous/daemon/src/local_transport/mod.rs
    - claudine/rendezvous/daemon/src/local_transport/unix.rs
    - claudine/rendezvous/daemon/src/local_transport/unix/tests.rs
    - claudine/rendezvous/daemon/src/local_transport/windows.rs
    - claudine/rendezvous/daemon/src/local_transport/windows/tests.rs
    - claudine/rendezvous/daemon/src/private_dir.rs
    - claudine/rendezvous/daemon/src/private_dir/tests.rs
    - claudine/rendezvous/daemon/src/server.rs
    - claudine/rendezvous/daemon/src/server/tests.rs
    - claudine/rendezvous/daemon/Cargo.toml
    - claudine/rendezvous/daemon/tests/peer_discovery.rs
    - claudine/rendezvous/daemon/tests/pairing_and_sync.rs
    - claudine/rendezvous/daemon/tests/phase6_integration.rs
    - claudine/rendezvous/client/tests/uds_round_trip.rs
    - claudine/rendezvous/client/tests/session_log_round_trip.rs
    - claudine/cli/src/commands/dashboard/tests.rs
    - claudine/cli/src/commands/wrap/session_report/tests.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
    - claudine/rendezvous/core/src/lib.rs
    - claudine/rendezvous/core/src/socket.rs
    - claudine/rendezvous/core/src/local_endpoint.rs
    - claudine/rendezvous/core/src/local_endpoint/test_support.rs
    - claudine/rendezvous/core/Cargo.toml
    - claudine/rendezvous/client/src/lib.rs
    - claudine/rendezvous/client/src/main.rs
    - claudine/rendezvous/client/Cargo.toml
    - claudine/rendezvous/client/tests/uds_round_trip.rs
    - claudine/rendezvous/client/tests/session_log_round_trip.rs
    - claudine/rendezvous/daemon/src/main.rs
    - claudine/rendezvous/daemon/src/server.rs
    - claudine/rendezvous/daemon/src/server/tests.rs
    - claudine/rendezvous/daemon/src/local_transport/mod.rs
    - claudine/rendezvous/daemon/Cargo.toml
    - claudine/rendezvous/daemon/tests/peer_discovery.rs
    - claudine/rendezvous/daemon/tests/pairing_and_sync.rs
    - claudine/rendezvous/daemon/tests/phase6_integration.rs
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/dashboard/mod.rs
    - claudine/cli/src/commands/dashboard/tests.rs
    - claudine/cli/src/commands/handle.rs
    - claudine/cli/src/commands/wrap/session_report.rs
    - claudine/cli/src/commands/wrap/session_report/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
    - rendezvous-core
    - rendezvous-client
    - rendezvous-daemon
    - claudine-cli
source_files_during_phase_7:
    - claudine/rendezvous/core/src/local_endpoint/tests.rs
    - claudine/rendezvous/client/src/connector/tests.rs
    - claudine/rendezvous/client/tests/local_round_trip.rs
    - claudine/rendezvous/daemon/src/server/tests.rs
    - claudine/rendezvous/daemon/src/local_transport/windows/tests.rs
    - .github/workflows/rendezvous-tests.yml
docs_updated_during_phase_7:
    - claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
    - rendezvous-core
    - rendezvous-client
    - rendezvous-daemon
---

# Cross-Platform, Per-User Rendezvous Local IPC — Execution Plan

Source: [`spec.md`](spec.md)

## Objective

Replace Rendezvous's path-shaped, Unix-server-only local IPC with one typed,
per-user control-plane contract: Unix-domain stream sockets on macOS, Linux,
and WSL; Windows named pipes on native Windows; and one shared daemon
initialization path for both transports. The endpoint, durable data root, and
node identity must all belong to the same stable OS user.

## Planning constraints

- `sniff` discovers the effective Unix UID or current process-token user SID;
  it does not authorize endpoints or add the identifier to ambient host
  inventory.
- `rendezvous-core` models and resolves `LocalEndpoint`; it performs no
  filesystem mutation.
- `rendezvous-daemon` owns endpoint/data-root authorization, listener setup,
  stale endpoint cleanup, and shutdown cleanup.
- `rendezvous-client` owns transport selection and bounded Windows pipe-busy
  retry; production callers stay platform-neutral.
- `default_local_endpoint()` must return a typed error when stable-user
  discovery or endpoint validation fails. It must never fall back to a
  username, `default`, or a process-random identifier.
- The Unix security boundary is the selected private runtime/data directory.
  Trusted ancestors such as `/tmp` may be shared, but every traversed component
  below that ancestor must be non-symlinked, and the private directory itself
  must be owned by the effective UID with owner-only access.
- Windows runtime tests are release-blocking. Cross-compilation is useful but
  cannot satisfy the named-pipe acceptance criteria.
- No compatibility aliases remain for `RENDEZVOUS_SOCKET`, `--socket`,
  `default_socket_path`, or `ServerHandle::socket_path()`.

## Success criteria

- All production local-control-plane callers use `LocalEndpoint` and the same
  `rendezvous_client::connect` entry point without target-specific branching.
- `spawn_local_server` performs persistence, identity, register, QUIC,
  discovery, and worker initialization exactly once, regardless of transport.
- Unix bind and cleanup reject unsafe types, owners, permissions, symlinks,
  active sockets, and endpoint replacement races.
- Windows named pipes use byte mode, reject remote clients, apply a current-user
  DACL, protect the first instance, keep an acceptor available, and support
  concurrent clients.
- The default durable root is the user's platform-local data directory and is
  private; `<tempdir>/rendezvous-data` is neither selected nor imported.
- Native macOS, Linux, and Windows runtime tests pass, and WSL remains on the
  Linux UID/UDS code path.

## Phase index

| Phase | Outcome | Depends on | Parallel work |
|---|---|---|---|
| 1 | Stable OS-user identity is available from Sniff | none | Unix and Windows implementations after the public contract is fixed |
| 2 | Core exposes the typed endpoint contract | 1 | Pure model/override tests and platform-default tests |
| 3 | Client connector consumes `LocalEndpoint` | 2 | Unix connector and Windows connector/error classification |
| 4 | Daemon initialization and ownership policy are transport-neutral | 1, 2 | Shared boot extraction and default data-root policy |
| 5 | Secure Unix and Windows server transports are implemented | 3, 4 | Unix and Windows workstreams |
| 6 | Every production caller and override uses the new API | 3, 5 | CLI call-site groups and test-fixture migration |
| 7 | Cross-platform runtime and CI gates prove the contract | 6 | OS-specific suites on native runners |
| 8 | Documentation, skills, and final regression checks are complete | 7 | Documentation surfaces and dependency records |

## Phase 1 — Add Stable User Identity to Sniff

**Goal:** Provide one narrow, uncached, on-demand API for the security principal
under which the current process runs.

- [x] Re-run GitNexus upstream impact analysis for `default_socket_path`,
  `spawn_uds_server`, `rendezvous_client::connect`, and the daemon data-root
  selection immediately before implementation; record direct callers,
  affected processes, and the existing CRITICAL endpoint risk in the change
  notes before editing those symbols.
- [x] Add `sniff/lib/src/os/user.rs` with public `StableUserId` variants
  `UnixUid(u32)` and `WindowsSid(String)`, plus a lossless deterministic
  endpoint/display projection that preserves the explicit variant semantics.
- [x] Add `current_user_id() -> sniff::Result<StableUserId>` and re-export the
  type and function from `sniff::os`; keep them out of `OsInfo`, `SniffResult`,
  default `sniff --json`, and existing host-capability caches.
- [x] Add typed `SniffError` variants for user-identity discovery/validation so
  OS failures preserve their operation and source instead of collapsing to
  username text.
- [x] Implement the Unix branch with `libc::geteuid()` only; document that WSL
  compiles through this branch and never inspects or correlates a Windows token.
- [x] Implement the Windows branch with RAII-managed process-token and SID
  resources: query `TokenUser`, validate the account SID, convert it to
  canonical `S-1-...` form, and release every handle/allocation on every path.
- [x] Confirm the exact `windows = 0.62` feature set against the pinned generated
  APIs, then add only the required target-specific features (expected:
  `Win32_Foundation`, `Win32_Security`, `Win32_Security_Authorization`, and
  `Win32_System_Threading`).
- [x] Add Sniff unit/platform tests for equality and display/endpoint
  projection, actual effective UID, environment independence from `USER`,
  `LOGNAME`, and `UID`, canonical Windows `TokenUser` SID, repeat-call
  stability, and typed failure behavior; do not add serde unless a real public
  consumer requires serialization.
- [x] **Parallelizable:** Once `StableUserId` and the error contract are fixed,
  implement and review the Unix and Windows backends independently.

**Validation checkpoint**

- [x] Run the focused `sniff::os::user` tests and `cargo check -p sniff
  --all-targets` on the development host.
- [x] Run `cd sniff && just test` and `cd sniff && just lint`; defer final
  macOS/Linux/Windows runtime confirmation to Phase 7.
- [x] Verify by source search that the new identifier is absent from `OsInfo`,
  `SniffResult`, default JSON output, caches, subprocess calls, and network
  paths.

## Phase 2 — Replace the Path-Shaped Core Endpoint API

**Goal:** Make transport semantics explicit before migrating any server or
client call sites.

- [x] Replace `rendezvous-core/src/socket.rs` with
  `rendezvous-core/src/local_endpoint.rs`, export it from `lib.rs`, and define
  `LocalEndpoint::{UnixSocket(PathBuf), WindowsNamedPipe(OsString)}` without an
  ambiguous common path accessor.
- [x] Add explicit target-appropriate accessors and a safe display/rendering
  projection; preserve the Windows pipe name as `OsString` end to end and never
  use `PathBuf::to_string_lossy()` for transport dispatch.
- [x] Define `RENDEZVOUS_ENDPOINT` and typed override parsing/validation; reject
  an empty, malformed, or target-incompatible endpoint with a distinct error.
- [x] Implement `default_local_endpoint() -> Result<LocalEndpoint, _>` using
  `sniff::os::current_user_id()`: validated
  `$XDG_RUNTIME_DIR/claudine/rendezvous/daemon.sock` on Linux/WSL, a
  UID-qualified private temp fallback on Unix, and a SID-qualified
  `\\.\pipe\claudine-rendezvous-<sid>` name on Windows.
- [x] Keep default resolution non-mutating: it may inspect ownership, type,
  symlink status, and mode, but directory creation/removal remains in the
  daemon. Treat an invalid XDG runtime directory as unavailable and use the
  UID-qualified fallback; do not accept an unsafe explicit override.
- [x] Add `sniff` as a direct `rendezvous-core` dependency and update dependency
  declarations without introducing a reverse edge from Sniff into Rendezvous.
- [x] Add core tests for enum/accessor behavior, `RENDEZVOUS_ENDPOINT`
  precedence, UID/SID-qualified defaults, username-environment independence,
  XDG acceptance/rejection, fallback selection, and target-incompatible
  variants. Serialize environment-mutating tests within each test process.
- [x] **Parallelizable:** Pure enum/override tests can proceed alongside the
  Unix and Windows default-derivation tests after the public signatures land.

**Validation checkpoint**

- [x] Run `cargo nextest run -p rendezvous-core` and `cargo check -p
  rendezvous-core --all-targets`.
- [x] Verify `rendezvous-core` contains no directory creation, endpoint removal,
  Unix listener, Windows pipe listener, or lossy pipe-name conversion.

## Phase 3 — Build the Portable Client Connector

**Goal:** Give all consumers one typed connector with distinguishable failures
before the server and call sites migrate.

- [x] Split `rendezvous-client` into `connector/mod.rs`, `connector/unix.rs`,
  and `connector/windows.rs`; make `connect` accept `LocalEndpoint` (or a
  borrow) and reject a target-incompatible variant explicitly.
- [x] Keep the Unix connector on `tokio::net::UnixStream` and retain tonic's
  placeholder URI only as HTTP/2 plumbing, not endpoint identity.
- [x] Open Windows pipes directly from `OsStr`/`OsString`; on
  `ERROR_PIPE_BUSY`, wait/retry against one bounded deadline with deterministic
  backoff and no unbounded sleep.
- [x] Expand `ConnectError` so endpoint-not-found, permission-denied,
  busy-timeout, incompatible endpoint, and other listener/tonic transport
  failures remain distinguishable; add a private OS-error classifier that
  preserves the original source chain.
- [x] Add deterministic unit tests for endpoint dispatch, deadline exhaustion,
  retry success, and OS error classification using injected connector/retry
  seams; keep real byte-stream tests for Phase 7.
- [x] **Parallelizable:** Implement Unix dispatch and Windows busy/error logic
  independently after the shared `connect` and `ConnectError` contracts are
  fixed.

**Validation checkpoint**

- [x] Run `cargo nextest run -p rendezvous-client --lib` and `cargo check -p
  rendezvous-client --all-targets`.
- [x] Verify no client API accepts a generic `PathBuf` for a Windows named pipe
  and no production caller needs a `cfg(unix)`/`cfg(windows)` branch.

## Phase 4 — Extract One Shared Daemon Boot and Ownership Policy

**Goal:** Separate daemon initialization from transport binding so the Windows
implementation cannot duplicate the storage/network stack.

- [x] Refactor `rendezvous-daemon/src/server.rs` into a shared preparation and
  serve pipeline plus `local_transport/mod.rs`; construct storage, projection,
  batcher, node identity, session log, registers, capability refresher, QUIC,
  discovery, peer workers, and `RendezvousService` exactly once.
- [x] Introduce `spawn_local_server(LocalEndpoint, DaemonConfig)` as the only
  production entry point. Retain `spawn_uds_server` only if a Unix-only test
  seam remains materially useful, and keep it out of production call sites.
- [x] Change `ServerHandle` to expose `local_endpoint()` and own a
  transport-specific cleanup token; keep graceful shutdown/drop behavior for
  shared workers while allowing Unix instance-safe cleanup and Windows
  handle-only teardown.
- [x] Define typed `ServerError` categories for invalid/incompatible endpoint,
  endpoint in use, access denied, ownership violation, listener failure, and
  cleanup failure while preserving existing storage/network error sources.
- [x] Add `default_data_dir()` under the user's platform-local data directory
  (`<local-data-dir>/claudine/rendezvous`) and route both default and
  `--data-dir` roots through the same private-directory validation contract.
- [x] Add one reusable daemon-private directory-security helper for the two
  justified consumers—the Unix runtime directory and durable data root—rather
  than duplicating owner/type/symlink/mode checks.
- [x] Prohibit automatic discovery or import of `<tempdir>/rendezvous-data`;
  make tests assert that the legacy path is neither selected nor read.
- [x] Add test instrumentation or a test-only initialization counter proving
  that portable startup initializes shared subsystems once per daemon.
- [x] **Parallelizable:** Shared boot extraction and default data-root selection
  can proceed together once the `ServerHandle`/error contracts are agreed.

**Validation checkpoint**

- [x] Run daemon library tests that do not require a bound local transport and
  `cargo check -p rendezvous-daemon --all-targets`.
- [x] Review platform modules to confirm they receive a prepared service and
  own only listener, accept, permission, and cleanup logic.

## Phase 5 — Implement Secure Platform Server Transports

**Goal:** Complete both native transports against the shared daemon pipeline.

### Unix workstream

- [x] Add `local_transport/unix.rs` to walk endpoint-parent components with
  non-following metadata, reject symlinks/non-directories, create the private
  runtime directory with mode `0700` without a permissive creation window, and
  verify effective-UID ownership plus no group/other access.
- [x] Classify a pre-existing endpoint with `symlink_metadata`; reject regular
  files, directories, symlinks, foreign-owned sockets, and active sockets, and
  remove only a stale socket owned by the expected UID.
- [x] Bind `UnixListener`, force socket mode `0600` independent of umask, and
  capture the bound socket's device/inode/owner identity for cleanup.
- [x] On shutdown/drop, remove the endpoint only if fresh non-following metadata
  still matches the captured socket instance; leave any replaced endpoint
  untouched and report cleanup failures without deleting foreign data.
- [x] Apply the same owner/type/symlink/private-mode policy to the default and
  overridden Unix data root before opening identity or database files.

### Windows workstream

- [x] Add `local_transport/windows.rs` using
  `tokio::net::windows::named_pipe::ServerOptions` in byte mode with
  `reject_remote_clients(true)` and `first_pipe_instance(true)` on initial
  creation.
- [x] Build an RAII-owned security descriptor/DACL granting the current user SID
  the required pipe access (with administrator/system handling limited to the
  stated threat boundary), pass it through Tokio's security-attributes creation
  API, and free every Win32 allocation on success and failure.
- [x] Implement an incoming stream that creates the next pipe instance before
  yielding the connected instance, supports concurrent clients, and closes all
  pending/connected instances on shutdown without filesystem cleanup.
- [x] Map first-instance collisions, access denial, connection/accept failure,
  and shutdown to the typed server errors from Phase 4.
- [x] Apply a current-user Windows DACL to the data root and validate explicit
  `--data-dir` overrides before the identity seed or databases are opened.

### Parallelization and review

- [x] **Parallelizable:** Run the Unix and Windows workstreams independently
  after Phases 3–4; they may share only the prepared-service interface,
  ownership policy inputs, and typed errors.
- [x] Review the two platform modules together to verify neither duplicates
  persistence, identity, register, QUIC, discovery, or worker initialization.

**Validation checkpoint**

- [x] On Unix, run focused tests for private directory creation, exact modes,
  safe existing directories, symlink parents, wrong endpoint types,
  foreign-owned entries where permitted, stale owned sockets, active sockets,
  and replacement-safe teardown.
    - Foreign-*owned* entries are the one gap: planting one needs a second UID,
      which the unprivileged macOS dev host cannot provision. The classifier's
      other refusals are covered; Phase 7 owns the multi-principal leg.
- [ ] On Windows, run focused tests for first-instance exclusion, same-user
  DACL inspection, remote-client rejection, acceptor continuity, and clean
  shutdown; Phase 7 supplies complete gRPC/concurrency runtime coverage.
    - **Blocked on this host, deferred to Phase 7's `windows-latest` runner.**
      The tests are written (`local_transport/windows/tests.rs`, plus the two
      Windows cases in `private_dir/tests.rs`) but cannot be *run* from macOS.
      Cross-compiling the daemon is also impossible here: `duckdb-sys`'s bundled
      C++ overflows mingw's COFF section limit (`too many sections`), and MSVC
      is the real target anyway. Evidence obtained instead: every Win32 surface
      Phase 5 adds (`private_dir`'s DACL/`CreateDirectoryW`/
      `GetNamedSecurityInfoW` half, `local_transport/windows.rs`, and both test
      files' Win32 bodies) was compiled clean for `x86_64-pc-windows-gnu` in a
      throwaway probe crate with the duckdb-bearing seams stubbed. That proves
      the API signatures, not the runtime behavior.
- [x] Run `cd claudine/rendezvous && just check` after both target branches
  compile.
    - Unix branch: `just check` + `just test` + `just lint` green. Windows
      branch: probe-checked only, per the note above.

## Phase 6 — Migrate Overrides, Production Callers, and Fixtures

**Goal:** Complete the atomic public API migration and remove all production
dependence on the legacy socket vocabulary.

- [x] Update the daemon binary to accept `--endpoint` /
  `RENDEZVOUS_ENDPOINT`, resolve `default_local_endpoint()`, select the private
  default data root, call `spawn_local_server`, and log the typed endpoint
  without assuming filesystem display semantics.
- [x] Update `rendezvous-test-client` to accept `--endpoint` /
  `RENDEZVOUS_ENDPOINT` as an OS-native value and connect through the portable
  client API.
- [x] Migrate dashboard access, lifecycle requeue, hook forwarding/session
  presence in `commands/handle.rs`, wrapped-session status reporting, the test
  client, daemon probes, and all associated tests to `LocalEndpoint` with no
  platform-specific call-site branches.
- [x] Move the `rendezvous-daemon` Claudine CLI dev-dependency out of its
  Unix-only target section once daemon-spawning tests compile and run through
  `spawn_local_server` on Windows.
- [x] Replace `ServerHandle::socket_path()` with `local_endpoint()` and migrate
  every daemon/client integration fixture. Use private temporary Unix parents
  or explicit test-only endpoint constructors; never weaken production
  ownership checks for tests.
- [x] Remove `RENDEZVOUS_SOCKET`, `--socket`, `default_socket_path`, the public
  `socket` module, ambiguous socket/path naming, and obsolete Unix-only test
  gates. Keep legacy names only in historical/superseded documentation that
  explicitly labels them as old behavior.
- [x] Update behavior-changing rustdoc/module comments in the same edits,
  removing the stale claim that the daemon is Unix-only and avoiding comments
  that merely narrate the implementation.
- [x] **Parallelizable:** After the endpoint/server APIs stabilize, migrate the
  dashboard/handle group, requeue/session-report group, daemon/test-client
  binaries, and integration fixtures independently.

**Validation checkpoint**

- [x] Run source searches proving all production local IPC call sites use
  `LocalEndpoint`, `rendezvous_client::connect`, and `spawn_local_server`.
- [x] Run source searches for the legacy names and review every remaining match
  as either a historical record or a defect.
    - Zero matches in any `.rs`/`.toml`. Two Markdown matches, both reviewed and
      neither a defect: `claudine/reviews/2026-06-26-cross-platform/review-1.md`
      is a dated historical review that *recommended* this rename, and
      `claudine/docs/rendezvous/current-state.md` still documents the old
      resolution order — Phase 8 owns that file by name, so it is left for
      Phase 8 rather than half-corrected here.
- [x] Run `cd claudine/rendezvous && just test` and the focused Claudine CLI
  dashboard, handle, requeue, and session-report tests on the development host.
    - `claudine/rendezvous`: 266 passed. Claudine CLI focused suites: 41 passed
      (dashboard, session-report, requeue, dispatch-inventory) + 10 handle.
      Full `cd claudine && just test`: 5453 passed. Two pre-existing failures
      unrelated to this phase, both verified against untouched files:
      `rendezvous-daemon local_transport::unix::tests::a_directory_at_the_endpoint_is_rejected`
      (flaky; passes in isolation; file untouched by Phase 6) and
      `claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`
      (missing fixture baseline on HEAD; `claudine/gen` untouched by Phase 6).

## Phase 7 — Prove Runtime Security and Cross-Platform Behavior

**Goal:** Turn the acceptance matrix into native-host regression and CI gates.

- [x] Consolidate local-control-plane integration coverage around a portable
  daemon/client gRPC round trip that runs on macOS, Linux, and Windows; rename
  Unix-specific test files where their scope becomes cross-platform.
    - `client/tests/uds_round_trip.rs` → `local_round_trip.rs`; its one
      remaining `cfg(unix)` case is the socket-unlink assertion, which has no
      Windows meaning and says so. `server/tests.rs`'s two release tests were
      Unix-only by accident (they called a `cfg(unix)` helper) and are now
      portable via an `endpoint_is_bound` probe.
- [x] Add Unix integration cases for `0700` runtime/data directories, `0600`
  sockets, unsafe parent and endpoint types, stale-versus-active sockets,
  foreign ownership where the runner permits it, and endpoint replacement
  during teardown.
    - Phase 5 landed these in `local_transport/unix/tests.rs` (16 cases, all
      driving the real `spawn_local_server`). Phase 7 adds the data-root half:
      `the_data_root_is_created_owner_only`. Foreign *ownership* remains the
      one gap — planting a foreign-owned entry needs a second UID, which
      neither the macOS dev host nor a GitHub runner can provision
      unprivileged.
- [x] Add Windows integration cases for a real daemon/client gRPC round trip,
  two concurrent clients, bounded `ERROR_PIPE_BUSY` recovery, second-daemon
  exclusion, remote-client rejection, same-user acceptance, DACL contents, and
  other-user denial where CI can provision a second principal.
    - Written, not run here (see the checkpoint below). Round trip + same-user
      acceptance + two concurrent clients: `local_round_trip.rs`, portable, so
      Windows runs the same assertions macOS does. Bounded `ERROR_PIPE_BUSY`:
      new `connector/tests.rs::live_pipe`, which saturates a *real* pipe — the
      pre-existing seam tests fabricate the error and so could not have caught
      a wrong `ERROR_PIPE_BUSY` constant. Second-daemon exclusion and DACL
      contents: Phase 5. Remote-client rejection: new
      `a_remote_form_client_is_refused_while_the_local_one_is_served`.
      Other-user denial: needs a second principal; not provisionable on a
      GitHub runner without administrative setup.
- [x] Add durable-root tests on every native OS proving the default is
  per-user/private, identity and databases share that root, overrides retain
  the authorization policy, and legacy temp state is never imported.
    - Per-user/private default: `default_data_dir_is_the_platform_local_data_directory`
      + `default_data_dir_is_not_the_legacy_temp_root` + the two new
      `the_data_root_is_created_*` cases. Shared root:
      `every_durable_path_sits_under_the_validated_root`. Overrides:
      `an_overridden_data_root_keeps_the_ownership_policy` (Unix mode) and the
      new `a_data_root_owned_by_another_account_is_rejected` (Windows
      ownership — there are no mode bits to check there). Legacy temp:
      `the_legacy_temp_root_is_neither_selected_nor_read`.
- [x] Add call-site round trips for dashboard, lifecycle requeue, hook
  forwarding, session reporting, test client, and daemon health/status; ensure
  the portable initialization counter is one for both Unix and Windows.
    - The 50 existing call-site tests are now run by the new workflow on all
      three OSes rather than only on Linux, which is what was missing. The
      counter assertion (`one_daemon_runs_the_shared_boot_exactly_once`) is
      transport-neutral and therefore already covers both.
- [x] Add Linux coverage that sets common WSL markers and username variables
  while asserting `UnixUid`/UDS selection, plus an actual smoke run on a
  Windows-hosted or designated WSL runner; do not close the phase until WSL has
  executed the Unix branch without adding native-Windows SID correlation.
    - Marker coverage added: `wsl_markers_do_not_move_resolution_off_the_uid_and_uds_path`
      and `wsl_markers_do_not_move_identity_off_the_unix_branch` set
      `WSL_DISTRO_NAME`/`WSL_INTEROP`/`WSLENV` plus the `USERNAME` that WSL
      interop really does propagate from the Windows side, and assert
      `UnixUid` + a UID-qualified socket with no name leaking into it.
    - **The actual WSL smoke run is not done.** GitHub-hosted runners do not
      offer WSL, so this needs a self-hosted or manually-attested run. The
      plan's own instruction is not to close the phase until WSL has executed
      the Unix branch, so this task is complete only as to the coverage, not
      the run.
- [x] Add or extend a path-filtered Rendezvous workflow to run
  `cargo check --all-targets` for `sniff`, all three Rendezvous crates, and
  `claudine-cli`, then run the relevant nextest suites on native
  `macos-latest`, `ubuntu-latest`, and `windows-latest` runners.
    - New `.github/workflows/rendezvous-tests.yml`. It deliberately does not
      use the shared `_area-ci.yml`: that block compile-checks macOS only and
      soft-fails Windows, and this phase requires the opposite of both.
- [x] Make the Windows Rendezvous runtime leg gating (not soft-fail) and retain
  test artifacts/logs that identify endpoint, permission, retry, and teardown
  failures without exposing the full user SID.
    - No `continue-on-error` on any leg. Logs are teed per OS and passed
      through a redaction step that collapses a user SID's trailing RIDs
      (`S-1-5-21-…` → `S-1-5-21-<redacted>`) while leaving well-known SIDs and
      the endpoint/permission/retry/teardown text readable.
- [x] Keep Sniff's existing three-OS matrix as the stable-user detector gate;
  add the new focused tests to that suite rather than creating a second Sniff
  workflow.
    - No change needed: `test.yml`'s `sniff-cross-platform` job already runs
      `cd sniff && just test` on all three OSes, and Phase 1's
      `sniff::os::user` tests are library unit tests, so that matrix is already
      the gate. The new workflow adds only Sniff's *compile* check, so a
      `sniff::os::user` change that breaks a Rendezvous consumer fails against
      Rendezvous rather than only against Sniff.
- [ ] **Parallelizable:** Run native macOS, Linux, and Windows validation legs
  concurrently once the migrated test suite is committed to the branch.
    - Blocked on the branch being pushed, which is a separate operation.

**Validation checkpoint**

- [ ] Confirm green native runtime results for macOS UDS, Linux UDS, and Windows
  named pipes; do not close the phase on cross-compilation-only evidence.
    - **macOS UDS: green on this host.** `cd claudine/rendezvous && just test`
      → 271 passed (82 core / 168 daemon / 21 client), `just lint` clean, and
      the 50 claudine-cli call-site tests pass.
    - **Linux UDS and Windows named pipes: not confirmed.** Both require the
      new workflow to actually run, which needs the branch pushed — a separate
      operation. Windows cannot be run or even cross-compiled here: the
      daemon's `duckdb-sys` overflows mingw's COFF section limit (`too many
      sections`), and every crate that reaches the daemon — including the
      client's own test targets, via its `rendezvous-daemon` dev-dependency —
      inherits that. MSVC is the real target regardless.
    - Evidence obtained here instead, which is signatures and not runtime:
      `cargo check --target x86_64-pc-windows-gnu -p rendezvous-core
      --all-targets` and `-p rendezvous-client --lib` both compile clean, and a
      throwaway probe crate compiled this phase's new tokio named-pipe test
      surfaces (`ServerOptions::first_pipe_instance/max_instances/create`,
      `ClientOptions::open`, the remote-form open) for `windows-gnu`.
    - Phase 7 therefore **cannot be closed** on this host. What it can and does
      deliver is that the Windows leg will compile and gate when it runs: the
      daemon's own test suite could not have compiled on Windows before this
      phase (see the defect note below).
- [ ] Confirm the CI matrix compiles all affected targets and exercises both
  transport branches, including Windows concurrency and busy-retry tests.
    - Blocked on the same push. The workflow's shape is verified (parses; three
      OSes; no `continue-on-error`; `--all-targets` over `sniff`,
      `rendezvous-{core,client,daemon}`, `claudine-cli`), and the redaction
      filter is verified against sample SID-bearing log text.
- [ ] Confirm a second user derives a distinct endpoint without consulting
  username environment variables; where multi-principal execution is
  unavailable, require DACL/owner inspection plus a documented external
  multi-user verification record.
    - Multi-principal execution is unavailable on both the unprivileged macOS
      dev host and GitHub-hosted runners. The fallback the plan allows —
      DACL/owner inspection — is covered
      (`the_pipe_dacl_names_this_user_and_nobody_else`,
      `the_current_user_descriptor_names_this_account_and_nobody_else`,
      `a_directory_owned_by_another_account_is_rejected`), and the
      username-independence half is covered by
      `default_ignores_username_environment_variables` and the two new WSL
      cases. **The external multi-user verification record is still owed.**

**Defect found while proving the contract (not introduced by this phase)**

- The daemon's own test suite could not compile on Windows: `server/tests.rs`
  called a `cfg(unix)` helper and imported `PermissionsExt` from three ungated
  tests, so `cargo check -p rendezvous-daemon --all-targets` would have failed
  on the very runner this phase adds. Fixed here — the release tests are now
  portable and the mode-bit tests are gated with Windows counterparts added.
  This is precisely what a compile-check-only Windows story hides, and why the
  new workflow runs `--all-targets`.
- [ ] **Open, deferred:** every endpoint refusal in `unix::serve`
  (`ensure_private_dir`, `clear_endpoint`, `bind`) runs *after*
  `prepare_daemon` has opened redb/DuckDB and spawned the batcher thread and
  capability refresher — and `PreparedDaemon` has no `Drop`, so dropping it on
  a transport failure leaks those workers with the storage handles still open.
  Two consequences: (a) under parallel load this trips nextest's 100ms
  `leak-timeout` (`result = "fail"` repo-wide), which is the pre-existing flake
  Phase 6 recorded against `a_directory_at_the_endpoint_is_rejected` and which
  reappeared here as `a_shared_parent_directory_is_rejected` — a flaky test in
  a *gating* leg is a real problem; (b) in production a failed bind leaves redb
  open briefly, which is the `DatabaseAlreadyOpen` trap this codebase already
  knows about and which `an_incompatible_endpoint_is_rejected_without_booting`
  exists to prevent for the sibling case. The fix is an ordering change — bind
  the endpoint before `prepare_daemon`, releasing it via the existing cleanup
  token if preparation then fails — which reopens Phase 4/5 boot code and
  touches the Windows transport that cannot be run here. Deferred rather than
  attempted blind; nextest's `retries = 3` masks it today.

## Phase 8 — Documentation, Skills, Drift, and Final Validation

**Goal:** Make the implemented contract authoritative everywhere and finish
with package-area and graph-based regression checks.

- [ ] Add `claudine/docs/rendezvous/local-ipc.md` as the authoritative document
  for transport selection, stable-user ownership, endpoint/data overrides,
  Unix permissions and cleanup, Windows DACL/accept behavior, WSL separation,
  errors/retry, manual legacy-data migration, and the threat boundary.
- [ ] Update `claudine/docs/rendezvous/design.md` to summarize and link to the
  authoritative IPC document, and correct spelling/terminology drift touched by
  that section without rewriting unrelated design content.
- [ ] Update `claudine/docs/rendezvous/current-state.md` so implementation
  status, endpoint names, flags/environment variables, diagrams, and platform
  claims match the shipped server.
- [ ] Reduce
  `claudine/features/2026-07-12-rendezvous-dashboard/windows-support-followup.md`
  to a superseded pointer to this fix and the new IPC document; preserve it as
  historical evidence rather than rewriting its findings.
- [ ] Update `claudine/rendezvous/README.md` diagrams and status to show the
  platform-native local plane separately from authenticated remote QUIC.
- [ ] Update `claudine/docs/dependencies.md` and the repository dependency
  record for the new `rendezvous-core -> sniff`, data-directory, and precise
  Windows API feature edges; add a Sniff area dependency note if no existing
  area document can record the expanded Windows features.
- [ ] Update `.claude/skills/claudine/{SKILL.md,architecture.md}` with the typed
  endpoint, portable daemon entry point, platform transports, data root, test
  commands, and crate responsibilities.
- [ ] Update the Sniff skill's public API/architecture references with
  `StableUserId` and its on-demand privacy boundary, without presenting it as
  default host inventory.
- [ ] **Parallelizable:** Documentation, dependency records, and the two skill
  updates can proceed together after Phase 7 behavior is stable, followed by
  one terminology/link consistency review.

**Final validation checkpoint**

- [ ] Run `cd sniff && just test && just test-l2 && just lint`.
- [ ] Run `cd claudine/rendezvous && just check && just build && just test &&
  just test-l2 && just lint`.
- [ ] Run the affected Claudine package-area suites with `cd claudine && just
  test && just test-l2 && just lint`.
- [ ] Run the repository macOS/Linux/Windows compile matrix and the dedicated
  native Rendezvous runtime matrix; archive the CI run links in the fix record.
- [ ] Run `cargo fmt --check` only as a read-only diagnostic; do not run
  `cargo fmt` write mode.
- [ ] Run GitNexus `detect_changes(scope: "compare", base_ref: "main")` and
  verify the affected symbols/processes are limited to stable-user discovery,
  local endpoint resolution, daemon startup, client connection, dashboard,
  lifecycle requeue/hook forwarding, and session reporting.
- [ ] Re-check every specification acceptance criterion against tests and
  documentation, then mark the fix complete only when all native runtime gates
  are green.
