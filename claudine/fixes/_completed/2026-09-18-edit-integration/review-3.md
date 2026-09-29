---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-18-edit-integration/spec.md`
fix: 2026-09-18-edit-integration/review-3.md
spec: 2026-09-18-edit-integration/spec.md
previous: 2026-09-18-edit-integration/review-2.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T18:05:47-07:00
implemented: false
ready: true
recurrence: true
human_review: true
human_review_items:
    - |-
      Provide an unattended test environment with an installed, authenticated Goose CLI, then run both `real_native_interactive_startup::goose` cases through `just test-real` in `claudine/`. The tests launch a hidden terminal, send a first message directly or through the editor, and check that Goose answers a second message in the same session. Please share the test result and captured terminal text if either case fails. This is the only native provider whose two-turn behavior has not been observed.

      DECISION: this will not be a blocking issue for this spec
has_blocked_findings: true
blocked: true
findings:
    - "[high] Goose interactive startup still lacks native two-turn verification"
---

# Review 3: Edit Integration

## Verdict

**Not ready for production.** The separator fix and current references address the two medium findings in review 2. Native tests now demonstrate first and second turns for seven more providers; Pi already had that evidence. Goose's test exists but skipped because Goose was unavailable, leaving the required interactive behavior unverified. The author decided in review 2 that Kimi's known one-turn behavior waits on updated provider research and does not hold up this fix. This review honors that decision; it does not ask for it again.

The change does not read a new file format or configuration field, so the input robustness matrix does not apply. Cross-OS CI evidence is outside this readiness verdict.

## Blocked Findings

### [high] Goose interactive startup still lacks native two-turn verification

**Defect class:** A generated interactive command and a test that skips when the native provider is unavailable cannot establish that the provider submits the first message and remains open for another turn.

The `claudine-cli` [Goose profile](../../cli/src/commands/wrap/profile/goose.rs) builds `run --text <prompt> --interactive`. Its argument-shape test passes, and the new [native test](../../cli/tests/real/real_native_interactive_startup.rs) covers direct and edited prompts in a detached tmux session. The implementation log records that both Goose cases skipped because Goose was not installed. **Blocked by the lack of an installed, authenticated Goose in an unattended test environment.** Once one is available, run both cases and repair the profile if either first-turn submission or the second turn fails. A passing test count that includes skips is insufficient here.

The same reproduction was applied to every provider site: launch a typed first message and an editor-authored Markdown bullet with `-i` in a detached real terminal, observe the first answer, submit a second message, and observe its answer in the same session. The table records the implementation log's native results; this review reran the focused wrapper checks, not credential-dependent native sessions.

| Provider site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Claude Code | Direct and edited first message, then second turn | Native 2.1.284: both pass | Two turns in one session |
| Codex | Same | Native 0.157.1: both pass | Same |
| Gemini CLI | Same | Native 0.61.0: both pass | Same |
| Goose | Same tests declared | Both skipped; binary unavailable | Both native cases pass |
| Kimi Code | Direct and edited first message | Known `--prompt` one-turn behavior; explicitly deferred by author | Follow the author's research decision outside this readiness gate |
| OpenCode | Direct and edited first message, then second turn | Native 1.18.33: both pass in the final four runs; an earlier edited run timed out without a retained pane | Two turns in one session; retain a pane if the timeout recurs |
| Qwen Code | Direct and edited first message, then second turn | Native 0.19.8: both pass | Two turns in one session |
| Kilo | Same | Native 7.3.45: both pass | Same |
| Pi | Direct, edited, long, and `@`-prefixed first messages, then second turn | Native 0.87.1 regression passes | Same |
| Antigravity | Direct and edited first message, then second turn | Native 1.2.12: both pass | Two turns in one session |

## Earlier findings and class sweep

| Earlier finding | Sibling sites checked with the same shape | Result |
| --- | --- | --- |
| Review 2: provider first-turn delivery | All ten provider profiles and the native test for each available provider | Seven new native providers and Pi are clean; Goose remains unverified; Kimi is deferred by the author's decision |
| Review 2: user `--` forwarded to the provider | All ten profiles with `hello -- --offline`, including direct and editor-authored prompts; Claude, Codex, and Pi with a dash-prefixed prompt that needs their own separator | Clean: the wrapper consumes its separator; provider options precede any profile-added separator. The fleet test and shipped CLI dry-run test pass |
| Review 2: editor reference drift | Getting-started guide, system-prompt topic, CLI README, Claudine skill overview, CLI reference, and timeline | Clean for each page's scope; the skill reference now explains editor selection, terminal requirement, cancellation, timeout order, and session mode |

The first two review iterations' wrapper acceptance tests remain in place: Level 2 detached-terminal tests cover editor handoff, cancellation, dry run, and Markdown prompt transport; Level 1 spawned-CLI tests cover non-TTY and timeout rejection. The native real-provider tests supply the higher verification level needed to show that a first message actually becomes a turn and the session continues. Goose is the sole missing native result. The native test module is declared by `claudine-cli`'s consolidated `real` target, selected by the live `test-real` recipe, and guarded so its tmux session remains detached. `just check-tier-coverage claudine` reported zero stranded tests. Focused Level 1 tests for all-provider separator placement and shipped-CLI dry-run forwarding passed.

## Recurrence

This finding repeats **review-1.md, “Interactive startup is broken for Kimi and unverified against most native providers,”** and **review-2.md, “Interactive first-turn delivery remains broken for Kimi and unverified with eight native providers.”** The earlier fix needed to sweep all ten native provider sites. It verified Pi, then seven more providers, but left Goose with a test that skips rather than a native result. The table above completes the site inventory for this review. Set `recurrence: true` so a human can inspect the repeated class before another automatic review/fix cycle.
