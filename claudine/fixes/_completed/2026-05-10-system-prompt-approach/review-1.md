---
ready: false
agent: codex
model: ""
---

# Review: System Prompt Delivery Redesign

## Findings

### High: scoped temp directory requirement is only applied to the new dispatcher, not all system-prompt overlay artifacts

The spec requires all overlay artifacts to live inside the user's trust boundary: `<repo_root>/.claudine/tmp/` in a repo, or `<launch_cwd>/.claudine-tmp/` otherwise. The direct and composition wrappers now compute and pass `scoped_tmp` into every profile, but several profiles ignore it and still use `tempfile::NamedTempFile::new()`, which writes under the host temp directory.

Affected code:

- `claudine/cli/src/commands/wrap/profile/claude.rs:34` and `:46`
- `claudine/cli/src/commands/wrap/profile/kimi.rs:38` and `:45`
- `claudine/cli/src/commands/wrap/profile/opencode.rs:60` and `:73`

This leaves the implementation only partially satisfying Goal 3. It also creates inconsistent behavior: Gemini/Codex/Qwen use scoped files, while Claude/Kimi/OpenCode still place prompt content outside the working trust boundary.

Verification level present: Level 1 unit tests only for `scoped_tmp_dir`, `scoped_tempfile`, and the new dispatcher in `claudine/cli/src/commands/wrap/system_prompt.rs`. No wrapper integration test asserts that every provider profile writes temp prompt files under the scoped directory.

Recommended fix: replace the remaining `NamedTempFile::new()` calls with the existing `scoped_tempfile(scoped_tmp, "system-prompt-")` helper, and add fake-provider wrapper tests that inspect the actual argv/env path for Claude/Kimi/OpenCode plus Gemini/Codex.

### High: Gemini/Codex/Qwen launch behavior is not verified at the wrapper boundary

The spec's core user-facing requirements are that `claudine gemini`, `claudine codex`, and `claudine qwen` preserve the real provider HOME/state while still delivering the system prompt. The implementation has good dispatcher unit tests, but I did not find integration tests that spawn the `claudine` binary with fake `gemini`, `codex`, or `qwen` shims and assert the actual child argv/env.

Current coverage examples:

- Dispatcher tests assert returned `SystemPromptApplication` values in `claudine/cli/src/commands/wrap/system_prompt.rs:425`, `:474`, `:522`, `:564`, `:607`, `:648`, `:687`, `:726`, `:770`, and `:810`.
- Existing wrapper tests only leave comments accepting extra Codex args at `claudine/cli/tests/wrap_commands.rs:261` and `:486`; they do not assert `-c developer_instructions=...`, `GEMINI_SYSTEM_MD`, Qwen inline flags, scoped paths, or that child `HOME` remains the caller's real `HOME`.

Verification level present: Level 1 unit tests for the dispatcher and provider catalog invariants. Missing Level 1 process-level integration tests for the user-observable wrapper behavior. Level 2/Level 3 are not required here because this is not terminal rendering or keyboard-input behavior.

Recommended fix: add fake-provider integration tests for direct wrapper and at least one composition path:

- Gemini append: child sees original `HOME`, `GEMINI_SYSTEM_MD=<repo>/.claudine/tmp/...`, and the file contains real `~/.gemini/GEMINI.md` plus overlay.
- Gemini replace: file contains only overlay.
- Codex append: child argv contains `-c developer_instructions=...` and original `HOME`.
- Codex replace: child argv contains `-c model_instructions_file=<repo>/.claudine/tmp/...`.
- Qwen append/replace: child argv contains the expected inline flags and original `HOME`.

### Medium: `.gitignore` augmentation runs outside repos and writes the wrong ignore entry

The helper is documented as "when `repo_root.join(".gitignore")` exists", but callers pass `launch_cwd` when `repo_root` is absent:

- `claudine/cli/src/commands/wrap/mod.rs:633`
- `claudine/cli/src/commands/wrap/composition/mod.rs:1097`

Outside a repo, `scoped_tmp_dir` creates `<launch_cwd>/.claudine-tmp`, while `maybe_gitignore_claudine_tmp` appends `.claudine/tmp/`. That means a non-repo directory with a `.gitignore` gets an irrelevant ignore entry and the actual `.claudine-tmp` directory remains unignored.

Verification level present: Level 1 unit tests cover idempotent append and missing `.gitignore`, but not the caller behavior for `repo_root: None`.

Recommended fix: call `maybe_gitignore_claudine_tmp` only when `launch_workspace.repo_root` is `Some`, or teach it which scoped temp entry is actually being used.

## Coverage Notes

No Level 2 or Level 3 verification appears necessary for this feature as specified. The behavior is child process argv/env/file placement and provider state visibility, not terminal rendering, keybinding behavior, paste/IME/mouse, or modifier-press visibility.

The manual checklist items for real Gemini/Codex/Qwen execution are still valuable, but the automated suite should first pin the wrapper contract with fake provider binaries so regressions are caught in CI.

## Ergonomics / Maintainability

`apply_system_prompt_via_spec` handles `EnvVarFile` by hardcoding `GEMINI.md` as the file to merge for append mode. That is acceptable for today's Gemini-only use, but it means the dispatcher is not fully spec-driven if another provider later uses `EnvVarFile`. A follow-up improvement would either move "append merge source" into `SystemPromptSpec` or keep the Gemini merge as an explicit provider-profile concern.

## Verification Performed

I attempted `cargo test -p claudine-cli system_prompt --color=never`, but it was still compiling after about 60 seconds, so I killed the cargo process per the non-interactive session guidance. No test results were produced from that command.

## Production Readiness

Not ready. The main mechanism for Gemini/Codex/Qwen appears directionally implemented, but the trust-boundary requirement is incomplete across provider profiles, and the wrapper boundary lacks the integration tests needed to prove the user-facing behavior actually reaches child processes.
