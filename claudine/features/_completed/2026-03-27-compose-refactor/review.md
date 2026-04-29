# Compose Refactor Review

## Findings

### [P1] Inline composition still uses provider-side file mutation as the source of truth

The refactor spec and tech design explicitly changed inline composition so the provider returns replacement body content and Claudine rewrites the file itself. That contract is not what the implementation does today.

- Non-harness inline runs read the target file back from disk and decide success based on whether the agent mutated the body on disk: [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L298), [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L389), [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L436)
- Harness inline runs do the same thing: they ignore `final_response`, reread the file from disk, and rebuild from the on-disk body: [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1494), [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1603), [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1624)
- Both paths even downgrade provider failures into success when the file changed on disk: [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L404), [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1641)

This is the core behavioral contract the refactor was meant to fix, so I consider it the highest-severity gap.

### [P1] Provider selection still violates the new precedence rules

Two required selection semantics are still wrong:

- Explicit provider selection is still filtered through `--exclude`, even though the spec says exclusions only apply when selection is not explicit: [claudine/lib/src/composition/select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/select.rs#L38), [claudine/lib/src/composition/select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/select.rs#L43)
- A valid `agent` hint that matches known providers but none of the installed candidates returns `AgentHintInvalid` instead of falling through to favorite provider / chooser / error: [claudine/lib/src/composition/select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/select.rs#L93), [claudine/lib/src/composition/select.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/select.rs#L99)

That second branch is specifically called out in the design, so returning a validation error here is a spec regression.

### [P1] Inline harness execution still falls back to raw frontmatter instead of effective composed frontmatter

The top-level executor correctly enables harnessing from `prepared.effective_frontmatter`, but the inline harness loop re-materializes frontmatter incorrectly.

- `prepare_inline()` returns composed frontmatter in `prepared.effective_frontmatter`
- `materialize_harness_prompt()` discards that and feeds `effective_markdown.frontmatter()` into the harness plan for inline mode instead: [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1274), [claudine/cli/src/commands/wrap/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L1284)

That reintroduces the exact drift the refactor was supposed to eliminate for inline/chained composition: composed `prompt` content can affect the effective frontmatter, but the harness loop does not see it.

### [P1] Top-level composition still does not inherit wrapper-grade MCP/session behavior

The spec/design call out MCP composition and tag handling as part of the shared launch path, but the top-level composition commands cannot participate in that flow today.

- `compose` / `inline-compose` only expose `--interactive`, `--exclude`, `--silent`, and provider flags; there is no `--mcp`, `--use`, or `--strict`: [claudine/cli/src/commands/compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L21), [claudine/cli/src/commands/compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L68)
- The executor hard-codes a no-MCP environment plan: [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L126)
- The docs now promise “env, MCP, harness, streaming” for the launch stage, which is not yet true: [claudine/docs/topics/composition.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md#L169)

This means the refactor did not fully converge on the wrapper-grade launch path; it only reused part of it.

### [P2] Interactive composition behavior is still weaker than the design

Two interactive requirements are incomplete:

- `inline-compose -i` is rejected unconditionally, instead of being allowed for providers that can recover the final assistant body: [claudine/cli/src/commands/wrap/composition.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L96)
- `compose -i` still cannot work for Claude or Kimi because their prompt delivery code refuses interactive prompt seeding outright: [claudine/cli/src/commands/wrap/profile.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/profile.rs#L387), [claudine/cli/src/commands/wrap/profile.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/profile.rs#L810)

The design explicitly said direct composition should always be able to launch interactively, with inline interactivity gated by provider capability. The current implementation is stricter and more ad hoc.

### [P2] The test suite is still validating the old inline contract, not the new one

The current CLI tests largely lock in the pre-refactor behavior:

- `inline_compose_validates_file_update` expects failure when the provider exits successfully but does not mutate the file on disk: [claudine/cli/tests/wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs#L1394)
- `inline_compose_preserves_frontmatter` simulates success by having the fake provider rewrite the file directly: [claudine/cli/tests/wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs#L1433)
- `inline_compose_no_overwrite_on_failure` only verifies “provider failed and did not mutate disk”, not “provider returned invalid/empty body” or “Claudine ignored provider-authored frontmatter in captured output”: [claudine/cli/tests/wrap_commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs#L1490)

Missing coverage I would expect from this refactor:

- explicit provider plus `--exclude` still choosing the explicit provider
- hinted-but-uninstalled provider falling through to favorite/chooser/error
- inline harness behavior using effective composed frontmatter
- inline closure using captured assistant output instead of disk mutation
- capability-gated `inline-compose -i`
- direct interactive composition on providers that require different prompt-seeding mechanics
- MCP tag/runtime behavior from top-level composition commands

### [P3] Documentation drift is still present

The user-facing docs were supposed to land with the implementation, but some docs still describe the removed wrapper switches:

- [claudine/cli/README.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/README.md#L125)
- [claudine/lib/README.md](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/README.md#L224)

`claudine/docs/topics/composition.md` is updated, so the remaining stale docs will keep confusing users unless they are brought into sync in the same change.

## Ergonomics / Performance

- The inline rewrite logic is duplicated in two places and neither path uses the library helpers that were added for the refactor. Routing both harness and non-harness closure through `composition::closure::{extract_replacement_body, apply_inline_closure}` would remove duplicate disk reads, reduce divergence, and put the core behavior under unit coverage instead of bespoke wrapper branches.
- `CompositionExecutionRequest` is still too small to express the full wrapper-grade launch contract. Adding MCP/session options there, instead of keeping them wrapper-only, would prevent another round of composition-vs-wrapper drift and make the launch path genuinely shared.

## Verification

- Ran `just test` in [claudine](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine)
- Result: library tests passed; CLI tests failed in `wrap_commands::wrapper_help_includes_expected_flags` because the stored snapshot is stale relative to current help output
