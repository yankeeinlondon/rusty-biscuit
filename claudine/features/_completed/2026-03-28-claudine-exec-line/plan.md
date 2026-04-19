# Claudine Execution Line — Implementation Plan

## Overview

This plan addresses three areas of the execution line: badge visibility logic, operation badge styling, and compose-based prompt display. All changes are scoped to `claudine/lib/src/badges.rs`, `claudine/cli/src/output.rs`, and `claudine/cli/src/commands/wrap/composition.rs` (plus the call-site in `wrap/mod.rs`).

---

## Phase 1: Badge Visibility Changes (Non-Interactive / Interactive)

### Current Behavior

| Scenario | Badge Shown |
|---|---|
| Prompt provided, no `-i` | `Non-Interactive` |
| Prompt provided + `-i` | `Interactive` |
| No prompt (default interactive) | *(none)* |

### Target Behavior

| Scenario | Badge Shown |
|---|---|
| Prompt provided, no `-i` | *(none)* — non-interactive is the norm when a prompt is given |
| Prompt provided + `-i` | `Interactive` — user opted into interactive |
| No prompt (default interactive) | `Interactive` — show for now even though it's the default |

### 1a. Update `INTERACTIVE` badge color scheme

**File:** `claudine/lib/src/badges.rs`

The spec says the Interactive badge should be the **inverse** of the Non-Interactive badge:
- Non-Interactive: `bg-slate-300` background + `purple-900` text (whitish bg, purplish text)
- Interactive (target): `bg-purple-900` background + `slate-300` text (purplish bg, whitish text)

**Change:** Replace the current `INTERACTIVE` definition (lines 81–87):

```rust
// BEFORE
pub static INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-green-300><bold><green-900><bold> Interactive </bold></green-900></bold></bg-green-300>",
    )
    .render_optimistic(None)
    .to_string()
});

// AFTER
pub static INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-purple-900><bold><slate-300><bold> Interactive </bold></slate-300></bold></bg-purple-900>",
    )
    .render_optimistic(None)
    .to_string()
});
```

### 1b. Change badge display logic in `log_wrapper_header`

**File:** `claudine/cli/src/output.rs` (lines 54–60)

```rust
// BEFORE
if non_interactive {
    header_parts.push(NON_INTERACTIVE.to_string());
} else if interactive_override {
    header_parts.push(INTERACTIVE.to_string());
}

// AFTER — Never show Non-Interactive badge; always show Interactive when session is interactive
if !non_interactive {
    header_parts.push(INTERACTIVE.to_string());
}
```

Rationale:
- When `non_interactive` is true (prompt given, no `-i`): show nothing (this is the norm).
- When `non_interactive` is false: the session is interactive — show the `Interactive` badge. This covers both:
  - User explicitly passed `-i` with a prompt (`interactive_override`).
  - No prompt at all (default interactive session).

### 1c. Simplify `log_wrapper_header` signature

With the new logic, the `interactive_override` parameter is no longer needed — we only check `non_interactive`. The parameter can be removed from:

- `log_wrapper_header()` signature in `output.rs`
- The call-site in `wrap/mod.rs:1032`
- The call-site in `composition.rs:491`

However, `interactive_override` is currently computed and may be used elsewhere, so confirm it has no other consumers before removing. If uncertain, leave the parameter and simply ignore it inside the function.

---

## Phase 2: Operation Badge Restyling

### Current Behavior

**File:** `claudine/cli/src/output.rs` (line 77)

```rust
header_parts.push(Prose::new(format!("<green><bold>OP:</bold> {op}</green>")).render(term));
```

Renders as green text `OP: {op}` with no background — not visually a "badge."

### Target Behavior

The spec requests:
1. Add a complementary background color so it looks like a badge
2. Format text as: `<b>Op(<dim><i>{op}</i></dim>)</b>`

### 2a. Update the operation display in `log_wrapper_header`

**File:** `claudine/cli/src/output.rs` (line 76–78)

```rust
// BEFORE
if let Some(op) = operation {
    header_parts.push(Prose::new(format!("<green><bold>OP:</bold> {op}</green>")).render(term));
}

// AFTER
if let Some(op) = operation {
    header_parts.push(
        Prose::new(format!(
            "<bg-green-900><green-100><bold> Op(<dim><i>{op}</i></dim>) </bold></green-100></bg-green-900>"
        ))
        .render(term),
    );
}
```

Design notes:
- `bg-green-900` + `green-100` gives a dark green background with light green text — consistent with the muted badge palette used by REPO_FLAG (`bg-gray-800` + `green-100`).
- Spaces inside the bold tags (` Op(...) `) provide badge padding consistent with other badges.
- The `{op}` value is rendered as `<dim><i>` inside the bold context per spec.

### 2b. Consider making this a `LazyLock` in `badges.rs`

Unlike other badges, the operation badge content is dynamic (it contains the `{op}` value). Two options:

**Option A — Keep inline (recommended):** Since the value varies per invocation, keep it rendered inline in `output.rs` as shown above. This is consistent with `PACKAGE_NAME` display on line 80–87.

**Option B — Create a helper function in `badges.rs`:**

```rust
pub fn operation_badge(op: &str) -> String {
    Prose::new(format!(
        "<bg-green-900><green-100><bold> Op(<dim><i>{op}</i></dim>) </bold></green-100></bg-green-900>"
    ))
    .render_optimistic(None)
    .to_string()
}
```

Go with **Option A** unless the operation badge is needed in other call sites.

---

## Phase 3: Compose-Based Prompt Display

### Current Behavior

**File:** `claudine/cli/src/commands/wrap/composition.rs` (lines 477–486)

For compose/inline-compose commands, the resolved prompt is truncated to 120 chars and displayed on the execution line — same as static string prompts.

### Target Behavior

For compose/inline-compose file-referenced prompts:
1. **Execution line:** Instead of the resolved prompt, show `prompt sourced from {file}` (dim/italic with blue filename).
2. **After env variables + blank line:** Render a `Prompt:` section with the composed prompt as a `BlockQuote`.
   - **Verbose mode:** Full prompt.
   - **Non-verbose:** First 10 lines, then truncation notice.

Static string prompts are **unchanged** — continue displaying and truncating on the execution line as today.

### 3a. Change prompt display for compose mode on the execution line

**File:** `claudine/cli/src/commands/wrap/composition.rs` (lines 477–503)

Replace the current truncated-prompt logic with a file-source indicator when in compose mode:

```rust
// BEFORE
let prompt_display = {
    let raw = &request.prepared.prompt;
    let flat = raw.replace('\n', "\\n").replace('\r', "\\r");
    if flat.len() > 120 {
        Some(format!("{}...", &flat[..120]))
    } else {
        Some(flat)
    }
};

// AFTER — Show source file indicator instead of resolved prompt
let source_filename = request.prepared.resolved_path
    .file_name()
    .map(|f| f.to_string_lossy().to_string())
    .unwrap_or_else(|| request.file_ref.clone());
let prompt_display = None; // Don't show prompt text on exec line
let source_display = Some(format!(
    "<dim><i>prompt sourced from <blue>{source_filename}</blue></i></dim>"
));
```

Then in the header call, pass `prompt_display` (now `None`) so no prompt text appears, and separately render `source_display` as an additional `header_part`:

```rust
if !silent {
    crate::output::log_wrapper_header(
        profile,
        yolo_enabled,
        effective_non_interactive,
        interactive_override,
        verbose_requested,
        request.repo,
        compose_display.as_ref(),
        request.operation.as_deref(),
        prompt_display.as_deref(),  // None — no prompt text on exec line
        &env_plan,
        &term,
    );
    // Append source-file indicator after the header
    if let Some(ref source) = source_display {
        // Render inline on the same conceptual line.
        // Since log_wrapper_header already printed, we need a different approach.
    }
}
```

**Better approach:** Rather than appending after, add a new parameter to `log_wrapper_header` for an optional suffix string, or handle the source display as part of `prompt_display` content:

```rust
let prompt_display = Some(format!(
    "prompt sourced from {source_filename}"
));
```

Then in `output.rs`, detect the compose case by checking `compose_display` and render the prompt_display with the dim/italic/blue styling instead of the normal dim styling. Alternatively, pass the already-styled content and let `output.rs` render it without escaping `<`.

**Recommended approach:** Pass a new `compose_source_hint: Option<&str>` parameter to `log_wrapper_header` containing just the filename. The function renders the styled text. This keeps styling centralized in `output.rs`:

**File:** `claudine/cli/src/output.rs`

Add parameter `compose_source_hint: Option<&str>` after `prompt_display`:

```rust
pub(crate) fn log_wrapper_header(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive: bool,
    interactive_override: bool,
    verbose_requested: bool,
    repo_requested: bool,
    compose_display: Option<&ComposeDisplay>,
    operation: Option<&str>,
    prompt_display: Option<&str>,
    compose_source_hint: Option<&str>,   // NEW
    env_plan: &EnvPlan,
    term: &Terminal,
) {
    // ... existing badge logic ...

    // Show compose source hint OR prompt text, never both
    if let Some(filename) = compose_source_hint {
        let prose_safe = filename.replace('<', "\\<");
        header_parts.push(
            Prose::new(format!(
                "<dim><i>prompt sourced from <blue>{prose_safe}</blue></i></dim>"
            ))
            .render(term),
        );
    } else if let Some(prompt) = prompt_display {
        // ... existing prompt truncation logic ...
    }
}
```

**Call-site updates:**
- `wrap/mod.rs:1032` — pass `None` for `compose_source_hint` (non-compose path)
- `composition.rs:491` — pass `Some(&source_filename)` and `None` for `prompt_display`

### 3b. Add `Prompt:` block after env variables

**File:** `claudine/cli/src/output.rs`

Create a new function:

```rust
/// Render the composed prompt as a BlockQuote after environment details.
pub(crate) fn log_compose_prompt(
    prompt: &str,
    verbose: bool,
    term: &Terminal,
) {
    use biscuit_terminal::components::block_quote::BlockQuote;

    log::message(&Prose::new("<bold>Prompt:</bold>").render(term));

    if verbose {
        let block = BlockQuote::new(
            RenderableContent::from(prompt.to_string()),
            None,
        );
        log::message(&block.fallback_render(term));
    } else {
        let lines: Vec<&str> = prompt.lines().collect();
        let truncated: String = lines.iter().take(10).copied().collect::<Vec<_>>().join("\n");
        let block = BlockQuote::new(
            RenderableContent::from(truncated),
            None,
        );
        log::message(&block.fallback_render(term));

        if lines.len() > 10 {
            log::message("");
            log::message(
                &Prose::new(
                    "<dim><i>remaining prompt truncated for brevity, use <blue>--verbose</blue> to show entire prompt</i></dim>"
                ).render(term)
            );
        }
    }
}
```

### 3c. Call `log_compose_prompt` from the composition execution path

**File:** `claudine/cli/src/commands/wrap/composition.rs`

After the header and env details are logged (after the equivalent of the `if !quiet` block in the compose path), add:

```rust
if !silent && !quiet {
    // ... existing env details, warnings, messages ...

    // Blank line separator
    log::message("");

    // Show composed prompt
    crate::output::log_compose_prompt(
        &request.prepared.prompt,
        verbose_requested,
        &term,
    );

    // Blank line to separate from execution output
    log::message("");
}
```

Check the compose path carefully — `composition.rs` currently does not replicate the full `if !quiet { ... }` env details block that `mod.rs` has. Verify whether env details are logged in the compose path and insert the `log_compose_prompt` call in the correct location (after env details, before execution).

### 3d. Non-compose path unchanged

The `wrap/mod.rs` wrapper path (static string prompts) requires **no changes** to prompt display. The prompt continues to be truncated and shown inline on the execution line.

---

## Phase 4: Testing & Verification

### Manual Testing Matrix

| Scenario | Expected Execution Line |
|---|---|
| `claudine claude` (no prompt) | `Claudine ▸ Claude [Interactive]` |
| `claudine claude 'hello'` | `Claudine ▸ Claude hello` (no badge) |
| `claudine claude -i 'hello'` | `Claudine ▸ Claude [Interactive] hello` |
| `claudine claude --op deploy 'hello'` | `Claudine ▸ Claude [Op(deploy)] hello` |
| `claudine compose '@file.md'` | `Claudine ▸ Claude [Compose] prompt sourced from file.md` |
| `claudine compose '@file.md' -v` | Same exec line + full Prompt: block below |
| `claudine compose '@file.md'` (no `-v`) | Same exec line + first 10 lines of Prompt: block + truncation notice |

### Unit Tests

- Badge color assertions can be added to existing test infrastructure if present
- `log_compose_prompt` should be testable by capturing its output

---

## File Change Summary

| File | Changes |
|---|---|
| `claudine/lib/src/badges.rs` | Restyle `INTERACTIVE` badge (green → purple inverse) |
| `claudine/cli/src/output.rs` | 1) Remove Non-Interactive badge display, always show Interactive when `!non_interactive`. 2) Restyle operation inline rendering. 3) Add `compose_source_hint` parameter to `log_wrapper_header`. 4) Add `log_compose_prompt` function. |
| `claudine/cli/src/commands/wrap/mod.rs` | Update `log_wrapper_header` call-site (new parameter, possibly remove `interactive_override`). |
| `claudine/cli/src/commands/wrap/composition.rs` | Replace truncated prompt display with source file hint. Call `log_compose_prompt` after env details. |

## Execution Order

1. **Phase 1** (badges) and **Phase 2** (operation) are independent — can be done in parallel.
2. **Phase 3** (compose prompt) depends on Phase 1 completing (the `log_wrapper_header` signature changes).
3. **Phase 4** (testing) follows all phases.
