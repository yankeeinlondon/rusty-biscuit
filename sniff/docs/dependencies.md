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

## Optional Features

- `network` (`reqwest` + `tokio` + `futures`, rustls) gates remote lookups; `remote`
  is an alias for it. `metrics` gates instrumentation. Both are off by default, so
  a consumer that only needs local detection — such as `rendezvous-core`, which
  depends on Sniff with `default-features = false` — pulls in neither an async
  runtime nor an HTTP stack.

## Path Presentation

- **`dunce` (CLI).** Detection reports worktree paths canonicalized, which on
  Windows carries the `\\?\` verbatim prefix. `dunce::simplified` removes it
  before the verbose worktree listing renders a display label and OSC8 href —
  the verbatim spelling is never user-facing and breaks `file://` targets.
