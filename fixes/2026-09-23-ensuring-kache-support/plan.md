---
total_phases: 5
created: 2026-09-23
phase: 4
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
source_files_during_phase_1:
    - .github/kache-min-version
    - scripts/kache-host.sh
    - scripts/kache-config-merge.py
    - scripts/fixtures/kache/config-with-cache.toml
    - scripts/fixtures/kache/config-without-cache.toml
    - scripts/fixtures/kache/config-malformed.toml
    - scripts/fixtures/kache/cargo-config.toml
    - tools/test-toolkit/tests/kache_host_contracts.rs
    - tools/test-toolkit/tests/kache_config_merge_contracts.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - fixes/2026-09-23-ensuring-kache-support/spike-doctor-json.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - justfile
    - tools/test-toolkit/tests/kache_recipe_contracts.rs
    - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_3:
    - README.md
    - docs/initialization.md
    - docs/kache-strategy.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/kache/SKILL.md
    - .claude/skills/kache/configuration.md
    - .claude/skills/kache/installation.md
    - .claude/skills/kache/platforms.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/macos.md
    - .claude/skills/rust-devops/kache.md
source_files_during_phase_4:
    - scripts/kache-host.sh
    - tools/test-toolkit/tests/kache_host_contracts.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/kache/installation.md
    - .claude/skills/kache/platforms.md
packages:
    - test-toolkit
---

# Plan: Make kache a host setup that `just init` owns end to end

Implements `fixes/2026-09-23-ensuring-kache-support` (spec: `spec.md`; spikes:
`spike-daemon-ignore-env.md`, `spike-shell-export-location.md`). The spec is
final, reviewed, and carries its 2026-09-23 rulings inline — this plan converts
it into ordered, observable work and adds no new design.

## Work Required and Success Criteria

The fix replaces the OS-name kache decision in `just init` with a
filesystem-qualification probe, makes `init` own kache install, store placement
(user config file as the single source of truth), daemon lifecycle, and
activation — under an explicit order of operations and failure contract — and
repairs `just kache-status`, the contract tests, docs, and skills that describe
the old behavior. It ends with host cleanup on the dev Mac (toolchain
`libLLVM.dylib` symlinks, `~/.env` line 51, daemon plist regeneration) and the
spec's eight verification checks.

Success looks like:

- On a qualifying host (filesystem probe passes), `just init` leaves: kache at
  the latest release (≥ 0.23.0 floor), daemon installed and running, macOS
  binary re-signed ad hoc with the `DYLD_*` env-passthrough probe passing,
  `~/.config/kache/config.toml` carrying `[cache] local_store` +
  `ignore_env = true` via targeted merge, and `[build] rustc-wrapper = "kache"`
  in `$CARGO_HOME/config.toml` — and prints one report block (spec §6).
- On a non-qualifying host, `init` leaves kache off, never installs it, and
  names the reason; a below-floor install triggers the confirmed binary-only
  upgrade path (non-interactive `init` errors instead).
- Any pre-activation failure prints WARNING lines, leaves kache off, and `init`
  completes its other steps (spec §4 failure contract).
- `just kache-status` reads the store from `kache doctor --json`, reports
  env-passthrough and checkout/worktree-base device checks, and fails loudly on
  drift; verdict and report share one probe script with `init`.
- Verification checks 1–8 (spec, Verification) pass on the dev Mac; check 6
  passes on WSL ext4; 1–4 also run on a qualifying Linux host if one is
  available. `just test tools` (contract tests) is green against the new
  justfile; README, `docs/initialization.md`, `docs/kache-strategy.md`, and the
  kache/os skills no longer state the old policy.
- Host cleanup done: toolchain symlinks removed with `just zed-wasm` still
  linking, `~/.env` line 51 deleted (allexport block intact), daemon plist
  regenerated; the abandoned 56 GiB store is reported to Ken, deletion left to
  him.

Not in scope (spec, "Not done here"): upstream entitlement report, CI extension
linking, sweep-wrapper root derivation, tracked `kache.toml` policy.

## Phase 1 — Foundation: rulings, spike, shared scripts, floor

Everything later phases build on. The two heavy spikes are already complete and
recorded in the spec; only one small measurement spike remains.

### Necessary Rules

The spec embeds its 2026-09-23 rulings; these are the implementation-level
decisions it leaves open, with the defaults this plan proceeds on (yolo mode —
apply the default unless Ken overrides):

1. **Probe script name.** Spec says "`scripts/kache-host.sh` or similar" — use
   `scripts/kache-host.sh`, with subcommands (`qualify`, `report`,
   `probe-passthrough`) rather than flags-per-function, so recipes and tests
   call one entry point.
2. **`install-kache` binary-only mode surface.** Spec keeps "an explicit
   binary-only mode (or flag)". Rule: a boolean recipe parameter
   (`just install-kache binary-only=true`), where binary-only = install/upgrade
   + macOS re-sign + passthrough verification and **no** config seeding or
   writes; the default (full) mode adds today's store-cap seed and nothing
   else. Daemon work exists in neither mode (ownership split, spec §3).
3. **CI guard.** Spec: CI never installs or upgrades kache and "if one ever
   does [run `just init`], init must not fight it". Rule: `_ensure-kache`
   detects a CI environment (`CI` or `GITHUB_ACTIONS` set), reports one skip
   line, and exits 0; a contract-test assertion pins that no workflow currently
   runs `just init`.
4. **Interactive confirmation.** "Interactive `init` asks; non-interactive
   stops with an error" (spec §3) — rule: interactivity is `[ -t 0 ]` on stdin;
   non-interactive below-floor on a non-qualifying host exits non-zero from
   `_ensure-kache` after printing the error (consistent with the failure
   contract's loud-but-local pattern, but this path is an error, not a
   WARNING).
5. **`kache doctor --json` field names.** The daemon spike ran 0.23.1;
   `init` will install latest (≥ 0.26.3). Rule: recipes read only field names
   the Phase 1 spike confirms on the version actually installed; if latest
   differs from 0.23.1, the spike's captured shapes win and the recipe task
   adapts.

Tasks:

- [x] **Doctor spike**
    - Capture `kache doctor --json` output shape on the currently installed
      kache (0.23.1) and on the latest release (fetch a pristine binary with
      `cargo binstall` into a temp `CARGO_HOME`/bin location — do not touch the
      host install yet); record the exact fields for: default store path,
      daemon-reported version, daemon running state, and runtime/socket
      location.
    - Record `kache --version` output format and how it compares to the doctor
      `daemon version` field, since the restart trigger compares the two
      (spec §3).
    - Note any 0.23.1 → latest JSON differences; write findings into this
      fix's directory as `spike-doctor-json.md` and reflect them in the recipe
      tasks.
    - Complexity: low risk, but it de-risks every `doctor --json` parse in
      Phases 2–4; verification later runs "against whatever it installs"
      (spec §3), so this spike is the only chance to pin shapes first.
- [x] **Floor bump**
    - `.github/kache-min-version`: `0.15.0` → `0.23.0` (spec Changes; floor is
      the measured 0.23 line — precedence stack and daemon spike ran on
      0.23.1).
    - Confirm the maintenance audit (`.github/workflows/maintenance-audit.yml`)
      needs no change — it reads the file's value verbatim.

Wave 1: **Doctor spike** and **Floor bump** run concurrently (investigation vs
one-line config; no shared files).

- [x] **Probe script**
    - Create `scripts/kache-host.sh` (bash, `set -euo pipefail`, style matched
      to `scripts/sweep.sh` / `cross-check.sh`) extracting the probe logic from
      today's `kache-status` justfile recipe so the verdict and the report
      cannot disagree (spec §1).
    - `qualify`: resolve the candidate store via the placement cascade's pure
      directory logic (macOS: writable `kache/` at the volume mount point, else
      the shared cascade; cascade: user cache dir when on the serving device,
      else a user-owned directory on that volume, else fail with reason —
      never a root-owned mount point); resolve the worktree base the way `wt`
      does (`WT` env, then `~/.worktree.json` `base_dir`) and report it without
      gating the verdict; check device ids (`stat -f %d` / `stat -c %d`) and
      clone-probe from the candidate store into the checkout (and the base when
      configured): `cp -c` + device check on macOS, `cp --reflink=always` on
      Linux, ReFS-by-fs-type on Windows (MINGW/MSYS/CYGWIN branch via
      `powershell.exe`, shipped unexercised per ruling).
    - Machine-readable verdict output (exit code + a stable line format) so
      `_ensure-kache` can branch on it; human lines in the `_ensure-*` style.
    - `report`: everything `kache-status` needs post-activation — store from
      `kache doctor --json` (never a reconstruction; fields from the spike),
      checkout/base device check, env-passthrough result.
    - `probe-passthrough`: the stub-compiler probe — a fake `rustc` that logs
      its environment, run through `RUSTC_WRAPPER=kache` with a `DYLD_*`
      variable exported; pass/fail on whether the variable arrives (spec §3).
    - Pre-requisites: Doctor spike (field names). Windows branch reuses the
      existing `kache-status` ReFS logic verbatim.
- [x] **Config merge helper**
    - Create `scripts/kache-config-merge.py` (python3, stdlib only): the
      targeted, line-oriented TOML merge from spec §2 — set exactly
      `local_store` and `ignore_env` in the `[cache]` table of
      `~/.config/kache/config.toml` (Windows path via `%APPDATA%` handled by
      the caller), preserving every other key and comment; replace an existing
      `key =` line in the table, insert at the table's end, or append a
      `[cache]` table when absent; write to a temp file, validate by parsing
      with `tomllib` (whole file parses; both keys hold the intended values),
      then rename over the original, keeping a dated backup of the previous
      file.
    - Also support the activation write: `[build] rustc-wrapper = "kache"` into
      `$CARGO_HOME/config.toml` under the same merge/validate/backup rules.
    - Validation failure exits non-zero leaving the original untouched (feeds
      the failure contract).
    - Testable in isolation: run it against fixture configs (existing file with
      `[cache]` and comments, file without `[cache]`, malformed file) during
      implementation; fixtures may live under `scripts/fixtures/`.

Wave 2: **Probe script** and **Config merge helper** run concurrently (two new
files; both depend only on Wave 1, not on each other).

## Phase 2 — Root justfile recipes (serialized: one file)

All three tasks edit the root `justfile`; they must not run concurrently. Order
follows the spec's ownership split: `install-kache` first (it has no
dependencies on the other two), then `_ensure-kache` (orchestrates it), then
`kache-status` (independent, serialized only by the shared file).

- [x] **install-kache rework**
    - Always install **or upgrade** to the latest release via `cargo binstall`
      (remove the meets-floor "not reinstalling" skip; the floor is a check,
      not a pin) (spec §3, Changes).
    - Add the boolean binary-only parameter (Necessary Rule 2); both modes:
      on macOS, re-sign the installed binary ad hoc
      (`codesign --force -s -`) immediately after install/upgrade, then verify
      with `kache-host.sh probe-passthrough`; the gate is the verification, and
      source install (`cargo install kache`) is the fallback if re-sign cannot
      produce a passing binary. No daemon work in either mode.
    - Full mode only: keep today's default store-cap seed for a config-less
      host (read-only config dir stays a WARNING, not fatal — build-linux
      precedent). Binary-only mode performs no config writes.
    - Replace the stale recipe comments and the closing epilogue text
      ("Installed, NOT activated…") with the new contract: on qualifying hosts
      `just init` owns activation; by hand, `just kache-status` rules.
    - Pre-requisites: Phase 1 Wave 2 (probe script). Validation: on the dev
      Mac, `just install-kache` upgrades, re-signs
      (`codesign -dvv` → `flags=0x2(adhoc)`), and passes the passthrough probe.
- [x] **_ensure-kache sequence**
    - Replace the OS `case` with `kache-host.sh qualify` (spec §1, §4): a
      non-qualifying verdict ends the sequence — kache absent → never install
      (one-line reason); kache present → floor check, meets-floor → report
      only, below-floor → interactive confirm (`[ -t 0 ]`) runs
      `just install-kache binary-only=true`, non-interactive → stop with an
      error (Necessary Rules 2, 4). CI guard first (Necessary Rule 3).
    - Qualifying path, in order (spec §4): (2) `just install-kache` (full);
      (3) `kache doctor --json` default store feeds placement (spec §2
      cascade; a doctor failure after install makes placement undecidable →
      skip bucket with reason); (4) write the user config via
      `kache-config-merge.py` — always write the ratified `local_store` +
      `ignore_env = true`, even when it equals the default (pin is deliberate);
      (5) macOS re-sign is already verified inside install-kache — re-assert
      the passthrough gate; (6) daemon lifecycle: `kache daemon install` when
      absent, ensure running, restart when the doctor-reported daemon version
      mismatches the installed binary or step (4) changed the config file
      (compare pre/post content, not mtime) — strictly after the config write;
      (7) activation last: `[build] rustc-wrapper = "kache"` in
      `$CARGO_HOME/config.toml` via the merge helper; never a tracked
      `.cargo/config.toml`.
    - Failure contract (spec §4): any failure before activation prints loud
      WARNING lines, leaves kache off (no wrapper write — or the undo when
      activation itself already happened and a later step fails), and `init`
      completes; existing active wrapper on a non-qualifying host is reported
      as drift with the undo printed, left to the human (spec §5).
    - Report one block (spec §6): verdict + deciding fact (device ids, probe
      result), worktree-base line (unconfigured or off-device, with both
      devices named — never a gate), version vs floor, passthrough result,
      store path and whether it moved, old/abandoned store size (reported,
      deletion left to the human), daemon state, activation state.
    - Update the recipe's comment block and the "Activation stays a host
      decision" inter-recipe comment — both are stale the moment this lands
      (repo comment-drift discipline).
    - Pre-requisites: all of Phase 1, install-kache rework. Complexity: highest
      of the plan — the ordering rules and restart triggers are the crux; keep
      each step a small bash function so the failure contract wraps them
      uniformly.
- [x] **kache-status rework**
    - Store from `kache doctor --json` (kills the `KACHE_DIR`/`~/Library/Caches`
      fallback false verdict); add the env-passthrough line
      (`kache-host.sh probe-passthrough`); add the checkout/worktree-base
      device check (base resolved as `wt` resolves it, reported when
      unconfigured) — all via `kache-host.sh report` so status and verdict
      share one implementation (spec Changes).
    - Keep the activation-precedence reporting (env / repo config / Cargo
      home) and the empty-string `RUSTC_WRAPPER=""` neutralization guidance,
      restated for the config-file era (spec §5).
    - Update the stale comment block above the recipe.
    - Pre-requisites: Phase 1 probe script. Validation: on the dev Mac the
      report names `/Volumes/coding/kache` (where `kache doctor` points), not
      the abandoned Data-volume store.
- [x] **Recipe checkpoint**
    - `just --list` and a `--dry-run` parse of `init` (no execution), `bash -n`
      on `scripts/kache-host.sh`, a manual `kache-config-merge.py` run against
      fixtures, and a live read-only `just kache-status` on the dev Mac.
    - Gate: no recipe text asserts the old policy; the three recipes'
      comments match their behavior.

Waves: Wave 1 = install-kache rework; Wave 2 = _ensure-kache sequence; Wave 3 =
kache-status rework; Wave 4 = recipe checkpoint. (Single-file serialization —
no cross-wave concurrency is safe here.)

## Phase 3 — Contract tests, documentation, and skills

All Wave 1 tasks are file-disjoint and run concurrently. Content derives from
the spec; verify each against the landed Phase 2 recipes before finishing.

- [x] **Contract tests**
    - Update `tools/test-toolkit/tests/ci_workflow_contracts.rs`
      (init_installs_the_compiler_cache_without_activating_it and the
      surrounding D1/D2 block) to the new contract (spec Changes): `init` must
      still run `_ensure-kache`; `install-kache` must remain an explicit
      recipe; activation is now permitted **only** inside `_ensure-kache`'s
      ordered sequence (keep forbidding bare `kache init` /
      `export RUSTC_WRAPPER=kache` lines and any tracked
      `.cargo/config.toml` wrapper); install-kache targets latest with the
      floor as a check; add the CI assertion (Necessary Rule 3): no workflow
      runs `just init`, and none installs or upgrades kache
      (`install-kache`, `binstall kache`).
    - Update the 2026-09-09 ruling doc-comments on those tests to cite the
      2026-09-23 spec rulings that supersede them.
    - Validation: `just test tools` (nextest) green with the new justfile.
- [x] **Strategy doc**
    - Rewrite `docs/kache-strategy.md`: repo-integration bullets and the
      2026-09-09 ruling table's install/activation wording → "`init` decides
      from the probe"; fix the Mac table's store path (the store is wherever
      `kache doctor` says it is); record the hardened-runtime/`DYLD_*` finding
      and the re-sign remedy; record the 0.23.0 floor and why; note the
      placement cascade and the config-file-as-single-source-of-truth ruling
      (spec §2).
- [x] **Init docs**
    - `docs/initialization.md`: rewrite "Build Caching" (Installing / Probing
      before activating / Activating) and "Platform Behavior" to the new
      init-owned flow — probe, install, config write, daemon, activation last,
      failure contract, non-qualifying paths.
    - `README.md` (~line 93): replace "installed by `just init` on macOS and
      Linux but never activated" with the probe-decided, init-owned summary.
- [x] **Kache skill**
    - `.claude/skills/kache/`: replace the 2026-09-09 ruling section in
      SKILL.md (init now owns placement, activation, daemon lifecycle);
      correct the store-location table in configuration.md to "`kache doctor`
      is the authority" plus the verified precedence stack (env → user config
      `local_store` with `ignore_env = true` → cwd `kache.toml` (real,
      lowest-priority, cannot carry the store path or gate env) → default;
      `KACHE_DISABLED` ungated); add the hardened-runtime/`DYLD_*` trap and
      ad hoc re-sign to installation.md; the 0.23.0 floor and rationale.
- [x] **OS skill**
    - `.claude/skills/os/macos.md`: short entry for the hardened-runtime
      `DYLD_*`-stripping trap (symptom: the `rust-lld`/`libLLVM.dylib` dyld
      abort through kache), pointing at the kache skill — per the AGENTS.md
      rule that OS traps learned the hard way land in that skill in the same
      change.

Wave 2 checkpoint:

- [x] **Contracts checkpoint**
    - `just test tools` green end-to-end; drift scan over the repo for stale
      claims (`rg -n "never activated|never reinstalling|KACHE_DIR"` over
      docs/, README, skills, justfile comments) — every hit is either updated
      or deliberately historical (dated ruling records stay).

## Phase 4 — Dev-Mac verification and host cleanup

The spec's eight verification checks, interleaved with the host cleanup that
some of them require. Dev-Mac tasks mutate one host and run serially; the WSL
negative-host task rides in Wave 1 on a different machine. Load the `os` skill
for host reach before claiming any host is unavailable.

- [x] **Host pre-cleanup** (dev Mac)
    - Remove the hand-made `libLLVM.dylib` symlinks from the `1.98.1` and
      `stable` toolchains (spec Changes; required by check 1).
    - Delete exactly `~/.env` line 51 (`KACHE_CACHE_DIR=...`) and nothing
      else — the `set -a` allexport block in `adaptive.sh` must survive
      (`spike-shell-export-location.md`); verify with a fresh interactive
      shell that `KACHE_CACHE_DIR` is unset and every other `~/.env` variable
      still exports.
    - Do NOT regenerate the daemon plist yet — check 8 needs its stale
      `KACHE_CACHE_DIR` (or re-sets it by hand later).
- [x] **WSL negative host**
    - Spec check 6 on the WSL ext4 host: `just init` leaves kache off, never
      installs it, names the reason; `kache-status` agrees.
    - With a below-0.23.0 kache installed there: non-interactive `init` stops
      with an error; an interactively confirmed upgrade is binary-only (no
      daemon installed or restarted on that host).
    - If the host is unreachable, record that and the attempt per the os
      skill; the APFS cross-device construction from check 6 may substitute on
      the dev Mac only if WSL is truly unavailable.

Wave 1: **Host pre-cleanup** (dev Mac) and **WSL negative host** run
concurrently — different hosts.

- [x] **First init + core checks** (dev Mac; after Wave 1)
    - Run `just init`; capture the one-block report (spec §6). Check 1:
      `just zed-wasm` in `darkmatter` links through kache with the pristine
      toolchain. Check 2: passthrough probe passes on the installed binary and
      **fails** on a pristine binstall release fetched to a temp location
      (proves the probe can fail). Check 5: a second `just init` changes
      nothing, restarts no daemon, reports the same verdict.
- [x] **Lifecycle parity** (dev Mac)
    - Check 3: `wt` create → build a package → record store size → destroy →
      fresh worktree → rebuild; rebuild served from the store, size stable
      apart from index/event-log churn, `kache doctor`/`kache stats` clean,
      nothing kache-specific run in between.
    - Check 4: launcher parity — `launchctl` one-shot or `env -i` with only
      `HOME`/`PATH` resolves the same store as an interactive shell; no new
      files appear under `~/Library/Caches/kache` (the `~/.env` deletion from
      Wave 1 is what makes this true).
- [x] **Daemon confirmation + plist** (dev Mac)
    - Check 8: with the plist still carrying (or re-set with)
      `KACHE_CACHE_DIR`, the running daemon resolves the same store as the
      CLI; the first `init` restarted it on the config-change trigger; from a
      fresh shell `kache daemon` (status) reaches the daemon at its new socket
      location.
    - Then regenerate the plist (`kache daemon uninstall && install`) —
      cleanup hygiene per the spike — and confirm the raw-env probe drips into
      the old store have stopped.
    - Report the abandoned 56 GiB `~/Library/Caches/kache` store to Ken;
      deletion stays with him.
- [x] **Failure contract drill** (dev Mac)
    - Check 7: downgrade kache below 0.23.0, run `just init` — upgrades to
      latest, re-signs, restarts the daemon, check 2 passes again. Then force
      a pre-activation failure (stop the daemon; place a regular file at its
      expected socket path so the restart fails) — WARNING lines, kache left
      off, `init` completes its other steps.

Waves: Wave 1 = pre-cleanup ∥ WSL negative host; Wave 2 = first init + core
checks; Wave 3 = lifecycle parity; Wave 4 = daemon confirmation + plist; Wave
5 = failure contract drill. (All dev-Mac waves share the daemon and store —
strictly serial.)

- [x] **Linux qualifier** (conditional, any wave after Wave 1)
    - Spec checks 1–4 on one qualifying Linux host (btrfs/XFS-reflink/ZFS), if
      one is available per the os skill's host map; otherwise record the
      unavailability in the fix directory — the spec marks this host
      "if one is available", so absence blocks nothing.

## Phase 5 — Closure and readiness

- [ ] **Final validation**
    - `just test tools` green; `bash -n` clean on new/edited scripts; the
      Phase 3 drift scan re-run clean; `git status`/`git diff` reviewed for
      comment/code coherence in every edited recipe (behavior-changing edits
      carry their comment updates — repo rule).
    - Summarize against the spec's Verification list: each of the 8 checks
      with where it ran and its result (or why unavailable), plus host
      cleanup state.
- [ ] **Ready for review**
    - Set the spec frontmatter `implemented: true` and `implemented_by` per
      repo convention; terminal state is "implementation complete, ready for
      review" — the author moves the fix to `_completed`; the agent never
      runs `just complete`.
