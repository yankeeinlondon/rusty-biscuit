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
