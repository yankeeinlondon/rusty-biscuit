---
spec: /Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/spec.md
plan: fixes/2026-09-23-ensuring-kache-support/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
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
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
    - .github/kache-min-version
    - justfile
    - scripts/kache-host.sh
    - scripts/kache-config-merge.py
    - scripts/fixtures/kache/config-with-cache.toml
    - scripts/fixtures/kache/config-without-cache.toml
    - scripts/fixtures/kache/config-malformed.toml
    - scripts/fixtures/kache/cargo-config.toml
    - tools/test-toolkit/tests/kache_host_contracts.rs
    - tools/test-toolkit/tests/kache_config_merge_contracts.rs
    - tools/test-toolkit/tests/kache_recipe_contracts.rs
    - tools/test-toolkit/tests/ci_workflow_contracts.rs
documentation:
    - README.md
    - docs/initialization.md
    - docs/kache-strategy.md
    - fixes/2026-09-23-ensuring-kache-support/spike-doctor-json.md
    - .claude/skills/kache/SKILL.md
    - .claude/skills/kache/configuration.md
    - .claude/skills/kache/installation.md
    - .claude/skills/kache/platforms.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/macos.md
    - .claude/skills/rust-devops/kache.md
completed_phase: 5
implemented: true
packages:
    - test-toolkit
---

# Implementation Log for 2026-09-23-ensuring-kache-support (5 phases)

## Phase 1

### Wave 1 — Doctor spike + Floor bump (complete)

**Doctor spike** → `fixes/2026-09-23-ensuring-kache-support/spike-doctor-json.md`

- Captured `kache doctor --json` on the installed 0.23.1 (host, read-only) and
  on pristine 0.26.3 fetched via `cargo binstall` into a temp `CARGO_HOME`,
  run against scratch `KACHE_CONFIG`/`KACHE_CACHE_DIR`/`KACHE_RUNTIME_DIR`
  and a scratch foreground daemon. Host install, config, store, and launchd
  never touched; verified afterwards (daemon pid 1622 unchanged, no stray
  processes, scratch dirs removed).
- Key findings recorded in the spike doc:
  - Store path = doctor `checks[label == "Cache dir"].detail` — same on both
    versions.
  - Daemon version: doctor `Daemon version` detail is `v0.26.3 (epoch …)`
    (v-prefixed) when reachable, `"daemon not reachable"` with `pass:false`
    when not; `kache daemon --json` is the cleaner source —
    `daemon_running` (bool), `daemon_version` (**bare** `"0.26.3"`),
    `socket`, `daemon_config_path`, `service_executable_mismatch`.
  - `kache --version` → `kache 0.26.3`; doctor `.version` and daemon-status
    `.daemon_version` are bare and directly comparable — the restart trigger
    should use `daemon --json` `daemon_version` vs doctor `.version`.
  - 0.23.1 → 0.26.3: new top-level `next` array, new `Service exe` check,
    `Daemon processes` is emit-conditional; no field the plan names changed
    name or format.

**Floor bump** — `.github/kache-min-version`: `0.15.0` → `0.23.0`.
Confirmed `.github/workflows/maintenance-audit.yml` needs no change (reads
the file verbatim at its line 62 `tr -d '[:space:]' <.github/kache-min-version`).
The remaining `0.15.0` references in `docs/kache-strategy.md` are the Phase 3
strategy-doc task, not this one.

### Wave 2 — Probe script + Config merge helper (complete)

**Probe script** → `scripts/kache-host.sh` (created, executable)

Subcommands `qualify` / `report` / `probe-passthrough`; machine-readable
facts on `kache-host: key=value` lines; exit 0/1/2 = yes/no/usage-or-error
(qualify), produced/cannot (report), pass/stripped/cannot-run
(probe-passthrough).

- `qualify` implements the spec §1/§2 cascade: macOS mount-point `kache/`
  first, then the one shared cascade (user cache dir when on the serving
  device, else a user-writable `kache/` at the checkout volume's mount
  point, else `no-user-writable-store-location-on-the-checkout-volume`).
  The clone probe is the verdict — device identity alone is not proof
  (ext4). The worktree base resolves exactly the way `wt` does (`WT` env,
  then `~/.worktree.json` `base_dir`, existing-dir check — verified against
  `worktree/lib/src/config.rs`) and is reported (`covered` / `off-device` /
  `unconfigured`), never gating. A candidate the cascade created is removed
  while still empty when the verdict is no.
- `report` reads the store from `kache doctor --json`
  (`checks[label == "Cache dir"].detail`, per the spike), reports devices,
  base, and the passthrough result; kache absent → `error=kache-absent`
  exit 2; doctor failure → `error=doctor-failed` exit 2.
- `probe-passthrough` — the design changed twice during implementation, and
  the findings matter for Phases 2 and 4:
  1. A bash-stub observer can NEVER see `DYLD_*`: bash expunges `DYLD_*`
     from its own environment at startup (verified with an ad hoc re-signed
     copy of /bin/bash — still stripped; it is bash, not dyld, doing it).
  2. A plain copy of a platform binary (/usr/bin/env, /usr/bin/printenv) is
     KILLED by AMFI when executed outside the system volume (exit 137), so
     copied-env fakes must always be ad hoc re-signed.
  3. Working design: the observer is an ad hoc re-signed copy of
     `/usr/bin/printenv` standing in for `rustc`, and the variable NAME
     rides in as the compiler argument — `kache rustc
     DYLD_FALLBACK_LIBRARY_PATH` makes the wrapped "compiler" print the
     value exactly when it survived kache (kache rejects a bare `kache
     rustc` with no args, which ruled out the no-arg `env` dump). A control
     invocation with a non-DYLD variable name separates "stripped" from
     "stub never ran".
  - Validated live on the dev Mac through the script's own interface:
    installed hardened 0.23.1 → fail (exit 1); ad hoc re-signed 0.23.1 →
    pass; pristine 0.26.3 → fail; ad hoc re-signed 0.26.3 → pass. This is
    spec verification check 2's machinery, proven in both directions.
- `KACHE_HOST_BIN` overrides which kache binary the probe runs (the plan's
  Phase 4 pristine-specimen check needs it).

**Config merge helper** → `scripts/kache-config-merge.py` (created,
executable; python3, stdlib only)

`FILE TABLE KEY VALUE [KEY VALUE ...]` — one helper for both writes
(`cache local_store <path> ignore_env true` and `build rustc-wrapper
kache`). Line-oriented merge confined to the target table (replace an
existing `key =` line in place, insert at the table end before boundary
blank lines, append the table when absent), CRLF and trailing-newline
conventions preserved, temp file → `tomllib` validation (whole file parses,
each requested key holds its intended typed value) → dated backup
(`FILE.bak-YYYYMMDD-HHMMSS`, collision-countered) → `os.replace`. Rejected
writes (malformed file, hostile table/key names) exit non-zero leaving the
original untouched and no temp file behind (the `SystemExit` from
validation had to be caught as `BaseException` or the temp file leaked).
A no-op rewrite (byte-identical) reports `already holds` and adds no
backup — that is what Phase 2's config-changed daemon-restart trigger
compares.

Fixtures under `scripts/fixtures/kache/`: `config-with-cache.toml`
(hand-tuned `[cache]` with comments and a stale `local_store`),
`config-without-cache.toml`, `config-malformed.toml`, `cargo-config.toml`.

**Tests** (all `just test tools` green: 365 passed; lint green)

- `tools/test-toolkit/tests/kache_host_contracts.rs` (new): floor file
  well-formed and ≥ 0.23.0; `bash -n` on the script; `qualify` prints the
  stable `kache-host: verdict=` line with exit-code agreement (format-only
  assertions — the verdict itself is a host property); macOS-only
  probe-passthrough discrimination in both directions (bash `exec "$@"`
  fake = natural DYLD stripper; ad hoc re-signed env copy = forwarder);
  non-macOS `n/a` line; `report` names kache absence (exit 2); unknown
  subcommand is a usage error.
- `tools/test-toolkit/tests/kache_config_merge_contracts.rs` (new): passive
  corpus over every shipped fixture; in-place merge with comment/key
  preservation and exactly one dated backup; repeated write is a
  byte-identical no-op with no second backup; missing table appended;
  Cargo activation write; malformed rejected untouched (no backup, no temp
  litter); absent file created with parents; value variants (spaces in
  path, boolean false); CRLF preserved (byte-counted); hostile names
  rejected; odd args exit 2.

### Requirement → test mapping (Phase 1)

| Behavior | Test(s) |
|---|---|
| Floor file is a well-formed version ≥ 0.23.0 | `kache_floor_version_file_is_wellformed` |
| Script stays parseable bash | `kache_host_script_parses` |
| Verdict line format + exit agreement | `qualify_prints_a_stable_verdict_line_that_matches_its_exit_code` |
| Passthrough probe detects stripping (macOS) | `probe_passthrough_fails_when_the_wrapper_strips_dyld` |
| Passthrough probe passes a forwarder (macOS) | `probe_passthrough_passes_when_the_wrapper_forwards_dyld` |
| Passthrough n/a off macOS | `probe_passthrough_is_not_applicable_outside_macos` |
| Report names kache absence | `report_names_kache_absence` |
| Unknown subcommand rejected | `unknown_subcommand_is_a_usage_error` |
| Merge preserves keys/comments + backup | `existing_cache_table_is_merged_in_place_with_a_dated_backup` |
| Merge idempotence (round trip) | `repeated_write_is_a_byte_identical_no_op` |
| Missing table appended | `missing_cache_table_is_appended` |
| Activation write | `cargo_activation_write_preserves_build_table` |
| Malformed rejected untouched | `malformed_file_is_rejected_untouched` |
| Absent file created | `absent_file_is_created_with_parent_directories` |
| Value variants | `value_variants_round_trip` |
| CRLF preserved | `crlf_endings_are_preserved` |
| Corpus holds shape | `corpus_every_fixture_holds_its_shape_through_the_kache_write` |
| Hostile names / usage | `hostile_names_and_bare_arguments_are_rejected`, `odd_argument_count_is_a_usage_error` |

Live (non-automated) validation on the dev Mac, this phase: qualify from
the repo (qualify, candidate `/Volumes/coding/kache`, base covered);
qualify from a Data-volume scratch checkout (qualify via the user-cache-dir
cascade step); cascade-fail negative (cross-device HOME, exit 1 with the
cascade reason); report against the real host (store `/Volumes/coding/kache`
from doctor — the correct store, not the old reconstruction); passthrough
probe against four binary states as listed above.

## Phase 2

All three recipes reworked in the root `justfile` (serialized waves), plus a
new contract-test file and a one-assertion bridge edit to keep the existing
D1/D2 contract green until Phase 3 reworks it.

### Wave 1 — install-kache rework

- Signature is now `install-kache binary_only="false":`. **Deviation from the
  plan's Necessary Rule 2 spelling**: just 1.56 has no named-argument syntax
  (`just install-kache binary-only=true` binds the literal string positionally
  — measured), and dashes are not even parseable in parameter names. The mode
  is a positional boolean: `just install-kache true` is binary-only. A mistyped
  value is a usage error (exit 2) that teaches the correct form.
- Always latest via plain `cargo binstall --no-confirm kache` — **no
  `--force`**: binstall itself no-ops when the latest is already installed
  (measured: "already installed, use --force to override"), which keeps a
  second init download-free; the meets-floor "not reinstalling" skip is gone.
- macOS gate restructured to **probe-first**: the passthrough probe runs
  before any re-sign. An already re-signed binary passes and is left
  untouched; a fresh binstall release is hardened, fails, and is re-signed;
  if the re-signed binary still fails, the fallback runs
  `cargo install --locked --force kache` (`--force` measured necessary:
  without it cargo install no-ops "already installed" and the fallback
  replaces nothing). Rationale beyond the spec's "the gate is the
  verification": an unconditional `codesign --force` **rewrites the binary
  file every init**, and the running daemon exits whenever its executable is
  replaced (launchd relaunches it, throttled ~10 s) — unconditional re-sign
  would flap the daemon on every `just init` and break verification check 5.
  Probe-first keeps same-version inits side-effect-free; no upgrade path can
  skip the re-sign because a fresh binstall binary always fails the probe.
- Full mode keeps today's store-cap seed verbatim (read-only config dir stays
  a WARNING); binary-only mode performs no config writes. No daemon work in
  either mode.

### Wave 2 — _ensure-kache sequence

- CI guard first (`CI`/`GITHUB_ACTIONS` → one skip line, exit 0), then
  `scripts/kache-host.sh qualify` replaces the OS `case`. Non-qualifying:
  reason line, floor check (meets → report only; below → `[ -t 0 ]`
  interactive confirm runs `just install-kache true`, non-interactive →
  ERROR + exit 1), active-wrapper drift report with both undos, exit 0.
- Qualifying path implements spec §4 steps (2)–(7) with `kache_off()`
  wrapping every step (WARNING lines, exit 0 so init completes). The undo in
  `kache_off` writes `rustc-wrapper = ""` through the merge helper — measured
  first: Cargo treats `rustc-wrapper = ""` in a config file as no-wrapper
  (control with a marker wrapper proved invocation), so the neutralized form
  is a validated, backed-up write.
- Placement (step 3) runs `kache-host.sh report` and compares devices:
  doctor-resolved store on the serving device → that IS the store ("pinned
  deliberately"); otherwise the qualify candidate. Doctor failure → skip
  bucket (`error=` reason). Verified fresh-host safety first: doctor's
  `Cache dir` check passes and carries the resolved path even when the store
  directory does not exist (scratch `KACHE_CONFIG`/`KACHE_CACHE_DIR`/fresh
  `HOME` probes), so no probe-script change was needed.
- Config write (step 4) via `kache-config-merge.py ... cache local_store
  "$store" ignore_env true`; the daemon-restart trigger compares pre/post
  file CONTENT (`cmp` against a snapshot), not mtime.
- Daemon lifecycle (step 6) reads `kache daemon --json` (spike
  recommendation): `daemon_running`, bare `daemon_version`, `service_installed`,
  `socket`. Restart when version mismatches OR config changed; install when
  absent; start + wait otherwise. Waits are 10 s then (after an explicit
  `kache daemon start`) 20 s — sized for launchd's ~10 s relaunch throttle
  discovered during the drill below.
- Activation (step 7) last, via the merge helper into `$CARGO_HOME/config.toml`.
- Report block (spec §6): verdict+devices, worktree-base line (covered /
  off-device / unconfigured), version vs floor, passthrough, store + moved,
  old/abandoned store size (`du -sh` of the built-in default cache dir when
  it differs and is non-empty — reported, never deleted), daemon state,
  activation state.
- **Bash trap fixed during implementation**: an apostrophe inside
  `${var:-word}` in a double-quoted string is a parse error
  (`${doctor_store:-kache's default}` → "unexpected EOF while looking for
  matching quote"); reworded. Embedded python one-liners must be indented to
  recipe-body level (just strips the common indent; column-0 lines end the
  recipe body).

### Wave 3 — kache-status rework

- Store/devices/base/passthrough all come from `scripts/kache-host.sh
  report` (store from `kache doctor`, never the old `KACHE_DIR`→
  `~/Library/Caches` reconstruction — that string is gone from the justfile).
- Activation-precedence reporting kept (env / repo config / Cargo home);
  drift while active now FAILS LOUDLY (exit 1) listing each problem: kache
  absent but wrapped, doctor undecidable, store off-device, worktree base
  off-device, passthrough fail/error. The `export RUSTC_WRAPPER=""` undo
  guidance is restated for the config-file era (the empty value overrides
  the config file — measured), plus the windows_hardlink/
  storage_layout_advice warning on the copy-restore drift.

### Live validation on the dev Mac (all read-only unless noted)

- `just kache-status` BEFORE any mutation: named the real store
  `/Volumes/coding/kache` from doctor (not the abandoned one), devices
  aligned, and correctly reported **DRIFT exit 1** — passthrough fail on the
  then-hardened 0.23.1. After install-kache: healthy verdict, exit 0.
- `just install-kache` (mutating, per the plan's Validation line): upgraded
  0.23.1 → 0.26.3, re-signed (`codesign -dvv` → `flags=0x2(adhoc),
  Signature=adhoc`), probe pass. Second run: binstall no-op, probe pass, no
  writes.
- `_ensure-kache` non-qualifying paths via a Data-volume worktree with a
  cross-device HOME (Phase 1's construction): reason named + meets-floor
  "report only" (exit 0); below-floor shim (`kache --version` → 0.15.0) +
  non-interactive → ERROR, exit 1; kache-absent PATH → "not installed"
  line, exit 0. CI=1 → guard line, exit 0.
- Full qualifying run: config written with dated backup (hand-tuned [cache]
  preserved), config-change daemon restart fired
  ("kache: restarting the daemon — config changed."), kache's own log line
  confirmed `ignore_env` honored ("ignoring set env override(s)
  [\"KACHE_CACHE_DIR\"] in favor of the config file"), activation no-op
  ("already holds"), report block complete (old store reported: "56G at
  /Users/ken/Library/Caches/kache — abandoned; deleting it is left to you").
- Idempotence (check-5 mechanics): immediate second run — same daemon PID,
  every write an "already holds" no-op, no restart, rc 0.
- Failure-contract drill (via `KACHE_HOST_BIN=/nonexistent`): drove
  install-kache through re-sign → source-fallback attempt → exit 2, then
  `_ensure-kache` printed WARNING lines, **undid the real activation**
  (`rustc-wrapper = ""`, backup kept), left kache off, and completed rc 0;
  one further normal run restored `rustc-wrapper = "kache"`. This drill is
  what found the daemon-flap-on-resign behavior and the fallback's missing
  `--force`.

### Host state after Phase 2 (for Phase 4)

kache 0.26.3 ad hoc re-signed at `~/.cargo/bin/kache`; daemon running 0.26.3
with `daemon_config_path` = the user config; `~/.config/kache/config.toml`
now carries `local_store = "/Volumes/coding/kache"` + `ignore_env = true`
(backups `.bak-20260923-1826*`/`183*` beside it); activation restored in
`~/.cargo/config.toml`. Still pending (Phase 4 by design): toolchain
libLLVM symlinks, `~/.env` line 51, daemon plist regeneration. Note:
`daemon --json`'s `daemon_epoch` does not track process restarts reliably —
use `daemon_version`, as the recipes do.

### Tests

New: `tools/test-toolkit/tests/kache_recipe_contracts.rs` (10 tests,
text-contract style matching `ci_workflow_contracts.rs` — the recipes are
host-mutating and cannot run inside a test; each test fails against the old
justfile text and passes against the new). Bridge edit in
`ci_workflow_contracts.rs`: the `install-kache:` assertion now accepts the
`install-kache binary_only=` signature, with a comment noting the 2026-09-23
spec supersedes the 2026-09-09 ruling and that Phase 3 reworks the block.

Gates: `just test tools` → 376 passed, 2 skipped (the skips are the
intentional `nextest_config_verification` fixtures, pre-existing); `just
lint` (tools area) green; `bash -n` on `scripts/kache-host.sh` clean; `just
--list` and `just --dry-run init` parse. An accidental whole-workspace
nextest run during debugging showed 2 pre-existing claudine failures
(`shipped_implement_prompts_have_not_drifted_from_their_fixture`, one leaky
composition test) — unrelated to this change, outside this phase's gates,
left as found.

### Requirement → test mapping (Phase 2)

| Behavior | Test(s) |
|---|---|
| Installer targets latest, binary-only mode, no reinstall skip | `install_kache_targets_latest_in_a_binary_only_mode` |
| Re-sign + probe gate + source fallback, probe gates BEFORE re-sign | `install_kache_resigns_and_gates_on_macos_with_source_fallback` |
| Installer owns the binary only (no daemon; seed is full-mode only) | `install_kache_owns_the_binary_and_nothing_else` |
| init decides via the shared probe, never OS names | `ensure_kache_decides_through_the_shared_probe` |
| CI guard before anything that installs/upgrades | `ensure_kache_steps_aside_in_ci_environments` |
| Spec §4 ordering incl. activation last; ratified config pair | `ensure_kache_runs_the_spec_section_4_order` |
| Below-floor upgrade: interactive confirm, binary-only; non-interactive errors | `ensure_kache_upgrades_below_floor_only_when_confirmed` |
| Failure contract: kache_off routing, undo via empty wrapper, init continues | `ensure_kache_failure_contract_leaves_kache_off` |
| One spec §6 report block incl. abandoned-store reporting | `ensure_kache_ends_with_the_one_report_block` |
| Status shares the probe, no KACHE_DIR reconstruction, drift fails loudly | `kache_status_shares_the_probe_and_fails_loudly_on_drift` |
| No stale policy text anywhere in the justfile | `justfile_carries_no_stale_kache_policy_text` |
| init still wires `_ensure-kache`; installer stays an explicit recipe | `ci_workflow_contracts::init_installs_the_compiler_cache_without_activating_it` (bridged) |

## Phase 3

### Resumed state

An earlier Phase 3 run had already landed the first three Wave 1 tasks
(checked off): contract tests in `ci_workflow_contracts.rs`, the
`docs/kache-strategy.md` rewrite, and the `docs/initialization.md` + README
rewrite. It had also partly done the kache skill (SKILL.md ruling section,
configuration.md store table and precedence stack) but wrote no log entry.
This run verified that work against the landed Phase 2 recipes and finished
the rest.

### Wave 1 — remaining tasks

**Kache skill** (`.claude/skills/kache/`)

- `installation.md`: replaced the stale "installs on macOS and Linux, never
  reinstalling / activation is a separate step" paragraph with the
  probe-decided flow. It documents both `install-kache` forms (positional
  `true` = binary-only, as the recipe actually accepts) and the **0.23.0
  floor and its rationale** (the measured line; a check, not a pin). Added a
  new section **"macOS: the hardened runtime strips `DYLD_*` — re-sign ad
  hoc"**: symptom text, root cause, the `codesign --force -s -` remedy with
  the `flags=0x2(adhoc)` check, the source-install fallback, probe-first
  gating, and rejected workarounds (toolchain symlinks, recipe `DYLD_*`
  exports). Rewrote "In this repository": init is the only activator, NTFS
  does not qualify while ReFS can, `RUSTC_WRAPPER=""` opts one command out,
  and what `kache-status` now reports. The uninstall paragraph now points at
  `kache doctor` for the store, not a per-OS table.
- `platforms.md`: the macOS store line is the *default*; `kache doctor`
  names the store actually in use.
- `SKILL.md`: the key-references entry for installation.md names the trap.
  The ruling section from the earlier run was checked against the justfile
  and is accurate (positional `true`, 0.23.0, `RUSTC_WRAPPER=""`).

**OS skill**: `.claude/skills/os/macos.md` gets a "Host conditions that look
like repo failures" entry for the `libLLVM.dylib` / LLD SIGABRT symptom,
pointing at the kache skill. The `os/SKILL.md` index line for macos.md
mentions it.

### Wave 2 — Contracts checkpoint

- **Lint found a Phase 3 defect**: clippy `collapsible_if` in the new
  `justfile_corpus()` helper (`ci_workflow_contracts.rs`). The earlier run had
  not run `just lint`. Fixed with a let-chain; `just lint` (tools) is now
  green.
- **Negative proof for the new contracts**: temporarily appended
  `- run: just init` to `.github/workflows/_area-ci.yml` and a
  `kache init -y` recipe line to `just/devops.just`.
  `ci_never_runs_init_nor_installs_or_upgrades_kache` and
  `init_owns_the_kache_host_setup_through_the_ensure_step` both FAILED with
  their intended messages. The imported-file case proves `justfile_corpus`
  follows `import`s. Both files were restored from backups, and `git status`
  is clean for them.
- **Drift scan** (`rg "never activated|never reinstalling|never reinstalls|
  KACHE_DIR|not reinstalling|NOT activated|kache init"` over docs/, README,
  .claude/skills/, justfile):
  - `.claude/skills/rust-devops/kache.md` "In this repo" still stated the
    2026-09-09 ruling ("macOS on / Windows and WSL off … never
    reinstalling"). It was not in the plan's file list; rewritten to the
    2026-09-23 init-owned summary.
  - Deliberately historical, left as-is: the justfile comment above
    `kache-status` ("the pre-2026-09-23 recipe guessed from KACHE_DIR") and
    `docs/kache-strategy.md:26` (records that the "not reinstalling" skip
    is gone).
  - Generic upstream `kache init` usage in the kache skill (quick reference,
    Post-install) and rust-devops/kache.md is vendor documentation, not repo
    policy. The repo rule against running it lives in installation.md
    "In this repository".
- Docs agree with the recipe on the positional `just install-kache true`
  form (docs/kache-strategy.md, docs/initialization.md, kache skill).
- Implementation-log bookkeeping: its metadata block sat as plain text under
  the H1, not as YAML frontmatter. Wrapped it in `---` delimiters at the top
  of the file (values unchanged) so the required frontmatter properties can
  be parsed.

### Gates

- `just test` (tools area): 377 passed, 2 skipped. The skips are
  pre-existing `#[ignore]` fixtures
  (`nextest_config_verification::cargo_nextest_flags_slow_test_in_output`,
  `slow_fixture_for_nextest_verification`).
- `just lint` (tools area): green after the collapsible-if fix.
- No OS-specific code changed in this phase (tests read repository text
  only), so no cross-check run: CI's Linux/macOS legs cover it.

### Requirement → test mapping (Phase 3)

| Behavior | Test(s) |
|---|---|
| `init` still runs `_ensure-kache`; installer stays an explicit recipe with `binary_only` | `ci_workflow_contracts::init_owns_the_kache_host_setup_through_the_ensure_step` |
| No bare `kache init` / `export RUSTC_WRAPPER=kache` in the justfile or any import | same test (proven failing on a planted `just/devops.just` violation) |
| Non-empty wrapper activation write only inside `_ensure-kache` | same test |
| Installer targets latest; floor is a check (no version pin literal) | `ci_workflow_contracts::kache_has_a_single_version_floor` |
| No workflow runs `just init` or installs/upgrades kache (Necessary Rule 3) | `ci_workflow_contracts::ci_never_runs_init_nor_installs_or_upgrades_kache` (proven failing on a planted workflow step) |
| No tracked wrapper / CI kache wiring | `ci_workflow_contracts::ci_does_not_wire_the_kache_wrapper` (unchanged) |
| Docs/skills no longer state the old policy | drift scan (manual; documentation is not test-pinned) |

## Phase 4

### Wave 1 — Host pre-cleanup (dev Mac)

- The hand-made symlinks were not beside `rust-lld` (`…/rustlib/aarch64-apple-darwin/bin/`)
  but one level over, at
  `~/.rustup/toolchains/{1.98.1,stable}-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libLLVM.dylib -> ../../../libLLVM.dylib`
  (the stable one dated 2026-09-05; the 1.98.1 one 2026-09-23 14:47, made
  during the original diagnosis). Both removed after confirming they were
  symlinks (`[ -L ]`); the real `<toolchain>/lib/libLLVM.dylib` files are
  untouched. No other `libLLVM*` exists under any toolchain's `rustlib/`.
- `~/.env` line 51 (`KACHE_CACHE_DIR=/Volumes/coding/kache`) deleted after
  asserting the line's exact text first; 54 → 53 lines, `diff` shows only
  that line. A backup is kept at `~/.env.bak-20260923-kache` (mode 600,
  because the file holds credentials). `~/.config/sh/adaptive.sh` untouched.
- Verified with a fresh interactive zsh under `env -i` (`zsh -i -c`):
  `KACHE_CACHE_DIR=[]`, and all 44 assignments in `~/.env` still arrive
  exported (`printenv` per key, 0 missing), so the allexport block survived.
- Daemon plist deliberately NOT regenerated yet (check 8 needs its stale env).

### Wave 1 — WSL negative host (`build-win`, spec check 6)

Method: `git archive HEAD` of the recipe inputs (`justfile`, `just/`,
`scripts/cargo-path.sh`, `scripts/kache-host.sh`,
`scripts/kache-config-merge.py`, `.github/kache-min-version`) unpacked into a
scratch git repo on the guest's ext4 root volume, run with a scratch
`HOME`/`CARGO_HOME`/`XDG_*` (real `RUSTUP_HOME`, real `~/.cargo/bin` tools on
PATH) so the build host's own install could not change. I ran
`just _ensure-kache` (the `init` step) instead of the whole `init`, which
would have reinstalled every host tool into the scratch home. The contract
tests pin that `init` calls `_ensure-kache`.
The guest has no kache installed. The scratch directory was removed afterwards,
and the guest's real home still has no kache and no kache systemd unit.

- **First attempt, under `/tmp`**: the verdict was `clone-unsupported-on-tmpfs`.
  The guest's `/tmp` is tmpfs, so this was not the ext4 case, and the scratch
  directory moved to `~`.
- **DEFECT FOUND AND FIXED — the placement cascade and a fresh home.** On ext4 with a
  scratch home that had no `~/.cache`, the verdict was
  `no-user-writable-store-location-on-the-checkout-volume` rather than the
  clone-probe failure. `candidate_store` decided the user cache dir's device
  from its *immediate parent* and skipped the step when that parent did
  not exist. It then fell through to the root-owned mount point (`/kache`) and
  failed. The same thing happened on the dev Mac for a Data-volume checkout
  with a home that has no `~/Library/Caches`. The consequence is that a
  fresh or minimal home on btrfs/XFS/APFS would be told it does not qualify.
  A second, smaller defect: on a non-qualifying host the cascade created
  `~/.cache/kache` and never removed it.
  Fix (`scripts/kache-host.sh`): the nearest *existing* ancestor now decides the
  device, so nothing is created before the device is known. A cache dir that the
  cascade creates is recorded (`CREATED_CANDIDATE` plus the outermost created
  directory, `CREATED_TOP`), and `drop_created_candidate` removes that chain
  while it is still empty on a no verdict. Regression test:
  `qualify_places_the_candidate_in_a_fresh_home_and_cleans_up_on_no`. It
  failed on macOS against the old script with the exact reason above, and
  passes after the fix. On macOS it exercises the qualify branch: the
  candidate is under the fresh home. On Linux ext4 it exercises the no-qualify
  cleanup branch: the home is left untouched.
- **After the fix, on ext4** (fresh home, then a home with `~/.cache`):
  `kache-host: verdict=no-qualify reason=clone-unsupported-on-ext4` →
  "kache: this filesystem does not earn kache (clone-unsupported-on-ext4);
  kache stays off." / "not installed — init leaves it that way", rc 0. The
  scratch home held nothing new afterwards (`~/.cache/kache` cleaned up, and a
  pre-existing `~/.cache` left alone). `just kache-status` agreed:
  "installed no / active no / VERDICT: not in use", rc 0.
- **Below floor**: a real kache 0.22.0 went into the scratch `CARGO_HOME` via
  `cargo binstall kache@0.22.0`.
  - Non-interactive (`</dev/null`): "ERROR: kache 0.22.0 is below the floor
    0.23.0 and this init is non-interactive; refusing to upgrade or skip
    silently…", recipe exit 1, binary unchanged.
  - Interactive (`printf 'y\n' | script -qec 'just _ensure-kache'` gives stdin
    a pty so `[ -t 0 ]` holds): the prompt appeared, `y` ran
    `just install-kache true` (0.22.0 → 0.26.3, x86_64-musl), and it printed
    "binary-only mode: no config written, no daemon touched.", rc 0.
    Afterwards there was no `$XDG_CONFIG_HOME` at all (no config), `kache
    daemon --json` showed `daemon_running: false`, `service_installed: false`,
    no daemon process, and no systemd unit.

### Wave 2 — First init + core checks (dev Mac)

Pre-state: kache 0.26.3 ad hoc (`flags=0x2(adhoc)`), config pair already
written by the Phase 2 drill, daemon PID 8160 running. PID 8160 is **not
launchd's job**: `launchctl print` shows the `ninja.kunobi.kache` job as
`exited`, and 8160's parent is `kache monitor` (PID 70818). Its environment
still carries `KACHE_CACHE_DIR=/Volumes/coding/kache`, and it listens at
`/Volumes/coding/kache/daemon.sock`.

- **`just init`** (full, `</dev/null`, 9m39s, rc 0). The one-block report:
  verdict qualifies (checkout/store/base all device 16777235), base covered
  (`/Volumes/coding/wt`, store→base clone probe: clone), version 0.26.3 (floor
  0.23.0), passthrough pass, store `/Volumes/coding/kache` "moved: no — pinned
  deliberately", old store "56G at /Users/ken/Library/Caches/kache —
  abandoned; deleting it is left to you", daemon running 0.26.3 at the
  store-dir socket, activation in `~/.cargo/config.toml`. binstall was a
  no-op ("already installed"), and both merge writes reported "already holds".
  Side effect worth knowing: `init` reinstalls the repo CLIs (sniff,
  darkmatter, claudine, …) from the checkout it runs in, so they now come from
  this worktree.
- **Finding: `llvm-tools-preview` masks check 1.** `init` →
  `scripts/ensure-ci-tools.sh:189` runs `rustup component add
  llvm-tools-preview`. That component installed a *real* `libLLVM.dylib`
  (139 MB, 19:16:15) at `…/1.98.1-…/lib/rustlib/aarch64-apple-darwin/lib/`,
  the same path the removed hand-made symlink occupied, and `rust-lld`'s
  `@rpath` finds it with or without `DYLD_*`. A plain `just zed-wasm` passing
  therefore proves nothing about the fix. It is rustup-managed, so it
  stays; the check was re-run with it renamed aside (trap-restored), as
  below. Consequence: on any host where `init` has run, the toolchain
  symptom is hidden, although `DYLD_*` stripping still matters for anything
  outside that component's reach.
- **Finding: the 17 `rust-objcopy` SIGABRT warnings in the init log are
  replayed diagnostics**, not live failures. All 17 carry the same dyld PID
  (26246) and belong to the same unit (`renderable` build script, "stripping
  debug info with `rust-objcopy` failed"). *Corrected in Wave 5:* I first
  attributed them to kache replaying stored stderr. The replay is actually
  **cargo's**: cargo re-emits cached warnings for up-to-date units. The same
  warning, with the same PID, reappears under `RUSTC_WRAPPER=""` (no kache at
  all). The unit dates from 14:46, during the original diagnosis, before any
  fix. Proof it is not live: a fresh `cargo build --release -p renderable`
  (scratch target dir, unique `--cfg`, libLLVM aside) produced 0
  `rust-objcopy` warnings. The stale unit is an unstripped build script and
  is harmless. It keeps replaying until that unit rebuilds.
- **Check 1 (pristine toolchain)**: PASS. `libLLVM.dylib` in rustlib renamed
  aside; `rm -rf dmls/zed-dmls/target`; a unique `RUSTFLAGS=--cfg …` forces
  every crate, including the final link, to execute rather than restore.
  - control, `RUSTC_WRAPPER=/tmp/p4/pristine/bin/kache` (pristine hardened
    0.26.3): rc 101, `linking with wasm-component-ld failed … dyld: Library
    not loaded: @rpath/libLLVM.dylib … failed to invoke LLD: signal: 6
    (SIGABRT)`, which is the original failure reproduced exactly.
  - installed re-signed kache through the config-file wrapper: rc 0, "rebuilt
    dmls/zed-dmls/extension.wasm", 0 dyld lines.
  - The tracked `extension.wasm` the recipe overwrites was restored with `git
    checkout`.
- **Check 2 (passthrough probe)**: PASS. The pristine binstall 0.26.3 fetched to
  `/tmp/p4/pristine` (`cargo binstall --root`) is `flags=0x10000(runtime)`.
  `KACHE_HOST_BIN=<pristine> scripts/kache-host.sh probe-passthrough` gave
  `passthrough=fail`, rc 1. The installed binary gave `passthrough=pass`, rc 0.
- **Check 5 (idempotence)**: PASS. A second full `just init` gave rc 0, the
  same verdict line, daemon PID 8160 unchanged (no restart, which also shows
  no re-sign rewrote the binary), `~/.config/kache/config.toml` and
  `~/.cargo/config.toml` SHA-1 identical before and after, and no new backup
  files.

### Wave 3 — Lifecycle parity (dev Mac)

**Finding: this agent session inherited the stale `KACHE_CACHE_DIR`.** The
Claude Code process was started from an interactive shell that had already
sourced `~/.env` before line 51 was deleted, so every command it ran carried
the variable. That includes both `init` runs, which account for the 873
probe files that appeared in `~/Library/Caches/kache` after the edit. It is
not a defect in the change: a fresh shell has no variable. From here on each
check runs under `env -u KACHE_CACHE_DIR` or `env -i` to reproduce a fresh
process faithfully.

- **Check 3 (worktree lifecycle)**: PASS, and nothing kache-specific ran between
  the steps.
  1. `wt create p4/kache-lifecycle --stay` created
     `/Volumes/coding/wt/rusty-biscuit/p4-kache-lifecycle`, and
     `cargo build -p sniff` ran there in 41.6 s: 395 crates, 385 hits, 10 misses
     (the new `sniff` crates at this commit), 0 errors, 0 store failures.
     `store_bytes = 115,413,897,337`, `entries = 22,487`.
  2. `wt remove p4-kache-lifecycle -ff -b` removed both the worktree and the
     branch.
  3. `wt create p4/kache-lifecycle-2 --stay` followed by `cargo build -p sniff`
     ran in **6.5 s**: 395 crates, **395 local hits, 0 misses**, 0 errors,
     0 store failures, 0 fallbacks.
     `store_bytes = 115,413,897,337`, `entries = 22,487`, byte-identical to
     step 1 (there was no index churn either).
  4. `kache doctor --json`: no failing checks. `kache stats` (last-build
     summary): errors 0, store_failures 0.
  5. The throwaway worktree and branch were removed.
  - Note: the store is over its cap (115.0 GB against a 107.4 GB
    `local_max_size`). `kache stats` suggests `kache clean --tracked --stale
    14d`. That is left to Ken.
- **Check 4 (launcher parity)**: PASS. `kache doctor --json` `Cache dir` was
  `/Volumes/coding/kache` in each context:
  - a fresh interactive zsh under `env -i` (`zsh -i -c`, which sources
    `~/.zshrc` → `adaptive.sh` → `~/.env`);
  - `env -i HOME PATH`;
  - a `launchctl submit` one-shot (its environment had no `KACHE*` variable).

  Old-store drip test: a clean-env compile (`env -i`, fresh target dir) added
  0 files to `~/Library/Caches/kache`. I ran a discriminating control because
  probe files are cached per toolchain, so a no-drip result alone proves
  nothing. The same compile with `KACHE_CACHE_DIR=<empty scratch dir>` wrote
  `probes/<hash>.json` into that dir, which shows the raw-env drip is live
  and that only the variable steers it. The "no managed launcher still sets
  `KACHE_CACHE_DIR`" clause is **not yet** true at this point: the daemon
  plist still carries it (kept deliberately for check 8). It is re-verified
  after Wave 4.

### Wave 4 — Daemon confirmation + plist (dev Mac, spec check 8)

**The check as written would not discriminate.** The plist's stale
`KACHE_CACHE_DIR` held the *same* value as the ratified store
(`/Volumes/coding/kache`), so "daemon resolves the same store as the CLI" was
true whether or not the daemon honors `ignore_env`. I used the spec's
"re-set by hand for the test" allowance to make it discriminate:

1. I backed up the plist and config (`/tmp/p4/plist.orig`, `config.orig`),
   set the plist's `KACHE_CACHE_DIR` to an empty scratch store
   `/tmp/p4/plist-store` (PlistBuddy), and set `ignore_env = false` through the
   merge helper as the **control** state.
2. Control: `kache daemon stop`, `launchctl bootout` + `bootstrap`. launchd's
   daemon (parent 1) carried `KACHE_CACHE_DIR=/tmp/p4/plist-store` and built a
   complete store *there* (`index.db`, `daemon.sock`, `store/`, …). This proves
   the plist env is live and reaches the daemon.
3. `env -u KACHE_CACHE_DIR just _ensure-kache`: the step-4 write reported
   "wrote … ignore_env" and the config-change trigger fired ("kache:
   restarting the daemon — config changed."). **That restart failed**, and
   the failure contract ran live: "WARNING: 'kache daemon restart' failed.",
   the existing activation was neutralized (`rustc-wrapper = ""`, dated
   backup), "kache left OFF; 'just init' continues…", rc 0.
4. Diagnosis of the failed restart: it was an artifact of my test
   construction, and the recipe did not change. Ken's long-running
   interactive `kache monitor` (PID 70818) autospawns a daemon, with its own
   inherited env, whenever none is reachable. After step 2's stop it had
   spawned one on `/Volumes/coding/kache`, so two daemons were serving two
   different stores at once. The restart failed only in that state. It
   succeeded 3/3 from the normal launchd-owned state, and it also succeeded
   from the monitor-owned state this phase started in (daemon 8160). In
   both cases it handed ownership to launchd (parent 1).
5. **Check 8 result**: PASS. After the (manual) restart there was exactly one
   daemon, launchd's, whose env still said
   `KACHE_CACHE_DIR=/tmp/p4/plist-store`. `lsof` showed it holding
   `/Volumes/coding/kache/{index.db,daemon.sock,daemon.control.v2.sock}`, and
   `kache daemon --json` from a fresh shell reached it at
   `/Volumes/coding/kache/daemon.sock`. The daemon honors the config over a
   stale plist env on the real host.

**Plist regeneration**: `env -u KACHE_CACHE_DIR kache daemon uninstall` then
`install`. This has to run from a clean env, because `install` is how the
variable got into the plist originally. The new plist's
`EnvironmentVariables` holds only `KACHE_LOG=kache=info`. Uninstall briefly
left no daemon, so the monitor respawned one with its stale env. One `kache
daemon restart` returned ownership to launchd: PID 69350, parent 1, env
`KACHE_LOG` only, serving `/Volumes/coding/kache`, reachable from `env -i`.
`env -u KACHE_CACHE_DIR just _ensure-kache` re-activated
(`rustc-wrapper = "kache"`, "already holds" for the config pair) and
reported the daemon running at the store-dir socket. The scratch store was
removed. The config's other keys are unchanged against the backup (only
`local_store`/`ignore_env` differ, now `"/Volumes/coding/kache"`/`true`).

**Drips**: 0 new files in `~/Library/Caches/kache` since the Wave 3 clean-env
marker. Check 4's remaining clause ("no managed launcher still sets
`KACHE_CACHE_DIR`") is now true. The one process that still carries the
variable is Ken's interactive `kache monitor`, whose env came from a shell
opened before the `~/.env` edit. It is not a managed launcher and clears
when the monitor is restarted from a fresh shell.

**Abandoned store (for Ken)**: `~/Library/Caches/kache` is **56 GB**, with
`index.db` last written 2026-09-12. Deleting it is left to Ken.

Merge-helper dated backups created by this wave's writes (left in place by
design): `~/.config/kache/config.toml.bak-20260923-193338`, `…-193433`;
`~/.cargo/config.toml.bak-20260923-193442`, `…-193622`.

### Wave 5 — Failure contract drill (dev Mac, spec check 7)

**Upgrade path**: `cargo binstall --force kache@0.22.0` installed kache 0.22.0,
`flags=0x10000(runtime)`, and the probe failed. A full `env -u KACHE_CACHE_DIR
just init` then ran in 1m10s, rc 0. The kache block shows binstall fetching
0.26.3, the probe reporting `passthrough=fail` (hardened), `codesign`
"replacing existing signature", and the probe then reporting
`passthrough=pass`. The config pair was "already holds", activation "already
holds", and the report showed version 0.26.3 (floor 0.23.0). Afterwards:
`kache --version` 0.26.3, `flags=0x2(adhoc)`, and check 2's probe passes.
- **Daemon restart: happened, but not through `init`'s version trigger.** The
  downgrade did not leave a 0.22.0 daemon running: launchd's daemon kept its
  0.26.3 image. `init`'s upgrade plus re-sign then replaced the executable,
  and that daemon exited. Ken's `kache monitor` respawned one a second later
  (PID 75713, 19:37:47) from the new file: its `txt` inode 733346752 is the
  re-signed 0.26.3 binary, mtime 19:37:46. When step 6 ran, the daemon
  already reported 0.26.3 = installed, so by design no explicit restart was
  needed. The end state is what the check asks for (the daemon runs the new,
  re-signed binary), but **the version-mismatch trigger itself was not
  exercised live**. That needs a 0.22.0 daemon running against the real
  115 GB store, and I deliberately did not do that: an older daemon opening
  an index written by 0.26.3 risks that index. The trigger's logic is
  text-pinned by `ensure_kache_runs_the_spec_section_4_order`. If Ken wants
  it exercised live, the construction is: downgrade, `kache daemon restart`
  (confirm `daemon_version` 0.22.0), then `just init`.

**Pre-activation failure**:
- **The spec's construction does not fail.** A regular file at
  `/Volumes/coding/kache/daemon.sock` after `kache daemon stop` was simply
  replaced by a socket: the monitor-respawned daemon bound it within 10 s. kache
  recovers from a stale socket-path file gracefully.
- A **non-empty directory** at the socket path (`daemon.sock/p4-block/keep`)
  does block it: no daemon could start (`daemon_running: false`, no process).
  Then a full `env -u KACHE_CACHE_DIR just init` ran, rc 0: "kache: daemon not
  running — starting it …" → "WARNING: the kache daemon is not running after
  install/start/restart." → the activation was neutralized (`rustc-wrapper =
  ""`, dated backup) → "WARNING: kache left OFF; 'just init' continues…". It
  went on to run `_ensure-gitnexus` ("GitNexus is ready."), `_ensure-git-hooks`,
  and every CLI install through to "Claudine installed!".
- (Wave 4 also ran the contract live by accident, through a failed
  `kache daemon restart`. The contract has now held for two different step-6
  failures.)
- Restore: removed the blocker, ran `kache daemon start`, and
  `env -u KACHE_CACHE_DIR just _ensure-kache` re-activated. Daemon PID 20063,
  parent 1, env `KACHE_LOG` only.

**Final host state** (`env -u KACHE_CACHE_DIR just kache-status`, rc 0):
installed 0.26.3 (floor 0.23.0); active YES via `~/.cargo/config.toml`; store
`/Volumes/coding/kache` (from doctor); checkout, store, and base all on device
16777235; base `/Volumes/coding/wt` on the store's device; passthrough pass;
"VERDICT: active on a filesystem that clones blocks".

### Linux qualifier (conditional)

No qualifying Linux host is available. `build-linux` (`$BUILD_LINUX`, the only
declared Linux host) runs ZFS (`rpool/data/subvol-700-disk-0`), but block
cloning does not work there: `cp --reflink=always` fails with "Operation not
permitted" (a container subvolume, or the `block_cloning` feature is off), and
its `/tmp` is tmpfs. The spec marks this host "if one is available", so its
absence blocks nothing and checks 1–4 on Linux are unmet for lack of a host.
As extra negative evidence, the fixed `qualify` ran there with a scratch home
and reported `verdict=no-qualify reason=clone-unsupported-on-zfs` (rc 1),
leaving the scratch home empty. That is the fresh-home cleanup branch on
real Linux.

### Skill updates (repo rule: OS traps learned the hard way land in the skill)

- `.claude/skills/kache/installation.md` (hardened-runtime section): how
  `llvm-tools-preview` masks the symptom and how to test around it, and that a
  repeating-PID dyld *warning* is cargo replaying a cached diagnostic.
- `.claude/skills/kache/platforms.md` (macOS): run `kache daemon install`
  from a clean env (a clean install writes only `KACHE_LOG`); `kache monitor`
  autospawns its own daemon, so the running daemon may not be launchd's;
  `restart` returns ownership; the two-daemon state makes `restart` fail.

### Linux cross-check: a Phase 1 test defect found and fixed

`just cross-check test-toolkit --os linux` (build-linux, ZFS without
cloning) first failed
`kache_host_contracts::qualify_prints_a_stable_verdict_line_that_matches_its_exit_code`.
The test required the `kache-host: devices` line on every verdict, but the
script emits it only on a qualify verdict. A no-qualify verdict prints just
the reason line (spec §6: "one line naming the reason"), and `_ensure-kache`
reads the devices line only on the qualifying path. The test had only ever
run on the qualifying dev Mac, so it would have failed on CI's
`ubuntu-latest` (ext4). The defect is in the test, not in the script: the
assertion moved into the qualify branch. Second run: all `kache_*` tests
pass on Linux, including the new fresh-home test on its no-qualify branch.

### Gates

- `just test` (tools area, macOS): 378 passed, 2 skipped. The skips are the
  pre-existing `#[ignore]` fixtures in `nextest_config_verification`.
- `just lint` (tools area): rc 0.
- `bash -n scripts/kache-host.sh`: clean (pinned by `kache_host_script_parses`).
- `just cross-check test-toolkit --os linux`: 376 passed, 1 failed, 2 skipped.
  The failure is **pre-existing and unrelated**:
  `ci_workflow_contracts::the_lint_step_measures_a_sub_second_command_instead_of_recording_zero`
  ("wrote duration_s=0.0" — the stub ran below the measurable resolution on
  build-linux). It failed on both runs, passes on macOS, and this phase does
  not touch its code or subject. Recorded, not fixed.
- Windows: no Windows-specific code changed (the cascade edit is in the
  darwin/linux/wsl branch; the ReFS branch is untouched), so there was no
  native-Windows run. The WSL guest ran the changed script live (above).

### Requirement → test mapping (Phase 4)

| Behavior | Evidence |
|---|---|
| Fresh home (no user cache dir) still gets the user-cache-dir placement | `kache_host_contracts::qualify_places_the_candidate_in_a_fresh_home_and_cleans_up_on_no` (failed before the fix with the exact wrong reason, passes after; qualify branch on macOS) |
| No-qualify verdict leaves no created cache dirs behind | same test, no-qualify branch (Linux cross-check), plus live WSL ext4 and build-linux ZFS runs |
| No-qualify output format (reason line only) is contract-correct | `kache_host_contracts::qualify_prints_a_stable_verdict_line_that_matches_its_exit_code` (fixed; green on macOS qualify and Linux no-qualify) |
| Spec checks 1–8 | host verification, not automatable (they mutate the dev Mac's toolchain, daemon, store, and Cargo config); evidence per check in Waves 1–5 above |

## Phase 5

Closure only. This phase changed no source, docs, or skill files; its only
edits are to this fix's plan, log, and spec frontmatter.

### Final validation (2026-09-23, dev Mac)

- `just test` (tools area → `test-toolkit`): **378 passed, 2 skipped**, rc 0.
  The 2 skips are the pre-existing `#[ignore]` fixtures in
  `nextest_config_verification`. All 33 `kache_*` contract tests
  (`kache_host_contracts`, `kache_config_merge_contracts`,
  `kache_recipe_contracts`) ran in L1 and passed.
- `just lint` (tools area): rc 0.
- `bash -n scripts/kache-host.sh`: clean. `python3 -m py_compile
  scripts/kache-config-merge.py`: clean.
- Drift scan (`rg -n "never activated|never reinstalling|not reinstalling|KACHE_DIR|NOT activated|Activation stays a host decision"`
  over `docs/`, `README.md`, `.claude/skills/`, `justfile`, `just/`,
  `scripts/`): 3 hits, all deliberately historical. Each describes the
  pre-2026-09-23 behavior as superseded: `justfile:1220` (kache-status
  header), `docs/kache-strategy.md:26` ("…skip is gone"),
  `scripts/kache-host.sh:331` (why doctor is the authority). A wider scan
  for the old OS-name policy (`installed by just init on macOS`,
  `never activate`, the `0.15.0` floor) found nothing kache-related.
- Coherence review: the working tree was clean at the start of the phase, and
  the whole fix is committed on `fix/dmls`. I re-read the header comments of
  `_ensure-kache`, `install-kache`, and `kache-status` against their behavior,
  plus the Phase 4 `candidate_store`/`drop_created_candidate` edit in
  `scripts/kache-host.sh` against its comments. All match, and no drift was
  found.

### Spec verification summary

| # | Check | Where | Result |
|---|---|---|---|
| 1 | Pristine toolchain: `just zed-wasm` links through kache | dev Mac | PASS. Pristine hardened kache reproduced the dyld SIGABRT; the re-signed install linked. `llvm-tools-preview`'s real `libLLVM.dylib` was renamed aside for the run because it masks the symptom |
| 2 | Passthrough probe can pass and fail | dev Mac | PASS. Installed (ad hoc) → `passthrough=pass`; pristine binstall 0.26.3 (`flags=0x10000(runtime)`) → `passthrough=fail` |
| 3 | Worktree lifecycle served from store | dev Mac | PASS. Rebuild in a fresh worktree: 395/395 local hits, byte-identical store size, doctor/stats clean |
| 4 | Launcher parity, no old-store drip | dev Mac | PASS. Fresh interactive shell, `env -i`, and a `launchctl` one-shot all resolve `/Volumes/coding/kache`; 0 new files in `~/Library/Caches/kache`. The drip control proved the test can fail |
| 5 | Idempotence | dev Mac | PASS. Second `init`: same verdict, same daemon PID, config files SHA-identical, no new backups |
| 6 | Negative host | WSL ext4 (`build-win`) | PASS. `clone-unsupported-on-ext4`, never installed, `kache-status` agrees; below floor: a non-interactive run errors and an interactive run does a binary-only upgrade with no daemon |
| 7 | Upgrade path + failure contract | dev Mac | PASS with two caveats. (a) The upgrade/re-sign/probe path passed, and the daemon ended on the new binary. It restarted because the executable was replaced, though, so `init`'s version-mismatch trigger was **not fired live** (that would need a 0.22.0 daemon on the real 115 GB store; deliberately avoided). The trigger is text-pinned by `ensure_kache_runs_the_spec_section_4_order`. (b) The spec's "regular file at the socket path" construction does not fail, because kache replaces the file. A non-empty directory at that path did fail, and the contract held: WARNING lines, kache left off, and `init` completed its other steps |
| 8 | Daemon honors the config over plist env | dev Mac | PASS. Made discriminating with a scratch-store plist env plus an `ignore_env = false` control; after the config-change restart the launchd daemon served the configured store |
| 1–4 | on a qualifying Linux host | — | UNAVAILABLE. `build-linux` is ZFS without working block cloning (`reflink` "Operation not permitted"), so the verdict is `clone-unsupported-on-zfs`. The spec says "if one is available" |

**Host cleanup state (dev Mac):** toolchain `libLLVM.dylib` symlinks removed
(1.98.1, stable); `~/.env` line 51 deleted (backup
`~/.env.bak-20260923-kache`); the daemon plist regenerated from a clean env
(`KACHE_LOG` only), with a launchd-owned daemon on `/Volumes/coding/kache`;
kache 0.26.3 ad hoc, config pair pinned, activation on, `kache-status` healthy.
**For Ken:** the abandoned `~/Library/Caches/kache` (56 GB) needs your
decision on deletion; the live store is over its cap (115.0 of 107.4 GB;
`kache clean --tracked --stale 14d` suggested); and the long-running
`kache monitor` still carries the stale `KACHE_CACHE_DIR` until it is
restarted from a fresh shell.

### Ready for review

Spec frontmatter set to `implemented: true`, `status: implemented`,
`implemented_by`. Terminal state: implementation complete, ready for review.
I have not moved the fix to `_completed` and have not run `just complete`.

### Requirement → test mapping (Phase 5)

Phase 5 changed no behavior, so it added no tests. The gates above re-ran the
full contract suite that pins Phases 1–4 (see those phases' mapping tables).
No skipped or failing tests beyond the 2 pre-existing `#[ignore]` fixtures on
macOS. The previously recorded build-linux-only failure
(`the_lint_step_measures_a_sub_second_command_instead_of_recording_zero`) is
pre-existing and unrelated, and was not re-run this phase.
