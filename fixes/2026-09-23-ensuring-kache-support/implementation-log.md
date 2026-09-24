# Implementation Log for 2026-09-23-ensuring-kache-support (5 phases)

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
packages:
    - test-toolkit

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
