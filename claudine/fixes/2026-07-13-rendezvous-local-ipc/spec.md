---
status: ready for review
reviewed: true
created: 2026-07-13
area: claudine
packages:
    - sniff
    - rendezvous-core
    - rendezvous-client
    - rendezvous-daemon
    - claudine-cli
review_iterations: 1
rulings:
    - local transport remains Unix-domain sockets on Unix and named pipes on Windows
    - daemon ownership is per stable OS user account
    - Sniff owns stable current-user UID/SID discovery
---

# Cross-Platform, Per-User Rendezvous Local IPC

## Status

Ready for review. This fix replaces Rendezvous's path-shaped local-endpoint model
with an explicit cross-platform IPC abstraction, implements the missing Windows
named-pipe server, and makes endpoint, identity, and persistent data ownership
consistently per user.

The design intentionally keeps two communication planes distinct:

- **local control plane:** tonic gRPC over a Unix-domain stream socket on macOS,
  Linux, and WSL; tonic gRPC over a Windows named pipe on native Windows;
- **remote mesh plane:** authenticated QUIC between Rendezvous nodes.

Loopback TCP is not a fallback for the local plane. POSIX FIFOs are not treated
as equivalent to Windows named pipes.

## Problem

The current code describes a portable local control plane but only implements
half of it:

- `rendezvous-client` dispatches between Unix-domain sockets and Windows named
  pipes;
- `rendezvous-daemon` only exposes `spawn_uds_server` and imports
  `tokio::net::UnixListener` unconditionally;
- `rendezvous-core::socket` represents both a filesystem socket and a Windows
  named-pipe name as `PathBuf`;
- `default_socket_path` derives the Unix fallback from `$USER` and uses one
  machine-global Windows pipe name;
- `ensure_parent_dir` creates directories without verifying ownership, type,
  symlink status, or owner-only permissions;
- `spawn_uds_server` removes any pre-existing entry at the endpoint before it
  binds, without first proving that the entry is an owned socket;
- the default data root is `<tempdir>/rendezvous-data`, which does not express
  the same per-user ownership boundary as the daemon identity and endpoint;
- documentation calls the local IPC plane cross-platform even though the
  Windows daemon listener does not exist.

These are one contract problem, not an isolated missing Windows function.
Endpoint naming, endpoint authorization, data ownership, daemon identity,
client retry behavior, and server startup must agree on the same owner and
transport semantics.

## Blast Radius

The pre-spec GitNexus impact analysis rated `default_socket_path` **CRITICAL**:
four direct production callers, two affected execution flows (dashboard and
wrapped provider execution), and sixteen symbols within three hops. Direct
consumers include daemon startup, the test client, dashboard access, lifecycle
requeue, hook forwarding, and session-presence reporting.

The implementation must therefore be an explicit API migration with one
portable entry point. It must not be a Windows-only listener copied beside the
existing Unix boot path.

## Decisions

### D1 — Platform-native local transports

Use a Unix-domain **stream** socket on `cfg(unix)` and a Windows named pipe on
`cfg(windows)`.

This is not “named pipes everywhere.” A POSIX FIFO is one shared byte queue and
does not provide the listener/accept and independent duplex-connection model
needed by tonic's HTTP/2 transport. Windows named pipes do provide connection
instances and duplex byte streams.

Windows AF_UNIX is rejected for v1. Tokio's Unix listener/stream API remains
Unix-only, so using AF_UNIX on native Windows would require a lower-level
Winsock implementation without improving the same-user security model.

Loopback TCP is also rejected for v1. It would require application-layer
authentication, port allocation/discovery, IPv4/IPv6 binding policy, secret
rotation, and stale endpoint metadata merely to recover the local ownership
boundary supplied by UDS permissions and named-pipe DACLs. Cross-host traffic
already belongs to the authenticated QUIC plane.

### D2 — Exactly one daemon per stable OS user

The ownership invariant is:

> Exactly one Rendezvous daemon, identity, default data store, and default local
> IPC endpoint per stable OS user account.

The principal is:

- the effective numeric UID on macOS/Linux/WSL;
- the user SID from the current process token on native Windows.

It is not `$USER`, `%USERNAME%`, a display name, a home-directory basename, or
a Windows logon-session SID. Usernames are mutable and environment variables
are caller-controlled. A logon SID changes between sign-ins and would prevent
terminals belonging to the same account from sharing one daemon.

Multiple terminal windows and desktop/logon sessions for the same account
share the daemon. Different non-privileged users can run independent daemons
concurrently. WSL is a separate Linux user/node from the native Windows user;
the two can pair through the mesh when desired.

### D3 — Sniff owns stable account identity discovery

Stable host/user identity is a system-discovery concern. Add one narrow,
on-demand API to `sniff` rather than teaching Rendezvous to parse usernames,
shell out to `id` or PowerShell, or implement a private UID/SID detector.

Sniff owns discovery and the portable value representation. Rendezvous owns
authorization and endpoint policy. In particular, returning a SID does not make
Sniff responsible for constructing a Windows DACL, and returning a UID does not
make Sniff responsible for checking a socket directory.

### D4 — Endpoint types express transport semantics

Replace the path-shaped core API with a typed endpoint, conceptually:

```rust
pub enum LocalEndpoint {
    UnixSocket(PathBuf),
    WindowsNamedPipe(OsString),
}
```

A Windows pipe name is not a filesystem path. Callers must not infer a
transport from a string prefix, call filesystem operations on a pipe name, or
round-trip a pipe name through `PathBuf::to_string_lossy()`.

### D5 — One portable server boot path

Daemon initialization is platform-neutral and runs exactly once. Only listener
construction, connection acceptance, and endpoint cleanup are platform-specific:

```text
prepare_daemon(config)
        │
        ├── bind Unix listener
        └── bind Windows named-pipe acceptor
                    │
                    ▼
          serve_local_incoming(...)
```

`spawn_local_server` is the production entry point. `spawn_uds_server` may
remain as a Unix-only test seam, but Windows support must not mirror the whole
storage, projection, identity, register, QUIC, discovery, and worker setup.

### D6 — Endpoint and data overrides do not weaken ownership

Rename the public override vocabulary atomically:

- `RENDEZVOUS_SOCKET` → `RENDEZVOUS_ENDPOINT`;
- `--socket` → `--endpoint`;
- `default_socket_path` → `default_local_endpoint`;
- `ServerHandle::socket_path()` → `ServerHandle::local_endpoint()`.

There are no established external users, so indefinite compatibility aliases
would add ambiguity without protecting a real installed base.

An override changes location/name, not the authorization policy. A Unix
override still requires an owner-controlled parent and socket. A Windows
override still receives a same-user DACL and rejects remote clients. Test
fixtures that need isolated endpoints must create private temporary parents or
use test-only constructors; production must not silently relax checks because
an override was supplied.

## Sniff Prerequisite: Stable User Identity

### API shape

Add a focused OS-domain module, preferably `sniff::os::user`, and re-export its
public surface from `sniff::os`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StableUserId {
    UnixUid(u32),
    WindowsSid(String),
}

pub fn current_user_id() -> sniff::Result<StableUserId>;
```

The exact Rust names may change during planning, but these semantics are fixed:

- the value identifies the security principal under which the current process
  is operating;
- variants remain explicit instead of collapsing unlike identifiers into an
  untagged string;
- the Windows value uses canonical SID string form (`S-1-...`);
- the type provides a lossless, deterministic endpoint component or display
  projection; it does not hash by default;
- failures are typed `SniffError` variants and never fall back to a username,
  the text `default`, or a process-random value.

Do not add this value to `OsInfo`, `SniffResult`, or default `sniff --json`
output in this fix. A stable account identifier is sensitive and is only
needed on demand. It must not become ambient inventory or enter the existing
host-capability cache as an accidental side effect.

### Unix implementation

On `cfg(unix)`, obtain the process's effective UID using the existing `libc`
dependency (`geteuid`). The effective UID is the correct authorization identity
for filesystem and socket access. Convert it to the portable numeric variant
without executing `id`, consulting NSS for a username, or reading `$UID` or
`$USER`.

WSL compiles and runs through this same Unix branch. It returns `UnixUid`; it
must not inspect the native Windows token or attempt to correlate the WSL user
with a Windows account.

### Windows implementation

On `cfg(windows)`, query the current process token and read `TokenUser`:

1. open the current process token with query access;
2. size and read the `TOKEN_USER` buffer;
3. validate the returned SID;
4. convert it to canonical string SID form;
5. close/free every OS-owned handle or allocation on all paths.

Use Windows APIs directly through the existing target-specific `windows`
dependency. Add only the features required for process-token and SID access
(expected families include `Win32_Security`, `Win32_Security_Authorization`,
and `Win32_System_Threading`; planning must confirm the precise generated API
set for the pinned crate). Do not invoke PowerShell, `whoami`, WMI, the registry,
or `%USERNAME%`.

The account/user SID is required. Do not use a token's logon SID, integrity SID,
app-container SID, group SID, or display name.

### Sniff contract tests

Tests must cover:

- enum equality, display/endpoint projection, and serialization if serde is
  intentionally exposed;
- the Unix path returns the actual effective UID on macOS and Linux;
- environment changes to `USER`, `LOGNAME`, and `UID` do not affect the result;
- WSL is documented and compiled through the Unix implementation;
- the Windows path returns a valid canonical user SID from `TokenUser`;
- the Windows SID is stable across two calls in one process;
- OS errors remain errors rather than degrading to username text;
- macOS, Linux, and Windows compile and run the platform-appropriate test;
- no subprocess or network access is used.

This is a Tier-3, module-level detector. It is cheap, synchronous, uncached, and
not part of full host detection unless a later feature explicitly opts it in.

## Core Endpoint Model

Replace `rendezvous-core/src/socket.rs` with a transport-honest module such as
`rendezvous-core/src/local_endpoint.rs` containing:

- `LocalEndpoint`;
- `default_local_endpoint()`;
- override parsing and validation;
- the endpoint environment-variable constant;
- non-mutating platform-neutral validation;
- stable user identity integration through `sniff::os::current_user_id`.

Default endpoint derivation:

- **Linux/WSL with a valid `XDG_RUNTIME_DIR`:**
  `$XDG_RUNTIME_DIR/claudine/rendezvous/daemon.sock`, after verifying that the
  runtime directory is owned by the effective UID and is not writable by other
  users;
- **macOS, Linux fallback, and WSL fallback:** a private directory containing
  the numeric UID, e.g. `<tempdir>/claudine-rendezvous-<uid>/daemon.sock`;
- **Windows:** a SID-qualified pipe such as
  `\\.\pipe\claudine-rendezvous-<user-sid>`.

The default name may include a version only if a protocol incompatibility
requires side-by-side endpoints. Do not include a mutable username.

`LocalEndpoint` may expose explicit `as_unix_path()` /
`as_windows_pipe_name()` accessors behind the relevant `cfg`, but it must not
offer one ambiguous “path” accessor.

## Unix Server Contract

Filesystem mutation leaves `rendezvous-core` and belongs to the Unix daemon
transport module. Before binding, it must:

1. inspect each existing parent/endpoint with non-following metadata;
2. reject symlinks and non-directory parent components;
3. create the private Rendezvous runtime directory with mode `0700` without a
   world-accessible creation window;
4. verify that an existing directory is owned by the effective UID and has no
   group/other access;
5. inspect an existing endpoint with `symlink_metadata`;
6. remove it only when it is a Unix socket owned by the expected UID and no
   live daemon is accepting connections;
7. reject regular files, directories, symlinks, foreign-owned sockets, and
   active sockets;
8. bind the socket and force owner-only access (`0600`), independent of umask;
9. clean up on normal shutdown/drop only when the path still identifies the
   socket instance created by this server.

The administrator/root account is outside Rendezvous's threat boundary because
the operating system already grants it control. Other non-privileged users must
not be able to connect or replace the endpoint.

## Windows Server Contract

The Windows transport module builds a tonic-compatible incoming byte stream
from `tokio::net::windows::named_pipe`.

It must:

- use byte mode, not message mode, for HTTP/2;
- reject remote clients explicitly;
- apply a DACL granting the current user SID the required access and excluding
  other non-privileged users;
- use the first-instance protection on initial creation so a second daemon
  cannot silently take the configured name;
- create the next pipe instance before yielding the connected instance, so one
  client cannot leave the server with no acceptor;
- support multiple concurrent clients;
- close all instances on shutdown without filesystem cleanup;
- return typed errors that distinguish endpoint-in-use, access-denied, and
  listener failures.

Rendezvous may parse the canonical SID supplied by Sniff into the Windows
security structure needed for the DACL. It must not perform a second,
username-based identity lookup.

## Client Contract

`rendezvous-client::connect` accepts `LocalEndpoint` and dispatches without
lossy conversion:

- Unix connects with `UnixStream`;
- Windows opens the named-pipe string as an OS string;
- `ERROR_PIPE_BUSY` triggers bounded wait/retry with a clear deadline;
- endpoint-not-found, permission-denied, timeout, and incompatible endpoint
  errors remain distinguishable;
- callers never need a transport-specific branch.

Dashboard, lifecycle requeue, hook forwarding, session reporting, test client,
and daemon health probes all use this portable connector.

## Per-User Data and Identity Root

Move the default data root away from `<tempdir>/rendezvous-data` to the user's
platform-local data directory, conceptually:

```text
<local-data-dir>/claudine/rendezvous/
```

It contains the node identity key, persistence database/state, and other
daemon-owned durable files. Its access policy must match the endpoint owner:

- owner-only directory access on Unix;
- a current-user Windows DACL (administrators/system remain outside the threat
  boundary).

`--data-dir` remains an explicit operational/test override, but the daemon
validates that the resulting root does not broaden access to identity secrets.

Do not silently import the old temp-directory default. It was not an adequate
ownership boundary, and automatically trusting its contents could import an
attacker-controlled identity. If development data must be retained, provide a
documented one-time manual migration that first verifies ownership and file
types.

## Module Shape

Recommended implementation boundary:

```text
sniff/lib/src/os/user.rs
    StableUserId
    current_user_id

claudine/rendezvous/core/src/local_endpoint.rs
    LocalEndpoint
    default_local_endpoint
    override parsing / validation

claudine/rendezvous/daemon/src/local_transport/
    mod.rs       spawn_local_server + shared serve path
    unix.rs      private UDS preparation, bind, cleanup
    windows.rs   named-pipe DACL, instance acceptor, incoming stream

claudine/rendezvous/client/src/connector/
    mod.rs       portable connect
    unix.rs      UnixStream connector
    windows.rs   named-pipe connector + busy retry
```

Exact filenames can change during planning. The responsibility split cannot:
Sniff discovers the principal, core models the endpoint, the daemon enforces
server-side ownership, and the client connects.

## Documentation and Drift

Implementation must update these documents in the same change:

- add `claudine/docs/rendezvous/local-ipc.md` as the authoritative transport,
  ownership, override, and threat-boundary document;
- make `claudine/docs/rendezvous/design.md` summarize and link to it;
- correct `claudine/docs/rendezvous/current-state.md`, which currently claims a
  cross-platform server even though the daemon is Unix-only;
- reduce
  `features/2026-07-12-rendezvous-dashboard/windows-support-followup.md` to a
  superseded pointer instead of retaining its recommendation to mirror the
  large Unix startup function;
- update `claudine/rendezvous/README.md` diagrams and implementation status;
- update per-area dependency documentation when the Sniff dependency edge or
  Windows API feature set changes;
- update the Claudine and Sniff skills after the architecture and public API
  land.

Completed historical reviews/specifications remain historical evidence. Add a
superseded link where useful; do not rewrite their past findings as though the
new implementation existed at review time.

## Testing and Verification

Use nextest through the package-area `just` recipes. Do not consider Windows
support complete based on cross-compilation alone.

Required coverage:

1. `sniff::os::current_user_id` passes the platform contract tests on macOS,
   Linux, and Windows; WSL compiles/runs the Unix branch.
2. `default_local_endpoint` produces a UID-qualified UDS fallback on Unix and a
   SID-qualified named-pipe name on Windows.
3. Two OS users derive distinct default endpoints without consulting username
   environment variables.
4. Unix tests cover new private-directory creation, restrictive modes,
   pre-existing safe directories, symlink parents, regular-file endpoints,
   directories, foreign-owned entries where the test environment permits, a
   stale owned socket, and an active socket.
5. Unix server teardown cannot remove an endpoint replaced after bind.
6. A real Unix daemon/client gRPC round trip passes on macOS and Linux.
7. A real Windows daemon/client gRPC round trip passes on Windows.
8. Two Windows clients connect concurrently.
9. A busy Windows pipe is retried within a bounded deadline.
10. Remote named-pipe clients are rejected and a second daemon cannot seize the
    endpoint.
11. Same-user clients are accepted; other non-privileged users are denied where
    CI can provision a second principal.
12. Dashboard, lifecycle requeue, hook forwarding, session reporting, and test
    client round trips use `LocalEndpoint` without platform-specific call-site
    branches.
13. The daemon's portable initialization path is exercised by both transports;
    platform files contain listener/cleanup logic, not duplicated daemon boot.
14. Persistent identity/data defaults are per-user and private; the old shared
    temp default is neither selected nor silently imported.
15. `just test`, `just test-l2`, and `just lint` pass in affected package areas,
    plus the repository's macOS/Linux/Windows compile matrix.

## Acceptance Criteria

The fix is complete when:

- one typed endpoint API represents both local transports without pretending a
  Windows pipe is a filesystem path;
- one portable daemon entry point runs the shared initialization exactly once;
- native Windows has a real, runtime-tested named-pipe daemon;
- the Windows acceptor remains available while clients are connected and the
  client handles `ERROR_PIPE_BUSY` within a bounded deadline;
- Unix endpoint creation, stale cleanup, permissions, and teardown are
  owner/type checked;
- endpoint, data root, and node identity all resolve to the same stable OS-user
  owner;
- Sniff supplies the effective Unix UID or process-token user SID without
  environment variables, subprocesses, caching, or username fallbacks;
- WSL remains on the Linux/UDS/UID path;
- endpoint/data overrides preserve the same-user authorization boundary;
- cross-platform documentation reports implemented state honestly;
- macOS, Linux, and Windows runtime tests pass.

## Non-Goals

- Replacing remote QUIC with local IPC or exposing local gRPC across hosts.
- Supporting POSIX FIFOs as a connection transport.
- Adding Windows AF_UNIX support.
- Adding loopback TCP, bearer tokens, or local mTLS.
- Correlating WSL users with native Windows accounts.
- Creating a general user-account inventory or emitting account SIDs in default
  Sniff JSON.
- Treating root/administrator compromise as preventable by Rendezvous.
- Automatically trusting or importing the old temp-directory state.
