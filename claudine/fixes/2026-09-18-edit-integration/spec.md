---
created: 2026-09-18
status: proposed
reviewed: false
implemented: false
area: claudine
packages:
    - claudine-cli
---

# Compose `--edit` with Interactive Provider Sessions

## Problem

Claudine rejects `--edit` when it is combined with `--interactive` (`-i`).
That restriction incorrectly couples two independent choices:

- `--edit` selects how the initial user prompt is authored; and
- `--interactive` selects the provider session mode after that prompt is
  authored.

Every provider wrapped by Claudine can accept an initial prompt when launched
interactively. Claudine already models and implements that capability through
the ordinary command-line form:

```sh
claudine <provider> "initial prompt" --interactive
```

An editor-authored prompt becomes the same `PromptSource::Inline` value as an
initial prompt supplied directly on the command line. Preventing that value
from continuing through the existing interactive prompt-delivery path is
therefore a wrapper validation defect, not a provider limitation.

The current behavior also contradicts the getting-started documentation, which
advertises `claudine codex --edit -i` as the interactive variant.

## Current Behavior and Root Cause

The restriction is enforced in two places:

1. `WrapperArgs::edit` declares `conflicts_with = "interactive"`, so clap
   rejects the ordinary argument ordering.
2. `validate_timeout_constraints` independently rejects the combination after
   wrapper flags have been recovered from the passthrough argument bucket.

The duplicate runtime check exists because wrapper-owned boolean flags can be
recognized on either side of the first positional argument. Both checks encode
the same invalid policy and must be removed together.

The original edit-command design justified the conflict by claiming that most
provider CLIs could not seed an initial turn into an interactive session. That
claim is false for Claudine's entire supported provider fleet. Current provider
profiles already distinguish interactive from non-interactive prompt delivery,
and their tests cover interactive delivery shapes such as positional prompts,
provider prompt flags, end-of-options separation, and provider-specific
argument placement.

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

This contract applies without exception to Claude Code, Codex, Gemini CLI,
Goose, Kimi Code, OpenCode, Qwen Code, Kilo, Pi, and Antigravity. Do not add a
provider capability flag, allowlist, fallback to non-interactive execution, or
warning for this combination.

### R2. Use the existing prompt pipeline

Editor output must continue to become `PromptSource::Inline`. Session-mode
selection must then determine how the selected provider profile delivers that
prompt, exactly as it does for a directly supplied initial prompt.

Do not introduce a second editor-to-provider delivery path, emulate an initial
turn through terminal keystrokes, or special-case individual providers. The
editor remains a pre-launch authoring step; provider-specific delivery remains
owned by `WrapperProfile::prompt_delivery` and the established launch pipeline.

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

### R4. Preserve unrelated mode constraints

This fix removes only the `--edit`/`--interactive` conflict. Existing timeout
rules remain unchanged: wall-clock and step-silence timeouts that are invalid
for interactive provider sessions must still be rejected.

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
`--edit` as non-interactive-only or repeat the false provider limitation. Keep
the getting-started `--edit -i` example and make the implementation conform to
it. Clarify where useful that "interactive terminal" in the editor diagnostic
refers to editor I/O, not provider session mode.

Historical completed specs remain historical records and need not be rewritten.
Current reference documentation and the Claudine skill snapshots must describe
the corrected behavior.

Any comment changed near the affected validation or prompt pipeline must be
reviewed against the resulting behavior. Delete commentary that exists only to
justify the false conflict rather than replacing it with implementation
narration.

## Acceptance Criteria and Regression Coverage

1. Replace the wrapper integration test that expects `--edit` and
   `--interactive` to conflict with a real-terminal test proving that an editor
   result is delivered as the initial prompt and the provider is launched in
   interactive mode.
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
