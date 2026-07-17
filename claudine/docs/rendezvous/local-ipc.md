---
title: Rendezvous — Local IPC
date: 2026-07-16
status: authoritative
fix: 2026-07-13-rendezvous-local-ipc
---

# Rendezvous — Local IPC

This is the authoritative document for the Rendezvous **local control plane**:
how a client on the same host reaches the daemon, who is allowed to, and what
happens when they are not.

It does not cover the **remote mesh plane** (authenticated QUIC between paired
nodes). The two are deliberately separate: local IPC is authorized by the
operating system's ownership model, remote sync by per-node Ed25519 signatures.
Local gRPC is never exposed across hosts, and QUIC is never a local fallback.

## 1. The Contract in One Paragraph

Exactly one Rendezvous daemon, one node identity, one durable data root, and one
local endpoint **per stable OS user account**. The endpoint is a Unix-domain
stream socket on macOS, Linux, and WSL, and a Windows named pipe on native
Windows. The account is the effective UID on Unix and the process token's
account SID on Windows — never `$USER`, `%USERNAME%`, a display name, or a logon
SID. Every terminal window and desktop session belonging to one account shares
that account's daemon; different non-privileged users get independent daemons
that cannot reach each other's.

## 2. Transport Selection

`LocalEndpoint` (in `rendezvous-core::local_endpoint`) is the typed contract:

```rust
pub enum LocalEndpoint {
    UnixSocket(PathBuf),
    WindowsNamedPipe(OsString),
}
```

The transport is carried by the variant, not inferred from a string. There is
deliberately **no common `path()` accessor** — a Windows pipe name is not a
filesystem path, is never stat-ed or unlinked, and must not round-trip through
`PathBuf::to_string_lossy()`. Callers use `as_unix_path()` or
`as_windows_pipe_name()`, both of which return `Option`, or dispatch on the
variant. `Display` exists for humans only and is documented as lossy.

`is_native()` / `ensure_native()` answer whether the current target can bind or
connect the endpoint; a named pipe on Unix or a socket on Windows is an explicit
`IncompatibleTransport` error rather than an attempt that fails obscurely later.

### Why not something simpler

| Rejected | Why |
|---|---|
| POSIX FIFOs "everywhere" | A FIFO is one shared byte queue with no listener/accept model and no independent duplex connections. Tonic's HTTP/2 transport needs both. Windows named pipes provide them; FIFOs do not. |
| Windows AF_UNIX | Tokio's `UnixListener`/`UnixStream` are Unix-only, so this would mean a bespoke Winsock listener — for no improvement to the same-user security model a pipe DACL already gives. |
| Loopback TCP | Would require application-layer auth, port allocation and discovery, bind policy, and secret rotation just to re-acquire the ownership boundary UDS modes and pipe DACLs supply for free. Cross-host traffic is already QUIC's job. |

## 3. Endpoint Resolution

`default_local_endpoint()` resolves in this order, first match wins:

1. **`RENDEZVOUS_ENDPOINT`**, parsed and validated for the current target.
2. **The per-user default**, qualified by `sniff::os::current_user_id()`:
   - **Linux/WSL** with a usable `$XDG_RUNTIME_DIR`:
     `$XDG_RUNTIME_DIR/claudine/rendezvous/daemon.sock`
   - **Other Unix**, and Unix without a usable runtime directory:
     `<tempdir>/claudine-rendezvous-uid-<uid>/daemon.sock`
   - **Windows**: `\\.\pipe\claudine-rendezvous-sid-<sid>`

The qualifier comes from `StableUserId::endpoint_component()`, which is lossless
and variant-tagged (`uid-501`, `sid-S-1-5-21-…`) — not a hash, so an operator can
recognize their own endpoint, and not ambiguous, so a UID can never be read as a
SID.

Two properties are load-bearing:

- **Resolution never mutates the filesystem.** It inspects type, symlink status,
  ownership, and mode to decide whether `$XDG_RUNTIME_DIR` is usable, but
  creating, chmod-ing, and removing belong to the daemon — the only component
  that binds. `rendezvous-core` contains no listener and no directory creation.
- **A bad `$XDG_RUNTIME_DIR` is *unavailable*, not an error.** It is an
  environment quirk, so resolution falls back to the UID-qualified temp
  directory. A bad *override*, by contrast, is a caller mistake and is rejected.

### Failure is typed

`default_local_endpoint()` returns `LocalEndpointError` and has **no fallback to
a username, the literal `default`, or a process-random name**. A process that
cannot learn who it is must not go on to create per-user private state — a
username-derived endpoint would be attacker-influenceable, and a random one
would silently split a user's single daemon into many.

## 4. Overrides

| Surface | Value |
|---|---|
| `RENDEZVOUS_ENDPOINT` | OS-native: a socket path on Unix, `\\.\pipe\<name>` on Windows |
| `rendezvous-daemon --endpoint` | Same, also readable from the env var |
| `rendezvous-test-client --endpoint` | Same |
| `--data-dir` / `RENDEZVOUS_DATA_DIR` | Durable root override |

**An override changes location, not policy.** A Unix override still requires an
owner-only parent and produces an owner-only socket; a Windows override still
receives a same-user DACL and still rejects remote clients; a `--data-dir`
override is still validated as private to the current user before the identity
seed or any database is opened. Nothing relaxes because a value was supplied
explicitly.

Override validation rejects, with distinct errors: an empty value
(`EmptyOverride` — a set-but-empty variable is a configuration mistake, not a
request for the default); a relative Unix path (the daemon and its clients must
resolve it identically regardless of working directory); a Unix path with no file
name; a pipe name on Unix or a bare path on Windows (`IncompatibleTransport`); a
Windows name with an empty tail, an embedded backslash, or one past the
256-character Win32 limit.

Tests that need isolated endpoints create private temporary parents or use the
`test-support` feature's constructors. Production ownership checks are never
weakened for tests.

### Legacy names are gone

`RENDEZVOUS_SOCKET`, `--socket`, `default_socket_path`, `ServerHandle::socket_path()`,
and the `socket` module no longer exist, with no compatibility aliases. There is
no installed base to protect, and keeping path-shaped names beside a typed
endpoint would preserve exactly the ambiguity the type was introduced to remove.

## 5. Stable User Identity (Sniff)

`sniff::os::current_user_id() -> Result<StableUserId>` is the single source of
the principal:

```rust
pub enum StableUserId {
    UnixUid(u32),
    WindowsSid(String),  // canonical S-1-... form
}
```

- **Unix**: `libc::geteuid()`. The *effective* UID is the identity the kernel
  actually authorizes filesystem and socket access with. No `id` subprocess, no
  NSS username lookup, no `$UID`/`$USER`/`$LOGNAME`.
- **Windows**: the current process token's `TokenUser`, validated and converted
  to canonical string SID form, with every handle and allocation released on
  every path. No PowerShell, `whoami`, WMI, registry, or `%USERNAME%`.

The division of labor: **Sniff discovers the principal; Rendezvous authorizes
with it.** Returning a SID does not make Sniff responsible for building a DACL,
and returning a UID does not make it responsible for checking a socket directory.

This is an on-demand, uncached, Tier-3 detector. It is deliberately **not** in
`OsInfo`, `SniffResult`, default `sniff --json`, or any host-capability cache: a
stable account identifier is sensitive and must not become ambient inventory as a
side effect.

### WSL

WSL compiles and runs the **Unix branch**, returns `UnixUid`, and gets a UDS
endpoint. It never inspects the native Windows token and never correlates the WSL
user with a Windows account. A WSL user is a separate Linux user and a separate
Rendezvous node from the native Windows user on the same machine; if the two
should share data, they pair through the mesh like any other pair of nodes.

Note that WSL interop really does propagate `USERNAME` from the Windows side into
the Linux environment — which is precisely why endpoint derivation reads the UID
and not a name. Regression tests set the WSL markers plus `USERNAME` and assert
that neither moves resolution off the UID/UDS path.

## 6. Unix Server Contract

Filesystem mutation lives in `rendezvous-daemon`'s `local_transport/unix.rs` and
`private_dir.rs`. Before binding:

1. Walk the endpoint's parent components with **non-following** metadata; reject
   symlinks and non-directory components.
2. Create the private runtime directory with mode `0700` **via
   `DirBuilder::mode()`**, so `mkdir(2)` itself applies the mode and there is no
   instant in which the directory is more permissive than owner-only.
3. Verify an already-existing directory is owned by the effective UID with no
   group or other access.
4. Inspect any existing endpoint with `symlink_metadata`.
5. Bind, then force the socket to `0600` regardless of umask.

**The security boundary is the directory, not the socket file.** `bind(2)` creates
a socket with `0777 & ~umask` and offers no way to ask for a narrower mode at
creation, so the mode is corrected immediately after. That correction is not a
race: the parent is already proven owner-only, so no other user can name the
socket during the window whatever the umask was.

Ancestors *above* the private directory may be shared and may be symlinks —
`/tmp` is world-writable and macOS reaches every temp directory through the
`/var` symlink, so rejecting those would reject the platforms this has to run on.
The requirement binds at the private directory itself and everything the daemon
creates below it.

### What may be removed from an endpoint

Exactly one occupant is reclaimable: **a socket this user owns that nobody is
listening on** — what an unclean shutdown leaves behind. Everything else is
somebody's data or somebody's service:

| Found at the endpoint | Outcome |
|---|---|
| Nothing | Bind |
| Owned socket, no listener | Remove it, bind |
| Owned socket, live listener | `EndpointInUse` — two daemons would fight over one database |
| Socket owned by another uid | `EndpointOccupied` |
| Regular file, directory, symlink, FIFO | `EndpointOccupied` |
| Unreadable | `AccessDenied` |

Liveness is decided by an actual **connect attempt**, because a socket file
outlives the process that bound it, so its mere presence proves nothing — and a
pidfile would only move the same staleness problem one file over.
`ECONNREFUSED`/`NotFound` means stale. A full backlog, a timeout, or an
unexplained error all resolve to *live*: refusing to boot is recoverable,
unlinking a live daemon's endpoint is not.

### Teardown cannot delete a successor

On shutdown or drop, the socket is unlinked **only if fresh non-following
metadata still matches the `(dev, ino, uid)` captured at bind**. A path is not an
identity: between bind and shutdown the entry can be unlinked and re-created by a
restarted daemon that now legitimately owns the name. If the endpoint was
replaced, teardown logs and leaves it alone — the postcondition ("this daemon's
socket is not at that path") already holds, and unlinking would take down whoever
replaced us. Cleanup failures are reported, never resolved by deleting foreign
data.

## 7. Windows Server Contract

`local_transport/windows.rs` builds a tonic-compatible incoming stream from
`tokio::net::windows::named_pipe`:

- **Byte mode**, not message mode. gRPC is a byte stream; message mode would
  frame it, wrongly.
- **`reject_remote_clients(true)`.** A pipe reachable over SMB is a pipe a remote
  host can reach. The local control plane is local.
- **`first_pipe_instance(true)` on the initial instance only.** This is the
  exclusion: a second daemon fails loudly instead of quietly adding an instance
  to the first daemon's pipe and racing it for connections. Successors must pass
  `false` — the flag is a claim on the *name*, so a successor asserting it would
  collide with its own predecessor.
- **A current-user DACL**, built from SDDL (`O:<sid>D:P(A;;GA;;;<sid>)`): owner
  is this user, the DACL is *protected* (`P`) so no inheritable ACE from a
  container can widen it, and the single ACE grants this user generic-all. An ACL
  with no matching ACE denies by default, so nothing else is needed to exclude
  other users. It is RAII-owned (`SecurityDescriptor`) and `LocalFree`-d exactly
  once on every path.
- **The successor instance is created *before* the connected one is yielded.** A
  pipe name with no unconnected instance rejects clients with
  `ERROR_FILE_NOT_FOUND` rather than queueing them, so creating the successor
  afterwards would make every accept a window where a client racing in is told
  the daemon does not exist.

Multiple concurrent clients are supported. Shutdown aborts the accept loop, which
drops the last unconnected instance; the name disappears with the last handle.
**There is no cleanup step and no path to remove** — a named pipe is not a
filesystem entry.

Rendezvous parses the canonical SID Sniff supplies into the Windows security
structure it needs. It never performs a second, username-based identity lookup.

## 8. Durable Data Root

The default is `<local-data-dir>/claudine/rendezvous` (`dirs::data_local_dir()`),
holding the node-identity seed (`node.key`), the redb source of truth
(`session.redb`), and the DuckDB projection (`projection.duckdb`).

It is validated by the **same** `ensure_private_dir` helper as the Unix runtime
directory — one helper for the two justified consumers rather than two
owner/type/symlink/mode checks drifting apart — and, on Windows, by the **same**
`current_user_descriptor()` the pipe endpoint uses, so the pipe's DACL and the
data root's cannot come to disagree about who this user is. On Windows the check
that matters for an existing directory is **ownership**, not the DACL as it
currently reads: an owner can rewrite a DACL at will, so a directory owned by
another account cannot be trusted however permissive or restrictive it looks
right now. `CreateDirectoryW` applies the DACL at creation, so — as on Unix —
there is no permissive window and no inherited ACE ever in force.

Endpoint, data root, and node identity therefore all resolve to the same stable
OS user by construction.

### The legacy temp root

`<tempdir>/rendezvous-data` is **not consulted, migrated, or read**, and tests
assert that it is neither selected nor opened. This is not tidiness: a shared temp
directory is not an ownership boundary, so a node identity found there could have
been planted by any local user. Adopting it automatically would let an attacker
choose the daemon's signing key.

If development data must be retained, migrate it deliberately: stop the daemon,
verify by inspection that the old directory and its files are owned by you and are
regular files (not symlinks), copy them into the new root, and confirm the new
root is `0700`/current-user-owned. Rendezvous will not do this for you, because
"verify the thing is not attacker-controlled" is not a step a program can perform
on the attacker's behalf.

## 9. Client Contract

`rendezvous_client::connect(&LocalEndpoint)` is the one entry point. It calls
`ensure_native()`, then dispatches: `tokio::net::UnixStream` on Unix, a named-pipe
open from the `OsStr` on Windows. **No production call site needs a `cfg` branch**
— dashboard access, lifecycle requeue, hook forwarding, session-presence
reporting, the test client, and daemon health probes all go through it unbranched.

Tonic's placeholder URI (`http://[::]:0`) is HTTP/2 plumbing only, never endpoint
identity: the supplied connector produces the byte stream, so that authority is
never resolved or dialed.

### Errors stay distinguishable

Tonic collapses every connector failure into one opaque
`tonic::transport::Error`, which would erase the distinction between "no daemon
is listening", "you may not open this", and "the pipe is saturated" — the three
cases callers actually branch on. The connector therefore classifies the OS error
*inside* the connector closure and carries the verdict out through an `ErrorSlot`,
rather than reading tonic's message back as text:

| `ConnectError` | Meaning |
|---|---|
| `IncompatibleEndpoint` | Wrong transport for this target, or unusable as one |
| `NotFound` | Nothing listening. Covers "no endpoint" *and* "refused" — a stale socket file gives the latter, and both mean "start the daemon" |
| `PermissionDenied` | Exists, but this process may not open it (Unix mode, Windows DACL) |
| `BusyTimeout` | Every instance stayed busy for the whole budget; the daemon is alive, just saturated |
| `Io` | Other OS failure, original preserved as source |
| `Transport` | The stream opened but tonic's HTTP/2 setup failed |

The original OS error is preserved in the source chain in every case.

### Busy retry (Windows)

`ERROR_PIPE_BUSY` (231) means the pipe exists but every instance is serving
someone. The open retries against **one bounded budget** — 5s total, fixed 50ms
backoff — then fails `BusyTimeout`. The backoff is fixed rather than exponential
because a busy pipe clears as soon as the daemon loops back to accept, so backing
off exponentially would add latency without easing contention.

The budget is spent in whole backoff units rather than measured against a clock:
every wait is one `sleep(backoff)`, so counting them bounds the wall clock just as
tightly as reading it would, and it keeps the loop deterministic under an injected
sleep — which is what makes deadline exhaustion testable without a fake clock or a
real named pipe. The retry machinery stays compiled on every target so its tests
run on macOS and Linux too, rather than only on the one host that can least afford
to be the first to find a bug in it.

## 10. Server Errors

`ServerError` (from `rendezvous-daemon::server`) categorizes startup failure:

| Variant | Meaning |
|---|---|
| `Endpoint` | Malformed or target-incompatible endpoint (wraps `LocalEndpointError`) |
| `EndpointInUse` | A live daemon holds it — Unix connect probe or Windows `first_pipe_instance` refusal |
| `EndpointOccupied` | Wrong entry type, or another user's, and the daemon will not remove it |
| `AccessDenied` | The OS refused this process access |
| `Ownership` | A directory violates the private-directory contract (wraps `PrivateDirError`) |
| `Listener` | Listener creation failed for another reason |
| `Cleanup` | The endpoint could not be released after shutdown; the daemon is down, a stale endpoint may remain |

Existing storage and network error sources are preserved unchanged.

## 11. Server Boot Shape

One portable entry point, `spawn_local_server(LocalEndpoint, DaemonConfig)`:

```text
prepare_daemon(config)      ← redb, projection, batcher, identity, session log,
        │                     registers, refresher, QUIC, discovery, peer
        │                     workers, RendezvousService — constructed once
        ├── unix::serve      ← private dir, bind, chmod, unlink-on-teardown
        └── windows::serve   ← DACL, first instance, accept loop, handle close
                    │
                    ▼
          serve_local_incoming(...)
```

`prepare_daemon` is transport-neutral and has no idea what will carry its bytes.
That split is the point: a second transport adds a listener, an acceptor, and a
cleanup rule and inherits everything else — it *cannot* grow a parallel copy of
the storage and network stack, because there is only one place to construct them.
A test-only initialization counter asserts the shared boot runs exactly once per
daemon, on both transports.

`spawn_uds_server` survives only as a Unix test seam and is absent from every
production call site.

## 12. Threat Boundary

**In scope.** Other non-privileged local users must not be able to connect to the
endpoint, replace it, read the node-identity seed, or write the daemon's durable
state. That is enforced by UDS directory/socket modes and ownership on Unix, and
by the pipe DACL plus data-root ownership on Windows.

**Out of scope.**

- **root / Administrators / SYSTEM.** The OS already grants them control: either
  can take ownership of any object, rewrite a DACL, and debug the process. An ACE
  denying them would be security theater, not a boundary.
- **Remote attackers on the local plane.** They have no access path to reach it —
  `reject_remote_clients(true)` on Windows, and a UDS has no network presence at
  all. Cross-host traffic is QUIC's, authenticated by signed envelopes.
- **Correlating WSL users with Windows accounts.** Explicitly not attempted.
- **The old temp-directory state.** Never trusted or imported; see §8.

## 13. Testing

Run from the package areas:

```bash
cd sniff && just test && just lint
cd claudine/rendezvous && just check && just build && just test && just lint
cd claudine && just test && just lint
```

`test-l2` is a no-op for the Rendezvous crates — there are no real-terminal
tests.

**Cross-compilation is not evidence of Windows support.** The named-pipe
acceptance criteria are runtime-gating and are proved on a native
`windows-latest` runner by `.github/workflows/rendezvous-tests.yml`, which
compile-checks `--all-targets` across `sniff`, all three Rendezvous crates, and
`claudine-cli`, then runs the suites natively on `macos-latest`,
`ubuntu-latest`, and `windows-latest` with no `continue-on-error` on any leg.
Logs are redacted so a user SID's trailing RIDs (`S-1-5-21-<redacted>`) never
reach an artifact while endpoint, permission, retry, and teardown text stays
readable.

Two verification gaps are known and tracked in the fix's plan rather than papered
over:

- **A real WSL smoke run.** Marker coverage exists (WSL markers plus `USERNAME`
  assert `UnixUid` + a UID-qualified socket), but GitHub-hosted runners do not
  offer WSL, so an actual run needs a self-hosted or manually-attested host.
- **Multi-principal denial.** Planting a foreign-owned entry, or connecting as a
  second user, needs a second UID/account that neither an unprivileged dev host
  nor a GitHub runner can provision. The documented fallback — DACL and owner
  inspection, plus username-independence tests — is covered; an external
  multi-user verification record is still owed.

## 14. Where Things Live

```text
sniff/lib/src/os/user.rs                          StableUserId, current_user_id
claudine/rendezvous/core/src/local_endpoint.rs    LocalEndpoint, default_local_endpoint,
                                                  override parsing/validation
claudine/rendezvous/daemon/src/private_dir.rs     ensure_private_dir, default_data_dir,
                                                  expected_uid / current_user_descriptor
claudine/rendezvous/daemon/src/server.rs          DaemonConfig, prepare_daemon, ServerError,
                                                  ServerHandle, serve_local_incoming
claudine/rendezvous/daemon/src/local_transport/
    mod.rs                                        spawn_local_server
    unix.rs                                       private UDS prep, bind, cleanup
    windows.rs                                    DACL, instance acceptor, incoming stream
claudine/rendezvous/client/src/connector/
    mod.rs                                        portable connect, ConnectError
    unix.rs                                       UnixStream connector
    windows.rs                                    named-pipe connector + busy retry
```

Sniff discovers the principal, core models the endpoint, the daemon enforces
server-side ownership, and the client connects. That split is the contract; the
filenames are not.

## See Also

- [`design.md`](design.md) — overall Rendezvous architecture
- [`current-state.md`](current-state.md) — what is built today
- [`../../fixes/2026-07-13-rendezvous-local-ipc/spec.md`](../../fixes/2026-07-13-rendezvous-local-ipc/spec.md) — the ratified specification behind this document
