---
sub-spec: true
depends-on:
  - ../05-git-observation/spec.md
  - ../../spec.md
phase: 6
status: complete
date: 2026-07-17
---

# Phase 6 — Reuse remote/network inputs and bound subprocesses

Implement umbrella requirements **R11** (reuse remote and network inputs) and **R12** (remove default
latency cliffs and bound subprocesses). The governing rule for this phase: **resolve a shared input
once per report, and never let an unbounded child process hold a detection hostage.**

As in Phases 1–5, acceptance evidence is a counter delta or a mock-server request count, not a
wall-clock comparison. Phase 6 changes only `remote.*`, `network.*`, and `process.*` counters; every
`filesystem.*` and `git.*` counter must be untouched — that is this phase's drift bracket.

## Upstream impact analysis

Required by the plan's execution rule before editing an existing symbol.

| Symbol | Risk | Direct callers | Disposition |
|---|---|---|---|
| `RemoteRepoProvider::fetch_report` | LOW | Default trait method; `GitRemote` inherits it (`remote/mod.rs:275-406` forwards the other 12 methods but not this one) | Lane A keeps the signature and adds a **defaulted** `snapshot` hook. No downstream provider is required to adopt it (R11.4). |
| `RemoteRepoProvider::list_documents` / `detect_cicd` | LOW | `fetch_report` only, plus tests | Signatures preserved exactly. Lane A rewrites internals to consume a shared snapshot when one is supplied. |
| `os::time::run_command_with_timeout` | LOW (`pub(crate)`, 3 call sites, all in `os/time.rs`) | `detect_ntp_status_*` | Lane D deletes it in favor of the shared helper. |
| `programs::schema::run_command_with_timeout` | LOW (private, 1 call site) | `version_from_path` | Lane D deletes it in favor of the shared helper. |
| `programs::host_capability::run_probe` | LOW (private) | verification probes | Lane D rewrites its body onto the shared helper; signature preserved. |
| `DetectionPlan::default` | **MEDIUM** — behavior change, but intentional and specified | `DetectionPlan::new()`, `detect()`, and every `..Default::default()` site | Lane C. R12.2 mandates it. `OsRequest::full()` is explicitly unchanged, so any caller that wants NTP keeps it by asking. |
| `services::systemd::list_systemd_services` / `runit::list_runit_services` | LOW (`pub(crate)`, 1 call site each via `ServiceManager::services_detailed`) | `services/mod.rs:394` | Lane D batches their internals. Return type `Vec<Service>` and the silent-empty failure contract preserved. |

No HIGH or CRITICAL-risk edit is made, so this phase does not stop for review.

## Lane D — shared subprocess execution

### The defect being fixed

Every pre-phase subprocess site was one of two shapes, and **both are wrong**:

1. **`Command::output()` with no deadline** — `services/systemd.rs`, `launchd.rs`, `openrc.rs`,
   `runit.rs`, `hardware/storage.rs` (`diskutil`), `os/locale.rs` (Windows PowerShell). `output()`
   drains both pipes correctly but blocks **forever** if the child hangs. A wedged `systemctl` wedges
   the whole detection.
2. **`try_wait()` polling with an undrained `Stdio::piped()` stdout** — `os/time.rs`,
   `programs/schema.rs`, `programs/host_capability.rs`. These have a deadline but **deadlock the
   child**: nothing reads the pipe while the loop polls, so a child emitting more than one pipe
   buffer (64 KiB on Linux, 16 KiB typical on macOS) blocks in `write()`, never exits, and is killed
   at the deadline. The output is lost and the timeout is misattributed to a slow child. R12.6 names
   this directly.

`process::run_with_timeout` fixes both: it drains stdout and stderr on **dedicated threads** while the
parent polls the deadline, kills on expiry, and always `wait()`s to reap.

### Named timeout defaults (R12.10)

Timeouts are a **policy**, not an incidental constant. They live in one place, `process::timeouts`:

| Constant | Value | Applies to |
|---|---:|---|
| `SERVICE_COMMAND` | 3s | `systemctl`, `launchctl`, `rc-status`, `sv` |
| `WINDOWS_LOCALE` | 3s | `powershell (Get-Culture).Name` |
| `DISKUTIL` | 5s | macOS `diskutil info` |
| `HOST_CAPABILITY` | 2s | install-plan verification probes (**retained**) |
| `PROGRAM_SCHEMA` | 3s | `--version` probes (**retained**) |
| `NTP` | 3s | `timedatectl` / `sntp` / `w32tm` (**retained**) |

Tests inject short `Duration`s rather than sleeping for a production deadline (R12.9).

### Batching (R12.4)

- **systemd** was `1 + N_running` spawns: one `list-units`, then one `systemctl show --property=MainPID`
  per *running* service. Now `1 + ceil(N_running / CHUNK)`: one `systemctl show` per chunk with
  `--property=Id --property=MainPID`, correlating by the `Id=` field.
- **runit** was `N` spawns — one `sv status` per service directory, unconditionally. Now
  `ceil(N / CHUNK)`: `sv status` accepts many service names and emits one `run:`/`down:` line each.

`CHUNK` (128) exists only for command-line length limits (R12.4), not as a policy bound.

**Partial results are preserved** (R12.11): a timed-out enrichment chunk leaves those services with
`pid: None` (systemd) or `(false, None)` (runit) — exactly the pre-phase degradation for a failed
probe — while every successfully parsed chunk is retained. A timed-out **primary listing** returns the
backend's existing empty `Vec`, unchanged.

## Lane C — default NTP policy

`DetectionPlan::default()` becomes "all domains with safe defaults", not "every domain at `full()`":
`os` is now `OsRequest::full().include_ntp_status(false)`. `OsRequest::full()` itself is **unchanged**
(R12.1), so `detect_with_plan(DetectionPlan::new().os(OsRequest::full()))` still probes NTP.

This removes an implicit network probe (up to 3s, and on Linux historically much worse) from the
Tier-1 `detect()` path. Every other `full()` field — package managers, locale, timezone — is retained;
only the live NTP probe is gated.

## Lane B — WAN IP

The pre-phase code violated R11.7 in three ways: **one** default endpoint (so "retry" had nothing to
fall back to), a `reqwest::blocking::Client` **rebuilt per attempt**, and a TTL cache that
unconditionally wrote negative results.

- Two default HTTPS endpoints: `https://api64.ipify.org`, `https://icanhazip.com`.
- One client built once per `detect()` and reused across attempts.
- Sequential, first-strictly-parsed-IP wins (R11.7: never disclose the caller's address to a second
  endpoint after a successful first attempt).
- Identical connect/request deadlines on every endpoint; response bodies never enter errors or
  counters (R11.8).
- TTL and `force_refresh` semantics unchanged (R11.9).

## Lane A — remote snapshot

`RemoteRepoSnapshot` is provider-private evidence resolved **once** per `fetch_report`: repository
metadata, default branch, and one recursive root tree. `fetch_report` builds it and hands it to
`list_documents_with` / `detect_cicd_with`, both **defaulted** trait hooks that fall back to the
existing `list_documents` / `detect_cicd` (R11.4 — no downstream provider is forced to adopt them).

Pre-phase request counts per `fetch_report`, and the bound after:

| Provider | metadata before | metadata after | tree before | tree after |
|---|---:|---:|---:|---:|
| GitHub | 3 (2 if workflow runs non-empty) | 1 | 2 (1 if runs non-empty) | 1 |
| GitLab | 0 (no `GetProject`; tree *is* the probe) | 0 | 3 | 1 |
| Gitea | 3 | 1 | 2 | 1 |
| Bitbucket | 3 | 1 | 2 root listings + ≤2 subdir | 1 root + ≤2 subdir |

### Truncation (R11.5)

`truncated` is deserialized by the GitHub and Gitea tree types and was **read nowhere** — a >100k-entry
or >7 MB repository silently reported partial documents and CI/CD as complete. Bitbucket's
`response.next` was likewise ignored (`bitbucket.rs:264`, "For MVP, just return the first page").

The snapshot now carries `truncated: bool`. Continuation requests are counted under a **separate**
operation slug (`tree_continuation`) so a report distinguishes a correctness-preserving continuation
from the duplicate root-tree request this phase removes.

## Acceptance

```
cd sniff && just test && just lint && just build && just doctest
```

## Results

All four lanes landed. `just lint` clean with zero warnings; `just build` and `just doctest` clean.
`just test` passes except the pre-existing `detect_area_errors_when_not_in_repo` timeout, which is a
known baseline failure verified against clean HEAD since Phase 4 — Phase 6 does not touch
`repo/area.rs`.

### `just test` did not run these tests before this phase

`sniff-lib`'s `default = []`, and the `sniff/justfile` `test` recipe passed no `--features`. The
feature-gated tests were therefore **never executed by the package's own test recipe** — including all
65 pre-existing `remote_providers` tests. They only ran under an explicit
`cargo nextest run --features remote`, which nothing in the repo did automatically.

That made the acceptance gate for this phase's Lanes A and B blind. The recipe now passes
`--features remote` (which enables `network`) for the sniff lib. Every one of those tests reaches a
local `MockServer`, never the internet.

**The gap was larger than the remote tests alone.** `just test` went from **1,414 to 1,604 tests** —
190 tests that had not been running. Every one of them passes.

### Three defects fixed that the performance work exposed

This phase was framed as removing duplicate work. Three of the things it removed were not merely
wasteful — they were wrong:

1. **The `try_wait()` deadlock.** `os/time.rs`, `programs/schema.rs`, and `host_capability.rs` polled
   for exit over an **undrained** piped stdout. Any child emitting more than one pipe buffer (64 KiB
   on Linux) blocked in `write()`, never exited, and was killed at its deadline — output lost, and the
   timeout misattributed to a slow child. `output_larger_than_a_pipe_buffer_does_not_deadlock` (1 MiB)
   is the regression test.
2. **`truncated` read nowhere.** GitHub and Gitea both deserialize it; `grep` found zero readers. A
   repository past ~100k entries or 7 MB reported "no docs, no CI" with full confidence. Bitbucket's
   `response.next` was ignored with an explicit `// For MVP` comment, which is the same defect.
3. **The single WAN endpoint.** `DEFAULT_WAN_IP_ENDPOINTS` had one entry, so the retry ladder had
   nothing to fall back to and the "fallback" was decorative.

Two more sites had **no deadline at all** and were outside the survey's framing: `host_capability`'s
`id -Gn` and `sudo -n true`.

### Post-review subprocess boundary

Review cycle 3 found one remaining bypass and a descendant-pipe gap in the shared helper. The
Windows BurntToast availability probe now uses the named three-second policy through
`process::run_for_stdout`. On Unix, children run in a dedicated process group; on Windows, they enter
a kill-on-close Job Object before resuming. Timeout and direct-child exit cleanup terminate the
whole group/job before joining pipe drains, so a descendant retaining stdout or stderr cannot extend
the deadline. Portable Level-1 fixtures spawn such a descendant and require prompt cleanup; the
Windows GNU all-target check compiles the Job Object path.

### Percent-encoding constrains the continuation design

An early `CONTINUATION_PREFIXES` used `.github/workflows`. A probe against a mock server showed the
schematic client percent-encodes the whole `branch:prefix` tree_sha into **one path segment**, so that
prefix went out as `main%3A.github%2Fworkflows`. A `%2F` inside a path segment is routinely rejected or
normalized by routers and proxies, so the continuation would have been unreliable against the real API
while passing happily against wiremock.

Prefixes are now single-component (`.github`, not `.github/workflows`) and the recursive response
supplies `workflows/ci.yml` beneath. The remaining `%3A` for `:` is standard path-segment encoding that
GitHub decodes.

**Caveat, stated plainly:** the truncation continuation is verified against wiremock only. Neither the
`%3A` decoding nor the `branch:prefix` subtree addressing has been exercised against live GitHub or
Gitea in this session, and truncation needs a >100k-entry repository to reproduce naturally. This is
the one part of the phase whose real-API behavior rests on documented API semantics rather than on an
observation.

### Duplication removed as a consequence of R11.2

Sharing the document projection across providers meant sharing its categorizer. All four providers
carried **byte-identical** copies of `categorize_document` + `is_documentation_file` and four
byte-identical copies of their five-test suite. Verified identical before deletion; one copy each now
lives in `snapshot.rs`.

### Deferred, with reasons

- **Criterion benchmarks → Phase 8**, following the Phase 3 and Phase 5 precedent. This host's timings
  are untrustworthy (Phase 3 recorded +330% for a byte-identical case at load 57-87/16 cores). Every
  claim here is a mock-server request count or a counter, which is what this feature judges on.
- **Timeout-during-listing / timeout-after-partial-enrichment shim tests.** Both paths return through
  the same arms the failure tests already exercise, and forcing a real 3s deadline in a unit test would
  violate the plan's own injected-deadline rule (R12.9).
- **`ServiceState::Initializing` is unreachable.** `services/mod.rs:399` filters with
  `state.matches(Some(s.running))`, and `matches` only returns true for `Initializing` when `running`
  is `None` — which no backend ever produces. Pre-existing, unrelated to R12, and out of scope.
