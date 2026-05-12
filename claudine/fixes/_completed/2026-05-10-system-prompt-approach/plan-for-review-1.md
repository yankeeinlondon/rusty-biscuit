# Plan: System Prompt Delivery Review 1 Fixes

Source: `review-1.md` — 3 findings (2 High, 1 Medium)

---

## Fix 1 — High: Migrate remaining profiles to scoped temp directory

**Problem**: Claude, Kimi, and OpenCode profiles use `tempfile::NamedTempFile::new()` which writes to the host temp directory instead of the trust-boundary scoped directory. Gemini/Codex/Qwen already use `scoped_tempfile()`.

**Scope**:

### 1a. `claudine/cli/src/commands/wrap/profile/claude.rs`

- Line 34 (Append, non-interactive): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "claude-system-prompt-")?`
- Line 46 (Replace, non-interactive): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "claude-system-prompt-")?`
- The `_scoped_tmp` parameter is already passed into `apply_system_prompt` but prefixed with `_` (unused). Remove the underscore prefix and wire it through.

### 1b. `claudine/cli/src/commands/wrap/profile/kimi.rs`

- Line 38 (Replace, prompt temp): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "kimi-system-prompt-")?`
- Line 45 (Replace, agent yaml temp): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "kimi-agent-yaml-")?`
- Remove `_` prefix from `_scoped_tmp` parameter.
- Add import for `scoped_tempfile` from `super::super::system_prompt` (or use crate-level path).

### 1c. `claudine/cli/src/commands/wrap/profile/opencode.rs`

- Line 60 (Append): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "opencode-system-prompt-")?`
- Line 73 (Replace): Replace `tempfile::NamedTempFile::new()?` with `scoped_tempfile(scoped_tmp, "opencode-system-prompt-")?`
- Remove `_` prefix from `_scoped_tmp` parameter.
- Add import for `scoped_tempfile`.

**Verification**: Each profile change should be validated by a unit test (or the new integration tests in Fix 2) that asserts the temp file path starts with the scoped directory.

---

## Fix 2 — High: Add wrapper-boundary integration tests

**Problem**: No integration tests spawn the `claudine` binary with fake provider shims and assert actual child argv/env for Gemini, Codex, or Qwen system-prompt delivery.

**Approach**: Extend `claudine/cli/tests/wrap_commands.rs` with fake-provider integration tests. Follow the existing pattern (shell-script shims that write `$@` to a file, `$ENV` to a file).

### 2a. Gemini direct wrapper tests

- Gemini append (non-interactive): Assert child sees original `HOME`, `GEMINI_SYSTEM_MD` env pointing to `<repo>/.claudine/tmp/...`, and the file at that path contains the merged content (existing `~/.gemini/GEMINI.md` + overlay).
- Gemini replace (non-interactive): Assert file at `GEMINI_SYSTEM_MD` path contains only the overlay content.
- Both: Assert temp file path is under the scoped tmp directory, not `/tmp`.

### 2b. Codex direct wrapper tests

- Codex append (non-interactive): Assert child argv contains `-c developer_instructions="..."` and original `HOME`.
- Codex replace (non-interactive): Assert child argv contains `-c model_instructions_file=<repo>/.claudine/tmp/...` and the file at that path has the replacement content.
- Both: Assert no `HOME` override in env.

### 2c. Qwen direct wrapper tests

- Qwen append/replace (non-interactive): Assert child argv contains the expected inline flags and original `HOME`.

### 2d. Claude / Kimi / OpenCode scoped-path verification

- Add at least one test per provider that asserts the temp file path written to the argv/env shim is under the scoped tmp directory (verifies Fix 1).

### 2e. Composition path smoke test

- At least one composition test that exercises system-prompt delivery through the composition pipeline (not just the direct wrapper) to ensure the scoped tmp and system prompt are wired correctly there too.

**Infrastructure needed**:

- A way to seed a system prompt for testing (e.g. a `SYSTEM_PROMPT` file in the test workspace, or an explicit `--system-prompt` / `--replace-system-prompt` CLI flag).
- The existing `seed_minimal_config` helper and fake binary shim pattern can be reused.

---

## Fix 3 — Medium: Guard `.gitignore` augmentation against non-repo paths

**Problem**: `maybe_gitignore_claudine_tmp` is called with `launch_cwd` when `repo_root` is `None`, which appends `.claudine/tmp/` to a non-repo `.gitignore`. The actual directory created is `.claudine-tmp`, so the entry is irrelevant.

### 3a. Guard the callers

**File**: `claudine/cli/src/commands/wrap/mod.rs:633-636`

Change from:
```rust
system_prompt::maybe_gitignore_claudine_tmp(
    launch_workspace.repo_root.as_deref().unwrap_or(&launch_workspace.launch_cwd),
);
```
To:
```rust
if let Some(repo_root) = &launch_workspace.repo_root {
    system_prompt::maybe_gitignore_claudine_tmp(repo_root);
}
```

**File**: `claudine/cli/src/commands/wrap/composition/mod.rs:1097-1100`

Same change pattern — guard with `if let Some(repo_root)`.

### 3b. Unit test for non-repo caller behavior

- Add a test that creates a non-repo workspace with a `.gitignore` and verifies that `maybe_gitignore_claudine_tmp` is NOT called (i.e. the `.gitignore` is untouched).
- This tests the caller guard, not the helper itself (which is already tested).

---

## Execution Order

1. **Fix 3** (Medium) — smallest, self-contained, no dependencies
2. **Fix 1** (High) — scoped tempfile migration in 3 profile files
3. **Fix 2** (High) — integration tests that verify both Fix 1 and existing behavior

After all fixes:
- Run `cargo test -p claudine-cli system_prompt --color=never`
- Run `cargo test -p claudine-cli wrap_commands --color=never`
- Run `cargo clippy -p claudine-cli` for lint
