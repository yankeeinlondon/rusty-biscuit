---
total_phases: 8
created: 2026-07-16
phase: 4
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

- [ ] Add `local_transport/unix.rs` to walk endpoint-parent components with
  non-following metadata, reject symlinks/non-directories, create the private
  runtime directory with mode `0700` without a permissive creation window, and
  verify effective-UID ownership plus no group/other access.
- [ ] Classify a pre-existing endpoint with `symlink_metadata`; reject regular
  files, directories, symlinks, foreign-owned sockets, and active sockets, and
  remove only a stale socket owned by the expected UID.
- [ ] Bind `UnixListener`, force socket mode `0600` independent of umask, and
  capture the bound socket's device/inode/owner identity for cleanup.
- [ ] On shutdown/drop, remove the endpoint only if fresh non-following metadata
  still matches the captured socket instance; leave any replaced endpoint
  untouched and report cleanup failures without deleting foreign data.
- [ ] Apply the same owner/type/symlink/private-mode policy to the default and
  overridden Unix data root before opening identity or database files.

### Windows workstream

- [ ] Add `local_transport/windows.rs` using
  `tokio::net::windows::named_pipe::ServerOptions` in byte mode with
  `reject_remote_clients(true)` and `first_pipe_instance(true)` on initial
  creation.
- [ ] Build an RAII-owned security descriptor/DACL granting the current user SID
  the required pipe access (with administrator/system handling limited to the
  stated threat boundary), pass it through Tokio's security-attributes creation
  API, and free every Win32 allocation on success and failure.
- [ ] Implement an incoming stream that creates the next pipe instance before
  yielding the connected instance, supports concurrent clients, and closes all
  pending/connected instances on shutdown without filesystem cleanup.
- [ ] Map first-instance collisions, access denial, connection/accept failure,
  and shutdown to the typed server errors from Phase 4.
- [ ] Apply a current-user Windows DACL to the data root and validate explicit
  `--data-dir` overrides before the identity seed or databases are opened.

### Parallelization and review

- [ ] **Parallelizable:** Run the Unix and Windows workstreams independently
  after Phases 3–4; they may share only the prepared-service interface,
  ownership policy inputs, and typed errors.
- [ ] Review the two platform modules together to verify neither duplicates
  persistence, identity, register, QUIC, discovery, or worker initialization.

**Validation checkpoint**

- [ ] On Unix, run focused tests for private directory creation, exact modes,
  safe existing directories, symlink parents, wrong endpoint types,
  foreign-owned entries where permitted, stale owned sockets, active sockets,
  and replacement-safe teardown.
- [ ] On Windows, run focused tests for first-instance exclusion, same-user
  DACL inspection, remote-client rejection, acceptor continuity, and clean
  shutdown; Phase 7 supplies complete gRPC/concurrency runtime coverage.
- [ ] Run `cd claudine/rendezvous && just check` after both target branches
  compile.

## Phase 6 — Migrate Overrides, Production Callers, and Fixtures

**Goal:** Complete the atomic public API migration and remove all production
dependence on the legacy socket vocabulary.

- [ ] Update the daemon binary to accept `--endpoint` /
  `RENDEZVOUS_ENDPOINT`, resolve `default_local_endpoint()`, select the private
  default data root, call `spawn_local_server`, and log the typed endpoint
  without assuming filesystem display semantics.
- [ ] Update `rendezvous-test-client` to accept `--endpoint` /
  `RENDEZVOUS_ENDPOINT` as an OS-native value and connect through the portable
  client API.
- [ ] Migrate dashboard access, lifecycle requeue, hook forwarding/session
  presence in `commands/handle.rs`, wrapped-session status reporting, the test
  client, daemon probes, and all associated tests to `LocalEndpoint` with no
  platform-specific call-site branches.
- [ ] Move the `rendezvous-daemon` Claudine CLI dev-dependency out of its
  Unix-only target section once daemon-spawning tests compile and run through
  `spawn_local_server` on Windows.
- [ ] Replace `ServerHandle::socket_path()` with `local_endpoint()` and migrate
  every daemon/client integration fixture. Use private temporary Unix parents
  or explicit test-only endpoint constructors; never weaken production
  ownership checks for tests.
- [ ] Remove `RENDEZVOUS_SOCKET`, `--socket`, `default_socket_path`, the public
  `socket` module, ambiguous socket/path naming, and obsolete Unix-only test
  gates. Keep legacy names only in historical/superseded documentation that
  explicitly labels them as old behavior.
- [ ] Update behavior-changing rustdoc/module comments in the same edits,
  removing the stale claim that the daemon is Unix-only and avoiding comments
  that merely narrate the implementation.
- [ ] **Parallelizable:** After the endpoint/server APIs stabilize, migrate the
  dashboard/handle group, requeue/session-report group, daemon/test-client
  binaries, and integration fixtures independently.

**Validation checkpoint**

- [ ] Run source searches proving all production local IPC call sites use
  `LocalEndpoint`, `rendezvous_client::connect`, and `spawn_local_server`.
- [ ] Run source searches for the legacy names and review every remaining match
  as either a historical record or a defect.
- [ ] Run `cd claudine/rendezvous && just test` and the focused Claudine CLI
  dashboard, handle, requeue, and session-report tests on the development host.

## Phase 7 — Prove Runtime Security and Cross-Platform Behavior

**Goal:** Turn the acceptance matrix into native-host regression and CI gates.

- [ ] Consolidate local-control-plane integration coverage around a portable
  daemon/client gRPC round trip that runs on macOS, Linux, and Windows; rename
  Unix-specific test files where their scope becomes cross-platform.
- [ ] Add Unix integration cases for `0700` runtime/data directories, `0600`
  sockets, unsafe parent and endpoint types, stale-versus-active sockets,
  foreign ownership where the runner permits it, and endpoint replacement
  during teardown.
- [ ] Add Windows integration cases for a real daemon/client gRPC round trip,
  two concurrent clients, bounded `ERROR_PIPE_BUSY` recovery, second-daemon
  exclusion, remote-client rejection, same-user acceptance, DACL contents, and
  other-user denial where CI can provision a second principal.
- [ ] Add durable-root tests on every native OS proving the default is
  per-user/private, identity and databases share that root, overrides retain
  the authorization policy, and legacy temp state is never imported.
- [ ] Add call-site round trips for dashboard, lifecycle requeue, hook
  forwarding, session reporting, test client, and daemon health/status; ensure
  the portable initialization counter is one for both Unix and Windows.
- [ ] Add Linux coverage that sets common WSL markers and username variables
  while asserting `UnixUid`/UDS selection, plus an actual smoke run on a
  Windows-hosted or designated WSL runner; do not close the phase until WSL has
  executed the Unix branch without adding native-Windows SID correlation.
- [ ] Add or extend a path-filtered Rendezvous workflow to run
  `cargo check --all-targets` for `sniff`, all three Rendezvous crates, and
  `claudine-cli`, then run the relevant nextest suites on native
  `macos-latest`, `ubuntu-latest`, and `windows-latest` runners.
- [ ] Make the Windows Rendezvous runtime leg gating (not soft-fail) and retain
  test artifacts/logs that identify endpoint, permission, retry, and teardown
  failures without exposing the full user SID.
- [ ] Keep Sniff's existing three-OS matrix as the stable-user detector gate;
  add the new focused tests to that suite rather than creating a second Sniff
  workflow.
- [ ] **Parallelizable:** Run native macOS, Linux, and Windows validation legs
  concurrently once the migrated test suite is committed to the branch.

**Validation checkpoint**

- [ ] Confirm green native runtime results for macOS UDS, Linux UDS, and Windows
  named pipes; do not close the phase on cross-compilation-only evidence.
- [ ] Confirm the CI matrix compiles all affected targets and exercises both
  transport branches, including Windows concurrency and busy-retry tests.
- [ ] Confirm a second user derives a distinct endpoint without consulting
  username environment variables; where multi-principal execution is
  unavailable, require DACL/owner inspection plus a documented external
  multi-user verification record.

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
