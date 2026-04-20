# Plan: Show System Prompt

## Summary

Display the composed system prompt in the preflight output using an orange-bordered `BlockQuote`, mirroring the existing Agent Prompt rendering. Unlike the Agent Prompt, the System Prompt is shown for **both interactive and non-interactive** sessions.

## Files to Modify

| File | Change |
|------|--------|
| `claudine/cli/src/output.rs` | Add `log_system_prompt()` function |
| `claudine/cli/src/commands/wrap/mod.rs` | Call `log_system_prompt()` in direct-wrapper preflight |
| `claudine/cli/src/commands/wrap/composition.rs` | Call `log_system_prompt()` in composition preflight |

No changes needed to `biscuit-terminal`, `darkmatter`, or `claudine/lib`.

---

## Step 1 — Add `log_system_prompt()` to `output.rs`

Add a new public function `log_system_prompt()` in `claudine/cli/src/output.rs`, placed immediately after `log_compose_prompt()` (after line 192).

### Signature

```rust
pub(crate) fn log_system_prompt(
    effective_sp: &claudine::system_prompt::EffectiveSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    term: &Terminal,
)
```

### Behavior

The function matches on `effective_sp` and handles each variant:

#### Variant: `EffectiveSystemPrompt::Ready(prepared)`

1. **Header**: `<bold>System Prompt(<dim><i>{variant}</i></dim>)</bold>` where `{variant}` is `"appended"` or `"replaced"` based on `prepared.mode` (`SystemPromptMode::Append` -> `"appended"`, `SystemPromptMode::Replace` -> `"replaced"`).
2. **Truncation**: If `verbose` is true, show the entire `prepared.composed_markdown`. Otherwise, take only the first **25 lines**.
3. **Markdown rendering**: Render through Darkmatter's `for_terminal()` with the same width-constraining logic as `log_compose_prompt()` (subtract border width `2` + left margin `2` + right margin `2` from terminal width).
4. **BlockQuote**: Wrap the rendered text in a `BlockQuote` with:
   - **Orange left border**: `Color::Tailwind(Tailwind::Orange700)`
   - **Border character**: `"▌ "` (same as Agent Prompt)
   - **Left margin**: 2 chars
   - **Right margin**: 2 chars
5. **Truncation notice**: If `!verbose` AND the prompt exceeds 25 lines, show: `- <dim>remaining prompt truncated for brevity, use <blue>--verbose</blue> to show entire prompt</dim>` (same pattern as Agent Prompt, just with different line threshold).

#### Variant: `EffectiveSystemPrompt::None`

- If `verbose`: render a single-line orange BlockQuote with the text `"the system prompt has not been modified"`.
  - Same `BlockQuote` settings (orange, `▌ `, margins).
  - No header line — just the BlockQuote itself.
- If `!verbose`: do nothing (return immediately).

#### Variant: `EffectiveSystemPrompt::Disabled { source }`

- Currently, the wrapper logs `"system prompt disabled by empty {source}"` as an info message. This existing behavior should be preserved — `log_system_prompt()` does **not** need to handle this variant because the existing inline `match` arms in both `mod.rs` and `composition.rs` already handle it before calling the function.
- Alternatively, this variant can be handled identically to `None` (verbose-only orange block quote saying `"the system prompt has been disabled"`). This is a minor design decision — both approaches work. The plan will use the same verbose-only pattern as `None` for consistency.

### Implementation Sketch

```rust
pub(crate) fn log_system_prompt(
    effective_sp: &claudine::system_prompt::EffectiveSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    use biscuit_terminal::utils::color::{Color, Tailwind};
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

    if silent {
        return;
    }

    match effective_sp {
        claudine::system_prompt::EffectiveSystemPrompt::None => {
            if verbose && !quiet {
                // One-line orange block quote
                let mut block = BlockQuote::new(
                    RenderableContent::from("the system prompt has not been modified".to_string()),
                    None::<&str>,
                )
                .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
                .with_border("▌ ");
                block.layout_mut().left_margin =
                    biscuit_terminal::utils::layout::Margin::Chars(2);
                block.layout_mut().right_margin =
                    biscuit_terminal::utils::layout::Margin::Chars(2);
                log::message(&block.render(term));
            }
        }
        claudine::system_prompt::EffectiveSystemPrompt::Disabled { source: _ } => {
            if verbose && !quiet {
                let mut block = BlockQuote::new(
                    RenderableContent::from("the system prompt has been disabled".to_string()),
                    None::<&str>,
                )
                .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
                .with_border("▌ ");
                block.layout_mut().left_margin =
                    biscuit_terminal::utils::layout::Margin::Chars(2);
                block.layout_mut().right_margin =
                    biscuit_terminal::utils::layout::Margin::Chars(2);
                log::message(&block.render(term));
            }
        }
        claudine::system_prompt::EffectiveSystemPrompt::Ready(prepared) => {
            let variant_label = match prepared.mode {
                claudine::system_prompt::SystemPromptMode::Append => "appended",
                claudine::system_prompt::SystemPromptMode::Replace => "replaced",
            };
            log::message(&Prose::new(format!(
                "<bold>System Prompt(<dim><i>{variant_label}</i></dim>)</bold>"
            )).render(term));
            log::message("");

            let full_text = &prepared.composed_markdown;
            let line_count = full_text.lines().count();
            let display_text = if verbose {
                full_text.clone()
            } else {
                full_text.lines().take(25).collect::<Vec<_>>().join("\n")
            };

            // Width calculation (same as log_compose_prompt)
            let left_margin: u16 = 2;
            let right_margin: u16 = 2;
            let border_width: u16 = 2;
            let content_width = (term.width() as u16)
                .saturating_sub(border_width)
                .saturating_sub(left_margin)
                .saturating_sub(right_margin);
            let mut opts = TerminalOptions::default();
            opts.max_width = Some(content_width);
            let rendered = match for_terminal(&Markdown::new(display_text.trim()), opts) {
                Ok(r) => r,
                Err(_) => display_text.clone(),
            };

            let mut block = BlockQuote::new(
                RenderableContent::from(rendered.trim_end().to_string()),
                None::<&str>,
            )
            .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
            .with_border("▌ ");
            block.layout_mut().left_margin =
                biscuit_terminal::utils::layout::Margin::Chars(left_margin as u32);
            block.layout_mut().right_margin =
                biscuit_terminal::utils::layout::Margin::Chars(right_margin as u32);
            log::message(&block.render(term));

            if !verbose && line_count > 25 {
                log::message("");
                log::message(
                    &Prose::new(
                        "- <dim>remaining prompt truncated for brevity, use <blue>--verbose</blue> to show entire prompt</dim>",
                    )
                    .with_word_wrap(WordWrap::WrapProse(None, Some(2)))
                    .render(term),
                );
            }
        }
    }
}
```

---

## Step 2 — Call `log_system_prompt()` in the Direct Wrapper (`mod.rs`)

**File**: `claudine/cli/src/commands/wrap/mod.rs`
**Location**: Inside the preflight output block (lines 1309–1355), between the env details and the blank line separator.

### Current code structure (lines 1327–1354):

```
if !quiet_requested {
    log_wrapper_env_details(...);

    // removed env info, repo flag, warnings, messages ...

    // Blank line to separate preamble from execution output
    log::message("");
}
```

### New code structure:

```
if !quiet_requested {
    log_wrapper_env_details(...);

    // removed env info, repo flag, warnings, messages ...

    // System Prompt display (shown for both interactive and non-interactive)
    crate::output::log_system_prompt(
        &effective_sp,
        detail_requested,
        silent_requested,
        quiet_requested,
        &term,
    );

    // Blank line to separate preamble from execution output
    log::message("");
}
```

### Where to insert

After the last `for message in &deferred_messages` loop (line 1349) and before the blank line separator (line 1353). Insert the call at approximately line 1350.

**Important**: The `effective_sp` variable is already available at this point — it is resolved at line 1045.

### Remove existing inline system prompt reporting

The existing `Disabled` handler (lines 1052–1058) currently logs an info message. Once `log_system_prompt()` handles all three variants, the `Disabled` and `None` match arms can be simplified or removed from the inline code. However, the `Ready` arm (lines 1060–1074) must remain because it performs the actual provider-specific application (`apply_system_prompt`). 

Recommended approach:
- Keep the `match &effective_sp` block as-is for the `Ready` variant (provider application).
- **Remove** the `Disabled` info log from the inline match (lines 1052–1058) since `log_system_prompt()` will handle it.
- **Remove** the `None` empty arm (line 1051) — no-op, no removal needed.

---

## Step 3 — Call `log_system_prompt()` in the Composition Path (`composition.rs`)

**File**: `claudine/cli/src/commands/wrap/composition.rs`
**Location**: Inside the preflight output block (lines 650–667).

### Current code structure (lines 650–667):

```rust
if !silent {
    if !quiet && (request.session_interactive || detail_requested) {
        crate::output::log_wrapper_env_details(&env_plan, None, &term, verbose);
    }
    if effective_non_interactive {
        crate::output::log_compose_prompt(&request.prepared.prompt, detail_requested, &term);
    }
    if !quiet {
        crate::log::message("");
    }
}
```

### New code structure:

```rust
if !silent {
    if !quiet && (request.session_interactive || detail_requested) {
        crate::output::log_wrapper_env_details(&env_plan, None, &term, verbose);
    }

    // System Prompt: shown for BOTH interactive and non-interactive
    if !quiet {
        crate::output::log_system_prompt(
            &effective_sp,
            detail_requested,
            silent,
            quiet,
            &term,
        );
    }

    // Agent Prompt: non-interactive only
    if effective_non_interactive {
        crate::output::log_compose_prompt(&request.prepared.prompt, detail_requested, &term);
    }

    if !quiet {
        crate::log::message("");
    }
}
```

### Key differences from current:

1. **System Prompt is shown for both interactive and non-interactive** — no `effective_non_interactive` guard.
2. **Ordering**: System Prompt is displayed **before** Agent Prompt (per spec).
3. **Guarded by `!quiet`**: Not shown when `--quiet` is active (consistent with env details).

### Remove existing inline system prompt reporting

Same as Step 2: remove the `Disabled` info log from the inline `match &effective_sp` block (lines 414–421). The `Ready` arm (lines 422–441) must remain for provider application.

---

## Ordering Summary

Per spec, the display order after implementation:

1. **Claudine Execution Line** (header)
2. **ENV Variables** (unless `--quiet` or `--silent`)
3. **System Prompt** — NEW (unless `--quiet` or `--silent`)
4. **Agent Prompt** — non-interactive only
5. **Blank separator line**
6. **Execution output**

---

## Edge Cases

| Case | Behavior |
|------|----------|
| No system prompt file found, no verbose | Nothing displayed |
| No system prompt file found, `--verbose` | Orange block quote: "the system prompt has not been modified" |
| System prompt disabled (empty file), no verbose | Nothing displayed |
| System prompt disabled, `--verbose` | Orange block quote: "the system prompt has been disabled" |
| System prompt ready, append mode | Header: `System Prompt(appended)`, orange block quote with content |
| System prompt ready, replace mode | Header: `System Prompt(replaced)`, orange block quote with content |
| System prompt > 25 lines, no verbose | Show first 25 lines + truncation notice |
| System prompt > 25 lines, `--verbose` | Show entire prompt |
| `--quiet` | No system prompt display at all |
| `--silent` | No system prompt display at all |
| Interactive session, system prompt present | System prompt IS shown (unlike Agent Prompt) |
| Interactive session, no system prompt, no verbose | Nothing displayed |
| `--dry-run` | Existing dry-run behavior preserved (uses `describe_effective()` not the new function) |

---

## Testing Strategy

1. **Manual testing scenarios**:
   - `claudine claude "hello"` with no `system-prompt.md` file → nothing shown
   - `claudine -v claude "hello"` with no `system-prompt.md` file → orange "not modified" block
   - `claudine claude "hello"` with a `system-prompt.md` in repo root → orange block with content
   - `claudine -v claude "hello"` with a long `system-prompt.md` → full content shown
   - `claudine --append-system-prompt ./my-prompt.md claude "hello"` → shows `System Prompt(appended)`
   - `claudine --replace-system-prompt ./my-prompt.md claude "hello"` → shows `System Prompt(replaced)`
   - `claudine claude` (interactive, no prompt) → system prompt shown if present
   - `claudine -q claude "hello"` → no system prompt shown
   - `claudine compose file.md` → system prompt shown before agent prompt
   - `claudine -v compose file.md` with no system prompt → orange "not modified" block

2. **Unit tests**: Add tests in `output.rs` for:
   - `log_system_prompt` with `EffectiveSystemPrompt::None` + verbose → renders block
   - `log_system_prompt` with `EffectiveSystemPrompt::None` + !verbose → no output
   - `log_system_prompt` with `Ready` + append mode → header contains "appended"
   - `log_system_prompt` with `Ready` + replace mode → header contains "replaced"
   - Truncation at 25 lines

---

## Out of Scope

- Changes to `--dry-run` behavior (it uses `describe_effective()` which is a different code path)
- Changes to system prompt composition logic
- Changes to `BlockQuote` or `biscuit-terminal`
- Changes to the sequence runner (it delegates to composition, so it inherits the change)
