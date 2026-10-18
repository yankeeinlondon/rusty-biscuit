---
kind: inventory
area: claudine
feature: 2026-10-25-loops-and-flow-control
plan_task: Coverage inventory (Phase 1, Wave 1)
created: 2026-10-16
---

# Coverage inventory

This is a read-only inventory of the worktree at
`feat/osc-9-4-protocol`. Nothing was built or run except `md hash`. All paths
are repository-relative. A `file:line` reference points at the line that
proves the claim.

## 1. Existing tests by subject

### 1.1 Document loops (`claudine compose` / `inline-compose`)

| Location | Binary / tier | Covers |
|---|---|---|
| `claudine/cli/tests/l1/loop_cli.rs` (18 tests, `#[cfg(unix)]`, `l1/main.rs:129`) | `claudine-cli` `l1`, L1 | iteration counting, `--max-iterations` / `CLAUDINE_MAX_ITERATIONS` precedence (`:167`, `:218`), cap exceeded (`:498`), step timeout (`:563`), rate-limit abort/pause (`:642`–`:890`), loop `fail_fast` true/false and the deprecated `FAIL_FAST` env (`:268`, `:387`, `:444`), `sequence_does_not_publish_a_fail_fast_env` (`:326`), exit reasons (`:1052`, `:1115`) |
| `claudine/cli/tests/l1/loop_gate_ambient.rs` (6, gated on `test-fixtures`, `l1/main.rs:133`) | `l1`, L1 | `loop:` gate concerns render the `loop` global, including on the ending pass; reads of retired `_loop_*` names fail before launch |
| `claudine/cli/tests/l1/loop_initialize_state.rs` (1) | `l1`, L1 | `initialize` uses the document namespace and keeps runtime writes |
| `claudine/cli/tests/l1/lifecycle_shell_templates.rs` (15, `test-fixtures`) | `l1`, L1 | template-approved body `::shell` across loop iterations, lifecycle `shell`/`set` |
| `claudine/cli/tests/l1/ctx_per_run.rs:329` | `l1`, L1, portable | a looping document's `ctx` per iteration in a scratch Git repo |
| `claudine/cli/tests/level2/level2_lifecycle_loop.rs` | `level2`, L2 (tmux) | gate order and `finalize` count, blocked first iteration, `initialize` skip, gate error |
| `claudine/lib/tests/l1/single_loop_engine.rs` (2) | `claudine` `l1`, L1 | source guard: one post-checked engine; no `execute_loop*` entry point returns |
| `claudine/lib/src/composition/looping/config/tests.rs` (43) | in-crate | loop config grammar: aliases, conditions, `max`, `fail_fast` type (`:223`), unknown keys |
| `claudine/lib/src/composition/looping/actions/tests.rs` (41) | in-crate | mutation DSL (`increment`, `set`, `append`, …) |
| `claudine/lib/src/composition/looping/engine/tests/*.rs` (60 total) | in-crate | `iteration_actions.rs` (16: predictive `is_last` at `:179`–`:202`, loop `fail_fast` at `:241`, `:282`, `:309`), `lifecycle_control.rs` (17, `:948` cancellation with `fail_fast: false`), `rate_limits.rs` (7), `seed_state.rs` (7), `iteration_bindings.rs` (5), `removed_loop_reads.rs` (4), `gate_ambient.rs`, `post_checked_timing.rs` |
| `claudine/lib/src/composition/looping/expression/tests.rs` (13 + 1) | in-crate | loop expression scope and resolution context |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/*.rs` (~126) | in-crate (cli) | lifecycle ordering (19), terminal evaluation/routing (19/9), retry/resume (12), overlays (31), proxy, shell approval (6), budget scoping; `target_launch/tests.rs` (37) |

### 1.2 Lifecycle stacks and control directives

| Location | Covers |
|---|---|
| `claudine/lib/src/composition/lifecycle/tests/*.rs` | `action_shape_control.rs` (95), `grammar.rs` (64; `:894` asserts `defer`, `prep`, `break` handling — `prep`/`break` are still rejected), `parse_config.rs` (26), `guard_runtime.rs` (18), `nested_span.rs` (16), `diagnostics.rs` (15), `validation.rs` (12), `container_runtime.rs` (8) |
| `claudine/lib/src/composition/lifecycle/executor/tests/*.rs` | action dispatch (23), `proxy_with_evaluation.rs` (21), event-time interpolation (20), `runtime_set.rs` (18), `set_shell_values.rs` (18; `:542` duplicate keys), `conditions_control.rs` (17), `retry_with_evaluation.rs` (7), `shell_templates.rs` (8) |
| `claudine/lib/src/composition/lifecycle/control/tests.rs` (27), `context/tests.rs` (39), `actions/tests.rs` (21), `bindings/tests.rs` (12) | control signals, binding scopes |
| `claudine/lib/tests/l1/lifecycle_control_flow_spike.rs` (4) | differential evidence for the control model against production APIs |
| `claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs` (8) | parser/action-model slice stays liftable into Darkmatter |
| `claudine/lib/tests/l1/claudine_schemas.rs` (33), `claudine_schemas_matrix.rs` (4) | schema bundle under `claudine/schemas/` agrees with the parser's verb inventory; schema robustness matrix |
| `claudine/cli/tests/l1/compose_initialize_acceptance.rs` (`#[cfg(unix)]`, `l1/main.rs:43`) | `initialize` `stop`, `error`, proxy chains, retry, resume, approval of commands reachable after `initialize` |
| `claudine/cli/tests/l1/lifecycle_*.rs` | `lifecycle_downgrade_outcome`, `lifecycle_literal_escapes`, `lifecycle_message_drain*`, `lifecycle_mixed_span_validation` (unix), `lifecycle_set_shell_values` |
| `claudine/cli/tests/l1/compose_retry_overlay.rs`, `provider_error_finalize.rs`, `handoff_owners.rs` | `retry.with`, finalize after provider error, handoff ownership |
| `claudine/cli/tests/level2/level2_lifecycle_{action_forms,control,dispatch,ctrl_c_tmux}.rs` | L2 lifecycle rendering; `level2_lifecycle_control.rs:5318` is the shipped `implement` route row (prompt-tests only, see section 3) |

`break` and `prep` are still **planned**: `claudine/schemas/lifecycle.yaml:168-169`
and `claudine/schemas/action.yaml:81,84,257-258,419-420` say "not yet accepted
by the parser". No runtime test of either exists.

### 1.3 Sequences and group containers

| Location | Binary | Covers |
|---|---|---|
| `claudine/cli/tests/l1/sequence_containers.rs` (20, ungated; Windows uses `claudine-fake-relay`) | `l1` | root events wrap children once, `seq.outputs` shape (`:34`), root retry clears abandoned outputs and finalizes once (`:75`), `recursive_dry_run_fires_no_container_events` (`:112`), blocked cleanup refusing shells, proxy cleanup, frozen approval on retry |
| `claudine/cli/tests/l1/sequence_frozen_graph.rs` (1), `sequence_referenced_state.rs` (3), `sequence_deep_cancelled.rs` (2) | `l1` | frozen child graph, referenced document state layering, forbidden `cancelled` actions three levels deep |
| `claudine/cli/tests/l1/sequence_cli.rs` (30, ungated) | `l1` | orchestration, summaries, cross-step approval reuse, `sequence_rejects_group_loop` (`:1022`), `dry_run_performs_the_same_preflight` (`:1130`) |
| `claudine/cli/tests/l1/sequence_groups.rs` (21, `#[cfg(unix)]`) | `l1` | concurrency, `max_parallel`, `a_group_loop_is_rejected_before_anything_runs` (`:355`), looping proxy target in a leg (`:1088`) |
| `claudine/cli/tests/l1/sequence_jit.rs` (13, unix) | `l1` | just-in-time re-read; `dry_run_composes_every_step_and_launches_nothing` (`:475`), `..._against_the_initial_state` (`:510`) |
| `claudine/cli/tests/l1/sequence_shell_templates.rs` (51, `test-fixtures`) | `l1` | shell approval by task position, late values (`outputs`, `loop`, `group`), loop gates (`:581`, `:911`) |
| `claudine/cli/tests/l1/sequence_sources_cli.rs` (23, ungated, all `--dry-run`) | `l1` | source format matrix (section 5) |
| `claudine/cli/tests/l1/sequence_errors_cli.rs` (38) | `l1` | pre-launch errors, portable |
| `claudine/cli/tests/l1/composition_outputs.rs` (14) | `l1` | `outputs`/`set` runtime layer across compose, inline-compose, sequence; `:484` looping document reading `seq.outputs` |
| `claudine/cli/tests/l1/research_sequence_artifacts.rs` | `l1` | shipped research fleet through `claudine sequence` with `launch_recorder` |
| `claudine/lib/src/composition/sequence/tests.rs` (117; `group_loop_rejected` `:1307`), `preflight/tests.rs` (80; `group_loop_is_rejected` `:647`), `task/tests.rs` (164), `task/container_regressions.rs` (12), `task/shell/*tests.rs` (22) | in-crate | normalization, preflight graph, container runtime |
| `claudine/cli/src/commands/wrap/sequence/{tests,jit/tests,task_run/tests,review/tests}.rs` | in-crate (cli) | `review/tests.rs:146` uses the frozen `review-loop.md` copy |
| `claudine/cli/tests/level2/level2_sequence_{container_ctrl_c,review_screen,task_stream_capture}.rs`, `level3/level3_*sequence_ctrl_c.rs` | L2/L3 | Ctrl-C and review screen |

### 1.4 Dry run and shell approval

| Location | Covers |
|---|---|
| `claudine/cli/src/commands/wrap/composition/dry_run/tests.rs` (37) | dry-run rendering |
| `claudine/cli/tests/l1/level1_dry_run_pty.rs` (2) | interactive approval prompt under `--dry-run` (PTY, L1) |
| `claudine/cli/tests/level2/level2_dry_run_{approval,metadata}_capture.rs` | L2 rendering |
| `claudine/cli/tests/l1/shell_approval_prompt.rs`, `shell_missing_values.rs`, `wrap_compose_preflight.rs:18` | no-terminal refusal, missing-value rule, compose dry-run shell gate |

No test asserts that `--dry-run` leaves a Git index unchanged, and no
`--dry-run-shell` flag exists in the tree.

### 1.5 Tests that pin behavior this feature changes

These must be rewritten or deleted alongside the change. They are not
regressions.

- **Loop `fail_fast`.** `loop_cli.rs:268,387,444`, `looping/config/tests.rs:223`,
  `engine/tests/iteration_actions.rs:241,282,309`,
  `engine/tests/lifecycle_control.rs:948`; engine default
  `looping/engine.rs:96`.
- **Predictive `is_last`.** `engine/tests/iteration_actions.rs:179-202`.
- **Group `loop:` rejected.** `sequence_cli.rs:1022`, `sequence_groups.rs:355`,
  `sequence/tests.rs:1307`, `sequence/preflight/tests.rs:647`.
- **Dry run.** Any test whose dry run currently executes a `shell:` task must
  be checked: `sequence_containers.rs:112`, `sequence_jit.rs:475,510`,
  `sequence_cli.rs:1130`, `sequence_sources_cli.rs` (all cases).
- **Review loop.** `cli/tests/prompts/shipped_prompts.rs:531`
  (`shipped_review_loop_stops_after_its_first_failed_prompt`) and
  `cli/src/commands/wrap/sequence/review/tests.rs:146`.

## 2. Consolidated test registration

| Package | `autotests` | Declared `[[test]]` binaries |
|---|---|---|
| `claudine` (`claudine/lib/Cargo.toml`) | `false` (`:5`) | `l1` → `tests/l1/main.rs` (`:14-16`). No other binary. |
| `claudine-cli` (`claudine/cli/Cargo.toml`) | `false` (`:9`) | `l1` → `tests/l1/main.rs` (no required features, `:187`); `level2` (`terminal-tests`, `:191`); `level3` (`terminal-tests`, `:196`); `real` (`real-tests`, `:201`); `prompts` (`prompt-tests`, `:206`) |

**Features.** `claudine`: `test-fixtures`, `prompt-tests` (`lib/Cargo.toml:22-29`).
`claudine-cli`: `daemon-tests`, `terminal-tests`, `real-tests`, `prompt-tests`,
`test-fixtures` (`cli/Cargo.toml:57-67`). Fixture binaries
`claudine-fake-{goose,pi,codex,completion,relay}` and `claudine-argv-recorder`
require `test-fixtures` (`cli/Cargo.toml:18-55`).

**`[package.metadata.ci.tests]`.**

- `claudine`: `source-inputs` only (`lib/Cargo.toml:148-155`).
- `claudine-cli` (`cli/Cargo.toml:214-232`):
  - `source-inputs` (four Darkmatter files)
  - `tiers = ["L1", "L2"]`
  - `features = ["daemon-tests", "terminal-tests", "test-fixtures"]`
  - `local-features = ["test-fixtures"]`
  - `l2-backends = ["tmux", "wezterm"]`
  - `runner-tools = ["ai-provider-stubs"]`
  - `sidecars = [...]`

  CI never enables `prompt-tests`.

**How to register a new file.** Put it in `tests/l1/`, then add `mod <name>;` to
`tests/l1/main.rs`, with a `#[cfg(...)]` gate when it needs one. The module
name becomes the first segment of every test path. Existing gates are
`#[cfg(unix)]` for `/bin/sh` stubs and `#[cfg(feature = "test-fixtures")]`
for fixture binaries. Shared helpers come in through
`#[path = "../common/mod.rs"] mod common;`. An L2 file is named `level2_*`
under `tests/level2/` and declared in `tests/level2/main.rs`.

**Layout gates.** These fail when a source file is reachable from no declared
target:

- `claudine/cli/tests/l1/test_placement.rs:504`
  `every_test_source_is_compiled_by_a_declared_target`, with
  `layout_gate_rejects_stray_roots_and_undeclared_modules` (`:531`) and
  `layout_gate_requires_explicit_targets` (`:597`). The same file also gates
  inline-test line budgets (`INLINE_TEST_LINE_LIMIT = 300`, production 800)
  and confines focus-stealing APIs to `level3_*` files.
- `claudine/lib/tests/l1/test_layout.rs:11`. It has the same purpose for the
  library and asserts at least 17 scanned files.
- Both use `test_toolkit::test_layout::{collect_test_sources, layout_violations}`
  (`tools/test-toolkit/src/test_layout.rs:31,67`).

**Recipes.**

- `claudine/justfile:120` `just test`: locally runs
  `claudine-cli --features test-fixtures`; in CI it adds
  `daemon-tests,terminal-tests`.
- `:156` `just test-l2`.
- `:71` `just test-prompts`, which runs
  `_test claudine-cli --features prompt-tests,test-fixtures --test prompts`.

## 3. Prompt tests, frozen fixtures, and route drift

### 3.1 Where the four named tests live

| Test | File | Binary / feature | Reads |
|---|---|---|---|
| `shipped_prompt_contract` | `claudine/cli/tests/prompts/shipped_prompt_contract.rs` (959 lines) | `prompts`, `prompt-tests` | live `prompts/` (`repository_root()` `:12`); copies the corpus into fixtures (`:40`, `:294`) |
| `shipped_prompt_route_drift` | `claudine/cli/tests/prompts/shipped_prompt_route_drift.rs` | `prompts`, `prompt-tests` | live `prompts/implement.md` and `prompts/_implement/implement-plan.md`, plus fixture `claudine/cli/tests/fixtures/shipped_implement_route/` |
| `compose_caller_file_provenance` | `claudine/cli/tests/l1/compose_caller_file_provenance.rs` (1087 lines; ungated module, some tests `#[cfg(unix)]`/`test-fixtures`) | **`l1`, ordinary L1** | frozen `tests/fixtures/frozen_prompts/prompts/review.md` (`:968,1006,1043,1070`); writes a stand-in `feature-review.md` |
| `compose_initialize_acceptance` | `claudine/cli/tests/l1/compose_initialize_acceptance.rs` (859 lines, `#[cfg(unix)]`) | **`l1`, ordinary L1** | `stage_frozen_prompts(["implement.md", "_implement/implement-plan.md"])` and `frozen_prompts/shipped_implement_route/_implement/implement-plan.md` (`:581-592`) |

Spec AC13 groups the four together, but two of them are ordinary L1 tests over
**frozen** copies. Migrating the live prompts does not affect them unless the
frozen copies are re-derived. Only the first two run under `prompt-tests`.

Other `prompts`-binary modules (`cli/tests/prompts/main.rs`):
`pr_flow_rehearsal` (`test-fixtures`), `prompt_guide_defects`,
`shipped_implement_router`, `shipped_prompts`. There is also an L1
`cli/tests/l1/shipped_prompts.rs`, which checks only Claudine's own artifacts.
Library prompt tests: `claudine/lib/src/composition/schema/tests.rs:6-201`
(`#[cfg(feature = "prompt-tests")]`, the `shipped_implement_plan_*` tests).
L2: `level2_lifecycle_control.rs:88,5318`, gated on `prompt-tests`.

### 3.2 Recipes

- `claudine/justfile:71` `just test-prompts` runs only the claudine-cli
  `prompts` binary. Its failures fail the recipe.
- Root `justfile:82` `just test-prompts` runs:
  - `md schema validate` over every tracked prompt
  - `cargo nextest run -p claudine -F prompt-tests -E 'test(/shipped_implement_plan_/)'`
  - the claudine-cli `prompts` binary
  - the claudine-cli L2 `shipped_implement` rows
  - the darkmatter `prompts` binary

  It is **informational: it exits 0 even when steps fail**
  (`justfile:76-80,107-111`), so a verification step must read its output.

### 3.3 Frozen prompts directories

- `claudine/cli/tests/fixtures/frozen_prompts/prompts/`. Its prompts include
  `review-loop.md`, `implement.md`, `review.md`, `plan.md`, `clarify.md`,
  `_implement/implement-plan.md`, and several `_*.md` fragments. Accessors are
  `common::prompt_staging::{frozen_prompts_dir, stage_frozen_prompts}`
  (`cli/tests/common/prompt_staging.rs:30,37`). The live tree is staged only
  through `stage_shipped_prompts`, from the `prompts` binary.
- `claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md`
  is an **older, separate** side-effect-free copy used by
  `compose_initialize_acceptance`. It differs from
  `cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`.
- `claudine/lib/tests/fixtures/frozen_prompts/prompts/` holds
  `_set_spec_schema.md`, `clarify.md`, `review.md`, and
  `_reviews/cross-platform.md`.
- `claudine/cli/tests/fixtures/nested_span_regression/commit.md`, read by
  `wrap_compose_validation.rs:597`, is a frozen `commit.md` that still ends
  `initialize` with `stop` (`:31`).

Frozen `review-loop.md` and `implement.md` are byte-identical to the live
files. Frozen `_implement/implement-plan.md` is not.

### 3.4 How `shipped_prompt_route_drift` derives its fixture and hash

- **Scope.** It covers only `prompts/implement.md` (router) and
  `prompts/_implement/implement-plan.md` (`:59-60`). `review-loop.md`,
  `feature-review.md`, `implement-suggestions.md`, and `commit.md` are not
  in this route.
- **Fixture.** `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
  is the shipped `implement-plan.md` minus exactly its `say:`, `effect:`, and
  `shell:` side effects (`:6-26`).
- **Hash.** Darkmatter's `Markdown::compute_hash(MdHashKind::Simple, &MdHashOptions::default())`,
  flattened to `"{frontmatter}-{body}"` (`:93-99`). This is the `md hash`
  default. Darkmatter's hash module uses `biscuit_hash::xx_hash`
  (`darkmatter/lib/src/markdown/hash/mod.rs:41`, `compute.rs:5`).
- **Pinned in** `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json`.
  `md hash` today reproduces both pins exactly
  (`d0c80616…-da1fe238…`, `67c88bc6…-34708b76…`), so the route is in sync.
- **Guards.**
  - Hash pin (`:112`).
  - `LOAD_BEARING_PLAN_KEYS` structural equality (`:65-74`, `:164`). The
    keys are `$schema`, `plan`, `phase`, `total_phases`, `spec`, `area`,
    `pass_icon`, `loop`.
  - Body equality (`:188`).
  - The router has no `say`/`speak`/`effect`/`shell` (`:203`).
- **Refresh.**
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli -F prompt-tests --test prompts shipped_prompt_route_drift::`
  (`:50-53`).
- **Consequence for this feature.** Removing `in_loop` from `implement-plan.md`
  changes its `$schema` (a load-bearing key) and `initialize`. Re-derive the
  fixture: copy the new `$schema`, and edit the fixture's `initialize`
  identically. Then refresh the pin. Also decide whether the separate
  `frozen_prompts/shipped_implement_route` copy and
  `frozen_prompts/prompts/_implement/implement-plan.md` are re-frozen. Both
  still carry `in_loop` at `:14` and `:40`.

### 3.5 The five shipped prompts and every `in_loop` caller

All five live in the **repository-root `prompts/`**, not `claudine/prompts/`
(which holds only `create-new-provider.md`).

| Prompt | `in_loop` |
|---|---|
| `prompts/review-loop.md` | passes `in_loop: true` at `:27,33,39,51,57,69,75,87,93,105,111,123` (every implement/review/repair step) |
| `prompts/_reviews/feature-review.md` | declares it at `:8`; gate at `:55` (`in_loop && previous_review && frontmatter(previous_review,'ready') == true`) |
| `prompts/_implement/implement-suggestions.md` | declares it at `:9`; gate at `:28` (`in_loop && ready == true`) |
| `prompts/_implement/implement-plan.md` | declares it at `:14`; gate at `:40` (`in_loop && spec && frontmatter(spec,'implemented') == true`) |
| `prompts/commit.md` | none |

Repository-wide, the only shipped caller passing `in_loop` is
`prompts/review-loop.md`. Frozen fixtures that carry it are:

- `claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md`: same
  12 lines
- `frozen_prompts/prompts/_implement/implement-plan.md:14,40`
- `frozen_prompts/shipped_implement_route/_implement/implement-plan.md:14,40`
- `fixtures/shipped_implement_route/_implement/implement-plan.md:14,40`

Other callers of the three helpers pass no `in_loop`:

- `prompts/implement.md`, the router via `proxy`
- frozen `implement.md:33,49`
- frozen `review.md:29,35`

**Surprises.**

- `prompts/commit.md:29` **already uses `skip`**. Spec section 9 / AC12's
  `stop`→`skip` edit has already landed in the live prompt; only AC12's test
  is missing. The frozen `nested_span_regression/commit.md:31` still has
  `stop`.
- The live `review-loop.md` takes `review_agent` and `repair_agent`
  (`:4-5`). The spec's target shape takes one `agent`. Section 11 says "the
  explicit `spec`, `agent`, and commit-message inputs are kept". This needs
  an author ruling on whether the two-agent split is preserved.

## 4. Test helpers a new loop test uses

All of these are in `claudine/cli/tests/common/`, unless a path says
otherwise.

**`CliProcessFixture`** (`mod.rs:257-469`):

- `CliProcessFixture::named(prefix)` creates a private workspace with
  `cwd()`, `home()`, and `bin_dir()`. It panics if the workspace is inside
  the checkout.
- `initialize_repository()` runs `git init` in `cwd` through `helper_command`
  (`mod.rs:947`).
- `seed_user_config()` writes a minimal user config.
- `audio_spool()` is the private spool path. Assert
  `!fixture.audio_spool().exists()`.
- `command()` returns an `assert_cmd::Command`. It sets child-only
  `PLAYA_DRY_RUN=1` and `PLAYA_SPOOL_DIR=audio_spool()` (`mod.rs:624-625`), a
  minimal `PATH` led by `bin_dir`, an unreachable Rendezvous endpoint, and
  `NO_COLOR`.
- `command_std()`; `command_builder()` with `.host_path()` (needs a
  call-site comment, for example when Git for Windows is required),
  `.fake_only_path()`, `.ambient_context(dir)`, and `.inherit_no_env()`.

**Writers.** `write`, `write_json`, `write_executable`, `sh_quote`, and
`strip_ansi`.

**Inline stub.** `InlineAgentStub::new(doc).body(..).exit_code(..).install(bin_dir, "claude")`
(`mod.rs:1211-1300`), plus `write_dry_run_provider_stub` (`mod.rs:1305`).

**`launch_recorder`** (`launch_recorder.rs`; **`#[cfg(unix)]` only**,
`mod.rs:155-156`):

- `install(&fixture, "goose" | "claude", behavior_sh) -> PathBuf` records
  every launch under `home()/launches/`: `launch-NNN` (argv, `\x1f`
  separated), `params-N` (`AGENT_PARAMS`), and `prompt-NNN` (stdin).
- `launches(&dir) -> Vec<Vec<String>>`, `prompts(&dir) -> Vec<String>`, and
  `occurrences(args, run)`.
- Claude receives the prompt on stdin, so use `prompts(&dir)`. Goose receives
  it as `-t <prompt>` in argv, so `sequence_jit.rs:40-47` extracts it from
  `launches`.

**Portable (Windows) stubs.**

- There is no portable prompt recorder.
- Windows tests copy `claudine-fake-relay` to `goose.exe` with a
  `goose.relay.json` (`sequence_containers.rs:6-14`), or write a `claude.cmd`
  (`ctx_per_run.rs:22-40`).
- `claudine-argv-recorder` plus `test_toolkit::argv_recorder::read_argv_records`
  records shell argv, not prompts.

**Scratch Git.** test-toolkit has **no** Git helper. Use:

- `common::helper_command("git")`, which strips `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/…
  (`host_tools.rs:25-44`).
- The pattern in `ctx_per_run.rs:45-83`: a `git()` helper that runs
  `-c user.name=Fixture -c user.email=fixture@example.com -c commit.gpgsign=false`,
  an initial commit, an untracked `a.txt`, and a
  `.darkmatter-shell-whitelist` with `exact git add a.txt`. Read the index
  with `git diff --cached --name-only` or `git ls-files --stage` through the
  same helper.

**`LockedAudioSpool`.**
`test_toolkit::LockedAudioSpool::new(root)` / `.clear_pending()`
(`tools/test-toolkit/src/spool.rs:65-185`). Only publication tests need it
(`cli/tests/l1/detached_audio.rs`). Loop tests rely on `PLAYA_DRY_RUN=1` and
assert that the spool is absent.

**Other.** `prompt_staging::stage_frozen_prompts`, and
`biscuit_test_harness::{manifest_dir!, bin_exe!}`. Never use `env!("CARGO_…")`.

**Minimal existing example: `claudine sequence` with a recording stub,
asserting recorded prompts.** From
`claudine/cli/tests/l1/research_sequence_artifacts.rs:198-232`, which is
`#[cfg(unix)]`:

```rust
let fixture = CliProcessFixture::named("shipped-research-fleet");
fixture.initialize_repository();
// ... write the sequence source and its prompt into fixture.cwd() ...
let recordings = crate::common::launch_recorder::install(&fixture, "claude", "printf 'research complete\n'");
fixture.command().args(["sequence", "--claude", "--yolo", "--silent"]).arg(&source).assert().success();
let prompts = crate::common::launch_recorder::prompts(&recordings);
assert_eq!(prompts.len(), 1);
assert!(!fixture.audio_spool().exists());
```

The AC1 counting reproduction should take the same shape. Write `count.md` and
`seq.md`, install the recorder for `claude`, then assert
`prompts(&rec) == ["Iteration 1", "Iteration 2", "Iteration 3"]` (trimmed)
under `compose`, `sequence`, and a one-leg group.

## 5. Executable configuration formats

### 5.1 Loaders

- **Whole-document data loader.** `claudine/lib/src/composition/sequence/data.rs:87-143`
  (`load_document`) dispatches as follows:
  - `.md`/`.markdown` → Darkmatter `Markdown::try_from`; the frontmatter is
    the value (`:88-99`)
  - `.yaml`/`.yml` and **every unknown extension** → `biscuit_file::Yaml`
    (`:41-56`, `:104-119`)
  - `.json` **and** `.json5` → `biscuit_file::Json5`, which uses
    `json_five::from_str` (`:120-137`). A `.json` file therefore also accepts
    JSON5 syntax.
  - `.jsonl`/`.ndjson` → `load_line_delimited`, which runs `serde_json::from_str`
    per line, skips blank lines, and trims each line (`:146-171`)
- **Direct `claudine sequence <file>`.**
  `claudine/cli/src/commands/sequence.rs:79-129` accepts Markdown through
  `resolve_composition_source`, plus `.yaml`/`.yml` through
  `load_yaml_document` (`lib/src/composition/resolve.rs:236-293`). Any other
  extension is `CompositionError::NotMarkdown` (`sequence.rs:121-125`).
- **`claudine compose` / `inline-compose`.** These accept Markdown only
  (`resolve.rs:200-209`).
- **Task `prompt:` targets** resolve through the Markdown composition path.

### 5.2 Which entry point accepts which format

| Carrier of task / group / loop fields | YAML frontmatter (`.md`) | YAML file | JSON | JSON5 | JSONL / NDJSON |
|---|---|---|---|---|---|
| Document `loop:` (the document primitive) | **yes** (compose, inline-compose, task `prompt:`) | no | no | no | no |
| Root sequence document (`sequence:` + root events; planned root `loop:`) run directly | yes | **yes** (`claudine sequence steps.yaml`) | **unsupported** (`NotMarkdown`) | **unsupported** | **unsupported** |
| Formal sequence document reached through `sequence: "<ref>"` with no offset or operator; carries its events and authored properties (`preflight/mod.rs:925-929`) | yes | yes | yes | yes | no (root is a list; no root keys) |
| Task list via `sequence: "<ref> -> offset"` or `"<ref>::op(..)"` (lenient "foreign data", `source.rs:1-11`, `normalize.rs:11-13`) | yes | yes | yes | yes | yes; operator only, an **offset is a typed error** (`source.rs:84-91`) |
| Task entries carrying `prompt`/`shell`/`group`/`sequence`/`when`/inline `loop` (executables are extracted in both strict and lenient modes, `normalize.rs:184-232`) | yes | yes | yes | yes | yes (one record per line; nested objects must fit on that line) |
| `kind: task` / `kind: group` / `kind: group-catalog` files (`preflight/mod.rs:460-482,2090-2235`) | yes | yes | yes | yes | **unsupported** (root is a list; `expect_kind` fails; `data.rs:65-72`) |
| Expression source `sequence: "{{ expr }}"` / shell source `sequence: "$( … )"` (`sequence/mod.rs:200-245`) | root sequence only | — | — | — | — |

Expression and shell sources yield **names only** from text lists (CSV, TSV,
lines, spaces). An expression may also yield an array of objects
(`mod.rs:210-215`). These are not a configuration *format*, so they are out of
the format matrix.

Unsupported as configuration formats: TOML, Markdown bodies, and JSON-family
files for direct `claudine sequence` / `compose`. Record them as
**unsupported**; do not add them.

**Existing format tests.**

- `sequence_sources_cli.rs:95`
  `yaml_json_json5_jsonl_and_ndjson_sources_produce_the_same_plan`: names
  only.
- `:621` `external_task_group_and_catalog_references_all_load_and_run`: YAML
  only.
- `:511`, `:527`: direct YAML.
- `:463` `a_formal_document_normalizes_identically_through_both_entry_paths`.

No test carries an executable task, a group object, a `loop:`, or a `when:`
through JSON, JSON5, or JSONL.

### 5.3 Duplicate keys and trailing content

| Format | Duplicate key | Trailing / invalid content |
|---|---|---|
| Markdown frontmatter | **Rejected.** Darkmatter parses with `serde_yaml_ng::Value`, whose `Mapping` visitor errors on an occupied entry (`~/.cargo/registry/.../serde_yaml_ng-0.10.0/src/mapping.rs:813-818`, reached from `value/de.rs:100-106`). Pinned by `claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs:542-545` ("never last-wins") and noted at `claudine/lib/src/composition/closure/persist.rs:604-605`. | The body after the closing fence is Markdown and is allowed. **An opening fence that never closes is body text and reads as *empty* frontmatter** (`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`, `try_frontmatter_fn` doc), so it is not an error. |
| Standalone YAML (`biscuit_file::Yaml::new` → `serde_yaml_ng::from_str::<Value>`, `biscuit-file/lib/src/yaml/types.rs:251-254`) | **Rejected**, by the same `mapping.rs:813-818` path | A second YAML document is rejected: "deserializing from YAML containing more than one document is not supported" (`serde_yaml_ng-0.10.0/src/error.rs:221`). |
| JSON / JSON5 (`json_five::from_str::<serde_json::Value>`, `biscuit-file/lib/src/json5/types.rs:99-103`) | **Silently last-wins.** `serde_json::Value`'s `visit_map` uses `Map::insert` (`serde_json-1.0.154/src/value/de.rs:136-142`). `MappingOrders` (`darkmatter/lib/src/markdown/mapping_orders.rs:144-150`) pushes a duplicated key twice into its order list. | Rejected: "Unexpected … token after value" (`json-five-0.3.1/src/parser.rs:545-551`). JSON5 comments are allowed. |
| JSONL / NDJSON (`serde_json::from_str` per line, `data.rs:159`) | **Silently last-wins** within a record (same `serde_json` path) | Trailing characters on a line are rejected with a line number (`SequenceLoadCause::JsonLine`, `data.rs:159-166`). Blank lines are skipped. Each further valid record is a new item. |

Plan rule 9 says to reject duplicate executable-configuration keys before
lossy map conversion. Today that holds for YAML and frontmatter only. JSON,
JSON5, JSONL, and NDJSON need a presence-aware or duplicate-detecting read.
The `MappingOrders` raw pass in `data.rs:127` is the existing second traversal
that could detect them for JSON and JSON5.

## 6. Acceptance criteria mapping

Unless noted, a proposed file is a new `claudine/cli/tests/l1/<name>.rs`
registered by `mod <name>;` in `cli/tests/l1/main.rs` (binary `l1`, L1). Files
that record prompts with `launch_recorder` are `#[cfg(unix)]`. A
Windows-portable variant needs a relay-based recorder (section 4).

| AC | Planned public assertion (observable result) | Proposed test location | Owning plan task |
|---|---|---|---|
| 1 Defect | Recorded prompts equal `Iteration 1, 2, 3` for `claudine compose @count.md`, for `claudine sequence @seq.md` (one step), and for a one-leg group; exit 0 each | `l1/loop_primitives.rs` (new) | Document adapter |
| 2 Uniform loops | A two-leg looping group and a three-step looping sequence each run 3 passes. The parent receives the published output (group: final pass's dictionary; sequence: flat `seq.outputs` of 6 or 9 entries). Per-pass `loop.count`/`is_first`/`is_last`/`last_output` are recorded via `append_line`/`stdout`. Event trace: 1× `initialize`, 3× `start`/terminal/`finalize` | `l1/loop_primitives.rs`; engine conformance in-crate `looping/engine/tests/` | Loop state machine; Container adapters |
| 3 Outcomes | For each primitive: condition met → exit 0; cap → typed exhaustion, exit 1, action count == passes − 1 (the last pass applies none); condition met on the final permitted pass → exit 0; broken → summary `broken` | `l1/loop_outcomes.rs` (new) | Loop state machine; Retry publication (outcome summaries) |
| 4 Loop keys | `fail_fast` under `loop:`, invalid caps (zero, negative, fractional, unresolved, overflow, templated), and unknown keys are located typed errors with no launch; `--max-iterations` / `CLAUDINE_MAX_ITERATIONS` apply per loop independently in a nested fixture | `l1/loop_keys.rs` (new) plus `looping/config/tests.rs`; rewrite `loop_cli.rs:268,387,444` | Configuration grammar |
| 5 `break` | Exit status and recorded launches for each listed case. The concurrent-group cases (first breaking leg in declaration order; queued `max_parallel` legs still run; sibling failure wins; `fail_fast: false` handled failure does not defeat it) use barrier files, not sleeps. Ctrl-C → exit 130 (portable signal helper `common/signal.rs`; Windows variant like `sequence_ctrl_c_windows.rs`) | `l1/loop_break.rs` (new); Ctrl-C rows may need `level2_*` if a PTY is required | Break propagation |
| 6 `prep` | Recorded event order shows control returning to the next item. Caller `seq.outputs`, frontmatter on disk, and runtime layer are unchanged. Child error cause appears in stderr. Mixed `prep`/`proxy` cycles are refused before launch. A helper called twice runs twice. A statically known target's shell appears in the front-loaded approval, with no prompt mid-run. `break`/`prep` under `cancelled` are parse errors (extend `sequence_deep_cancelled.rs`) | `l1/lifecycle_prep.rs` (new); parser rows in `lifecycle/tests/grammar.rs` (`:894` flips) | Prep target runner; Returning call wiring; Configuration grammar (parse) |
| 7 `when:` | A false guard records `skipped`, makes no launch, fires no event, and appends nothing; it is re-evaluated each outer pass; an expression error is located and fails the task; a false guard over an invalid task still fails preflight; sequence-root `when` is a typed error; delayed admission keeps the runtime snapshot while file reads stay live | `l1/task_when.rs` (new) | Task guard binding; Configuration grammar |
| 8 Caller values | `claudine sequence seq.md agent=x` renders `{{ agent }}` in a `shell:` task (argv via `claudine-argv-recorder`) and in authored step data; a missing setter fails preflight with no launch | `l1/sequence_caller_values.rs` (new, `test-fixtures`), or extend `sequence_shell_templates.rs` | Task guard binding |
| 9 Dry run | Scratch-repo `git diff --cached` is empty after `--dry-run` and lists `a.txt` after `--dry-run --dry-run-shell`; `--dry-run-shell` alone is a usage error (exit 2); a file-dependent loop terminates after one preview with no lifecycle or `prep` effects; a `side_effect:` scratch file stays absent and its runtime value stays unset under both flag sets; report shows `would run` | `l1/sequence_dry_run_shell.rs` (new, portable through the `ctx_per_run.rs` Git pattern) | Dry run traversal |
| 10 Path anchors | Table-driven public-expression results per function: return type, and the missing-target result or error | `darkmatter/lib/tests/l1/<new>.rs` (Darkmatter `l1` binary, `autotests = false`), registered in `darkmatter/lib/tests/l1/main.rs` | Path contracts |
| 11 Review loop rehearsal | Scratch repo with stub providers. Unimplemented two-phase spec → both phases, one not-ready review, one repair and commit, then ready, exit 0. Implemented spec → no implement and no commit launches. Recurrence → exit 2, no repair/stage/commit. Invalid, stale, or pre-existing review files fail before repair. Cap → exit 1 | `l1/review_loop_rehearsal.rs` (new) over a **re-frozen** `frozen_prompts/prompts/review-loop.md` and helpers; optional live-tree twin in `prompts/` binary | Workflow rehearsals |
| 12 `commit.md` | With nothing staged, the recorder shows zero launches for the commit step | In `l1/review_loop_rehearsal.rs` over a re-frozen `commit.md`; live check in `prompts/shipped_prompt_contract.rs:588` | Review prompt migration (prompt already has `skip`); Workflow rehearsals (test) |
| 13 Shipped prompts | The four named tests pass in their registered tiers: two in `just test-prompts`, two in `just test` | existing files (section 3.1) | Workflow rehearsals |
| 14 Retry in a looping container | `seq.outputs` at retried-pass start equals pass 1's results; final array has one entry per child per completed pass; a recorded external `append_line` repeats; `initialize` 1×; `loop.count` is unchanged; `finalize` runs once per pass after its final attempt; group and nested variants | `l1/loop_retry_passes.rs` (new) | Retry publication |

Review-file metadata (AC11) needs presence-aware reads.
`frontmatter(file, prop)` returns `null` for **both** an absent key and an
explicit `null`
(`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2430-2446`).
`has_key(frontmatter(file), 'prop')` (`functions/collections.rs:6`) is the
existing presence-aware spelling. With it, an absent `review_iterations` reads
as 0 and an explicit `null` is an error, as the matrix requires.

## 7. Robustness matrix fixture cases

### 7.1 Base positive controls, one per supported format

Each control must demonstrably launch, loop, skip, or select. Apply exactly
one edit per matrix cell to the control, and assert public results: exit code,
recorder launch count, published output, and summary.

| Format | Base control | How an edit is applied |
|---|---|---|
| YAML frontmatter (`.md`) | (a) `count.md` from spec section 3 (document `loop`); (b) `seq.md` with root `loop:` + `sequence:` + one `group:` leg carrying `loop:`/`when:`; (c) lifecycle `break`/`prep` items in that group's `start`/`success` | Rewrite the YAML text. Duplicate key = repeat the key at the same indent (expected Error; already rejected). Trailing content = an unclosed fence, which **today reads as empty frontmatter**, so record it as a cell to decide |
| Standalone YAML (`claudine sequence steps.yaml`, and `kind: group` `.yaml`) | Same graph as (b) at the file root; `kind: group` file with `loop:` + `tasks:` | Text edit. Trailing content = append `---\nx: 1` (a second document; rejected today) |
| JSON (`sequence: "@tasks.json"`, `group: "@group.json"`) | Formal document `{"sequence":[...], ...}` with an executable group leg; a `kind: group` JSON file | Serialize with `serde_json`, then edit one member. Duplicate key = hand-insert a repeated key (**accepted last-wins today**, so the expected Error needs new code). Trailing = `}{` or a stray token (rejected today). Note: `.json` accepts JSON5 syntax |
| JSON5 | As JSON, with unquoted keys and a comment | As JSON. Comments are allowed content and are not a trailing-content cell |
| JSONL / NDJSON (`sequence: "@tasks.jsonl::name(label)"`) | Two records; record 1 is a `group:` task with `loop`/`when`, record 2 a valid `shell:` task | Edit only record 1; record 2 stays valid. Duplicate key within record 1 (accepted last-wins today). Trailing = garbage after record 1's closing brace on the same line (rejected today). `kind:` files and offsets are unsupported, so mark those cells N/A |

Document-only fields (`loop` on a document; review/spec metadata `implemented`,
`review_iterations`, `ready`, `recurrence`) are YAML-frontmatter-only cells.

### 7.2 Code smells on load-bearing loop, task, and lifecycle fields

These come from greps over non-test sources in `claudine/lib/src/composition/{looping,sequence}`
and `lifecycle/{parse,validate,action_shape,signatures,actions,control}`.

- **`#[serde(default)]`:** none. Loop, task, and lifecycle configuration is
  hand-parsed from `serde_json::Value`.
- **`unwrap_or_default()` that matters:**
  - `looping/engine.rs:97-100`: `on_rate_limit` absent → default policy. The
    neighboring `fail_fast` defaults to `true` (`:96`), which is code to
    remove.
  - `lifecycle/action_shape.rs:517,538`: `proxy`/`retry` `with` absent ≡
    `with: {}`. This is intentional and commented. `with: null` is a typed
    error (`parse.rs:1078-1110`). The same rule would carry to `prep.with`.
  - `sequence/preflight/mod.rs:455`: non-object `caller_setters` → empty map
    silently. This is load-bearing for AC8.
  - `sequence/preflight/mod.rs:930`: a referenced sequence's events default
    to empty when an offset or operator is used. This is by design.
  - `sequence/task/mod.rs:704,1319`, `task/group.rs:163`.
  - Diagnostic-only hits: `lifecycle/parse.rs:975`, `looping/config.rs:286`.
- **`.ok()` that matters:**
  - `looping/config.rs:24,31,41`: `CLAUDINE_FAIL_FAST`, `FAIL_FAST`, and
    `CLAUDINE_MAX_ITERATIONS` values that do not parse are **silently
    ignored**. An invalid runtime cap override falls back to the authored or
    default cap, which is relevant to AC4. Note `CLAUDINE_MAX_ITERATIONS=0`
    parses as `0`; check whether it is validated downstream.
  - `looping/config.rs:55` (pause margin; documented as ignored).
  - `sequence/preflight/mod.rs:1869`: a schema that fails to load → "no
    declared types" (commented as reported elsewhere).
  - `looping/expression.rs:226` and `sequence/expr.rs:137`: an invalid
    resolution context → `None` with a warning.
  - `looping/actions.rs:376`.
- **`filter_map(.. as_str())` / `filter_map(as_object)`:**
  - `sequence/preflight/mod.rs:2131`: names for a "did you mean" list only;
    not load-bearing.
  - `sequence/grammar.rs:414`: `group.tasks` legs that are not objects are
    **skipped** by `validate_task_grammar`'s recursive removed-key check.
    Confirm they are rejected elsewhere.
- **Absent and null conflated, or unrecognized keys becoming data:**
  - `frontmatter(file, prop)` (section 6).
  - `when:` on a task is **not** a reserved key today. It falls into
    authored step state (`sequence/reserved.rs` `TASK_OPTION_KEYS`;
    `normalize.rs:201-216`), so a `when: false` task currently runs.
  - `loop:` on a task is rejected only incidentally, as a reserved state key
    (`ROOT_OVERLAY_KEYS`).
- **Cap shape:** `loop.max` accepts only a JSON unsigned integer
  (`looping/dsl.rs:218-234`). A templated `"{{ max }}"`, as review-loop needs,
  is a type error today.
- **Group `loop:`** is rejected today (section 1.5), and
  `resolve_loop_config` reads only document frontmatter
  (`looping/config.rs:67-75`).

## 8. Other observations

- A looping proxy **target** inside a group leg is exercised
  (`sequence_groups.rs:1088`), but that test asserts only stack safety. It
  does not say how many iterations ran. The Document adapter should
  characterize proxy-target loops in sequences next to direct `prompt:` tasks.
- `compose_initialize_acceptance` and most sequence and loop CLI files are
  `#[cfg(unix)]`. The spec requires portability to native Windows. New
  recording tests need a Windows recorder; none exists today.
