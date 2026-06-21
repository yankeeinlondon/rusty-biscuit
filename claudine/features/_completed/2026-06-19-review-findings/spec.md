---
created: "2026-06-19T00:00:00"
title: "Comprehensive Review Remediation — claudine Package Area"
source_review: "claudine/reviews/2026-06-19-comprehensive/review.md"
status: "ready for planning and implementation"
reviewed: true
review_iterations: 3
---

# Specification — Comprehensive Review Remediation

This specification translates every finding in
[`claudine/reviews/2026-06-19-comprehensive/review.md`](../../reviews/2026-06-19-comprehensive/review.md)
into a concrete problem statement and proposed solution. Findings are grouped by
remediation theme and ordered by priority, with severity preserved from the
review. Each item lists its source location(s), the defect, the proposed fix,
and the tests that must accompany it.

**Status:** Reviewed and ready for planning and implementation. The inline review
filled the missing Priority 2 protect section, chose the protect posture, and
kept the accepted review findings scoped as remediation work.

Reviewed crates: `claudine` (lib), `claudine-cli`, `claudine-contract`,
`rendezvous-core`, `rendezvous-client`, `rendezvous-daemon`.

## Scope and Non-Goals

- **In scope:** all 2 High panic fixes, the `protect` posture/extraction/path
  hardening, the rendezvous daemon concurrency hardening, the wrapper
  termination/`set_var` hardening, the lifecycle ternary guard hole,
  contract-crate polish, and the cross-cutting hygiene items (JSON-walk
  de-cloning, secret detection, `which` unification, silent-swallow logging).
- **Rejected findings (do NOT re-raise):** shell injection in the bash action
  runner; "Qwen untagged enum drops arrays"; `LazyLock` regex `.expect()` panic;
  git double-strip / frontmatter slice-order panic / fs-probe collision in the
  harness validators. These were investigated and verified as non-issues. The
  git porcelain rename/quoted-path handling is a real but Low detection-accuracy
  gap, retained below under the Low section.
- **Non-goals:** no behavioral redesign beyond what each finding requires; the
  rendezvous daemon stays a LAN POC (the permissive QUIC verifier is acceptable
  for that posture and only flagged as a forward-looking gate).

## Acceptance Criteria (whole effort)

1. Both confirmed UTF-8 panics are fixed and covered by fail-first regression
   tests that panic before the fix.
2. `protect`'s security posture is explicitly decided, documented at module
   level, and the chosen posture (best-effort vs. real boundary) is locked by
   tests.
3. The rendezvous daemon no longer holds a `parking_lot` mutex across an fsync
   and no longer runs synchronous redb/DuckDB I/O directly on tokio worker
   threads; the staging→commit and sealer-counter races are closed.
4. The wrapper cannot hang indefinitely on a wedged child, and the
   per-iteration `env::set_var("PWD")` race is eliminated.
5. The lifecycle undefined-variable guard descends ternary conditions.
6. All "silent swallow" sites emit a `debug!`/`warn!`.
7. `just test` and `just test-l2` pass on macOS; the changes are written to
   compile on macOS, Windows, and Linux.

---

# Priority 1 — Confirmed Panics (High)

Both are on paths fed by external/model-influenced text and are cheap, isolated
fixes. Both were verified against source during review.

## P1.1 — UTF-8 byte-slice panic in OpenCode error classifier

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/stream/logs/opencode/errors.rs:133-134`

### Problem

The per-error parse path is fed directly by untrusted upstream provider JSON.
When an `error` tag is not valid JSON and exceeds 500 bytes, the fallback slices
the raw string at a fixed **byte** index:

```rust
if error_tag.len() > 500 {
    return format!("{}...", &error_tag[..497]);
}
```

A multi-byte UTF-8 codepoint straddling byte 497 panics with
`byte index 497 is not a char boundary`, crashing the classifier on a single
oversized error string.

### Proposed Solution

Char-safe truncation. Either:

```rust
return format!("{}...", error_tag.chars().take(497).collect::<String>());
```

or compute a char boundary via `error_tag.char_indices().nth(497)` and slice
there. Prefer whichever keeps the `497 + "..."` intent; `chars().take(497)`
caps at 497 *characters* (acceptable — the value is a truncated diagnostic, not
a byte-exact field).

### Tests

Fail-first: an OpenCode `error` tag >500 bytes with a multi-byte char at byte
496–498 must classify without panic.

## P1.2 — UTF-8 byte-slice panic in protect tracing span

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/protect/service.rs:79`

### Problem

`evaluate_bash_command` runs on *every* bash-command protect evaluation, with
`command` being model/attacker-influenced text. The tracing span field slices at
a fixed byte index:

```rust
command_truncated = &command[..command.len().min(80)]
```

A command whose 80th byte falls inside a multi-byte codepoint panics the protect
evaluation. Because `protect` is a guard, a panic here can degrade the gate
depending on how the hook host treats a panicking evaluation.

### Proposed Solution

```rust
command_truncated = command
    .char_indices()
    .nth(80)
    .map_or(command, |(i, _)| &command[..i])
```

or `command.chars().take(80).collect::<String>()`.

### Tests

Fail-first: a >80-byte multibyte command into `evaluate_bash_command` must not
panic. Add to the protect test module.

---

# Priority 2 — Protect Posture and Extraction Hardening

The review identified two different classes of protect risk: the catalog is an
unparsed regex deny list, and the runtime extractor can return "no request" for
tool shapes that still clearly look command- or write-like. The reviewed design
does **not** promote protect into a hard security boundary. It keeps protect as a
best-effort defense-in-depth layer for accidents, obvious destructive commands,
and simple prompt-injection payloads, while closing fail-open extraction cases
that are cheap and low-risk.

> **Reader's note from inline review:** The pre-review draft referenced P2.1
> through P2.6 in acceptance and testing but omitted the Priority 2 body. This
> section restores that scope and makes the posture decision explicit. If a later
> feature wants protect to become a real boundary, it must specify shell parsing,
> provider-specific tool schemas, and fail-closed compatibility behavior as a
> separate design.

## P2.1 — Decide and document protect as defense-in-depth, not a security boundary

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/protect/catalog.rs`,
  `claudine/lib/src/protect/matcher.rs:90-109`,
  `claudine/lib/src/protect/service.rs:76-120`,
  `claudine/docs/topics/protect-service.md`

### Problem

Protect rules run regexes over literal, unparsed command text. The shell later
performs quoting, variable expansion, word splitting, globbing, and command
chaining. That makes rules such as `rm\s+-rf\s+/$`, `curl ... | bash`, and
`git push --force` useful for obvious cases but bypassable by ordinary shell
forms (`rm -fr /`, `\rm -rf /`, `X=rm; $X -rf /`, case changes, refspec force
pushes, and separator/chaining variants).

The current module and topic docs accurately describe a default-allow deny
catalog, but they do not state the operational consequence plainly enough:
protect is not the security boundary.

### Reviewed Design Decision

Keep protect as **best-effort defense-in-depth** for this remediation. Do not
try to make the regex catalog a complete shell security boundary in this spec.
Making it a boundary would require shell-aware parsing, provider-specific tool
schemas, Windows command parsing, and stricter compatibility decisions that are
larger than this review remediation.

Mitigate the side effect by documenting the posture at module level and in
`docs/topics/protect-service.md`, and by adding a bypass-corpus test suite whose
assertions encode the chosen posture:

- obvious destructive examples remain blocked;
- documented bypass forms are either blocked where cheap to support or marked as
  known non-boundary cases;
- provider permission systems and contract sandboxing remain the load-bearing
  controls for actual isolation.

### Tests

Add a protect bypass corpus covering shell variants from the review. Each case
must state whether it is expected to block under the current best-effort posture
or is a documented non-boundary case. This prevents future docs from drifting
back into boundary language without a corresponding implementation change.

## P2.2 — Fail-open extraction on command/write-shaped tools

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/dispatch/mod.rs:261-284`,
  `claudine/lib/src/dispatch/mod.rs:350-374`,
  `claudine/lib/src/protect/observe.rs:52-101`

### Problem

Protect blocks only when `extract_protect_request(...)` returns `Some` and the
service returns `Block`. A command-like tool whose command lives under `cmd`,
`script`, `input`, or an array currently returns `None` and is allowed. A
write-like tool using `filename`, `dest`, or `paths[]` similarly bypasses
sensitive-path scanning. Tool-name detection also misses common names such as
`run_command` and `terminal`.

### Proposed Solution

Introduce an explicit extraction outcome instead of overloading `Option`:

```rust
enum ProtectObservation<'a> {
    Request(ProtectRequest<'a>),
    NoOpinion,
    Unparsed { surface: ScanSurface, reason: &'static str },
}
```

The dispatch boundary should handle `Unparsed` according to the reviewed
posture:

- for tool names that are clearly command- or write-shaped, return a blocking
  provider response with a loud `warn!`;
- for unrelated tools, keep `NoOpinion` and allow normal execution;
- include the tool name and a secret-free reason in tracing.

Broaden command keys to include at least `command`, `cmd`, `script`, `input`,
and string arrays. Broaden write path keys to include at least `path`,
`file_path`, `file`, `target`, `filename`, `dest`, and `paths[]`.

### Tests

A Bash-like tool with the command under `cmd`, `script`, `input`, and an array is
not silently allowed. A write-like tool with `filename`, `dest`, and `paths[]` is
scanned. An unrelated tool with no relevant payload remains `NoOpinion`.

## P2.3 — `allow_paths` matching is too loose

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/protect/path.rs:173-188`

### Problem

For relative allow entries, any matching path segment permits the target:
`allow_paths = ["build"]` permits `/etc/build/passwd`. For absolute entries,
prefix matching must remain boundary-aware; `/var/tmp` must not permit
`/var/tmpevil`.

### Proposed Solution

Use the same boundary-aware prefix semantics for absolute entries everywhere.
For relative entries, match an anchored component sequence under the evaluated
target rather than any same-named segment anywhere in the path. Keep the common
developer use case (`node_modules`, `target`, `dist`, `build`, `.cache`) working
for project-local destructive commands.

### Tests

`/etc/build/passwd` is not allowed by `allow_paths = ["build"]`.
`/var/tmpevil` is not allowed by `allow_paths = ["/var/tmp"]`.
Existing intended suppressions for `rm -rf node_modules` and `rm -rf target`
continue to pass.

## P2.4 — Sensitive write-path prefix list omits credential locations

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/lib/src/protect/path.rs:14-29`

### Problem

The sensitive-path catalog is the only write-path guard and currently covers a
small set of absolute and home-relative prefixes. It misses common credential
and provider config files such as `~/.aws`, `~/.kube`, `~/.docker/config.json`,
`~/.netrc`, `~/.npmrc`, `~/.git-credentials`, `~/.config/gh`, and the agentic
CLI provider config directories. The absolute list also omits high-impact
system locations such as `/Library/LaunchDaemons`, `/sbin`, `/bin`, `/opt`, and
`/root`.

### Proposed Solution

Extend the built-in home-relative and absolute lists. Keep platform-specific
entries gated or harmless on other platforms:

- macOS: `/Library/LaunchDaemons`, `/System` (already present);
- Unix-like: `/bin`, `/sbin`, `/root`, `/opt`;
- home-relative: `.aws`, `.kube`, `.docker/config.json`, `.netrc`, `.npmrc`,
  `.git-credentials`, `.config/gh`, `.claude`, `.codex`, `.gemini`, `.goose`,
  `.opencode`, `.qwen`, `.roo`.

Do not add a user-configurable sensitive-path catalog in this remediation unless
implementation discovers that the static list creates unacceptable false
positives. A configurable catalog is useful, but it is not required to close the
review finding.

### Tests

Writes to the added credential/config paths are blocked. Platform-shaped
absolute paths are covered with OS-gated tests where needed.

## P2.5 — Custom protect patterns apply only to bash commands

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/protect/matcher.rs:74-82`,
  `claudine/lib/src/protect/service.rs` MCP evaluation path

### Problem

`custom_patterns` are compiled onto the bash-command scan surface only. A user
who adds a custom deny pattern expecting to block an MCP payload gets no block
and no warning. That is a security-relevant configuration trap even under the
best-effort posture.

### Proposed Solution

Add a `surface` field to `CustomPattern` with a default of `bash_command`.
Accepted values: `bash_command`, `mcp_response`, and `write_path` if write-path
string scanning is implemented; otherwise reject `write_path` with a clear
validation error. Route `mcp_response` custom patterns through
`evaluate_mcp_response`.

This is an intended config expansion. Mitigate compatibility risk by preserving
the old default and documenting that omitted `surface` means `bash_command`.

### Tests

A custom pattern with `surface = "mcp_response"` blocks an MCP payload. A custom
pattern with no `surface` still applies to bash commands. Invalid surfaces are
rejected at config validation.

## P2.6 — `allow_paths` is advertised for commands whose targets are not parsed reliably

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/lib/src/protect/path.rs:139-159`,
  built-in rules with `supports_allow_paths`

### Problem

`extract_target_paths` is effectively an `rm` operand heuristic. Some rules such
as `find ... -delete`, `chmod`, and `chown` advertise `supports_allow_paths`
even though their operand grammar differs from `rm`. That makes
`allow_paths` unreliable for those rules and can surprise users who think an
allow-list is active.

### Proposed Solution

For this remediation, mark `supports_allow_paths = false` for rules whose target
grammar is not parsed correctly, unless a small per-command extractor is added
in the same change. Document the limitation in `docs/topics/protect-service.md`.

### Tests

`find . -delete` with `allow_paths = ["."]` does not silently claim reliable
suppression unless a dedicated `find` extractor is implemented. `rm`-shaped
allow-path behavior remains covered by P2.3.

---


# Priority 3 — Rendezvous Daemon Concurrency Hardening

These gate any real concurrent load or network exposure. The daemon currently
reads as a well-built POC.

## P3.1 — Blocking redb/DuckDB I/O on async runtime threads, sync mutex held across fsync

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/rendezvous/daemon/src/service.rs:123/151/271/354`;
  root cause `session_log.rs:390-483` (lock held across `save_snapshot` at
  `:455`) and `storage.rs:204-232`.

### Problem

tonic `async fn` handlers run on tokio worker threads. `append_entry`
synchronously performs a redb `begin_write()`/`commit()` (an fsync) **while
holding `Arc<Mutex<ManagerInner>>`**, and `query_projection` calls DuckDB under a
`parking_lot::Mutex`, none wrapped in `spawn_blocking`. Under concurrent clients
this (a) blocks a runtime worker for the duration of disk I/O and (b) serializes
*all* sessions/peers behind one global fsync. A slow disk stalls the entire
daemon. The snapshot bytes are computed at `:451`, so the store does **not** need
the lock held during persistence. The projection batcher already uses a
dedicated thread — the RPC paths don't follow that pattern.

### Proposed Solution

1. Compute the staged snapshot under the lock, **drop the lock**, `save_snapshot`
   without it, then re-acquire briefly to swap state and bump the cursor
   (re-check the active chunk index on re-acquire; idempotent snapshots tolerate
   it).
2. Wrap the synchronous persistence in `tokio::task::spawn_blocking` (handles are
   `Clone + Send`), or move OLTP/OLAP behind a blocking actor as already done for
   the projection batcher.

### Tests

Simultaneous `append_entry` to one session exposes lock-across-fsync and verifies
cursor correctness; the handler does not serialize unrelated sessions.

## P3.2 — Staging→commit TOCTOU can clobber a concurrent commit

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/rendezvous/daemon/src/session_log.rs:642-711`
  (`stage_remote_update`) and `:718-729` (`commit_staged_update`).

### Problem

The lock is released between staging (validates the append-only prefix against
the *current* live chunk) and committing (`inner.chunks.insert(key, staged.state)`
overwrites the live chunk wholesale). Two concurrent inbound sync sessions for
the same peer-owned chunk can both stage against version N then both commit, the
second validated against a now-stale base — silently dropping entries. Inbound
responders are spawned concurrently per `accept_bi`, so this is reachable.

### Proposed Solution

Hold a per-chunk lock across stage+commit, **or** re-import the staged delta into
the current live doc (merge) rather than replacing, **or** re-run append-only
validation against current state before insert and retry on conflict.

### Tests

Two concurrent inbound sync sessions on one chunk; assert no entries are dropped.

## P3.3 — Sealer counter persisted under a second independent lock — can reissue a message-id

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/rendezvous/daemon/src/sync.rs:450-458`; analogous
  `session_log.rs:347-352`.

### Problem

`seal` and the subsequent `next_counter()` read for persistence use **two
separate** `sealer.lock()` acquisitions. The sealer is shared, so another task
can seal+persist between them; an interleaving can persist a lower value after a
higher one, and a post-restart `with_start` could reissue an already-used
message_id — undermining the durable inbox dedup.

### Proposed Solution

Capture the counter-to-persist inside the same lock scope as the seal (have
`seal` return the new counter, or read it before dropping the guard), then
persist outside the lock — or persist the counter transactionally with the
accepted-envelope write.

### Tests

Sealer-counter monotonicity across interleaved seals plus a simulated restart.

## P3.4 — Inbound peers keyed by `inbound:{addr}`, so `SyncWithPeer`/`connection_for` never find them

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/rendezvous/daemon/src/peers.rs:310-327`
  (`record_inbound`) vs `:183` (`connection_for(node_id)`), consumed at
  `service.rs:326`.

### Problem

An inbound connection's `PeerRecord` is stored under a synthetic
`inbound:<socketaddr>` key; its real `node_id` (learned in the hello handshake at
`sync.rs:346`) is never reconciled into the registry. `connection_for(&node_id)`
returns `None` for any inbound-connected peer → `failed_precondition "no active
QUIC connection"` despite a live connection.

### Proposed Solution

After the responder handshake validates `node_id`, re-key/merge the inbound
record under the real hex node_id. **Or** explicitly document that inbound peers
are intentionally responder-only for this phase.

### Tests

An inbound-connected peer targeted by `SyncWithPeer` resolves to its live
connection.

## P3.5 — `rebuild_projection_from_storage` truncates then fire-and-forgets re-submission

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/rendezvous/daemon/src/session_log.rs:819-851`.

### Problem

Startup `self.projection.truncate()?` (destructive) is followed by
`let _ = self.batcher.submit(...)` per row. If the batcher channel is closed or a
flush errors, the projection is left silently empty/partial after the truncate;
`QueryProjection` can observe an incomplete rebuild with no signal (redb stays
authoritative, so it's recoverable but wrong meanwhile).

### Proposed Solution

Write rebuild rows synchronously (bypass the async batcher) so
truncate+repopulate is atomic from the query path, **or** propagate submit errors
and defer the truncate.

### Tests

A submit failure during rebuild does not leave a silently-truncated projection.

## P3.6 — mDNS browse blocking task can outlive shutdown and leak a thread

- **Severity:** Low · **Confidence:** low
- **Location:** `claudine/rendezvous/daemon/src/discovery.rs:130-154`
  (+ `Drop` at `:88-90`).

### Problem

`browse_task` is a `spawn_blocking` loop on `receiver.recv()`. `Drop`'s
`task.abort()` is a no-op for blocking tasks; if mdns-sd doesn't error the
receiver promptly on shutdown, it survives the 1s timeout and leaks a
blocking-pool thread — visible under `just test-leaks`.

### Proposed Solution

Use `recv_timeout` with a periodic shutdown-flag check, **or** confirm
`daemon.shutdown()` drops the browse sender and document the dependence.

### Tests

`just test-leaks` shows no leaked browse thread after daemon shutdown.

## P3.7 — Permissive QUIC server-cert verifier (forward-looking flag)

- **Severity:** Low · **Confidence:** high (as a forward-looking flag)
- **Location:** `claudine/rendezvous/daemon/src/quic.rs:257-303`
  (`AcceptAnyServerCert`).

### Problem

The client accepts any server cert (documented; the envelope layer authenticates
payloads). But the QUIC connection identity isn't bound to the expected `node_id`
before data flows — a LAN MITM completes the handshake and is only rejected at
the hello-mismatch check, after resource spend. Acceptable for the stated LAN
POC.

### Proposed Solution

**Before shipping beyond LAN**, bind the QUIC cert to the node's Ed25519 key and
verify it in the custom verifier. For now: retain as a documented gate; no code
change required for the POC.

---

# Priority 4 — Wrapper Termination & Environment Hardening

The wrapper must not hang on a wedged child, and the per-iteration environment
mutation race must go.

## P4.1 — `wait_with_timeout` can block forever on `child.wait()` after SIGKILL

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/cli/src/commands/wrap/exec/timeouts.rs:59-66`.

### Problem

After the grace period this path sends `SIGKILL` then calls the **blocking**
`child.wait()?`. If the child is unkillable (D-state on a stuck FS/NFS mount, or
re-parented), `wait` never returns and the wrapper hangs — defeating the timeout
it was enforcing. It also kills a single PID (`child.id()`), not the process
group, so descendants aren't reaped. The structured path uses non-blocking
`try_wait` loops; this legacy path regresses.

### Proposed Solution

Replace the blocking `wait` with a bounded `try_wait` poll loop, kill `-pid` when
spawned in its own group, and cap the post-SIGKILL reap. **First confirm whether
`wait_with_timeout` is dead/legacy** — if so, remove it to eliminate the
divergent behavior (preferred, per Rule 2/3).

### Tests

An unkillable D-state child reap times out (simulated via a non-returning
`try_wait` seam) rather than hanging.

## P4.2 — PID-recycle TOCTOU on loop-driven kills

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/cli/src/commands/wrap/exec/termination.rs:109-194`
  (kills at `:138`, `:167`, `:190`).

### Problem

The SIGINT handler correctly checks `exited.load()` before `libc::kill`. But the
watchdog/early-termination/grace kills happen in the **poll loop**, computed from
`child_pid` captured once at `:75`, and are not gated on `child_exited` nor
preceded by a `try_wait`. Between the loop-top `try_wait` returning `None` and the
later `libc::kill`, the child can exit and the PID recycle. The positive-PID
branch (`:135-137`, `child_in_own_pgroup == false`) is a real single-PID recycle
window; the unconditional grace SIGKILL at `:190` is the most exposed.

### Proposed Solution

Re-check `child.try_wait()?.is_none()` immediately before each loop-driven kill,
or gate them on the same `child_exited` atomic. Prefer always killing via the
negative-PID group form and document why the positive-PID branch is benign.

### Tests

PID recycle around loop-driven kills (not just the handler) — assert no kill is
issued after the child exits.

## P4.3 — Per-iteration `env::set_var("PWD")` in the compose loop while reader/ticker threads may be alive

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/cli/src/commands/compose/loop_run.rs:196-201`.

### Problem

`std::env::set_var` is `unsafe` in edition 2024 (UB if another thread reads the
environment concurrently). The safety comment claims a single-threaded loop
driver — true at startup — but `set_var`/`remove_var("PWD")` runs at the **top of
every iteration**, and each iteration spawns reader threads, watchdog tickers, and
timing monitors that call `std::env::var`/`var_os`. A leaked reader from iteration
N (detached on timeout at `exec/mod.rs:450/468`) can be calling `getenv` when
iteration N+1 mutates `PWD`.

### Proposed Solution

Stop mutating the process-global environment in the loop. `PWD` is already
injected onto the child `Command` env map (`env/mod.rs:351`), so set it only there
via `.env("PWD", …)`. This removes the `unsafe` and the cross-iteration race.

### Tests

`set_var("PWD")` race under a leaked reader thread is no longer possible (the loop
no longer calls `set_var`); child still receives correct `PWD`.

## P4.4 — Disconnected watchdog channel silently disables timeout enforcement

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/cli/src/commands/wrap/exec/termination.rs:173-175`.

### Problem

`Err(TryRecvError::Disconnected) => {}` — if the watchdog ticker thread dies
(panic in send/render), the channel disconnects and both timeout rules are
silently disabled for the rest of the run, with no log.

### Proposed Solution

Emit `tracing::warn!("watchdog ticker channel disconnected; timeout enforcement
disabled for remainder of run")` and optionally stop polling.

### Tests

Disconnected watchdog channel asserts the `warn!` (and documents whether
enforcement stops).

## P4.5 — Unbounded/blocking waits and a magic grace constant in termination paths

- **Severity:** Low · **Confidence:** high (timeout overflow), medium (others)
- **Location:** `termination.rs:178-197` (post-SIGKILL 75ms spin, no upper bound);
  `exec/mod.rs:478-491` (`kill_process_group` ignores kill result, hard-coded
  200ms unrelated to `kill_grace`, fires at post-reap PGID); `exec/timeouts.rs:18`
  (`Instant::now() + Duration::from_secs(seconds)` can panic on absurd
  `--timeout`).

### Problem

A wedged (D-state) child can spin the wrapper forever post-SIGKILL; the 200ms
grace diverges from the configurable `kill_grace`; a pathological `--timeout`
panics on `Instant` overflow.

### Proposed Solution

Bound the post-SIGKILL reap and return a synthesized "could not reap" outcome;
derive the grace from `TimeoutConfig::kill_grace`; use
`Instant::now().checked_add(...)`.

### Tests

Absurd `--timeout` does not panic; post-SIGKILL reap is bounded.

## P4.6 — `is_sensitive_key` env sanitization is substring-only and misses common secrets

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/cli/src/commands/wrap/env/sanitize.rs:85-95`.

### Problem

Matches only a fixed substring set (`API_KEY`, `TOKEN`, `PASSWORD`, `SECRET`, …).
Real secrets under `STRIPE_KEY`, `SENDGRID_KEY` (bare `*_KEY`), `NPM_AUTH`,
`*_PAT`, `*_PWD`, `*_PEM` are not caught. The wrapper's purpose includes not
leaking host secrets into provider children, so false negatives are a
confidentiality risk.

### Proposed Solution

Add word-boundary `_KEY`, `AUTH`, `_PAT`, `PWD`, `_PEM` matching. Mind false
positives like `SSH_AUTH_SOCK`. (`contains("PRIVATE_KEY")` already excludes
`PUBLIC_KEY` — preserve that.)

### Tests

`STRIPE_KEY`, `NPM_AUTH`, `*_PAT`, `*_PWD`, `*_PEM` are redacted; `SSH_AUTH_SOCK`
is not falsely redacted.

## P4.7 — `redact_sensitive_args` is case-sensitive and misses short/aliased flags

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/cli/src/commands/wrap/env/sanitize.rs:101-148`.

### Problem

Only exact long flags (`--api-key`, …) are caught; `-k sk-…`, `--ApiKey`,
`--bearer …` leak verbatim into `AGENT_PARAMS` (serialized/logged).

### Proposed Solution

Lowercase before prefix-match, add aliases, and add a value-shape redactor for
known token prefixes (`sk-`, `ghp_`, `xox[bp]-`, `AKIA`).

### Tests

`redact_sensitive_args` case-insensitivity/alias coverage; value-shape redaction.

---

# Priority 5 — Lifecycle Guard Hole

## P5.1 — Undefined-variable lifecycle guard skips the ternary *condition*

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/composition/lifecycle.rs:784`
  (`find_undefined_variable`).

### Problem

The guard rejects lifecycle strings whose bare variable silently collapses to
`""` after composition. It descends function args, comparisons, and arithmetic,
but skips `Expr::Fallback` *and* `Expr::Ternary` wholesale:

```rust
Expr::Fallback { .. } | Expr::Ternary { .. } => None,
```

Skipping `Fallback` is correct; skipping the entire `Ternary` is not — the
**condition is evaluated**. `{{ missing == 'x' ? 'a' : 'b' }}` resolves `missing`
→ `""`, the condition is false, the else-branch renders cleanly, the post-compose
leak guard sees no surviving span and passes, and the undefined-var guard skipped
the node — so a typo'd variable in a ternary condition silently dispatches the
wrong lifecycle side effect (Discord/Slack/TTS). This is exactly the failure
class the guard was built to prevent.

### Proposed Solution

Descend the condition, keep skipping the branches:

```rust
Expr::Ternary { condition, .. } => find_undefined_variable(condition, defined),
Expr::Fallback { .. } => None,
```

### Tests

`{{ missing == 'x' ? 'a' : 'b' }}` and `{{ missing ? 'a' : 'b' }}` through
`validate_no_undefined_lifecycle_variables` are rejected; the branch operands of a
ternary remain tolerated (defined-condition + undefined-branch still passes).
Also add `{{ missing[0] }}` / `{{ missing.foo }}` for Index/MemberAccess descent.

---

# Priority 6 — Contract Crate Polish

## P6.1 — Shadow-`HOME` auth copy is all-or-nothing and collapses the real error

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/contract/src/home.rs:74-85` and
  `claudine/contract/src/adapter.rs:156-159`.

### Problem

`build_shadow_home` aborts on any single `copy`/`create_dir_all` error, and
`infer` maps it with `.map_err(|_| inference_error(Provider, "failed to create
isolated home"))`, discarding the real `io::Error` (ENOSPC, unreadable
credential, partial multi-file copy). The session fails with a generic message
and no diagnostic trail; a partial copy fails the whole run rather than
authenticating with what was available.

### Proposed Solution

`tracing::warn!(error = %err, ...)` before collapsing to the secret-free message.
Reconsider whether a failed auth copy should be fatal vs. letting the session
surface a clearer `Unauthorized`. The external (returned) message must stay
secret-free; only the local trace gains the detail.

### Tests

A simulated copy failure logs the underlying `io::Error` and still returns the
secret-free `InferenceError`.

## P6.2 — Codex `read-only` sandbox is documented as blocking network — overstated

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/contract/src/session.rs:236-240` (and `lib.rs` framing).

### Problem

The isolation story leans on "`--sandbox read-only` blocks every write and
network call." Codex's `read-only` restricts filesystem writes; network behavior
is governed separately and has varied by release. If a Codex build permits
outbound network under `read-only`, untrusted prompt text could exfiltrate the
real credentials present in the shadow `HOME`; the post-hoc `check_security`
backstop only catches actions that surface as stream tool/command items, not a
silent in-runtime call.

### Proposed Solution

Soften the comment to what the flag is verified to do (deny writes + post-hoc
stream rejection); treat network denial as a defense-in-depth assumption. If
network isolation is load-bearing, add an explicit Codex network-sandbox flag to
`tool_denial_args`.

### Tests

(Documentation change.) If a network-sandbox flag is added, assert it is present
in the Codex argv.

## P6.3 — Contract crate over-exports an internal session API

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/contract/src/lib.rs:54`
  (`pub use session::{RawSession, SessionPlan, SessionRunner}`).

### Problem

The crate's purpose is the `InferenceAdapter` impl. `with_runner`/`with_env_source`
are `#[cfg(test)]`, so there is no public way to inject a `SessionRunner` — yet
the trait + plan types are `pub`, committing the crate to a stable plan/argv API
that is really an internal/test seam.

### Proposed Solution

Make them `pub(crate)` unless deliberately part of the consumer
(Reaper/Darkmatter) contract; if intended, document that intent on each type.

### Tests

Crate compiles with the narrowed visibility; downstream consumers unaffected
(verify against `biscuit-contract` consumers).

## P6.4 — Missing secret-redaction-at-error-boundary test (headline security property)

- **Severity:** (testing gap) · **Confidence:** high
- **Location:** contract error boundary (`adapter.rs` error mapping).

### Problem

The headline security property — secrets in stderr never reach the returned
`InferenceError.message` — is currently unguarded by any test.

### Proposed Solution

Add a test that feeds stderr containing `sk-…` and asserts
`!error.message.contains("sk-")`. Also add the contract tests the review names:
spawn failure (`NotFound`/`PermissionDenied` → `Unavailable`); non-zero exit +
valid text → `Ok`; `rate_limit` via `retry_after_ms` only;
`stderr_diagnostics.auth_failures` path; a one-line note documenting the
deliberate absence of an internal timeout.

---

# Priority 7 — Cross-Cutting Hygiene

## P7.1 — JSON-walk hot paths deep-clone the whole subtree per access

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/dispatch/expression.rs:199, 209-217`; same
  pattern in `claudine/lib/src/stream/protocol/codex.rs`
  (`resolved_input`/`resolved_output`).

### Problem

`nested_pointer` does `let mut current = value.clone();` (full deep clone of
`tool_input`/`tool_response`/`doc`) then clones again per segment, discarding all
but one leaf. Every `{{tool_input.foo}}` interpolation and every drill-in matcher
clones the whole payload (file contents, diffs) on the per-event hot path.

### Proposed Solution

Walk by reference, clone only the leaf:

```rust
let mut current = value;
for part in path.split('.') {
    current = current.as_object()?.get(part)?;
}
Some(current.clone())
```

Apply the same to `resolve_extra` and the Codex `resolved_input`/`resolved_output`
clone chains.

### Tests

Existing interpolation/matcher tests must still pass (behavior-preserving); add a
type/size assertion if practical. This is the single most-repeated idiom worth
fixing (see Rust-Idiomaticity Notes).

## P7.2 — HTTP status defaulted to 429 / lossy `as u16` cast on external status code

- **Severity:** Medium · **Confidence:** high (cast), medium (429 default)
- **Location:** `claudine/lib/src/stream/logs/opencode/errors.rs:169, 309`.

### Problem

`get_http_status_description(code as u16)` wraps a malformed `statusCode: 70000`;
`status_code.unwrap_or(429)` stamps 429 onto a usage cap whose real code was 403
(Kimi billing). The `ProviderLimitKind` stays correct, but any consumer reading
the numeric code is misled.

### Proposed Solution

Use `u16::try_from(code).ok()` and skip the description on overflow; make the cap
status `Option<u16>` (preferred — sentinels in a numeric field read as real data
downstream) or document that `kind` is authoritative.

### Tests

`statusCode: 70000` does not produce a bogus description; a 403 usage cap is not
reported as 429 (or `kind` is asserted authoritative).

## P7.3 — Silent swallow on malformed provider error JSON hides schema drift

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/lib/src/stream/logs/opencode/errors.rs:129-138,
  193-229`.

### Problem

When an `error` tag or nested `responseBody` is non-JSON/truncated, the code falls
back to the raw string with no `debug!`. A provider schema change silently
degrades rich classification with no maintainer signal.

### Proposed Solution

`debug!(%err, "opencode error tag not valid JSON; falling back to raw")` on the
parse-failure arms.

### Tests

Malformed error JSON emits the `debug!` (capture via tracing test subscriber) and
still returns the raw fallback.

## P7.4 — Matcher fail-open vs fail-closed asymmetry is silent at load time

- **Severity:** Low · **Confidence:** high (behavior), medium (severity)
- **Location:** `claudine/lib/src/dispatch/matcher.rs:60-90, 121-143`.

### Problem

An *uncompilable* matcher → `None` → `matches()` returns `true` (binding fires
**unconditionally**); an expression that parses but fails to evaluate returns
`false`. A typo can silently enable a gated action; the per-binding `warn!` is
easy to miss across a large config.

### Proposed Solution

Emit one aggregated load-time `warn!` listing every binding whose matcher compiled
to `None` ("will fire unconditionally").

### Tests

A config with N uncompilable matchers produces one aggregated warning naming all
N bindings.

## P7.5 — `cleanup_old_backups` swallows remove failures

- **Severity:** Low · **Confidence:** high
- **Location:** `claudine/lib/src/config/backup.rs:40-69`.

### Problem

`if fs::remove_file(path).is_ok() { deleted += 1; }` — persistent permission
failure lets backups grow unbounded with no warning. (The
lexical==chronological sort invariant is correct and documented.)

### Proposed Solution

`warn!` on remove failure.

### Tests

A non-removable backup file triggers a `warn!`.

## P7.6 — `relative_path`/`create_resource_link` symlink TOCTOU and unenforced precondition

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/lib/src/linking/symlink.rs:75-115, 183-207`.

### Problem

`relative_path` doc requires both paths absolute but asserts nothing; a
`common_len == 0` input yields a target that escapes unexpectedly — and these
write symlinks into provider config dirs. `create_resource_link` stats-then-acts
(small TOCTOU) and `dest.is_dir()` follows symlinks.

### Proposed Solution

`debug_assert!` the absolute precondition (or return `Result`) and test the
no-common-prefix case; consider attempt-`symlink`-then-handle-`AlreadyExists`.
Mind Windows symlink semantics (privilege/dir-vs-file).

### Tests

The no-common-prefix case is covered; precondition is enforced.

## P7.7 — `which` major-version skew within the area

- **Severity:** Low · **Confidence:** high
- **Location:** `claudine/lib/Cargo.toml` (`which = "7"`) vs
  `claudine/cli/Cargo.toml` (`which = "8"`).

### Problem

Two majors of a PATH-resolution crate in one area inflate the graph/build and can
diverge subtly between lib and CLI.

### Proposed Solution

Unify on one major (prefer `8`); verify against `docs/dependencies.md` per the
repo drift rule (update the dependencies doc if the version changes).

### Tests

Build passes with unified version; `docs/dependencies.md` updated.

## P7.8 — Misc parsed-data robustness gaps

- **Severity:** Low · **Confidence:** medium
- **Locations / fixes:**
  - `errors.rs:22-27` — `statusCode` regex `r#""statusCode":(\d{3})"#` matches the
    first 3 digits of `4291`; add a `(?:\D|$)` boundary.
  - `session_log.rs:519-540` — `list_chunk_entries` disk fallback fabricates
    metadata (`created_at=0`) that would fail its own validator; read real
    metadata or comment the read-only intent.
  - `runner/null_strip.rs` — `null_strip` silently leaves nulls past depth 64;
    `warn!` once when the cap is hit.
  - `storage.rs:208/398` — `io::Error::new(ErrorKind::Other, …)`; use
    `io::Error::other`.

### Tests

`statusCode` boundary regression (`4291` → no match / correct capture); null-strip
depth-cap warning.

## P7.9 — Git porcelain rename/quoted-path detection-accuracy gap (retained from rejected set)

- **Severity:** Low · **Confidence:** medium
- **Location:** harness git porcelain parsing (per review's rejection note).

### Problem

The git porcelain *rename/quoted-path* handling is a real but Low
detection-accuracy gap (the rest of the "git double-strip / frontmatter
slice-order panic / fs-probe collision" cluster was verified non-issue).

### Proposed Solution

Handle rename (`R ` / `->`) and quoted-path porcelain forms in the dirty-files
parse; add a test with a renamed and a quoted (special-char) path.

### Tests

Renamed + quoted-path porcelain lines parse to the correct file set.

---

# Rust-Idiomaticity Directives (apply across the above)

These are stylistic constraints the review calls out; fold them into the relevant
fixes rather than treating as separate work items:

1. **Walk JSON by reference, clone leaves only** (P7.1) — the single most-repeated
   idiom worth fixing.
2. **No catch-all on the security-relevant `Provider` match.** `support.rs::
   auth_env_vars` uses `_ => &[]` though all 8 providers are enumerated. Dropping
   the `_` makes adding a 9th provider a compile error — apply wherever feasible
   on security-relevant `Provider` matches.
3. **Type-model "advisory vs guarantee" in protect** (supports P2.2) — distinguish
   `NoOpinion` (couldn't parse) from `Allow` (parsed, permitted) so fail-open/
   fail-closed is an explicit, testable choice at the dispatch boundary.
4. **`Option<u16>` over sentinel `429`** (P7.2) for provider status codes.
5. **Confirm `mark_user_interrupted` is lock-free** (Unsafe Review follow-up):
   `interrupt.rs` calls `crate::output::mark_user_interrupted()` from a signal
   handler — confirm it is a pure atomic store (no `OnceLock`/`Mutex` init) and
   add a contract comment at its definition.

---

# Consolidated Testing Plan

In rough priority order (mirrors review §4). Use **nextest** (`just test`,
`just test-l2`); fail-first for confirmed bugs.

1. **UTF-8 boundary panics** (P1.1, P1.2) — fail-first.
2. **Lifecycle ternary-condition undefined variable** (P5.1) — plus
   `{{ missing[0] }}` / `{{ missing.foo }}`.
3. **protect bypass corpus** (P2.1) — assert failure-to-bypass (or document the
   intentional gaps under Posture A).
4. **protect fail-open** (P2.2) — Bash-like tool with command under
   `cmd`/`script`/array; write tool with path under `dest`/`filename`.
5. **protect allow_paths boundary** (P2.3, P2.6) + custom-pattern MCP surface
   (P2.5).
6. **rendezvous concurrency** (P3.1–P3.4) — concurrent `append_entry`; two inbound
   sync sessions on one chunk; inbound peer targeted by `SyncWithPeer`; two
   concurrent `connect()` to one node_id; sealer-counter monotonicity + restart.
7. **wrap/exec** (P4.1–P4.7) — PID recycle around loop-driven kills; disconnected
   watchdog channel; unkillable D-state reap timeout; (PWD race removed by P4.3);
   the `#[cfg(not(unix))]` `wait_with_signal_and_early_termination` branch
   (currently untested); `redact_sensitive_args` case-insensitivity/alias.
8. **contract crate** (P6.1–P6.4) — spawn failure → `Unavailable`; non-zero exit +
   valid text → `Ok`; `rate_limit` via `retry_after_ms`; `auth_failures` path;
   **secret-redaction at the error boundary**; documented no-internal-timeout.
9. **Per-provider stream parsing** — missing discriminator, `tool_input` as a
   string instead of object, truncated JSON line → documented fallback not panic;
   `parse → serialize → parse` round-trip plus "`extra` stays empty for known
   payloads" so a new actionable field landing in `extra` fails a test.

---

# Cross-Platform Notes (macOS / Windows / Linux)

- The `libc::kill`/process-group/signal paths (P4.1, P4.2, P4.5) are Unix-only;
  the `#[cfg(not(unix))]` branch must keep parity and gains the missing test (§7).
- New sensitive absolute paths (P2.4) are Unix-shaped — gate or document Windows
  behavior; the home-relative entries are portable.
- Symlink changes (P7.6) must respect Windows symlink privilege/dir-vs-file
  semantics.
- All other items are platform-neutral.
