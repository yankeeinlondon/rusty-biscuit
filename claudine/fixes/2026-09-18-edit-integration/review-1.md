---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-18-edit-integration/spec.md`
fix: 2026-09-18-edit-integration/review-1.md
spec: 2026-09-18-edit-integration/spec.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T16:39:09-07:00
implemented: false
ready: false
human_review: true
human_review_items:
    - |-
      Decide what Claudine should do when someone supplies a first message and asks Kimi Code to remain interactive. Kimi's current `--prompt` option runs one turn and exits, so `claudine kimi --edit -i` cannot meet the specification today. Please choose one:

      - **Refuse the combination until Kimi supports it (recommended).** Show a clear error before launching Kimi. This is truthful but requires amending the specification's promise that every provider supports this form.
      - **Keep the current one-turn behavior.** Document that `-i` does not keep Kimi interactive. This preserves the current command but makes the flag misleading.
      - **Run Kimi twice, then resume interactively.** This may provide the intended experience, but session selection can race and it conflicts with the specification's single-launch prompt pipeline.

      The implementation team needs this decision to complete the Kimi branch and its regression test. The other findings can be addressed independently.
has_blocked_findings: true
blocked: false
recurrence: false
findings:
    - "[high] Interactive startup is broken for Kimi and unverified against most native providers"
    - "[medium] Current Claudine reference material omits the editor contract"
---

# Review 1: Edit Integration

## Verdict

**Not ready for production.** Claudine's editor handoff works in the tested terminal paths, including both flag positions, an empty edit, editor failure, dry run, and a Markdown prompt beginning with a dash. Pi has a real-provider test of its first and second turns. Kimi still exits after one turn despite `-i`, and the other providers' native first-turn behavior has only argument-shape evidence. The current reference material also leaves out the new editor contract.

This change does not add or alter a file-format or configuration reader, so the input robustness matrix does not apply. Cross-OS proof is outside this readiness verdict.

## Findings

### [high] Interactive startup is broken for Kimi and unverified against most native providers

**Defect class:** Claudine equates an interactive-looking provider argument list with a native provider session that submits the first message and remains open for another turn.

The `claudine-cli` [fleet assertion](../../cli/src/commands/wrap/profile/tests/positional.rs) checks the arguments from every [wrapper profile](../../cli/src/commands/wrap/profile/mod.rs), but cannot observe a provider's input handling or second turn. The [real Pi test](../../cli/tests/real/real_pi_interactive_startup.rs) does observe both turns and is clean. The [editor terminal test](../../cli/tests/level2/level2_edit_interactive_capture.rs) uses fake providers: it proves Claudine passes the edited text and a terminal through, but says nothing about how a native provider interprets them. For Kimi, the [Kimi profile](../../cli/src/commands/wrap/profile/kimi.rs) sends `--prompt`; the profile's own comment and the [getting-started page](../../docs/getting-started/index.md) say this is a one-turn mode. Thus the fleet test passes while the required user experience fails.

The same public-wrapper check was run for every row: `claudine <provider> --dry-run -i hello` exits successfully and previews an interactive launch, including Kimi. The remaining reproduction is to start each provider with a supplied prompt and `-i` in a detached real terminal, confirm the provider submits that text as turn one, submit a second turn, and confirm the same session responds. Repeat the startup with `--edit -i` using the fake editor. The table records the strongest existing result for each site; “argument only” means the native second turn was not observed. The Level 1 fleet test also checks a multiline prompt beginning with `-` for every row.

| Provider profile | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Claude Code | Direct and edited startup argument shape | Argument only | Native first and second turns verified |
| Codex | Direct argument shape and edited prompt through detached terminal | Fake provider receives prompt and TTY; native second turn unverified | Native first and second turns verified |
| Gemini CLI | Direct and edited startup argument shape | Argument only | Native first and second turns verified |
| Goose | Direct and edited startup argument shape, including dash prefix | Argument only | Native first and second turns verified |
| Kimi Code | Direct and edited prompt with `-i` | `--prompt` starts a one-turn session and exits; known failure | Interactive first and second turns, or a clearly specified refusal before launch |
| OpenCode | Direct and edited startup argument shape | Argument only | Native first and second turns verified |
| Qwen Code | Direct and edited startup argument shape | Argument only | Native first and second turns verified |
| Kilo | Direct and edited startup argument shape, including dash prefix | Argument only | Native first and second turns verified |
| Pi | Direct and edited startup prompts, long prompt, and `@` prefix | Real Pi 0.87.1 test records first and second turns; clean | Keep the real-provider regression test |
| Antigravity | Direct and edited startup argument shape | Argument only | Native first and second turns verified |

Kimi's repair is **blocked by the human decision above**: the specification forbids a provider exception or a second editor delivery path, while Kimi has no native interactive startup option. The remaining provider checks are not blocked. Add opt-in real-provider tests using a detached or hidden terminal and deterministic local model where available; each should assert the first and second turns. A generated argument list or fake provider cannot close this user-visible requirement. The strongest existing checks are Level 1 for the fleet, Level 2 for Claudine's Codex/Pi terminal handoff with fakes, and a native-provider terminal test for Pi.

### [medium] Current Claudine reference material omits the editor contract

**Defect class:** Current user and agent references do not consistently describe the behavior implemented by the edited-prompt workflow.

The [getting-started page](../../docs/getting-started/index.md) correctly distinguishes plain `--edit`, `--edit -i`, the editor's terminal precondition, timeouts, Pi's minimum version, and Kimi's present limitation. The [Claudine skill reference](../../../.claude/skills/claudine/cli-reference.md) lists shared wrapper flags but has no `--edit` row or editor behavior description; the skill's [overview](../../../.claude/skills/claudine/SKILL.md) mentions editing without saying how it composes with `-i`; its [timeline](../../../.claude/skills/claudine/timeline.md) has no entry for this changed workflow. A developer using the reference cannot learn the preconditions, mode choice, or cancellation behavior from the current record.

| Reference site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Getting started | Plain edit, `--edit -i`, timeouts, Pi, Kimi | Describes current behavior; clean, subject to the Kimi decision | Keep synchronized with the chosen Kimi contract |
| Skill CLI reference | Shared flags and wrapper behavior | Omits `--edit` and its interaction with `-i` | Explain editor selection, terminal requirement, cancellation, timeout order, and mode choice |
| Skill overview | Direct-wrapper summary | Mentions optional editing only | Link or summarize the mode rule where wrapper behavior is introduced |
| Skill timeline | Recent wrapper behavior | No edit-integration entry | Record the behavior change and provider-specific delivery repair |

The [implementation log](implementation-log.md) already contains prepared reference edits. Apply them after the Kimi decision so the CLI reference, overview, timeline, and getting-started page agree. This is documentation maintenance, not a new design choice.

## Verification performed

- `just check-tier-coverage claudine`: passed; no stranded tests. The Level 2 and real Pi files are declared in their consolidated test binaries and selected by live tier recipes.
- `just test-cli every_provider_delivers_an_interactive_startup_prompt`: one Level 1 test passed. Its Kimi row explicitly says the argument is not a verified interactive form.
- The shipped CLI returned a successful interactive dry-run preview for all ten providers with `claudine <provider> --dry-run -i hello`. This verifies wrapper mode selection, not native provider behavior.
- `just test-l2 edit_interactive_capture`: all seven detached-terminal tests passed. They use fake providers and never bring a terminal window to the foreground.
- The implementation log records four native Pi 0.87.1 cases passing. This review did not rerun that opt-in provider suite.
- Source review covered all ten profile delivery sites, the early timeout check, editor handoff, prompt extraction, the fleet assertion, current documentation, and test placement. I found no separate ergonomic or performance change justified within this fix.
