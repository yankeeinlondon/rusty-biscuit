# Spike: `kache doctor --json` field shapes on 0.23.1 and 0.26.3

Date: 2026-09-23 · macOS (dev Mac) · host kache 0.23.1 untouched; 0.26.3
fetched pristine via `cargo binstall` into a temp `CARGO_HOME`, run against
scratch `KACHE_CONFIG`/`KACHE_CACHE_DIR`/`KACHE_RUNTIME_DIR` locations and a
scratch foreground daemon (`kache daemon run` + kill, launchd never touched).

## Questions

Phase 2–4 recipes parse `kache doctor --json` for the default store path, the
daemon-reported version, the daemon running state, and the runtime/socket
location; the restart trigger compares `kache --version` against the
doctor-reported daemon version. The plan's Necessary Rule 5 pins the field
names on the version actually installed, using this spike.

## Observed shapes

### `kache doctor --json` — identical shape on 0.23.1 and 0.26.3

Top level (keyed by name; parse by label, never by array position):

| field | 0.23.1 | 0.26.3 | meaning |
|---|---|---|---|
| `schema_version` | `1` | `1` | |
| `command` | `"doctor"` | `"doctor"` | |
| `version` | `"0.23.1"` | `"0.26.3"` | the running binary's own version, **bare** (no `v`) |
| `rustc` | `"rustc 1.98.1 (…)"` | same | |
| `issues` | `0` | `1` | count of failed **non-optional** checks (the 0.26.3 scratch run failed one non-optional check plus three optional ones and reported `1`) |
| `next` | *absent* | present | 0.26.3 addition: array of `{argv, why}` fix suggestions |

`checks[]` entries are `{label, pass, optional, detail, fix}`; the labels the
recipes read:

| label | detail when healthy | detail when not | both versions |
|---|---|---|---|
| `Cache dir` | the resolved store path (e.g. `/Volumes/coding/kache` on this host) | — | same |
| `Cache FS` | `apfs (local)` | — | same |
| `Daemon version` | `v0.23.1 (epoch 1789651776)` / `v0.26.3 (epoch 1789939897)` — **`v`-prefixed, epoch suffix** | `daemon not reachable`, `pass: false`, `fix: "start daemon with \`kache daemon start\` or \`kache daemon install\`"` | same |
| `Daemon service` | launchd plist path | — | same |
| `Service exe` | *(0.26.3 only)* plist exe vs current exe comparison | `plist points to … but current exe is …` | new in 0.26.3 |
| `Daemon processes` | *(0.23.1)* `1 live daemon process(es) (pid 1622), socket unreachable` when a process exists but the socket is not | not observed on 0.26.3 (no stray process existed to report) | emit-conditional; do not rely on its presence |

The runtime/socket location is **not** in `doctor --json` — see
`kache daemon --json` below.

### `kache daemon --json` — daemon state and socket (0.26.3)

`kache daemon` (bare or `status`) accepts `--json`; far better suited to the
"ensure running" and restart-trigger steps than parsing doctor labels:

```json
{
  "schema_version": 1,
  "command": "daemon-status",
  "version": "0.26.3",
  "epoch": 1789939897,
  "daemon_running": false,
  "service_installed": true,
  "service_path": "…/LaunchAgents/ninja.kunobi.kache.plist",
  "socket": "…/runtime/daemon.sock",
  "daemon_version": null,
  "daemon_epoch": null,
  "daemon_config_path": null,
  "service_executable_mismatch": true,
  "next": [ … ]
}
```

When the scratch daemon was running: `daemon_running: true`,
`daemon_version: "0.26.3"` (**bare, no `v`**), `daemon_epoch`, `socket`
(full path), and `daemon_config_path` (the config file the daemon read).
`service_executable_mismatch` is true whenever the plist names a different
binary than the one running — the doctor `Service exe` check in JSON form.

### `kache --version` vs the daemon-version fields

- `kache --version` → `kache 0.23.1` / `kache 0.26.3` — `kache ` prefix, bare
  version (the justfile's `cut -d' ' -f2` keeps working).
- doctor `.version` → bare `0.26.3`, exactly comparable.
- doctor `Daemon version` detail → `v0.26.3 (epoch 1789939897)` — strip the
  leading `v` and cut at the first ` (` to compare.
- `daemon --json` `.daemon_version` → bare `0.26.3`, directly comparable to
  `.version` / `kache --version`.

**Restart-trigger recommendation (feeds Phase 2):** prefer
`kache daemon --json` — `daemon_running` for "ensure running", bare
`daemon_version` vs doctor `.version` for the binary-changed trigger, and
`socket` for reporting the runtime location. Keep doctor's `Cache dir` label
as the store-path source (it is the only place the resolved store appears).

## 0.23.1 → 0.26.3 differences that matter to the recipes

1. New top-level `next` array — ignore it; nothing parses unknown keys.
2. New `Service exe` check (plist exe vs current exe) — read by label only
   when wanted; absent on 0.23.1.
3. `Daemon processes` is emit-conditional — absence means nothing.
4. Runtime dir contents grew (`daemon.control.v2.sock`, `*.bind.lock`,
   `daemon.run.lock`, `events.jsonl` beside `daemon.sock` on 0.26.3) — only
   relevant when a failure drill wants to block the socket path; the primary
   socket name is still `daemon.sock`.
5. `kache daemon` subcommand surface on both lines: `status`, `run`, `start`,
   `stop`, `restart`, `install`, `uninstall`, `log` — the lifecycle verbs the
   plan names (`kache daemon install`, restart) exist unchanged.

No field the plan names changed name or format between the two versions; the
recipes can parse by the labels above against either.

## Safety confirmation

Real daemon (pid 1622, `ninja.kunobi.kache`) and `kache monitor` (70818)
were the only kache processes before and after; every scratch daemon was
killed and its temp dir removed (one `tmutil addexclusion` child of a killed
scratch daemon exited on its own within seconds); the real
`~/.config/kache/config.toml` and `/Volumes/coding/kache` were never written
by the spike (scratch `KACHE_CONFIG`/`KACHE_CACHE_DIR`/`KACHE_RUNTIME_DIR`
on every pristine-binary invocation).
