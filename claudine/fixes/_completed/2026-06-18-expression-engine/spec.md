---
created: 2026-06-18
reviewed: false
status: draft
severity: bug
area:
  - claudine
  - darkmatter
component:
  - composition
  - markdown/compose/interpolation
---

# Expression Engine Errors Must Not Leak Into Lifecycle Messages

## Problem

Running:

```bash
claudine compose prompts/suggestions.md -y \
  review="features/2026-06-18-composition-shell-error-diagnostics/review-1.md" \
  --opencode \
  --model zai-coding-plan/glm-5.2
```

sent this Discord lifecycle message:

```text
starting the implementation of the {{ parent_dir(review)) }} review suggestions (, iteration:)
```

That output is wrong in two ways:

1. The lifecycle message contains raw Darkmatter interpolation syntax.
2. The `area` and `iteration` fields are empty instead of resolving or failing
   with an author-facing diagnostic.

Lifecycle messages are user-visible side effects. If their interpolation cannot
be parsed or evaluated, Claudine must fail preparation before sending the
message. It must not deliver partially rendered template text to Discord,
Slack, TTS, stderr, or any other notification route.

## Verified Cause

The immediate trigger is authored prompt frontmatter in
`prompts/implement-suggestions.md`:

```yaml
start:
    message: "🏃 starting the _implementation_ of the `{{ parent_dir(review)) }}` review suggestions ({{area}}, iteration: {{interation}})"
```

There are two prompt defects:

- `{{ parent_dir(review)) }}` has an extra closing parenthesis.
- `{{interation}}` is misspelled; it should be `{{iteration}}` if that value is
  intended.

`parent_dir` itself is not missing. Darkmatter registers it in the expression
function catalog and runtime dispatch table:

- `darkmatter/lib/src/markdown/compose/expression/catalog.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions.rs`

The expression parser correctly rejects `parent_dir(review))` because it sees
an unmatched `)`. The regression is that this parse failure is allowed to
continue as a warning in at least one lifecycle/frontmatter path, preserving the
raw expression text until Claudine sends it as a notification.

## Root Cause

Darkmatter interpolation currently has a lenient mode controlled by
`ComposeOptions::fail_fast`. In `fail_fast = false` paths,
`interpolate_text` records parse failures as warnings and leaves the original
`{{ ... }}` span in place. That behavior is explicitly visible in
`darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`.

That leniency is unacceptable for lifecycle frontmatter:

- A syntax error can never become valid later in the same compose run.
- A raw `{{ ... }}` span is not ordinary user prose once Darkmatter has
  recognized it as an interpolation expression.
- Claudine side effects happen outside the composed prompt body, so the user may
  see a bad notification even if the provider run later succeeds or fails for a
  different reason.

There is also a prompt-quality issue: Claudine's bundled prompt fixtures are
not covered by a test that composes lifecycle metadata and asserts no raw
interpolation delimiters survive.

## Goals

1. Fix the bundled prompt typo in `prompts/implement-suggestions.md`.
2. Treat interpolation parse errors in lifecycle/frontmatter values as hard
   preparation failures, regardless of Darkmatter's non-fail-fast body behavior.
3. Ensure Claudine never sends lifecycle side effects containing raw
   `{{ ... }}` spans.
4. Report the bad expression with enough context to fix it: source file,
   frontmatter key path, expression text, and parser error.
5. Add regression coverage for this exact prompt shape and for the generic
   lifecycle-message failure path.

## Non-Goals

- Redesigning the full Darkmatter expression grammar.
- Removing lenient missing-variable behavior for ordinary prompt body text.
- Changing provider execution, model selection, shell expansion, or messaging
  transport behavior.
- Making `parent_dir` a Claudine-specific helper. It is already a Darkmatter
  expression function and should stay there.

## Required Behavior

### Prompt Fixture Correction

Update `prompts/implement-suggestions.md` so the start message uses valid
expressions:

```yaml
start:
    message: "🏃 starting the _implementation_ of the `{{ parent_dir(review) }}` review suggestions ({{area}}, iteration: {{iteration}})"
```

Also review the nearby lifecycle strings for documentation drift and obvious
typos introduced by this same block. For example, `implemention` should be
corrected if the prompt text is touched.

### Strict Lifecycle Interpolation

Claudine must validate rendered lifecycle side-effect strings before dispatch.
This includes at least:

- `start.message`
- `success.message`
- `failure.message`
- `success.say`
- any other lifecycle strings that are sent to messenger, TTS, sound, or stderr

If a rendered lifecycle string still contains a recognized interpolation span,
Claudine must not send the side effect. It should return a composition
preparation error that identifies the source.

The preferred fix is to make Darkmatter expose strict interpolation diagnostics
for frontmatter/lifecycle contexts so Claudine does not need a separate parser.
If that API is not available yet, Claudine may add a narrow guard at the
composition boundary as an interim defense, but the final design should keep
Darkmatter as the interpolation authority.

### Parse Errors Are Fatal For Frontmatter

For frontmatter interpolation, a lexer/parser failure inside a recognized
`{{ ... }}` span must be fatal even when the broader compose operation is using
non-fail-fast mode.

Example:

```yaml
start:
  message: "{{ parent_dir(review)) }}"
```

Expected result:

- No provider execution starts.
- No lifecycle notification is sent.
- The error mentions `parent_dir(review))`.
- The error mentions the parser reason, such as an unexpected `)`.
- The error identifies the key path `start.message`.

### Evaluation Errors Must Not Leak Raw Delimiters

Unknown functions are already treated as fatal in newer Darkmatter paths. Keep
that behavior.

For other non-fatal evaluation errors, do not preserve raw `{{ ... }}` text in
frontmatter values that can feed lifecycle side effects. Either:

- fail the lifecycle/frontmatter context, or
- replace with a safe null/empty value and emit a warning before any side
  effect dispatch.

For lifecycle messages, failing is preferred. A malformed notification is more
harmful than a skipped notification because it creates false status in external
systems.

### Undefined Lifecycle Variables

The screenshot also shows empty `area` and `iteration` values. The fix should
make the intended contract explicit:

- If `area` is supposed to be available, define it in prompt frontmatter or use
  the correct context variable such as `ctx.area`.
- If `iteration` is optional, the prompt should provide a default.
- If a lifecycle message references an undefined bare variable, Claudine should
  warn or fail before dispatching the side effect. Silent empty strings are
  acceptable in body prose, but lifecycle messages are operational status.

## Implementation Notes

Darkmatter already has most of the machinery:

- `ExpressionFinder` finds `{{ ... }}` spans.
- `parse` returns structured parser errors.
- `Evaluator` can collect context warnings and evaluate expressions.
- `interpolate_text` already has a fatal path when `fail_fast` is true.

The implementation should avoid adding a second expression parser in Claudine.
Prefer one of these approaches:

1. Add a Darkmatter interpolation mode for strict frontmatter/lifecycle
   contexts and have Claudine opt into it during composition preparation.
2. Promote frontmatter parse errors to fatal in Darkmatter generally, matching
   the active direction in `darkmatter/fixes/interpolation-error-handling`.
3. Add a temporary Claudine post-prepare guard that scans lifecycle strings for
   remaining `{{ ... }}` spans and reports a typed `CompositionError`, then
   remove or narrow it once Darkmatter exposes the strict contract.

Any new terminal diagnostic rendered by Claudine must use existing
`TerminalRenderable` components such as `Prose`, `UnorderedList`, or the
existing composition error renderers. Do not add ad hoc ANSI formatting.

## Test Plan

Add focused coverage at two levels.

### Darkmatter Unit Tests

- A frontmatter value with `{{ parent_dir(review)) }}` fails composition with a
  parse error even when normal body interpolation would be lenient.
- The error includes the expression text.
- The error includes the frontmatter key path when available.
- Valid `{{ parent_dir(review) }}` still resolves for relative paths on macOS,
  Windows, and Linux. Use forward-slash fixture strings and avoid shelling out.

### Claudine Unit Or CLI Tests

- A compose fixture with malformed `start.message` fails before provider
  execution and before messenger dispatch.
- The exact `prompts/implement-suggestions.md` fixture composes without leaving
  `{{` or `}}` in lifecycle metadata when passed:

  ```text
  review=features/2026-06-18-composition-shell-error-diagnostics/review-1.md
  ```

- A fake messenger route records zero sends when lifecycle interpolation fails.
- A regression test asserts `iteration` is populated or defaults correctly in
  the start message.

Use in-process tests where possible. If a CLI test is needed, use existing
Claudine test helpers and keep provider execution stubbed or dry-run isolated.

## Acceptance Criteria

- The bundled prompt no longer contains `parent_dir(review))` or `interation`.
- The command above cannot send a Discord message containing raw
  `{{ parent_dir(review)) }}`.
- Any lifecycle interpolation parse error fails before external side effects.
- The failure diagnostic points to the bad frontmatter key and expression.
- Regression tests cover both the prompt typo and the lifecycle side-effect
  guard.
