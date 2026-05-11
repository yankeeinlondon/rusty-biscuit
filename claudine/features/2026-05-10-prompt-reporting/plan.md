# Prompt Reporting — Execution Plan

## Summary

Overhaul how Claudine reports the **System Prompt** and **User/Agent Prompt** in the preflight output. The current implementation always shows the full system prompt body and truncates the agent prompt at 10 lines. The new implementation introduces a layered verbosity system (CLI > ENV > Frontmatter > Prompt Length > Default) with four rendering modes: Summary, Partial (Truncate/FrontBack), Full, and Silent. The system prompt defaults to Summary-only; the agent prompt defaults to up to 40 lines with FrontBack truncation beyond that.

## Current State

### System Prompt (`cli/src/output/mod.rs:217-300`)

- `log_system_prompt()` receives `(effective_sp, verbose, silent, quiet, term)`
- `--silent`: nothing shown
- `None`/`Disabled` variants: shown only when `verbose && !quiet`
- `Ready`: **always** shows the full composed markdown body in an orange `BlockQuote`
- No concept of "summary", "partial", or token counts

### Agent Prompt (`cli/src/output/mod.rs:155-215`)

- `log_compose_prompt()` receives `(prompt, verbose, term)`
- `verbose=false`: first 10 lines + truncation notice
- `verbose=true`: full prompt in a green `BlockQuote`
- No `--quiet`/`--silent` gating

### Flags (`cli/src/commands/wrap/flags.rs`)

- `--verbose`/`-v`: global flag, increments `u8` counter
- `--quiet`/`-q`: wrapper flag, suppresses env details but still shows system prompt
- `--silent`: wrapper flag, suppresses all preflight output
- Merged at `wrap/mod.rs:356-358`

### Types (`lib/src/system_prompt/types.rs`)

- `SystemPromptMode`: `Append | Replace`
- `PreparedSystemPrompt`: has `mode`, `source`, `raw_text`, `composed_markdown`, `non_interactive_appendix`
- `EffectiveSystemPrompt`: `None | Disabled { source } | Ready(PreparedSystemPrompt)`
- No token count, no verbosity hint, no frontmatter verbosity

### System Prompt Pipeline

1. `resolve_system_prompt_source()` in `lib/src/system_prompt/resolve.rs` discovers the file
2. `resolve_and_prepare_for_session()` in `lib/src/system_prompt/prepare.rs` composes through Darkmatter
3. Darkmatter parses frontmatter from `raw_text` but strips it from `composed_markdown`
4. `raw_text` retains frontmatter (used in test `frontmatter_not_forwarded`)

### Frontmatter API

- `Markdown::frontmatter()` → `&Frontmatter` with `.get::<T>(key)` for typed access
- `FrontmatterMap` is an `IndexMap<String, serde_json::Value>`
- `raw_text` is available on `PreparedSystemPrompt` for re-parsing

---

## Phases

### Phase 1: Types & Verbosity Resolution (lib crate)

**Goal:** Introduce types that capture the reporting configuration and make verbosity resolution testable in the lib crate.

#### Task 1.1 — Add `PromptReportStyle` enum

**File:** `lib/src/system_prompt/types.rs`

Add alongside the existing types:

```rust
/// How a prompt should be reported in the preflight output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptReportStyle {
    /// Show only a one-line summary (token count, file path, action).
    #[default]
    Summary,
    /// Show the full composed prompt body.
    Full,
    /// Show summary first, then the full prompt body.
    SummaryAndFull,
    /// Show nothing at all.
    Silent,
}

/// Which section of the prompt body to render (when body is shown at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptBodyMode {
    /// Render the entire prompt.
    Full,
    /// Show only the first N lines, then truncate.
    Truncate { max_lines: usize },
    /// Show first N lines, a horizontal rule, then last M lines.
    FrontBack { front_lines: usize, back_lines: usize },
}
```

No existing code changes — purely additive.

#### Task 1.2 — Add `PromptVerbosityHint` enum

**File:** `lib/src/system_prompt/types.rs`

```rust
/// A named verbosity level extracted from CLI flags, ENV vars, or frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptVerbosityHint {
    Verbose,
    Quiet,
    Silent,
}
```

#### Task 1.3 — Add token estimation utility

**File:** `lib/src/system_prompt/types.rs` (or a new `lib/src/system_prompt/token.rs` if preferred)

```rust
/// Rough token estimate: ~4 chars per token (conservative for English prose).
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}
```

#### Task 1.4 — Add `resolve_system_prompt_report_style()` function

**File:** `lib/src/system_prompt/types.rs` (or new module)

This function implements the precedence chain from the spec:

```
CLI Switches > CLAUDINE_SYSTEM_PROMPT ENV > Frontmatter verbosity > Prompt Length > Default
```

```rust
/// Resolve the report style for the system prompt.
///
/// Precedence (highest wins):
/// 1. CLI switches (`--verbose`, `--quiet`, `--silent`)
/// 2. `CLAUDINE_SYSTEM_PROMPT` environment variable
/// 3. `verbosity` frontmatter key in `system-prompt.md`
/// 4. Prompt length (< 10 lines → never partial)
/// 5. Default: `Summary`
pub fn resolve_system_prompt_report_style(
    verbose: bool,
    quiet: bool,
    silent: bool,
    env_value: Option<&str>,
    frontmatter_verbosity: Option<&str>,
    prompt_line_count: usize,
) -> PromptReportStyle {
    // 1. --silent always wins
    if silent {
        return PromptReportStyle::Silent;
    }

    // 2. --verbose: Summary + Full
    if verbose {
        return PromptReportStyle::SummaryAndFull;
    }

    // 3. --quiet: Summary only
    if quiet {
        return PromptReportStyle::Summary;
    }

    // 4. CLAUDINE_SYSTEM_PROMPT env var
    if let Some(val) = env_value {
        match val.to_lowercase().as_str() {
            "verbose" => return PromptReportStyle::SummaryAndFull,
            "quiet" => return PromptReportStyle::Summary,
            "silent" => return PromptReportStyle::Silent,
            _ => {}
        }
    }

    // 5. Frontmatter verbosity
    if let Some(val) = frontmatter_verbosity {
        match val.to_lowercase().as_str() {
            "verbose" => return PromptReportStyle::SummaryAndFull,
            "quiet" => return PromptReportStyle::Summary,
            "silent" => return PromptReportStyle::Silent,
            _ => {}
        }
    }

    // 6. Default: Summary
    PromptReportStyle::Summary
}
```

**Tests:** Unit tests for every branch of the precedence chain.

#### Task 1.5 — Add `resolve_agent_prompt_body_mode()` function

**File:** `lib/src/system_prompt/types.rs` (or new module)

```rust
/// Resolve the body mode for the agent/user prompt.
///
/// The agent prompt defaults to showing up to 40 lines in full.
/// Beyond 40 lines, switches to FrontBack (first 20, last 10).
///
/// `--verbose` forces Full regardless of length.
pub fn resolve_agent_prompt_body_mode(
    prompt_line_count: usize,
    verbose: bool,
) -> PromptBodyMode {
    if verbose {
        return PromptBodyMode::Full;
    }
    if prompt_line_count <= 40 {
        PromptBodyMode::Full
    } else {
        PromptBodyMode::FrontBack {
            front_lines: 20,
            back_lines: 10,
        }
    }
}
```

**Tests:** Unit tests for boundary conditions (39 lines, 40 lines, 41 lines, verbose override).

---

### Phase 2: Frontmatter Verbosity Extraction (lib crate)

**Goal:** Parse the `verbosity` key from the system prompt file's frontmatter and carry it through the pipeline.

#### Task 2.1 — Extract verbosity from system prompt frontmatter

**File:** `lib/src/system_prompt/prepare.rs`

In `prepare_system_prompt_with_ctx()` and `prepare_system_prompt()`, after composing the markdown, parse frontmatter from `raw_text` to extract a `verbosity` hint. Store it on `PreparedSystemPrompt`:

**File:** `lib/src/system_prompt/types.rs`

Add field to `PreparedSystemPrompt`:

```rust
pub struct PreparedSystemPrompt {
    pub mode: SystemPromptMode,
    pub source: SystemPromptSource,
    pub raw_text: String,
    pub composed_markdown: String,
    pub non_interactive_appendix: Option<PreparedNonInteractiveAppendix>,
    // NEW: verbosity hint from frontmatter, if present
    pub frontmatter_verbosity: Option<String>,
}
```

In `prepare_system_prompt_with_ctx()`:

```rust
let frontmatter_verbosity = {
    let md: Markdown = raw_text.into();
    md.fm_get::<String>("verbosity").ok().flatten()
};
```

This uses the existing `Markdown::fm_get()` API from darkmatter. The `raw_text` is the un-composed source, so frontmatter is still present.

---

### Phase 3: Rewrite `log_system_prompt()` (CLI crate)

**Goal:** Replace the current full-body-always behavior with the new layered system.

#### Task 3.1 — Add `log_system_prompt_summary()` helper

**File:** `cli/src/output/mod.rs`

New function that renders the Summary view:

```rust
fn log_system_prompt_summary(
    prepared: &PreparedSystemPrompt,
    action: &str, // "appended to" | "replaced" | "unchanged"
    term: &Terminal,
) {
    // Render inside an orange BlockQuote:
    //   The system prompt was **{action}**; the content was _composed_ from
    //   {relative-path} ({token-message})
    // OR when no change:
    //   There was no change to the system prompt.
}
```

Key details:
- Use `Prose` for bold/italic/links
- Build an OSC8 hyperlink: `<a href="file://{absolute_path}">{relative_path}</a>`
- Compute relative path from CWD to the source file path
- Token message: `"Roughly {n} tokens were appended to the end of the prompt."` or `"The replacement system prompt is roughly {n} tokens."`
- Wrap in orange `BlockQuote` with `▌` border

#### Task 3.2 — Add `log_system_prompt_body()` helper

**File:** `cli/src/output/mod.rs`

New function for rendering the full/partial body:

```rust
fn log_system_prompt_body(
    prepared: &PreparedSystemPrompt,
    body_mode: PromptBodyMode,
    term: &Terminal,
) {
    // Render the composed_markdown through Darkmatter for_terminal()
    // Apply body_mode truncation logic
    // Wrap in orange BlockQuote
}
```

For `PromptBodyMode::Truncate { max_lines }`:
- Take first `max_lines` lines
- Render through Darkmatter with width constraint
- Append an HR marker

For `PromptBodyMode::FrontBack { front_lines, back_lines }`:
- Take first `front_lines` and last `back_lines`
- Strip leading blank lines from both sections (per spec)
- If first/last line of either section is blank, trim one line to land on valid content
- Render front section through Darkmatter
- Insert an HR divider (use `BlockQuote`'s built-in HR or a `---` rendered as dim)
- Render back section through Darkmatter

#### Task 3.3 — Rewrite `log_system_prompt()` to use the new system

**File:** `cli/src/output/mod.rs:217-300`

The function signature gains an `env_value: Option<&str>` parameter for `CLAUDINE_SYSTEM_PROMPT`:

```rust
pub(crate) fn log_system_prompt(
    effective_sp: &EffectiveSystemPrompt,
    verbose: bool,
    silent: bool,
    quiet: bool,
    env_value: Option<&str>,  // NEW: CLAUDINE_SYSTEM_PROMPT value
    term: &Terminal,
)
```

New behavior:

```
1. If silent → return immediately
2. Match on effective_sp:
   a. None / Disabled → only show when verbose && !quiet (unchanged behavior, just summary text)
   b. Ready(prepared):
      i.   Resolve PromptReportStyle via resolve_system_prompt_report_style()
      ii.  If Silent → return (overrides everything except --verbose)
      iii. Print Line 1: icon + "System Prompt" header with action label
           "📕 System Prompt(appended)"  /  "📕 System Prompt(replaced)"
      iv.  If Summary → render summary inside orange BlockQuote, return
      v.   If SummaryAndFull → render summary, then fall through to body
      vi.  If Full → determine body mode based on line count:
           - < 10 lines: Full
           - >= 10 lines: could use Full or FrontBack (spec says default is Summary, but
             when Full is explicitly requested, render everything)
      vii. Render body via log_system_prompt_body()
```

#### Task 3.4 — Update call sites

**Files:**
- `cli/src/commands/wrap/mod.rs:934` — pass `std::env::var("CLAUDINE_SYSTEM_PROMPT").ok().as_deref()`
- `cli/src/commands/wrap/composition/mod.rs:1456` — same

Read `CLAUDINE_SYSTEM_PROMPT` once during flag merge and thread it through, or read it at the call site. Simpler to read at the call site since it's an env var with no CLI flag equivalent.

---

### Phase 4: Rewrite `log_compose_prompt()` (CLI crate)

**Goal:** Change the agent/user prompt rendering to use the new system with 🗣️ icon, green border, and FrontBack truncation.

#### Task 4.1 — Update header and icon

**File:** `cli/src/output/mod.rs:155-215`

Change the header from `"Agent Prompt:"` to:

```
🗣️ Agent Prompt
```

Using Prose: `Prose::new("🗣️ <bold>Agent Prompt</bold>")`.

#### Task 4.2 — Implement FrontBack truncation for agent prompt

Replace the current 10-line truncate logic:

```rust
pub(crate) fn log_compose_prompt(
    prompt: &str,
    verbose: bool,
    term: &Terminal,
) {
    let line_count = prompt.lines().count();
    let body_mode = claudine::system_prompt::resolve_agent_prompt_body_mode(line_count, verbose);

    // ... render header ...
    // ... render body in green BlockQuote based on body_mode ...
    // ... FrontBack: front 20 lines + HR + last 10 lines ...
    // ... ensure no leading blank lines on front or back sections ...
}
```

FrontBack blank-line handling (per spec):
1. Strip all leading whitespace from the prompt
2. When splitting into front/back, trim leading blank lines from front and trailing blank lines from back
3. If after trimming, the front section's last line or back section's first line is blank, trim one additional line

#### Task 4.3 — Remove the old truncation notice

The existing `- remaining prompt truncated for brevity, use --verbose to show entire prompt` is replaced by the FrontBack HR marker between front and back sections. When using FrontBack, render a dim horizontal rule instead of the bullet-point notice.

---

### Phase 5: ENV Variable Support

**Goal:** Read `CLAUDINE_SYSTEM_PROMPT` env var and thread it through.

#### Task 5.1 — Read `CLAUDINE_SYSTEM_PROMPT` in the wrap command

**File:** `cli/src/commands/wrap/mod.rs`

After flag merge (~line 356), read the env var:

```rust
let system_prompt_env = std::env::var("CLAUDINE_SYSTEM_PROMPT").ok();
```

Pass to `log_system_prompt()`.

#### Task 5.2 — Update both call sites

Both `wrap/mod.rs:934` and `wrap/composition/mod.rs:1456` need to pass the env value.

---

### Phase 6: Tests

**Goal:** Comprehensive test coverage for all new behavior.

#### Task 6.1 — Lib crate unit tests

**File:** `lib/src/system_prompt/types.rs` (or new test module)

Test `resolve_system_prompt_report_style()`:
- `--silent` wins over everything
- `--verbose` → `SummaryAndFull`
- `--quiet` → `Summary`
- ENV `verbose`/`quiet`/`silent` when no CLI override
- Frontmatter `verbose`/`quiet`/`silent` when no CLI/ENV override
- Default → `Summary`
- CLI overrides ENV, ENV overrides frontmatter
- Unknown ENV/frontmatter values fall through to default
- Prompt length < 10 lines (currently unused in precedence but ready for future)

Test `resolve_agent_prompt_body_mode()`:
- ≤ 40 lines → `Full`
- > 40 lines → `FrontBack { front_lines: 20, back_lines: 10 }`
- `verbose=true` → `Full` regardless of length

Test `estimate_tokens()`:
- Empty string → 0
- Known-length string → expected result

#### Task 6.2 — CLI crate integration tests

**File:** `cli/tests/wrap_commands.rs` (or new test file)

Test `log_system_prompt()` behavior:
- `--silent`: nothing emitted
- `--quiet`: only summary BlockQuote (no full body)
- `--verbose`: summary + full body
- Default (no flags): summary only
- `CLAUDINE_SYSTEM_PROMPT=silent`: nothing emitted
- `CLAUDINE_SYSTEM_PROMPT=verbose`: summary + full body (without `--verbose` flag)
- Frontmatter `verbosity: silent`: nothing emitted

Test `log_compose_prompt()` behavior:
- Short prompt (≤ 40 lines): full body in green BlockQuote
- Long prompt (> 40 lines): front 20 + HR + last 10 in green BlockQuote
- `--verbose`: full body regardless of length
- FrontBack: no blank leading/trailing lines

---

## File Change Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `lib/src/system_prompt/types.rs` | Add | `PromptReportStyle`, `PromptBodyMode`, `PromptVerbosityHint` enums |
| `lib/src/system_prompt/types.rs` | Add | `estimate_tokens()`, `resolve_system_prompt_report_style()`, `resolve_agent_prompt_body_mode()` |
| `lib/src/system_prompt/types.rs` | Modify | Add `frontmatter_verbosity: Option<String>` to `PreparedSystemPrompt` |
| `lib/src/system_prompt/prepare.rs` | Modify | Extract `verbosity` from frontmatter during preparation |
| `cli/src/output/mod.rs` | Rewrite | `log_system_prompt()` — layered verbosity, summary/body split |
| `cli/src/output/mod.rs` | Add | `log_system_prompt_summary()`, `log_system_prompt_body()` |
| `cli/src/output/mod.rs` | Rewrite | `log_compose_prompt()` — icon, FrontBack, green border |
| `cli/src/commands/wrap/mod.rs` | Modify | Read `CLAUDINE_SYSTEM_PROMPT`, pass to `log_system_prompt()` |
| `cli/src/commands/wrap/composition/mod.rs` | Modify | Same env threading |
| `lib/src/system_prompt/mod.rs` | Modify | Re-export new types |

## Dependency Order

```
Phase 1 (types) ──────────────────────────────────┐
    │                                               │
Phase 2 (frontmatter extraction)                   │
    │                                               │
    ├──> Phase 3 (system prompt rendering) <───────┘
    │
    ├──> Phase 4 (agent prompt rendering)
    │
    ├──> Phase 5 (ENV variable)
    │
    └──> Phase 6 (tests)
```

Phases 3 and 4 can be parallelized after Phase 2 completes. Phase 5 is small and can be folded into Phase 3.

## Parallelizable Work

- **Phase 3** (system prompt) and **Phase 4** (agent prompt) are independent rendering rewrites
- **Task 1.4** and **Task 1.5** (resolution functions) can be written in parallel
- **Task 6.1** and **Task 6.2** (lib vs CLI tests) can be written in parallel

## Validation Checkpoints

1. **After Phase 1:** `cargo check -p claudine` — new types compile
2. **After Phase 2:** `cargo test -p claudine -- system_prompt` — frontmatter extraction works
3. **After Phase 3:** `cargo check -p claudine-cli` — system prompt rendering compiles
4. **After Phase 4:** `cargo check -p claudine-cli` — agent prompt rendering compiles
5. **After Phase 5:** Manual smoke test with `CLAUDINE_SYSTEM_PROMPT=verbose claudine claude "hello"`
6. **After Phase 6:** `cargo test -p claudine -- system_prompt && cargo test -p claudine-cli -- prompt`

## Open Questions / Design Decisions

1. **"unchanged" action label:** The spec mentions `action` can be `'appended' | 'replaced' | 'unchanged'`. Currently there's no "unchanged" variant — the system prompt is either None/Disabled/Ready. "unchanged" likely means "the same prompt was used as last time" which requires session history. For MVP, we should support `appended` and `replaced` only and add `unchanged` when session comparison is available.

2. **Body rendering when style is Full:** The spec says if prompt < 10 lines, never use Partial. When `PromptReportStyle::Full` is resolved (explicitly via frontmatter or ENV), should we still skip Partial even for long prompts? Yes — "Full" means show everything.

3. **Token estimation accuracy:** The spec says "roughly {#} tokens". A simple `len()/4` heuristic is sufficient. If more accuracy is needed later, integrate tiktoken or a similar tokenizer.

4. **`PromptReportStyle::Full` vs `PromptBodyMode::Full`:** These are separate concerns. `ReportStyle` controls *whether* the body is shown at all (summary vs full). `BodyMode` controls *how* the body is rendered (full text vs truncated vs frontback). When `ReportStyle::Full` is selected, `BodyMode` is always `Full` for system prompts (per spec, partial is only for agent prompts). The system prompt spec says: if prompt < 10 lines, never use partial. For system prompts, partial is never the default — it's always Summary or Full.

5. **FrontBack HR marker:** Use a dim horizontal rule (`───`) rendered through Darkmatter or Prose, not a markdown `---` which would render as an `<hr>` in HTML but as a divider line in terminal. The biscuit-terminal `Hr` component or a simple `Prose::new("<dim>────────────────────────────────────────</dim>")` would work.

6. **Line 1 icon alignment:** The spec says the BlockQuote vertical line should "align with the center of the icon found in the first line." Since 📕 is a single wide glyph and the `▌` border is 1 char + 1 space, the current left margin of 2 chars should provide reasonable alignment. Exact pixel alignment is not possible in terminal — this is a best-effort visual alignment.
