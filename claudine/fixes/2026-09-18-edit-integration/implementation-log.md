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
