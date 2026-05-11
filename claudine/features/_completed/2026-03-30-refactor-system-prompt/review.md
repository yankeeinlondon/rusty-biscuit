# Review: System Prompt Refactor

Target reviewed:
- [spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/features/2026-03-30-refactor-system-prompt/spec.md)
- [tech-design.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/features/2026-03-30-refactor-system-prompt/tech-design.md)

Targeted test run:
- `cargo test -p claudine system_prompt --lib` passed

## Findings

### 1. High: system-prompt resolution/composition failures are silently dropped

Both wrapper execution paths convert any `resolve_and_prepare()` error into `EffectiveSystemPrompt::None`, so an explicit missing file, unreadable file, or Darkmatter composition failure is ignored instead of failing the run.

Evidence:
- [wrap/mod.rs:1045](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1045)
- [wrap/mod.rs:1046](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1046)
- [wrap/composition.rs:406](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:406)
- [wrap/composition.rs:408](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:408)

Why this matters:
- The spec/design explicitly make explicit file selection authoritative.
- A typo in `--append-system-prompt` / `--replace-system-prompt` currently degrades into "no system prompt" with no error.
- A broken `::file`, `::shell`, or composition error inside `system-prompt.md` is also silently skipped.

Recommendation:
- Propagate `resolve_and_prepare()` errors to the caller.
- Add CLI/integration tests for missing explicit files and composition failures.

### 2. High: append-mode system prompts are broken for composition flows on Codex/Gemini/Qwen

The composition executor builds `env_plan` first, then refuses to apply a `HOME` override from `apply_system_prompt()` if `HOME` is already present. Since `HOME` is already present in the sanitized child environment, append-mode providers that rely on an overlay home never receive their overlay in compose/inline-compose/sequence.

Evidence:
- [wrap/composition.rs:205](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:205)
- [wrap/composition.rs:406](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:406)
- [wrap/composition.rs:426](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:426)
- [wrap/composition.rs:427](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:427)
- [profile.rs:607](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/profile.rs:607)
- [profile.rs:878](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/profile.rs:878)
- [profile.rs:1149](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/profile.rs:1149)

Why this matters:
- The design explicitly required the new switches on composition entry points, not only direct wrappers.
- Today `compose`, `inline-compose`, and `sequence` will silently fail to append the prompt for the providers that need home overlays.

Recommendation:
- Move system-prompt application ahead of env-plan finalization in composition flows, or make `HOME` replacement explicit and safe.
- Add integration coverage for `compose` and `inline-compose` with append-mode prompts on Codex/Gemini/Qwen.

### 3. High: append-mode home overlays are not merged with repo/MCP shadow homes

In the normal wrapper path, system-prompt append mode writes `HOME` into `env_overrides`, but `build_child_env()` may later replace `HOME` again when repo isolation or MCP shadow-home support is active. That means the overlay prompt and the shadow-home resources are never combined into one effective home.

Evidence:
- [wrap/mod.rs:1091](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1091)
- [wrap/mod.rs:1094](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1094)
- [wrap/env.rs:106](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/env.rs:106)
- [wrap/env.rs:119](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/env.rs:119)
- [wrap/system_prompt.rs:50](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/system_prompt.rs:50)

Why this matters:
- The tech design called for choosing a base home and layering the prompt overlay on top of it.
- Current behavior makes append-mode prompts incompatible with `--repo` and Codex/Gemini MCP runtime injection.
- `create_ephemeral_overlay_home()` also starts from an empty temp home, so it does not preserve provider startup/auth state as designed.

Recommendation:
- Replace the current "provider returns `HOME` env var" approach with a single launch-home builder that can start from real home or repo shadow home, then apply the prompt overlay as the last mutation.
- This likely wants a richer `apply_system_prompt()` contract again, closer to what the tech design proposed.

### 4. Medium: explicit system-prompt files do not use Claudine's normal file-reference resolution

The tech design says explicit files should resolve through the same file-resolution rules used elsewhere in Claudine, but `resolve_file_ref()` only supports absolute paths or paths relative to `cwd`.

Evidence:
- [system_prompt/resolve.rs:49](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/system_prompt/resolve.rs:49)
- [system_prompt/resolve.rs:63](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/system_prompt/resolve.rs:63)
- [system_prompt/resolve.rs:145](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/system_prompt/resolve.rs:145)
- [composition/resolve.rs:21](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/resolve.rs:21)

Why this matters:
- `compose`/`sequence` users already get `biscuit-file::FileReference` semantics for normal source files.
- The new system-prompt switches do not honor that same contract, so `@repo/...` and similar references are a designed feature gap.

Recommendation:
- Reuse the same `biscuit-file::FileReference` resolution path used by composition sources.
- Add tests for repo-relative and magic references on both wrapper and composition entry points.

### 5. Low: documentation updates required by the design are incomplete and partly inaccurate

The design explicitly called for doc updates alongside the implementation, but the public READMEs still advertise the retired `--system-prompt` switch, and the topic doc says the new flags accept only absolute paths even though the implementation accepts relative paths.

Evidence:
- [cli/README.md:129](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/README.md:129)
- [cli/README.md:171](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/README.md:171)
- [lib/README.md:281](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/README.md:281)
- [docs/topics/system-prompt.md:15](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/system-prompt.md:15)

## Test Coverage Gaps

- There is strong unit coverage for `claudine::system_prompt`, but I could not find wrapper/composition integration tests that exercise the new system-prompt flags end-to-end. That gap is large enough that the broken `HOME` handling and swallowed-error path both shipped unnoticed.
- Provider-specific `apply_system_prompt()` logic appears untested for Claude, Codex, Gemini, Goose, Kimi, OpenCode, and Qwen. The tech design called out these cases explicitly.
- The current user-home fallback tests are nondeterministic because they depend on the real `~/.claudine/system-prompt.md` state of the machine under test. Stronger tests should override `HOME` so the result is deterministic.

Recommended additions:
- Wrapper integration tests for explicit missing files and composition failures.
- Wrapper integration tests for append/replace behavior per provider, especially Codex/Gemini/OpenCode/Kimi/Qwen.
- Composition integration tests for `compose`, `inline-compose`, and `sequence` with append-mode prompts.
- Tests for `--repo` and `--mcp` combined with append-mode prompts on Codex/Gemini.
- Deterministic user-home fallback tests using a temporary home directory.

## Ergonomics / Performance Suggestions

- Unify repo shadow-home handling, MCP shadow-home handling, and system-prompt overlay handling behind one launch-home planner. That will remove duplicated `HOME` logic, fix the current conflicts, and make provider behavior easier to reason about.
- Replace the bespoke `resolve_file_ref()` helper with the same `biscuit-file::FileReference` pipeline used elsewhere. This removes duplicate path logic and makes the user-facing contract consistent.
- Consider moving system-prompt preparation errors into normal CLI diagnostics instead of degrading to `None`. That is better ergonomically for debugging and avoids silent misconfiguration.
