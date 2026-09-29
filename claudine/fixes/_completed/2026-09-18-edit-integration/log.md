---
review: claudine/fixes/2026-09-18-edit-integration/review-1.md
spec: claudine/fixes/2026-09-18-edit-integration/spec.md
deferred_perf_measurement: false
implementation_2: "2026-09-28T17:14:41-07:00"
---

# Edit Integration: Review-to-Implement Log

## Implementation of Review Findings #1

> **started at:** 2026-09-28T16:54:05-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-09-18-edit-integration/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on '[high] Interactive startup is broken for Kimi and unverified against most native providers' at 16:54:40

## Implementation of Review Findings #2

> **started at:** 2026-09-28T17:14:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-09-18-edit-integration/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on '[high] Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers' at 17:15:03
        - defect class: a generated interactive command is not verified to make the native provider submit the first message and stay open for a second one
        - added one table-driven opt-in `real_` module that runs `claudine <provider> "<prompt>" -i` and `claudine <provider> --edit -i` (fake editor writing a Markdown bullet prompt, so the `-`-prefixed delivery path runs too) in a detached tmux session, waits for the model's answer to the first turn, types a second turn, waits for its answer, and quits with Ctrl+C; one direct and one edited test per provider; skips unless `CLAUDINE_CONTRACT_REAL=1`, the provider binary is on `PATH`, and tmux is available
        - discoveries
                - answers are matched as a whole word on a line that does not echo a user turn; a bare token match is not enough because the TUIs echo the prompt, Qwen draws a ghost suggestion ("Concatenate ALPHA and BRAVO") in its input box, and Kilo draws a sidebar on the answer's line
                - several providers draw a start-up screen before or after the first turn; the test answers each known one once: Claude Code and Codex folder trust, Antigravity project trust, Gemini folder trust (answered "Don't trust", which persists nothing; Gemini draws it after already answering the first turn), and Codex 0.157's "Cannot use the background server" dialog, which also appears for a bare `codex` launch on this host and is unrelated to prompt delivery
                - each test uses one stable working directory under the system temp directory so a provider that persists a trust answer records it once, not per run
                - the default 30s nextest termination ceiling cut the first Codex run short; added a `.config/nextest.toml` override for this module sized to its own 330s of deadlines
        - native results on this host (macOS), each run through `just test-real real_native_interactive_startup::<provider>`
                - Claude Code 2.1.284: pass, direct and edited, first and second turns
                - Codex 0.157.1: pass, direct and edited, first and second turns (after the test learned the Codex trust dialog)
                - Gemini CLI 0.61.0: pass, direct and edited, first and second turns
                - OpenCode 1.18.33: pass, direct and edited, first and second turns, in four consecutive runs; the first run's edited test timed out waiting for the first answer and its pane was not retained, so that one failure is unexplained
                - Qwen Code 0.19.8: pass, direct and edited, first and second turns
                - Kilo 7.3.45: pass, direct and edited, first and second turns
                - Antigravity 1.2.12: pass, direct and edited, first and second turns
                - Pi 0.87.1: covered by the existing offline Pi regression; not changed
                - Goose: unverified: `goose` is not installed on this host; both tests skip with that reason
                - Kimi Code: deferred by the author's decision in review 2; profile and fleet row unchanged
        - no provider profile needed a repair; the fleet table's rows are unchanged and its doc comment now names the native tests that back it
        - class sweep: native interactive first-turn and second-turn delivery; sites checked: Claude Code, Codex, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, Kilo, Pi, Antigravity; fixed here: none (no profile was broken; native verification added for Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Kilo, Antigravity, and written but unverified for Goose); clean: Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Kilo, Antigravity, Pi (Kimi Code deferred by decision; Goose unverified, binary not installed)
        - Goose native verification is **deferred**: the test is written and selected by `test-real`, but no host here has `goose` installed and authenticated
- work completed for '[high] Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers' at 17:53:11
- starting the work on '[medium] The wrapper forwards its argument separator into provider commands' at 17:53:11
        - defect class: the wrapper kept its own `--` boundary in the child arguments, where the provider read it as its own end-of-options marker and took the options after it as messages
        - discovery: the defect only appeared when clap kept the `--` in the passthrough, which happens when a positional prompt (or an unknown provider flag) precedes it; when the `--` came before every positional, clap already consumed it and the tail reached the provider as options, so the two spellings behaved differently
        - discovery: the flag extractor located the boundary from the passthrough first, so in `claudine pi -- hello -- --offline` it would have treated the provider's second `--` as Claudine's; the boundary is now located from the raw process arguments (the first raw `--`), and only that token is dropped; a later `--` stays in the tail as the provider's own
        - discovery: prompt extraction still searches the tail for the first message when none precedes the `--` (existing behavior that several spawned tests rely on, e.g. `claudine codex -- --json "summarize repo"`); only wrapper-flag recovery is suppressed after the boundary. A dash-prefixed first message on the command line now needs a second, provider-level `--` (`claudine opencode -- -- "- fix the bug"`); the single-`--` form only worked before when an unknown provider flag happened to precede the `--`
        - the profiles that add their own `--` before an interactive dash-prefixed first message are Claude, Codex, and Pi (Kilo and OpenCode attach it as `--prompt=…` interactively and add `--` only in non-interactive `run`); with the tail forwarded, each now ends `… --offline -- "- item"` with exactly one `--`
        - updated the existing tests that pinned the forwarded `--`: the flag-extractor unit tests now expect the tail without it, and `direct_wrap_opencode_argv` passes its dash-prefixed prompt after a second `--`
        - added tests: `every_provider_receives_options_after_the_user_separator_as_options` and `profile_separator_before_a_dash_prompt_is_the_only_separator` (all ten providers from `PROVIDERS_DISPLAY_ORDER`, interactive and non-interactive, positional and edited first messages); extractor tests for `--edit` after `--`, a second provider `--`, and a consumed-then-literal `--`; spawned L1 `wrapper_dry_run_forwards_the_tail_after_the_user_separator_without_it` over all ten providers
        - docs: the `-- ...` row in the CLI README and the Claudine skill CLI reference, and the separator rule in the CLI pre-parsing topic page, now describe the consumed separator
        - dry-run check with the built binary, `claudine <provider> --dry-run -i hello -- --offline` for all ten providers: every command line is `<binary> --offline` (Antigravity: `agy --offline`); no live Pi run, since an interactive launch cannot run in this non-interactive session
        - this change resolves the unscheduled follow-up `wrapper-user-separator-forwarded`; its spec is left in place for the author to move
        - class sweep: wrapper `--` boundary kept in child argv; sites checked: direct wrapper flag extractor and boundary finder, direct wrapper prompt extraction (`--` before a positional prompt), per-profile separator insertion (Claude/Codex/Pi interactive, OpenCode/Kilo non-interactive, shared stdin-or-append helper), native-output detection before `--`, the after-`--` flag warning, `argv::normalize` rules 1/2/4, composition tail partition (compose/inline-compose/sequence), composition forwarded-tail notice, `--perf` bootstrap scan, shell-completion `--` handling, resume argv builders, dry-run command rendering, recorded `AGENT_PARAMS`; fixed here: direct wrapper flag extractor and boundary finder (dry-run rendering and launch argv follow from it); clean: prompt extraction, per-profile separator insertion, native-output detection, after-`--` warning, `argv::normalize`, composition tail partition (already consumes its `--`), forwarded-tail notice, `--perf` scan, completion, resume builders (no user args), recorded `AGENT_PARAMS` (records what the user typed, not child argv)
- work completed for '[medium] The wrapper forwards its argument separator into provider commands' at 18:02:50
- starting the work on '[medium] Current Claudine references still omit the editor contract' at 18:02:50
        - defect class: current references for the same wrapper command disagreed or left out the editor's preconditions (editor selection, terminal requirement, empty-buffer cancellation, timeout rejection before the editor) and its session-mode behavior (plain `--edit` non-interactive, `--edit -i` interactive)
        - applied the prepared Phase 4/5 skill text, with the Kimi wording revised to the author's review-2 decision: Kimi Code's interactive startup prompt currently goes through `--prompt`, runs one turn and exits, and a fix is pending updated Kimi research
        - Claudine skill CLI reference: added the `--edit` row to the Shared Wrapper Flags table and the session-mode rule (with per-provider native delivery examples and the Kimi exception) to the "Interactivity default" bullet; the `-- ...` row updated earlier this cycle is kept
        - Claudine skill timeline: added an `edit-integration` entry (flag conflict removed, timeout rejection before the editor, interactive startup delivery repairs, native first/second-turn real tests for Claude, Codex, Gemini, OpenCode, Qwen, Kilo, Antigravity, Goose written but unverified, Pi via its own real test, Kimi deferred) and a sibling `edit-integration (wrapper --)` entry for the consumed user separator and the second `--` a dash-prefixed first message now needs
        - Claudine skill overview: added the Phase 4 spawn-guard sentence (`real_` files exempt from the emulator-session naming rule) to the L1 spawn contract paragraph, and summarized the `--edit`/`-i` rule and the `--` boundary in the Wrapped Execution row with a link to the CLI reference
        - CLI README: added the same `--edit` row and session-mode sentence (it carried the same flag table and "Interactivity default" bullet as the skill, without either)
        - getting-started guide: reworded the Kimi bullet, which said Kimi has no option to start interactively with a first message; it now states the observed one-turn behavior and that the fix waits on updated Kimi research (no fix named)
        - verified the wording against code: `darkmatter::editor::resolve_editor_command` ($EDITOR → $VISUAL → installed editor), `prompt_source.rs` terminal error text, `wrapper_stages::reject_interactive_timeouts`, the profile delivery options, and the spawn-site guard's `real_` exemption; the `--edit` help string in `flags.rs` is consistent and was not changed
        - ran the drift test that reads the getting-started page: `just test getting_started_edit_interactive_form_is_accepted` passed
        - class sweep: wrapper `--edit`/`-i` reference drift; sites checked: Claudine skill CLI reference, Claudine skill overview, Claudine skill timeline, Claudine skill architecture, Claudine skill supported platforms, CLI README, package README, getting-started guide, system-prompt topic, CLI pre-parsing topic, wrapped-execution-switches topic, `docs/cli/` pages (about, budget, sequence), `--edit` help string in `flags.rs`; fixed here: Claudine skill CLI reference, Claudine skill overview, Claudine skill timeline, CLI README, getting-started guide (Kimi bullet); clean: Claudine skill architecture, Claudine skill supported platforms, package README, system-prompt topic, CLI pre-parsing topic, wrapped-execution-switches topic, `docs/cli/` pages, `--edit` help string
        - recomputed the skill timeline's `hash:` frontmatter with `md hash --save` after its body changed
- work completed for '[medium] Current Claudine references still omit the editor contract' at 18:05:18

### Successful Completion

The implementation of review cycle 2 has completed successfully in 51 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was partially deferred (see reasons below):

- **[high] Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers**: partially deferred
        - done: native first-turn and second-turn real tests pass on macOS for Claude Code, Codex, Gemini CLI, OpenCode, Qwen Code, Kilo, and Antigravity, for both a typed and an edited first message; no profile needed a repair; Pi stays covered by its existing native regression
        - deferred (Kimi Code): by the author's decision in review 2, Kimi's interactive startup prompt stays one-turn (`--prompt`) until its research is updated; the references now say so, and this does not block production readiness
        - deferred (Goose): its native test is written and selected by `test-real`, but no available host has `goose` installed and authenticated, so it has not produced evidence
- **[medium] The wrapper forwards its argument separator into provider commands**: fixed; this also resolves the unscheduled follow-up `wrapper-user-separator-forwarded`, which is left in place for the author to close
- **[medium] Current Claudine references still omit the editor contract**: fixed

The files changed in this cycle are:

- `.claude/skills/claudine/SKILL.md`, `cli-reference.md`, `timeline.md`
- `.config/nextest.toml`
- `claudine/cli/README.md`
- `claudine/cli/src/commands/wrap/flags.rs` and `flags/tests.rs`
- `claudine/cli/src/commands/wrap/profile/tests/positional.rs`
- `claudine/cli/tests/l1/wrap_basics.rs`, `claudine/cli/tests/l1/wrap_direct_argv.rs`
- `claudine/cli/tests/real/main.rs`, `claudine/cli/tests/real/real_native_interactive_startup.rs` (new)
- `claudine/docs/getting-started/index.md`, `claudine/docs/topics/cli-pre-parsing.md`
