# Plan: Harden `--frontmatter-prompt` workflow

## Context

The `--frontmatter-prompt` (aka `--fp`) feature extracts a `prompt` from a markdown file's frontmatter, sends it to an agent, and writes the agent's response back as the file body. Currently there are several issues:

1. **Agent output bleeds into the file** — STDOUT/STDERR from the agent (thoughts, metadata) gets written into the body instead of just the intended content
2. **No guardrail enforcement** — if the agent modifies frontmatter despite being asked not to, no recovery happens
3. **No post-run validation** — success is reported even if the body wasn't actually updated
4. **No configurable guardrail file** — the guardrails are hardcoded in Rust; users can't customize them
5. **Missing summary detection** — when the agent provides no STDOUT summary, the user gets no feedback

## Changes

### 1. Create `.claudine/frontmatter-prompt.md` template system

**New file to create**: `claudine/lib/src/composition/guardrails.rs`

- Define `const DEFAULT_FRONTMATTER_PROMPT_GUARDRAILS` with the user's template content
- Add `fn load_or_create_guardrails(repo_root: Option<&Path>) -> String`:
    - Check for `{repo_root}/.claudine/frontmatter-prompt.md`
    - If it exists, read and return its contents
    - If it doesn't exist, create it from the default template, return the default
    - If no repo_root, just return the default (no file creation)
- This replaces the current `INLINE_PROMPT_GUARDRAILS` const in `prepare.rs`

**Files modified**:

- `claudine/lib/src/composition/prepare.rs` — call `load_or_create_guardrails()` instead of using the hardcoded const
- `claudine/lib/src/composition/mod.rs` — add `mod guardrails;` and re-export if needed

**Signature change for `prepare_inline_prompt`**: Add `repo_root: Option<&Path>` parameter so it can locate the guardrails file. The caller in `wrap/mod.rs` already has `env_plan.repo_root`.

### 2. Hash frontmatter + body before agent execution

**File**: `claudine/cli/src/commands/wrap/mod.rs` (lines ~652-678)

After resolving and preparing the source, capture pre-execution hashes:

```rust
let pre_fm_hash = source.markdown.hash_frontmatter(false);
let pre_body_hash = source.markdown.hash_body(false);
```

Store these alongside `inline_composition_source` (extend the tuple or create a small struct).

### 3. Post-execution validation (structured path, lines ~1047-1087)

After the agent completes successfully (`exit_code == 0`):

**a) Check if body was updated:**

- Re-read the file from disk (agent may have written to it directly)
- OR check `summary.assistant_text` — if it's empty or only whitespace, warn
- Compare `hash_body` of the new content vs `pre_body_hash`
- If unchanged: report error to stderr and set exit_code to 1:

  ```
  log::error("The referenced file -- {relative_path} -- did not get updated even though the Agent reported a successful outcome!")
  ```

**b) Check if frontmatter was modified:**

- Before writing, re-read the file to check if the agent wrote to it directly
- After constructing `updated_md` with the agent's text, compare frontmatter hash
- If the agent's response somehow includes frontmatter (detected by re-reading from disk), restore original frontmatter:

  ```
  log::warn("the Agent modified frontmatter properties of the referenced file after being asked not to! We have set these properties back to their original state before updating the 'last_updated' property.")
  ```

- Use `*updated_md.frontmatter_mut() = source.markdown.frontmatter().clone()` to restore, then `fm_insert("last_updated", &today)`

**c) Check for empty summary (STDOUT):**

- After the structured stream completes, if `summary.assistant_text.trim().is_empty()` AND exit_code == 0:
    - Write to stderr: `log::warn("the agent did not provide a summarized message on their completed work!")`
    - Do NOT change exit code

### 4. Same validations for legacy capture path (lines ~1089-1129)

Apply the same body-update check and frontmatter-restoration logic to the `run_child_capture` code path, using `captured.stdout` / `profile.parse_captured_output()`.

### 5. Account for interactive sessions

When `interactive_requested` is true:

- The agent runs interactively — we can't capture structured output
- Post-run validation still applies: re-read the file from disk, compare hashes
- Frontmatter restoration still applies
- The "empty summary" warning doesn't apply (interactive sessions don't capture text)

## File-by-file changes

| File | Change |
|------|--------|
| `claudine/lib/src/composition/guardrails.rs` | **NEW** — load/create `.claudine/frontmatter-prompt.md` |
| `claudine/lib/src/composition/mod.rs` | Add `mod guardrails;` + re-export |
| `claudine/lib/src/composition/prepare.rs` | Accept `repo_root` param, use `guardrails::load_or_create_guardrails()` instead of hardcoded const |
| `claudine/cli/src/commands/wrap/mod.rs` | Pre-hash, pass repo_root to prepare, post-execution validation (both paths) |

## Key utilities to reuse

- `darkmatter::markdown::Markdown::hash_frontmatter(false)` / `hash_body(false)` — `/Volumes/coding/personal/rusty-biscuit/darkmatter/lib/src/markdown/hash.rs`
- `Markdown::frontmatter_mut()` — for restoring original frontmatter
- `Markdown::try_from(path)` — for re-reading file from disk post-agent
- `log::error()` / `log::warn()` — styled stderr output using Prose (`claudine/cli/src/log.rs`)
- `Path::strip_prefix(repo_root)` — for relative display paths
- `claudine::config::atomic::atomic_write()` — existing safe file write
- `env_plan.repo_root` — available at the call site in wrap/mod.rs

## Verification

1. **Unit tests** in `guardrails.rs`:
   - Test default template creation when no file exists
   - Test reading existing template file
   - Test graceful fallback when no repo_root

2. **Unit tests** in `prepare.rs`:
   - Update existing tests to pass `repo_root: None` (uses default guardrails)

3. **Manual test**: Run `claudine claude --fp some-file.md` and verify:
   - `.claudine/frontmatter-prompt.md` is created on first run
   - Guardrails from the file are prepended to the prompt
   - If agent modifies frontmatter, it's restored with a warning
   - If body is unchanged, error is reported
   - `last_updated` is set to today's date
