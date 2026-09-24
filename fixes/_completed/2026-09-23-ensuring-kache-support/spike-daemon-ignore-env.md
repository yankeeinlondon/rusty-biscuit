# Spike: does the kache daemon honor `ignore_env` from the config file?

Date: 2026-09-23 · kache 0.23.1 · macOS (dev Mac) · scratch environment only

## Question

The spec's design writes `[cache] local_store` plus `[cache] ignore_env = true`
into `~/.config/kache/config.toml` so the file wins over `KACHE_CACHE_DIR`.
CLI behavior was already verified; the daemon's was not. If the daemon ignores
`ignore_env`, plist regeneration must be a hard prerequisite of activation,
not cleanup hygiene.

## Method

Scratch dir via `mktemp -d /tmp/kache-spikeA.XXXXXX` containing `config.toml`,
empty `file-store/`, `env-store/`, `runtime/`. Foreground daemon backgrounded
from bash (never `daemon start/install`, so launchd was never touched); the
scratch `KACHE_RUNTIME_DIR` kept the socket away from the real daemon. After
~5 s, observe which store materialized daemon artifacts (`index.db`,
`daemon.sock`, `store/`, `gc_stats.json`); `kache doctor --json` for the
reported view; kill the scratch PID only. Control run repeats without
`ignore_env = true`.

```bash
S=$(mktemp -d /tmp/kache-spikeA.XXXXXX); mkdir -p "$S"/{file-store,env-store,runtime}
printf '[cache]\nlocal_store = "%s/file-store"\nignore_env = true\n' "$S" > "$S/config.toml"
KACHE_CONFIG=$S/config.toml KACHE_RUNTIME_DIR=$S/runtime \
  KACHE_CACHE_DIR=$S/env-store KACHE_LOG=kache=info kache daemon run > $S/daemon.log 2>&1 &
```

## Observations

| Run | config | `KACHE_CACHE_DIR` env | store that materialized | daemon socket |
|---|---|---|---|---|
| experimental | `local_store` + `ignore_env = true` | set → `env-store` | **file-store** | `<file-store>/daemon.sock` |
| control | `local_store` only | set → `env-store` | **env-store** | `$runtime/daemon.sock` |

Key evidence, experimental run (daemon process's own log):

```text
WARN kache::config: [cache] ignore_env = true: ignoring set env override(s) ["KACHE_CACHE_DIR", "KACHE_RUNTIME_DIR"] in favor of the config file
INFO kache::daemon: daemon listening on /tmp/kache-spikeA.XXXXXX/file-store/daemon.sock
```

`file-store/` then held `index.db`, `daemon.sock`, `daemon.state.json`,
`gc_stats.json`, `store/`; `env-store/` stayed empty apart from `probes/`.
The control run (same config, `ignore_env` removed) put its own `index.db`,
`gc_stats.json`, and `store/` in `env-store/` — the observation discriminates,
so the spike is conclusive. `kache doctor --json` agreed: `Cache dir ->
.../file-store` (experimental) vs `.../env-store` (control).

Nuances observed (secondary, worth knowing at implementation time): `ignore_env` gates **all** env overrides, including `KACHE_RUNTIME_DIR` — hence
the socket moved into the store dir in the experimental run. And one
version-probe json (`probes/<hash>.json`) still landed under the raw-env dir
(`env-store/`) during that run: the probe-cache path reads raw env, so a stale
`KACHE_CACHE_DIR` can keep dripping probe files into the old location even
after `ignore_env` is written — one more reason the env export should still be
removed.

## Verdict

**Yes — the daemon honors `ignore_env`.** With `KACHE_CACHE_DIR` present in
its environment, the daemon put its store, socket, and index in the
config-declared location.

## Design consequence

Plist regeneration is **cleanup hygiene, not a hard prerequisite** of
activation: a daemon still carrying `KACHE_CACHE_DIR` in a stale plist
resolves the ratified store anyway. Regeneration stays in the plan (it removes
the misleading env and stops raw-env probe drips), but activation does not
gate on it.

## Safety confirmation

Before and after: `launchctl list | grep -i kunobi` → `1622 0 ninja.kunobi.kache`
(unchanged); `pgrep -fl kache` → real daemon 1622 and `kache monitor` 70818
only, no strays; real store `index.db` mtime predates the spike; real
`~/.config/kache/config.toml` untouched (no `local_store`/`ignore_env` keys); scratch dir removed.
