# CLI Parsing and Reporting — Implementation Plan

## Overview

This plan implements two major changes from the [spec](./spec.md):

1. **Interactive/Non-Interactive logic inversion** — replace `--non-interactive`/`-n` with `--interactive`/`-i`, change the default behavior so that providing a prompt implies non-interactive
2. **Execution line cleanup** — stop leaking provider-specific switches into the displayed execution line; show only the user's prompt (truncated, newlines escaped)

---

## Phase 1: Interactive/Non-Interactive Logic Inversion

### 1.1 Update `WrapperArgs` struct

**File:** `claudine/cli/src/commands/wrap/mod.rs` (lines 346–433)

- Remove `non_interactive` field (`-n`, `--non-interactive`, `--ni`)
- Add `interactive` field (`-i`, `--interactive`)
- The prompt string is already captured as part of `passthrough` — no structural change needed there

```rust
// REMOVE:
#[arg(short = 'n', long = "non-interactive", visible_alias = "ni")]
pub non_interactive: bool,

// ADD:
/// Force interactive mode even when a prompt string is provided.
#[arg(short = 'i', long = "interactive")]
pub interactive: bool,
```

### 1.2 Update `ExtractedWrapperFlags`

**File:** `claudine/cli/src/commands/wrap/mod.rs` (lines 1492–1564)

- Rename `non_interactive` → `interactive` in struct
- Update `extract_wrapper_flags_from_passthrough()`:
  - Remove match arms for `-n`, `--non-interactive`, `--ni`
  - Add match arms for `-i`, `--interactive`

### 1.3 Determine whether a prompt string is present

We need a function to detect whether the user provided a prompt string (a non-switch argument). This is subtler than it sounds because `passthrough` already has switches stripped by clap for Claudine-owned flags, but may still contain provider passthrough switches. However, the key insight is:

- The prompt string is a positional argument (not a switch)
- After `extract_wrapper_flags_from_passthrough()` runs, any remaining non-switch arguments in `child_args` are either the prompt or provider-specific positional args
- We also need to account for composition modes (`--prompt-file`, `--frontmatter-prompt`, `--compose`) which provide a prompt programmatically

**Add helper:**

```rust
/// Returns true if a prompt string is present — either as a remaining
/// non-switch arg in `child_args` or from a composition source.
fn has_prompt_source(
    args: &WrapperArgs,
    child_args: &[String],
    stdin_seed: Option<&str>,
) -> bool {
    // Composition switches provide a prompt
    if args.prompt_file.is_some() || args.frontmatter_prompt.is_some() || args.compose.is_some() {
        return true;
    }
    if stdin_seed.is_some() {
        return true;
    }
    // Check for a non-switch positional arg in passthrough
    child_args.iter().any(|arg| !arg.starts_with('-'))
}
```

> **Note:** This heuristic (non-switch = prompt) is good enough because Claudine has already consumed its own switches. Remaining positional args are the user's prompt or provider args. When in doubt, the user can force the mode with `-i`.

### 1.4 Rewrite interactivity resolution in `run_provider_wrapper_inner()`

**File:** `claudine/cli/src/commands/wrap/mod.rs` (line ~467 onward)

Current logic:
```rust
let non_interactive_requested = args.non_interactive || extracted.non_interactive;
```

New logic:
```rust
let interactive_requested = args.interactive || extracted.interactive;

// Determine if a prompt is present (checked early, before composition pipelines)
// Composition pipelines also imply non-interactive by default.
let has_prompt = has_prompt_source(&args, &child_args, None);

// Default: interactive when no prompt, non-interactive when prompt present
// --interactive/-i overrides the default back to interactive
let non_interactive_requested = if interactive_requested {
    false
} else {
    has_prompt
};
```

**Important:** The composition pipeline section (lines 562–700) currently force-applies non-interactive mode. Under the new logic:
- Composition still _defaults_ to non-interactive (because `has_prompt_source` returns true)
- But if `--interactive`/`-i` is also set, we skip the `apply_non_interactive()` calls
- The `effective_non_interactive` calculation (line 696) needs updating:

```rust
let effective_non_interactive = if interactive_requested {
    false
} else {
    non_interactive_requested
        || prompt_file_dry_run.is_some()
        || inline_composition_source.is_some()
        || chained_composition
};
```

### 1.5 Update composition pipelines to respect `--interactive`

**Files:** `claudine/cli/src/commands/wrap/mod.rs` (lines 562–700)

Currently each composition pipeline unconditionally calls:
```rust
if !non_interactive_requested {
    profile.apply_non_interactive(&mut child_args)?;
    profile.apply_non_interactive_defaults(&mut child_args);
}
```

Change to:
```rust
if !interactive_requested && !non_interactive_requested {
    profile.apply_non_interactive(&mut child_args)?;
    profile.apply_non_interactive_defaults(&mut child_args);
}
```

Also change `apply_prompt_body()` calls — currently all pass `true` (non-interactive). When `--interactive` is set, pass `false`:
```rust
profile.apply_prompt_body(
    &mut child_args,
    &mut stdin_seed,
    &composed.body,
    !interactive_requested, // non-interactive unless --interactive
)?;
```

### 1.6 Update `--timeout` validation

**File:** `claudine/cli/src/commands/wrap/mod.rs` (lines 476–481)

Current: `--timeout` requires `--non-interactive`

New: `--timeout` requires that the session will be non-interactive (either because a prompt is present or because no `--interactive` override):

```rust
if args.timeout.is_some() && !effective_non_interactive_early {
    return Err(eyre!(
        "--timeout can only be used in non-interactive mode \
         (provide a prompt or use a composition switch)"
    ));
}
```

This check needs to move slightly later in the function, after we can determine `effective_non_interactive_early` but before the provider subprocess launches. Since composition hasn't run yet at line 476, we'll need a two-pass approach:
- **Early check:** If `--interactive` is set AND `--timeout` is set → error immediately
- **Late check:** After composition, if `effective_non_interactive` is false AND `--timeout` is set → error

### 1.7 Update `INTERACTIVE` env var

**File:** `claudine/cli/src/commands/wrap/env.rs` (lines 73–82)

The env builder sets `INTERACTIVE` based on `non_interactive_requested`. Update the parameter name and invert the sense if needed — the env var value should reflect the final resolved interactivity, not just the flag.

### 1.8 Update badge display

**File:** `claudine/cli/src/output.rs` (lines 26–111)

- The `non_interactive_requested` parameter to `log_wrapper_header()` should continue to work — we just need to pass `effective_non_interactive` which is now derived differently
- No badge rename needed; the badge text "Non-Interactive" is still accurate when active
- **Add:** A new `INTERACTIVE` badge for when the user explicitly forces interactive mode with `-i` while a prompt is present (makes it clear they overrode the default)

**File:** `claudine/lib/src/badges.rs`

```rust
pub static INTERACTIVE: LazyLock<String> = LazyLock::new(|| {
    Prose::new(
        "<bg-green-300><bold><green-900> Interactive </green-900></bold></bg-green-300>",
    )
    .render_optimistic(None)
    .to_string()
});
```

Display logic in `log_wrapper_header()`:
```rust
// Show Non-Interactive badge when session is non-interactive
// Show Interactive badge only when user explicitly forced it (prompt + -i)
if effective_non_interactive {
    header_parts.push(NON_INTERACTIVE.to_string());
} else if interactive_override {
    header_parts.push(INTERACTIVE.to_string());
}
```

---

## Phase 2: Execution Line Cleanup

### 2.1 Stop displaying provider-specific switches in execution line

**File:** `claudine/cli/src/output.rs` (lines 82–108)

Currently `log_wrapper_header()` receives `child_args` (which includes provider-specific switches like `--print`, `--dangerously-skip-permissions`, `--quiet`, etc.) and displays them in the dim trailing section.

**Change:** Stop passing `child_args` to the display. Instead, only show the user's prompt text.

The execution line format becomes:
```
Claudine ▸ {agent} {badges} {prompt}
```

Where `{prompt}` is:
- The user's prompt string (if present), truncated to fit one line
- A composition file path summary for `--compose`/`--frontmatter-prompt`/`--prompt-file`
- Empty when interactive with no startup prompt

### 2.2 Extract prompt text for display

**File:** `claudine/cli/src/commands/wrap/mod.rs`

Add a function to extract the user's raw prompt from the original args (before provider adaptation):

```rust
/// Extract the user's prompt string from the raw passthrough args.
/// Returns the first non-switch argument, if any.
fn extract_user_prompt(passthrough: &[String]) -> Option<String> {
    passthrough.iter().find(|arg| !arg.starts_with('-')).cloned()
}
```

Then compute the prompt display text:

```rust
let prompt_display: Option<String> = if let Some(ref pf) = args.prompt_file {
    Some(format!("--prompt-file {pf}"))
} else if let Some(ref fp) = args.frontmatter_prompt {
    Some(format!("--frontmatter-prompt {fp}"))
} else if let Some(ref c) = args.compose {
    Some(format!("--compose {c}"))
} else {
    extract_user_prompt(&args.passthrough)
};
```

### 2.3 Refactor `log_wrapper_header()` signature

**File:** `claudine/cli/src/output.rs`

Change the function signature:

```rust
pub(crate) fn log_wrapper_header(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive: bool,
    interactive_override: bool,  // NEW: user forced -i with a prompt
    verbose_requested: bool,
    repo_requested: bool,
    compose_display: Option<&ComposeDisplay>,
    operation: Option<&str>,
    prompt_display: Option<&str>,  // CHANGED: was child_args + prompt_summary
    env_plan: &EnvPlan,
    term: &Terminal,
)
```

### 2.4 Update prompt display formatting

**File:** `claudine/cli/src/output.rs`

The prompt display should:
1. Replace literal `\n` chars with visible `\n` text (already handled by `shell_escape()`)
2. Truncate with trailing ellipsis to keep on one line (existing `truncate_args()` logic)
3. Prefix with `--prompt` for display clarity → actually no, per spec just show the prompt text dimmed

```rust
if let Some(prompt) = prompt_display {
    let escaped = shell_escape(prompt);
    let prefix = header_parts.join(" ");
    let prefix_width = visible_width(&prefix) as usize;
    let used = prefix_width + 1;
    let term_width = term.width() as usize;
    let available = term_width.saturating_sub(used);
    let truncated = truncate_args(&escaped, available);
    header_parts.push(Prose::new(format!("<dim>{truncated}</dim>")).render(term));
}
```

Key change from current: `available` is based on `term_width` (one line) not `term_width * 2`. The spec says "truncate with trailing ellipsis to keep reporting to one line".

### 2.5 Update `log_dry_run()` header

**File:** `claudine/cli/src/output.rs` (lines 162–237)

Apply the same principle — don't dump `child_args` into the header. The dry-run already shows the full command separately, so the header just needs badges + prompt summary.

---

## Phase 3: Update Tests

### 3.1 Update `extract_wrapper_flags_from_passthrough` tests

**File:** `claudine/cli/src/commands/wrap/mod.rs` (test module, line ~1577+)

- Remove tests for `-n`/`--non-interactive`/`--ni` extraction
- Add tests for `-i`/`--interactive` extraction
- Test that `-n` now passes through to the provider (not consumed by Claudine)

### 3.2 Add interactivity resolution tests

New tests for the default logic:
- No prompt, no flags → interactive
- Prompt present, no flags → non-interactive
- Prompt present + `-i` → interactive (override)
- `--compose FILE`, no `-i` → non-interactive
- `--compose FILE` + `-i` → interactive
- `--timeout` + prompt → OK (non-interactive by default)
- `--timeout` + `-i` → error
- `--timeout` + no prompt → error (interactive by default)

### 3.3 Update execution line display tests

- Verify provider switches (e.g., `--print`, `--dangerously-skip-permissions`) do NOT appear in execution line
- Verify prompt text IS shown (truncated, newlines escaped)
- Verify one-line truncation (not two-line)

### 3.4 Update existing integration tests

Any existing tests that pass `-n` or `--non-interactive` need updating to use the new semantics. Tests that rely on the old default (interactive unless `-n`) need a prompt added or `-i` flag.

---

## Phase 4: Documentation and Cleanup

### 4.1 Update help text

- `WrapperArgs` doc comments already need updating from Phase 1
- Ensure `claudine claude --help` accurately describes the new behavior

### 4.2 Update CLAUDE.md / skill docs

- Update the claudine skill if it mentions `--non-interactive`
- Note the new `--interactive`/`-i` switch

### 4.3 Remove dead code

- Remove any `non_interactive`-specific helper functions that are no longer needed
- Clean up any comments referencing the old behavior

---

## File Change Summary

| File | Changes |
|------|---------|
| `claudine/cli/src/commands/wrap/mod.rs` | `WrapperArgs`: remove `-n`, add `-i`; `ExtractedWrapperFlags`: rename field; `extract_wrapper_flags_from_passthrough()`: swap match arms; `run_provider_wrapper_inner()`: rewrite interactivity logic; composition pipelines: respect `-i`; add `has_prompt_source()`, `extract_user_prompt()` helpers; update `--timeout` validation; update tests |
| `claudine/cli/src/output.rs` | `log_wrapper_header()`: new signature, remove `child_args` display, show prompt only; `log_dry_run()`: matching changes; truncation to one line |
| `claudine/lib/src/badges.rs` | Add `INTERACTIVE` badge |
| `claudine/cli/src/commands/wrap/env.rs` | Update `INTERACTIVE` env var logic |
| `claudine/cli/src/commands/wrap/profile.rs` | No structural changes (adapter pattern unchanged) |

## Risks and Considerations

1. **Breaking change for users** — Anyone scripting `claudine claude -n ...` will break. Consider a deprecation period where `-n` is still accepted with a warning, or document the break clearly.
2. **Prompt detection heuristic** — The "first non-switch arg = prompt" heuristic may misidentify provider-specific positional args. The `-i` escape hatch mitigates this.
3. **Provider compatibility** — Some providers may behave differently when launched interactively with a startup prompt. Testing each provider is essential.
4. **Composition + interactive** — Sending a composed prompt as the first message in an interactive session is a new code path. Each provider's `apply_prompt_body()` needs to handle the `non_interactive=false` case correctly — verify for all 7 providers.
