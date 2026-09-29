---
created: 2026-09-18
status: human-in-the-loop
human_review: true
human_review_items:
    - |-
      **Kimi cannot start an interactive session with a first message. What should `claudine kimi --edit -i` promise?**

      **DECISION:** KIMI is not working until we update the research and get caught up to latest version. This WILL work but we are dependent on this research -> gen task. This should not prevent this spec from being marked as production ready.

      The spec promises that every provider opens an interactive session with the edited text as the first message. Kimi Code's own docs say its only prompt option (`--prompt`) runs one prompt and exits. It has no positional message and no option that seeds an interactive session, and an open upstream feature request asks for exactly that. So today `claudine kimi "hello" -i` quietly runs a one-shot session instead of an interactive one. This must be decided before Phase 3 finishes, because Kimi's row in the new all-provider test must encode whichever answer you pick. Phase 2 does not depend on it.

      - **A. Exclude Kimi from the all-provider promise, and have `claudine kimi … -i` with a prompt fail clearly before launch** (for example "Kimi cannot start an interactive session with an initial prompt; drop `-i` or start `kimi` without a prompt").
        Pros: honest, predictable, and a small change. It also fixes the existing direct `claudine kimi "x" -i`, which silently does the wrong thing today.
        Cons: this is a provider-specific exception, which the spec asked us to avoid, and it needs a one-line spec amendment.
      - **B. Keep today's behavior: pass `--prompt`, and Kimi runs one turn and exits.**
        Pros: no code change.
        Cons: `-i` would be a lie for Kimi. The session is not interactive, and nothing tells the user.
      - **C. Emulate it: run `kimi -p "…"`, then relaunch `kimi --continue` in the terminal.**
        Pros: the user ends up in an interactive session holding the first answer.
        Cons: it adds a second, Kimi-only launch path, which the spec explicitly forbids. It is also fragile (`--continue` picks "the most recent session", which is unsafe if two runs overlap) and splits one conversation across two processes.

      **Recommendation: A.** It is the only option that is both truthful and inside the spec's rule against new delivery paths. When Kimi ships an initial-prompt option, the refusal is replaced by that option.

      *Still open after Phase 5 (2026-09-28).* The docs now describe today's behavior honestly (Kimi runs one turn and exits), so choosing B needs no further doc change; A or C needs a code change plus a docs update.
    - |-
      **The Claudine agent skill was not updated, because the agent was not allowed to write to it. Please apply the prepared text, or let an agent do it.**

      The skill is the reference other agents read. Until it is updated, it does not mention `--edit` in the wrapper flag table, has no timeline entry for this fix, and misses one testing-rule note. This should be done before the fix is closed, so the next agent working on the wrapper sees the new behavior.

      - **A. Apply the three prepared edits by hand.** The exact text is in `implementation-log.md` under "Phase 5 → Skill edits not made" (and the `SKILL.md` sentence under "Phase 4 → Skill update not made"). Pros: a few minutes of copy-and-paste. Cons: manual.
      - **B. Run a short agent session that is allowed to write under `.claude/skills/claudine/`.** Pros: no manual work. Cons: one more session.

      **Recommendation: A.** The text is final and short; a new session adds nothing.
message_to_agent: |-
  Phase 1 (rulings and spikes) is done. Read `spike-interactive-startup.md` in this directory before Phase 3. Phase 2 (wrapper validation) does not depend on any of the open items below and can proceed as planned.

  1. Evidence tier: the live tmux run of the real CLIs was NOT possible, because the Phase 1 session's permissions refused `tmux` and every provider binary. Verdicts come from upstream source, changelog, and docs, plus the in-repo research. The spike doc lists the live checks still owed.
  2. N2 fired for Kimi (no interactive startup-prompt surface). The spec is `human-in-the-loop`. Do not add a refusal, fallback, or allowlist for Kimi until the author rules. Only Kimi's Phase 3 task and Kimi's row in the fleet expectation table wait on that ruling.
  3. Pi: stdin forces print mode (source `main.ts resolveAppMode`), so interactive must be `AppendArgs(["--", prompt])`, which needs Pi 0.84.3 or later. Two open points: (a) upstream issue #9200, a reported SIGKILL for positional messages of about 993 bytes or more, is escalated to the author, so do not finalize the Pi repair until it is ruled or checked live; (b) after `--`, Pi still reads a token starting with `@` as a file, so decide and test how a prompt starting with `@` is handled.
  4. Goose: clap `-t/--text` has no `allow_hyphen_values`, so use `--text=<prompt>` for a `-`-prefixed prompt, with `--interactive`. The same defect breaks the non-interactive `run -t <p>` path. That is outside this fix; file it in Phase 5 next to the N6 defect unless the author widens scope.
  5. S3 correction: only Pi newly moves to argv. Antigravity and Goose already deliver on argv in both modes. Use a per-profile size guard for Pi.

  Phase 2 (wrapper validation) is done; see the implementation log's `## Phase 2`.

  6. The two open human-review items above (Kimi, Pi #9200) were NOT touched by Phase 2 and still gate their Phase 3 tasks.
  7. New seam: `wrapper_stages::reject_interactive_timeouts(args, interactive_requested)` runs at the end of Stage 2, before the editor. `validate_timeout_constraints` is now `(args, non_interactive_requested)` and holds only the post-edit "can only be used in non-interactive mode" rules. Do not re-add interactive checks there.
  8. `claudine/cli/tests/l1/wrap_basics.rs` has a local `write_marker_executable` helper (sh or `.cmd` stub that only creates a marker file) plus `marker_editor_path`. Reuse them for the Phase 5 drift test `getting_started_edit_interactive_form_is_accepted`. Any `codex --edit …` L1 test now needs a `codex` stub on the fixture PATH, because binary resolution (Stage 1) runs before the editor.
  9. Phase 5: the `--edit` help line changed to "Draft the initial prompt in an external editor; combine with -i for an interactive session". `claudine/docs/getting-started/index.md` still quotes the old sentence ("Open the prompt in an external editor before launching the provider"); update it in the getting-started task.

  Phase 3 (interactive delivery repairs and fleet test) is done except for the Kimi and Pi rulings; see the implementation log's `## Phase 3`.

  10. Interactive shapes now: Goose `run --text <p> --interactive` (with `run` inserted at argv index 0 when absent; `--text=<p>` for a `-` prefix), Kilo `--prompt <p>` / `--prompt=<p>`, Antigravity `--prompt-interactive <p>` / `=<p>`, Pi `-- <p>` (a leading `@` gets a leading space: `-- " @…"`). Non-interactive shapes are unchanged. Kimi is untouched (N2 unruled).
  11. Pi's code landed before #9200 was settled, because this session also could not run `pi` or `tmux`. The plan's Pi task is deliberately unticked. Phase 4's `real_pi_interactive_startup.rs` must include a prompt of about 2 KB and a prompt starting with `@`, confirm that `initialMessages` auto-submits, and record the outcome. If the crash reproduces, change only the interactive branch of `profile/pi.rs`.
  12. Phase 4 L2: the fleet test lives in `profile/tests/positional.rs` (`every_provider_delivers_an_interactive_startup_prompt`, table keyed by `Provider::as_slug()`). For the L2 Pi stub case, expect argv `-- <prompt>` and no stdin seed.
  13. `claudine/docs/providers/dispatch-inventory.json` must match `cli/src` byte for byte. Any new `Provider::X` in `cli/src` or `lib/src`, including `#[cfg(test)]` files under `src/`, needs a regeneration: `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::` in `claudine/`. Phase 3's session could not set that variable, so it hand-edited 12 `direct-ref` records. The byte-compare test passes. Integration tests under `cli/tests/` are not scanned.
  14. Commit `57968bb03` was made by a separate process during Phase 3. It contains the Phase 3 source changes. Its message wrongly says "Phase 2" and claims the whole fleet is covered, which is not true while Kimi and Pi are open. Correct it at review time if the history is rewritten.

  Phase 4 (terminal-backed coverage) is done; see the implementation log's `## Phase 4`. Only the Kimi ruling (N2) remains open for the author.

  15. Pi is settled. `real_pi_interactive_startup.rs` passed on Pi 0.87.1 for the direct, edited, ~2 KB, and `@`-prefixed prompts: each first turn was submitted and the TUI took a second turn. #9200 did not reproduce, so the Pi human-review item was removed and the plan's Pi task is ticked. A negative control showed the leading-space `@` rule is required. Phase 5's Pi research note can state `-- <message>` as live-verified on 0.87.1 (0.84.3 minimum per the changelog).
  16. File a second `_unscheduled` fix next to N6's: a user `--` after the positional is forwarded to the provider verbatim. `claudine pi "hello" -i -- --offline` launches `pi -- --offline -- hello`, so the provider reads the "opaque" arguments as positionals, and profiles that append their own `--` (Pi interactive, Codex/Kilo for `-`-prefixed prompts) emit two separators. Observed live in Phase 4. Not fixed (Rule 3).
  17. Docs: `prompt empty; aborted` and `opening <editor> for prompt...` are `log::info`, printed only with `RUST_LOG` or `--debug info` (not `-v`). By default an empty buffer exits 0 silently. Do not document the message as always shown.
  18. The skill write to `.claude/skills/claudine/SKILL.md` was refused by Phase 4's permissions. The log's "Skill update not made" section holds the sentence to add (the spawn-site guard now exempts `real_` files that build an emulator session). Apply it with Phase 5's skill edits.
  19. Pre-existing, not from this fix: `cargo clippy -p claudine-cli --tests --features terminal-tests -- -D warnings` fails on `needless_lifetimes` in `level2_dry_run_metadata_capture.rs:283`. `just lint` does not enable that feature, so it stays hidden. Mention it rather than fix it.

  Phase 5 (docs, drift, closure) is done; see the implementation log's `## Phase 5`. This was the last phase.

  20. `implemented` is `true` because the Phase 5 instructions require it, but `status` stays `human-in-the-loop`: Kimi (N2) is still unruled, and the skill edits under `.claude/skills/claudine/` were refused by the session's permissions (exact text in the log). Plan tasks left unticked for those reasons: Kimi resolution, Reference docs (the `cli-reference.md` part), Skill snapshots, and the fleet and docs/skills Definition-of-Done items.
  21. Three `_unscheduled` fixes were filed as `<name>/spec.md`: `wrapper-trailing-flags-leak` (N6), `wrapper-user-separator-forwarded` (user `--` forwarded), and `goose-dash-prefixed-one-shot-prompt` (one-shot `run -t -…`).
  22. If Kimi is ruled A or C, update `claudine/docs/getting-started/index.md` (its Kimi bullet), the Kimi row of the fleet test, and the prepared `cli-reference.md` text together.
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-28
review_iterations: 1
implemented: true
area: claudine
packages:
    - claudine-cli
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
---

# Compose `--edit` with Interactive Provider Sessions

## Problem

Claudine rejects `--edit` when it is combined with `--interactive` (`-i`).
That restriction incorrectly couples two independent choices:

- `--edit` selects how the initial user prompt is authored; and
- `--interactive` selects the provider session mode after that prompt is
  authored.

Claudine intends to support an initial prompt when a provider is launched
interactively. Its direct wrapper exposes that intent through the ordinary
command-line form:

```sh
claudine <provider> "initial prompt" --interactive
```

An editor-authored prompt becomes the same `PromptSource::Inline` value as an
initial prompt supplied directly on the command line. Preventing that value
from continuing through the existing interactive prompt-delivery path is
therefore a wrapper validation defect, not a provider limitation.

The current behavior also contradicts the getting-started documentation, which
advertises `claudine codex --edit -i` as the interactive variant. That page
also incorrectly says plain `claudine codex --edit` launches an interactive
session; an edited, non-empty prompt selects non-interactive mode by default.

## Current Behavior and Root Cause

The restriction is enforced in two places:

1. [WrapperArgs::edit](../../cli/src/commands/wrap/flags.rs) in `claudine-cli`
   declares `conflicts_with = "interactive"`, so clap rejects the ordinary
   argument ordering.
2. [validate_timeout_constraints](../../cli/src/commands/wrap/wrapper_stages.rs)
   in `claudine-cli` independently rejects the combination after wrapper flags
   have been recovered from the passthrough argument bucket.

The duplicate runtime check exists because wrapper-owned boolean flags can be
recognized on either side of the first positional argument. Both checks encode
the same invalid policy and must be removed together.

The original edit-command design justified the conflict by claiming that most
provider CLIs could not seed an initial turn into an interactive session.
Claudine's wrapper profiles already distinguish interactive from
non-interactive prompt delivery, and their tests cover shapes such as
positional prompts, provider prompt flags, end-of-options separation, and
provider-specific argument placement. Those tests establish Claudine's
intended arguments, not that every native provider consumes them as an initial
turn. Pi needs the additional verification described under Open Questions.

The phrase `--edit requires an interactive terminal` describes a separate and
valid precondition: Claudine needs attached terminal input and output to launch
and wait for an external editor. It does not prescribe the mode of the provider
session launched afterward. Terminal availability and provider interactivity
must remain separate concepts in code, diagnostics, documentation, and tests.

## Required Behavior

### R1. Make prompt authoring and session mode orthogonal

All direct provider wrappers must accept both of these forms:

```sh
claudine <provider> --edit
claudine <provider> --edit --interactive
```

The first form retains its current behavior: after editing, launch the provider
in the default mode selected for a supplied prompt, which is non-interactive.

The second form must:

1. open the resolved editor with an empty buffer or the supplied seed prompt;
2. wait for the editor to exit;
3. read the resulting text through the existing editor workflow;
4. deliver that text as the provider's initial user turn using the existing
   interactive prompt-delivery path; and
5. leave the provider session interactive for subsequent user turns.

The target contract applies to Claude Code, Codex, Gemini CLI, Goose, Kimi
Code, OpenCode, Qwen Code, Kilo, Pi, and Antigravity. Do not add a provider
capability flag, allowlist, fallback to non-interactive execution, or warning
for this combination. Resolve the Pi delivery question before claiming the
fleet-wide contract is implemented.

### R2. Use the existing prompt pipeline

Editor output must continue to become
[PromptSource::Inline](../../cli/src/commands/wrap/profile/mod.rs) in
`claudine-cli`, the wrapper's typed form of a supplied prompt. Session-mode
selection must then determine how the selected provider profile delivers that
prompt, exactly as it does for a directly supplied initial prompt.

Do not introduce a second editor-to-provider delivery path or emulate an
initial turn through terminal keystrokes. The editor remains a pre-launch
authoring step; provider-specific delivery remains owned by
[WrapperProfile::prompt_delivery](../../cli/src/commands/wrap/profile/mod.rs)
in `claudine-cli` and the established launch pipeline. If a
profile's existing interactive delivery is defective, repair that profile for
all interactive startup prompts, whether edited or supplied on the command
line.

Seed prompts must work in either ordering already supported by wrapper flag
extraction:

```sh
claudine codex --edit --interactive "review this repository"
claudine codex "review this repository" --edit --interactive
```

Arguments after the explicit `--` separator remain opaque provider arguments.
A provider-owned `--edit` after that separator must not activate Claudine's
editor workflow.

### R3. Preserve editor preconditions and cancellation

`--edit` must continue to require terminal input and output regardless of the
provider session mode. Piped stdin, redirected stdout, or another non-TTY
launch must fail before opening the editor with the existing diagnostic.

Preserve all other editor behavior:

- `$EDITOR`, then `$VISUAL`, then installed-editor fallback resolution;
- Markdown temporary-buffer creation and GUI-editor wait flags;
- optional command-line seed text;
- trimming trailing whitespace;
- clean exit without launching a provider when the edited buffer is empty;
- typed failures for editor launch, exit, I/O, or deleted-buffer errors; and
- deletion of the temporary buffer after the handoff.

An interactive provider session must not start when editing is cancelled or
fails.

The wrapper currently resolves the provider executable before opening the
editor. This fix does not change that preflight order for live runs: a missing
provider can fail before editing begins. `--dry-run` continues to avoid
executable resolution.

### R4. Preserve unrelated mode constraints

This fix removes only the `--edit`/`--interactive` conflict. Existing timeout
rules remain unchanged: wall-clock and step-silence timeouts that are invalid
for interactive provider sessions must still be rejected.

Validate an explicit interactive request combined with either timeout after
wrapper flag extraction and before opening the editor. The current timeout
check runs after editing, so simply deleting the edit conflict would make a
user edit a prompt before receiving a predictable flag error. Keep the
post-edit validation needed for mode decisions that depend on whether the
edited prompt is empty.

Editor time remains outside provider execution timing. `--quiet`, `--silent`,
`--dry-run`, model selection, system-prompt options, MCP composition, sandbox
selection, YOLO intent, repository mode, and operation labeling retain their
current behavior unless a test demonstrates that the stale conflict prevented
their already-defined composition with `--edit --interactive`.

For `--dry-run --edit --interactive`, Claudine must run the editor because the
edited prompt is an input to the preview, render an interactive launch plan,
and not require or launch the provider executable.

### R5. Correct the behavior contract and remove stale rationale

Update user-facing help and Claudine documentation wherever they describe
`--edit` as non-interactive-only or repeat the unsupported provider limitation.
Correct the getting-started page's plain `--edit` example to say it starts a
non-interactive session after a non-empty edit. Keep its `--edit -i` example
and make the implementation conform to it. Clarify where useful that
"interactive terminal" in the editor diagnostic refers to editor I/O, not
provider session mode.

Historical completed specs remain historical records and need not be rewritten.
Current reference documentation and the Claudine skill snapshots must describe
the corrected behavior.

Any comment changed near the affected validation or prompt pipeline must be
reviewed against the resulting behavior. Delete commentary that exists only to
justify the false conflict rather than replacing it with implementation
narration.

## Acceptance Criteria and Regression Coverage

1. Replace the wrapper integration test that expects `--edit` and
   `--interactive` to conflict with a terminal-backed test proving that an
   editor result is delivered as the initial prompt and the provider is launched
   in interactive mode. Use the repository's terminal test harness with a
   background or hidden session; L2 and L3 tests must not take focus.
2. Exercise both `--interactive` and `-i`, including a wrapper-owned flag found
   in the passthrough bucket after a positional seed prompt. Neither parsing
   route may retain the conflict.
3. Cover an empty editor buffer in interactive mode. Claudine exits successfully
   and the provider is not launched.
4. Cover editor failure in interactive mode. The typed editor diagnostic is
   preserved and the provider is not launched.
5. Cover non-TTY rejection with `--edit --interactive`. The editor and provider
   are both not launched, and the diagnostic remains about the terminal
   requirement rather than a flag conflict.
6. Cover `--dry-run --edit --interactive` with a fake editor. The preview
   contains the edited prompt, reports interactive mode, and does not require
   the provider binary.
7. Retain or extend profile-level tests so every wrapped provider has evidence
   that a supplied prompt can be delivered in interactive mode. The fleet
   assertion must cover all variants in the compiled `Provider` inventory so a
   newly added provider cannot silently restore the false assumption.
8. Include at least one multiline Markdown prompt beginning with `-` to retain
   coverage of provider-specific option-separator or attached-value handling in
   interactive mode.
9. Confirm plain `--edit` still defaults a non-empty edited prompt to
   non-interactive execution and that plain `--interactive` with a direct
   command-line prompt is unchanged.
10. Update current documentation and generated/help snapshots as required, and
    add a drift assertion or focused test for the advertised
    `claudine codex --edit -i` form.
11. Prove `--interactive --timeout` and `--interactive --step-timeout` fail
    before the editor starts, including a flag recovered after a positional
    seed prompt.
12. Verify Pi's interactive startup prompt with evidence beyond generated
    arguments. If its stdin delivery does not submit the first turn while
    preserving the terminal session, repair the Pi profile and cover both
    edited and direct startup prompts before closing this fix.

## Open Questions

### How should Pi receive an interactive startup prompt?

The [Pi wrapper profile](../../cli/src/commands/wrap/profile/pi.rs) in
`claudine-cli` currently pipes every supplied prompt to stdin, regardless of
session mode. That profile's comment and the repository's Pi research describe
stdin as a non-interactive print-mode channel. It is unclear whether the same
channel submits the first turn to Pi's interactive terminal interface and
leaves that interface usable. A unit test of the generated delivery shape
cannot settle this behavior.

1. **Recommended: verify Pi's native interactive startup contract and use it
   inside the existing profile.** Pros: preserves a single editor pipeline and
   makes direct `claudine pi "prompt" --interactive` work by the same route. Cons: may
   require a Pi-specific profile correction and a terminal-backed test.
2. **Keep stdin delivery after verifying it in a real terminal.** Pros: no
   profile change if Pi already consumes stdin as intended. Cons: the current
   print-mode evidence does not establish interactive behavior, so this choice
   needs a direct proof and a regression test.
3. **Exclude Pi from the all-provider promise.** Pros: avoids changing a
   provider profile if Pi has no suitable launch surface. Cons: weakens the
   intended consistent wrapper behavior and requires a user-facing exception.

Choose the first option if Pi's native launch path supports an interactive
startup prompt: it fixes the underlying direct-prompt behavior rather than
adding an editor-only workaround. If investigation shows that Pi has no such
path, revisit the fleet-wide promise before implementation rather than
silently falling back to a one-shot run.

## Non-Goals

- Adding `--edit` to `compose`, `inline-compose`, or `sequence`.
- Changing how provider profiles encode or transport an interactive initial
  prompt.
- Persisting editor buffers or adding prompt templates.
- Relaxing the TTY requirement for an external editor.
- Changing provider permission, approval, sandbox, resume, or system-prompt
  semantics.
- Reinterpreting provider arguments after the explicit `--` separator.

## Success Condition

`--edit` is solely an initial-prompt authoring mechanism. After the editor
returns a non-empty prompt, Claudine treats it identically to the same prompt
written on the command line. `--interactive` independently selects an
interactive provider session, for every supported provider, and the edited
prompt becomes that session's initial user turn.
