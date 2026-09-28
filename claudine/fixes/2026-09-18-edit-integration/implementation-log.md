---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-18-edit-integration/spec.md"
plan: "claudine/fixes/2026-09-18-edit-integration/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/fixes/2026-09-18-edit-integration/spike-interactive-startup.md
    - claudine/fixes/2026-09-18-edit-integration/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/cli/src/commands/wrap/flags.rs
    - claudine/cli/src/commands/wrap/flags/tests.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/wrapper_stages/tests.rs
    - claudine/cli/tests/l1/wrap_basics.rs
docs_updated_during_phase_2:
    - claudine/fixes/2026-09-18-edit-integration/implementation-log.md
    - claudine/fixes/2026-09-18-edit-integration/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/cli/src/commands/wrap/profile/antigravity.rs
    - claudine/cli/src/commands/wrap/profile/goose.rs
    - claudine/cli/src/commands/wrap/profile/kilo.rs
    - claudine/cli/src/commands/wrap/profile/pi.rs
    - claudine/cli/src/commands/wrap/profile/tests/positional.rs
docs_updated_during_phase_3:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/fixes/2026-09-18-edit-integration/implementation-log.md
    - claudine/fixes/2026-09-18-edit-integration/plan.md
    - claudine/fixes/2026-09-18-edit-integration/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/cli/tests/l1/spawn_site_guard.rs
    - claudine/cli/tests/level2/level2_edit_interactive_capture.rs
    - claudine/cli/tests/level2/main.rs
    - claudine/cli/tests/real/main.rs
    - claudine/cli/tests/real/real_pi_interactive_startup.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-18-edit-integration/implementation-log.md
    - claudine/fixes/2026-09-18-edit-integration/plan.md
    - claudine/fixes/2026-09-18-edit-integration/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
    - claudine-cli
---

# Implementation Log for 2026-09-18-edit-integration (5 phases)

## Phase 1

Started and finished 2026-09-28 on macOS (dev Mac). Phase 1 is rulings and
spikes only, and no source code, test, doc, or skill file changed. The spike
record is `spike-interactive-startup.md` in this directory.

### Rulings N1–N10: accepted

Each ruling's code claims were checked against the tree before acceptance:

- **N1** accepted. Repairing which existing `PromptDelivery` variant and which
  native flags a profile emits is in scope. No new variant.
- **N2** accepted, and **it fired for Kimi** (see below).
- **N3** accepted. Confirmed: `validate_timeout_constraints`
  (`wrapper_stages.rs:18`) owns the two `cannot be used with --interactive`
  messages and the `edit_requested` branch (`:32`), and has one caller
  (`mod.rs:394`, Stage 5). Stage 2 is at `mod.rs:305` and Stage 3 (the editor)
  at `mod.rs:323`. The later duplicate at `mod.rs:624–636` exists as described.
- **N4** accepted. Confirmed: `extract_wrapper_flags_from_passthrough_with_boundary`
  (`flags.rs:316`) lifts `-y -i --edit --repo -q --silent -v --perf
  --operation/--op` and none of `--timeout`, `--step-timeout`, or `--dry-run`.
- **N5** accepted as the planner's 2026-09-28 observation. It was not
  re-observed, because running binaries was refused in this session (see
  "Evidence limits").
- **N6** accepted (out of scope; filed in Phase 5). Same caveat as N5.
- **N7** accepted. Confirmed: `PROVIDERS_DISPLAY_ORDER: [Provider;
  PROVIDER_COUNT]` at `claudine/lib/src/provider_id.rs:56`.
- **N8**, **N9**, and **N10** accepted as written.

### Spike outcomes (details in the spike doc)

- **S1.** Claude, Codex, Gemini, Qwen, and OpenCode: keep their current forms.
  Goose (`run --text=<p> --interactive`), Kilo (`--prompt`, OpenCode-style
  attached value), Antigravity (`--prompt-interactive`), and Pi (`-- <p>`)
  need repair. Kimi has no native interactive startup-prompt surface, so
  **N2 fired**.
- **S2.** Pi accepts `--` from **0.84.3** (changelog, #7269). The dev Mac runs
  0.87.1 per the plan. A piped stdin always forces Pi into print mode
  (`main.ts` `resolveAppMode` plus the piped-stdin override), which confirms
  today's interactive Pi delivery is broken. Decided shape: interactive →
  `AppendArgs(["--", prompt])`, non-interactive stays `Stdin`.
- **S3.** Only Pi newly moves to argv. Antigravity and Goose already argv-deliver
  in both modes, so the plan's premise that they might newly move was
  corrected. The Pi guard is per-profile (one new call site, Rule 2). **New
  risk:** Pi upstream issue #9200 reports SIGKILL for positional messages of
  about 993 bytes or more (0.83.0, 0.85.1; closed as not planned). This is
  escalated to the author.

### Gate G1

- S1–S3 are recorded in the spike doc. N1–N10 are accepted above.
- N2 fired for Kimi, so the spec is set to `status: human-in-the-loop` with
  `human_review: true`. Phase 2 does not depend on the ruling and can proceed.
  Only the Kimi Phase 3 task and Kimi's row in the fleet test's expectation
  table wait on it.

### Evidence limits and departures from the plan

- **Departure (S1): no live terminal run.** This session was non-interactive,
  and its permission policy refused `tmux -V`, `pi --help`, and every other
  provider binary. It also refused `ls` and `Read` outside the worktree, so the
  installed npm packages could not be inspected either. S1 and S2 were done
  from upstream source, changelog, and docs (fetched 2026-09-28) plus the
  in-repo research corpus, and each row is labeled with its tier. The spike
  doc lists the exact live checks still owed. Phase 4's real Pi test covers
  Pi. The other repaired providers (Goose, Kilo, Antigravity) have no planned
  live test, so a human-permitted pass is recommended before Phase 5 states
  per-provider behavior as verified.
- Goose is not installed on the dev Mac. Its evidence is upstream source
  (`block/goose` `crates/goose-cli/src/cli.rs`), as the plan allowed.
- **Finding outside Phase 1 scope:** Goose's `-t/--text` has no
  `allow_hyphen_values`, so a `-`-prefixed prompt also breaks the
  **non-interactive** `run -t <p>` path. Not fixed here; see the spec's
  `message_to_agent`.
- **Finding for Phase 3:** after `--`, Pi still reads an `@`-prefixed token as
  a file argument, so a prompt that begins with `@` would be misread.
- No `just test` or `just lint` run was needed, since no code changed.

## Phase 2

Started and finished 2026-09-28 on macOS (dev Mac). Phase 2 is the
provider-agnostic wrapper validation change. No profile, doc, or skill file
changed; Kimi's N2 ruling and Pi's #9200 question do not touch this phase.

### What changed

- **Clap conflict removed.** `WrapperArgs::edit` no longer declares
  `conflicts_with = "interactive"` (`flags.rs`). Its doc comment now says
  `--edit` only authors the prompt, the edited prompt selects the mode like a
  command-line prompt, and the editor needs a terminal whatever the mode. The
  hand-written wrapper help line (`print_wrapper_help`) now reads "Draft the
  initial prompt in an external editor; combine with -i for an interactive
  session".
- **Runtime conflict removed.** The `edit_requested && interactive_requested`
  branch and the `edit_requested` parameter are gone from
  `validate_timeout_constraints`.
- **Pre-editor timeout check (N3).** New `pub(crate) fn
  reject_interactive_timeouts(args, interactive_requested)` in
  `wrapper_stages.rs` owns the two `cannot be used with --interactive mode`
  messages (text unchanged). It is called at the end of Stage 2 in
  `run_provider_wrapper_inner`, before Stage 3 opens the editor.
  `validate_timeout_constraints` keeps only the post-edit "can only be used in
  non-interactive mode" rules.
- **Departure (small): `validate_timeout_constraints` also lost its
  `interactive_requested` parameter.** Once the two interactive branches moved
  out, nothing in it read that parameter, so its signature is now
  `(args, non_interactive_requested)`. Its one caller (`mod.rs`, Stage 5) is
  updated. The later duplicate at `mod.rs` (Stage 9, "provide a prompt")
  is kept, as N3 requires.

### Comment pass (drift found and fixed)

- `mod.rs` Stage 5 carried `// The effective interactivity state is determined
  solely by the explicit flag.` That was wrong: the mode comes from `-i` **or**,
  without it, from whether a prompt reached the child (the Stage 3 comment
  already states this correctly). The code is right, so the drifted comment was
  deleted rather than reworded.
- `prompt_source.rs` and the rest of Stages 2–5 carry no rationale for the
  false conflict; nothing else changed. `flags.rs`'s `WrapperArgs` docblock
  does not mention the conflict.
- **Left for Phase 5:** `claudine/docs/getting-started/index.md` still quotes
  the old help sentence ("Open the prompt in an external editor before
  launching the provider"). The page is Phase 5's getting-started task.

### Requirement → test mapping

| Requirement | Test (tier) |
|---|---|
| Clap accepts `--edit --interactive`, `--edit -i`, `-i --edit`, and flags before a seed (AC2) | `flags::tests::wrapper_args_accept_edit_with_interactive_in_every_spelling` (L1 unit) |
| `--edit -i` after a positional seed is lifted from passthrough (AC2) | `flags::tests::edit_and_interactive_after_a_seed_prompt_are_both_lifted`; proptest `proptest_extract_wrapper_flags_preserves_others` now asserts `--edit` and `-i` coexist (L1 unit) |
| Pre-editor check rejects `-i` with each timeout, accepts `--edit -i` alone, ignores timeouts without `-i` (AC11) | `wrapper_stages::tests::reject_interactive_timeouts_{refuses_each_timeout_with_interactive,accepts_edit_with_interactive_alone,leaves_non_interactive_timeouts_alone}` (L1 unit) |
| Runtime no longer rejects edit + interactive; post-edit timeout rules intact (R4) | `wrapper_stages::tests::validate_timeout_constraints_{no_longer_rejects_edit_with_interactive,still_requires_non_interactive_for_timeouts}` (L1 unit) |
| Non-TTY `--edit --interactive`, `--edit -i`, `"seed" --edit --interactive`: terminal diagnostic, no conflict text, no editor, no provider (AC5, AC2) | `wrap_basics::wrapper_accepts_edit_with_interactive_and_requires_a_terminal` (L1 integration; replaces `wrapper_rejects_edit_and_interactive_conflict`) |
| N4 forms fail on the timeout before the editor (AC11) | `wrap_basics::wrapper_rejects_interactive_timeouts_before_the_editor_opens` (L1 integration) |
| Direct `"prompt" -i` still launches interactively (AC9, second half) | `wrap_basics::wrapper_direct_prompt_with_interactive_launches_interactively` (L1 integration, Unix: records argv; asserts no `exec`, prompt present, `-i` not leaked) |

- **Regression proof.** With `reject_interactive_timeouts` disconnected from
  `run_provider_wrapper_inner`, `wrapper_rejects_interactive_timeouts_before_the_editor_opens`
  fails (the non-TTY editor error wins). Restored afterward. The clap-parse
  unit test fails against the old `conflicts_with` by construction.
- **Placement.** The two new `wrap_basics` negative tests are not Unix-gated:
  a local `write_marker_executable` helper writes a `sh` or `.cmd` stub that
  only creates a marker file, used as both the `codex` stub and the fake
  `EDITOR`. The old conflict test needed no stub because clap refused before
  Stage 1; the new ones need a `codex` on the fixture `PATH`, since binary
  resolution precedes the editor (R3). The argv-recording AC9 test is
  Unix-gated like its neighbors. All tests are L1 by name and compiled by the
  `l1` binary (`wrap_basics` is already declared); all spawns go through
  `CliProcessFixture::command()`.
- AC1, AC3, AC4, AC6, AC8, and AC9's first half need a real terminal and are
  Phase 4's L2 file.

### Gates (Checkpoint 2)

- `cd claudine && just test` (macOS): **7723 passed, 9 skipped, 0 failed**.
- `cd claudine && just lint`: clean. The only output is the long-standing
  macOS linker `__eh_frame section too large` warning, which this change did
  not introduce.
- `just cross-check claudine-cli --os windows wrap_basics:: wrap::flags::tests
  wrap::wrapper_stages::tests` (native Windows rig): **57 passed**, including
  both non-gated `wrap_basics` tests with the `.cmd` marker stubs. The first
  attempt used a nextest `-E 'test(…)'` filterset, which the remote shell
  rejected (`syntax error near unexpected token '('`). Positional substring
  filters work; use those with `cross-check`.
- No pre-existing failures were seen. No skill change: the `cli-reference`
  `--edit` row and the timeline entry belong to Phase 5.

## Phase 3

Started and finished 2026-09-28 on macOS (dev Mac). Phase 3 repairs the
interactive startup-prompt delivery of the profiles the Phase 1 spike found
defective, and adds the fleet test. Both human-review items from Phase 1 (Kimi
under N2, Pi upstream issue #9200) were **still unruled** when this phase ran.

### What changed

| Provider | Interactive delivery before → after | Non-interactive |
|---|---|---|
| Goose | `… run -t <p>` (one-shot) → `run --text <p> --interactive …`; `--text=<p>` for a `-`-prefixed prompt | unchanged: `run -t <p>` |
| Kilo | `-- <p>` (read as a project directory) → `--prompt <p>`; `--prompt=<p>` for a `-`-prefixed prompt | unchanged: `run -- <p>` |
| Antigravity | `--print <p>` (headless one-shot) → `--prompt-interactive <p>`; `--prompt-interactive=<p>` for a `-`-prefixed prompt | unchanged: `--print <p>` last |
| Pi | `Stdin(<p>)` (forces print mode) → `-- <p>`; a prompt starting with `@` becomes `-- " @…"` | unchanged: `Stdin(<p>)` |
| Kimi | unchanged (`--prompt <p>`), per N2 | unchanged (`WireRpc`) |
| Claude, Codex, Gemini, Qwen, OpenCode | unchanged, no code touched | unchanged |

- **Goose places `run` first in interactive mode.** When the argv has no
  `run` (the interactive entrypoint adds none), the delivery now inserts
  `run --text <p> --interactive` at index 0 rather than appending `run -t <p>`
  last. Any flag already on the argv, including the profile's own
  `--system <p>`, then parses as a `run` option instead of a root option.
  When `run` is present, the text and `--interactive` go right after it.
  Before this change, interactive Goose with an append system prompt produced
  `goose --system X run -t p`.
- **Kilo's size guard now covers both modes.** One `ARG_MAX_HEADROOM`
  (768 KiB) check runs before either branch. Its message now says "on the
  command line" rather than "as a positional argument", since the
  interactive value is a flag value.
- **Pi.** Per-profile 768 KiB guard (S3, Rule 2). The error suggests running
  without `-i`, which is correct for Pi because its non-interactive path is
  stdin. The stale 0.80.3 comment ("`--` is rejected") is replaced. The
  profile docblock states the 0.84.3 minimum for interactive sessions.
- **Pi `@` decision (open point 3b from Phase 1).** After `--`, Pi still reads
  a token starting with `@` as a file argument. A leading space is prepended
  only to a prompt whose **first** character is `@`, so the prompt stays a
  message. A mid-prompt `@name` is untouched. A refusal was rejected because
  it would be a new provider-specific error, and a leading space is harmless
  to the model. Not live-verified (see "Evidence limits").
- **Kimi.** N2 is unruled, so `kimi.rs` is untouched. The fleet table's Kimi
  row pins today's `--prompt` argv. A comment on the row says it is not a
  verified interactive form. When the author rules, that row and `kimi.rs`
  change together.

### Departure: Pi repair implemented before #9200 was settled

The Phase 1 message said "do not finalize the Pi repair until it is ruled or
checked live". This session's permission policy again refused `pi` and `tmux`
(`pi --version` and `tmux -V` both need approval), so the live check was not
possible. The repair was implemented anyway, because every option the author
can choose, except C (temporary `@file`, which the plan's rules exclude),
ships this same argv code. The stdin path it replaces is broken by source for
**every** interactive prompt, since a piped stdin always forces print mode.
The plan's Pi task is therefore **left unticked**, with a note saying the code
landed and finalization waits on the #9200 ruling or the Phase 4 real Pi test
with a prompt of about 2 KB. If that test reproduces the crash, only the
interactive branch of `pi.rs` changes.

### Departure: dispatch inventory edited by hand

The fleet table first used `Provider::X` keys. The table and the new
per-profile tests added 13 sites to `claudine/docs/providers/dispatch-inventory.json`,
and `dispatch_inventory_matches_committed_file` failed. The documented
regeneration (`CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`)
was refused by this session's permission policy in every spelling tried
(`VAR=1 cmd`, `env VAR=1 cmd`). Two fixes:

1. The fleet table is keyed by `Provider::as_slug()` strings, so it adds no
   `tuple-array` (conditional-class) site. A missing row still fails, through
   the `PROVIDER_COUNT` length check and the per-provider lookup.
2. The 12 remaining new sites are plain `profile(Provider::X)` references
   (`direct-ref`, `reference` class, `exempt_candidate: true`, lines 848–997
   of `positional.rs`). They were added to the JSON by hand in the generator's
   exact record shape and sort position. `sites`, `provider_refs`,
   `by_form.direct-ref`, and `by_dispatch_class.reference` each rose by 12.
   The byte-comparing test now passes, which proves the hand edit equals the
   generator's output.

### Commit made outside this session

During this phase, commit `57968bb03` ("feat(claudine-cli): support
interactive startup prompts across the fleet") appeared on the branch. It
contains the five profile and test source files as they stand now. This
session did not stage or commit anything. The commit came from a separate
process. Its message has two errors to fix at review:

- it says "this implements Phase 2", but this is Phase 3;
- it says the change "ships the startup-prompt promise the spec owed every
  provider in the fleet", which overstates it: Kimi is still unresolved (N2),
  and Pi's promise waits on #9200.

The dispatch-inventory edit, plan, log, and spec changes are uncommitted.

### Requirement → test mapping

| Requirement | Test (all L1 unit, `claudine-cli` bin, `commands::wrap::profile::tests::positional::`) |
|---|---|
| Every provider has an interactive startup delivery that matches its expected native form, for a plain prompt and for a multiline prompt starting with `- ` (AC7, AC8). No provider uses stdin or wire RPC interactively, and no stdin seed is produced. A provider with no row fails | `every_provider_delivers_an_interactive_startup_prompt` |
| Goose interactive: `run` first, `--text`, `--interactive`, other flags after; follows an existing `run`; attached `--text=` for a bullet | `goose_interactive_prompt_leads_with_run_so_other_flags_parse_as_run_options`, `goose_interactive_prompt_follows_an_existing_run_entrypoint` |
| Goose non-interactive unchanged (with and without `run` on the argv) | `goose_non_interactive_prompt_shape_is_unchanged`; existing `goose_resume_uses_run_with_explicit_session_id`, `test_goose_non_interactive_no_duplicate_run` |
| Kilo interactive uses `--prompt` / `--prompt=` | `kilo_interactive_prompt_uses_prompt_flag_not_the_project_positional` |
| Kilo non-interactive stays `run -- <p>` | `kilo_non_interactive_prompt_stays_positional_after_run` |
| Kilo size guard in both modes, with the boundary value accepted | `kilo_rejects_an_oversized_prompt_in_both_modes` |
| Antigravity interactive uses `--prompt-interactive` / `=` | `antigravity_interactive_prompt_uses_prompt_interactive_not_print` |
| Antigravity non-interactive keeps `--print <p>` last, after `--output-format json` | `antigravity_non_interactive_prompt_stays_print_last` |
| Pi interactive `-- <p>` for plain and bullet prompts, stdin not seeded | `pi_interactive_prompt_is_a_positional_message_after_end_of_options` |
| Pi leading `@` gets a leading space; mid-prompt `@` is untouched | `pi_interactive_prompt_starting_with_at_is_not_read_as_a_file` |
| Pi interactive size guard (boundary accepted) | `pi_interactive_rejects_a_prompt_too_large_for_argv` |
| Pi non-interactive stays stdin whatever the prefix or size | `pi_non_interactive_prompt_stays_on_stdin_whatever_its_size_or_prefix` |
| The five unchanged providers and Kimi keep their shapes | fleet test rows; existing `claude_/codex_/opencode_/gemini_/qwen_…` and `kimi_interactive_continues_using_prompt_argv_flag` tests |

- **Regression proof (by construction, not re-run).** The pre-Phase-3
  profiles emit `run -t <p>` (Goose), `-- <p>` (Kilo), `--print <p>`
  (Antigravity), and `Stdin` (Pi). Each differs from its fleet row, and Pi
  also trips the no-stdin assertion, so the fleet test fails on the old code.
- **Checkpoint 3 row-deletion check.** With the Pi row deleted, the fleet test
  failed with "one expectation row per provider". The row was restored.
- **Placement.** All new tests live in `profile/tests/positional.rs`, which the
  `claudine` bin target compiles as `#[cfg(test)] mod positional`. No path
  segment carries a tier marker, so they run in L1 (`just test`).
- **End-to-end coverage** of a repaired profile through the real pipeline
  (a Pi stub receiving `-- <p>` with a TTY on stdin) is Phase 4's L2 task.
  The live Pi behavior, including #9200 and whether `initialMessages`
  auto-submits, is Phase 4's `real_pi_interactive_startup.rs`.

### Evidence limits

- No live provider or tmux run was possible (see the Pi departure above). The
  Goose, Kilo, and Antigravity forms rest on the Phase 1 spike's
  source, docs, and research tiers. A human-permitted pass over the spike doc's
  "Outstanding live checks" is still recommended before Phase 5 describes
  per-provider behavior as verified.
- **OS.** The change builds argv vectors only: no paths, processes, or
  `cfg` branches. `just cross-check` was not run for this phase. CI's
  Linux and macOS legs cover the unit tests, and the Windows leg runs
  after merge.

### Gates (Checkpoint 3)

- `cd claudine && just test` (macOS): **7739 passed, 9 skipped, 0 failed**.
  The first run failed only on `dispatch_inventory_matches_committed_file`
  (resolved above).
- `cd claudine && just lint`: clean. The only output is the long-standing
  macOS linker `__eh_frame section too large` warning.
- No docs or skill drift was found. `rg` over `claudine/docs` and
  `.claude/skills/claudine` found no current page stating the old Goose,
  Kilo, Antigravity, or Pi interactive shapes. Phase 5 still owes the
  research notes (Pi `--` floor) and the timeline entry.

## Phase 4

Started and finished 2026-09-28 on macOS (dev Mac). Phase 4 adds the
terminal-backed coverage: an L2 file driving `--edit -i` through a detached
tmux session with fake providers, and an opt-in `real_` file driving the
installed Pi. No production source changed. Unlike Phases 1–3, the live runs
were possible: this session's policy still refused `tmux` and `pi` as direct
commands, but `just test-l2` and `just test-real` ran them inside the tests.

### What changed

- **New L2 file** `claudine/cli/tests/level2/level2_edit_interactive_capture.rs`,
  declared `#[cfg(unix)]` in `level2/main.rs` (N9). Each case builds its own
  `CliProcessFixture` and runs `claudine` under `env -i` from a launcher
  script in the shared tmux pane, following `level2_provider_overlay_capture.rs`.
  The fake editor is named by bare `EDITOR=fake-editor` (Claudine splits the
  editor command on whitespace) and acts on a `mode` file: `write`, `empty`,
  or `fail` (exit 3), copying its starting buffer to `seed.md` first. The fake
  provider records one file per argument (so a multiline argument survives
  intact), the argument count, both TTY checks, non-TTY stdin, and one line
  per launch. `TMPDIR` keeps the editor buffer inside the fixture.
- **New real file** `claudine/cli/tests/real/real_pi_interactive_startup.rs`,
  declared `#[cfg(unix)]` in `real/main.rs`. It is gated in the body (no
  `#[ignore]`, per the `rust-testing` skill) on `CLAUDINE_CONTRACT_REAL=1`,
  `pi` on `PATH`, and tmux. Each test runs `claudine pi … -i` in its own owned
  tmux session, waits for the deterministic `claudine-probe` model
  (`tests/fixtures/steering/pi-probe.ts`) to answer the first turn, types a
  second turn, waits for that answer, checks that `probe.json` saw exactly
  the two user turns, then quits with Ctrl+C. A `pi` shim in the fixture `bin`
  records Claudine's argv and `exec`s the real Pi, so each test also asserts
  that the argv ends in `-- <message>`.
- **Guard amended** (`claudine/cli/tests/l1/spawn_site_guard.rs`,
  `a_terminal_tier_name_must_match_the_resource_the_file_owns`). The guard
  failed the real file: any file that builds an emulator session must be
  named `level2_`/`level3_`, on the premise that `just test` otherwise selects
  it. A `real_` name also keeps a file out of `just test`, so the premise does
  not hold for it. The unlabeled check now skips `real_` files; the
  mislabeled check is unchanged. The alternative, a `level2_` name inside the
  `real` binary, would carry two tier markers.

### Real Pi result (AC12, #9200)

`just test-real real_pi_interactive_startup::` on Pi **0.87.1**
(`/Users/ken/.bun/bin/pi`): **4 passed**. The four cases are the direct
`"<p>" -i` prompt, `--edit -i` with a multiline bullet prompt, a ~2 KB prompt,
and a prompt starting with `@`. Each first turn was submitted from argv, and
the TUI took a second turn in the same session.

- **#9200 did not reproduce** at 2 KB on 0.87.1. This evidence resolves the
  Pi human-review item, and the plan's Pi task is ticked.
- **Negative control for `@`.** With the leading-space rule temporarily
  disabled in `profile/pi.rs`, the `@` test failed: Claudine passed
  `["--", "@TEMPLATE_NONCE is not a file"]` and Pi never answered. The rule is
  load-bearing. `pi.rs` was restored (`git diff` empty), and the test passed
  again.

### Departures and findings

- **Pi's fixture model comes from `settings.json`, not Pi flags.** The first
  attempt passed Pi's flags after Claudine's `--`
  (`claudine pi "<p>" -i -- --offline --provider claudine-probe …`). Pi
  received `pi -- --offline … -- <p>` and read every flag as a message. The
  test now writes `defaultProvider`, `defaultModel`, `defaultThinkingLevel`,
  `extensions`, and `sessionDir` to `settings.json` in `PI_CODING_AGENT_DIR`
  and sets `PI_OFFLINE=1`, so Claudine's argv holds only wrapper arguments.
  Passing the flags without `--` was rejected: Claudine's positional-prompt
  scan knows only `COMMON_VALUE_TAKING_FLAGS`, so in the `--edit` case (no
  positional) `claudine-probe` would be taken as the seed prompt.
- **Finding, out of scope (Rule 3): a user `--` after the positional reaches
  the provider verbatim.** Claudine keeps the `--` literal in the provider
  argv. Every provider then reads the "opaque" arguments after it as
  positionals, not options. A profile that appends its own `--` (Pi
  interactive, and Codex and Kilo for `-`-prefixed prompts) produces two
  separators. Reproduction: `claudine pi "hello" -i -- --offline` launches
  `pi -- --offline -- hello`. File it in Phase 5 next to the N6 defect.
- **`prompt empty; aborted` is invisible by default.** It is `log::info`,
  which prints only when tracing is enabled (`RUST_LOG` or `--debug info`),
  not under `-v`. The AC3 test runs `claudine --debug info codex --edit -i
  seed` to observe it, and also asserts `opening fake-editor for prompt...`.
  The default silent clean exit is existing behavior (R3) and was not
  changed. Phase 5 docs should not promise that message without `--debug`.
- **The editor diagnostic is a status block.** AC4 asserts its fields
  (`EditorError: editor exited with error`, `Editor: fake-editor`,
  `Exit code: 3`) rather than the one-line `Display` text.
- **Nextest's 30 s slow-timeout bounds the real tests.** The turn wait is
  20 s. Each test took about 2.3 s.

### Requirement → test mapping

| Requirement | Test (tier) |
|---|---|
| AC1/AC2: `--edit --interactive`, `--edit -i "seed"`, and `"seed" --edit -i` launch interactive Codex with the edited text as the leading positional, a TTY on stdin and stdout, no stdin feed, no `exec`, no leaked wrapper flag or seed, and no conflict text. The editor started from the seed (or empty) | `level2_edit_interactive_capture::level2_tmux_edit_interactive_delivers_the_edited_prompt_as_the_first_turn` (L2) |
| AC8: a multiline prompt starting with `- ` reaches Codex whole after `--` | `…::level2_tmux_edit_interactive_delivers_a_bullet_prompt_after_end_of_options` (L2) |
| AC3: an empty buffer exits 0, logs the abort, and launches nothing | `…::level2_tmux_edit_interactive_empty_buffer_aborts_without_launching` (L2) |
| AC4: editor exit 3 shows the typed diagnostic, exits 1, and launches nothing | `…::level2_tmux_edit_interactive_editor_failure_launches_nothing` (L2) |
| AC6: `--dry-run --edit -i` runs the editor; the header shows `Codex`, the prompt, and `Interactive`; `[DRY RUN]` appears; exit 0 with no `codex` on `PATH` | `…::level2_tmux_edit_interactive_dry_run_previews_without_a_provider` (L2) |
| AC9 (first half): plain `--edit` launches `exec` with the prompt on stdin | `…::level2_tmux_edit_without_interactive_stays_non_interactive` (L2) |
| Repaired profile through the pipeline: Pi gets `-- <bullet prompt>`, a TTY on stdin, and no print-mode flag | `…::level2_tmux_pi_edit_interactive_passes_the_prompt_on_argv_and_keeps_the_terminal` (L2) |
| AC12: real Pi submits the direct and the edited startup prompt and stays interactive | `real_pi_interactive_startup::real_pi_direct_interactive_prompt_is_the_first_turn`, `…::real_pi_edited_interactive_prompt_is_the_first_turn` (real) |
| #9200: a ~2 KB positional message | `…::real_pi_two_kilobyte_interactive_prompt_is_the_first_turn` (real) |
| A prompt starting with `@` stays a message | `…::real_pi_interactive_prompt_starting_with_at_is_a_message` (real) |
| The tier-name guard admits a `real_` file with an emulator session | `spawn_site_guard::a_terminal_tier_name_must_match_the_resource_the_file_owns` (L1, amended) |

- **Regression proof.** With `conflicts_with = "interactive"` temporarily
  restored on `WrapperArgs::edit`, 6 of the 7 L2 tests failed. The plain
  `--edit` control passed, as it should. `flags.rs` was restored (`git diff`
  empty). The `@` negative control is described above.
- **Placement.** Both files are compiled by declared binaries (`level2`,
  `real`) and selected by live recipes, since every test path starts with
  `level2_` or `real_`. `just check-tier-coverage claudine` reports 0 stranded.

### Gates (Checkpoint 4)

- `just test-l2 edit_interactive_capture` (macOS, tmux): **7 passed**.
- `just test-real real_pi_interactive_startup::` (macOS, Pi 0.87.1):
  **4 passed**.
- `cd claudine && just test` (macOS): **7739 passed, 9 skipped, 0 failed**.
  The first run failed only on the tier-name guard (resolved above).
- `cd claudine && just lint`: clean. The only output is the long-standing
  macOS linker `__eh_frame` warning. `just lint` does not enable
  `terminal-tests` or `real-tests`, so the new files were also checked
  directly. `cargo clippy -p claudine-cli --test real --features real-tests
  -- -D warnings` is clean. `cargo clippy … --features terminal-tests`
  reports nothing in the new L2 file but fails on a **pre-existing**
  `needless_lifetimes` lint in `level2_dry_run_metadata_capture.rs:283`,
  which this phase did not touch.
- **OS.** Both new files are Unix-only. The L1 change is a filename rule with
  no OS dependence, so `just cross-check` (which runs L1 only) was not run.
  Linux L2 evidence comes from the PR CI leg, as the plan's checkpoint
  states. The `real_` tier does not run in CI.

### Skill update not made

The session's permission policy refused the write to
`.claude/skills/claudine/SKILL.md`. The intended addition goes after the
sentence ending "so any raw spawn is now simply a failure." in the L1 spawn
contract paragraph:

> The same guard requires a test file that builds an emulator session
> (`TmuxHarness`, `WezTermHarness`, …) to be named `level2_`/`level3_`; a
> `real_` file is exempt, since `just test` already excludes it, so a
> real-provider test may drive its provider's TUI in tmux
> (`real/real_pi_interactive_startup.rs`).

Phase 5 should apply it along with its planned skill edits.
