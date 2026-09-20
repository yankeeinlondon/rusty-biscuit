# Sniff Dependencies

Notable dependency edges and the constraints behind them. This is not a full
inventory — `sniff/lib/Cargo.toml` is the source of truth. It records the edges
whose *reason* is not obvious from the manifest.

## Stable User Identity (`os::user`)

`sniff::os::current_user_id()` returns the security principal the current process
runs as. Its consumer is the Rendezvous local control plane, which uses it to
qualify the per-user endpoint and to check ownership of the daemon's private
directories. See
[`claudine/docs/rendezvous/local-ipc.md`](../../claudine/docs/rendezvous/local-ipc.md).

- **Unix — `libc`.** `geteuid()` only. The *effective* UID is the identity the
  kernel actually authorizes filesystem and socket access with. Deliberately no
  subprocess (`id`), no NSS username lookup, and no `$UID`/`$USER`/`$LOGNAME`:
  environment variables are caller-controlled and usernames are mutable, so
  neither is a security principal. WSL compiles and runs this branch.
- **Windows — `windows` 0.62.** Reads `TokenUser` from the current process
  token. No PowerShell, `whoami`, WMI, registry, or `%USERNAME%`.

### Windows feature set

`windows` is a target-specific dependency (`cfg(target_os = "windows")`). Its
features are enumerated rather than broad, because the crate's generated API
surface is large and each family costs build time. Those the identity detector
added, and what each supplies:

| Feature | Supplies |
|---|---|
| `Win32_Security` | `TOKEN_USER`, `TokenUser`, `IsValidSid` |
| `Win32_Security_Authorization` | `ConvertSidToStringSidW` — canonical `S-1-...` form |
| `Win32_System_Threading` | `GetCurrentProcess`, `OpenProcessToken` |

`Win32_Foundation`, `Win32_System_Services`, and `Win32_System_Environment`
predate this work and serve other detectors.

Note that the Rendezvous daemon declares its **own** `windows` features for the
DACL work (`Win32_Storage_FileSystem`, and `Win32_Security_Authorization` for
`GetNamedSecurityInfoW`). Sniff's boundary stops at discovery: returning a SID
does not make Sniff responsible for building a security descriptor. See
[`claudine/docs/dependencies.md`](../../claudine/docs/dependencies.md).

## Network Primitives (`network::{address, gateway, icmp}`)

Scoped interface addresses, default gateways, and ICMP echo probes, consumed by
Darkmatter's `ctx.tailnet`/`ctx.gateway*` values and `ping()` functions (see
`darkmatter/features/2026-09-09-more-context/spec.md`).

- **`ipnet` 2.** CIDR values for address-bit membership
  (`ScopedIpAddr::is_within`, the `100.64.0.0/10` CGNAT predicate). Darkmatter's
  `ipv4()`/`ipv6()` filters use the same crate, so both sides agree on prefix
  parsing. It was already in the lockfile through `reqwest`.
- **`socket2` 0.6 (`cfg(unix)` only).** Unprivileged `SOCK_DGRAM` ICMP and
  ICMPv6 sockets on macOS, Linux, and WSL2 — no raw socket, `setuid` binary, or
  `ping` subprocess. Linux admits them only for groups inside
  `net.ipv4.ping_group_range`; outside it the probe returns
  `IcmpError::NotPermitted` rather than degrading to "no reply".
- **No routing crate.** Gateways are parsed from `/proc/net/route` and
  `/proc/net/ipv6_route` (Linux), the `PF_ROUTE` `NET_RT_DUMP` sysctl through
  `libc` (macOS), and `route print` through the supervised subprocess boundary
  (Windows). Each parser is a pure function so fixtures run on every host.

### Windows feature set (network)

| Feature | Supplies |
|---|---|
| `Win32_NetworkManagement_IpHelper` | `IcmpCreateFile`, `Icmp6CreateFile`, `IcmpSendEcho2`, `Icmp6SendEcho2`, `Icmp6ParseReplies`, reply structures and `IP_*` status codes |
| `Win32_Networking_WinSock` | `SOCKADDR_IN6` for the IPv6 source and scoped destination |
| `Win32_System_IO` | `PIO_APC_ROUTINE`, required by the `Icmp*SendEcho2` signatures |

## Git Access

- **`gix` (pinned `=0.84.0`).** All production git access is pure-Rust gix. The
  exact-version pin is deliberate — gix's API moves between minor releases.
  Feature selection is explicit (`default-features = false`) and each entry backs
  a specific probe; `merge` in particular exists only for the worktree
  merge-conflict check, which merges unmerged branches in-memory and never writes
  to the repository.
- **`git2` (dev-dependency only).** Retained *only* for fixture operations gix
  has no write API for — worktree-snapshot commits, checkout, linked-worktree
  creation — plus a libgit2 differential-parity oracle. It must never become a
  production dependency; anything gix can do uses gix.

## Executable Lookup

- **`which` (pinned to major `8`, `default-features = false`, `real-sys`).**
  Pinned to the same major as `claudine` and `claudine-cli` so provider and tool
  discovery behave consistently across the workspace.

## Portable Path Text

- **`biscuit-file` (default features off).** The CLI uses the workspace path-text
  policy when filesystem paths cross into terminal text, JSON snapshots, and
  file hyperlinks. This removes safely reducible Windows verbatim-disk prefixes
  without changing the native `PathBuf` values used for filesystem access.

## Optional Features

- `network` (`reqwest` + `tokio` + `futures`, rustls) gates remote lookups; `remote`
  is an alias for it. `metrics` gates instrumentation. Both are off by default, so
  a consumer that only needs local detection — such as `rendezvous-core`, which
  depends on Sniff with `default-features = false` — pulls in neither an async
  runtime nor an HTTP stack.
- The CLI's `test-fixtures` feature carries `biscuit-test-harness` into the
  non-test build and enables two helper binaries, exclusively for L2 styling
  tests. Their test targets declare `required-features`, so ordinary local L1
  does not build empty L2 binaries. CI enables the feature for its reusable
  all-tier archive; `just test-l2` enables it explicitly.
- `biscuit-test-harness` is additionally an unconditional `[dev-dependencies]`
  entry of both packages, independent of that feature: L1 tests call its
  `manifest_dir!()` so fixture lookups resolve against the run-time manifest
  directory, which is what makes a nextest archive built elsewhere find its
  fixtures in the executing checkout.
