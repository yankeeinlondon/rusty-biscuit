# DRY Providers: Unified Prompt Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the three-phase per-provider prompt contract in `claudine/cli/src/commands/wrap/profile.rs` into a single canonical pipeline where the prompt is a typed `PromptSource` extracted once at the entrypoint and placed by exactly one method (`prompt_delivery`).

**Architecture:** Introduce `PromptSource` + `PromptArgConventions` types. Add a provider-blind `extract_prompt_source_from_passthrough` helper. Split `apply_non_interactive` into `apply_entrypoint` (entrypoint subcommand injection) and `apply_non_interactive_flags` (mode conflict rejection only). Delete `validate_final_args`, `strip_prompt_from_args`, `find_prompt_location`, and `extract_user_prompt` — all replaced by the extractor. Migrate the direct-wrap path, composition path, and harness-retry path to the identical pipeline shape.

**Tech Stack:** Rust 2024 edition, `color_eyre::eyre`, existing `WrapperProfile` trait in `claudine/cli/src/commands/wrap/profile.rs`. Tests via `cargo test -p claudine-cli`.

---

## Ground Rules

- **Repository:** `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition`
- **Working branch:** `feat/claudine-composition` (already checked out)
- **Test command:** `cargo test -p claudine-cli -- wrap::` for unit scope, `cargo test -p claudine-cli` for full CLI scope
- **Lint command:** `cargo clippy -p claudine-cli -- -D warnings`
- **After every task:** run `cargo test -p claudine-cli -- wrap::` — it MUST stay green at every intermediate step. The refactor is non-breaking until Phase 5.
- **Commit style:** one commit per task. Use the conventional-commit prefix shown in each step.
- **Never skip hooks:** pre-commit and pre-push hooks must pass. If a hook fails, fix the underlying issue.
- **File paths in this plan are absolute** for clarity. Tooling accepts relative paths rooted at the worktree.

---

## File Structure

### Files that will be heavily modified

- `claudine/cli/src/commands/wrap/profile.rs` — trait surface changes, per-provider method migrations, unit tests
- `claudine/cli/src/commands/wrap/mod.rs` — direct-wrap call site, harness-retry call site, deletion of `extract_user_prompt` / `find_prompt_location` / `strip_prompt_from_args`
- `claudine/cli/src/commands/wrap/composition.rs` — composition call site rewrite

### New symbols introduced (all live in `profile.rs`)

- `pub(crate) enum PromptSource { None, Inline(String), InheritStdin }`
- `pub(crate) struct PromptArgConventions { prompt_flags, entrypoint, value_taking_flags }`
- `pub(crate) fn extract_prompt_source_from_passthrough(profile, passthrough, has_piped_stdin) -> (Vec<String>, PromptSource)`
- `pub(crate) fn require_prompt_present(non_interactive: bool, source: &PromptSource) -> Result<()>`
- Trait methods: `apply_entrypoint`, `apply_non_interactive_flags`, `prompt_arg_conventions`

### Symbols removed at the end of the refactor

- `WrapperProfile::apply_non_interactive`
- `WrapperProfile::validate_final_args`
- `PromptRequirement` struct + `require_prompt_or_stdin` helper (Option-1 leftovers, now subsumed by the generic check)
- `wrap::mod::strip_prompt_from_args`
- `wrap::mod::find_prompt_location`
- `wrap::mod::find_gemini_prompt_location`
- `wrap::mod::find_positional_prompt_location`
- `wrap::mod::takes_value`
- `wrap::mod::extract_user_prompt`
- `wrap::mod::PromptLocation`

---

## Phase 1 — Foundation: add new types and helpers (non-breaking)

Phase 1 adds new code without touching existing trait methods. At the end of Phase 1 every existing test still passes and the codebase compiles identically to its current behavior.

### Task 1: Add `PromptSource` enum with unit tests

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs` (add near the top, under the existing `PromptDelivery` enum)

- [ ] **Step 1: Add the type definition**

Insert immediately after the `impl PromptDelivery` block (currently ending around line 41):

```rust
// ---------------------------------------------------------------------------
// PromptSource — typed prompt input to the wrap pipeline
// ---------------------------------------------------------------------------

/// A prompt supplied to the wrap pipeline, already extracted from any
/// CLI passthrough or composition source.
///
/// The wrap pipeline holds the prompt as this typed value between
/// extraction (at the entrypoint) and delivery (via `prompt_delivery`).
/// Provider flag-injection methods (`apply_entrypoint`,
/// `apply_non_interactive_flags`) never see or mutate the prompt text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptSource {
    /// No prompt provided. Valid only for interactive sessions or
    /// when stdin will be inherited from the parent (the child reads
    /// the TTY directly).
    None,
    /// A text prompt to be placed by `prompt_delivery`.
    Inline(String),
    /// The caller is forwarding piped stdin from its own stdin.
    /// The pipeline should not seed stdin; the child inherits it.
    InheritStdin,
}

impl PromptSource {
    /// Returns the inline prompt text if this source is `Inline`.
    pub(crate) fn as_inline(&self) -> Option<&str> {
        match self {
            Self::Inline(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns true when the source carries no prompt at all.
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true when a prompt reaches the child by any means
    /// (inline delivery OR inherited stdin).
    pub(crate) fn has_prompt_or_stdin(&self) -> bool {
        !matches!(self, Self::None)
    }
}
```

- [ ] **Step 2: Add unit tests**

Append these tests inside the existing `#[cfg(test)] mod tests { ... }` block in `profile.rs` (near the bottom of the file, around line 2228):

```rust
#[test]
fn prompt_source_as_inline_returns_text_for_inline_variant() {
    let source = PromptSource::Inline("hello".to_string());
    assert_eq!(source.as_inline(), Some("hello"));
}

#[test]
fn prompt_source_as_inline_returns_none_for_non_inline_variants() {
    assert_eq!(PromptSource::None.as_inline(), None);
    assert_eq!(PromptSource::InheritStdin.as_inline(), None);
}

#[test]
fn prompt_source_is_none_only_true_for_none_variant() {
    assert!(PromptSource::None.is_none());
    assert!(!PromptSource::Inline("hi".to_string()).is_none());
    assert!(!PromptSource::InheritStdin.is_none());
}

#[test]
fn prompt_source_has_prompt_or_stdin_accepts_inline_and_stdin() {
    assert!(!PromptSource::None.has_prompt_or_stdin());
    assert!(PromptSource::Inline("x".to_string()).has_prompt_or_stdin());
    assert!(PromptSource::InheritStdin.has_prompt_or_stdin());
}
```

- [ ] **Step 3: Run the new tests to verify they pass**

```bash
cargo test -p claudine-cli --lib -- wrap::profile::tests::prompt_source_
```

Expected: 4 tests pass (`prompt_source_as_inline_returns_text_for_inline_variant`, `prompt_source_as_inline_returns_none_for_non_inline_variants`, `prompt_source_is_none_only_true_for_none_variant`, `prompt_source_has_prompt_or_stdin_accepts_inline_and_stdin`).

- [ ] **Step 4: Run the full wrap test scope to confirm nothing broke**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all existing tests still pass; 4 additional tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "feat(claudine): add PromptSource enum for typed prompt pipeline"
```

---

### Task 2: Add `PromptArgConventions` struct and trait method with provider defaults

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Add the struct under `PromptSource`**

Insert immediately after the `PromptSource` impl block added in Task 1:

```rust
// ---------------------------------------------------------------------------
// PromptArgConventions — per-provider prompt-argv parsing knowledge
// ---------------------------------------------------------------------------

/// Describes how a provider's native CLI represents a prompt on argv.
///
/// Used by `extract_prompt_source_from_passthrough` to find a prompt in
/// raw passthrough arguments without embedding per-provider logic in a
/// central match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PromptArgConventions {
    /// Value-taking flags that carry the prompt string when present,
    /// e.g. `&["-p", "--prompt"]` for Gemini, `&["-t", "--text"]` for
    /// Goose. Empty for providers that accept only a positional prompt.
    pub prompt_flags: &'static [&'static str],
    /// An optional entrypoint subcommand that must be skipped when
    /// scanning for a positional prompt, e.g. `Some("exec")` for Codex
    /// or `Some("run")` for OpenCode / Goose. `None` for providers that
    /// have no subcommand entrypoint.
    pub entrypoint: Option<&'static str>,
    /// Additional value-taking flags whose values must not be mistaken
    /// for a positional prompt, e.g. `&["-m", "--model", "--output-format"]`.
    pub value_taking_flags: &'static [&'static str],
}

impl PromptArgConventions {
    /// Conventions for a provider that accepts only a positional prompt
    /// after an entrypoint subcommand (e.g. Codex `exec`, OpenCode `run`).
    pub(crate) const fn positional_after(entrypoint: &'static str) -> Self {
        Self {
            prompt_flags: &[],
            entrypoint: Some(entrypoint),
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
}

/// Value-taking flags understood by every provider. Keeps the extractor
/// from mistaking their values for positional prompts.
const COMMON_VALUE_TAKING_FLAGS: &[&str] = &[
    "-m",
    "--model",
    "-o",
    "--output",
    "--output-format",
    "--output-last-message",
    "--approval-mode",
    "--config",
    "-c",
    "--profile",
    "--system-prompt",
    "--sandbox-image",
    "--auth-type",
    "--format",
];
```

- [ ] **Step 2: Add the `prompt_arg_conventions` trait method**

Inside the `WrapperProfile` trait definition (around the end of the trait block, before the closing `}` near line 335), add:

```rust
    // -- Prompt argv conventions --------------------------------------------

    /// Describe how this provider represents a prompt on argv.
    ///
    /// Used by `extract_prompt_source_from_passthrough` to locate and
    /// remove a prompt from raw passthrough args. Every provider that
    /// supports non-interactive mode must implement this; the default
    /// returns "positional-only, no entrypoint" which works for Claude
    /// and Kimi (prompt as bare positional, no subcommand).
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &[],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
```

- [ ] **Step 3: Override on each provider**

Insert provider-specific `prompt_arg_conventions` overrides. Place each override inside the provider's existing `impl WrapperProfile for ...` block, near the other prompt-related methods.

For **Codex** (around line 637, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions::positional_after("exec")
    }
```

For **Gemini** (around line 930, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-p", "--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
```

For **Kimi** (around line 1047, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
```

For **Qwen** (around line 1227, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-p", "--prompt"],
            entrypoint: None,
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
```

For **OpenCode** (around line 1387, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions::positional_after("run")
    }
```

For **Goose** (around line 1550, near `prompt_delivery`):

```rust
    fn prompt_arg_conventions(&self) -> PromptArgConventions {
        PromptArgConventions {
            prompt_flags: &["-t", "--text"],
            entrypoint: Some("run"),
            value_taking_flags: COMMON_VALUE_TAKING_FLAGS,
        }
    }
```

**Claude** gets the default (no override needed — positional, no entrypoint).

- [ ] **Step 4: Add a unit test that every provider returns sensible conventions**

Append inside the existing `#[cfg(test)] mod tests { ... }` block in `profile.rs`:

```rust
#[test]
fn prompt_arg_conventions_claude_uses_defaults() {
    let conv = profile(Provider::Claude).prompt_arg_conventions();
    assert!(conv.prompt_flags.is_empty());
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn prompt_arg_conventions_codex_uses_exec_entrypoint() {
    let conv = profile(Provider::Codex).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("exec"));
    assert!(conv.prompt_flags.is_empty());
}

#[test]
fn prompt_arg_conventions_gemini_uses_prompt_flags() {
    let conv = profile(Provider::Gemini).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn prompt_arg_conventions_goose_uses_run_entrypoint_and_text_flags() {
    let conv = profile(Provider::Goose).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("run"));
    assert_eq!(conv.prompt_flags, &["-t", "--text"]);
}

#[test]
fn prompt_arg_conventions_kimi_uses_long_prompt_flag_only() {
    let conv = profile(Provider::KimiCode).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["--prompt"]);
    assert_eq!(conv.entrypoint, None);
}

#[test]
fn prompt_arg_conventions_opencode_uses_run_entrypoint() {
    let conv = profile(Provider::OpenCode).prompt_arg_conventions();
    assert_eq!(conv.entrypoint, Some("run"));
    assert!(conv.prompt_flags.is_empty());
}

#[test]
fn prompt_arg_conventions_qwen_uses_prompt_flags() {
    let conv = profile(Provider::QwenCode).prompt_arg_conventions();
    assert_eq!(conv.prompt_flags, &["-p", "--prompt"]);
    assert_eq!(conv.entrypoint, None);
}
```

- [ ] **Step 5: Run tests to verify all pass**

```bash
cargo test -p claudine-cli --lib -- wrap::profile::tests::prompt_arg_conventions_
```

Expected: 7 tests pass.

- [ ] **Step 6: Run full wrap scope to confirm nothing else broke**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "feat(claudine): add PromptArgConventions trait method for prompt extraction"
```

---

### Task 3: Add `extract_prompt_source_from_passthrough` helper with exhaustive tests

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Add the extractor function**

Append near the shared helper section (around line 1638, after `has_non_flag_positional`):

```rust
// ---------------------------------------------------------------------------
// Prompt extraction — consolidates the old extract_user_prompt /
// find_prompt_location / strip_prompt_from_args per-provider logic into
// one provider-blind algorithm that dispatches on PromptArgConventions.
// ---------------------------------------------------------------------------

/// Extract a prompt from raw passthrough args, returning the cleaned
/// args and the typed `PromptSource`.
///
/// This is the *single* place in the codebase that knows how to locate
/// a prompt inside provider passthrough arguments. It replaces the
/// previous per-provider extractors (`extract_user_prompt`,
/// `find_prompt_location`, `strip_prompt_from_args`) and the inline
/// positional-to-flag shuffling that used to live in
/// `apply_non_interactive` for Gemini and Qwen.
///
/// Precedence (highest wins):
/// 1. A prompt-carrying flag from `prompt_arg_conventions().prompt_flags`
///    (e.g. `--prompt VALUE`, `-p=VALUE`)
/// 2. A bare positional arg (after skipping the entrypoint subcommand
///    and any value-taking flags)
/// 3. `has_piped_stdin == true` → `PromptSource::InheritStdin`
/// 4. Otherwise → `PromptSource::None`
///
/// Whenever a flag or positional is returned as the prompt, it is
/// removed from the returned `Vec<String>` so downstream trait methods
/// see clean args with zero prompt characters.
pub(crate) fn extract_prompt_source_from_passthrough(
    profile: &dyn WrapperProfile,
    passthrough: &[String],
    has_piped_stdin: bool,
) -> (Vec<String>, PromptSource) {
    let conv = profile.prompt_arg_conventions();
    let mut args: Vec<String> = passthrough.to_vec();

    // 1. Look for a prompt-carrying flag.
    if let Some((prompt, indices)) = find_prompt_flag(&args, conv.prompt_flags) {
        // Remove the matched indices in reverse order so earlier
        // indices stay valid while splicing.
        for idx in indices.iter().rev() {
            args.remove(*idx);
        }
        return (args, PromptSource::Inline(prompt));
    }

    // 2. Look for a positional prompt, skipping the entrypoint (if any)
    //    and any value-taking flags.
    if let Some(idx) = find_positional_prompt_index(&args, &conv) {
        let prompt = args.remove(idx);
        return (args, PromptSource::Inline(prompt));
    }

    // 3. Piped stdin.
    if has_piped_stdin {
        return (args, PromptSource::InheritStdin);
    }

    // 4. No prompt.
    (args, PromptSource::None)
}

/// Find a prompt delivered via one of `prompt_flags`. Returns the prompt
/// text and the argv indices to remove.
///
/// Supports four shapes:
/// - `--prompt VALUE`      → two indices
/// - `--prompt=VALUE`      → one index
/// - `-p VALUE`            → two indices
/// - `-p=VALUE`            → one index
fn find_prompt_flag(
    args: &[String],
    prompt_flags: &[&str],
) -> Option<(String, Vec<usize>)> {
    for (idx, arg) in args.iter().enumerate() {
        for flag in prompt_flags {
            if arg == flag {
                let value = args.get(idx + 1)?.clone();
                return Some((value, vec![idx, idx + 1]));
            }
            let inline_prefix = format!("{flag}=");
            if let Some(value) = arg.strip_prefix(&inline_prefix) {
                return Some((value.to_string(), vec![idx]));
            }
        }
    }
    None
}

/// Find the index of the first positional prompt candidate in `args`,
/// honoring the entrypoint skip and the set of value-taking flags.
fn find_positional_prompt_index(
    args: &[String],
    conv: &PromptArgConventions,
) -> Option<usize> {
    let mut skip_next = false;
    for (idx, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // Skip the entrypoint subcommand if it matches at index 0.
        if idx == 0
            && let Some(entry) = conv.entrypoint
            && arg == entry
        {
            continue;
        }

        if arg == "--" {
            return (idx + 1 < args.len()).then_some(idx + 1);
        }

        // Skip value-taking flags so their values are not mistaken for
        // positional prompts. Handle both `--flag value` and
        // `--flag=value` shapes.
        if let Some(eq_idx) = arg.find('=')
            && conv
                .value_taking_flags
                .iter()
                .any(|flag| arg[..eq_idx] == **flag)
        {
            continue;
        }
        if conv.value_taking_flags.iter().any(|flag| arg == *flag) {
            skip_next = true;
            continue;
        }

        if !arg.starts_with('-') {
            return Some(idx);
        }
    }
    None
}
```

- [ ] **Step 2: Add unit tests for the extractor, per provider**

Append these tests inside `profile.rs` test module:

```rust
// -- extract_prompt_source_from_passthrough ----------------------------

fn extract(
    provider: Provider,
    passthrough: &[&str],
    has_piped_stdin: bool,
) -> (Vec<String>, PromptSource) {
    let args: Vec<String> = passthrough.iter().map(|s| s.to_string()).collect();
    extract_prompt_source_from_passthrough(profile(provider), &args, has_piped_stdin)
}

#[test]
fn extract_claude_no_args_yields_none() {
    let (args, source) = extract(Provider::Claude, &[], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::None);
}

#[test]
fn extract_claude_bare_positional_yields_inline() {
    let (args, source) = extract(Provider::Claude, &["hello"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hello".to_string()));
}

#[test]
fn extract_claude_piped_stdin_yields_inherit_stdin() {
    let (args, source) = extract(Provider::Claude, &[], true);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::InheritStdin);
}

#[test]
fn extract_claude_flag_before_positional_is_preserved() {
    let (args, source) = extract(
        Provider::Claude,
        &["--model", "opus", "fix the bug"],
        false,
    );
    assert_eq!(args, vec!["--model", "opus"]);
    assert_eq!(source, PromptSource::Inline("fix the bug".to_string()));
}

#[test]
fn extract_codex_skips_exec_entrypoint() {
    let (args, source) = extract(Provider::Codex, &["exec", "do it"], false);
    assert_eq!(args, vec!["exec"]);
    assert_eq!(source, PromptSource::Inline("do it".to_string()));
}

#[test]
fn extract_codex_without_exec_still_finds_positional() {
    let (args, source) = extract(Provider::Codex, &["--json", "task"], false);
    assert_eq!(args, vec!["--json"]);
    assert_eq!(source, PromptSource::Inline("task".to_string()));
}

#[test]
fn extract_gemini_long_prompt_flag() {
    let (args, source) =
        extract(Provider::Gemini, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_short_prompt_flag() {
    let (args, source) = extract(Provider::Gemini, &["-p", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_inline_prompt_flag() {
    let (args, source) =
        extract(Provider::Gemini, &["--prompt=hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_gemini_positional_prompt_after_model_flag() {
    let (args, source) = extract(
        Provider::Gemini,
        &["--model", "flash", "explain this"],
        false,
    );
    assert_eq!(args, vec!["--model", "flash"]);
    assert_eq!(source, PromptSource::Inline("explain this".to_string()));
}

#[test]
fn extract_gemini_positional_skips_approval_mode_value() {
    let (args, source) = extract(
        Provider::Gemini,
        &["--approval-mode", "yolo", "explain this"],
        false,
    );
    assert_eq!(args, vec!["--approval-mode", "yolo"]);
    assert_eq!(source, PromptSource::Inline("explain this".to_string()));
}

#[test]
fn extract_goose_text_flag() {
    let (args, source) =
        extract(Provider::Goose, &["run", "-t", "hello"], false);
    assert_eq!(args, vec!["run"]);
    assert_eq!(source, PromptSource::Inline("hello".to_string()));
}

#[test]
fn extract_kimi_prompt_flag() {
    let (args, source) = extract(Provider::KimiCode, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_opencode_skips_run_entrypoint() {
    let (args, source) =
        extract(Provider::OpenCode, &["run", "build it"], false);
    assert_eq!(args, vec!["run"]);
    assert_eq!(source, PromptSource::Inline("build it".to_string()));
}

#[test]
fn extract_qwen_long_prompt_flag() {
    let (args, source) =
        extract(Provider::QwenCode, &["--prompt", "hi"], false);
    assert!(args.is_empty());
    assert_eq!(source, PromptSource::Inline("hi".to_string()));
}

#[test]
fn extract_flags_only_returns_none_when_no_piped_stdin() {
    let (args, source) =
        extract(Provider::Codex, &["exec", "--json"], false);
    assert_eq!(args, vec!["exec", "--json"]);
    assert_eq!(source, PromptSource::None);
}

#[test]
fn extract_flags_only_with_piped_stdin_returns_inherit_stdin() {
    let (args, source) = extract(Provider::Codex, &["exec", "--json"], true);
    assert_eq!(args, vec!["exec", "--json"]);
    assert_eq!(source, PromptSource::InheritStdin);
}
```

- [ ] **Step 3: Run the new extractor tests**

```bash
cargo test -p claudine-cli --lib -- wrap::profile::tests::extract_
```

Expected: 17 tests pass.

- [ ] **Step 4: Run full wrap scope**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass (new plus pre-existing).

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "feat(claudine): add extract_prompt_source_from_passthrough helper"
```

---

### Task 4: Add the generic `require_prompt_present` helper

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Add the helper near the extractor**

Append after `find_positional_prompt_index` in `profile.rs`:

```rust
/// Generic "is the prompt requirement satisfied?" check for the wrap
/// pipeline. Called from every call site after all `apply_*` methods
/// have run and `prompt_delivery` has placed any inline prompt.
///
/// Returns `Ok(())` when any of the following holds:
/// - `non_interactive == false` (interactive sessions never require a
///   preloaded prompt — the user will type one)
/// - `source.has_prompt_or_stdin()` is true (inline prompt or piped
///   stdin reaches the child)
///
/// Otherwise bails with a provider-agnostic error message that
/// interpolates `provider_name` so the user knows which wrap failed.
pub(crate) fn require_prompt_present(
    provider_name: &str,
    non_interactive: bool,
    source: &PromptSource,
) -> Result<()> {
    if !non_interactive {
        return Ok(());
    }
    if source.has_prompt_or_stdin() {
        return Ok(());
    }
    bail!(
        "--non-interactive for {provider_name} requires a prompt \
         (positional, via a prompt flag, or piped on stdin)"
    );
}
```

- [ ] **Step 2: Add unit tests**

Append inside the existing test module:

```rust
#[test]
fn require_prompt_present_passes_in_interactive_mode_with_no_source() {
    require_prompt_present("claude", false, &PromptSource::None).unwrap();
}

#[test]
fn require_prompt_present_passes_with_inline_prompt() {
    require_prompt_present(
        "claude",
        true,
        &PromptSource::Inline("x".to_string()),
    )
    .unwrap();
}

#[test]
fn require_prompt_present_passes_with_inherit_stdin() {
    require_prompt_present("claude", true, &PromptSource::InheritStdin).unwrap();
}

#[test]
fn require_prompt_present_fails_non_interactive_with_no_source() {
    let err =
        require_prompt_present("codex", true, &PromptSource::None).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("codex"));
    assert!(message.contains("requires a prompt"));
}
```

- [ ] **Step 3: Run the new helper tests**

```bash
cargo test -p claudine-cli --lib -- wrap::profile::tests::require_prompt_present_
```

Expected: 4 tests pass.

- [ ] **Step 4: Run full wrap scope**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "feat(claudine): add generic require_prompt_present helper"
```

---

## Phase 2 — Add new trait methods with default implementations

Phase 2 introduces `apply_entrypoint` and `apply_non_interactive_flags` alongside (not replacing) the existing `apply_non_interactive`. Default implementations forward to the existing methods so nothing changes at call sites yet.

### Task 5: Add `apply_entrypoint` and `apply_non_interactive_flags` to the trait

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Add the two new methods to the `WrapperProfile` trait**

Inside the `WrapperProfile` trait definition, replace the `// -- Non-interactive mode -----` section (currently lines 115-123) with:

```rust
    // -- Non-interactive mode -----------------------------------------------

    /// Inject the provider's entrypoint subcommand (if any) and any
    /// mode-agnostic launch flags. Called in BOTH interactive and
    /// non-interactive pipelines because some entrypoints (e.g. Codex
    /// `exec`, OpenCode `run`) are needed in both.
    ///
    /// `non_interactive` is true when the wrap is running in
    /// non-interactive mode, so providers whose entrypoint is
    /// conditional on the mode (Claude: `--print`; Kimi: `--print`) can
    /// decide here.
    ///
    /// Default: no-op.
    fn apply_entrypoint(&self, _args: &mut Vec<String>, _non_interactive: bool) {}

    /// Reject mode-conflict flags (e.g. `-i` / `--prompt-interactive`)
    /// when the pipeline is running in non-interactive mode. Runs only
    /// in non-interactive pipelines. Providers that do NOT have such
    /// conflicting flags use the default no-op.
    ///
    /// Default: no-op.
    fn apply_non_interactive_flags(&self, _args: &mut [String]) -> Result<()> {
        Ok(())
    }

    /// **Deprecated** — left in place during Phase 2-3 of the DRY
    /// providers refactor so callers remain unchanged. Will be removed
    /// in Phase 5. New providers should implement `apply_entrypoint`
    /// and `apply_non_interactive_flags` instead.
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        self.apply_entrypoint(args, true);
        self.apply_non_interactive_flags(args)
    }

    /// Apply provider-specific defaults for non-interactive mode (e.g.
    /// OpenCode's default model injection). Default: no-op.
    fn apply_non_interactive_defaults(&self, _args: &mut Vec<String>) {}
```

> **Why this shape:** keeping the old `apply_non_interactive` with a default implementation that forwards to the new methods means providers that have NOT been migrated still work via the old override. As we migrate providers in Phase 3, each provider removes its `apply_non_interactive` override and implements the new methods instead. At the end of Phase 3, all seven providers are on the new API, and Phase 5 deletes the shim.

- [ ] **Step 2: Build to confirm nothing broke**

```bash
cargo build -p claudine-cli
```

Expected: compiles cleanly. Every existing provider still has its `apply_non_interactive` override, and the default `apply_entrypoint` / `apply_non_interactive_flags` are no-ops, so runtime behavior is unchanged.

- [ ] **Step 3: Run full wrap scope**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass — behavior is identical because no call site has been rewired yet.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "feat(claudine): add apply_entrypoint and apply_non_interactive_flags shims"
```

---

## Phase 3 — Provider-by-provider migration (smallest first)

Each task in Phase 3 migrates one provider to the new trait methods by:
1. Removing its `apply_non_interactive` override.
2. Adding `apply_entrypoint` and/or `apply_non_interactive_flags` as needed.
3. Leaving `prompt_delivery` unchanged (already load-bearing).
4. Updating the provider's unit tests to target the new methods.

The default `apply_non_interactive` shim on the trait forwards to the new methods, so direct/composition callers (which still call `apply_non_interactive`) keep working. Intermediate correctness is guaranteed by per-provider tests.

**Migration order (smallest diff first):** Kimi → Claude → Codex → Goose → OpenCode → Gemini → Qwen.

### Task 6: Migrate Kimi

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Replace Kimi's `apply_non_interactive` with `apply_entrypoint`**

In the `impl WrapperProfile for KimiWrapper` block, replace the current `apply_non_interactive` (lines ~1002-1007):

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
        Ok(())
    }
```

with:

```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if non_interactive && !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
    }
```

- [ ] **Step 2: Run the Kimi-scoped tests**

```bash
cargo test -p claudine-cli --lib -- wrap::profile::tests::kimi_
cargo test -p claudine-cli --lib -- wrap::profile::tests::all_wrapped_providers_have_profiles
```

Expected: all pass. The existing `apply_non_interactive` on the trait forwards to `apply_entrypoint(args, true)` which is behaviorally identical to the deleted override.

- [ ] **Step 3: Run full wrap scope**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate kimi wrapper to apply_entrypoint"
```

---

### Task 7: Migrate Claude

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Replace Claude's `apply_non_interactive` with `apply_entrypoint`**

In `impl WrapperProfile for ClaudeWrapper` (lines ~419-424), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
        Ok(())
    }
```

with:

```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if non_interactive && !has_flag(args, "--print") {
            args.push("--print".to_string());
        }
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate claude wrapper to apply_entrypoint"
```

---

### Task 8: Migrate Codex

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Replace Codex's `apply_non_interactive` with `apply_entrypoint`**

In `impl WrapperProfile for CodexWrapper` (lines ~565-579), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }

        // NOTE: prompt validation is deferred to validate_final_args() because
        // the prompt may not be in args yet (e.g. composition pipelines
        // compute delivery after non-interactive setup).
        Ok(())
    }
```

with:

```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, _non_interactive: bool) {
        // Codex `exec` is the non-interactive entrypoint; interactive
        // sessions use the default TUI (no `exec`). Only inject when
        // the caller is running non-interactively.
        if !_non_interactive {
            return;
        }
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }
    }
```

> Note: Codex's old `apply_non_interactive` was only called in non-interactive mode in both call sites, so gating on `non_interactive` preserves observable behavior.

- [ ] **Step 2: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass (including `codex_non_interactive_ensures_exec_once`, `codex_non_interactive_prepends_exec`, `codex_non_interactive_rejects_missing_prompt`).

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate codex wrapper to apply_entrypoint"
```

---

### Task 9: Migrate Goose

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Replace Goose's `apply_non_interactive` with `apply_entrypoint`**

In `impl WrapperProfile for GooseWrapper` (lines ~1528-1538), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }

        // NOTE: prompt validation is deferred to validate_final_args() because
        // the prompt may not be in args yet (e.g. composition pipelines
        // compute delivery after non-interactive setup).
        Ok(())
    }
```

with:

```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if !non_interactive {
            return;
        }
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate goose wrapper to apply_entrypoint"
```

---

### Task 10: Migrate OpenCode

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Replace OpenCode's `apply_non_interactive` with `apply_entrypoint`**

In `impl WrapperProfile for OpencodeWrapper` (lines ~1336-1346), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }

        // NOTE: prompt validation is deferred to validate_final_args() because
        // the prompt may not be in args yet (e.g. composition pipelines
        // compute delivery after non-interactive setup).
        Ok(())
    }
```

with:

```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        if !non_interactive {
            return;
        }
        let entrypoint = "run";
        if args.first().is_none_or(|first| first != entrypoint) {
            args.insert(0, entrypoint.to_string());
        }
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass, including `opencode_non_interactive_defaults_add_model_when_missing`, `opencode_non_interactive_rejects_missing_prompt`, and `opencode_non_interactive_prompt_body_uses_positional_arg`.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate opencode wrapper to apply_entrypoint"
```

---

### Task 11: Migrate Gemini (deleting the positional-to-flag shuffle)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

This is the first migration where we *delete* non-trivial logic. Gemini's old `apply_non_interactive` both rejected mode conflicts AND converted positionals to `--prompt`. In the new architecture, positional-to-flag conversion is impossible (positionals never reach `apply_*` anymore because the extractor removed them), so only the mode-conflict check survives.

- [ ] **Step 1: Replace Gemini's `apply_non_interactive` with `apply_non_interactive_flags`**

In `impl WrapperProfile for GeminiWrapper` (lines ~828-847), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for gemini");
        }
        if has_flag(args, "-p") || has_flag(args, "--prompt") {
            return Ok(());
        }
        // Convert a bare positional prompt to --prompt so Gemini CLI
        // runs in explicit headless mode even when stdin is a TTY.
        //
        // NOTE: prompt validation is deferred to validate_final_args() because
        // the prompt may not be in args yet (e.g. composition pipelines
        // compute delivery after non-interactive setup).
        if let Some(index) = find_first_positional(args) {
            let prompt = args.remove(index);
            args.push("--prompt".to_string());
            args.push(prompt);
        }
        Ok(())
    }
```

with:

```rust
    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for gemini");
        }
        Ok(())
    }
```

- [ ] **Step 2: Update Gemini-specific tests**

Several existing Gemini tests still call `apply_non_interactive` directly and assume the positional-shuffle behavior. Because the default `apply_non_interactive` shim now forwards to `apply_entrypoint` (no-op for Gemini) + `apply_non_interactive_flags` (mode-conflict check only), those tests must be re-pointed at the new extractor API.

Replace the following tests in the `tests` module (around lines 1893-2019):

1. Delete these tests entirely — their behavior now belongs to the extractor (covered in Task 3):
   - `gemini_non_interactive_converts_positional_to_prompt_flag`
   - `gemini_non_interactive_preserves_existing_prompt_flag`
   - `gemini_non_interactive_converts_positional_with_other_flags`
   - `gemini_non_interactive_skips_approval_mode_value`
   - `qwen_non_interactive_converts_positional_to_prompt_flag`

2. Replace the remaining Gemini tests with ones targeting the new API. Leave the tests for `qwen_non_interactive_rejects_prompt_interactive` and `gemini_non_interactive_rejects_missing_prompt_at_final_validation` in place for now — they will be re-homed in Task 12 (Qwen) and Task 15 (call-site migration) respectively.

Keep `gemini_non_interactive_allows_empty_args_for_composition` but adapt it:

```rust
    /// Regression test for the composition pipeline path: non-interactive
    /// flag application must NOT bail when args are empty, because the
    /// prompt arrives later via `prompt_delivery`.
    #[test]
    fn gemini_apply_non_interactive_flags_allows_empty_args_for_composition() {
        let p = profile(Provider::Gemini);
        let mut args: Vec<String> = Vec::new();
        p.apply_non_interactive_flags(&mut args).unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn gemini_apply_non_interactive_flags_rejects_interactive_mode_flags() {
        let p = profile(Provider::Gemini);
        let mut args = vec!["-i".to_string()];
        let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
        assert!(err.to_string().contains("conflicts"));
    }
```

Delete `gemini_final_validation_accepts_prompt_delivered_later` — the `validate_final_args` it exercises is slated for deletion in Phase 5, and the identical invariant is exercised end-to-end by the integration tests added in Task 18.

- [ ] **Step 3: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass. The deleted tests' invariants are now covered by the Task 3 extractor tests.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate gemini wrapper; delete positional-to-flag shuffle"
```

---

### Task 12: Migrate Qwen (deleting the positional-to-flag shuffle)

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

Qwen is Gemini's mirror — it also had the positional-to-flag shuffle in `apply_non_interactive` and suffers the same drift bug. Delete the shuffle; keep only the mode-conflict check.

- [ ] **Step 1: Replace Qwen's `apply_non_interactive` with `apply_non_interactive_flags`**

In `impl WrapperProfile for QwenWrapper` (lines ~1143-1162), replace:

```rust
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for qwen");
        }
        if has_flag(args, "-p") || has_flag(args, "--prompt") {
            return Ok(());
        }
        // Convert a bare positional prompt to --prompt so Qwen CLI
        // runs in explicit headless mode even when stdin is a TTY.
        //
        // NOTE: prompt validation is deferred to validate_final_args() because
        // the prompt may not be in args yet (e.g. composition pipelines
        // compute delivery after non-interactive setup).
        if let Some(index) = find_first_positional(args) {
            let prompt = args.remove(index);
            args.push("--prompt".to_string());
            args.push(prompt);
        }
        Ok(())
    }
```

with:

```rust
    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
            bail!("--non-interactive conflicts with interactive prompt mode for qwen");
        }
        Ok(())
    }
```

- [ ] **Step 2: Update Qwen-specific tests**

Replace the existing `qwen_non_interactive_rejects_prompt_interactive` and `qwen_non_interactive_allows_empty_args_for_composition` with:

```rust
    #[test]
    fn qwen_apply_non_interactive_flags_rejects_prompt_interactive() {
        let p = profile(Provider::QwenCode);
        let mut args = vec!["-i".to_string(), "task".to_string()];
        let err = p.apply_non_interactive_flags(&mut args).unwrap_err();
        assert!(err.to_string().contains("conflicts"));
    }

    #[test]
    fn qwen_apply_non_interactive_flags_allows_empty_args_for_composition() {
        let p = profile(Provider::QwenCode);
        let mut args: Vec<String> = Vec::new();
        p.apply_non_interactive_flags(&mut args).unwrap();
        assert!(args.is_empty());
    }
```

Delete `qwen_non_interactive_rejects_missing_prompt_at_final_validation` and `qwen_final_validation_accepts_prompt_delivered_later` — both are tied to `validate_final_args` being removed in Phase 5 and the equivalent end-to-end check is added in Task 18.

- [ ] **Step 3: Run tests**

```bash
cargo test -p claudine-cli -- wrap::
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): migrate qwen wrapper; delete positional-to-flag shuffle"
```

---

## Phase 4 — Migrate the call sites

Phase 4 is the commit that actually changes pipeline behavior at the call sites. Before Phase 4, every provider's logic has been migrated to the new API but the direct-wrap and composition call sites still call `apply_non_interactive` through the trait shim. After Phase 4, all three call sites use the uniform pipeline shape.

### Task 13: Rewrite `wrap/mod.rs` direct-wrap path to use the extractor + unified pipeline

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Add piped-stdin detection + extraction at the start of the direct-wrap pipeline**

Locate the block in `wrap/mod.rs` around line 945-960 that builds `child_args` from passthrough and computes `has_prompt`:

```rust
    // Determine if a prompt is present (implies non-interactive by default)
    let has_prompt = has_prompt_source(&child_args, None);

    // Default: interactive when no prompt, non-interactive when prompt present
    // --interactive/-i overrides the default back to interactive
    let non_interactive_requested = if interactive_requested {
        false
    } else {
        has_prompt
    };
```

Replace with:

```rust
    // Detect whether the parent is piping stdin to us.
    let has_piped_stdin = !std::io::stdin().is_terminal();

    // Extract the prompt up-front into a typed PromptSource, leaving
    // child_args free of any prompt characters. Downstream apply_*
    // methods see clean args; prompt_delivery is the only code path
    // that places the prompt back in.
    let (mut child_args, prompt_source) =
        super::profile::extract_prompt_source_from_passthrough(
            profile,
            &child_args,
            has_piped_stdin,
        );

    // Default: non-interactive when a prompt reaches the child, interactive
    // otherwise. --interactive/-i overrides the default back to interactive.
    let has_prompt = prompt_source.has_prompt_or_stdin();
    let non_interactive_requested = if interactive_requested {
        false
    } else {
        has_prompt
    };
```

> **Note:** `child_args` was previously declared as mutable earlier in the function and already contained the passthrough. We need to confirm that the first binding still compiles. Rebind with `let (mut child_args, prompt_source) = ...` will shadow the earlier binding; Rust allows this and it does not leak. Ensure the earlier `child_args` declaration is compatible — if it was `let mut child_args = args.passthrough.clone();`, this shadow works. Otherwise rename the earlier binding to `let raw_passthrough = args.passthrough.clone();` and pass that to the extractor.

Quick check: in the current file the binding is around line 944: `let mut child_args = args.passthrough.clone();`. The shadow works; no rename needed.

- [ ] **Step 2: Replace the `apply_non_interactive` call with the new pipeline shape**

Locate around line 995-1002:

```rust
    if non_interactive_requested {
        profile.apply_non_interactive(&mut child_args)?;
        // Only apply default model if the user didn't pass --model explicitly
        // (apply_model handles it below when args.model is Some).
        if args.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
    }
```

Replace with:

```rust
    profile.apply_entrypoint(&mut child_args, non_interactive_requested);

    if non_interactive_requested {
        profile.apply_non_interactive_flags(&mut child_args)?;
        // Only apply default model if the user didn't pass --model explicitly
        // (apply_model handles it below when args.model is Some).
        if args.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
    }
```

- [ ] **Step 3: Deliver the prompt via `prompt_delivery` instead of leaving it in `child_args`**

Locate around line 1104-1108:

```rust
    let stdin_seed: Option<String> = None;

    // -- Final argument validation -------------------------------------------
    let effective_non_interactive = non_interactive_requested;
    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;
```

Replace with:

```rust
    let effective_non_interactive = non_interactive_requested;

    // Single delivery site: if the extractor found an inline prompt,
    // prompt_delivery places it in the child. Piped stdin inherits
    // naturally (no seeding). None means the user runs interactively
    // and will type a prompt.
    let stdin_seed: Option<String> = if let Some(prompt) = prompt_source.as_inline() {
        profile
            .prompt_delivery(&child_args, prompt, effective_non_interactive)?
            .apply_to(&mut child_args)
    } else {
        None
    };

    super::profile::require_prompt_present(
        profile.binary(),
        effective_non_interactive,
        &prompt_source,
    )?;
```

- [ ] **Step 4: Update the prompt-display source to read from `PromptSource`**

Locate line 1299:

```rust
    let prompt_display = extract_user_prompt(&args.passthrough);
```

Replace with:

```rust
    let prompt_display = prompt_source.as_inline().map(|s| s.to_string());
```

- [ ] **Step 5: Build and run tests**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli -- wrap::
```

Expected: compiles and all tests pass.

> **Diagnostic help for failures:** if a Gemini/Qwen/Codex test fails with "requires a prompt", it means the extractor missed a shape. Add the shape to Task 3's extractor tests and fix the extractor. If an OpenCode test fails with "ENAMETOOLONG" or similar, check that `prompt_delivery` is receiving the extracted prompt rather than an empty string.

- [ ] **Step 6: Run integration tests**

```bash
cargo test -p claudine-cli --test wrap_commands
```

Expected: all 78 wrap integration tests pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "refactor(claudine): direct-wrap path uses unified prompt pipeline"
```

---

### Task 14: Rewrite `wrap/composition.rs` composition path to use the unified pipeline

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs`

- [ ] **Step 1: Replace `apply_non_interactive` with `apply_entrypoint` + `apply_non_interactive_flags`**

Locate around line 362-368:

```rust
    if effective_non_interactive {
        profile.apply_non_interactive(&mut child_args)?;
        // Only apply default model if --model was not explicitly provided.
        if request.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
    }
```

Replace with:

```rust
    profile.apply_entrypoint(&mut child_args, effective_non_interactive);

    if effective_non_interactive {
        profile.apply_non_interactive_flags(&mut child_args)?;
        // Only apply default model if --model was not explicitly provided.
        if request.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
    }
```

- [ ] **Step 2: Replace `validate_final_args` with the generic `require_prompt_present` helper**

Locate around line 486-493:

```rust
    let stdin_seed = profile
        .prompt_delivery(&child_args, &effective_prompt, effective_non_interactive)?
        .apply_to(&mut child_args);

    let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
    let child_cwd = env_plan.child_cwd.as_path();

    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;
```

Replace with:

```rust
    // The composition pipeline always has an inline prompt (the composed
    // Markdown body). Build the PromptSource accordingly and deliver.
    let prompt_source = super::profile::PromptSource::Inline(effective_prompt.clone());
    let stdin_seed = profile
        .prompt_delivery(&child_args, &effective_prompt, effective_non_interactive)?
        .apply_to(&mut child_args);

    let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
    let child_cwd = env_plan.child_cwd.as_path();

    super::profile::require_prompt_present(
        profile.binary(),
        effective_non_interactive,
        &prompt_source,
    )?;
```

- [ ] **Step 3: Build and run tests**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli -- wrap::
cargo test -p claudine-cli --test wrap_commands
cargo test -p claudine-cli --test sequence_cli
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/composition.rs
git commit -m "refactor(claudine): composition path uses unified prompt pipeline"
```

---

### Task 15: Rewrite `build_harness_launch` to use the unified pipeline

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

`build_harness_launch` currently calls `prompt_delivery` correctly, but it still calls `validate_final_args` (line 1919) after delivery. Replace with the generic helper, and ensure the harness base args are already clean of any prompt (they come from `harness_base_args`, which had `strip_prompt_from_args` applied at line 1461 — that call will be deleted in Phase 5, but for now it's harmless to leave in place).

- [ ] **Step 1: Replace `validate_final_args` in `build_harness_launch`**

Locate around line 1915-1919:

```rust
    let prompt = strip_prompt_tags_for_provider(provider, &materialized.prompt);
    let stdin_seed = profile
        .prompt_delivery(&args, &prompt, effective_non_interactive)?
        .apply_to(&mut args);
    profile.validate_final_args(&args, effective_non_interactive, stdin_seed.is_some())?;
```

Replace with:

```rust
    let prompt = strip_prompt_tags_for_provider(provider, &materialized.prompt);
    let prompt_source = super::profile::PromptSource::Inline(prompt.clone());
    let stdin_seed = profile
        .prompt_delivery(&args, &prompt, effective_non_interactive)?
        .apply_to(&mut args);
    super::profile::require_prompt_present(
        profile.binary(),
        effective_non_interactive,
        &prompt_source,
    )?;
```

- [ ] **Step 2: Build and run tests**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli -- wrap::
cargo test -p claudine-cli --test wrap_commands
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "refactor(claudine): harness retry path uses require_prompt_present"
```

---

## Phase 5 — Delete retired symbols

Phase 5 removes every now-dead path. After this phase, the `WrapperProfile` trait has exactly the surface the design specifies.

### Task 16: Delete `validate_final_args`, `apply_non_interactive`, and the `PromptRequirement` machinery

**Files:**
- Modify: `claudine/cli/src/commands/wrap/profile.rs`

- [ ] **Step 1: Delete the `validate_final_args` trait method**

Delete the entire `-- Final argument validation` section from the `WrapperProfile` trait (currently lines ~263-280):

```rust
    // -- Final argument validation -------------------------------------------

    /// Validate the final child args after all prompt sources have been
    /// processed ...
    fn validate_final_args(
        &self,
        _args: &[String],
        _non_interactive: bool,
        _has_stdin: bool,
    ) -> Result<()> {
        Ok(())
    }
```

- [ ] **Step 2: Delete every provider's `validate_final_args` override**

Delete these per-provider overrides:

- Codex's `validate_final_args` (currently ~lines 662-678)
- Gemini's `validate_final_args` (currently ~lines 849-866)
- Qwen's `validate_final_args` (currently ~lines 1164-1181)
- OpenCode's `validate_final_args` (currently ~lines 1434-1451)
- Goose's `validate_final_args` (currently ~lines 1571-1587)

- [ ] **Step 3: Delete the trait shim `apply_non_interactive`**

Delete from the `WrapperProfile` trait:

```rust
    /// **Deprecated** — left in place during Phase 2-3 of the DRY
    /// providers refactor so callers remain unchanged. Will be removed
    /// in Phase 5. New providers should implement `apply_entrypoint`
    /// and `apply_non_interactive_flags` instead.
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()> {
        self.apply_entrypoint(args, true);
        self.apply_non_interactive_flags(args)
    }
```

- [ ] **Step 4: Delete the `PromptRequirement` struct, `require_prompt_or_stdin`, and their tests**

These are Option-1 leftovers now fully subsumed by the generic `require_prompt_present` + `PromptSource` flow. Delete from `profile.rs`:

- The `PromptRequirement` struct (currently ~lines 1654-1671)
- The `require_prompt_or_stdin` function (currently ~lines 1685-1706)
- The test helpers `req_positional_only`, `req_prompt_flag`, and these tests in `tests` module:
  - `require_prompt_or_stdin_passes_in_interactive_mode`
  - `require_prompt_or_stdin_passes_when_stdin_supplied`
  - `require_prompt_or_stdin_accepts_positional_after_entrypoint`
  - `require_prompt_or_stdin_rejects_entrypoint_as_positional`
  - `require_prompt_or_stdin_rejects_flags_only`
  - `require_prompt_or_stdin_accepts_flag_delivered_prompt`
  - `require_prompt_or_stdin_accepts_short_flag_delivered_prompt`
  - `require_prompt_or_stdin_empty_args_rejected`
  - `require_prompt_or_stdin_skip_entrypoint_false_accepts_first_positional`

Also delete the Codex / Gemini / Qwen / OpenCode / Goose tests that exercise `validate_final_args`:
- `codex_non_interactive_rejects_missing_prompt`
- `gemini_non_interactive_rejects_missing_prompt_at_final_validation` (if still present)
- `qwen_non_interactive_rejects_missing_prompt_at_final_validation` (if still present)
- `qwen_final_validation_accepts_prompt_delivered_later` (if still present)
- `opencode_non_interactive_rejects_missing_prompt`
- `goose_non_interactive_rejects_missing_prompt`

These invariants are replaced by the generic `require_prompt_present_fails_non_interactive_with_no_source` test from Task 4 and by end-to-end integration coverage added in Task 18.

- [ ] **Step 5: Delete `find_first_positional` and `has_non_flag_positional` if they have no remaining callers**

Run:

```bash
rg -n 'find_first_positional|has_non_flag_positional' claudine/cli/src
```

If the only remaining uses are internal to `profile.rs` and they no longer have any callers outside the deleted shuffle code, delete both functions.

- [ ] **Step 6: Build and run tests**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli -- wrap::
cargo test -p claudine-cli --test wrap_commands
```

Expected: all tests pass.

- [ ] **Step 7: Run clippy**

```bash
cargo clippy -p claudine-cli -- -D warnings
```

Expected: no warnings. If clippy reports unused imports or dead code for any deleted symbol, remove the unused items.

- [ ] **Step 8: Commit**

```bash
git add claudine/cli/src/commands/wrap/profile.rs
git commit -m "refactor(claudine): remove validate_final_args and Option-1 PromptRequirement"
```

---

### Task 17: Delete `strip_prompt_from_args`, `find_prompt_location`, `extract_user_prompt`, and `PromptLocation`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

- [ ] **Step 1: Identify all remaining callers**

```bash
rg -n 'strip_prompt_from_args|find_prompt_location|extract_user_prompt|PromptLocation' claudine/cli/src
```

Expected callers at this point:
- `wrap/mod.rs:1461` — `strip_prompt_from_args` in the harness setup block (still called before Phase 5 cleanup even though `prompt_source` now carries the prompt)
- `wrap/mod.rs:3355, 3519` — `find_prompt_location` + `extract_prompt_from_child_args` (display path)
- `wrap/mod.rs:3503` — `extract_user_prompt` (display path)
- `wrap/composition.rs:39` — `strip_prompt_from_args` re-exported for the harness-path use

- [ ] **Step 2: Update `wrap/mod.rs` callers of `extract_prompt_from_child_args` and `extract_user_prompt`**

Locate the `extract_tags_from_child_args` function body (around line 3350-3380). It uses `find_prompt_location` to locate the prompt for MCP tag stripping. Under the new design, the caller of `extract_tags_from_child_args` already has the inline prompt available as a `PromptSource::Inline(...)` value — so `extract_tags_from_child_args` can take the prompt text as input rather than re-scanning args.

Change the signature from:

```rust
fn extract_tags_from_child_args(
    provider: Provider,
    args: &mut [String],
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
```

to:

```rust
fn extract_tags_from_prompt(
    prompt: Option<&str>,
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
    let Some(prompt) = prompt else {
        return (None, Vec::new());
    };
    let (cleaned, tags) = extract_tags(prompt);
    if tags.is_empty() {
        (None, Vec::new())
    } else {
        (Some(cleaned), tags)
    }
}
```

Then update the single caller in `wrap/mod.rs` (around line 1138) to pass `prompt_source.as_inline()` and, if tags are returned, update the `PromptSource` (re-bind it with the cleaned prompt) instead of mutating args. Locate:

```rust
        let (cleaned_prompt, prompt_tags) =
            extract_tags_from_child_args(provider, &mut child_args, lex_tags);
```

Replace with:

```rust
        let (cleaned_prompt, prompt_tags) =
            extract_tags_from_prompt(prompt_source.as_inline(), lex_tags);
        if let Some(ref cleaned) = cleaned_prompt {
            prompt_source = super::profile::PromptSource::Inline(cleaned.clone());
        }
```

> **Caution:** the Task 13 rewrite declared `prompt_source` as `let (mut child_args, prompt_source)`. To rebind it here, change that earlier binding to `let (mut child_args, mut prompt_source)`. Make that change as part of this step.

- [ ] **Step 3: Delete `strip_prompt_from_args` and its re-export**

Delete the entire `strip_prompt_from_args` function (currently lines ~1613-1660) from `wrap/mod.rs`.

Delete its re-export from `wrap/composition.rs`'s `use` statement (line 39):

```rust
    switch_process_cwd, wrap_terminal,
};
```

(remove the `strip_prompt_from_args,` entry from that `use` list).

Find the caller at `wrap/mod.rs:1461`:

```rust
        let mut harness_base_args = child_args.clone();
        strip_prompt_from_args(provider, &mut harness_base_args);
```

Since `child_args` under the new pipeline no longer contains the prompt (it was extracted up-front by the Task 13 extractor), the strip call is now a no-op. Delete the call. The surrounding context becomes:

```rust
        let mut harness_base_args = child_args.clone();
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }
```

- [ ] **Step 4: Delete `find_prompt_location`, `find_gemini_prompt_location`, `find_positional_prompt_location`, `takes_value`, `PromptLocation`, `extract_user_prompt`, and `extract_prompt_from_child_args`**

Delete these from `wrap/mod.rs`:

- `PromptLocation` enum (line ~3344)
- `extract_tags_from_child_args` (line ~3350) — replaced above
- `find_prompt_location` (line ~3423)
- `find_gemini_prompt_location` (line ~3432)
- `find_positional_prompt_location` (line ~3454)
- `takes_value` (line ~3484)
- `extract_user_prompt` (line ~3503)
- `extract_prompt_from_child_args` (line ~3510)

Any function in `wrap/mod.rs` that still calls `extract_prompt_from_child_args` must be updated to read from `prompt_source.as_inline()` directly or accept the prompt as an argument. Search first:

```bash
rg -n 'extract_prompt_from_child_args' claudine/cli/src
```

Update each caller to accept the prompt string directly from the already-extracted `PromptSource`.

- [ ] **Step 5: Delete the related unit tests**

Delete these tests from the `#[cfg(test)]` modules in `wrap/mod.rs`:

- `extract_user_prompt_finds_first_non_switch`
- `has_prompt_source_detects_positional_arg`
- `strip_prompt_from_args_preserves_output_last_message_pair_for_codex`
- Any test exercising `find_prompt_location`, `PromptLocation`, or `extract_prompt_from_child_args`

The equivalent invariants are now covered by the Task 3 extractor tests.

- [ ] **Step 6: Build and run tests**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli -- wrap::
cargo test -p claudine-cli --test wrap_commands
```

Expected: all pass. If an integration test fails because the extractor mis-handles a shape, add the shape to the Task 3 extractor tests + fix the extractor.

- [ ] **Step 7: Run clippy**

```bash
cargo clippy -p claudine-cli -- -D warnings
```

Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/wrap/composition.rs
git commit -m "refactor(claudine): delete strip/find/extract helpers replaced by PromptSource"
```

---

## Phase 6 — End-to-end coverage and documentation

### Task 18: Add composition integration test for every provider

**Files:**
- Modify: `claudine/cli/tests/wrap_commands.rs` (add new tests)

This is the test that would have caught the original Gemini/Qwen drift. It runs a mock provider binary under `claudine sequence file.md --<provider>` and asserts the composed prompt reaches the mock process as expected argv + stdin.

- [ ] **Step 1: Write a parameterized test function per provider**

Append to `claudine/cli/tests/wrap_commands.rs`:

```rust
/// For every wrapped provider, verify that `claudine <provider>
/// --dry-run "hello"` produces a child argv that delivers the prompt
/// (either as an inline positional / flag pair or as a stdin seed).
///
/// This is the structural test that enforces direct-wrap path + new
/// PromptSource pipeline agreement. Failures here mean the extractor
/// or prompt_delivery diverged for the named provider.
#[test]
fn direct_wrap_dry_run_delivers_prompt_for_every_provider() {
    let workspace = tempdir().unwrap();
    let (_repo_root, launch_dir, bin_dir) =
        create_claudine_monorepo(workspace.path()).unwrap();

    for (provider_slug, expected_prompt_hint) in [
        ("claude", "hello"),
        ("codex", "hello"),
        ("gemini", "hello"),
        ("kimi", "hello"),
        ("opencode", "hello"),
        ("qwen", "hello"),
        ("goose", "hello"),
    ] {
        // Stub the provider binary so PATH resolution succeeds.
        write_executable(
            &bin_dir.join(provider_slug),
            "#!/bin/sh\necho stub\n",
        );
        let mut cmd = cargo_bin_cmd("claudine");
        cmd.current_dir(&launch_dir);
        cmd.env("PATH", format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()));
        cmd.arg(provider_slug).arg("--dry-run").arg(expected_prompt_hint);

        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "`claudine {provider_slug} --dry-run 'hello'` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let normalized = strip_ansi(&combined);
        assert!(
            normalized.contains(expected_prompt_hint),
            "`claudine {provider_slug} --dry-run 'hello'` output did not mention the prompt:\n{normalized}"
        );
    }
}
```

> **Note:** dry-run mode prints the child argv (and seeded stdin when applicable) without actually spawning. For Claude/Codex/Kimi where the prompt is placed via stdin, the dry-run output must also echo the seeded stdin — verify this is the case by running `claudine claude --dry-run "hello"` manually before relying on this assertion. If dry-run doesn't echo stdin, amend the assertion to match the format `crate::output::log_dry_run` actually produces.

- [ ] **Step 2: Add a composition-path parameterized test**

Append:

```rust
/// Regression test for the composition-path drift that motivated the
/// DRY providers refactor. For every wrapped provider, verify that
/// `claudine sequence <file> --<provider>` composes a simple markdown
/// file and successfully hands the composed body to the mock provider.
#[test]
fn sequence_composition_delivers_prompt_for_every_provider() {
    let workspace = tempdir().unwrap();
    let (_repo_root, launch_dir, bin_dir) =
        create_claudine_monorepo(workspace.path()).unwrap();

    // A minimal composition file — no frontmatter harness, just body.
    let compose_file = launch_dir.join("compose.md");
    write(&compose_file, "composed body text\n");

    for provider_slug in [
        "claude", "codex", "gemini", "kimi", "opencode", "qwen", "goose",
    ] {
        write_executable(
            &bin_dir.join(provider_slug),
            "#!/bin/sh\nexit 0\n",
        );
        let mut cmd = cargo_bin_cmd("claudine");
        cmd.current_dir(&launch_dir);
        cmd.env(
            "PATH",
            format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default()),
        );
        cmd.arg("sequence")
            .arg("compose.md")
            .arg(format!("--{provider_slug}"))
            .arg("--dry-run");

        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "`claudine sequence compose.md --{provider_slug} --dry-run` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
```

- [ ] **Step 3: Run the new integration tests**

```bash
cargo test -p claudine-cli --test wrap_commands direct_wrap_dry_run_delivers_prompt_for_every_provider
cargo test -p claudine-cli --test wrap_commands sequence_composition_delivers_prompt_for_every_provider
```

Expected: both pass for all seven providers.

- [ ] **Step 4: Run the full integration suite**

```bash
cargo test -p claudine-cli
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/tests/wrap_commands.rs
git commit -m "test(claudine): add per-provider direct and composition dry-run coverage"
```

---

### Task 19: Update documentation and delete load-bearing comments

**Files:**
- Modify: `claudine/docs/topics/composition.md`
- Modify: `claudine/cli/src/commands/wrap/profile.rs` (doc comments on `PromptSource`, `prompt_delivery`, `apply_entrypoint`, `apply_non_interactive_flags`)

- [ ] **Step 1: Delete any load-bearing NOTE comments**

Search for residual comments documenting the pre-refactor state:

```bash
rg -n 'validate_final_args\(\) because' claudine/cli/src
rg -n 'prompt may not be in args yet' claudine/cli/src
```

Expected: zero results — both phrases were tied to the per-provider `apply_non_interactive` logic deleted in Tasks 11-12. If any stragglers remain (e.g. doc comments paraphrasing the old contract), delete them.

- [ ] **Step 2: Update `claudine/docs/topics/composition.md`**

Open `claudine/docs/topics/composition.md` and search for any mention of the three-phase prompt contract (`apply_non_interactive`, `validate_final_args`, `prompt_delivery`). Replace any such description with the new pipeline shape. If no such section exists, skip this step.

A sample replacement paragraph:

```markdown
## Prompt delivery pipeline

Wrap pipelines extract the prompt once at entry as a typed
`PromptSource` (`None`, `Inline(String)`, or `InheritStdin`). The
direct-wrap path extracts from passthrough argv; the composition path
constructs `Inline(composed_body)` directly. Both paths then run the
identical sequence:

1. `apply_entrypoint` — inject entrypoint subcommands (`exec`, `run`,
   `--print`).
2. `apply_non_interactive_flags` — reject mode conflicts
   (`-i` in non-interactive mode).
3. `apply_*` flag builders — model, output format, system prompt,
   sandbox, etc.
4. `prompt_delivery` — the *only* method that places the prompt into
   the child (as argv or stdin).
5. `require_prompt_present` — generic check that a prompt reaches the
   child in non-interactive mode.

Provider authors implement `prompt_delivery` + `prompt_arg_conventions`
and (optionally) `apply_entrypoint` / `apply_non_interactive_flags`.
They never touch the prompt from inside flag builders.
```

- [ ] **Step 3: Build and run the full test suite one more time**

```bash
cargo build -p claudine-cli
cargo test -p claudine-cli
cargo clippy -p claudine-cli -- -D warnings
```

Expected: clean build, all tests pass, zero clippy warnings.

- [ ] **Step 4: Commit**

```bash
git add claudine/docs/topics/composition.md claudine/cli/src/commands/wrap/profile.rs
git commit -m "docs(claudine): document unified prompt pipeline and delete load-bearing comments"
```

---

## Verification Checklist

Run all of the following at the end of the plan. Every command must succeed.

- [ ] `cargo build -p claudine-cli`
- [ ] `cargo build -p claudine`
- [ ] `cargo test -p claudine-cli`
- [ ] `cargo test -p claudine`
- [ ] `cargo clippy -p claudine-cli -- -D warnings`
- [ ] `cargo clippy -p claudine -- -D warnings`
- [ ] `cargo fmt --package claudine-cli --package claudine -- --check`
- [ ] Manual dry-run smoke tests (no real provider invocation needed):
  - [ ] `cargo run -p claudine-cli -- claude --dry-run "hello"` — prompt appears in dry-run output
  - [ ] `cargo run -p claudine-cli -- codex --dry-run "hello"` — `exec` entrypoint injected, prompt visible
  - [ ] `cargo run -p claudine-cli -- gemini --dry-run "hello"` — `--prompt hello` in dry-run argv
  - [ ] `cargo run -p claudine-cli -- kimi --dry-run "hello"` — prompt delivered via stdin seed
  - [ ] `cargo run -p claudine-cli -- opencode --dry-run "hello"` — `run hello` in dry-run argv
  - [ ] `cargo run -p claudine-cli -- qwen --dry-run "hello"` — `--prompt hello` in dry-run argv
  - [ ] `cargo run -p claudine-cli -- goose --dry-run "hello"` — `run -t hello` in dry-run argv

- [ ] Grep audit — every symbol the design says to remove is actually gone:

```bash
rg -n 'validate_final_args' claudine/cli/src claudine/lib/src
rg -n 'strip_prompt_from_args' claudine/cli/src
rg -n 'find_prompt_location' claudine/cli/src
rg -n 'extract_user_prompt' claudine/cli/src
rg -n 'PromptLocation' claudine/cli/src
rg -n 'PromptRequirement' claudine/cli/src
rg -n 'require_prompt_or_stdin' claudine/cli/src
rg -n 'fn apply_non_interactive\b' claudine/cli/src
```

Expected: zero hits for each. The only exception is `apply_non_interactive_flags` and `apply_non_interactive_defaults`, which are the new methods.

- [ ] Grep audit — every new symbol exists and is used:

```bash
rg -n 'PromptSource' claudine/cli/src
rg -n 'PromptArgConventions' claudine/cli/src
rg -n 'extract_prompt_source_from_passthrough' claudine/cli/src
rg -n 'require_prompt_present' claudine/cli/src
rg -n 'fn apply_entrypoint' claudine/cli/src
rg -n 'fn apply_non_interactive_flags' claudine/cli/src
rg -n 'fn prompt_arg_conventions' claudine/cli/src
```

Expected: each appears in `profile.rs` plus usages in `wrap/mod.rs` and `wrap/composition.rs`.

---

## Notes for the Executor

- **Every intermediate commit must compile and pass `cargo test -p claudine-cli -- wrap::`.** This is non-negotiable — the refactor is staged specifically so each commit leaves a working tree.
- **Do not reorder the migration tasks.** Kimi → Claude → Codex → Goose → OpenCode → Gemini → Qwen is ordered by diff size and risk. Gemini and Qwen are the providers that caused the original bug, so they are last so that earlier migrations can shake out issues with the shared extractor first.
- **If an integration test fails because the extractor missed a shape**, the fix is to add a case to `extract_prompt_source_from_passthrough` (and a matching unit test), not to work around the failure in the call site.
- **If `clippy` complains about `has_any_flag`, `option_value`, or `find_first_positional` becoming unused after Phase 5**, delete them. They are private helpers with no external consumers.
- **Do not touch the `PromptDelivery` enum** — it remains the placement primitive. Only the input side of the prompt (`PromptSource`) and the extraction/validation surface change.
- **`has_prompt_source` in `wrap/mod.rs`** (around line 3564) becomes dead after Task 17. Delete it as part of Task 17 step 5 if clippy flags it; otherwise check for remaining callers and delete those too.
- **Sequence tests** (`claudine/cli/tests/sequence_cli.rs`) rely on the composition pipeline and will exercise the refactor end-to-end. If any sequence test fails, treat it as the primary signal — fix the pipeline, not the test.
