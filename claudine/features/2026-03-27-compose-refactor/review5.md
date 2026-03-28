# Compose Refactor Review 6

`just test` in `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine` passed while reviewing this change. The issues below are remaining spec/design fidelity gaps and lighter coverage/ergonomics concerns in the current implementation.

## Findings

### 1. [P2] Repo-scoped composition context is still derived from the caller's cwd instead of the resolved source document

- [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L195) resolves the source file first, but [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L200) still discovers the inline repo root with `find_git_root()` from `current_dir()`, not from `source.resolved_path`.
- [composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L71) then loads the favorite provider from the cwd repo via [composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L92), and builds wrapper env/MCP repo context from that same cwd via [composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L136).
- That means `claudine compose /other/repo/prompt.md` or `claudine inline-compose /other/repo/prompt.md` can silently ignore the target repo's `.claudine/config.json`, repo MCP defaults, and custom inline guardrails even though the composition source was resolved successfully.

This is still untested end to end. I would add an integration test that runs composition from outside the target repo and proves the target repo's favorite provider / guardrails / repo defaults win.

### 2. [P2] Built-in inline writability checks still bypass the harness recovery model

- [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L197) hard-fails inline composition on filesystem writability before the wrapper-grade executor or handler system is entered.
- Once execution begins, the harness only parses validations already present in effective frontmatter via [composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L279); there is no system-owned `has_write_permission` rule being prepended for inline runs.

The tech design explicitly called for prep-time built-ins to participate in the same conceptual validation model as harness rules. In the current shape, `redirect`, `deviate`, or other handler recovery paths can never respond to a built-in inline permission failure because the run exits before handler resolution exists.

Coverage is light here too: there is no CLI test for a built-in inline permission failure, and no test that proves provider-sandbox writability is checked when it is determinable.

### 3. [P3] `InlineClosurePlan` carries tamper/managed-field state that the rewrite path never actually uses

- [types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/types.rs#L89) stores `original_frontmatter_hash` "for tamper detection" and a generic `managed_fields` set.
- [prepare.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/prepare.rs#L84) populates both fields for every inline run.
- But [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/closure.rs#L43) only checks `original_body_hash`, and [closure.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/closure.rs#L73) still hardcodes `last_updated` instead of using `managed_fields`.

That leaves the type-level contract ahead of the implementation: concurrent frontmatter edits are still overwritten silently, and adding another Claudine-managed inline field will require touching the rewrite algorithm anyway. I would either implement the advertised tamper/managed-field behavior now or trim the unused state until it is real.

### 4. [P3] The user-facing migration docs are still out of sync with the shipped CLI behavior

- [composition.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md#L151) still says `--frontmatter-prompt` maps to either `compose` or `inline-compose`, and [composition.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md#L153) still maps `--prompt-file` to `claudine compose`.
- The actual wrapper migration guidance rejects those flags and points users to `inline-compose` in [wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L557).
- The inline guardrail file and comments still carry the retired `frontmatter-prompt` naming in [guardrails.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/guardrails.rs#L1).

This is low severity, but it is still product drift: users following the docs will be pointed at behavior that the CLI no longer claims to support.

## Ergonomics / Performance

- The composition request should probably carry a resolved `source_repo_root` derived from `prepared.resolved_path`, then reuse it everywhere for favorite-provider lookup, guardrails, MCP defaults, and harness path resolution. That removes repeated cwd-based discovery and makes cross-repo composition deterministic.
- If `managed_fields` is meant to survive, drive the rewrite through that set instead of hardcoding `last_updated`. If it is not meant to survive yet, deleting the unused state will make the closure contract easier to reason about.

## Verification

- Ran `just test` in [claudine](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine)
- Result: passed
