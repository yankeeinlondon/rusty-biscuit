# Configuration and best practices

## Where things live

| Item | Path |
| --- | --- |
| Config file | `~/.config/kache/config.toml` (verified on macOS; `kache config` opens it) |
| Store (macOS) | `~/Library/Caches/kache` + `index.db` |
| Store (Linux) | `~/.cache/kache` |
| Store (Windows) | under `%LOCALAPPDATA%` — confirm with `kache doctor` |
| Cargo wiring | `~/.cargo/config.toml` → `[build] rustc-wrapper = "kache"` |
| Daemon service | launchd agent (macOS) / systemd user unit (Linux) |

`kache doctor` prints the resolved paths — use it rather than guessing, especially if `~/.config`
or `~/.cargo` is a synced or shared directory across hosts.

## Minimal config

```toml
[cache]
local_max_size = "100GiB"
```

The default store cap is **50 GiB** (matching the CI action's `max-size` default). Most tuning is
this one key plus a gc cadence.

## Sizing the store — the rule that matters

**A store that sits above its cap is worse than a smaller store.** When the store exceeds
`local_max_size`, LRU evicts — and it can evict *fresh* entries before they ever score a hit, so you
pay to populate entries that are discarded before returning value. A real example, from a machine
that hit this after a cache-key version bump:

> "at the 50 GiB default the store sat at 117% and LRU evicted fresh entries before they could
> score a hit. 100 GiB lets [new] entries warm up; the dead [old] blobs age out naturally as the
> coldest, never-hit entries."

Practical guidance:

- Give headroom above the working set, especially after a **kache upgrade that bumps the key
  version** — every entry is invalidated at once and the store must repopulate while still holding
  the dead blobs.
- On a capped or shared filesystem, size against the *volume*, not habit. A 100 GiB store on a
  200 GB filesystem that also holds `target/` is a collision waiting to happen; 30 GiB is saner.
- On hardlink filesystems, remember the cap bounds *unique* store bytes while live `target/` links
  keep evicted blobs alive — see [platforms.md](platforms.md).
- Watch actual usage with `kache stats` and `kache list --sort size`.

## Garbage collection

```bash
kache gc                    # LRU eviction down to local_max_size
kache gc --max-age 7d       # also drop anything older than 7 days
kache purge                 # wipe everything
kache purge --crate-name serde
```

Age-based gc is the right periodic job on a workstation; LRU alone suffices where the cap is
comfortably above the working set. Schedule `gc --max-age` weekly rather than relying solely on
cap-triggered eviction, so cold blobs leave before they force pressure.

## Keep `target/` small — for speed, not disk

The least obvious operating requirement. kache does per-crate file operations against the build
tree, and those degrade badly on enormous trees. Measured on a real workstation:

> "a 957k-file / 1.4TB `target/deps` directory [made] kache's per-crate file ops crawl... ~18s/crate
> keying. On a clean target, keying is ~30-170ms."

That is a ~100× swing in keying cost, and it looks exactly like "kache is slow" or "kache is
broken". It isn't — it's the build tree.

So target hygiene stays necessary even with kache, and this is the strongest argument for keeping a
periodic sweep or clean job. Options, in rough order of preference:

- **`cargo sweep --time <N>`** on a timer — prunes stale artifacts while keeping the tree live.
  Still valuable *specifically* to protect keying speed, even though kache's own `gc` handles the
  store's retention.
- **`kache clean`** — removes whole `target/` directories under the cwd (`--dry-run` first). Blunt,
  but with a warm store the rebuild is mostly link-restores rather than compiles, which is what
  makes it acceptable here when it wouldn't be otherwise.

## The daemon

```bash
kache daemon start | stop | restart | install | uninstall | log | run
```

There is **no `daemon status`** subcommand (as of 0.7.0) — use `kache doctor`, which reports
reachability, or `kache stats`, whose last lines show `Daemon: offline|online` and whether a remote
is configured. On macOS, `launchctl list | grep kunobi` showing `-` in the PID column means the
agent is loaded but not running.

The daemon handles remote sync and prefetch warming. `kache doctor` flags two common faults:

- `daemon not reachable` — installed as a service but not running; `kache daemon restart`
- `stale locks — N legacy lock file(s) from a previous daemon` — also cleared by `daemon restart`

A dead daemon degrades quietly: local caching keeps working, remote sync silently doesn't. Check
`doctor` periodically rather than assuming sync is happening.

Under heavy concurrent load — several agents building simultaneously — the daemon can become a
bottleneck. A gating wrapper in front of `kache` is one mitigation pattern if you hit it.

## Environment variables

| Variable | Effect |
| --- | --- |
| `RUSTC_WRAPPER=kache` | Enable kache for this shell/step without editing cargo config |
| `KACHE_CACHE_EXECUTABLES=1` | Also cache binary crates, dylibs and proc-macros (off by default) |
| `KACHE_PLANNER_ENDPOINT` / `KACHE_PLANNER_TOKEN` | Point the daemon at a remote planner (preview) |

`KACHE_CACHE_EXECUTABLES` is off by default because those outputs depend on linker behaviour and
platform specifics like macOS code signing. Enable it only if you've verified the results, and
expect it to grow the store noticeably.

## Cache keys — what invalidates entries

The blake3 key covers: rustc version (with commit hash), target triple, crate name/types/edition,
codegen flags, feature flags, source files, environment variables, and dependency hashes. It
**excludes** absolute paths and machine identity, which is what makes remote sharing portable.

Consequences worth planning around:

- **Changing a profile setting invalidates entries.** Do profile changes (e.g. `debug =
  "line-tables-only"`) *before* warming a cache, or you populate the store twice.
- **A rustc upgrade invalidates everything.** Expect a repopulation period and give the store
  headroom through it.
- **A kache upgrade may bump the internal key version**, with the same effect.

Use `kache why-miss <crate>` when something you expected to hit didn't — it names the differing key
component instead of leaving you to guess.

## Observability

```bash
kache stats --since 24h                          # hits, misses, bytes saved
kache monitor                                    # live TUI during a build
kache list --sort size|hits|age                  # what's actually in the store
kache report --format markdown --since 7d        # shareable summary
kache report --format perfetto -o trace.json     # build trace for profiling
```

`report` also emits `json`, `chrome-trace`, `github` and `text`, with `--top N` and `--root <path>`
to scope to one build tree.
