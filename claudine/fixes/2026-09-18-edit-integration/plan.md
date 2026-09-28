---
created: 2026-09-28
total_phases: 5
phase: 1
agent: claude/opus
yolo: true
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
packages:
    - claudine-cli
---

# Plan: Compose `--edit` with Interactive Provider Sessions

Spec: [spec.md](./spec.md) (`2026-09-18-edit-integration`).

## Summary and Definition of Done

### The work

The spec reads as a two-line deletion: drop clap's `conflicts_with = "interactive"`
on `WrapperArgs::edit` (`claudine/cli/src/commands/wrap/flags.rs:48`), and drop the
duplicate `edit_requested && interactive_requested` branch in
`validate_timeout_constraints` (`claudine/cli/src/commands/wrap/wrapper_stages.rs:33`).
Planning research shows the deletion is the small part. The work has four
parts:

1. **Wrapper validation (provider-agnostic).** Remove both conflict checks.
   Add a pre-editor check that rejects an explicit interactive request combined
   with `--timeout` or `--step-timeout`. Today the timeout checks run in Stage 5,
   *after* the editor (Stage 3 of `run_provider_wrapper_inner`,
   `claudine/cli/src/commands/wrap/mod.rs:275`), so without that check a user
   would draft a prompt and then hit a flag error.
2. **Interactive startup-prompt delivery across the fleet.** Once `--edit -i`
   is accepted, every profile's *interactive* `prompt_delivery` is exercised by
   a real user. Reading the profiles against the installed provider CLIs
   (2026-09-28) shows the spec's Open Question covers only Pi, but five
   profiles are suspect:

   | Provider | Current interactive delivery | Native evidence (help / research) | Suspected defect |
   |---|---|---|---|
   | Goose | `run -t <p>` (ignores mode) | `goose run -t "…" --interactive` continues interactively (research `agent-cli/goose.md:444`) | runs one-shot; never interactive |
   | Kilo | `-- <p>` (ignores mode) | TUI is `kilo [project]` with `--prompt`; positional is a **project path** | prompt read as a directory |
   | Antigravity | `--print <p>` (ignores mode) | `agy --prompt-interactive` "Run an initial prompt interactively and continue" | headless one-shot |
   | Kimi | `--prompt <p>` | kimi 2.0.2: `-p, --prompt` "Run one prompt non-interactively"; no positional message | **possibly no interactive surface** |
   | Pi | stdin (ignores mode) | pi 0.87.1 help: `pi [options] [--] [@files...] [messages...]`, `--` now documented | stdin to a TUI breaks the TTY; profile comment (0.80.3) says `--` is rejected — stale |

   Claude, Codex, Gemini, Qwen, and OpenCode already use a documented
   interactive surface (positional prompt, `--prompt-interactive`, `--prompt`),
   and their unit tests exist in `profile/tests/positional.rs`. They still need
   real-terminal confirmation in the Phase 1 spike.
3. **Regression coverage.** Unit, L1, a hidden-terminal L2, and one opt-in
   `real` test for Pi (AC12).
4. **Docs, help, and skill drift.** Covers the getting-started page's wrong
   claim about plain `--edit`, the help text, the `cli-reference` skill page,
   and the timeline.

### Definition of done

- [ ] `claudine <provider> --edit --interactive` and `-i`, with a seed on either
      side of the positional, launch every provider interactively with the
      edited text as the first user turn. Evidence: Phase 1 spike records for
      the real CLIs, a unit fleet test for every compiled `Provider`, and the L2
      test for the pipeline.
- [ ] Plain `--edit` still launches non-interactive after a non-empty edit, and
      plain `<prompt> -i` still selects interactive mode.
- [ ] `--interactive` with `--timeout` or `--step-timeout` fails **before** the
      editor opens.
- [ ] Empty buffer, editor failure, and non-TTY each end with no provider
      launch, and the existing diagnostics are unchanged.
- [ ] `--dry-run --edit -i` runs the editor, shows the edited prompt and the
      `Interactive` badge, and needs no provider binary.
- [ ] No provider allowlist, capability flag, silent fallback, or warning was
      added. Any provider without a native interactive startup surface was
      escalated per Rule N2 instead.
- [ ] Docs, help, and skills describe the corrected behavior. A drift test pins
      the advertised `claudine codex --edit -i`.
- [ ] `just test`, `just test-l2`, and `just lint` in `claudine/` are green. The
      implementation log records every departure from the spec.

Input Robustness Matrix: **not applicable**. No file format, manifest, or
configuration reader is added or changed.

## Phase 1 — Rulings and Spikes

### Necessary Rules

These rulings resolve ambiguities found while planning. Phases 2–5 implement
them as written. An implementer who disagrees raises it in the implementation
log before departing.

- [x] **N1. Repairing a defective interactive delivery is in scope.**
  R2 ("If a profile's existing interactive delivery is defective, repair that
  profile for all interactive startup prompts") overrides the Non-Goal
  "Changing how provider profiles encode or transport an interactive initial
  prompt". Read the non-goal as "no new delivery mechanism": no new
  `PromptDelivery` variant, no keystroke injection, no editor-only path.
  Correcting which existing variant and which native flags a profile emits is
  in scope. It applies to the direct `<prompt> -i` path too, which is the same
  code.
- [x] **N2. A provider with no native interactive startup-prompt surface stops the fleet claim.**
  The spec demands this for Pi. The rule applies to any provider, and Kimi is
  the likely case. If the Phase 1 spike finds no surface, do not implement a
  refusal, fallback, warning, or allowlist. Record the evidence, set the
  spec's `status: human-in-the-loop`, and ask the author to amend the
  fleet-wide promise. Phase 2 and the repairs for the other providers
  continue. Only that provider's Phase 3 task and the fleet test's expectation
  for it wait on the ruling.
- [x] **N3. Where the pre-editor timeout check sits.**
  It runs right after Stage 2 (wrapper flag extraction) and before Stage 3
  (prompt extraction and editor). It rejects only `interactive_requested &&
  (timeout || step_timeout)`, with today's messages:
  `--timeout cannot be used with --interactive mode` and
  `--step-timeout cannot be used with --interactive mode`. Binary resolution
  (Stage 1) keeps its place (R3). Keep the post-edit
  "only in non-interactive mode" checks: they depend on whether the edited
  prompt is empty. Keep the later duplicate at `mod.rs:624–636` too, since
  removing it is out of scope. Drop the now-unused `edit_requested` parameter
  from `validate_timeout_constraints`.
- [x] **N4. Only `-i`/`--interactive` is recovered from passthrough, not the timeouts.**
  `extract_wrapper_flags_from_passthrough` lifts `-y -i --edit --repo -q
  --silent -v --perf --operation`. It does **not** lift `--timeout`,
  `--step-timeout`, or `--dry-run`. AC11's "flag recovered after a positional
  seed prompt" is therefore the interactive flag. Test forms:
  `claudine codex --timeout 5m "seed" --edit -i` and
  `claudine codex --step-timeout 5m "seed" --edit --interactive`.
- [x] **N5. The dry-run preview is the wrapper header.**
  The dry-run `Command:` line is built from `child_args` before Stage 13
  delivers the prompt, so it never shows a prompt, on any path. The header
  already renders `Claudine ▸ Codex  Interactive  '<prompt>'` (observed
  2026-09-28). AC6 asserts the header (run without `--silent`), plus the
  `[DRY RUN]` marker and that the provider stub did not run. Changing the
  `Command:` line is out of scope. `--dry-run` must come before the positional
  (N4).
- [x] **N6. Out-of-scope defect found while planning: flags after the positional leak to the provider.**
  `claudine codex "hello" -i --dry-run` passed `--dry-run` to the real `codex`
  and **launched it**. The same applies to `--timeout` and `--step-timeout`.
  Do not fix it here (Rule 3). Phase 5 files it as an `_unscheduled` fix.
- [x] **N7. Fleet test inventory: no new `match Provider`.**
  `dispatch_inventory.rs` guards against new decentralized `match Provider`.
  The fleet test iterates `claudine::provider::PROVIDERS_DISPLAY_ORDER`, as
  the neighboring fleet tests do. It asserts that the list covers
  `PROVIDER_COUNT` distinct providers and that its expectation table has a row
  for every provider. A missing row fails, so a new variant cannot pass
  silently.
- [x] **N8. The editor diagnostic text is unchanged.**
  `--edit requires an interactive terminal` stays byte-identical because
  tests assert it. Help and docs explain that it means the editor's terminal
  I/O, not the provider session mode.
- [x] **N9. OS coverage for the L2 test.**
  The L2 test runs in a detached tmux session on Unix (macOS and Linux), like
  `level2_provider_overlay_capture.rs`. The code change only deletes checks
  and reorders validation, so Windows is covered by L1 (validation order and
  conflict removal through the non-TTY path) and by the profile unit tests. A
  Windows WezTerm variant is not required. Record this in the implementation
  log.
- [x] **N10. What "plain `-i` with a direct prompt is unchanged" means.**
  AC9's "unchanged" covers **mode selection**. For the profiles repaired in
  Phase 3, direct `claudine goose "x" -i` (and likewise for the other repaired
  providers) intentionally changes from a broken launch to a correct
  interactive one. The README and docs say so.

### Spikes

- [x] **S1. Fleet interactive startup-prompt verification (real CLIs, hidden terminal)**
  - *Done at source/docs/research tier only (2026-09-28): the Phase 1
    session's permission policy refused `tmux` and every provider binary. The
    live checks are listed in `spike-interactive-startup.md` → "Outstanding
    live checks". Result: Kimi triggers N2.*
  - Launch each installed provider (`claude codex gemini kimi opencode qwen
    kilo pi agy`) in a **detached tmux session** (no focus) with its native
    interactive startup-prompt form. Use a harmless prompt: "Reply with the
    single word READY." Use one plain prompt and one multiline prompt whose
    first line is `- item`.
  - For each, record whether the first turn was submitted, whether the TUI
    stayed usable for a second turn, and the exact argv, flag spelling, and
    leading-dash handling (`--flag=value`, `--`, attached value). Then quit
    through the TUI or by killing the session.
  - Goose is not installed on the dev Mac. Take its evidence from another host
    listed by the `os` skill, or from the upstream CLI source and docs for
    `run --interactive` and `-t` leading-dash handling. Mark the evidence tier.
  - Kimi: confirm or refute that 2.0.x has no interactive startup prompt.
    Check stdin to the TUI, a positional message, and any config or env route.
    If none exists, trigger **N2**.
  - Output: `claudine/fixes/2026-09-18-edit-integration/spike-interactive-startup.md`,
    one row per provider with version, verified form, and evidence.
- [x] **S2. Pi version floor for `--`**
  - Find the first Pi release that accepts `--` as end-of-options, from the
    changelog or by bisecting npm versions with `bunx`/`npx` in a scratch
    directory. Confirm that `pi -- "<multiline - prompt>"` submits the first
    turn in the TUI (S1).
  - Decide the profile shape: interactive → `AppendArgs(["--", prompt])`;
    non-interactive stays stdin. If the floor is newer than what the research
    docs record, update `docs/research/agent-cli/pi.md` in Phase 5 and state
    the minimum version. Do not add version sniffing.
- [x] **S3. Argument-size headroom for newly argv-delivered prompts**
  - Pi (and Antigravity or Goose, if repaired) move to argv in interactive
    mode. Confirm that Kilo's `ARG_MAX_HEADROOM` pattern (`kilo.rs:60`) is the
    precedent to reuse. Decide whether a shared helper or a per-profile guard
    is simpler: one call site means per-profile, per Rule 2.

### Gate

- [x] **G1.** Mark S1–S3 findings in the spike doc, and mark N1–N10 as accepted
  in the implementation log (`implementation-log.md`, created now). If N2
  fired, set the spec to `status: human-in-the-loop`, note which provider is
  blocked, and continue with Phase 2.

**Wave 1** (parallel): S1, S2, and S3. S1 and S2 share the Pi row, so S2 runs as
a sub-task of the same agent, or right after S1's Pi row. Every sub-agent brief
must state that the session tree is non-interactive and that tmux sessions stay
detached.

## Phase 2 — Wrapper Validation (provider-agnostic)

Depends on Phase 1 rulings only, not on the spikes. It can start while Wave 1
runs.

- [x] **Remove clap conflict**
  - Delete `conflicts_with = "interactive"` from `WrapperArgs::edit`
    (`flags.rs:48`).
  - Update the `--edit` doc comment and the rich help line (`flags.rs:255`) to
    describe prompt authoring only. Add a note that the prompt can be combined
    with `-i` for an interactive session.
- [x] **Remove runtime conflict**
  - Delete the `edit_requested && interactive_requested` branch and the
    `edit_requested` parameter from `validate_timeout_constraints`. Update its
    single caller (`mod.rs:394`).
- [x] **Pre-editor timeout check**
  - Add the N3 check between Stage 2 and Stage 3 in
    `run_provider_wrapper_inner`, as a small `pub(crate)` function in
    `wrapper_stages.rs` beside `validate_timeout_constraints`, so it can be
    unit-tested.
  - Remove the matching two branches from `validate_timeout_constraints`,
    which the pre-editor check now owns, so one policy lives in one place.
- [x] **Comment pass**
  - Review every `///` and `//` comment within the Stage 2–5 region, in
    `prompt_source.rs`, and in `flags.rs`. Delete any rationale that exists
    only to justify the false conflict. Fix any drifted comment and log it.
- [x] **Unit tests (L1, in-crate)**
  - `flags/tests.rs`: `WrapperArgs` parses `--edit --interactive` and
    `--edit -i` (`try_parse_from`). Extend the existing proptest (`flags/tests.rs:405`)
    to assert that `--edit` and `-i` can coexist after the positional.
  - `wrapper_stages.rs`: the pre-editor check rejects `-i` with each timeout
    and accepts `--edit -i` alone. `validate_timeout_constraints` no longer
    rejects edit with interactive.
- [x] **L1 integration tests (`claudine/cli/tests/l1/wrap_basics.rs`)**
  - Replace `wrapper_rejects_edit_and_interactive_conflict` with non-TTY
    tests for `--edit --interactive`, `--edit -i`, and
    `"seed" --edit --interactive`. Each expects
    `--edit requires an interactive terminal`, no `cannot be used with`
    text, no fake-editor marker, and no provider stub invocation (AC5, AC2).
  - AC11: the N4 forms, with a fake `EDITOR` that writes a marker file, fail
    with the timeout message and leave no marker. Under non-TTY this also
    proves ordering: the timeout error wins over the TTY error only if the
    check runs first.
  - AC9 (second half): `claudine codex "prompt" -i` still launches the stub
    interactively (no `exec` entrypoint). Reuse an existing test if one covers
    this; otherwise add one.
  - Use `CliProcessFixture::command()`. No raw spawns (`spawn_site_guard.rs`).

**Wave 2** (parallel with Wave 1): one agent does all Phase 2 tasks in order,
since they are small and touch the same three files.

**Checkpoint 2:** `cd claudine && just test` is green, and `just lint` is clean for
`claudine-cli`.

## Phase 3 — Interactive Delivery Repairs and Fleet Evidence

Depends on G1. Each repair uses only the native form verified in S1/S2. A
provider blocked under N2 skips its repair task until the author rules.

- [x] **Goose repair** (`profile/goose.rs`)
  - Interactive: `run -t <prompt> --interactive`. Handle a leading `-` per S1
    (for example `--text=<p>`). Non-interactive shape unchanged.
- [x] **Kilo repair** (`profile/kilo.rs`)
  - Interactive: the `--prompt` form, mirroring OpenCode's attached-value
    handling for a leading `-` (`positional.rs:174`). Keep `-- <p>` and the
    `ARG_MAX_HEADROOM` guard for `run`.
- [x] **Antigravity repair** (`profile/antigravity.rs`)
  - Interactive: `--prompt-interactive <p>` (or `=<p>` per S1). Non-interactive
    stays `--print` last on argv. Update the profile comment.
- [ ] **Pi repair** (`profile/pi.rs`)
  - Interactive: `AppendArgs(["--", prompt])` per S2, with the S3 size guard.
    Non-interactive stays stdin. Rewrite the stale 0.80.3 comment to state
    both channels and the minimum version.
  - *Code landed 2026-09-28 (Phase 3), including the `@`-prefix handling and
    the size guard. Left open until upstream issue #9200 is ruled on by the
    author or checked live (Phase 4 real Pi test with a prompt of about 2 KB);
    see the implementation log.*
- [ ] **Kimi resolution** (`profile/kimi.rs`)
  - If S1 found a surface, repair to it. If N2 fired, leave the code, add
    nothing, and wait for the author's ruling. Do not tick the fleet
    definition-of-done item until it is resolved.
  - *N2 still unruled at Phase 3 (2026-09-28): `kimi.rs` untouched; the fleet
    table's Kimi row pins today's `--prompt` argv with a comment saying it is
    not a verified interactive form.*
- [x] **Confirm the unchanged five**
  - Claude, Codex, Gemini, Qwen, and OpenCode: no code change unless S1
    disproves the current form. If it does, repair in the same pattern and
    log it.
- [x] **Fleet test** (`profile/tests/positional.rs`, AC7 and AC8)
  - Add `every_provider_delivers_an_interactive_startup_prompt` per N7. For
    each provider it asserts the exact interactive `PromptDelivery` shape from
    the expectation table, for (a) a plain prompt and (b) a multiline Markdown
    prompt starting with `- `. It also asserts no provider returns `Stdin` or
    `WireRpc` in interactive mode, since stdin must stay the TTY.
  - Keep the existing per-provider interactive tests. Add per-profile tests
    for each repaired provider, covering plain and leading-dash prompts.

**Wave 3** (parallel after G1): Goose, Kilo, Antigravity, and Pi repairs, one
agent each, each owning only its profile file and that profile's unit tests.
Kimi joins when ruled. **Wave 4**: the fleet test, written after Wave 3 merges,
since it asserts every repaired shape.

**Checkpoint 3:** `just test` is green. The fleet test fails if you delete any
row from its table: try it locally once, then revert.

## Phase 4 — Terminal-backed Coverage

Depends on Phase 2. The Pi and repaired-profile assertions depend on Phase 3.

- [ ] **L2 edit-interactive capture** (`claudine/cli/tests/level2/level2_edit_interactive_capture.rs`, registered in `level2/main.rs`)
  - Harness: `biscuit_test_harness::TerminalHarness` in a **detached tmux**
    session. Launch under `env -i` with only fixture variables, following
    `level2_provider_overlay_capture.rs`. Gate with `require_level!`. Never
    take focus.
  - Fixtures: a fake editor (`EDITOR` script) that writes a known prompt,
    empties the buffer, or exits non-zero, depending on a mode file. A fake
    `codex` stub records argv, whether stdin and stdout are TTYs, and draws a
    banner.
  - Cases (each with its own fixture):
    - AC1/AC2: `codex --edit --interactive`, `codex --edit -i "seed"`, and
      `codex "seed" --edit -i`. The stub sees the edited text as the first
      turn with the interactive shape (no `exec`) and has a TTY. For the seed
      cases, assert that the editor buffer began as the seed.
    - AC8: the fake editor writes a multiline prompt starting with `- `. The
      stub receives it with the `--` separator.
    - AC3: an empty buffer exits 0 with `prompt empty; aborted` and no stub
      record.
    - AC4: the editor exits non-zero. The typed editor diagnostic is shown,
      there is no stub record, and the exit is non-zero.
    - AC6: `codex --dry-run --edit -i`, with **no** `codex` on the fixture
      `PATH`. The header shows `Interactive` and the edited prompt, plus
      `[DRY RUN]`, and the exit is 0 (N5).
    - AC9 (first half): `codex --edit` with a non-empty edit. The stub
      records the non-interactive `exec` shape.
  - Also cover one repaired provider end to end (Pi fake stub:
    `--` + prompt on argv, stdin is a TTY) to prove that Phase 3 output flows
    through the real pipeline.
- [ ] **Real Pi startup test** (`claudine/cli/tests/real/real_pi_interactive_startup.rs`, AC12)
  - Opt-in `real-tests` tier, following `real_pi_steering.rs`. Run real `pi`
    in detached tmux, once with `claudine pi "Reply READY" -i` and once with
    `--edit -i` using a fake editor. Assert that the TUI shows the reply and
    accepts a second input, then quit.
  - The spike record plus this test is AC12's "evidence beyond generated
    arguments". Run it once locally and record the result in the
    implementation log.

**Wave 5** (parallel): the L2 file and the real Pi test (different agents and
different files).

**Checkpoint 4:** `cd claudine && just test-l2 edit_interactive_capture` is
green on macOS. Linux evidence comes from the PR CI leg. The real Pi test
passed once locally, with the result logged.

## Phase 5 — Docs, Drift, and Closure

Depends on Phases 3 and 4, since the docs state the final per-provider behavior.

- [ ] **Getting-started fix** (`claudine/docs/getting-started/index.md:316–319`)
  - Plain `claudine codex --edit` starts a **non-interactive** session after a
    non-empty edit. Keep `claudine codex --edit -i` as the interactive variant.
    Note that `--edit` needs a terminal for the editor, whatever the session
    mode.
- [ ] **Reference docs**
  - `.claude/skills/claudine/cli-reference.md`: add an `--edit` row to the
    wrapper-flag table (~line 479) and extend the "Interactivity default"
    bullet (~line 506) so an edited prompt behaves like a direct one.
  - `claudine/docs/topics/system-prompt.md:42`: check that the wording still
    holds.
  - Correct any other current doc that `grep -rn -- "--edit"` finds under
    `claudine/docs` and `.claude/skills/claudine` and that implies a
    non-interactive-only restriction. Completed specs stay untouched.
- [ ] **Provider research and profile notes**
  - `docs/research/agent-cli/pi.md`: record the `--` floor (S2) and the
    interactive positional message.
  - Antigravity, Kilo, and Goose research: add the verified interactive
    startup forms if missing. No generated `data.rs` change is expected. If
    `claudine providers generate` shows drift, regenerate and include it.
- [ ] **Drift test** (AC10)
  - L1 test `wrap_basics.rs::getting_started_edit_interactive_form_is_accepted`.
    It reads `docs/getting-started/index.md`, asserts the literal
    `claudine codex --edit -i` is present, then runs `codex --edit -i` through
    `CliProcessFixture` and asserts the terminal-requirement diagnostic, not a
    conflict.
  - Spell the repository read in a form the `rust-testing` skill lists, so CI
    schedules the narrowed test-input cell (`docs/cicd/test-inputs.md`).
- [ ] **Skill snapshots**
  - Add a `.claude/skills/claudine/timeline.md` entry dated on landing
    covering the conflict removal, the pre-editor timeout check, and the
    repaired profiles.
  - Update `SKILL.md`'s wrapper line only if its wording implies the old
    restriction.
- [ ] **File the N6 defect**
  - Add `claudine/fixes/_unscheduled/wrapper-trailing-flags-leak.md`
    describing `--dry-run`, `--timeout`, and `--step-timeout` after the
    positional reaching the provider, with the reproduction from N6.
- [ ] **Final validation**
  - `cd claudine && just test && just test-l2 && just lint`. Run
    `just ci-local --plan` and review the scheduled cells before any push.
  - Grep for leftovers: `rg "cannot be used with --interactive" claudine`
    returns only the two timeout messages, and
    `rg "conflicts_with = \"interactive\"" claudine/cli/src/commands/wrap`
    returns nothing.
  - Complete `implementation-log.md` (rulings applied, spike outcomes,
    departures, and the N2 status). Set the spec's `implemented: true` only
    if N2 did not block. Otherwise leave it `human-in-the-loop`. Do not move
    the fix to `_completed`: the author does that.

**Wave 6** (parallel): the getting-started and reference docs, the research
notes, and the timeline and N6 filing. **Wave 7**: the drift test (it reads the
edited page), then final validation.

**Checkpoint 5:** everything in the Definition of Done is ticked, or explicitly
deferred to the author under N2.
