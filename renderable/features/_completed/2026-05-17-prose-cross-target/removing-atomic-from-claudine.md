# Removing Atomic Prose Tokens From Claudine

Audit scope: `claudine/lib` and `claudine/cli`.

The Prose cross-target work removes atomic Prose tokens such as `{{dim}}`,
`{{bold}}`, and `{{reset}}`. Prose call sites should move to bracketed tags
such as `<dim>...</dim>` or to the supported Markdown subset.

## Summary

- `claudine/lib/src`: no atomic-token Prose use found.
- `claudine/cli/src`: atomic-token Prose use is concentrated in hook-related
  command output.
- Non-Prose `{{...}}` template/interpolation strings were excluded unless they
  are passed through `Prose`.

## Findings

### Stage 1

Stage 1 items are simple one-for-one replacements where the styled range is
obvious and the content is static or already controlled.

| Stage | Location | Atomic Prose Use |
|-------|----------|------------------|
| Stage 1 | `claudine/cli/src/commands/hooks/capture_method.rs:52-57` | Table cells render `{{cyan}}acp{{reset}}` and `{{dim}}-{{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/capture_method.rs:69-76` | Static legend/note strings use repeated `{{dim}}`, `{{cyan}}`, and `{{reset}}` tokens. |
| Stage 1 | `claudine/cli/src/commands/hooks/mapping.rs:26-32` | Static legend uses `{{dim}}...{{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/describe.rs:39-46` | Static response/return schema legends use `{{dim}}...{{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/support.rs:58-62` | Static support legend uses `{{dim}}`, `{{reset}}`, and a literal `{NO_SUPPORT}` placeholder inside the string. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:137-142` | Invalid sound effects header uses `{{yellow}}{{bold}}...{{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:167-180` | Static hint lines use `{{dim}}`, `{{blue}}`, and `{{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:366-376` | Provider detail header/docs lines use atomic bold/dim wrapping around controlled provider metadata. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:398-415` | Support/action table cells render `{{dim}}non-hook{{reset}}`, `{{cyan}}acp{{reset}}`, `{{dim}}-{{reset}}`, and `{{dim}}(no actions){{reset}}`. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:438-443` | Provider summary uses atomic bold/yellow styling around controlled counts and provider display text. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:458-466` | Event description heading uses atomic bold/red styling. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:480-485` | Unsupported event name cell uses atomic red/strikethrough around controlled event names. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:615-627` | Static simple-view hints use atomic outer dim styling mixed with bracketed `<blue><bold>...</bold></blue>` spans. |
| Stage 1 | `claudine/cli/src/commands/hooks/list.rs:692-705` | Verbose legend and hints use atomic dim/reset wrappers. |
| Stage 1 | `claudine/cli/src/commands/hooks/variables.rs:53-56` | Context variables header uses atomic bold and dim styling around static text. |
| Stage 1 | `claudine/cli/src/commands/hooks/variables.rs:110-112` | Example header uses `{{bold}}Example{{reset}}`. |

### Stage 2

Stage 2 items need more consideration because they involve dynamic text,
intentional literal `{{...}}` template placeholders, mixed style restoration,
or composed fragments whose current reset behavior is doing more than a simple
close tag.

| Stage | Location | Atomic Prose Use | Why Stage 2 |
|-------|----------|------------------|-------------|
| Stage 2 | `claudine/cli/src/commands/hooks/list.rs:146-162` | Invalid sound effect rows are built with escaped format-string braces for `{{dim}}`, `{{red}}`, `{{green}}`, and `{{reset}}`. | The styled spans wrap dynamic effect names and suggestions. A bracketed rewrite should also decide whether to Prose-escape those inserted values before placing them inside tags. |
| Stage 2 | `claudine/cli/src/commands/hooks/list.rs:244-347` and rendered at `claudine/cli/src/commands/hooks/list.rs:411-419` | `DI` and `DI_R` constants inject `{{dim}}{{italic}}` and `{{normal-font-weight}}{{not-italic}}` into formatted hook action strings. | This is not a normal reset. The comment says it preserves table-striping background while undoing dim+italic only. Bracketed tags may be cleaner, but the surrounding action formatters include dynamic command/message/template text and need escaping plus a check that nested tag closure preserves the table background behavior. |
| Stage 2 | `claudine/cli/src/commands/hooks/list.rs:490-493` | Unsupported event descriptions are wrapped with `{{dim}}...{{reset}}`. | The inserted description text comes from event metadata. It is likely controlled, but a bracketed migration should still confirm descriptions cannot contain Prose-significant characters. |
| Stage 2 | `claudine/cli/src/commands/hooks/list.rs:592-607` | Legend fragments are assembled with atomic red/yellow/dim/strikethrough tokens, then inserted into an outer atomic dim legend. | The current fragments intentionally transition between red/yellow and dim using atomic resets. A bracketed rewrite should preserve the intended nested styling and avoid invalid overlapping spans. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:21-27` | Header and intro use atomic bold/dim/cyan styling, and the intro displays literal template placeholders such as `{{tool_name}}` and `{{error}}`. | The header is simple, but the intro must preserve literal template syntax inside styled output. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:38-46` | Variable placeholders and availability values are wrapped with atomic cyan/dim tokens. | `var.placeholder()` intentionally returns literal template syntax such as `{{tool_name}}`. The rewrite must preserve those literal braces as display text while applying color. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:75-80` | Category divider rows wrap `cat.label()` with atomic dim styling. | The label is controlled, but dynamic. A direct tag replacement is probably fine, but the migration should confirm category labels cannot contain Prose markup characters. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:87-96` | Current-value cells use atomic dim for `-` and atomic green for detected values. | Empty values are simple, but detected environment values are dynamic and should be escaped or otherwise confirmed safe before being placed inside bracketed tags. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:98-100` | Context variable placeholders are wrapped with atomic cyan tokens. | Same literal-template concern as `var.placeholder()` above: display text like `{{git.branch}}` must remain literal inside the styled span. |
| Stage 2 | `claudine/cli/src/commands/hooks/variables.rs:114-120` | Example JSON block is wrapped in `{{dim}}...{{reset}}` while the example body intentionally contains `{{tool_name}}`, `{{git.branch}}`, and `{{error}}`. | This is a multi-line literal example containing template placeholders. A bracketed wrapper can work, but the migration should verify the JSON/template content remains literal and does not need Markdown code-fence rendering instead. |

## Excluded Atomic-Looking Strings

The following strings contain `{{...}}` but were not counted as atomic Prose
usage because they are not passed through `Prose`:

- `claudine/cli/src/commands/actions.rs:59-61`
- `claudine/cli/src/commands/actions.rs:179-181`
- Template/interpolation code and tests under `claudine/lib/src`, including
  dispatch templates, validation templates, sequence templates, loop actions,
  and provider configuration placeholders.

## Stage Suggestions

### Pattern Identification

- **Escaped dynamic text inside styled spans**: several Stage 2 items place
  runtime strings inside Prose markup. These should use a Prose-level escaping
  utility, for example `Prose::escape_text`, `ProseText`, or a builder method
  that inserts text nodes without letting `<...>` or `{{...}}` become markup.
- **Literal template placeholders as display text**: hook variable output needs
  to show strings like `{{tool_name}}` and `{{git.branch}}` literally while
  still applying style. This is a strong signal for a small Prose helper such as
  `Prose::literal` or `Prose::styled_literal(style, value)`.
- **Scoped style composition instead of reset choreography**: composed legends
  currently rely on atomic resets to switch between red, yellow, strikethrough,
  and dim. A tag or builder API that emits valid nested spans would remove the
  need to reason about overlapping styles by hand.
- **Partial style restoration inside table cells**: `DI`/`DI_R` intentionally
  undo dim and italic without clearing table striping background. This suggests
  Prose needs either verified nested tag behavior that preserves ambient table
  style or an explicit scoped style API that only applies and removes the
  foreground/text attributes it owns.
- **Preformatted literal blocks**: the example JSON is semantically a literal
  block, not ordinary prose. A Prose utility for dimmed literal blocks, or a
  documented preference for Markdown code fences when rendering examples, would
  make this migration less fragile.

### Implementation Suggestions

- `claudine/cli/src/commands/hooks/list.rs:146-162`: use **Escaped dynamic text
  inside styled spans** for `effect.invalid_name` and `similar`. Render the row
  as bracketed/static Prose structure, but insert both dynamic values through a
  literal-text escape helper before wrapping them in `<red>` and `<green>`.
- `claudine/cli/src/commands/hooks/list.rs:244-347` and rendered at
  `claudine/cli/src/commands/hooks/list.rs:411-419`: use **Partial style
  restoration inside table cells** and **Escaped dynamic text inside styled
  spans**. Replace `DI`/`DI_R` with a scoped dim+italic span only after verifying
  the closing tag preserves the table background; escape command, message,
  argument, and template strings before inserting them into the span.
- `claudine/cli/src/commands/hooks/list.rs:490-493`: use **Escaped dynamic text
  inside styled spans**. Event descriptions are controlled metadata today, so a
  direct `<dim>...</dim>` replacement is probably enough, but the safer pattern
  is to route `event.description()` through the same literal-text helper.
- `claudine/cli/src/commands/hooks/list.rs:592-607`: use **Scoped style
  composition instead of reset choreography**. Build each legend fragment as a
  complete nested span, then join the fragments inside one outer dim context
  without relying on reset tokens to restore dim styling.
- `claudine/cli/src/commands/hooks/variables.rs:21-27`: use **Literal template
  placeholders as display text**. The header can move directly to `<bold>`, but
  the intro example should style the full literal sample through a helper that
  preserves `{{tool_name}}` and `{{error}}` as text.
- `claudine/cli/src/commands/hooks/variables.rs:38-46`: use **Literal template
  placeholders as display text** for `var.placeholder()` and **Escaped dynamic
  text inside styled spans** for `var.availability()`. The variable placeholder
  should be rendered as a cyan literal, while availability can be dimmed after
  escaping or confirming it remains controlled text.
- `claudine/cli/src/commands/hooks/variables.rs:75-80`: use **Escaped dynamic
  text inside styled spans** lightly. Category labels are controlled enum data,
  so this can be a direct `<dim># ...</dim>` migration if labels remain
  Prose-safe; otherwise pass `cat.label()` through the literal-text helper.
- `claudine/cli/src/commands/hooks/variables.rs:87-96`: use **Escaped dynamic
  text inside styled spans**. Keep the empty `-` as a simple dim literal, and
  escape the detected environment value before wrapping it in `<green>`.
- `claudine/cli/src/commands/hooks/variables.rs:98-100`: use **Literal template
  placeholders as display text**. Render `var.placeholder()` as cyan literal
  text so placeholders like `{{git.branch}}` are displayed, not interpreted.
- `claudine/cli/src/commands/hooks/variables.rs:114-120`: use **Preformatted
  literal blocks** and **Literal template placeholders as display text**. Prefer
  a dimmed literal block or Markdown code-fence path for the JSON example; if it
  remains a raw Prose string, wrap the whole block with a literal-preserving
  helper before applying dim styling.
