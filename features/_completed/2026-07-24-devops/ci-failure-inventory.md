---
title: CI failure inventory
measured: 2026-07-27
source_run: PR #7, run 30231664586 (full scope, 34 failing jobs)
status: snapshot
---

# CI failure inventory — 2026-07-27

Every failing job in a full-scope CI run, with its **actual root cause** rather
than its job name. Taken from PR #7's run; the same set appears on every
full-scope run, and 31 of the 34 also appear on PR #6, which changed nothing but
one line of `areas.json`.

**34 failing jobs resolve to about 14 distinct causes.** Most are one-line lint
fixes amplified by `RUSTFLAGS: -D warnings`, not deep defects. Three are CI
infrastructure defects belonging to this feature.

## A. CI infrastructure — this feature's own defects (3 causes, 6 jobs)

These are not product debt. They were introduced by the DevOps work and are
already on `main`.

### A1. `NEXTEST_PROFILE: ci` breaks nested Cargo workspaces — 4 jobs, 1 latent

```
error: profile `ci` not found (known profiles: default, default-miri)
```

`schematic/`, `unchained-ai/`, and `tree-hugger/` each declare their own
`[workspace]` in `Cargo.toml`. Nextest resolves its config at
`<workspace-root>/.config/nextest.toml`, so when CI runs `cd <area> && just test`
it looks in the *nested* root, which has no config — and never sees the repo-root
`[profile.ci]`.

Introduced by Task 2.3, which set `NEXTEST_PROFILE: ci` in `_area-ci.yml`'s `env`.
Reproduces locally: `cd unchained-ai && NEXTEST_PROFILE=ci cargo nextest list -p model_id`.

- Failing: `schematic / test` (ubuntu, windows), `unchained-ai / test` (ubuntu, windows)
- **Latent:** `tree-hugger / test` is currently *skipped* because tree-hugger's
  lint fails first (D4 staging). It will start failing the moment that lint is
  fixed.

`--config-file` has no environment-variable form, so this cannot be fixed by
adding another env var. Options, cheapest first:

1. Have the shared `_test`/`_sanity` recipes in `just/devops.just` pass
   `--config-file <repo-root>/.config/nextest.toml`. One edit, one authority, no
   duplicated config.
2. Give each nested workspace its own `.config/nextest.toml`. Three copies to
   keep in sync — the exact drift this feature exists to remove. Not recommended.
3. Flatten the nested workspaces. Correct long-term, far out of scope here.

### A2. `homelab / lint` needs the `sniff` CLI — 1 job

```
bash: line 1: sniff: command not found
error: backtick failed with exit code 1
```

Homelab's `justfile` evaluates a backtick that shells out to `sniff`. The
`_area-ci.yml` lint job installs `just` but not the repo's own CLIs. Either
install `sniff` for that job or make the recipe degrade when it is absent.

### A3. Affected coverage cannot build the workspace — 1 job

```
error: failed to run custom build command for `gdk-sys v0.18.2`
```

`cargo llvm-cov --workspace` compiles `visualizer` (Tauri), which needs GTK
development headers. Nothing declares them, and `visualizer` is **exempt** from
area ownership, so there is no `areas.json` record to declare them in — the
`native` policy has no home for a package that no area owns. Either give exempt
packages a native declaration path or exclude them from workspace coverage.

## B. Product debt — lint under `-D warnings` (9 causes, ~15 jobs)

All mechanical. Each belongs to its owning area.

| Cause | Failing jobs |
|---|---|
| **`biscuit-terminal`**: `parse_cpr_response`, `parse_csi_14t_response`, `OSC_QUERY_ATTEMPT_TARGET` dead on Windows | `biscuit-icon / test (win)`, `canary / playa / test (win)`, `darkmatter / test (win 1–4/4)` — **6 jobs from one defect** |
| `sniff`: unused import `super::*`, unused variable `helpers` | `sniff / test` (ubuntu, windows) |
| `biscuit-speaks`: unused variable `stack` | `biscuit-speaks / lint` |
| `research`: clippy — block rewritable with `?` | `research / lint` |
| `worktree-cli`: clippy — block rewritable with `?` | `worktree / lint` |
| `messenger`: clippy — unneeded pattern element vs `..` | `claudine / lint` |
| `biscuit-file`: dead `SpanObject`, unused `serialize` | `tree-hugger / lint` |
| `model-citizen`: unused variables `src`, `dest` | `model-citizen / test (win)` |
| `biscuit-tui-cli`: unused imports `std::io::Write`, `Path` | `biscuit-tui / test (win)` |

The `biscuit-terminal` row is the single highest-leverage fix in this table:
three dead-code items behind `cfg` gates account for six red jobs, and it also
blocked the (now-removed) Windows Ctrl+C workflow.

## C. Product debt — real test failures (6 causes, 9 jobs)

These need actual investigation by their owning areas.

| Job | Signal |
|---|---|
| `darkmatter / test (ubuntu 1–4/4)` | `1573 tests run: 1567 passed, 6 timed out` — 6 timeouts, 4 jobs |
| `sniff / test (macos)` | test run failed (compiles; tests fail) |
| `queue / test (win)` | test run failed |
| `renderable / test (win)` | test run failed |
| `biscuit-file / test (win)` | test run failed |
| `Claudine generator and signals drift` | test run failed |
| `rendezvous / native (windows)` | Rendezvous suite failed — red on `main` since 2026-07-23 |

## D. Environment / third-party (2 jobs)

| Job | Cause |
|---|---|
| `messenger-desktop / WSL2 ubuntu` | `Vampire/setup-wsl@v4`: `wsl.exe` exit 100 installing Ubuntu-24.04. Red on `main` since 2026-06-17. Not our code. |
| `biscuit-terminal / lint (ubuntu)` | **Not determined** — GitHub refused the log download (`ServerBusy: Egress is over the account limit`). Almost certainly the same dead-code items as the Windows failures, but that is inference, not evidence. |

## What this changes

- The apparent explosion of red is ~9 small lint fixes plus ~6 genuine test
  problems, not 34 independent disasters.
- Fixing `biscuit-terminal` alone clears 6 jobs.
- Three failures (A1–A3) belong to this feature and should be fixed here, not
  filed against the areas.
- `-D warnings` is doing exactly what it was set up to do; it just means dead
  code behind a `cfg` gate fails a whole platform's job.
