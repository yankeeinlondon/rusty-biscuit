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
implementation_1: "2026-09-23T20:57:32-07:00"
implementation_2: "2026-09-23T21:50:21-07:00"
implementation_3: "2026-09-24T02:21:33-07:00"
implementation_4: "2026-09-24T02:53:39-07:00"
implementation_5: "2026-09-24T03:08:38-07:00"
implementation_6: "2026-09-24T03:38:20-07:00"
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

## Implementation of Review Findings #1

> **started at:** 2026-09-23T20:57:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Status can certify a filesystem that cannot clone' at 20:58:18
        - discovered: `kache-host.sh report` emitted only device ids and `kache-status` treated matching ids as clone-capable, so a same-device store on a non-cloning filesystem got a healthy verdict that `qualify` would have rejected
        - changed `scripts/kache-host.sh`: extracted `clone_check <src> <dst> <role>` (device check, then the clone probe; the ReFS check on Windows) and routed `qualify` through it with identical reasons; `report` now runs it from the doctor-reported store into the checkout and, when configured, the worktree base, emitting `kache-host: clone checkout=clone|copy [reason=…]` and `kache-host: clone base=clone|copy|- [reason=…]`
        - changed the root `justfile` `kache-status` recipe: prints both clone results and adds a named drift problem (nonzero exit) when the store cannot clone into the checkout, or into a same-device worktree base; off-device cases keep their existing device problems
        - corrected the drifted "the verdict and the report cannot disagree" claims in `scripts/kache-host.sh`, the `kache-status` recipe comments, `docs/initialization.md`, and `docs/kache-strategy.md`: they share the clone check but start from different stores, and status re-probing is what catches drift
        - added a reusable L1 subprocess harness, `tools/test-toolkit/tests/common/kache.rs` (`KacheHostFixture`): a temp checkout with the real justfile, `just/`, and `scripts/`, a fake `kache` (`--version`, `doctor --json`, `rustc`), a fake `cp` whose clone flags fail into named directories, and a cleared environment with fixture-owned `HOME`, `CARGO_HOME`, `TMPDIR`, XDG dirs, and `WT`
        - added `tools/test-toolkit/tests/kache_status_contracts.rs` (unix): clone failure into the checkout and into the worktree base each fail status with the reason; a cloning control reports no clone problem (and is healthy off macOS, where the bash fake cannot forward `DYLD_*`)
        - non-vacuous check: with the two new recipe problem branches disabled, both failure tests went red and the control stayed green; restored with a fresh mtime
        - verified: `cargo nextest run -p test-toolkit` 381 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `rustfmt --check` clean on the new files; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; live `just kache-status` on this Mac still healthy with `store -> checkout: clone`
        - not done: the Windows arm of `clone_check` has not run on Windows (no host in this session), and `_ensure-kache` step (3) still accepts a same-device doctor store without the clone check; that is outside this finding
- work completed for 'Status can certify a filesystem that cannot clone' at 21:03:21
- starting the work on 'An empty Cargo wrapper is reported as active kache' at 21:05:16
        - discovered: `kache-status` marked a config active whenever it contained the text `rustc-wrapper`, and `_ensure-kache`'s non-qualifying drift report did the same (plus treated any non-empty `RUSTC_WRAPPER` as kache), so the `rustc-wrapper = ""` that `kache_off` writes, or an `sccache` wrapper, was reported as active kache; `kache_off` itself only recognized the literal `"kache"`, missing a path-valued activation
        - considered `cargo config get build.rustc-wrapper`: it needs `-Z unstable-options` (nightly), so it was not used
        - added a shared helper, `scripts/kache-host.sh wrapper` (python3 + `tomllib`): Cargo's precedence — `RUSTC_WRAPPER` when set at all (empty = no wrapper, overriding config), then `CARGO_BUILD_RUSTC_WRAPPER`, then `build.rustc-wrapper` from the checkout's `.cargo/config[.toml]`, then `$CARGO_HOME/config[.toml]`; classifies each value as `kache` (bare name, or a path whose basename is `kache`/`kache.exe`, case-insensitive), `none` (empty), or `other`. Emits `kache-host: wrapper-source scope=env|repo|host kind=K source=SRC value=V` per defining source (highest precedence first) and `kache-host: wrapper=K source=SRC value=V` for the winner (`wrapper=none source=- value=` when unset). Exit 2 with `kache-host: error=…` on an unparseable config, a non-string value, or a python3 without `tomllib`
        - assumptions: only the checkout's own `.cargo/` is read, not every ancestor directory Cargo would also walk (the repo tracks no `.cargo/config.toml`); when a directory holds both `config` and `config.toml`, both are read with the legacy `config` first — Cargo reads only `config` there, so the winner differs only if `config` lacks the key and `config.toml` has it; the helper needs Python ≥ 3.11, the same dependency `kache-config-merge.py` already imposes on init
        - changed `kache-status`: the `active` line comes from the helper — YES only when the winner is kache; an empty winner reads "sets an empty wrapper, which disables wrapping"; a foreign winner is named ("wraps rustc with 'sccache', not kache"); a kache source shadowed by a higher one is reported on a `shadowed` line; a helper error prints `active UNKNOWN` and is a drift problem (exit 1). The not-in-use verdict now reads "Cargo builds without kache", since a foreign wrapper can be present
        - changed `_ensure-kache`: the non-qualifying drift warning fires only when the helper's winner is kache (a helper error warns that activity is undecidable); `kache_off` neutralizes `$CARGO_HOME/config.toml` when the helper classifies that file's value as kache, so a path-valued activation is undone too
        - harness (`tools/test-toolkit/tests/common/kache.rs`): `write_config_wrapper(value)` (`activate_wrapper` now delegates to it), `set_rustc_wrapper_env(value)`, and a link to the test host's `python3` in the fixture `bin/` — macOS `/usr/bin/python3` is 3.9 without `tomllib`; `host_just` generalized to `host_tool(name)`. Recorded the PATH trap in the `os` skill (`macos.md`)
        - tests: `kache_status_contracts.rs` +5 (empty config wrapper → not in use, exit 0; `kache` and `/opt/kache/bin/kache` → active; `sccache` → not kache, exit 0; `RUSTC_WRAPPER=''` over a kache config → not in use with the config reported as shadowed; `RUSTC_WRAPPER=kache` with no config → active); new `kache_ensure_contracts.rs` +2 (non-qualifying host: neutralized wrapper → no ACTIVE warning; kache wrapper → ACTIVE warning). Active-kache exit codes are asserted off macOS only, where the bash fake cannot forward `DYLD_*`
        - repointed the source-text contract `kache_status_shares_the_probe_and_fails_loudly_on_drift` (it pinned the old inline precedence strings) to require `./scripts/kache-host.sh wrapper`
        - updated `docs/initialization.md`: the non-qualifying drift sentence now states the shared decision and Cargo's empty-value semantics
        - non-vacuous check: with the old substring logic restored in the justfile, the four defect tests (empty config, `sccache`, empty env override, init neutralized) went red while the kache positive controls stayed green; restored by plain copy and `touch`
        - verified: `cargo nextest run -p test-toolkit` 388 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses; live `just kache-status` on this Mac still active and healthy
        - not done: `kache_off`'s neutralization has no subprocess test (triggering it needs a qualifying host whose later step fails); the `wrapper` subcommand has not run on native Windows or WSL2 in this session
- work completed for 'An empty Cargo wrapper is reported as active kache' at 21:08:48
- starting the work on 'Status omits required daemon and configuration drift checks' at 21:09:39
        - discovered: `kache-status` printed a below-floor version without failing, and never read the daemon state, `[cache] local_store`, or `ignore_env`, so a stopped or stale daemon, a removed or re-pointed pin, or a disabled `ignore_env` got a healthy verdict while the wrapper was active
        - real output shapes (read-only, kache 0.26.3 on this Mac): `kache daemon --json` → `{schema_version, command: "daemon-status", version, epoch, daemon_running: true, service_installed: true, service_path, socket: "/Volumes/coding/kache/daemon.sock", daemon_version: "0.26.3" (bare), daemon_epoch, daemon_config_path: "~/.config/kache/config.toml", service_executable_mismatch: false}`; `kache doctor --json` → `{schema_version, command, version, rustc, issues: 0, checks[{label, pass, optional, detail, fix}]}` with labels Binary, RUSTC_WRAPPER, Cargo config, Host config, Cache dir, Cache FS, Store DB, Link layout, Daemon version (`v0.26.3 (epoch …)`, optional), Daemon service, C/C++ shims, Compiler probe; the `ignore_env` WARN lines go to stderr. Matches `spike-doctor-json.md`
        - chose `kache daemon --json` over doctor's `Daemon version` check as the daemon source: it separates "service not installed" from "not running" (doctor folds both into one optional failure) and is the source `_ensure-kache` step (6) already restarts from
        - changed `scripts/kache-host.sh`: new `config-path` subcommand (`%APPDATA%\kache\config.toml` on Windows, else `$XDG_CONFIG_HOME/kache/config.toml`, default `~/.config`); `_ensure-kache` step (4) now takes its config path from it, so init's write and status's read cannot diverge. `report` emits, before asking doctor (so they survive a doctor failure), `kache-host: daemon installed=yes|no running=yes|no version=V|-` (or `daemon error=daemon-status-unreadable`), `kache-host: config state=present|absent|unparseable|python3-without-tomllib path=P`, and `kache-host: config ignore_env=true|false|absent`; after doctor, `kache-host: config pin=match|mismatch|absent value=V`, where match compares `local_store` with the doctor store after `~` expansion and `realpath`. The config is read with python3 `tomllib`, the same dependency as `wrapper` and `kache-config-merge.py`
        - changed the `kache-status` recipe: prints `daemon` and `config` lines; while kache is active adds one named problem per failing invariant — "kache X is below the floor F", "the kache daemon service is not installed", "the kache daemon is not running", "the running daemon is kache A but the installed binary is B", "the daemon state is unreadable", "[cache] local_store is missing from P", "[cache] local_store (V) in P is not the store kache resolves (S)", "[cache] ignore_env is false in P", "[cache] ignore_env is missing from P", "the kache user config P cannot be read" — alongside the existing doctor/device/clone/passthrough problems. A missing config file reports both the pin and `ignore_env` as missing. Recipe header comment updated to list the new facts
        - decision (not-active semantics): with no kache wrapper in use (including a non-qualifying host) status prints every fact but judges none and exits 0, as before — consistent with spec §5 (non-qualifying hosts are never activated; `_ensure-kache`, not status, owns the non-qualifying floor check) and with the existing "not in use" verdict; nothing of kache's is in use for a stopped daemon or a missing pin to break
        - harness (`common/kache.rs`): the fake `kache` now answers from files in a fixture-owned `fake-kache/` directory, re-read per invocation — `version` (`--version` and doctor `.version`), `daemon.json` (the whole `daemon --json` document), `passthrough` (`forward`|`strip`) — and appends each invocation to `invocations.log`. New API: `FakeDaemon { service_installed, running, version }`, `set_kache_version(&str)`, `set_daemon(&FakeDaemon)`, `set_passthrough(bool)`, `kache_config() -> PathBuf`, `write_kache_config(&str)`, `kache_invocations() -> String`. `new()` is now a fully healthy host: floor version, daemon installed/running at the floor, config pinning `store/` with `ignore_env = true`, passthrough forwarding
        - better macOS seam: `forward` makes the fake's `rustc` arm answer the probe's `DYLD_FALLBACK_LIBRARY_PATH` request with the probe's sentinel value (a bash fake can never see a `DYLD_*` variable — dyld strips it first), so healthy exit codes are now asserted on macOS too; removed the three `cfg!(not(target_os = "macos"))` gates in `kache_status_contracts.rs` and updated the control test's doc comment that described the old gate. Also fixed a latent fake bug: its `rustc` arm did `shift; exec "$@"`, exec'ing the probe's argument instead of the stub `rustc`, so the probe always reported `error` (hidden by those gates)
        - tests: `kache_status_contracts.rs` +9 — one drift test per invariant (below floor, daemon not running, service not installed, daemon/binary version mismatch, `local_store` missing, `local_store` ≠ doctor store, `ignore_env` false, `ignore_env` missing), each asserting exit 1, `VERDICT: DRIFT`, and exactly one problem line naming the reason; plus not-in-use drift is printed but exits 0. The existing cloning control is the healthy exit-0 case
        - non-vacuous check: with the new problem block removed from the justfile, all 8 drift tests went red and every control stayed green; restored from a copy with a fresh mtime
        - no recipe text-contract test was invalidated (`kache_recipe_contracts.rs` still passes unchanged; the step-(4) merge-call string it pins was kept)
        - docs: `docs/initialization.md` (By hand section) lists the status drift problems and the not-in-use behavior; `.claude/skills/kache/installation.md` status paragraph updated
        - verified: `cargo nextest run -p test-toolkit` 397 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `rustfmt --check` clean on the touched test files; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses; live `just kache-status` on this Mac: daemon installed/running 0.26.3 = binary 0.26.3, config pins `/Volumes/coding/kache` = doctor store, `ignore_env true` → healthy, exit 0
        - not done: `KACHE_CONFIG` (kache's config-path override, seen in the spike) is not honored by status or init — both read the default path; the daemon's own `daemon_config_path` is not compared with that path (Windows path spelling would need care); none of this has run on Linux, WSL2, or native Windows in this session
- work completed for 'Status omits required daemon and configuration drift checks' at 21:14:56
- starting the work on 'Fresh ReFS hosts cannot place an off-device default store' at 21:15:34
        - discovered: the Windows branch of `qualify` published `candidate=-` for ReFS and never ran the placement cascade, so `_ensure-kache` step (3) had nothing to place when `kache doctor` resolved its default (`%LOCALAPPDATA%\kache`, on the NTFS `C:`) off the Dev Drive, and turned kache off on the layout the spec qualifies. `user_cache_dir` also had no Windows value and `mount_point_of` relied on `df`, which a Windows drive root does not need
        - changed `scripts/kache-host.sh`: the Windows branch keeps its ReFS filesystem-type gate first (so an NTFS host gets no directory created), then runs the same `candidate_store` cascade and falls through to the shared `clone_check` (device match plus ReFS type) and verdict path; on Windows the user cache dir is `%LOCALAPPDATA%\kache` (kache's default) and the volume root is the checkout's drive root (`cygpath -u "B:/"`), so the cascade is `%LOCALAPPDATA%\kache` when on the ReFS device, else `kache\` at the drive root. New helpers `windows_drive`, `windows_fstype` (the `Get-Volume` call, now shared by `qualify` and `clone_check`), and `published_path`: the candidate line is published in `cygpath -m` spelling (`B:/kache`), which kache.exe, Windows python3, and the MSYS shell all accept; identity elsewhere. `_ensure-kache` needed no change — its step (3) already consumes the candidate
        - harness (`common/kache.rs`): `emulate_windows()` puts fakes of `uname` (`MINGW64_NT`), `powershell.exe` (`Get-Volume` answers from `fake-host/fstype` for `B:`), `cygpath` (`B:/` → `volume/`, else identity), `stat -c %d` (host device, or a second device for prefixes listed by `place_off_device`), and no-op `cargo`/`cargo-binstall` on the fixture PATH, and exports `APPDATA`/`LOCALAPPDATA`; accessors `windows_volume_root`, `local_app_data`, `set_windows_checkout_fstype`. The fake `kache` now accepts `daemon install|start|restart`, and the fixture links the host `just` into `bin/` (recipes call `just install-kache`). No production test hook was added
        - tests (`kache_ensure_contracts.rs`, +3, run on any unix host): ReFS checkout with `%LOCALAPPDATA%` and the doctor store off-device → `_ensure-kache` qualifies with `candidate=<B:>/kache`, places and pins that store, and activates; `%LOCALAPPDATA%` on the ReFS drive → it is the store and nothing is created at the drive root; NTFS checkout (control) → `filesystem=NTFS-is-not-ReFS`, no directory created
        - non-vacuous check: with the old `candidate=-` branch restored, the two ReFS placement tests went red and the NTFS control stayed green; restored by copy, `cmp`-verified
        - real Windows evidence (`build-win-native`, scratch dir on `B:` ReFS, scratch `LOCALAPPDATA`, removed afterwards): under Git Bash and under Cygwin bash, `qualify` printed `candidate=B:/coding/kache-refs-evidence/lad/kache` with `%LOCALAPPDATA%` on `B:`, and `candidate=B:/kache` with it on `C:` (the transient empty `B:\kache` was removed); a checkout on `C:` gave `no-qualify reason=filesystem=NTFS-is-not-ReFS`. Recorded in the `os` skill (`build-hosts.md`) that `bash` on that host's PATH is Cygwin's, not Git Bash
        - blocker (not this change): `just cross-check test-toolkit --os linux` failed to build on `build-linux` twice with "output file … is not writeable" — this worktree's standing clone has read-only, 4-link `target/release` artifacts (the kache hardlink trap in `build-hosts.md`); left for the host owner. `cross-check --os windows` was not run: the new tests are unix-only (the harness is `cfg(unix)`), so it would be compile evidence only, and the direct script run above is behavioral
        - docs: `docs/initialization.md` step 3 names the Windows cascade; its runtime table row now records what ran on a Dev Drive
        - verified: `cargo nextest run -p test-toolkit` 400 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `rustfmt --check` clean on the touched test files; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses
        - not done: a full `just init` on native Windows; `_ensure-kache`'s report block still computes the abandoned default store as `~/.cache/kache` on Windows (should be `%LOCALAPPDATA%\kache`), so "old store" reads "none found" there
- work completed for 'Fresh ReFS hosts cannot place an off-device default store' at 21:21:46
- starting the work on 'The placement cascade misses writable directories on the checkout volume' at 21:22:33
        - discovered: after the off-device user cache dir and a non-creatable `<mount point>/kache`, `candidate_store` reported `no-user-writable-store-location-on-the-checkout-volume` without looking at the user-owned directories the checkout lives under (the Linux layout `/data` root-owned, `/data/src` user-owned, checkout `/data/src/repo`)
        - changed `scripts/kache-host.sh` `candidate_store`: a new last step, after `<mount point>/kache` and before the failure. Rule: the store is `kache/` in the HIGHEST ancestor of the checkout that lies strictly below the volume's mount point, is on the checkout's device, is owned (`-O`) and writable by the current user, and is not inside a Git working tree (neither it nor an ancestor between it and the mount point holds `.git`). An existing `kache/` there is used when writable; otherwise it is created and removed again by `drop_created_candidate` on a no verdict. The walk starts at the checkout's parent, so the store is never in the checkout or its `target/`; the mount point itself is never a candidate at this step, so a root-owned mount point is still never written. Highest over nearest because that directory is shared by every checkout beside this one on the volume (`/data/src/kache`), and it is not a worktree base's child that `wt` would list. The candidate then goes through the same `clone_check` (device plus clone probe) as every other placement. When `df` gives no mount point the walk is bounded by the device change alone
        - decision (Windows): the step is part of the shared cascade, so it also runs on Windows after `kache\\` at the drive root; in practice a drive root is creatable there, and if Git Bash's `-O` misreads a Windows owner the step finds nothing and the old failure reason stands, so there is no regression. macOS keeps its mount-point `kache/` first
        - updated the two no-qualify prose lines to name the ancestor step, the `candidate_store` header comment, and `docs/initialization.md` step 3 (the exact rule with the `/data/src/kache` example, and the Windows order)
        - test seam (`kache_host_contracts.rs`, `CheckoutVolumeHost`): runs `qualify` directly under a cleared environment with fakes on `PATH`: `uname -s` says Linux, `df -P` names `<scratch>/vol` as every path's mount point, `stat -c %d` puts `<scratch>/home` (the user cache dir) on device 999999, and `cp --reflink=always` copies. `vol` is made read-only with chmod 0555 in place of a root-owned mount point. Each test first checks that a 0555 directory actually blocks writes, and skips with a reason if it does not (root in a CI container). Paths are canonicalized so they match the script's `pwd` behind macOS's `/var` symlink
        - tests (+3): read-only `vol`, writable `vol/src/team`, checkout `vol/src/team/repo` → `verdict=qualify candidate=<vol>/src/kache`, exit 0, nothing written at `vol/` or in the checkout; `vol` and `vol/src` read-only → the existing `no-user-writable-store-location-on-the-checkout-volume`, exit 1; a writable ancestor that holds `.git` (checkout nested in another working tree) → same failure, no `kache/` created there
        - non-vacuous check: against the pre-change script the placement test went red and the other two stayed green (both pin behavior the old code already had); with only the `.git` guard removed, the working-tree test went red. Restored by copy, `cmp`-verified
        - verified: `cargo nextest run -p test-toolkit` 403 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `rustfmt --check` clean on the added code (the file already had formatting drift at lines 10–190, left alone); `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses; live `./scripts/kache-host.sh qualify` on this Mac → `candidate=/Volumes/coding/kache`, exit 0
        - not done: no Linux run. `just cross-check test-toolkit --os linux` was not attempted: the previous finding's run hit the read-only hardlinked `target/release` in this worktree's clone on build-linux (`build-hosts.md`), and the skill does not authorize clearing that target dir. The new tests use fakes for every host decision except the real `stat`/`cp`, so the Linux leg would mainly exercise GNU `stat -c %d`
- work completed for 'The placement cascade misses writable directories on the checkout volume' at 21:26:04
- starting the work on 'A pre-activation failure can leave an inherited wrapper active' at 21:27:19
        - discovered: `kache_off` neutralized only a kache wrapper in `$CARGO_HOME/config.toml` and then printed "kache left OFF" unconditionally, even when the neutralization write failed or an inherited `RUSTC_WRAPPER`/`CARGO_BUILD_RUSTC_WRAPPER` (or a repo/legacy Cargo config) still made kache the effective wrapper
        - changed `justfile` `_ensure-kache` `kache_off`: after the attempted neutralization it re-runs `scripts/kache-host.sh wrapper` and prints "kache left OFF" only when the effective wrapper is not kache. Otherwise it prints `WARNING: kache STILL ACTIVE — manual action required` followed by one line per kache source down to the first source that sets another value (undoing only the winner would expose the next): env sources get `unset VAR`, files get "set [build] rustc-wrapper = \"\" there, or delete that line". When the helper itself fails (exit 2, e.g. an unparseable Cargo config) it prints `kache MAY STILL BE ACTIVE — manual action required` with the helper's error instead of claiming OFF
        - decision (exit status): `kache_off` still exits 0 in every branch. Spec §4 requires `just init` to complete its other setup steps and `init` runs `just _ensure-kache` under `set -e`, so a nonzero exit would abort init; the remedy (an env var in the caller's shell, a file the user owns) is the human's anyway. The unresolved state is distinguishable by its own WARNING text and never contains "kache left OFF"
        - harness (`tests/common/kache.rs`): `fail_kache_install()` (fake `cargo`/`cargo-binstall` exiting 1, so `install-kache` fails as the first pre-activation step on a qualifying host) and `make_cargo_home_read_only()` (config 0444 + dir 0555, returns false when still writable so the test skips as root); a `Drop` impl restores the directory mode so the tempdir can be removed
        - tests (`kache_ensure_contracts.rs`, +4): writable activation → `rustc-wrapper = ""` written, "kache left OFF", exit 0 (control); inherited `RUSTC_WRAPPER=kache` → no OFF claim, STILL ACTIVE naming `environment RUSTC_WRAPPER — undo: 'unset RUSTC_WRAPPER'`, the neutralized config not listed; read-only Cargo home → "could not neutralize", STILL ACTIVE naming the file, config unchanged; unparseable config → MAY STILL BE ACTIVE with `unparseable-cargo-config`. `kache_recipe_contracts.rs` text contract gains the STILL ACTIVE assertion
        - non-vacuous check: with the old `kache_off` restored the env and read-only tests failed (control passed); with only the undecidable branch removed the undecidable test failed. Restored by copy + `touch`, `cmp`-verified
        - docs: `docs/initialization.md` failure-contract paragraph now states the effective-wrapper re-check and the manual-action state; spec §4 left to the author
        - verified: `cargo nextest run -p test-toolkit` 407 passed, 2 skipped; clippy `-p test-toolkit --all-targets -D warnings` clean; `rustfmt --check` clean on `kache_ensure_contracts.rs` and `common/kache.rs`; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses; `scripts/kache-host.sh` untouched
- work completed for 'A pre-activation failure can leave an inherited wrapper active' at 21:29:40
- starting the work on 'Init and status behavior lacks repeatable end-to-end tests' at 21:30:28
        - harness (`tests/common/kache.rs`): one ordered `fake-kache/commands.log` (`commands()`, `clear_command_logs()`) that the fake `kache`, new logging fakes of `cargo` / `cargo-binstall` / `codesign`, the clone path of the fake `cp`, and a `python3` shim (logs each `kache-config-merge.py` write, then execs the host python3) all append to. `cargo binstall` installs `fake-kache/kache-bin` at the `release` version (`set_release`) as a hardened build (passthrough `strip`) and is a no-op when that version is already installed; `codesign` on the installed kache flips passthrough to `forward` and passes every other file to the host's codesign. The fake daemon is now a state machine over `daemon.json` (`install` registers the service without starting it, `start`/`restart` run it at the binary's version, `fail_daemon_restart()` makes restart exit 1). Also `uninstall_kache()`, `kache_bin()`, `daemon_version()`, a no-op `sleep` (the daemon waits poll state that changes synchronously), and `just_command()` (the unspawned `just()` command, for the PTY)
        - tests (`tests/kache_init_contracts.rs`, new, +9, `#![cfg(unix)]`, L1): (1) fresh qualifying host → order qualify → install → (macOS) re-sign → doctor → config write → daemon install → daemon start → activation, activation the last mutation, no restart; config pins the store with `ignore_env = true`, `$CARGO_HOME/config.toml` has `rustc-wrapper = "kache"`, then `kache-status` exits 0 with the healthy verdict; (2) daemon at another version, config unchanged → "binary changed" restart, then activation, daemon now at the installed version; (3) config changed, versions equal → "config changed" restart between config write and activation; (4) second run → both config dirs byte- and mtime-identical with no backup, no re-sign/daemon work, binstall a no-op, same verdict lines; (5) failed restart → exit 0, WARNING, "kache left OFF", no activation command, no wrapper; (6) non-qualifying, below floor, no TTY → nonzero exit, "refusing…", no `cargo` call, no daemon or config command; (7) PTY (expectrl, 60 s expect timeout, no window) answering `y` → binstall ran, version upgraded, "binary-only mode" printed, no daemon/config/activation; answering `n` → nothing installed, version unchanged; (8) non-qualifying, kache absent → no `cargo` or `kache` call, binary still absent. Scenario 8's report-only and the neutralization/STILL ACTIVE failure paths were already in `kache_ensure_contracts.rs` and are not repeated
        - no bugs found: every scenario passed against the current justfile and scripts. Observation, not changed: the spec says the version trigger reads the daemon version from `kache doctor --json`; the recipe reads `kache daemon --json` `daemon_version` (what finding #3's status check also reads). Model assumption: the fake's `daemon install` does not start the daemon; if real kache's install also starts it, a fresh host gets one redundant (harmless) config-change restart after install — only a real host can tell
        - non-vacuous check (justfile mutated, rerun, restored by plain copy, `cmp`-verified): version trigger disabled → (2) and (5) red; config trigger disabled → (3) red; config trigger always on → (2) and (4) red; restart failure ignored → (5) red; `-t 0` check replaced by `true` → (6) red; confirmed upgrade in full mode → (7 y) red; activation moved before the daemon lifecycle → (1), (2), (3), (5) red
        - removed from `kache_recipe_contracts.rs` (superseded by the behavioral tests): `ensure_kache_runs_the_spec_section_4_order`, `ensure_kache_upgrades_below_floor_only_when_confirmed`, `ensure_kache_failure_contract_leaves_kache_off`, `ensure_kache_decides_through_the_shared_probe` (its stale "skipped on WSL/Windows" check moved into `justfile_carries_no_stale_kache_policy_text`). Kept: the three `install_kache_*` tests (source-install fallback and full-mode gating are not exercised by the fixture), the CI step-aside guard (the fixture clears the environment), the report-block fields, the status text contract, and the stale-text scan; the module doc now says what it still covers and where behavior is pinned
        - dependency: `expectrl = "0.8"` under `[target.'cfg(unix)'.dev-dependencies]` in `tools/test-toolkit/Cargo.toml` (same version five workspace suites already build); recorded in `docs/dependencies.md`
        - real-host smoke stays manual (spec Verification 2, 5, 7): the real re-sign and a real launchd/systemd daemon are not L1
        - verified: `cargo nextest run -p test-toolkit` 412 passed, 2 skipped; clippy `-p test-toolkit --all-targets -D warnings` clean; `rustfmt --check` clean on the new and touched test files; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses; justfile and scripts unchanged; `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` compiles (one pre-existing unused-import warning in `kache_host_contracts.rs` on Windows, not from this change)
        - not done: `just cross-check test-toolkit --os linux` not attempted (the worktree clone's read-only hardlinked `target/release` on build-linux, see the earlier findings); the Linux leg would exercise the non-macOS order (no re-sign) and GNU `cp --reflink` through the fake
- work completed for 'Init and status behavior lacks repeatable end-to-end tests' at 21:37:46
- starting the work on 'Targeted config edits discard inline comments' at 21:38:33
        - discovered: `merge()` replaced a whole matching `key =` line with `key = <value>`, dropping indentation, `=` spacing, and any trailing `# comment`; the no-op check only held because the rewritten line happened to equal the original when it had no comment
        - changed `scripts/kache-config-merge.py`: new `value_end()` finds where the existing single-line value ends. It is quote-aware (basic `"..."` with backslash escapes, literal `'...'`), so a `#` inside a string is part of the value, and it tracks bracket depth for single-line arrays/inline tables. New `replace_value()` swaps only the value and keeps everything before it (indentation, key, `=` spacing) and the tail (spacing before `#` plus the comment). When the existing value already parses (tomllib) to the requested value and type, the line is left byte-for-byte, so a repeat is a no-op whatever spelling the file uses (e.g. `'/x'` vs `"/x"`)
        - decision (multi-line / unexpected shapes): a value that opens `"""`/`'''`, an unterminated string, or an array/inline table not closed on its line makes the script fail with `error=<key>: existing value is multi-line or unterminated; edit it by hand`; text after the value that is not a comment fails too. Both exit 1 before any temp file or backup is written, so the original is untouched. Refusal over a best-effort rewrite because replacing only the first line of a multi-line value would leave its tail lines behind. A bare value containing a space (a TOML local datetime) is also refused this way; not a realistic shape for these keys
        - module docstring bullet updated to describe the value-only replacement and the refusal
        - fixture: `scripts/fixtures/kache/config-inline-comments.toml` — indented `local_store = "/old/#1 store"   # retain this placement reason`, `ignore_env = false # intentional for old setup`, and a `#` inside a quoted value in `[other]`; added to the passive corpus
        - tests (`kache_config_merge_contracts.rs`, +3): a changing write rewrites only the two values (whole file compared with the expected text; values checked through the `toml` crate; the script's own tomllib gate passed with exit 0; one backup); writing the current values is a byte-identical no-op with no backup, and a repeat after a changing write is also a no-op with still one backup; a literal-quoted value with `#` keeps its comment, and a `"""` multi-line value is refused with the file unchanged
        - the corpus and in-place tests no longer exclude `local_store` lines: new `assert_line_survives` requires non-target lines verbatim and target lines (`local_store`, `ignore_env`) to keep their prefix through `=` and their trailing comment
        - non-vacuous check: against the pre-change script the three new tests and the corpus test failed (10 passed, 4 failed); restored by plain write + `touch`, `cmp`-verified
        - verified: `cargo nextest run -p test-toolkit` 415 passed, 2 skipped (includes `kache_init_contracts`'s second-run byte-identical check); clippy `-p test-toolkit --all-targets -D warnings` clean; `python3 -m py_compile` ok; `just check-tier-coverage test-toolkit` 0 stranded; the new Rust code passes `rustfmt --check` (the file's 14 pre-existing drift hunks were left alone)
- work completed for 'Targeted config edits discard inline comments' at 21:40:37
- orchestrator follow-up after the last finding, at 21:41:16
        - `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` warned about an unused `process::Command` import in `kache_host_contracts.rs`; the warning was already there at `HEAD`, because every use of `Command` in that file is `#[cfg(unix)]`
        - gated the import with `#[cfg(unix)]`; the Windows-target check now reports 0 warnings
        - final verification on macOS: `cargo nextest run -p test-toolkit` 415 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `just --summary` parses
        - not run: `just cross-check test-toolkit --os linux`. This worktree's clone on `build-linux` has read-only, hardlinked files in `target/release` left by kache hardlink mode, and the build fails on them. The clone was left alone because no skill authorizes clearing it. The new tests fake the platform facts they depend on, so CI's Linux leg is the next place they run on Linux.

### Successful Completion

The implementation of review cycle 1 has completed successfully in 44 minutes. During this implementation all 8 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 8 were fixed, 0 were deferred (see reasons below):

- no finding was deferred

The files changed or added during this cycle:

- `justfile` (`kache-status`, `_ensure-kache`, `kache_off`)
- `scripts/kache-host.sh` (`clone_check`, `wrapper`, `config-path`, the extended `report`, the ReFS candidate, and the user-owned checkout-volume fallback in `candidate_store`)
- `scripts/kache-config-merge.py` (value-only replacement that keeps inline comments)
- `scripts/fixtures/kache/config-inline-comments.toml` (new)
- `tools/test-toolkit/Cargo.toml` (`expectrl` as a unix-only dev-dependency)
- `tools/test-toolkit/tests/common/mod.rs` and `tools/test-toolkit/tests/common/kache.rs` (new shared L1 host fixture)
- `tools/test-toolkit/tests/kache_status_contracts.rs`, `kache_ensure_contracts.rs`, `kache_init_contracts.rs` (new)
- `tools/test-toolkit/tests/kache_host_contracts.rs`, `kache_config_merge_contracts.rs`, `kache_recipe_contracts.rs` (updated; the recipe text contracts that the behavioral tests superseded were removed)
- `docs/initialization.md`, `docs/kache-strategy.md`, `docs/dependencies.md`
- `.claude/skills/kache/installation.md`, `.claude/skills/os/macos.md`, `.claude/skills/os/build-hosts.md`

## Implementation of Review Findings #2

> **started at:** 2026-09-23T21:50:21-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Cargo configuration in ancestor directories escapes wrapper detection' at 21:50:50
        - discovered: `kache-host.sh wrapper` read only the working directory's `.cargo/config[.toml]` and `$CARGO_HOME`, and its comment declared the ancestor search out of scope; Cargo reads `.cargo/` in the working directory and every ancestor (nearer wins), then `$CARGO_HOME` unless the walk already read it (`walk_tree`), so a parent activation left `kache-status` at "not in use" and `_ensure-kache`'s failure path at "kache left OFF"
        - changed `scripts/kache-host.sh` (`wrapper`): walks cwd then every ancestor to `/`, then `$CARGO_HOME`; env precedence (RUSTC_WRAPPER, then CARGO_BUILD_RUSTC_WRAPPER) and the legacy-`config`-first order are unchanged; new scope `ancestor` with an absolute source path; a `$CARGO_HOME` that is an ancestor's `.cargo/` keeps its walk position (as in Cargo) but is reported once as `scope=host` under the `$CARGO_HOME/…` spelling `kache_off` greps for, so neutralization still works for checkouts under `$HOME`
        - decision: kept listing both `config` and `config.toml` from one directory as sources (pre-existing behavior; Cargo reads only `config` when both exist, but the winner is already correct), per surgical-change scope
        - replaced the drifted script comment that declared the ancestor search omitted, and updated the subcommand's usage doc (sources, scope `env|repo|ancestor|host`)
        - changed the root `justfile` `kache-status`: an `ancestor` source renders as "<path> (a parent directory of this checkout)" instead of falling into the host-wide label; `_ensure-kache`'s `kache_off` needed no change (non-env sources already print a per-file undo)
        - updated `docs/initialization.md`'s wrapper-precedence paragraph to name the ancestor walk
        - added `KacheHostFixture::write_parent_config_wrapper` (`tests/common/kache.rs`), writing `.cargo/config.toml` in the checkout's parent
        - added L1 tests: `kache_status_contracts::status_reports_a_kache_wrapper_in_a_parent_directory_config_as_active`; `kache_ensure_contracts::ensure_reports_a_parent_directory_kache_wrapper_as_still_active_when_a_step_fails` (names the parent file's undo, leaves it unedited, no "kache left OFF"); `kache_host_contracts::wrapper_reads_ancestor_configs_nearer_first_before_cargo_home`, `wrapper_reports_a_cargo_home_among_the_ancestors_once_as_the_host_file`, and `wrapper_names_the_parent_config_wrapper_that_real_cargo_invokes` (real `cargo check --offline` in a scratch project under a parent `.cargo/config.toml` naming a failing stub `kache`; asserts the stub ran, then that the helper names that file and value). The real-Cargo test is L1 because test-toolkit already declares `requires-toolchain = true`
        - non-vacuous check: with the ancestor walk removed (old directory list), all 5 new tests went red and the 15 sibling wrapper tests stayed green; script restored byte-exact
        - verified: `cargo nextest run -p test-toolkit` 420 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage test-toolkit` 0 stranded; `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` clean; `rustfmt --check` clean on new code (pre-existing unformatted hunks in `kache_host_contracts.rs` lines 10-191 left untouched)
- work completed for 'Cargo configuration in ancestor directories escapes wrapper detection' at 21:54:27
- starting the work on 'Init can claim activation while Cargo uses another wrapper' at 21:55:03
        - discovered: `_ensure-kache` wrote `[build] rustc-wrapper = "kache"` into `$CARGO_HOME/config.toml` and then printed "kache: qualified and set up —" and the activation line unconditionally; an inherited `RUSTC_WRAPPER` (including set-but-empty), `CARGO_BUILD_RUSTC_WRAPPER`, or a repository/ancestor `.cargo/config.toml` outranks that entry, so Cargo never ran kache while init claimed success (and `kache-status` said "not in use")
        - changed the root `justfile` `_ensure-kache`: new step (8) after the activation write re-runs `scripts/kache-host.sh wrapper` (the helper `kache-status` and `kache_off` already use). When the winner is kache the report is unchanged. Otherwise the header becomes "kache: qualified, but activation INCOMPLETE — Cargo does not run kache for this checkout —", the activation line adds "— written, but NOT in effect (see WARNING below)", and a "kache NOT ACTIVE" WARNING names every non-kache source above the host file with its undo: `unset <VAR>` for an environment source (value shown, so `RUSTC_WRAPPER=""` is visible), "delete its [build] rustc-wrapper line" for a repository config (made absolute from `$PWD`) or ancestor config. It lists every source above the host file, not only the winner, because removing only the winner would expose the next. An undecidable wrapper (helper exit ≠ 0) gets "kache NOT CONFIRMED ACTIVE" with the helper's error
        - decision: the host entry is kept, not neutralized. It is correct, harmless while overridden, and takes effect once the override goes; the WARNING says so ("the host entry stays in … and applies once those are gone"). Neutralizing it would make the user re-run init after removing the override for no gain; kache itself passed every check (unlike the pre-activation failure path, where the wrapper sits on a broken kache)
        - decision: init still exits 0 and completes its other steps (spec §4 failure contract); the remedy is the human's (their shell or a config init does not own). The full report block still prints because store, daemon, and version facts are true
        - decision: the undo for a repository config says "delete" only; setting it to "kache" would be a tracked wrapper, which stays forbidden
        - fixture (`tests/common/kache.rs`): `set_cargo_build_rustc_wrapper_env` and `write_checkout_config_wrapper` (checkout `.cargo/config.toml`, returns the canonical path); `just_command` exports `CARGO_BUILD_RUSTC_WRAPPER` only when set; its doc updated
        - tests (`kache_init_contracts.rs`, +5 L1 qualifying-host cases through the shared `ensure_with_overriding_wrapper`): `RUSTC_WRAPPER=""`, `RUSTC_WRAPPER=sccache`, `CARGO_BUILD_RUSTC_WRAPPER=sccache`, a checkout `.cargo/config.toml`, and a parent-directory `.cargo/config.toml`. Each asserts exit 0, activation ran, the host entry is still `kache`, no "qualified and set up —" header, the INCOMPLETE header, "written, but NOT in effect", the WARNING, and the source-specific undo line. Control: `ensure_sets_up_a_fresh_qualifying_host_in_spec_order` now also asserts the "qualified and set up —" header and no "kache NOT ACTIVE"
        - docs: `docs/initialization.md` (step 6 describes the re-check and the INCOMPLETE report; the report-block paragraph says the header reads "qualified and set up" only when Cargo will run kache), `.claude/skills/kache/installation.md` ("In this repository"), `.claude/skills/kache/SKILL.md`; `kache_init_contracts.rs` module doc
        - non-vacuous check: with the new check neutered (`if false`, which reproduces the old output), all 5 new tests went red and the other 9 in the binary (including the control) stayed green; justfile restored by plain write + `touch`, `cmp`-verified
        - verified: `cargo nextest run -p test-toolkit` 425 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean (script unchanged); `just --summary` parses; `just check-tier-coverage test-toolkit` 0 stranded; `rustfmt --check` clean on `kache_init_contracts.rs` and `common/kache.rs`; `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` clean
- work completed for 'Init can claim activation while Cargo uses another wrapper' at 21:57:49
- starting the work on 'Init accepts a running daemon at the wrong version after restart' at 21:59:01
        - discovered: `_ensure-kache`'s `wait_daemon` returned as soon as `daemon_running=yes`, so after a version-mismatch restart that the service manager reported as done while the old daemon kept answering, init wrote the activation and printed "qualified and set up"; the fake `kache daemon restart` always set the new version synchronously, so no test could see it
        - changed the root `justfile` `_ensure-kache` step (6): `wait_daemon` now succeeds only when the daemon is running **and** reports the installed binary's version, in every case (post-install/start, the version trigger, and the config-change-only trigger — the daemon must match the binary before activation). Still bounded (10 reads, then 20 after a `start`, one per second)
        - changed: a daemon still running at another version after the first wait is a pre-activation failure right away through `kache_off` (the existing neutralization path; kache left off; exit 0): "the kache daemon still runs version <running>, not the installed <installed>, after restart — it must match the binary before activation." A `kache daemon start` is not attempted there, because start cannot replace a running daemon. The post-start timeout names the versions the same way when the daemon is up, and keeps "is not running after install/start/restart" when it is not. The step word is `restart` only when a restart was issued (`lifecycle_step`), otherwise `install/start`
        - decision: no env var for the wait timeout. The fixture's no-op `sleep` already makes a timed-out wait cost only its reads (the new test runs in about 1.7 s), so a knob would be a production surface with no test benefit. The fixture module doc now says so
        - decision: a running daemon that reports no version (`-`) never counts as ready. It already fired the version restart trigger before this change; now, if it still reports none afterward, init leaves kache off instead of activating on an unconfirmed daemon
        - fixture (`tests/common/kache.rs`): `keep_daemon_on_restart` makes the fake `kache daemon restart` exit 0 and leave `daemon.json` unchanged
        - test (`kache_init_contracts.rs`, +1 L1): `ensure_leaves_kache_off_when_the_restarted_daemon_keeps_the_old_version`: daemon at 0.0.1, stale restart; asserts exit 0, the restart ran, the daemon is still 0.0.1, no activation step, no Cargo wrapper, the WARNING naming 0.0.1 and the installed floor version, "kache left OFF", and no "qualified and set up". The synchronous-restart controls (`ensure_restarts_a_daemon_running_another_version_before_activation`, `ensure_restarts_the_daemon_when_the_config_write_changed_the_file`, and the fresh-host order test) still pass. Module doc updated
        - docs: `docs/initialization.md` step 5 and `.claude/skills/kache/installation.md` ("In this repository") now say that the daemon counts only at the installed version
        - non-vacuous check: against the pre-change justfile, the new test failed at the "never reaches activation" assertion and the other 14 in the binary passed. The justfile was restored by plain write + `touch` and verified with `cmp`
        - verified: `cargo nextest run -p test-toolkit` 426 passed, 2 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean (script unchanged); `just --summary` parses; `just check-tier-coverage test-toolkit` 0 stranded; `rustfmt --check` clean on both touched Rust files; `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` 0 warnings
- work completed for 'Init accepts a running daemon at the wrong version after restart' at 22:00:54
- starting the work on 'Worktree restore behavior has no repeatable real-cache test' at 22:01:32
        - discovered: kache 0.26.3 honors `XDG_CONFIG_HOME` on macOS, so a scratch `HOME`/`XDG_CONFIG_HOME` with `[cache] local_store` + `ignore_env = true` pins a scratch store; this agent session still carries the stale `KACHE_CACHE_DIR=/Volumes/coding/kache`, which the scratch config's `ignore_env` gates and the test's cleared environment drops anyway
        - discovered: with no daemon, `kache stats` prints "no daemon reachable …; starting one inheriting this process's environment" and tries to spawn one (it failed to start here); `kache daemon status --json` does not autostart and, under a scratch `HOME`, reports `service_installed: false`, so the host launchd service is never touched
        - discovered: `kache stats --json --root <dir>` does not filter the aggregate counters; hits and misses are taken as deltas over the previous snapshot instead (the store is private to the test)
        - discovered (the hard way): `kache daemon run` under macOS's per-user `$TMPDIR` (`/private/var/folders/…/T/…/store/daemon.control.v2.sock`) exits with "path must be shorter than SUN_LEN"; the test roots its scratch tree under `/tmp` when the temp dir is longer than 48 bytes (same APFS volume). Recorded in `.claude/skills/os/macos.md` (Paths)
        - discovered: the store's SQLite write-ahead journal (`index.db-wal`) grew ~111 KiB for B's three hits and is only bounded by SQLite's checkpointing, so it is excluded from the churn measure; the rest of the store grew 9,698-9,701 bytes (event log + `.build-sessions`)
        - added fixture `tools/test-toolkit/tests/fixtures/kache-worktree/` — its own workspace (like `scripts/ci/fixtures/shared-deps`): three chained lib crates (`gamma` -> `beta` -> `alpha`), no registry deps, committed `Cargo.lock`; builds offline in about a second
        - added `tools/test-toolkit/tests/real_kache_worktree_restore.rs` (`#![cfg(unix)]`, per-file target as test-toolkit uses Cargo's discovery): `real_kache_restores_a_new_worktree_from_the_shared_store_without_growth` copies the fixture into a scratch Git repo, starts a test-owned foreground `kache daemon run` (readiness = `daemon status --json` running with its socket inside the scratch store; exit detected; `daemon stop` then kill on drop), builds in worktree A (`cargo build --offline --locked`, `RUSTC_WRAPPER=<real kache>`), removes A, creates B, builds again; children run with a cleared environment (inherited `PATH`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`; scratch `HOME`, `XDG_CONFIG_HOME`, `CARGO_HOME`, `TMPDIR`)
        - asserts: A = 0 hits / 3 misses and 3 entries; B = 3 local hits / 0 misses; B's `target/debug/deps` holds the three rlibs; kache `entries` and `disk.store_bytes` unchanged; the on-disk artifact tree (`<store>/store`, minus `staging/` and `*.lock`) identical in bytes and file count; whole-store growth (WAL/SHM excluded) <= 64 KiB. Stats are polled to the final condition (hits + misses reached) with a 10 s deadline, under nextest's 30 s termination
        - gating: skips with a `SKIP:` line when `kache` is not on `PATH` or the scratch filesystem cannot clone (`cp -c` on macOS, `cp --reflink=always` on Linux, other Unixes skip); `BISCUIT_KACHE_REAL_REQUIRED=1` turns either into a failure (checked: `PATH=/usr/bin:/bin` passes as a skip, and fails with the variable set). No `#[ignore]`
        - changed `tools/justfile`: `test-real` was a "not applicable" stub; it now runs `just _test_real test-toolkit`. `[package.metadata.ci.tests] tiers` stays `["L1"]` — `real` is an opt-in local tier, not a CI tier (`KNOWN_TIERS`), and the test-input index already drops `real_` tests from narrowed L1 cells
        - decision: hits come from `kache stats --json` (a public interface) rather than the internal `events.jsonl`; the daemon is test-owned rather than absent because `stats` would otherwise try to start one
        - updated `.claude/skills/kache/SKILL.md` (rusty-biscuit ruling) to name the test, how it runs, and the `stats` autostart trap; the spec's Verification step 3 was left to the author
        - observed on this Mac (APFS, kache 0.26.3), 4 consecutive runs: A misses=3 entries=3 store_bytes=26,115 artifacts=30,593 B/12 files store_dir=184,274 B; B hits=3 misses=0 entries=3 store_bytes=26,115 artifacts=30,593 B/12 files store_dir=193,972 B (+9,698 B); one run saw store_dir 45,009 -> 54,707 B (index not yet checkpointed out of the WAL), same delta. Test time 1.0-2.5 s
        - non-vacuous check: building B with `RUSTFLAGS=-Copt-level=1` (different keys) made the test fail at "worktree B must restore every fixture crate" with B hits=0 misses=3, entries 6, artifacts 61,491 B/24 files; source restored and `touch`ed
        - verified: `just test-real` (in `tools/`) 1 passed, 428 skipped; `just test` (L1 filter) 426 passed, 3 skipped, the real test not selected; bare `cargo nextest run --color=never -p test-toolkit` 427 passed, 2 skipped (a bare run has no tier filter, so it includes the real test, as for every `real_` test in the repo); `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `rustfmt --edition 2024 --check` clean on the new file; `just check-tier-coverage tools` 0 stranded (`test-toolkit` is not an area path, so that spelling checks nothing); `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` clean; host daemon PID 20063 unchanged, no scratch daemon or `/tmp/real-kache-wt-*` left behind
- work completed for 'Worktree restore behavior has no repeatable real-cache test' at 22:13:02
- orchestrator final verification at 22:14:15
        - `cargo nextest run -p test-toolkit -E 'not test(/real_/)'`: 423 passed, 6 skipped; `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `shellcheck scripts/kache-host.sh` clean
        - `just check-tier-coverage tools`: 0 stranded. The package area is `tools`. Earlier log entries that ran `just check-tier-coverage test-toolkit` checked nothing, because `test-toolkit` has no justfile of its own
        - not run: `just cross-check test-toolkit --os linux`. This worktree's `build-linux` clone still has read-only hardlinked files in `target/release` (see review cycle 1). CI's Linux leg is the next place these tests run on Linux

### Successful Completion

The implementation of review cycle 2 has completed successfully in 24 minutes. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred (see reasons below):

- no finding was deferred

The files changed or added during this cycle:

- `scripts/kache-host.sh` (`wrapper` walks ancestor `.cargo/config[.toml]` files the way Cargo does; new `scope=ancestor`)
- `justfile` (`kache-status` ancestor label; `_ensure-kache` now checks the effective wrapper after activation and requires the daemon to report the installed version)
- `tools/justfile` (`test-real` is now a real recipe)
- `tools/test-toolkit/tests/common/kache.rs`, `kache_host_contracts.rs`, `kache_status_contracts.rs`, `kache_ensure_contracts.rs`, `kache_init_contracts.rs`
- `tools/test-toolkit/tests/real_kache_worktree_restore.rs` and `tools/test-toolkit/tests/fixtures/kache-worktree/` (new)
- `docs/initialization.md`, `.claude/skills/kache/SKILL.md`, `.claude/skills/kache/installation.md`, `.claude/skills/os/macos.md`

## Implementation of Review Findings #3

> **started at:** 2026-09-24T02:21:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'A missing wrapper executable is reported as healthy kache' at 02:21:41
        - discovered: nothing checked which binary a kache-named wrapper would run; `scripts/kache-host.sh wrapper` decided "this is kache" from the basename alone. The binary the recipes verify (re-signed by `install-kache`, read by `kache --version`, run by the passthrough probe) is the `kache` on `PATH`, so that is the one a wrapper must resolve to
        - changed `scripts/kache-host.sh`: when the winning wrapper is kache-named, `wrapper` adds `kache-host: wrapper-binary state=certified|missing|not-executable|different path=P certified=C`
                - a bare name is looked up on `PATH`; a value containing `/` or `\` is joined onto the working directory (environment variable) or onto the directory holding the config's `.cargo/` (config file), with `$CARGO_HOME/config.toml` resolving against the parent of `$CARGO_HOME`; on Windows `.exe` is tried for an extensionless path
                - the resolved path must exist and be executable, and must match the `kache` on `PATH` after resolving symlinks and case
                - the name-based `kind=` classification is unchanged, so the neutralize and undo paths still work
        - changed `justfile` `_ensure-kache` step 8: activation now requires `wrapper=kache` **and** `wrapper-binary state=certified`; otherwise it reports `activation INCOMPLETE` plus a `kache NOT ACTIVE` WARNING naming the resolved path, its source, and the reason, and still exits 0 (failure contract)
        - changed `justfile` `kache-status`: an uncertified kache winner prints `active BROKEN — … resolves to P (state)` and adds one drift problem; the verdict is `DRIFT` with exit 1
        - tests (L1, auto-discovered in `tools/test-toolkit/tests/`)
                - `common/kache.rs`: new `install_other_kache()` places a copy of the fake kache outside the fixture `PATH`
                - `kache_status_contracts.rs`: fixed `status_reports_a_kache_config_wrapper_as_active_by_name_or_path` (the nonexistent `/opt/kache/bin/kache` is replaced by the fixture binary's absolute path and a config-relative `bin/kache`); added `status_fails_when_the_kache_wrapper_path_does_not_exist` (the review's exact `RUSTC_WRAPPER=/definitely/missing/kache`, plus a real `cargo check --offline` in a scratch crate that must fail naming the path; it skips only if cargo cannot be started, and it ran here) and `status_fails_when_the_kache_wrapper_is_another_executable`
                - `kache_init_contracts.rs`: added `ensure_reports_activation_incomplete_under_a_missing_kache_wrapper_path` (env var) and `ensure_reports_activation_incomplete_under_another_kache_executable` (parent-directory config)
                - non-vacuous check: with the script temporarily forced to print `state=certified`, all 4 new tests failed; the script was then restored byte-for-byte (`cmp`)
        - docs: `docs/initialization.md` (step 6 and wrapper-precedence paragraph), `.claude/skills/kache/installation.md`, `.claude/skills/kache/SKILL.md` now describe the resolve-and-certify rule; `docs/kache-strategy.md` never described the name-only behavior and is unchanged
        - decisions: the failure path (`kache_off`) and the non-qualifying drift branch still classify by name, because there the question is "is a kache wrapper configured that needs undoing"; when kache is not installed at all, status keeps its existing "not installed" problem instead of adding a second one
        - verified (subagent): `just test` in `tools/` 430 passed, 3 skipped; `just lint` clean; `just check-tier-coverage tools` 0 stranded; `shellcheck scripts/kache-host.sh` clean; plain `just kache-status` on this host still healthy (`state=certified` for `~/.cargo/bin/kache`)
        - verified (orchestrator, 02:27): `shellcheck` clean; `cargo nextest run -p test-toolkit -E 'binary(/kache_/) and not test(/real_/)'` 81 passed, 2 skipped; `RUSTC_WRAPPER=/definitely/missing/kache just kache-status` now prints `active BROKEN` and `VERDICT: DRIFT` and exits 1 (it exited 0 before this change)
        - not run: `just cross-check test-toolkit --os linux`, for the same reason as review cycle 2 (read-only hardlinked files in this worktree's `build-linux` clone). The new code is POSIX bash plus the existing `.exe` handling; CI's Linux leg covers it
- work completed for 'A missing wrapper executable is reported as healthy kache' at 02:27:40

### Successful Completion

The implementation of review cycle 3 has completed successfully in 7 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no finding was deferred

The files changed or added during this cycle:

- `scripts/kache-host.sh` (`wrapper` resolves a kache-named wrapper to an executable and certifies it against the verified `kache` on `PATH`)
- `justfile` (`_ensure-kache` activation requires a certified wrapper binary; `kache-status` reports `active BROKEN` as drift)
- `tools/test-toolkit/tests/common/kache.rs`, `kache_status_contracts.rs`, `kache_init_contracts.rs`
- `docs/initialization.md`, `.claude/skills/kache/SKILL.md`, `.claude/skills/kache/installation.md`

## Implementation of Review Findings #4

> **started at:** 2026-09-24T02:53:39-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- starting the work on 'A legacy Cargo config can hide the wrapper written by init' at 02:54:14
        - discovered (cargo 1.98.1, scratch crate, isolated `CARGO_HOME`): Cargo reads ONE config file per directory — the legacy extensionless `config` whenever it exists — and ignores a `config.toml` beside it, warning only `both … exist. Using …/config`; this holds for `$CARGO_HOME` and for a project `.cargo/` alike. A `config` that is a directory fails Cargo's read (`Is a directory`), and a `config -> config.toml` symlink is followed without a warning
        - discovered: `scripts/kache-host.sh wrapper` read both files per directory, so a wrapper in the ignored `config.toml` was reported (and could win); `_ensure-kache` always wrote, neutralized, and named `$CARGO_HOME/config.toml`, so under a legacy `config` init reported `qualified and set up` while Cargo built without kache
        - decision: activation writes the file Cargo reads rather than reporting `activation INCOMPLETE`. It is simpler (one path variable, no new failure branch), it makes activation actually work, and `kache-config-merge.py` is filename-agnostic TOML merging that keeps a dated backup either way. A refusal would have left a correct, verified setup inactive over a filename
        - changed `scripts/kache-host.sh` `wrapper`: per directory it reads `config` if it exists (anything — a directory then fails as `error=unparseable-cargo-config`, as in Cargo), else `config.toml`; the `$CARGO_HOME` host-file map lists `config` last so a `config -> config.toml` symlink is reported as `$CARGO_HOME/config`, the name Cargo reads; header usage comment updated
        - changed `justfile` `_ensure-kache`: new `host_config` (`$CARGO_HOME/config` when it exists, else `config.toml`) is used by the activation write, the step-8 summary and override list, the `kache_off` neutralization match and write, and the non-qualifying drift undo line; `kache-status`'s host-wide undo line names the same file. Status needed no other change: it reads the helper, which now reports only what Cargo reads
        - tests (L1; `tools/test-toolkit` uses Cargo's default test discovery, no tier markers in the new names)
                - `common/kache.rs`: `write_legacy_cargo_config()` and `cargo_check_runs_kache()` — a real `cargo check --offline` of a scratch crate against the fixture's `CARGO_HOME` with a logging pass-through `kache` first on `PATH`; it panics if the build fails, so "the wrapper did not run" cannot hide a broken build
                - `kache_host_contracts.rs`: `wrapper_reads_only_the_legacy_config_where_both_files_exist` (real Cargo in both `$CARGO_HOME` and a project `.cargo/`; the review's `[term] color = "never"` legacy file), `wrapper_names_a_legacy_config_symlink_as_the_host_file`, `wrapper_reports_a_legacy_config_directory_as_unreadable`
                - `kache_init_contracts.rs`: `ensure_activates_through_a_legacy_cargo_home_config` (activation lands in `config`, its `[term]` table kept, `config.toml` byte-identical, status `YES` naming `config`, and real Cargo runs the wrapper)
                - `kache_ensure_contracts.rs`: `ensure_neutralizes_an_activation_in_a_legacy_cargo_home_config_when_a_step_fails`
                - `kache_status_contracts.rs`: `status_reports_a_kache_wrapper_hidden_by_a_legacy_config_as_inactive` (real Cargo does not run it) and `status_reports_a_kache_wrapper_in_a_legacy_config_as_active` (real Cargo runs it)
                - all real-Cargo halves ran here, none skipped (checked with `--no-capture`)
                - non-vacuous check: with the helper restored to read both files and `host_config` forced to `config.toml`, 6 of the 7 new tests failed. The 7th, `status_reports_a_kache_wrapper_in_a_legacy_config_as_active`, is a positive control (the old code already read `config` first). Both files were restored byte-for-byte (`cmp`) and touched
        - docs: `docs/initialization.md` (step 6, failure contract, precedence paragraph, manual undo), `docs/kache-strategy.md`, `.claude/skills/kache/SKILL.md`, and `.claude/skills/kache/installation.md` now name the legacy file
        - noted, not changed: when `$CARGO_HOME/config` is a symlink to `config.toml`, the merge script's atomic rename replaces the symlink with a regular file. Cargo still reads the activated `config`, so the outcome is correct, but the two files then diverge and Cargo starts warning. That is pre-existing merge-script behavior for any symlinked target
        - verified: `just test` in `tools/` 437 passed, 3 skipped; `just lint` exit 0; `just check-tier-coverage tools` 0 stranded; `shellcheck scripts/kache-host.sh` clean; `just kache-status` on this host still `active YES — ~/.cargo/config.toml`
- work completed for 'A legacy Cargo config can hide the wrapper written by init' at 02:59:27
- starting the work on 'ReFS worktree-base reporting uses the Unix clone probe' at 03:00:06
        - discovered: `do_qualify` was the only direct caller of `clone_probe` outside `clone_check`; `do_report` already used `clone_check` for both the checkout and the base, so `kache-status` needed no change
        - changed `scripts/kache-host.sh` `do_qualify`: the covered-base line now calls `clone_check "$candidate" "$BASE_PATH" base`, reads `clone check store -> base: …`, and on failure appends `(${CLONE_REASON})` so init names the same reason status would; the header's `qualify` usage comment now names `clone_check` instead of "clone-probes"
        - no negative case added: in the fake (and on a real host) a base on the checkout's device is on the same volume, so a same-device non-ReFS base under a ReFS checkout cannot be constructed; the NTFS-checkout control `ensure_leaves_an_ntfs_windows_checkout_without_a_store` already covers the non-ReFS branch of the Windows decision
        - docs: `docs/initialization.md` and `.claude/skills/kache/` describe the probe generically and never quote the base line, so nothing drifted
        - tests (L1, `kache_ensure_contracts.rs`, compiled by Cargo's default discovery): `ensure_reports_a_refs_worktree_base_by_filesystem_type` — emulated Windows ReFS Dev Drive host with `WT` configured to a same-device base and a fake `cp` that fails every reflink into that base (as `cp --reflink=always` does under Git Bash); asserts `base=covered`, the `clone check store -> base: clone` line, and that the fake `cp` logged no clone attempt
        - non-vacuous check: with the base line reverted to `clone_probe`, the new test failed (`… store -> base: FAILED`); the script was restored byte-for-byte (`cmp`) by a plain write and touched
        - verified: `just test` in `tools/` 438 passed, 3 skipped; `just lint` exit 0; `just check-tier-coverage tools` 0 stranded; `shellcheck scripts/kache-host.sh` clean
        - not run: `just cross-check test-toolkit --os windows` — the kache contract binaries are `#![cfg(unix)]`, so a native-Windows run would not execute this test; the Windows branch is proven through the Git Bash emulation above
- work completed for 'ReFS worktree-base reporting uses the Unix clone probe' at 03:01:20
- orchestrator verification at 03:01:48: `shellcheck scripts/kache-host.sh` clean; `cargo nextest run -p test-toolkit -E 'binary(/kache_/) and not test(/real_/)'` 89 passed, 2 skipped

### Successful Completion

The implementation of review cycle 4 has completed successfully in 9 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no finding was deferred

The files changed or added during this cycle:

- `scripts/kache-host.sh` (`wrapper` reads one config file per directory as Cargo does — the legacy `config` when present, else `config.toml`; `qualify` reports a covered worktree base with `clone_check`, so a ReFS base is decided by filesystem type)
- `justfile` (`_ensure-kache` activates, and undoes on failure, in the file Cargo actually reads; `kache-status` names that file in its undo line)
- `tools/test-toolkit/tests/common/kache.rs`, `kache_host_contracts.rs`, `kache_init_contracts.rs`, `kache_ensure_contracts.rs`, `kache_status_contracts.rs`
- `docs/initialization.md`, `docs/kache-strategy.md`, `.claude/skills/kache/SKILL.md`, `.claude/skills/kache/installation.md`

## Implementation of Review Findings #5

> **started at:** 2026-09-24T03:08:38-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- starting the work on 'Invalid worktree-base settings can receive a healthy kache verdict' at 03:08:51
        - read `wt`'s resolver (`worktree/lib/src/config.rs` `resolve_base_dir`) for the exact contract: a non-empty `WT` wins (empty falls through) and must exist, then must not contain a `.git` entry (file or directory); otherwise an absent `~/.worktree.json` is "not configured", a present one must read as UTF-8 and parse with serde_json into `{ base_dir: String }` (missing, null, non-string, or repeated `base_dir` is malformed; unknown keys are ignored; the derived struct also accepts the one-element array form), and its `base_dir` is checked for `.git` *before* existence — the reverse of the `WT` order
        - decided the line format: `kache-host: base=invalid reason=R path=P`, with `reason` placed before `path` so the existing consumers' `${base_line#*path=}` parse still yields the path; `covered`/`off-device`/`unconfigured` lines are unchanged; for `report`, an invalid base prints `kache-host: clone base=- reason=invalid` (no clone check), mirroring `unconfigured`
        - reasons are keyed to `wt`'s error variants: `wt-path-missing`, `wt-path-is-a-git-repository`, `config-malformed`, `config-unreadable`, `config-base-dir-is-a-git-repository`, `config-base-dir-missing`; `P` is the offending path, or `~/.worktree.json` itself for a malformed/unreadable config; a python3 failure other than those is `config-undecidable`
        - deliberate deviation from `wt`: an existing path that is not a directory (`wt` accepts it) is `invalid` with `wt-not-a-directory`/`config-base-dir-not-a-directory`, since kache cannot clone into it and the old script already refused it; recorded in the function's comment
        - changed `scripts/kache-host.sh`: `worktree_base` now sets `BASE_PATH`/`BASE_STATE`/`BASE_REASON` directly (no subshell, so the reason survives) and no longer swallows JSON errors; `resolve_base` turns `configured` into `covered`/`off-device`; `emit_base_lines` adds the reason; `qualify` prints an `INVALID` prose line (reason, path, "kache-status fails until it is fixed") and still exits 0; the header usage comment now documents both base-line shapes
        - changed `justfile`: `_ensure-kache` parses `reason=` and reports `worktree base  INVALID setting (R: P) — wt refuses it; not covered by this verdict, and kache-status fails until it is fixed`; `kache-status` shows the same fact and adds the drift problem `the worktree base setting is invalid (R: P) — wt refuses it, so worktree coverage is undecidable`, so an active host exits 1
        - docs: `docs/initialization.md`, `docs/kache-strategy.md`, and `.claude/skills/kache/installation.md` listed what fails status without the invalid-base case; each gained one clause; no doc enumerated base states otherwise
        - tests (L1, Cargo default discovery, no tier markers): shared `InvalidWorktreeBase` cases plus `configure_invalid_worktree_base`, `write_worktree_config`, `set_wt_env`, `home`, and `host_script` (the `just` command's isolated environment, now shared through `isolated_command`) in `tests/common/kache.rs`; `kache_host_contracts.rs` gained `mod common` and 5 tests — one per case asserting the `report` base line and `clone base=- reason=invalid`, plus `report_resolves_the_worktree_base_by_the_wt_contract` (WT wins over a malformed config, empty WT defers, array form, config base_dir naming a Git repository, missing/null/repeated `base_dir`, WT naming a file, nothing configured); `kache_status_contracts.rs` 4 tests (exit 1, `VERDICT: DRIFT`, drift line and facts line name reason and path); `kache_init_contracts.rs` 4 tests (exit 0, `verdict=qualify`, the invalid base line, the report block line, `qualified and set up`, wrapper activated)
        - non-vacuous check: with `worktree_base`/`resolve_base` restored to the old implementation and the justfile's invalid-base drift removed, all 13 new tests failed; both files were restored byte-for-byte (`cmp`) by a plain write, giving a fresh mtime; the live host still reports `base=covered path=/Volumes/coding/wt`
        - verified: `just test` in `tools/` 451 passed, 3 skipped; `just lint` exit 0; `shellcheck scripts/kache-host.sh` clean; `just check-tier-coverage tools` 0 stranded
- work completed for 'Invalid worktree-base settings can receive a healthy kache verdict' at 03:14:45
- starting the work on 'CI does not select the kache contract tests for their script inputs' at 03:14:45
        - confirmed the gap: `test_input_references` in `scripts/ci/affected_scope.py` drops every source path before the scan, and the recipe-level binaries (`kache_init_contracts`, `kache_ensure_contracts`, `kache_status_contracts`) named only `root.join("scripts")`, a single top-level directory the index refuses as too coarse; `kache_recipe_contracts` read `read("justfile")`, a bare literal the index cannot resolve
        - measured the general alternative (scan every tracked source path, keep references from packages other than the owner): 1,875 source paths matched, almost all through directory-walking guard tests (`claudine::boundary_lint` alone 1,062, `claudine-cli`'s golden-stderr tests 554); rejected as broad scheduling, and it would also have attached `ci_workflow_contracts` to most `scripts/ci/` edits
        - design: a declared, narrow opt-in, `[package.metadata.ci.tests] source-inputs = [...]` on the READING package; a declared source path is added to the test-input scan and only the declarer's references count, so the result is the existing narrowed `{package, ubuntu-latest, L1}` cell with a derived `test_filter`; which tests read the file is still derived from source, not declared
        - planner: `source-inputs` joins `CI_TEST_FIELDS`; `validate_source_inputs` refuses a non-list, duplicates, a non-normalized spelling, a non-source path (already scanned without a declaration), and a missing file (a stale declaration); `package_ci_policy` refuses the declarer's own source; new `declared_source_inputs(policy)` feeds `test_input_references`, whose docstring now states the rule and why
        - trade-off recorded in `docs/cicd/test-inputs.md`: the cross-host narrowed-receipt exception still applies to these cells, although a script can branch on the OS; the nightly full L1 is the backstop and the owning `repo-deps` is still selected on every scheduled environment. Did not add a schema field to make such cells host-bound — that would be a plan/receipt schema bump beyond this finding
        - `tools/test-toolkit/Cargo.toml` declares `source-inputs = ["scripts/kache-config-merge.py", "scripts/kache-host.sh"]`
        - `tests/common/kache.rs`: `RepoInputs::scripts_dir` became `scripts: Vec<PathBuf>`; the fixture checkout's `scripts/` is now a real directory with each listed script symlinked by name (`cargo-path.sh`, `kache-config-merge.py`, `kache-host.sh` — every script the kache recipes run), so each binary's `repo_inputs()` spells both kache scripts through `repo_root()` at crate root and narrows to the whole binary; a recipe that starts running another script now fails in the fixture instead of passing through the directory link
        - `kache_recipe_contracts.rs`: `read` takes a `PathBuf` and both callers spell `read(repo_root().join("justfile"))`
        - Python tests: new `SourceInputSelectionTests` (5 tests: a declared script schedules the declarer's one narrowed Linux L1 cell with the exact filter; undeclared, the same references schedule only the owner; an undeclared sibling script schedules only the owner; a declarer whose own source also changed gains no narrowed cell; 7 malformed declarations refused) and 3 `RealWorkspaceTestInputTests` (each kache script selects its contract binaries; `justfile` selects `kache_recipe_contracts`; every declared source input is spelled by an L1 test of its declarer)
        - non-vacuous check: with `declared_source_inputs(policy)` replaced by `{}`, 5 of the new tests failed (the synthetic declared case, both kache scripts, and both declaration-reachability subtests); restored by a plain write
        - resolved plans (`affected_scope.py --resolved-plan --event pull_request <file>`): (a) `scripts/kache-host.sh` -> `repo-deps` lint/L1 ubuntu + L1 macos, plus `test-toolkit ubuntu-latest L1` narrowed to `kache_host_contracts`, `kache_init_contracts`, `kache_ensure_contracts`, `kache_status_contracts` (previously `repo-deps` only); (b) `scripts/kache-config-merge.py` -> the same `repo-deps` cells plus `test-toolkit ubuntu-latest L1` narrowed to `kache_config_merge_contracts` and the same four binaries; (c) `justfile` -> narrowed `repo-deps` (`ci-build` archive tests) and `test-toolkit ubuntu-latest L1` narrowed to `kache_recipe_contracts`, the four other kache binaries, and four `ci_workflow_contracts` tests
        - docs: `docs/cicd/test-inputs.md` (new "Another package's source" section, the OS caveat, two Limits bullets, pieces table), `.github/ci/README.md` field table, `.claude/skills/rust-devops/ci-cd.md` (selection bullet, a lesson on when declaring a coupling is right), `.claude/skills/rust-testing/SKILL.md` (helper-module literals and `source-inputs`), and `CLAUDE.md`'s CI Structure changed-file paragraph
        - verified: every `scripts/ci/test_*.py` module passes (`test_affected_scope` 384, `test_resolved_plan` 103, `test_ci_local` 101, and the rest); `just _test repo-deps` passed; in `tools/`, `just test` 451 passed, 3 skipped (run before and after the doc edits), `just lint` exit 0, `just check-tier-coverage tools` 0 stranded
- work completed for 'CI does not select the kache contract tests for their script inputs' at 03:29:51
- orchestrator verification at 03:29:51: `shellcheck scripts/kache-host.sh` clean; `cargo nextest run -p test-toolkit -E 'binary(/kache_/) and not test(/real_/)'` 102 passed, 2 skipped; `affected_scope.py --resolved-plan --event pull_request scripts/kache-host.sh` schedules a `{test-toolkit, ubuntu-latest, L1}` cell narrowed to the kache host/init/ensure/status binaries

### Successful Completion

The implementation of review cycle 5 has completed successfully in 21 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- open question for the author: narrowed test-input cells can still be satisfied by an exact-tree run from any host (`docs/cicd/test-inputs.md`). For a changed script that behaves differently by OS, a macOS pre-push run can therefore stand in for the Linux cell. The caveat is documented, but the cells were not tied to Linux, because that would need a new plan/receipt field and a version bump
- deliberate deviation from `wt`: a worktree base that exists but is not a directory is reported as invalid, although `wt` accepts it, because kache cannot clone into a file

The files changed or added during this cycle:

- `scripts/kache-host.sh` (worktree base resolved by `wt`'s contract; an invalid setting is reported as `base=invalid reason=R path=P`)
- `justfile` (`_ensure-kache` reports an invalid base without gating qualification; `kache-status` fails with drift on it)
- `scripts/ci/affected_scope.py`, `scripts/ci/test_affected_scope.py` (`[package.metadata.ci.tests] source-inputs` schedules a declaring package's narrowed L1 tests for another package's changed source)
- `tools/test-toolkit/Cargo.toml` (declares the two kache scripts as `source-inputs`)
- `tools/test-toolkit/tests/common/kache.rs`, `kache_host_contracts.rs`, `kache_init_contracts.rs`, `kache_ensure_contracts.rs`, `kache_status_contracts.rs`, `kache_recipe_contracts.rs`
- `docs/initialization.md`, `docs/kache-strategy.md`, `docs/cicd/test-inputs.md`, `.github/ci/README.md`, `CLAUDE.md`, `.claude/skills/kache/installation.md`, `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/rust-testing/SKILL.md`

## Implementation of Review Findings #6

> **started at:** 2026-09-24T03:38:20-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-dmls/fixes/2026-09-23-ensuring-kache-support/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- starting the work on 'An existing unwritable store directory stops the placement cascade' at 03:38:40
        - confirmed from `scripts/kache-host.sh`: the macOS first branch rejected an existing unwritable `<mount>/kache` only in its `-d && -w` test, then fell to `mkdir -p`, which succeeds on an existing directory, so the directory was chosen and recorded as `CREATED_CANDIDATE`; the shared mount-point attempt had no `-w` test at all. `clone_check` then failed on `mktemp` (`clone-unsupported`) and `drop_created_candidate` removed the pre-existing empty directory. An existing writable directory was also accepted without a device check, so a `kache/` that is itself on another device ended the cascade with `store-device-…`
        - design: one helper, `use_candidate DIR DEVICE`, now decides every placement in `candidate_store` (macOS mount point, user cache dir, shared mount point, user-owned ancestor): an existing directory is used only when writable and on the checkout's device and is otherwise skipped untouched; a missing one is created with `mkdir -p`, recorded in `CREATED_CANDIDATE`/`CREATED_TOP` only then, and removed again (while empty) when it turns out unwritable or off-device. The user cache dir keeps its nearest-existing-ancestor device pre-check so nothing is created off-device, and now also gets the device check on the directory itself (an existing `kache/` that is a symlink or mount onto another device)
        - kept: a writable same-device candidate whose clone probe fails still ends `qualify` with `clone-unsupported`; every later placement is on the same device and filesystem, so trying them would not change the verdict
        - `scripts/kache-host.sh`: new `use_candidate`; `candidate_store` routes all four placements through it (the ancestor loop's own exists/`-w`/`mkdir` branch folded in); the cascade's comment block states that unusable existing placements are skipped untouched
        - `tools/test-toolkit/tests/kache_host_contracts.rs`: `CheckoutVolumeHost` gained `macos()` (fake `uname -s` = Darwin, `stat -f %d` answered too, `cp -c` a plain copy), `place_off_device` (a prefix file replaces the hard-coded off-device home; the default keeps `home` off-device, so the three existing tests are unchanged), and `pre_existing_dir`. Four new L1 tests: `qualify_skips_an_unwritable_mount_point_store_on_macos_and_keeps_it`, `qualify_skips_an_unwritable_shared_mount_point_store_and_keeps_it`, `qualify_skips_an_off_device_mount_point_store_on_macos`, `qualify_reports_no_writable_location_and_keeps_every_unusable_store_directory` (mount point, user cache dir, and ancestor `kache/` all pre-existing at 0500; verdict names `no-user-writable-store-location-on-the-checkout-volume`, all three survive). The permission-dependent three follow the file's existing `permissions_bind` guard, which skips under root (0500 is writable there)
        - non-vacuous check: with the pre-fix `candidate_store` restored in place, all four new tests failed and the other 22 passed; the fixed script was restored by a plain copy and compared byte-for-byte
        - verified: `shellcheck scripts/kache-host.sh` clean; in `tools/`, `just test` 455 passed, 3 skipped, `just lint` exit 0; `just check-tier-coverage tools` 0 stranded
        - not run: `just cross-check test-toolkit --os linux` failed before any test on the build host itself (`target/release/deps/*.rmeta is not writeable` in the standing clone); the Linux path is covered by the emulated-Linux fixture here and by the narrowed `{test-toolkit, ubuntu-latest, L1}` CI cell
- work completed for 'An existing unwritable store directory stops the placement cascade' at 03:42:15
- starting the work on 'An alternate kache config can bypass init's store pin' at 03:42:34
        - investigated with the installed kache 0.26.3 against a scratch `HOME` and scratch configs (live config/daemon untouched): `kache doctor --json` has top-level `schema_version`, `command`, `version`, `rustc`, `issues`, `checks`, `next` and no config-path field anywhere (the text form names only the `Host config` `/etc/kache/config.toml`); `kache daemon --json` carries `daemon_config_path` (the file the running daemon loaded; `null` when none answers) next to `daemon_running`, `service_installed`, `service_path`, `socket`, `daemon_version`, `daemon_epoch`, `service_executable_mismatch`. Its `socket` follows the selected config's store, so a shell with `KACHE_CONFIG` asks a different daemon
        - selector semantics measured: a non-empty `KACHE_CONFIG` selects its file whatever the default file's `ignore_env` says (doctor's `Cache dir` followed the alternate `local_store`); a `KACHE_CONFIG` naming a missing file leaves kache on its built-in default store instead of falling back to the default file; an empty `KACHE_CONFIG` is ignored
        - second trap found on the way: doctor's `Cache dir` detail is `<path> (will be created on first build)` while the store directory does not exist, so `report` published that suffix as part of the store path (a false pin `mismatch`, and a false failure of the new post-write check on a fresh store). `doctor_store` now strips that exact suffix
        - design: the CLI side is read from the selector itself (doctor exposes nothing better) and corroborated by the store doctor resolves after the write (pin must `match`); the daemon side from `daemon_config_path`. New `scripts/kache-host.sh config-source` prints `config-source scope=cli|daemon state=managed|override|unknown selector=KACHE_CONFIG|daemon_config_path|- path=P`, then doctor's store and the managed file's pin line; `report` prints the same two config-source lines before doctor, so `kache-status` reads them too. Paths compare after `~` expansion, `realpath`, and `normcase`
        - `_ensure-kache`: `check_config_source cli` right after the config write (before the passthrough gate and before any daemon work, so no service is installed or restarted under an alternate config) and `check_config_source daemon` after the daemon lifecycle, before activation. An override prints WARNING lines naming `KACHE_CONFIG=<path>` (or the daemon's loaded file) plus the undo, then `kache_off`; a pin that doctor does not resolve, or an undecidable source, is also `kache_off`
        - `kache-status`: an `override` display line per overridden scope and a drift problem naming `KACHE_CONFIG=<path>` (CLI) or the daemon's loaded file; a running daemon that names no config is drift too ("unconfirmed")
        - fixture (`tools/test-toolkit/tests/common/kache.rs`): the fake doctor now resolves like kache does with no `KACHE_*` store variable — the `local_store` of the file it loads (`KACHE_CONFIG` when non-empty, else the managed file), else the built-in default (the fixture store). Two Windows-emulation placement tests had passed only because the old fake ignored the pin; they fail under the new post-write check with the old fake and pass with the faithful one. `set_doctor_store` keeps the one case that models doctor disagreeing with the pin (`status_fails_when_the_pin_names_another_store_than_kache_resolves`, one added line). Also added: `daemon_config_path` in the fake `kache daemon --json` (from `fake-kache/service-config`, applied on start/restart), `set_daemon_service_config`, `set_kache_config_env`, and `place_off_device` on the native host (a `stat` shim sharing the Windows emulation's `off_device` list)
        - tests added (L1): `kache_init_contracts::ensure_leaves_kache_off_when_kache_config_selects_an_off_device_store` (the review scenario: alternate file pinning an off-device store; asserts the WARNING naming `KACHE_CONFIG=<path>`, `kache left OFF`, no activation, no daemon install/start/restart, init's own placement in the managed file, then `kache-status` drift naming the override), `kache_init_contracts::ensure_leaves_kache_off_when_the_daemon_loaded_another_config`, `kache_status_contracts::status_fails_when_kache_config_selects_another_config_file`, `kache_status_contracts::status_fails_when_the_daemon_loaded_another_config_file` (both single-problem `assert_drift` cases: the alternate file pins the same store, so the override is the only drifted fact)
        - test added (real tier): `tools/test-toolkit/tests/real_kache_config_selector.rs`, `real_kache_config_selector_bypasses_the_managed_pin` — the real `scripts/kache-host.sh config-source` against the installed kache in a scratch home: control (managed, pinned store resolved while it does not exist yet, pin `match`, daemon `unknown`), `KACHE_CONFIG` (override, alternate store, pin `mismatch`), and a scratch `kache daemon run` under the selector reporting `daemon_config_path`. Skips without kache; `BISCUIT_KACHE_REAL_REQUIRED=1` fails instead; selected by `tools/test-real`
        - non-vacuous: with the two `check_config_source` calls removed, both new init tests failed; with only the two status `override` arms accepting the override, exactly the two new status tests failed (26 others passed); with the suffix strip removed, the real test failed on the `store=` line. Every file was restored by a plain copy plus `touch` and compared byte-for-byte
        - docs: `docs/initialization.md` (steps 4 and 5, the status drift list), `docs/kache-strategy.md` (the precedence-stack row gains the selector above it), `.claude/skills/kache/configuration.md` (the `KACHE_CONFIG` trap and the doctor suffix), `installation.md` (status facts), `SKILL.md` (failure contract and reference line). The spec's §2 precedence stack does not mention the selector; left for the author
        - verified: `shellcheck scripts/kache-host.sh` clean; in `tools/`, `just test` 459 passed, 4 skipped (the new `real_` test among them), `just lint` exit 0; `BISCUIT_KACHE_REAL_REQUIRED=1 just test-real` 2 passed (scratch store and scratch daemon only; the host's launchd daemon untouched); `just check-tier-coverage tools` 0 stranded
- work completed for 'An alternate kache config can bypass init's store pin' at 03:52:31
- orchestrator verification at 03:53:23: `shellcheck scripts/kache-host.sh` clean; `cargo nextest run -p test-toolkit -E 'binary(/kache_/) and not test(/real_/)'` 110 passed, 3 skipped; live `just kache-status` on the dev Mac exits 0 with the verdict "active on a filesystem that clones blocks", so the new daemon config-source check accepts this host's launchd daemon

### Successful Completion

The implementation of review cycle 6 has completed successfully in 15 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- open questions for the author:
        - `kache-status` treats a running daemon that reports no `daemon_config_path` as drift (the strict reading); a lenient reading would tolerate kache releases that do not report the field
        - the spec's section 2 precedence stack does not mention the `KACHE_CONFIG` config-file selector, which `ignore_env` does not gate; the spec text was left unedited for the author
        - no L1 test covers a running daemon that reports no config path, although both init and status now treat it as drift
- not run: `just cross-check test-toolkit --os linux` failed before any test on `build-linux` (`target/release/deps/*.rmeta is not writeable` in the standing clone); the Linux placement branch is covered by the emulated-Linux fixtures and CI's narrowed `{test-toolkit, ubuntu-latest, L1}` cell

The files changed or added during this cycle:

- `scripts/kache-host.sh` (`use_candidate` accepts an existing store directory only when it is writable and on the checkout's device, records only directories this run created, and continues the cascade; a new `config-source` subcommand and `report` lines compare the CLI's `KACHE_CONFIG` selector and the daemon's `daemon_config_path` with the managed file; `doctor_store` strips the "will be created on first build" suffix)
- `justfile` (`_ensure-kache` checks the CLI config source after the pin write and the daemon's after the lifecycle, leaving kache off on an override; `kache-status` reports an override as drift)
- `tools/test-toolkit/tests/common/kache.rs`, `kache_host_contracts.rs`, `kache_init_contracts.rs`, `kache_status_contracts.rs`, `real_kache_config_selector.rs` (new, real tier)
- `docs/initialization.md`, `docs/kache-strategy.md`, `.claude/skills/kache/SKILL.md`, `.claude/skills/kache/configuration.md`, `.claude/skills/kache/installation.md`
