---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-18-edit-integration/spec.md`
fix: 2026-09-18-edit-integration/review-2.md
spec: 2026-09-18-edit-integration/spec.md
previous: 2026-09-18-edit-integration/review-1.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T17:05:00-07:00
log: claudine/fixes/2026-09-18-edit-integration/log.md
implemented: true
next: 2026-09-18-edit-integration/review-3.md
implemented_by: claude/opus
ready: false
recurrence: true
human_review: true
human_review_items:
    - |-
     **DECISION:** KIMI is not working until we update the research and get caught up to latest version. This WILL work but we are dependent on this research -> gen task. This should not prevent this spec from being marked as production ready.
    
      Decide the behavior when someone gives Kimi Code a first message and asks to remain interactive. Kimi's `--prompt` option runs one turn and exits, so Claudine currently accepts `claudine kimi --edit -i` but does not provide the requested session. Choose one contract:

      - **Refuse a first message with `-i` until Kimi supports it (recommended).** Explain the limitation before launching Kimi; amend the specification's every-provider promise.
      - **Keep the one-turn behavior.** Amend the specification to say that `-i` does not keep Kimi interactive, accepting a misleading flag.
      - **Run once and resume in a second process.** This may give a second turn, but session selection can race and the specification currently requires one launch path.

      The implementation team needs this decision for both edited and direct first messages. The other findings can be repaired independently.
has_blocked_findings: true
blocked: false
findings:
    - "[high, closed] Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers"
    - "[medium] The wrapper forwards its argument separator into provider commands"
    - "[medium] Current Claudine references still omit the editor contract"
---

# Review 2: Edit Integration

## Verdict

**Not ready for production.** The editor handoff, cancellation, timeout ordering, dry run, and tested argument shapes work. The first review's Kimi and native-provider finding remains open, as does its reference-documentation finding. A shared argument-separator defect also changes provider options into positional input. No implementation change followed review 1; the intervening commit recorded that review.

This fix changes no file-format or configuration reader, so the input robustness matrix does not apply. Cross-OS results are outside this readiness verdict.

## Unblocked Findings

### [high] Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers

**Defect class:** A generated interactive command does not establish that the native provider submits the first message and stays open for a second one.

**Partially blocked:** Kimi's final contract needs the human choice in the frontmatter. Verification of the other eight native providers is unblocked.

In `claudine-cli`, [KimiWrapper::prompt_delivery](../../cli/src/commands/wrap/profile/kimi.rs) still supplies `--prompt` in interactive mode. Kimi treats that as a one-turn run. The [fleet test](../../cli/src/commands/wrap/profile/tests/positional.rs) explicitly pins this known failing shape. Its other rows inspect arguments only. The detached [Codex/Pi capture tests](../../cli/tests/level2/level2_edit_interactive_capture.rs) exercise fake providers; they prove the wrapper preserves prompt bytes and the terminal, not native provider behavior. The [real Pi regression](../../cli/tests/real/real_pi_interactive_startup.rs) alone observes the first and second turns of a native provider. The first review already identified every provider profile in this class; the same fleet fixture was run again and all ten rows were inspected.

| Provider site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Claude Code | Direct first message and Markdown bullet in fleet fixture | Arguments only; no native second-turn test | Native first and second turns verified |
| Codex | Fleet fixture; edited prompt through detached fake-provider terminal | Fake receives prompt and terminal; native second turn unverified | Native first and second turns verified |
| Gemini CLI | Both fleet prompt shapes | Arguments only | Native first and second turns verified |
| Goose | Both fleet prompt shapes | Arguments only | Native first and second turns verified |
| Kimi Code | Both fleet shapes; direct and edited `-i` route to `--prompt` | Known one-turn behavior; **broken** | Chosen truthful contract, tested before launch or through two native turns |
| OpenCode | Both fleet prompt shapes | Arguments only | Native first and second turns verified |
| Qwen Code | Both fleet prompt shapes | Arguments only | Native first and second turns verified |
| Kilo | Both fleet prompt shapes | Arguments only | Native first and second turns verified |
| Pi | Direct, edited, long, and `@`-prefixed first messages | Native Pi 0.87.1 test recorded first and second turns; **clean** | Retain the native regression |
| Antigravity | Both fleet prompt shapes | Arguments only | Native first and second turns verified |


### [medium] The wrapper forwards its argument separator into provider commands

**Defect class:** The wrapper keeps its own `--` boundary in child arguments, where a provider interprets it as its own end-of-options marker.

The [flag extractor](../../cli/src/commands/wrap/flags.rs) protects arguments after `--` from Claudine's flag handling, but leaves the marker in `child_args`. The shipped CLI reproduction `claudine <provider> --dry-run -i hello -- --offline` shows `-- --offline` at the beginning of every provider command. The local dry-run preview does not include the final prompt-delivery arguments, so the Pi native result below comes from the recorded live Pi 0.87.1 reproduction in the [follow-up record](../_unscheduled/wrapper-user-separator-forwarded/spec.md). The same wrapper boundary handles every provider; the table sweeps all ten sites rather than treating Pi as the only affected one.

| Provider site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Claude Code | `-i hello -- --offline` | Dry run begins `claude -- --offline`; dash-prefixed first messages can add another `--` | Provider receives `--offline` as an option before any prompt separator |
| Codex | Same | Begins `codex -- --offline`; dash-prefixed first messages can add another `--` | Same |
| Gemini CLI | Same | Begins `gemini -- --offline` | Same |
| Goose | Same | Begins `goose -- --offline` | Same |
| Kimi Code | Same | Begins `kimi -- --offline` | Same |
| OpenCode | Same | Begins `opencode -- --offline` | Same |
| Qwen Code | Same | Begins `qwen -- --offline` | Same |
| Kilo | Same | Begins `kilo -- --offline`; dash-prefixed first messages can add another `--` | Same |
| Pi | Same; plus live `claudine pi "hello" -i -- --offline` | Begins `pi -- --offline`; live Pi reads `--offline` as message text, and prompt delivery adds another `--` | Pi receives `--offline` as an option, then one separator and the first message |
| Antigravity | Same | Begins `agy -- --offline` | Same |

Consume the user's separator as a Claudine boundary while keeping its trailing arguments opaque to Claudine. Then test both ordinary and dash-prefixed first messages across the profiles that add a prompt separator. This defect predates the fix, but the specification explicitly preserves arguments after `--` as provider arguments, and edited interactive startup must work alongside those arguments. The existing unscheduled follow-up documents the defect; it does not satisfy this contract.

### [medium] Current Claudine references still omit the editor contract

**Defect class:** Current references for the same wrapper command disagree or leave out the editor's preconditions and session-mode behavior.

The [getting-started guide](../../docs/getting-started/index.md) describes the implemented plain and interactive forms, cancellation, terminal requirement, and timeout ordering. The [current topic page](../../docs/topics/system-prompt.md) mentions that a direct wrapper can edit its user prompt; it does not attempt to teach wrapper operation. The three agent-skill reference sites below still lack the behavior needed by an agent or developer using the CLI. These are the same sibling sites identified in review 1.

| Reference site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Getting started | Plain `--edit`, `--edit -i`, timeout and cancellation notes | Describes current behavior, including Kimi's known exception; **clean after the Kimi decision is reflected** | Keep in sync with the chosen contract |
| System-prompt topic | Scope of `--edit` relative to system prompts | Correctly says it edits the user prompt; **clean for this page's purpose** | Keep the distinction |
| Claudine skill CLI reference | Shared wrapper flag table and behavior | No `--edit` entry or editor workflow | State editor selection, terminal requirement, empty-buffer cancellation, timeout order, and session-mode choice |
| Claudine skill overview | Direct-wrapper summary | Mentions optional editing but not its relationship to `-i` | Summarize the rule and point to the CLI reference |
| Claudine skill timeline | Recent wrapper behavior | No edit-integration entry | Record the behavior change and Pi delivery correction |

Apply the prepared skill text in the [implementation log](implementation-log.md), revising the Kimi wording after the contract decision. The first implementation log says an earlier agent could not write the skill files; the current workspace permits these documentation edits. They do not require another human decision.

## Blocked Findings

The Kimi portion of the first finding remains blocked by the same human contract choice recorded in review 1 and the spec. No later decision or implementation unblocked it. The other portions of that finding and both medium findings are unblocked.

## Recurrence

- The first finding repeats **review-1.md, “Interactive startup is broken for Kimi and unverified against most native providers.”** That fix needed to sweep all ten provider profiles through native first-turn and second-turn behavior. It did so for Pi only; the eight providers listed as argument-only above and Kimi's failing branch remain.
- The documentation finding repeats **review-1.md, “Current Claudine reference material omits the editor contract.”** That fix needed to sweep the getting-started page, the Claudine skill CLI reference, overview, and timeline. Getting started was updated, but the three skill sites remain incomplete. The system-prompt topic was also checked and is clean for its stated scope.

Because these are repeated defect classes, `recurrence: true` stops an automatic review/fix loop for human inspection before another cycle.

## Verification and requirement levels

| User-visible requirement | Strongest evidence | Assessment |
| --- | --- | --- |
| Edit then launch interactively; both flag positions and both spellings | Level 2 detached tmux with fake Codex | Wrapper handoff verified; native-provider result covered by first finding |
| Empty edit, editor failure, and dry run without a provider binary | Level 2 detached tmux | Verified at the appropriate level for editor and wrapper behavior |
| Non-TTY rejection and timeout rejection before editor launch | Level 1 spawned CLI with marker executables | Verified; no terminal encoder behavior is involved |
| Plain edit defaults to one-shot; direct prompt with `-i` remains interactive | Level 2 fake Codex for plain edit; Level 1 spawned CLI for direct prompt | Wrapper behavior verified; native second turn still needs evidence |
| Multiline Markdown prompt beginning with `-` | Level 2 fake Codex; Level 1 fleet table | Argument preservation verified; native interpretation remains part of the first finding |
| Pi edited and direct first message, followed by a second turn | Opt-in real Pi terminal test, recorded passing on 0.87.1 | Verified for Pi; this review did not rerun the credential-dependent suite |
| Provider options after Claudine's explicit `--` | Shipped CLI dry run for all ten; recorded native Pi run | Fails as described in the second finding |

`just check-tier-coverage claudine` passed with zero stranded tests. `just test-cli every_provider_delivers_an_interactive_startup_prompt` passed one Level 1 test. `just test-l2 edit_interactive_capture` passed seven detached-terminal tests. The Level 2 test file is declared by its consolidated binary and selected by a live recipe; the real Pi file is declared by the real-test binary and selected by `test-real`. I found no justified performance or ergonomic change beyond fixing the command contract and its references.
