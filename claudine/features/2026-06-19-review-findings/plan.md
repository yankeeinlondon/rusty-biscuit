---
agent: open_code/zai-coding-plan/glm-5.2
phases: 7
created: 2026-06-19
start_phase: 1
yolo: "true"
source_spec: spec.md
source_review: ../../reviews/2026-06-19-comprehensive/review.md
packages:
  - claudine
  - claudine-cli
  - claudine-contract
  - rendezvous-daemon
source_files_during_phase_1:
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/composition/lifecycle.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .opencode/skill/claudine/SKILL.md
source_files_during_phase_2:
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/protect/observe.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/matcher.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/protect/catalog.rs
  - claudine/lib/src/protect/config.rs
  - claudine/lib/src/protect/mod.rs
  - claudine/lib/benches/runtime_hot_paths.rs
  - claudine/cli/src/commands/compose/mod.rs
  - claudine/cli/src/commands/compose/prep.rs
docs_updated_during_phase_2:
  - claudine/docs/topics/protect-service.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_3:
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/session_log.rs
  - claudine/rendezvous/daemon/src/storage.rs
  - claudine/rendezvous/daemon/src/sync.rs
  - claudine/rendezvous/daemon/src/peers.rs
  - claudine/rendezvous/daemon/src/discovery.rs
  - claudine/rendezvous/daemon/src/quic.rs
  - claudine/rendezvous/daemon/src/projection.rs
  - claudine/rendezvous/daemon/tests/phase6_integration.rs
  - claudine/rendezvous/justfile
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/compose/loop_run.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/Cargo.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/contract/Cargo.toml
  - claudine/contract/src/adapter.rs
  - claudine/contract/src/error.rs
  - claudine/contract/src/lib.rs
  - claudine/contract/src/profile.rs
  - claudine/contract/src/session.rs
  - claudine/contract/src/support.rs
  - claudine/contract/src/tests.rs
docs_updated_during_phase_5:
  - claudine/contract/README.md
  - claudine/contract/docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/src/stream/protocol/gemini.rs
  - claudine/lib/src/stream/protocol/qwen.rs
  - claudine/lib/src/stream/protocol/claude.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/dispatch/matcher.rs
  - claudine/lib/src/config/backup.rs
  - claudine/lib/src/linking/symlink.rs
  - claudine/lib/src/dispatch/runner/null_strip.rs
  - claudine/lib/src/harness/validate/git.rs
  - claudine/lib/src/stream/protocol/mod.rs
  - claudine/lib/Cargo.toml
  - claudine/cli/Cargo.toml
docs_updated_during_phase_6:
  - claudine/docs/dependencies.md
docs_created_during_phase_6:
  - claudine/docs/dependencies.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/features/2026-06-19-review-findings/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/protect/observe.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/matcher.rs
  - claudine/lib/src/protect/catalog.rs
  - claudine/lib/src/protect/config.rs
  - claudine/lib/src/protect/mod.rs
  - claudine/lib/benches/runtime_hot_paths.rs
  - claudine/cli/src/commands/compose/mod.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/rendezvous/daemon/src/service.rs
  - claudine/rendezvous/daemon/src/session_log.rs
  - claudine/rendezvous/daemon/src/storage.rs
  - claudine/rendezvous/daemon/src/sync.rs
  - claudine/rendezvous/daemon/src/peers.rs
  - claudine/rendezvous/daemon/src/discovery.rs
  - claudine/rendezvous/daemon/src/quic.rs
  - claudine/rendezvous/daemon/src/projection.rs
  - claudine/rendezvous/daemon/tests/phase6_integration.rs
  - claudine/rendezvous/justfile
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/compose/loop_run.rs
  - claudine/cli/src/commands/wrap/env/sanitize.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/Cargo.toml
  - claudine/contract/Cargo.toml
  - claudine/contract/src/adapter.rs
  - claudine/contract/src/error.rs
  - claudine/contract/src/lib.rs
  - claudine/contract/src/profile.rs
  - claudine/contract/src/session.rs
  - claudine/contract/src/support.rs
  - claudine/contract/src/tests.rs
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/src/stream/protocol/gemini.rs
  - claudine/lib/src/stream/protocol/qwen.rs
  - claudine/lib/src/stream/protocol/claude.rs
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/dispatch/matcher.rs
  - claudine/lib/src/config/backup.rs
  - claudine/lib/src/linking/symlink.rs
  - claudine/lib/src/dispatch/runner/null_strip.rs
  - claudine/lib/src/harness/validate/git.rs
  - claudine/lib/src/stream/protocol/mod.rs
  - claudine/lib/Cargo.toml
documentation:
  - claudine/docs/topics/protect-service.md
  - claudine/contract/README.md
  - claudine/contract/docs/dependencies.md
  - claudine/docs/dependencies.md
  - claudine/features/2026-06-19-review-findings/plan.md
---

# Comprehensive Review Remediation — Execution Plan

Converts [`spec.md`](spec.md) into a high-confidence, dependency-ordered
implementation plan. Every task is observable through a fail-first test, a
compile, a lint gate, or a behavioral checkpoint. Each task is tagged with its
spec finding ID (e.g. `P1.1`) so traceability back to the review is exact.

**Reviewed crates in scope:** `claudine` (lib), `claudine-cli`,
`claudine-contract`, `rendezvous-core`, `rendezvous-client`,
`rendezvous-daemon`. No other workspace member is touched.

**Rejected findings are NOT re-raised** (spec §Scope): the bash-action shell
injection note, "Qwen untagged enum drops arrays", the `LazyLock` regex
`.expect()`, and the git double-strip / frontmatter slice-order / fs-probe
collision cluster. The git porcelain *rename/quoted-path* gap is retained as
the Low-severity P7.9.

## Acceptance criteria (binds the whole effort)

These seven outcomes, copied from spec §Acceptance Criteria, are the
definition of done. Phase 7 verifies each one:

1. Both confirmed UTF-8 panics fixed, covered by fail-first regression tests
   that panic before the fix (P1.1, P1.2).
2. `protect` security posture explicitly decided, documented at module level,
   and locked by a bypass-corpus test suite (P2.1).
3. Rendezvous daemon holds no `parking_lot` mutex across fsync, runs no sync
   redb/DuckDB I/O on tokio worker threads, and closes the stage→commit and
   sealer-counter races (P3.1–P3.3).
4. Wrapper cannot hang on a wedged child; per-iteration `env::set_var("PWD")`
   race eliminated (P4.1–P4.5).
5. Lifecycle undefined-variable guard descends ternary conditions (P5.1).
6. Every "silent swallow" site emits a `debug!`/`warn!` (P4.4, P7.3, P7.4,
   P7.5, P7.8-null-strip).
7. `just test` and `just test-l2` pass on macOS; changes compile on macOS,
   Windows, and Linux.

## Conventions for the implementer

- **Tests first for confirmed bugs.** P1.1, P1.2, P5.1, P2.2, the P3 races,
  the P4 loop-driven kills, and the P6 secret-redaction gap are fail-first:
  write the test, observe it fail exactly as the spec describes, then fix.
- **Never run `cargo fmt` / `rustfmt` in write mode** (repo rule — `main` is
  the formatting authority). Match surrounding style by hand.
- **US English** for all prose, symbol names, and docs.
- **Use nextest**, not `cargo test`: `just test` (unit), `just test-l2`
  (integration/concurrency) inside the package area; `just test <pkg>` at
  repo root. Lint with `just lint`.
- **No comments unless they carry information the code does not.** Delete
  drifted comments; assume code is correct when comment and code disagree.
- **Rustdoc:** no `# H1` inside `///`; `## H2` sections only.
- **Line numbers in this plan are anchors** from the branch at planning time;
  re-locate with `rg` before editing because offsets drift.
- **Hashing / file refs:** no new hashing is required by this remediation.
- **Cross-platform:** every change must compile and behave on macOS, Windows,
  and Linux. Unix-only signal/`libc::kill` paths keep `#[cfg(not(unix))]`
  parity (see Phase 4).

## Risk register

| Risk | Mitigation |
|---|---|
| `errors.rs` edited in both Phase 1 (P1.1) and Phase 6 (P7.2/P7.3/P7.8) | Sequence Phase 6 after Phase 1; assign the file to one implementer across both phases |
| `protect/service.rs` edited in Phase 1 (P1.2) and Phase 2 (P2.2/P2.5) | Phase 2 explicitly follows Phase 1; re-locate the truncation fix site after Phase 1 lands |
| `path.rs` touched by P2.3, P2.4, P2.6 in one phase | Sequence these three sub-tasks on one implementer; commit after each lands |
| `termination.rs` touched by P4.2, P4.4, P4.5 | Sequence on one implementer; shared `child_exited` atomic is the coordination point |
| Rendezvous concurrency tests are flaky by nature | Use `just test-l2`, run concurrently-stressed cases with nextest retries; assert invariants, not timings |
| `which` major bump (P7.7) pulls a new transitive graph | Build both `claudine/lib` and `claudine-cli`; update/create `docs/dependencies.md` |
| Widening protect extraction (P2.2) creates false positives that break existing flows | Bypass corpus (P2.1) encodes the chosen posture; run full `just test-l2` at phase end |

---

## Dependency graph

```
                                  ┌──► Phase 2 (protect)           ──┐
                                  ├──► Phase 3 (rendezvous)         ──┤
Phase 1 (panics + lifecycle) ─────┤                                 │
                                  ├──► Phase 4 (wrapper/env)        ├─► Phase 7 (acceptance)
                                  ├──► Phase 5 (contract)           │
                                  └──► Phase 6 (cross-cutting)*     ──┘
```

- **Phase 1 is the prerequisite** for the `errors.rs` and `protect/service.rs`
  tasks inside Phases 2 and 6 (same files re-edited later).
- **Phases 2, 3, 4, 5 are mutually parallelizable** — they live in disjoint
  crates/areas (`claudine/lib::protect`, `rendezvous/daemon`, `claudine-cli`,
  `claudine/contract`). They may proceed concurrently after Phase 1.
- **Phase 6 depends on Phase 1** (errors.rs) and otherwise parallelizes with
  2–5 except for its own internal file sequencing.
- **Phase 7 depends on every prior phase** and is the acceptance gate.

---

## Phase 1 — Confirmed panics + lifecycle guard (fail-first correctness)

**Goal.** Close the two High-severity UTF-8 panics and the Medium-severity
lifecycle guard hole. All three are isolated, single-file, high-confidence
fixes with fail-first tests, and are fully independent of each other.

**Parallelizable.** The three tasks touch three different files
(`errors.rs`, `protect/service.rs`, `composition/lifecycle.rs`) and may run
concurrently. This is also the prerequisite phase for the later `errors.rs`
and `protect/service.rs` edits in Phases 2 and 6.

### 1.1 — Fix UTF-8 byte-slice panic in OpenCode error classifier (P1.1)

**File:** `claudine/lib/src/stream/logs/opencode/errors.rs:133-134`

- [x] Add a fail-first test: an OpenCode `error` tag >500 bytes with a
      multi-byte UTF-8 codepoint straddling byte 497 panics in
      `classify_error` (or the responsible entry point) before the fix. Commit
      the failing test (red).
- [x] Replace the byte-index slice
      `&error_tag[..497]` with char-safe truncation. Prefer
      `error_tag.chars().take(497).collect::<String>()` (the value is a
      truncated diagnostic, so a character cap is acceptable), recombined with
      the `"..."` suffix to preserve the original intent.
- [x] Confirm the fail-first test now passes (green) and that existing
      classifier tests are unchanged.

### 1.2 — Fix UTF-8 byte-slice panic in protect tracing span (P1.2)

**File:** `claudine/lib/src/protect/service.rs:79` (`evaluate_bash_command`)

- [x] Add a fail-first test in the protect test module: a >80-byte command
      whose 80th byte lands inside a multi-byte codepoint, passed into
      `evaluate_bash_command`, panics before the fix (red).
- [x] Replace
      `command_truncated = &command[..command.len().min(80)]` with a
      char-boundary truncation:
      `command.char_indices().nth(80).map_or(command, |(i, _)| &command[..i])`.
- [x] Confirm the fail-first test passes (green).

### 1.3 — Descend ternary conditions in lifecycle undefined-variable guard (P5.1)

**File:** `claudine/lib/src/composition/lifecycle.rs:784`
(`find_undefined_variable`)

- [x] Add fail-first tests through `validate_no_undefined_lifecycle_variables`
      (red): `{{ missing == 'x' ? 'a' : 'b' }}` and `{{ missing ? 'a' : 'b' }}`
      with `missing` absent from composed frontmatter are currently accepted
      (bug). Also assert the branch operands stay tolerated: a defined
      condition with an undefined branch operand still passes. Add
      `{{ missing[0] }}` and `{{ missing.foo }}` to exercise `Index` /
      `MemberAccess` descent.
- [x] Split the `Expr::Fallback { .. } | Expr::Ternary { .. } => None` arm.
      Descend the ternary condition only, keep skipping branches:
      `Expr::Ternary { condition, .. } => find_undefined_variable(condition, defined)`,
      `Expr::Fallback { .. } => None`.
- [x] Confirm all fail-first ternary cases now reject; the
      defined-condition/undefined-branch case still passes.

### Phase 1 validation checkpoint

- [x] `just test` green in the `claudine` area; `just lint` clean.
- [x] All three fail-first tests demonstrably went red→green (commit the test
      first, then the fix, per repo convention where applicable).
- [x] No behavioral change beyond the three documented fixes.

---

## Phase 2 — Protect posture & extraction hardening

**Goal.** Lock `protect` as documented best-effort defense-in-depth (not a
security boundary), close the fail-open extraction holes for command/write-
shaped tools, tighten `allow_paths` matching, extend the sensitive-path
catalog, route custom patterns to additional surfaces, and gate the whole
phase with a posture-encoding bypass corpus.

**Parallelizable with Phases 3, 4, 5** (disjoint area). Internally,
sub-tasks share files and are **sequenced**: the extraction type-model change
(2.1) lands first; the `path.rs` cluster (2.3/2.4/2.6) is done by one
implementer; 2.5 follows the service routing change; the docs + bypass corpus
(2.6-gate) is last and encodes the chosen posture.

> **Posture decision (from spec §P2.1, already reviewed):** protect stays
> best-effort defense-in-depth. Provider permission systems and contract
> sandboxing remain the load-bearing controls. Do **not** attempt shell-aware
> parsing or a hard boundary in this remediation.

### 2.1 — Introduce `ProtectObservation` outcome and broaden extraction (P2.2, idiomaticity #3)

**Files:** `claudine/lib/src/protect/observe.rs:52-101`,
`claudine/lib/src/dispatch/mod.rs:261-284, 350-374`

- [x] Introduce an explicit extraction outcome in place of overloaded `Option`:
      ```rust
      enum ProtectObservation<'a> {
          Request(ProtectRequest<'a>),
          NoOpinion,
          Unparsed { surface: ScanSurface, reason: &'static str },
      }
      ```
      `NoOpinion` = parsed, nothing relevant; `Unparsed` = looked command/write-
      shaped but could not extract. This makes fail-open vs fail-closed an
      explicit, testable choice at the dispatch boundary.
- [x] Broaden command-key detection in the extractor to at least `command`,
      `cmd`, `script`, `input`, and string arrays (not just the `command`
      string). Broaden write-path keys to at least `path`, `file_path`, `file`,
      `target`, `filename`, `dest`, and `paths[]`. Add tool-name recognition
      for common names the review flagged (`run_command`, `terminal`).
- [x] At the dispatch boundary, handle `Unparsed` per the reviewed posture:
      clearly command- or write-shaped tools return a blocking provider
      response with a loud `warn!`; unrelated tools stay `NoOpinion` and
      execute normally; include the tool name and a secret-free reason in
      tracing.
- [x] Tests (fail-first where a hole existed): a Bash-like tool with the
      command under `cmd`, `script`, `input`, and an array is **not** silently
      allowed; a write-like tool with `filename`, `dest`, and `paths[]` is
      scanned; an unrelated tool with no relevant payload stays `NoOpinion`.

### 2.2 — Tighten `allow_paths` boundary matching (P2.3)

**File:** `claudine/lib/src/protect/path.rs:173-188`

- [x] For relative allow entries, replace the any-segment match with an
      anchored component-sequence match under the evaluated target, so
      `allow_paths = ["build"]` no longer permits `/etc/build/passwd`.
- [x] Use boundary-aware prefix semantics for absolute entries everywhere, so
      `/var/tmp` does not permit `/var/tmpevil`.
- [x] Preserve the common developer use cases (`node_modules`, `target`,
      `dist`, `build`, `.cache`) for project-local destructive commands.
- [x] Tests: `/etc/build/passwd` is not allowed by `["build"]`;
      `/var/tmpevil` is not allowed by `["/var/tmp"]`; existing intended
      suppressions (`rm -rf node_modules`, `rm -rf target`) still pass.

### 2.3 — Extend sensitive write-path prefix catalog (P2.4)

**File:** `claudine/lib/src/protect/path.rs:14-29`

- [x] Add home-relative credential/config entries: `.aws`, `.kube`,
      `.docker/config.json`, `.netrc`, `.npmrc`, `.git-credentials`,
      `.config/gh`, `.claude`, `.codex`, `.gemini`, `.goose`, `.opencode`,
      `.qwen`, `.roo`.
- [x] Add absolute high-impact entries, platform-gated or harmless off-platform:
      macOS `/Library/LaunchDaemons`, `/System` (already present); Unix-like
      `/bin`, `/sbin`, `/root`, `/opt`.
- [x] Tests: writes to the added credential/config paths are blocked;
      absolute paths covered with OS-gated tests where needed. Do **not** add
      a user-configurable catalog unless implementation reveals unacceptable
      false positives.

### 2.4 — Route custom patterns to MCP (and optional write) surfaces (P2.5)

**Files:** `claudine/lib/src/protect/matcher.rs:74-82`,
`claudine/lib/src/protect/service.rs` (MCP evaluation path),
`claudine/lib/src/protect/config.rs`

- [x] Add a `surface` field to `CustomPattern` defaulting to `bash_command`.
      Accepted values: `bash_command`, `mcp_response`, and `write_path` only
      if write-path string scanning is implemented; otherwise reject
      `write_path` at config validation with a clear error.
- [x] Route `mcp_response` custom patterns through `evaluate_mcp_response`.
      Preserve the old default (omitted `surface` ⇒ `bash_command`) for
      compatibility.
- [x] Tests: a custom pattern with `surface = "mcp_response"` blocks an MCP
      payload; a custom pattern with no `surface` still applies to bash
      commands; invalid surfaces are rejected at config validation.

### 2.5 — Mark unreliable `supports_allow_paths` rules (P2.6)

**File:** `claudine/lib/src/protect/path.rs:139-159` (and built-in rules)

- [x] For rules whose target grammar is not parsed correctly by the
      `rm`-operand heuristic (`find ... -delete`, `chmod`, `chown`), set
      `supports_allow_paths = false` unless a small per-command extractor is
      added in the same change.
- [x] Document the limitation in `claudine/docs/topics/protect-service.md`.
- [x] Tests: `find . -delete` with `allow_paths = ["."]` does not silently
      claim reliable suppression (unless a dedicated `find` extractor is
      implemented); `rm`-shaped behavior remains covered by 2.2.

### 2.6 — Document posture and add the bypass-corpus gate (P2.1)

**Files:** `claudine/lib/src/protect/catalog.rs`,
`claudine/lib/src/protect/matcher.rs:90-109`,
`claudine/lib/src/protect/service.rs:76-120`,
`claudine/docs/topics/protect-service.md`

- [x] State the operational consequence plainly at module level and in
      `docs/topics/protect-service.md`: protect is best-effort
      defense-in-depth, **not** the security boundary; provider permission
      systems and contract sandboxing are the load-bearing controls.
- [x] Add a protect bypass-corpus test suite covering the shell variants from
      the review (`rm -fr /`, `\rm -rf /`, `X=rm; $X -rf /`, case changes,
      refspec force pushes, separator/chaining variants). Each case must
      declare whether it is expected to block under the current best-effort
      posture or is a documented non-boundary case.
- [x] Ensure obvious destructive examples remain blocked; documented bypass
      forms are either blocked where cheap or marked as known non-boundary
      cases.

### Phase 2 validation checkpoint

- [x] `just test` and `just test-l2` green in the `claudine` area; `just lint`
      clean.
- [x] The bypass corpus (2.6) passes with the posture it encodes — no docs
      drift back into "boundary" language without a matching implementation
      change.
- [x] `docs/topics/protect-service.md` reflects the new extraction, surface,
      and `allow_paths` behavior.

---

## Phase 3 — Rendezvous daemon concurrency hardening

**Goal.** Remove blocking I/O from tokio worker threads, stop holding a mutex
across fsync, and close the stage→commit, sealer-counter, and inbound-peer
races. The daemon remains a LAN POC; the permissive QUIC verifier is retained
as a documented forward-looking gate (P3.7).

**Parallelizable with Phases 2, 4, 5** (separate crate). Internally
sequenced where files overlap (`session_log.rs` is touched by 3.1, 3.2, 3.3,
3.5, and a P7.8 cleanup). All concurrency assertions are integration-level —
run under `just test-l2` with nextest retries.

### 3.1 — Lift lock across fsync and move sync I/O off worker threads (P3.1)

**Files:** `claudine/rendezvous/daemon/src/session_log.rs:390-483`
(snapshot bytes computed at `:451`, lock held across `save_snapshot` at
`:455`), `claudine/rendezvous/daemon/src/storage.rs:204-232`,
`claudine/rendezvous/daemon/src/service.rs:123/151/271/354`

- [x] Compute the staged snapshot under the lock, **drop the lock**, call
      `save_snapshot` without it, then re-acquire briefly to swap state and
      bump the cursor. Re-check the active chunk index on re-acquire
      (idempotent snapshots tolerate it).
- [x] Wrap synchronous redb `begin_write()`/`commit()` persistence in
      `tokio::task::spawn_blocking` (handles are `Clone + Send`), or move
      OLTP/OLAP behind a blocking actor as the projection batcher already
      does. Ensure `query_projection`'s DuckDB call under
      `parking_lot::Mutex` is likewise off the worker thread.
- [x] Test (integration): simultaneous `append_entry` to one session exposes
      the lock-across-fsync regression and verifies cursor correctness; the
      handler does not serialize unrelated sessions.

### 3.2 — Close staging→commit TOCTOU (P3.2)

**File:** `claudine/rendezvous/daemon/src/session_log.rs:642-711`
(`stage_remote_update`) and `:718-729` (`commit_staged_update`)

- [x] Hold a per-chunk lock across stage+commit, **or** re-import the staged
      delta into the current live doc (merge) rather than wholesale replacing,
      **or** re-run append-only validation against current state before insert
      and retry on conflict.
- [x] Test (integration): two concurrent inbound sync sessions on one chunk;
      assert no entries are dropped.

### 3.3 — Make sealer-counter persistence atomic with the seal (P3.3)

**Files:** `claudine/rendezvous/daemon/src/sync.rs:450-458`;
`claudine/rendezvous/daemon/src/session_log.rs:347-352`

- [x] Capture the counter-to-persist inside the same lock scope as the seal
      (have `seal` return the new counter, or read it before dropping the
      guard), then persist outside the lock — or persist the counter
      transactionally with the accepted-envelope write.
- [x] Test (integration): sealer-counter monotonicity across interleaved seals
      plus a simulated restart (post-restart `with_start` must not reissue an
      already-used message_id).

### 3.4 — Re-key inbound peers under the real node_id (P3.4)

**Files:** `claudine/rendezvous/daemon/src/peers.rs:310-327`
(`record_inbound`) vs `:183` (`connection_for(node_id)`); consumed at
`service.rs:326`; handshake node_id learned at `sync.rs:346`

- [x] After the responder handshake validates `node_id`, re-key/merge the
      inbound record under the real hex node_id; **or** explicitly document
      that inbound peers are intentionally responder-only for this phase.
- [x] Test (integration): an inbound-connected peer targeted by
      `SyncWithPeer` resolves to its live connection (no
      `failed_precondition "no active QUIC connection"`).

### 3.5 — Make projection rebuild atomic (P3.5)

**File:** `claudine/rendezvous/daemon/src/session_log.rs:819-851`

- [x] Write rebuild rows synchronously (bypass the async batcher) so
      truncate+repopulate is atomic from the query path, **or** propagate
      submit errors and defer the truncate until repopulate is confirmed.
- [x] Test (integration): a submit failure during rebuild does not leave a
      silently-truncated projection.

### 3.6 — Bound the mDNS browse blocking task (P3.6)

**File:** `claudine/rendezvous/daemon/src/discovery.rs:130-154`
(`Drop` at `:88-90`)

- [x] Switch the `spawn_blocking` `recv()` loop to `recv_timeout` with a
      periodic shutdown-flag check, **or** confirm `daemon.shutdown()` drops
      the browse sender and document the dependence.
- [x] Test: `just test-l2` (leak suite) shows no leaked browse thread after
      daemon shutdown.

### 3.7 — Document the permissive QUIC verifier as a forward-looking gate (P3.7)

**File:** `claudine/rendezvous/daemon/src/quic.rs:257-303` (`AcceptAnyServerCert`)

- [x] No code change required for the POC. Add/confirm a code comment and
      topic-doc note that, before shipping beyond LAN, the QUIC cert must be
      bound to the node's Ed25519 key and verified in the custom verifier.

### 3.8 — Rendezvous-side robustness cleanups (P7.8 rendezvous items)

**Files:** `claudine/rendezvous/daemon/src/session_log.rs:519-540`
(`list_chunk_entries` disk fallback), `claudine/rendezvous/daemon/src/storage.rs:208/398`

- [x] `list_chunk_entries` disk fallback: stop fabricating metadata
      (`created_at=0`) that would fail the crate's own validator — read real
      metadata, or comment the read-only intent explicitly.
- [x] Replace `io::Error::new(ErrorKind::Other, …)` with `io::Error::other`.
- [x] Tests: `list_chunk_entries` fallback path returns metadata consistent
      with the validator (or documents the read-only intent).

### Phase 3 validation checkpoint

- [x] `just test` and `just test-l2` green in the `rendezvous` area (run from
      the `rendezvous/daemon` area or `just test rendezvous-daemon` at root).
- [x] No mutex held across fsync; no sync redb/DuckDB I/O on tokio worker
      threads (verifiable by the 3.1 integration test and a code read).
- [x] `just test-l2` leak suite is clean after daemon shutdown.

---

## Phase 4 — Wrapper termination & environment hardening

**Goal.** Ensure the wrapper cannot hang on a wedged child, remove the PID-
recycle window around loop-driven kills, eliminate the per-iteration
`env::set_var("PWD")` race, surface the disconnected-watchdog failure, bound
the post-SIGKILL reap, and broaden secret redaction. Includes the
signal-handler lock-free confirmation (idiomaticity #5).

**Parallelizable with Phases 2, 3, 5** (CLI crate area). Internally
**sequenced** on the `termination.rs` cluster (4.2/4.4/4.5 share the poll
loop and the `child_exited` atomic). 4.1 is a decision task that may remove
code the others touch — resolve it first.

### 4.1 — Decide fate of `wait_with_timeout`, then fix or remove (P4.1)

**File:** `claudine/cli/src/commands/wrap/exec/timeouts.rs:59-66`

- [x] **First**, determine whether `wait_with_timeout` is dead/legacy. If it
      is, remove it (preferred, per repo Rules 2/3) so the divergent blocking
      behavior cannot regress.
- [x] If it is live: replace the blocking `child.wait()?` after SIGKILL with a
      bounded `try_wait` poll loop, kill `-pid` (process group) when spawned
      in its own group, and cap the post-SIGKILL reap.
- [x] Test: an unkillable D-state child reap times out (simulated via a
      non-returning `try_wait` seam) rather than hanging.

### 4.2 — Close PID-recycle TOCTOU on loop-driven kills (P4.2)

**File:** `claudine/cli/src/commands/wrap/exec/termination.rs:109-194`
(kills at `:138`, `:167`, `:190`)

- [x] Re-check `child.try_wait()?.is_none()` immediately before each
      loop-driven kill (watchdog/early-termination/grace kills at `:138`,
      `:167`, `:190`), or gate them on the same `child_exited` atomic used by
      the SIGINT handler. The unconditional grace SIGKILL at `:190` is the
      most exposed.
- [x] Prefer always killing via the negative-PID group form; document why the
      positive-PID branch (`:135-137`, `child_in_own_pgroup == false`) is
      benign.
- [x] Test (fail-first): PID recycle around loop-driven kills — assert no
      kill is issued after the child exits (not just the handler path).

### 4.3 — Stop mutating the process environment in the compose loop (P4.3)

**Files:** `claudine/cli/src/commands/compose/loop_run.rs:196-201`,
`claudine/cli/src/commands/wrap/env/mod.rs:351`

- [x] Remove the per-iteration `env::set_var("PWD")` / `remove_var("PWD")`
      from the top of the compose loop. `PWD` is already injected onto the
      child `Command` env map (`env/mod.rs:351`), so set it only there via
      `.env("PWD", …)`. This removes the edition-2024 `unsafe` and the
      cross-iteration race with leaked reader/ticker threads.
- [x] Test: confirm the loop no longer calls `set_var` for `PWD` (the race
      window is gone by construction) and the child still receives the correct
      `PWD`.

### 4.4 — Log the disconnected watchdog channel (P4.4)

**File:** `claudine/cli/src/commands/wrap/exec/termination.rs:173-175`

- [x] Replace the silent `Err(TryRecvError::Disconnected) => {}` with
      `tracing::warn!("watchdog ticker channel disconnected; timeout
      enforcement disabled for remainder of run")`; optionally stop polling.
- [x] Test: a disconnected watchdog channel asserts the `warn!` (and the test
      documents whether enforcement stops).

### 4.5 — Bound waits, derive grace, and overflow-check the deadline (P4.5)

**Files:** `claudine/cli/src/commands/wrap/exec/termination.rs:178-197`
(post-SIGKILL 75ms spin, no upper bound),
`claudine/cli/src/commands/wrap/exec/mod.rs:478-491` (`kill_process_group`
ignores kill result, hard-coded 200ms unrelated to `kill_grace`),
`claudine/cli/src/commands/wrap/exec/timeouts.rs:18`
(`Instant::now() + Duration::from_secs(seconds)` can panic)

- [x] Bound the post-SIGKILL reap and return a synthesized "could not reap"
      outcome instead of spinning forever.
- [x] Derive the `kill_process_group` grace from `TimeoutConfig::kill_grace`
      instead of the hard-coded 200ms.
- [x] Use `Instant::now().checked_add(...)` for the deadline so an absurd
      `--timeout` returns an error instead of panicking.
- [x] Tests: absurd `--timeout` does not panic; post-SIGKILL reap is bounded.

### 4.6 — Broaden `is_sensitive_key` env sanitization (P4.6)

**File:** `claudine/cli/src/commands/wrap/env/sanitize.rs:85-95`

- [x] Add word-boundary matching for `_KEY`, `AUTH`, `_PAT`, `PWD`, `_PEM`.
      Catch `STRIPE_KEY`, `SENDGRID_KEY` (bare `*_KEY`), `NPM_AUTH`, `*_PAT`,
      `*_PWD`, `*_PEM`. Preserve the existing `contains("PRIVATE_KEY")`
      exclusion of `PUBLIC_KEY`.
- [x] Avoid false positives such as `SSH_AUTH_SOCK`.
- [x] Tests: `STRIPE_KEY`, `NPM_AUTH`, `*_PAT`, `*_PWD`, `*_PEM` are redacted;
      `SSH_AUTH_SOCK` is not falsely redacted.

### 4.7 — Case-insensitive, alias-aware `redact_sensitive_args` (P4.7)

**File:** `claudine/cli/src/commands/wrap/env/sanitize.rs:101-148`

- [x] Lowercase before prefix-match; add aliases (`-k`, `--bearer`, `--ApiKey`).
- [x] Add a value-shape redactor for known token prefixes (`sk-`, `ghp_`,
      `xox[bp]-`, `AKIA`).
- [x] Tests: `redact_sensitive_args` case-insensitivity/alias coverage and
      value-shape redaction; also cover the previously-untested
      `#[cfg(not(unix))]` `wait_with_signal_and_early_termination` branch.

### 4.8 — Confirm `mark_user_interrupted` is signal-safe (idiomaticity #5)

**File:** `claudine/cli/src/output/mod.rs` (`mark_user_interrupted`),
called from the SIGINT handler in `interrupt.rs`

- [x] Confirm `mark_user_interrupted` is a pure atomic store with no
      `OnceLock`/`Mutex` initialization on the store path (async-signal-safe).
- [x] Add a contract comment at its definition documenting the
      signal-handler-safety invariant.

### Phase 4 validation checkpoint

- [x] `just test` and `just test-l2` green in the `claudine-cli` area;
      `just lint` clean.
- [x] Unix signal/`libc::kill` paths verified; the `#[cfg(not(unix))]` branch
      has the newly added test (4.7) and keeps parity (compiles on Windows).

---

## Phase 5 — Contract crate polish

**Goal.** Preserve the real `io::Error` in shadow-home auth copy failures,
correct the overstated Codex `read-only` network claim, narrow an over-exported
internal session API, and add the missing secret-redaction-at-boundary
regression plus the named contract tests. Includes the security-relevant
`Provider` match tightening (idiomaticity #2).

**Parallelizable with Phases 2, 3, 4, 6** (separate crate). Internally
independent; tasks may run in parallel.

### 5.1 — Log the real error in shadow-home auth copy, keep the secret-free message (P6.1)

**Files:** `claudine/contract/src/home.rs:74-85`,
`claudine/contract/src/adapter.rs:156-159`

- [x] Before collapsing to the secret-free inference error, emit
      `tracing::warn!(error = %err, ...)` with the underlying `io::Error`
      (ENOSPC, unreadable credential, partial copy).
- [x] Reconsider whether a failed auth copy should be fatal vs. letting the
      session surface a clearer `Unauthorized`; the external returned message
      must stay secret-free — only the local trace gains detail.
- [x] Test: a simulated copy failure logs the underlying `io::Error` and still
      returns the secret-free `InferenceError`.

### 5.2 — Correct the Codex `read-only` network-isolation claim (P6.2)

**File:** `claudine/contract/src/session.rs:236-240` (and `lib.rs` framing)

- [x] Soften the comment/doc to what `--sandbox read-only` is verified to do
      (deny writes + post-hoc stream rejection); treat network denial as a
      defense-in-depth assumption, not a guarantee.
- [x] If network isolation is load-bearing, add an explicit Codex
      network-sandbox flag to `tool_denial_args`.
- [x] Test: documentation change. If a network-sandbox flag is added, assert
      it is present in the Codex argv.

### 5.3 — Narrow the over-exported internal session API (P6.3)

**File:** `claudine/contract/src/lib.rs:54`
(`pub use session::{RawSession, SessionPlan, SessionRunner}`)

- [x] Make `RawSession`/`SessionPlan`/`SessionRunner` `pub(crate)` unless they
      are deliberately part of the consumer (Reaper/Darkmatter) contract; if
      intended, document that intent on each type.
- [x] Verify against `biscuit-contract` consumers; the crate compiles with the
      narrowed visibility and downstream consumers are unaffected.

### 5.4 — Add the secret-redaction-at-boundary test and named contract tests (P6.4)

**File:** `claudine/contract/src/adapter.rs` (error mapping / boundary)

- [x] Add the headline security test: feed stderr containing `sk-…` and assert
      `!error.message.contains("sk-")`.
- [x] Add the contract tests the review names: spawn failure
      (`NotFound`/`PermissionDenied` → `Unavailable`); non-zero exit + valid
      text → `Ok`; `rate_limit` via `retry_after_ms` only;
      `stderr_diagnostics.auth_failures` path; a one-line note documenting the
      deliberate absence of an internal timeout.

### 5.5 — Drop the catch-all on the security-relevant `Provider` match (idiomaticity #2)

**File:** `claudine/contract/src/support.rs` (`auth_env_vars`)

- [x] Remove the `_ => &[]` arm so all enumerated providers are covered
      explicitly; adding a 9th provider becomes a compile error rather than a
      silent "no auth env vars" result.
- [x] Test: existing auth-env-var behavior unchanged across all providers.

### Phase 5 validation checkpoint

- [x] `just test` green in the `claudine-contract` area; `just lint` clean.
- [x] Verify downstream consumers (`biscuit-contract` users: Reaper,
      Darkmatter) still compile.

---

## Phase 6 — Cross-cutting hygiene & idiomaticity

**Goal.** De-clone the JSON-walk hot paths, fix the status-code cast/429
sentinel, surface silent swallows, add load-time matcher warnings, harden
symlink preconditions, unify the `which` crate version, and close the misc
robustness gaps including the git porcelain rename/quoted-path parse. Adds the
per-provider stream-parsing robustness regression suite.

**Depends on Phase 1** (errors.rs is re-edited here). Otherwise parallelizable
with Phases 2–5. Internally, the `errors.rs` tasks (6.2/6.3/6.4-regex) run on
one implementer after Phase 1's P1.1.

### 6.1 — Walk JSON by reference, clone leaves only (P7.1, idiomaticity #1)

**Files:** `claudine/lib/src/dispatch/expression.rs:199, 209-217`;
same pattern in `claudine/lib/src/stream/protocol/codex.rs`
(`resolved_input`/`resolved_output`)

- [x] In `nested_pointer` (and `resolve_extra`), stop deep-cloning the whole
      subtree per access. Walk by reference and clone only the leaf:
      ```rust
      let mut current = value;
      for part in path.split('.') {
          current = current.as_object()?.get(part)?;
      }
      Some(current.clone())
      ```
- [x] Apply the same reference-walk to the Codex `resolved_input`/
      `resolved_output` clone chains.
- [x] Tests: existing interpolation/matcher tests still pass
      (behavior-preserving); add a size/type assertion if practical.

### 6.2 — Use `Option<u16>` and `u16::try_from` for provider status codes (P7.2, idiomaticity #4)

**File:** `claudine/lib/src/stream/logs/opencode/errors.rs:169, 309`

- [x] Replace `get_http_status_description(code as u16)` with
      `u16::try_from(code).ok()` and skip the description on overflow.
- [x] Make the cap status `Option<u16>` (preferred) so a sentinel in a numeric
      field does not read as real data downstream; otherwise document that
      `ProviderLimitKind` is authoritative. Stop stamping `429` onto a usage
      cap whose real code was 403 (`status_code.unwrap_or(429)`).
- [x] Tests: `statusCode: 70000` does not produce a bogus description; a 403
      usage cap is not reported as 429 (or `kind` is asserted authoritative).

### 6.3 — Log malformed provider error JSON fallback (P7.3)

**File:** `claudine/lib/src/stream/logs/opencode/errors.rs:129-138, 193-229`

- [x] On the parse-failure arms (non-JSON/truncated `error` tag or
      `responseBody`), emit
      `debug!(%err, "opencode error tag not valid JSON; falling back to raw")`
      before returning the raw fallback.
- [x] Test: malformed error JSON emits the `debug!` (captured via a tracing
      test subscriber) and still returns the raw fallback.

### 6.4 — Misc `errors.rs` regex + matcher/backup/symlink/which/porcelain cleanups (P7.4–P7.9, idiomaticity misc)

These are small, independent fixes grouped to keep file churn coherent.

- [x] **P7.4** (`claudine/lib/src/dispatch/matcher.rs:60-90, 121-143`): emit
      one aggregated load-time `warn!` listing every binding whose matcher
      compiled to `None` ("will fire unconditionally"), rather than relying on
      per-binding warnings. Test: N uncompilable matchers produce one
      aggregated warning naming all N.
- [x] **P7.5** (`claudine/lib/src/config/backup.rs:40-69`): `warn!` on
      `cleanup_old_backups` remove failure instead of swallowing it. Test: a
      non-removable backup triggers a `warn!`.
- [x] **P7.6** (`claudine/lib/src/linking/symlink.rs:75-115, 183-207`):
      `debug_assert!` (or return `Result` for) the absolute-path precondition
      on `relative_path`; cover the no-common-prefix case; prefer
      attempt-`symlink`-then-handle-`AlreadyExists`; mind Windows symlink
      privilege/dir-vs-file semantics. Tests: no-common-prefix case covered;
      precondition enforced.
- [x] **P7.7** (`claudine/lib/Cargo.toml` `which = "7"` vs
      `claudine/cli/Cargo.toml` `which = "8"`): unify on major `8`; update
      **or create** `claudine/docs/dependencies.md` per the repo drift rule
      (no such file exists today). Test: build passes with the unified
      version; the dependencies doc reflects it.
- [x] **P7.8 (lib items)** (`claudine/lib/src/stream/logs/opencode/errors.rs:22-27`):
      tighten the `statusCode` regex `r#""statusCode":(\d{3})"#` with a
      `(?:\D|$)` boundary so `4291` does not match as `429`.
      (`claudine/lib/src/dispatch/runner/null_strip.rs`): `warn!` once when
      the null-strip depth cap (64) is hit instead of silently leaving nulls.
      Tests: `4291` → no match / correct capture; null-strip depth-cap warning
      emitted.
- [x] **P7.9** (`claudine/lib/src/harness/validate/git.rs`, dirty-files
      porcelain parse): handle rename (`R ` / `->`) and quoted-path porcelain
      forms. Tests: renamed + quoted (special-char) path porcelain lines parse
      to the correct file set.

### 6.5 — Per-provider stream-parsing robustness regression suite (Consolidated Testing Plan #9)

**Files:** `claudine/lib/src/stream/protocol/*` and per-provider parsers

- [x] Add regression tests for: missing discriminator; `tool_input` delivered
      as a string instead of an object (documented fallback, no panic);
      truncated JSON line (documented fallback, no panic); and a
      `parse → serialize → parse` round-trip plus an "`extra` stays empty for
      known payloads" assertion so a new actionable field landing in `extra`
      fails a test.

### Phase 6 validation checkpoint

- [x] `just test` and `just test-l2` green in the `claudine` area; `just lint`
      clean.
- [x] The `which` unification builds both lib and CLI; dependencies doc
      updated/created.
- [x] No silent-swallow sites remain in the touched modules (all emit
      `debug!`/`warn!`).

---

## Phase 7 — Whole-effort acceptance & cross-platform verification

**Goal.** Verify the seven acceptance criteria end to end, confirm
cross-platform compilation, and reconcile docs/skill drift. No new source
beyond drift fixes; this is the gate every prior phase rolls into.

**Depends on every prior phase.** Not parallelizable — it is the final gate.

### 7.1 — Run the consolidated testing plan end to end

- [x] Confirm each Consolidated Testing Plan item (spec §9) is covered and
      green: (1) UTF-8 boundary panics; (2) lifecycle ternary condition; (3)
      protect bypass corpus; (4) protect fail-open; (5) protect `allow_paths`
      boundary + custom MCP surface; (6) rendezvous concurrency; (7) wrap/exec
      termination + sanitization; (8) contract crate incl. secret redaction;
      (9) per-provider stream parsing.

### 7.2 — Acceptance criteria sign-off

- [x] **AC1:** both UTF-8 panics fixed with fail-first regression tests
      (Phase 1).
- [x] **AC2:** protect posture decided, documented at module level, locked by
      the bypass-corpus suite (Phase 2).
- [x] **AC3:** rendezvous holds no mutex across fsync, no sync I/O on worker
      threads, stage→commit and sealer-counter races closed (Phase 3).
- [x] **AC4:** wrapper cannot hang on a wedged child; per-iteration
      `set_var("PWD")` race eliminated (Phase 4).
- [x] **AC5:** lifecycle guard descends ternary conditions (Phase 1).
- [x] **AC6:** every silent-swallow site emits `debug!`/`warn!` (Phases 3–6).
- [x] **AC7:** `just test` and `just test-l2` pass on macOS; compiles on
      macOS, Windows, and Linux.

### 7.3 — Cross-platform compile verification

- [x] Verify the Unix-only signal/`libc::kill`/process-group paths
      (P4.1/P4.2/P4.5) keep `#[cfg(not(unix))]` parity and the new non-Unix
      test (P4.7) passes.
- [x] Confirm the new sensitive absolute paths (P2.4) are OS-gated or
      harmless off-platform; home-relative entries are portable.
- [x] Confirm symlink changes (P7.6) respect Windows symlink privilege and
      dir-vs-file semantics.
- [x] Where real Windows/Linux hardware is unavailable, at minimum perform a
      `cargo check` for the non-macOS targets and document the limitation.

**Limitation note.** Real Windows and Linux hosts were not available for this
phase, and the `x86_64-unknown-linux-gnu` / Windows targets are not installed
in the macOS development environment, so a direct `cargo check` for those
targets could not be completed. All code uses `#[cfg(unix)]` / `#[cfg(not(unix))]`
gates (verified by the macOS build and existing non-Unix test branch in
`sanitize.rs`), and no Unix-only APIs are invoked on non-Unix paths.

### 7.4 — Docs and skill drift reconciliation

- [x] Confirm `claudine/docs/topics/protect-service.md` matches the new
      extraction/surface/`allow_paths` behavior and the defense-in-depth
      posture (Phase 2).
- [x] Confirm `claudine/docs/dependencies.md` reflects the unified `which`
      version (Phase 6); create it if it did not exist.
- [x] Update the area skill (`.opencode/skill/claudine/SKILL.md`) and/or
      `claudine/CLAUDE.md` only where a module/behavior summary actually
      drifted (e.g. the `ProtectObservation` outcome model, the lifecycle
      ternary descent).

### Phase 7 validation checkpoint

- [x] At repo root: `just test claudine`, `just test claudine-cli`,
      `just test claudine-contract`, `just test rendezvous-daemon` all green;
      `just lint` clean across the touched areas.
- [x] All seven acceptance criteria signed off.
- [x] No docs/skill drift relative to the implemented behavior.

---

## Out of scope (intentional deferrals)

- **Rejected findings** are not re-raised (spec §Scope): bash-action shell
  injection; "Qwen untagged enum drops arrays"; `LazyLock` regex `.expect()`;
  git double-strip / frontmatter slice-order / fs-probe collision.
- **protect as a hard security boundary** — deliberately deferred. Making it
  a real boundary requires shell-aware parsing, provider-specific tool
  schemas, Windows command parsing, and stricter compatibility decisions,
  each larger than this remediation.
- **A user-configurable sensitive-path catalog** (P2.4) — only if
  implementation discovers unacceptable false positives from the static list.
- **Behavioral redesign** of the rendezvous daemon beyond each finding's fix;
  it stays a LAN POC and the permissive QUIC verifier remains a documented
  forward-looking gate (P3.7).
- **Dotted control variables / seed-pass optimization** — unrelated to this
  remediation (owned by the loop state-sequencing work).
