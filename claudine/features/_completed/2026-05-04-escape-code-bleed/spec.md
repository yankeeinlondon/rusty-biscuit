in non-interactive sessions we sometimes find that terminal escape codes start to become malformed and rather than
appearing in the terminal as escape codes are simply printed to the terminal as characters. I believe this happens
across providers but here is an example where it's happening when using OpenCode:

```sh
⏱️ 18:06 PDT running the prompts/implement-phase.md prompt
- OpenCode session ID ses_20a538d4

 ← Skill(successful, claudine)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(error File not found: /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/reviews/2026-05-04-sentrux/plan.md)
 ← Glob(successful, **/2026-05-04-sentrux/**)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Glob(successful, **/plan.md)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/reviews/2026-05-04-sentrux/plan.md)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Todowrite(successful, {"todos":[{"content":"Phase 3.1: Split commands::wrap::profile into profile/
    directory","priority":"high","status":"pending"},{"content":"Phase 3.2: Split commands::wrap::live_semantic_sink into
    live_semantic_sink/ directory","priority":"high","status":"pending"},{"content":"Phase 3.3: Split commands::wrap::exec and
    absorb wire_io + subagent_watchdog into exec/ directory","priority":"high","status":"pending"},{"content":"Phase 3.4: Split
    commands::wrap::mod into multiple files","priority":"high","status":"pending"},{"content":"Run tests and lint after all
    splits","priority":"high","status":"pending"},{"content":"Update plan.md frontmatter with phase 3
    changes","priority":"high","status":"pending"}]})
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/live_semantic_sink.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/exec.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/wire_io.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/subagent_watchdog.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Bash(successful, bash wc -l /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile.rs
    /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/live_semantic_sink.rs
    /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/exec.rs
    /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/wire_io.rs
    /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/subagent_watchdog.rs
    /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Bash(successful, bash cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap && grep -n
    "^pub(crate) trait\|^impl WrapperProfile\|^pub(crate) struct\|^fn " profile.rs | head -100)
^[]11;rgb:1a1a/1b1b/2626^[\ ← Bash(successful, bash cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap && grep -n
    "^pub(crate) trait\|^pub(crate) struct\|^pub(crate) enum\|^pub(crate) fn\|^fn\|^impl " profile.rs | head -50)
^[]11;rgb:1a1a/1b1b/2626^[\⏱️ 18:16 PDT running the prompts/implement-phase.md prompt for 10m
⏱️ 18:26 PDT running the prompts/implement-phase.md prompt for 20m
 ← Task(successful, Split profile.rs into directory)
^[]11;rgb:1a1a/1b1b/2626^[\⏱️ 18:36 PDT running the prompts/implement-phase.md prompt for 30m
 ← Task(successful, Split live_semantic_sink.rs into directory)
^[]11;rgb:1a1a/1b1b/2626^[\⏱️ 18:46 PDT running the prompts/implement-phase.md prompt for 40m
```

As you can see from this example, the prompt starts find but after loading the "claudine" skill and reporting on it
correctly, we find that the `Glob` tool calls has started repeating the '^[]11;rgb:1a1a/1b1b/2626^[\' characters before
each tool call icon afterward. The pattern is always that once it starts happening it will continue.

Your task in this feature is to identify the reason(s) for this escape code bleed and fix it.
