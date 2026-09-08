---
total_phases: 10
created: 2026-09-07
phase: 1
agent: claude/default
yolo: "true"
fix: 2026-09-07-faster-claudine-tests
spec: claudine/fixes/2026-09-07-faster-claudine-tests/spec.md
packages:
    - claudine
    - claudine-cli
    - claudine-contract
    - claudine-catalog-types
    - claudine-gen
    - rendezvous-core
    - rendezvous-daemon
    - rendezvous-client
---

# Execution plan — Faster Claudine tests through complete evaluation and explicit fixtures

Converts [spec.md](spec.md) into ten ordered phases. Required behaviors are
referenced as **RB1**–**RB5** and acceptance criteria as **AC1**–**AC7**, in
the order they appear in the spec.

## Grounding facts (verified on this branch, 2026-09-07)

These were checked against the working tree, not assumed. A phase that
contradicts one of them should stop and re-derive rather than proceed.

- **The predecessor has not landed.** `fix/cli-slow-tests` is 191 commits ahead
  of `main`; `claudine/fixes/2026-08-01-cli-slow-tests/` is tracked and its
  implementation (`cd3a28115`, `3e318802d`, `5b5b92bfb`) sits on this branch
  only. Its `log.md` records finding 2 (AC4 CI evidence) and the compile half of
  finding 3 as **deferred, closing by push**. The spec's sequencing rule
  therefore has teeth: Phase 1 is a real gate, not a formality.
- **The spawn burn-down is 170 sites in 36 allow-listed files**, split
  `outside this fix's scope` = 34 files / 168 sites and
  `needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither`
  = 2 files / 2 sites (`wrap_ctrl_c_windows.rs`, `sequence_ctrl_c_windows.rs`).
  Governed population 83 files. The isolation gate is at 0 escapes across 32
  governed files with an **empty** `ISOLATION_ALLOWLIST`.
- **Per-file raw-spawn census** (grep of the three detected forms; the guard's
  own count is authoritative and must be re-read in Phase 5):

  | file | sites | file | sites |
  |---|---|---|---|
  | `context_command.rs` | 24 | `sequence_groups.rs` | 4 |
  | `skills_integration.rs` | 22 | `sequence_magic_reference.rs` | 4 |
  | `compose_schema_cli.rs` | 20 | `sequence_overlay_pty.rs` | 4 |
  | `sequence_cli.rs` | 20 | `effective_diagnostic_render.rs` | 3 |
  | `loop_cli.rs` | 18 | `sequence_prompt_property.rs` | 3 |
  | `wrap_sequence_composition.rs` | 9 | `completion_contract.rs` | 2 |
  | `compose_interactive_timeout_cli.rs` | 5 | `completion_perf.rs` | 2 |
  | `errors_command.rs` | 5 | `composition_outputs.rs` | 2 |
  | `compose_cli.rs` | 4 | `handle_deadline.rs` | 2 |
  | | | `inline_compose_cli.rs` | 2 |
  | | | `level1_structured_error_message.rs` | 2 |

  Single-site files: `completion_resolution_round_trip.rs`,
  `compose_removed_validation_keys.rs`, `compose_system_prompt_lifetime.rs`,
  `compose_ttff_perf.rs`, `handle_blocking_output.rs`,
  `level1_compose_autocomplete_failure_pty.rs`,
  `level1_inline_compose_mismatch_pty.rs`, `protect_cli.rs`,
  `provider_error_finalize.rs`, `sequence_errors_cli.rs`, `sequence_jit.rs`,
  `sequence_sources_cli.rs`, `shipped_prompts.rs`, `wrap_sigint.rs`, plus the
  two Windows files.
- **The live-child cohort is six files, not two.** `.spawn()` on a raw
  `std::process::Command` appears in `wrap_sigint.rs`, `handle_deadline.rs`,
  `compose_ttff_perf.rs`, `wrap_ctrl_c_windows.rs`,
  `sequence_ctrl_c_windows.rs` and `spawn_inventory.rs`; the two
  `level1_*_pty.rs` binaries plus `sequence_overlay_pty.rs` drive
  `expectrl::session::OsSession` through `common/pty.rs`. The builder's
  `build()` returns `assert_cmd::Command`, which has no `spawn` and no way back
  to the inner `std::process::Command`.
- **The environment policy is applied inline in `ClaudineCommandBuilder::build`**
  (`common/mod.rs:453`) — `env_clear` + `restore_windows_console_variables`,
  then `scrub_inherited_environment`, then eleven `.env`/`.env_remove` calls and
  `path_value()`. It is not extractable as-is: it is written against
  `assert_cmd::Command` receivers.
- **Test population is ≈7,300 attributes** across the eight packages
  (`lib` 4086, `cli` 2728 of which 1036 in `cli/tests`, `rendezvous` 296,
  `gen` 159, `contract` 52, `catalog-types` 21). A one-row-per-test inventory is
  not achievable by hand; RB1's "exactly one row **or explicitly enumerated
  family**" has to be mechanically reconciled.
- **Runner overrides in force** (`.config/nextest.toml`): default profile has 9
  per-test `slow-timeout` overrides plus the `package(claudine-cli) &
  test(/level2_/)` blanket; the CI profile has 9 more plus three `test-group`
  bindings (`claudine-l1` max-threads 4, `claudine-cli-ci-l1` max-threads 1,
  `sniff-windows-l1`). Claudine-scoped ones the spec's AC5 governs:
  `compose_loop_rate_limit_pause_waits_then_continues`,
  `agents_and_commands_route_to_empty_state_messages`,
  `composition::loop_engine::tests::seeded_loop_repro_runs_to_completion_with_live_derived_variable`,
  `every_catalog_variable_survives_ambient_options`,
  `exhausted_remediation_fails_finalize_and_preserves_findings`,
  `context_reports_preserve_all_columns_at_minimum_supported_width`,
  `compose_perf_stdout_matches_non_perf`,
  `inline_compose_perf_stdout_matches_non_perf`, the two `claudine-*-l1` test
  groups, and the L2 blanket.
- **Canonical-recipe gaps already visible.** `claudine/justfile`'s `test-real`
  shells out to `cargo test`, not nextest — the only recipe in the area that
  does. `claudine/rendezvous/justfile` defines 8 recipes of the canonical 12
  (no `sanity`, `test-l3`, `test-browser`, `test-real`, `doctest`, `bench`,
  `coverage`, `fuzz`, `all`). `claudine-contract`, `claudine-catalog-types`,
  and all three rendezvous crates carry **no** `[package.metadata.ci.tests]`,
  so their CI tier/feature route is defaulted rather than declared.
- **Bench entry points exist**: `claudine/lib/benches/` holds `claude_parse`,
  `opencode_parse`, `pre_flight_checks`, `prompt_preparation`,
  `runtime_hot_paths`. No `fuzz/` directory in the area.
- **Known-open residual from the predecessor**: `COLUMNS=44 just test-cli`
  reddens `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub`
  and `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`.
  Both are `SPAWN_ALLOWLIST` files; they close by migration (Phase 5A), which is
  exactly what AC3 asks for.
- **Local reference point, not a target**: the predecessor's final run recorded
  `just test-cli` at 2411 passed / 10 skipped in ~13.5 s and `just test-l2` at
  230 passed in ~53 s on a 16-core Mac.

## Assumptions and stated decisions

1. **Phase 1 is human-gated.** Committing, pushing, and merging are separate
   operator actions per `CLAUDE.md`. Phases 2–3 are document-only and may run
   while Phase 1's CI window is open; **no code change from Phases 4–8 may be
   committed until Phase 1's checkpoint passes**, because the same runs are the
   predecessor's evidence and this fix's baseline.
2. **The inventory is family-based with a mechanical completeness proof.** A
   TypeScript reconciler (house convention — the predecessor shipped
   `junit-metrics.ts`) joins `cargo nextest list` output against the inventory's
   declared families and fails on any unassigned or doubly-assigned identity.
   Hand enumeration of 7,300 identities is not a credible deliverable.
3. **The shared environment policy becomes data, not a receiver method.** The
   only way `assert_cmd::Command` and `std::process::Command` can share "one
   shared implementation" is a computed description — clear flag, ordered
   removes, ordered sets, `current_dir` — applied by two thin adapters. Both
   adapters are three lines; the policy is computed once. This is the design
   Phase 4 implements, and the drift test asserts the two adapters produce the
   same effective environment.
4. **`--dry-run`-style scope discipline on assertions.** Assertion repair and
   test-population changes land in their own commits, separate from fixture
   migration, so a reviewer can read either without the other (spec RB3, and
   `CLAUDE.md` § Scope discipline).
5. **`results.md` and `inventory.md` live in this fix directory.** `inventory.md`
   carries the baseline table, the numeric budgets beside it, and the
   dispositions; `results.md` carries measurements, coverage deltas, residual
   findings and the three separate completion claims.
6. **No new override, retry, tier change, or disabled assertion** may be
   introduced at any point (AC5, AC7). `git diff main -- .config/nextest.toml`
   must be inspected at every checkpoint.

---

## Phase 1 — Land the predecessor and open the attribution window

**Gate. Nothing from Phases 4–10 may be committed until this phase's checkpoint
passes.** The spec fixes this sequencing: interleaving would make the
predecessor's deferred evidence measure this fix's changes.

- [ ] Confirm `fix/cli-slow-tests` still carries the predecessor's three
      implementation commits and that `git diff main -- .config/nextest.toml`
      is empty.
- [ ] Run the local gates from the `claudine` package area and record the
      output verbatim: `just lint`, `just test`, `just test-cli`, `just test-l2`.
- [ ] Run `just check-windows` (mingw `x86_64-pc-windows-gnu`, `--tests`) if the
      toolchain is present on the host; record "toolchain absent" explicitly if
      not. This is the predecessor's finding-3 compile half and this fix's AC6.
- [ ] Run `just ci-local --lint-only` then `just ci-local` at the repo root for
      the branch's affected scope, per the repo's pre-push discipline.
- [ ] Hand off to the operator for commit, push, and merge of the predecessor
      to `main`. Record the merge SHA.
- [ ] Collect **three consecutive green CI runs** on `main` after the merge,
      one artifact set per configured leg: `ubuntu-latest`, `macos-latest`,
      `windows-latest`, `wsl2-ubuntu`. Record every intervening failed attempt
      with its cause — selecting only the successful attempts is disallowed.
- [ ] Store the JUnit artifacts under
      `claudine/fixes/2026-09-07-faster-claudine-tests/baseline/<run-id>/`,
      keeping build time, runner elapsed time, and summed test duration as
      three separate columns.
- [ ] Extend or fork the predecessor's `junit-metrics.ts` into this fix
      directory so it reads the four-leg baseline and emits the table that
      `inventory.md` will hold. It must reject malformed reports, missing
      expected artifacts or tests, duplicate identities, invalid durations, and
      failed runs — a script that prints a miss and exits 0 is not a gate.
- [ ] Record in the predecessor's `deferred-performance.md` that its finding 2
      and the finding-3 compile half are now closed (or, if a leg is red, that
      they are not).

**Validation checkpoint 1** — three green runs on four legs exist, their
artifacts are stored, `junit-metrics.ts` reproduces the baseline table from
them, and the predecessor's two deferrals are resolved in writing. Blocked
legs are named as pending rather than assumed.

---

## Phase 2 — Reconciled inventory (RB1, AC1) ‖ runs during Phase 1's CI window

Document-only. No source file changes. Produces `inventory.md`.

- [ ] Build the enumeration substrate: for each of the eight packages, capture
      `cargo nextest list --message-format json` under every feature selection
      that CI or a canonical recipe uses — bare, `daemon-tests`,
      `terminal-tests`, `real-tests` — and under each platform the host can
      enumerate. Record the command, revision, toolchain, and features next to
      each capture.
- [ ] Capture the source-side population separately (attribute scan over
      `#[test]`, `#[tokio::test]`, `#[rstest]`, plus `#[ignore]` and `#[cfg]`
      gates) and diff it against the runner discovery. Every source test that
      the runner never lists is a **cfg/feature exclusion row**, listed
      separately from executed tests with its reason and actual execution route.
- [ ] Inventory the doctest population (`just doctest`) and the five
      `lib/benches` entry points; confirm the area has no fuzz targets and
      record that as a finding rather than an omission.
- [ ] Inventory the rendezvous family explicitly through its own recipes
      (`just test` inside `claudine/rendezvous`, or `just test-rendezvous` from
      the parent area) and record that the parent area's `just test` covers only
      the five claudine crates.
- [ ] Inventory shared fixture machinery as first-class rows: `common/mod.rs`
      (builder + scrub + containment guard), `common/wrap.rs`,
      `common/pty.rs`, `common/completion.rs`, `common/source_scan.rs`,
      `cli/tests/error_guards/`, and the `_ensure-md` area pre-build.
- [ ] Write the per-family rows. Each records: purpose; assertion quality
      (does it distinguish a plausible failure?); shared helpers; required
      effects; CWD / home / cache / environment / tool dependencies; timing
      floor; runner overrides in force; resource ownership; tier, features and
      platforms; canonical recipe; observed cost with provenance; disposition.
- [ ] Enumerate family membership explicitly. A family row is valid only when
      its members are listed and share the same setup and proof; anything that
      differs gets its own row.
- [ ] Write the runner-override census as its own table: all 18 per-test
      `slow-timeout` overrides plus the three `test-group` bindings and the L2
      blanket, each marked *justified* (with the contract its floor expresses)
      or *remove with the cost it hides* (AC5).
- [ ] Record the recipe/metadata reconciliation findings: `test-real` bypassing
      nextest, rendezvous's 8-of-12 canonical recipes, and the five crates with
      no `[package.metadata.ci.tests]`.
- [ ] Implement the completeness reconciler (TypeScript, in this fix directory).
      It reads the nextest listings plus the inventory's declared families and
      **fails** on any identity assigned to zero or more than one row. Run it
      and paste its output into `inventory.md`.
- [ ] State the disposition of every row as one of: satisfactory, remediation in
      this fix, or linked follow-up naming the unmet requirement and the reason
      for deferral. No row may be dispositioned by timing threshold.

**Validation checkpoint 2** — the reconciler exits 0; the inventory covers all
eight packages, all tiers, doctests, benches, excluded/ignored tests and shared
fixtures; every row has a disposition; the override census is complete.

---

## Phase 3 — Attribution and ratified budgets (RB5 first half; resolves the spec's four draft decisions)

Document-only. Depends on Phases 1 and 2.

- [ ] Attribute cost by family against the Phase 1 baseline, keeping build,
      elapsed, and summed-duration columns separate. Answer draft decision 1
      — *which non-spawn families account for the remaining execution cost* —
      with numbers, not intuition.
- [ ] Measure the source-scan families (`error_guards`, `test_placement`,
      `dispatch_inventory`, `spawn_inventory`, `spawn_site_guard`) individually.
      Nextest runs each in its own process, so a process-local cache shares
      nothing across cases; record the per-binary scan cost and the number of
      processes that repeat it. Answer draft decision 2 — *which scans can share
      work without losing independent failure detail*.
- [ ] Measure the context/render families (`context_command`,
      `composition_seams`, `every_catalog_variable_survives_ambient_options`,
      the loop pause tests) and the corpus loaders (`shipped_prompts`,
      `shipped_prompt_contract`, `shipped_prompt_route_drift`).
- [ ] Enumerate the live-child cohort and decide, per file, whether it still
      needs an exception once Phase 4 ships a raw-command path. Answer draft
      decision 3 — *which technical exceptions remain necessary*.
- [ ] Ratify per-family numeric budgets **on the existing CI runners**, derived
      from the baseline and written into `inventory.md` **beside the baseline
      table**, so budget review and evidence review are one act. Answer draft
      decision 4. Local timing may attribute cost; it may not set a target.
- [ ] Record the timeout/clock semantics inherited from the implemented
      [startup-stall fix](../2026-08-31-silent-success-and-startup-stall/spec.md):
      every floor and budget recorded here is derived under its spawn-fallback
      silence clock, not the pre-fix first-event grace.
- [ ] For any production defect surfaced during attribution, open a linked
      issue/spec with the evidence rather than fixing it here.

**Validation checkpoint 3** — every draft decision has a written answer backed
by a measurement; budgets are in `inventory.md` next to the baseline; no budget
was derived from a local run alone.

---

## Phase 4 — One environment policy, two command surfaces (RB2 infrastructure)

First code phase. Requires checkpoint 1. Everything downstream of the builder
depends on this, so it lands alone.

- [ ] Extract the inline policy in `ClaudineCommandBuilder::build` into a
      computed description — clear flag, Windows console restores, ordered
      removes (`CLAUDINE_*` by prefix, `GIT_PLUMBING_VARS`, the three render
      inputs, `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME`), ordered sets (home
      variables, `PATH`, `CLAUDINE_RENDEZVOUS_REPORT`, `NO_COLOR`), and
      `current_dir`. Preserve the existing ordering contract exactly: scrub
      before defaults, so `CLAUDINE_RENDEZVOUS_REPORT` survives its own
      namespace sweep, and a per-key `.env` after `build()` still wins.
- [ ] Add two thin adapters that apply that description — one to
      `assert_cmd::Command`, one to `std::process::Command`. `build()` keeps its
      signature and behavior; the new `command_std()` / `command_builder()
      .build_std()` yields a `std::process::Command` carrying the same policy.
- [ ] Add a drift test proving the two surfaces produce the same effective
      environment: run the recording stub through both paths and compare the
      captured key/value sets, including the Windows arm via `cfg!(windows)`
      rather than `#[cfg]` so both arms compile everywhere.
- [ ] Keep the named escapes working on the raw path: `fake_only_path()`,
      `host_path()`, `ambient_context(dir)`, `inherit_no_env()`, and the
      `checkout_containment_error` precondition — including after
      canonicalization, so a fixture root can never sit inside the checkout.
- [ ] Teach `spawn_site_guard.rs`'s **spawn** gate that the builder's raw path
      is a sanctioned form, and that a raw `std::process::Command::new(
      claudine_bin())` outside it is still a violation. Add detector unit tests
      for both, including negatives (`bin_exe!("md")`, prose, string literals).
- [ ] Teach the **isolation** gate the resulting command forms: a
      `.current_dir` / `.env("PATH", …)` / `.env_remove("PATH")` /
      `.env_clear()` / `augmented_path` on a raw fixture command is the same
      escape it already flags on the `assert_cmd` one.
- [ ] Widen the isolation gate's governed population from 32 files toward the
      rest of the L1 suite, as each file leaves `SPAWN_ALLOWLIST` in Phase 5.
      Where a governed file legitimately targets a *different* command
      (parent-side `git`, `rustc`, `md`), resolve it with an
      `ISOLATION_ALLOWLIST` entry naming that command — **never** by weakening
      the detector or dropping stale-entry failure.
- [ ] Harden the helper commands too: audit every parent-side
      `Command::new("git")` / `rustc` / `md` invocation in `common/` and the
      fixtures for inherited Git plumbing; isolating only the claudine child
      does not protect those.
- [ ] Prove non-vacuity for each new or changed guard arm: apply a neuter, show
      the named failure, restore, and `diff` the file back to identical.
      Transcribe into the log.
- [ ] Compile-verify the Windows-only arms with `just check-windows` where the
      mingw toolchain is present; otherwise name `windows-latest` as the
      authority and record the check as pending.

**Validation checkpoint 4** — `just test-cli` and `just lint` green from the
`claudine` area; `just test-l2` green (Phase 4 touches `common/`, which every
L2 binary compiles); both guards' censuses printed and unchanged except for the
new sanctioned form; `git diff main -- .config/nextest.toml` empty.

---

## Phase 5 — L1 spawn burn-down (RB2, AC2)

Four batches. **5A, 5B and 5C are mutually parallelizable** — they touch
disjoint file sets and each ends by deleting its own `SPAWN_ALLOWLIST` entries.
**5D depends on Phase 4's raw-command path** and is the only batch that may
need a new allow-list reason. Each batch ends green before the next merges.

### Phase 5A — compose / composition family (‖ with 5B, 5C)

- [ ] Migrate `compose_cli.rs` (4), `compose_schema_cli.rs` (20),
      `compose_removed_validation_keys.rs` (1),
      `compose_system_prompt_lifetime.rs` (1), `composition_outputs.rs` (2),
      `inline_compose_cli.rs` (2) to `CliProcessFixture::command()` /
      `command_builder()`, each escape carrying a call-site comment naming the
      tool or proof it needs.
- [ ] Delete those files' `SPAWN_ALLOWLIST` entries; confirm the stale-entry arm
      would fire if an entry were left behind.
- [ ] Re-run the predecessor's inherited-width probe: `COLUMNS=44 just test-cli`
      must no longer redden
      `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub`
      or `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`.
      This is AC3's named case.

### Phase 5B — sequence / loop family (‖ with 5A, 5C)

- [ ] Migrate `sequence_cli.rs` (20), `loop_cli.rs` (18),
      `wrap_sequence_composition.rs` (9), `sequence_groups.rs` (4),
      `sequence_magic_reference.rs` (4), `sequence_prompt_property.rs` (3),
      `sequence_errors_cli.rs` (1), `sequence_jit.rs` (1),
      `sequence_sources_cli.rs` (1).
- [ ] `sequence_cli.rs` alone carries ~20 `.env("PATH", augmented_path(…))`
      pairs. Replace each with the named escape that expresses its actual need
      (`host_path()` where a real tool is the subject, `fake_only_path()` where
      absence is the assertion, default otherwise) — do not carry the raw pair
      across.
- [ ] Delete the batch's `SPAWN_ALLOWLIST` entries.

### Phase 5C — context / errors / completion / resources family (‖ with 5A, 5B)

- [ ] Migrate `context_command.rs` (24), `skills_integration.rs` (22),
      `errors_command.rs` (5), `effective_diagnostic_render.rs` (3),
      `completion_contract.rs` (2), `completion_perf.rs` (2),
      `handle_deadline.rs` (2), `level1_structured_error_message.rs` (2),
      `completion_resolution_round_trip.rs` (1), `compose_ttff_perf.rs` (1),
      `handle_blocking_output.rs` (1), `protect_cli.rs` (1),
      `provider_error_finalize.rs` (1), `shipped_prompts.rs` (1).
- [ ] `handle_deadline.rs` and `compose_ttff_perf.rs` hold a live child — route
      them through Phase 4's raw-command path rather than `assert_cmd`.
- [ ] Delete the batch's `SPAWN_ALLOWLIST` entries.

### Phase 5D — live-child cohort (depends on Phase 4)

- [ ] Route `wrap_sigint.rs` (1) through the raw-command path; the call site
      keeps only its subject — signal delivery, output draining, child reaping.
- [ ] Route `common/pty.rs`'s session construction through the raw-command path
      so `level1_compose_autocomplete_failure_pty.rs`,
      `level1_inline_compose_mismatch_pty.rs` and `sequence_overlay_pty.rs`
      inherit the policy; verify the `#[cfg(unix)]` gate on `mod pty;` still
      holds and that `expectrl` receives a configured `std::process::Command`.
- [ ] Route `wrap_ctrl_c_windows.rs` and `sequence_ctrl_c_windows.rs` through
      the raw-command path. The call site keeps `CREATE_NEW_PROCESS_GROUP`, the
      targeted console signal, the readiness poll, and the reaping — nothing
      else. **Do not** move these to L3 and **do not** drop Windows coverage.
      This closes the `NEEDS_LIVE_CHILD` reason and the environment hazard the
      predecessor's log recorded but did not fix (an inherited `CLAUDINE_TIMEOUT`
      producing a silent false pass).
- [ ] Delete `NEEDS_LIVE_CHILD` and its two entries once no site cites it.
- [ ] Compile-verify with `just check-windows` where the mingw toolchain is
      present; name `windows-latest` as the authority otherwise.

### Phase 5E — burn-down closure

- [ ] Assert `SPAWN_ALLOWLIST` contains **zero** generic
      `outside this fix's scope` entries (AC2). Any survivor carries a specific
      technical necessity **and** equivalent isolation proof, written at the
      entry.
- [ ] Confirm the isolation gate's governed population now covers the migrated
      files and that `ISOLATION_ALLOWLIST` entries, if any, each name the
      non-claudine command their site targets.
- [ ] Keep the negative guard tests and the stale-exemption failure arms; add a
      neuter transcript for any arm whose behavior changed.
- [ ] Add the contamination probes AC3 requires, using **disposable state only**
      — never an edit to the real checkout or the user's configuration:
      application variables (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`), Git
      plumbing (`GIT_DIR`/`GIT_WORK_TREE` pointed at a throwaway repo),
      home/cache relocation, `PATH`, inherited width/color (`COLUMNS=44`,
      `FORCE_COLOR=1`), and a checkout-ancestor `TMPDIR`. Each probe must leave
      unrelated test results unchanged.

**Validation checkpoint 5** — `just test-cli`, `just test`, `just lint` and
`just test-l2` green from the `claudine` area; the burn-down artifact
(`$STAGE/spawn-site-burn-down.jsonl`) shows zero generic exemptions; every
contamination probe passes; test count reconciles (additions and removals
reported separately, never netted).

---

## Phase 6 — Non-spawn cost and test quality (RB3) ‖ partially with Phase 5

Touches different files from Phase 5 for the most part; the loop/context items
must land **after** 5A/5B if they touch the same binaries.

- [ ] Inspect every consumer of the expensive context/discovery helpers
      (`ComposeContext::capture`, `LaunchContext`, repository discovery).
      Substitute synthetic context where discovery is incidental; **retain real
      discovery against test-built repositories where it is the assertion's
      subject**.
- [ ] Keep real shipped-prompt coverage through an isolated copy of the corpus
      with relative references preserved — do not replace the shipped artifact
      with a synthetic stand-in.
- [ ] Consolidate the repeated source scans **only** where coverage, diagnostic
      attribution, and selective execution survive. Where justified, use the
      established shape: one shared passive corpus binary that scans once and is
      *extended* per regression, rather than a new rescanning process per case,
      with representative end-to-end cases retained through real shipped
      artifacts.
- [ ] Re-measure the loop pause and context rendering families against Phase 3's
      attribution; report what actually dominated rather than what was assumed.
- [ ] Work through the override census from Phase 2. Each survivor gets a
      justification tying its floor to a real contract; each other one is
      **removed together with the cost it was hiding** — including the 30 s
      entry on `context_reports_preserve_all_columns_at_minimum_supported_width`.
      Adding a new override is disallowed (AC5, AC7).
- [ ] Repair tautological assertions and stale test identities **in separately
      reviewable commits**. For each, record the original failing input where
      one exists, the defect the old assertion could not distinguish, and the
      additional failure the replacement now detects.
- [ ] Move exhaustive value/representation combinations to the cheapest boundary
      that proves them, retaining representative real-binary cases. Record every
      test-population change with its replacement coverage.
- [ ] Reconcile the recipe/metadata findings from Phase 2: bring `test-real`
      onto nextest or record why it cannot be; declare
      `[package.metadata.ci.tests]` for `claudine-contract`,
      `claudine-catalog-types`, and the three rendezvous crates, or record the
      deliberate default; close or link the rendezvous canonical-recipe gap.
- [ ] Run `just check-tier-coverage` and `just check-test-interrupts` after any
      tier-marker or recipe change.

**Validation checkpoint 6** — `just test`, `just test-cli`, `just test-gen`,
`just test-contract`, `just doctest`, `just bench` and `just lint` green;
`just test-rendezvous` green; the override census in `inventory.md` shows every
entry justified or removed; `git diff main -- .config/nextest.toml` shows
removals only.

---

## Phase 7 — Bounded time and resource ownership (RB4) ‖ with Phase 6

- [ ] Replace every readiness sleep with bounded observation of the **final
      required condition**, with a deadline. Audit all 7 L1 sleep sites in
      `cli/tests` (`common/pty.rs` ×5, `wrap_sigint.rs`, `completion_perf.rs`,
      and the Windows pair's 3 each) and the lib-side sleeps in
      `composition/sequence/task/tests.rs` (8), `render/assistant_stream.rs`
      (4), `composition/sequence/task/shell.rs` (2), and the singletons in
      `render/thinking_stream.rs`, `model_catalog/`, `config/atomic.rs`,
      `composition/looping/engine.rs`.
- [ ] Retain sleeps that *are* the timeout contract, and justify each budget,
      polling cadence, and shutdown margin in the inventory row — derived under
      the startup-stall fix's spawn-fallback silence clock.
- [ ] Give daemon / session / IPC tests isolated endpoints, data directories,
      processes and cleanup. Cover `rendezvous-daemon`'s `pairing_and_sync.rs`,
      `peer_discovery.rs`, `phase6_integration.rs`, `rendezvous-client`'s
      `local_round_trip.rs` and `session_log_round_trip.rs`, and the
      `daemon-tests`-gated CLI targets.
- [ ] Confirm unrelated tests disable reporting (`CLAUDINE_RENDEZVOUS_REPORT`
      is already a builder default — verify it reaches the raw path too).
- [ ] Prove cleanup on panic, error and cancellation for the process-owning
      cohorts with **nextest's per-test leak policy plus the root
      `just test-leaks` post-run sweep** — inspection alone is not proof.
- [ ] Reduce runner-visible serialization to resources that are actually shared;
      remove `#[serial]` where the resource is per-test. Note that
      `serial_test` is a no-op across nextest processes, so anything genuinely
      shared needs runner-visible coordination instead.
- [ ] Verify no terminal or browser gains focus in any tier touched.

**Validation checkpoint 7** — `just test-leaks` at the repo root reports no
survivors; `just test-daemon` and `just test-rendezvous` green; the ten reruns
scheduled in Phase 9 have a stable target set recorded.

---

## Phase 8 — Local measurement (RB5, first evidence tranche)

Requires Phases 5–7 complete and green.

- [ ] Warm each revision's artifacts, then collect **five alternating warm local
      runs per revision** for every changed cohort and for the relevant full L1
      suites. Alternate baseline/candidate on the same host with matching
      toolchain, features, profile, concurrency and fixture inputs. Record
      revision, dirty state, platform, cache state, and the exact commands.
- [ ] Re-run every changed timeout or concurrency case **ten times under
      representative suite load**, not in isolation.
- [ ] Measure any cold-build claim in an isolated build directory — never by
      clearing the developer's working cache.
- [ ] Prove eliminated discovery and unrelated launches with **work counters or
      sentinel effects**, independently of timing. A timing improvement is not
      evidence that a walk was removed.
- [ ] Keep the three costs separate in every table: build/setup, runner elapsed,
      summed test duration. Track identities, counts, failures, skips, timeouts,
      retries and slow cases alongside speed so lost coverage cannot read as an
      optimization.
- [ ] Record local numbers as **attribution only**; they establish no CI target.

**Validation checkpoint 8** — five alternating runs exist per changed cohort;
ten reruns exist per changed timeout/concurrency case; each eliminated-work
claim has a counter or sentinel behind it.

---

## Phase 9 — CI evidence (RB5, second tranche; AC6)

Human-gated like Phase 1: push and read.

- [ ] Run `just ci-local --lint-only`, then `just ci-local`, for the affected
      scope before pushing.
- [ ] Run `just check-windows` where the mingw toolchain is present; state the
      limitation explicitly where it is not.
- [ ] Push and collect **three consecutive candidate CI runs for each configured
      package/environment leg** — `ubuntu-latest`, `macos-latest`,
      `windows-latest`, and the WSL2 leg. Record every intervening failed
      attempt and its cause.
- [ ] Compare **matched tests within each environment** against that
      environment's own baseline. Report additions and platform exclusions
      separately; do not require identical cross-platform counts.
- [ ] Run `junit-metrics.ts` as the gate over both baseline and candidate sets;
      it must fail on malformed reports, missing artifacts or tests, duplicate
      identities, invalid durations, and failed runs.
- [ ] Compare against the Phase 3 budgets. **Report misses and their causes;
      do not invent a universal speedup percentage** and do not close a miss by
      adjusting the budget after the fact.
- [ ] Run the affected L2/L3/real tiers through their canonical recipes only
      where the resources exist (`just test-l2`, `just test-l3`,
      `just test-real`); record unavailable runtime evidence as **pending**, not
      as passing.

**Validation checkpoint 9** — three consecutive green runs exist per leg, with
failures disclosed; every budget is met or its miss is explained; no override,
retry, tier change, or disabled assertion was used to reach a number.

---

## Phase 10 — Closure: `results.md`, drift, acceptance sweep

- [ ] Write `results.md` with: the measurements (baseline and candidate, three
      costs separate, per leg); coverage changes (tests added, removed, moved
      boundary, replacement coverage for each changed assertion); residual
      findings; and **separate** implementation / verified-locally / verified-on-CI
      completion status.
- [ ] Give every deferred finding evidence, a reason, and a linked owner
      document. Generic fixture migration may **not** be deferred (AC4).
- [ ] Update area docs and the `rust-testing` / `claudine` skills **only where
      workflow or architecture changed** — the raw-command builder surface and
      the widened isolation-gate population are the likely candidates.
      `CLAUDE.md` § Drift Maintenance governs.
- [ ] Sweep the acceptance criteria explicitly, one subsection each:
  - [ ] **AC1** — every discovered test/family has a reviewed disposition and a
        reconciled platform/feature/tier route; none omitted by timing threshold
        (reconciler output attached).
  - [ ] **AC2** — zero generic residual spawn exemptions; live-child and
        ordinary paths share the policy; negative guard tests and Windows proof
        present.
  - [ ] **AC3** — the two inherited-width failures are covered; every
        contamination probe cannot alter unrelated results; probes used only
        disposable state.
  - [ ] **AC4** — shared-setup, cleanup, assertion and reachability findings in
        scope are resolved; deferrals are evidenced and linked.
  - [ ] **AC5** — every pre-existing override is justified in the inventory or
        removed with the cost it hid; none was added.
  - [ ] **AC6** — local gates pass, `just check-windows` result recorded,
        platform limitations explicit, budgets have compatible CI evidence.
  - [ ] **AC7** — `results.md` complete; docs and skills updated only where
        workflow or architecture changed.
- [ ] Final gate run from the `claudine` package area: `just lint`, `just test`,
      `just test-cli`, `just test-l2`, `just doctest`, `just bench`,
      `just test-rendezvous`; plus `just test-leaks` at the repo root.
- [ ] Confirm `git diff main -- .config/nextest.toml` contains removals only.

**Validation checkpoint 10** — all seven acceptance criteria are answered with
evidence or an explicitly linked deferral; `results.md` keeps the three
completion claims separate; no gate was weakened to close a criterion.

---

## Parallelism map

| Can run concurrently | Why it is safe |
|---|---|
| Phase 2 ‖ Phase 1's CI window | Phase 2 is document-only; it lands no code, so it cannot contaminate the predecessor's attribution window. |
| Phase 5A ‖ 5B ‖ 5C | Disjoint file sets, each deleting only its own allow-list entries. The guard's stale-entry arm catches a mis-merge. |
| Phase 6 ‖ Phase 7 | Different concerns (cost/quality vs. time/ownership) and largely different files. Serialize only where both touch a binary Phase 5 also touched. |
| Phase 6/7 ‖ Phase 5D | 5D is confined to the live-child cohort plus `common/pty.rs`. |

**Strictly serial**: Phase 1 → Phase 4 (no code lands before the baseline
window closes); Phase 4 → Phase 5D (the raw-command path must exist); Phases
5–7 → Phase 8 → Phase 9 → Phase 10 (evidence follows the change it measures).

## Dependency order (summary)

```text
1 ──┬─→ 4 ─→ 5A ─┐
    │        5B ─┤
    │        5C ─┼→ 5E ─┬→ 8 ─→ 9 ─→ 10
    │        5D ─┘      │
    └─→ 2 ─→ 3 ─────────┴→ 6 ‖ 7 ──┘
```

## Out of scope (guard rails)

Named so a phase does not quietly widen. Anything here that turns out to be
necessary becomes a linked follow-up, not an in-flight expansion:

- production behavior changes, new provider features, wholesale CLI/library
  restructuring, CI runner redesign, generalized cross-package fixture
  frameworks;
- a second mechanical rewrite of the 29 already-migrated binaries — they get
  evaluation and regression verification only;
- moving Windows console-control tests to L3 or dropping Windows coverage to
  close the live-child exemption;
- weakening the isolation detector or dropping stale-entry failure to resolve a
  false positive — the sanctioned resolution is an allow-list entry naming the
  command the site targets;
- any new `slow-timeout` override, retry, tier change, or disabled assertion as
  a substitute for a fix;
- fixing a production defect found during attribution in place of filing it.
