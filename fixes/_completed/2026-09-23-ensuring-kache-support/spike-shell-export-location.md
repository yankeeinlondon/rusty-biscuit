# Spike: where `KACHE_CACHE_DIR` is actually set

## Question

`KACHE_CACHE_DIR=/Volumes/coding/kache` is present in interactive zsh, yet no
literal assignment exists in `~/.zshrc`, `~/.zshenv`, `~/.zprofile`, or the
`~/.config/sh/` scripts. Locate the real setter (file, line, mechanism), confirm
by neutralization, and record which shell classes receive the variable.

## Answer

**`~/.env`, line 51** — `KACHE_CACHE_DIR=/Volumes/coding/kache` — reached via
`adaptive_setup()` in `~/.config/sh/adaptive.sh`, lines 330–336:

```sh
# source user's `.env` in home directory
# if it exists.
if file_exists "${HOME}/.env"; then
    set -a
    # shellcheck disable=SC1091
    source "${HOME}/.env"
    set +a
fi
```

The `set -a` / `set +a` pair (allexport) turns **every** assignment in
`~/.env` — none of which carry an `export` keyword — into an exported
environment variable. `~/.env` is a 54-line key/value file also holding API
keys, webhook URLs, and `GITNEXUS_*` tuning; only line 51 concerns kache.

Sourcing chain: `~/.zshrc:9` sources `~/.config/sh/adaptive.sh` →
`adaptive.sh:583` calls `adaptive_setup` → `adaptive.sh:334` sources `~/.env`
with allexport active.

## How it was traced

1. Reproduced in a clean environment: `env -i HOME=$HOME
   PATH=/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin TERM=xterm zsh -c
   'source ~/.zshrc ...; printf "[%s]\n" "$KACHE_CACHE_DIR"'` → the value.
   Sourcing only `~/.config/sh/adaptive.sh` the same way also produced it,
   isolating the chain to that file.
2. `zsh -x` over `adaptive.sh` with the trace captured (`2>/tmp/trace.txt`);
   `grep -n KACHE_CACHE /tmp/trace.txt` yielded one executed line:
   `+/Users/ken/.env:51> KACHE_CACHE_DIR=/Volumes/coding/kache`.

## Neutralization confirmation

Copied `~/.env` minus the `KACHE_CACHE_DIR=` line into a scratch
`/tmp/spike-home/.env` and sourced the real `adaptive.sh` with
`HOME=/tmp/spike-home` (no real file edited):

```text
neutralized=[] exit-ok
```

The variable comes back empty and `adaptive_setup` completes normally. Also
verified the value is truly exported (visible to `printenv`), not a mere
shell variable.

## Shell classes that receive it today

Only **interactive zsh**. `~/.zshenv` (all zsh invocations) and `~/.zprofile`
contain no reference to `adaptive.sh` or `~/.env`, and empirical checks in the
clean environment returned empty for both a login non-interactive shell
(`zsh -l -c`) and a plain non-interactive shell (`zsh -c`). Bash profiles
(`~/.bashrc`, `~/.bash_profile`, `~/.profile`) do not source `~/.env` either;
`adaptive.sh:331–334` is the file's only consumer. Editors, launchd jobs, and
agent sessions therefore never see it — matching the spec's evidence.

## Removal guidance

Delete **only line 51** (`KACHE_CACHE_DIR=/Volumes/coding/kache`) from
`~/.env`. Blast radius is that single variable: the `set -a` mechanism must
survive because it exports every other line of `~/.env` (credentials and
`GITNEXUS_WORKER_POOL_SIZE`/`GITNEXUS_WORKER_SUB_BATCH_MAX_BYTES` at lines
53–54, among others). Do not remove or reorder the `set -a` block in
`adaptive.sh`. After removal, the spec's step 8 verification (daemon honors
`ignore_env` while the plist still carries the variable) proceeds unchanged,
followed by plist regeneration.
