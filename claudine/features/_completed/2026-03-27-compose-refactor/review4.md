# Compose Refactor Review 4

`just test` in `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine` passed while reviewing this implementation. The comments below are spec/design fidelity gaps, behavioral issues, and remaining coverage/ergonomics concerns.

## Findings

### 1. Inline file-state post-checks run before Claudine applies the deterministic rewrite

- Files:
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:2232`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:2243`
- Severity: High

`run_harness_loop()` evaluates user-defined `post_checks` before it calls `try_inline_closure()`. That means inline checks that are supposed to observe the final rewritten document, such as `file_changed`, frontmatter comparisons, or other on-disk state checks, are still looking at the pre-closure file.

I verified this locally with a harnessed inline document that used:

```yaml
post_checks:
  file_changed: "test.md"
```

and a provider that returned a new body. The run failed in post-check validation and the file on disk remained unchanged, because closure never ran.

That is out of line with the spec/design’s phase model for inline composition. Inline closure is supposed to be the deterministic file mutation step owned by Claudine, and file-state checks should evaluate the final post-closure artifact, not the pre-closure source file.

Recommended fix:

- move inline closure application ahead of file-state post-check evaluation, or
- fold closure into the post-check stage so built-in closure validations and user-authored file checks operate on the same final document state

### 2. Retired wrapper composition entry points are still accepted and forwarded downstream

- Files:
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:571`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:636`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1094`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs:2006`
- Severity: High

The spec says `claudine <agent> --compose <file>`, `--frontmatter-prompt <file>`, and `--prompt-file <file>` are retired and should be removed rather than kept as drift-preserving aliases. That is not fully true in the current CLI surface.

`WrapperArgs` still accepts arbitrary trailing passthrough args, and the wrapper path does not reject the retired composition switches. I verified locally with a stub `codex` binary that:

```sh
target/debug/claudine codex --compose test.md
```

is accepted by Claudine and forwarded to Codex as:

```txt
exec
--compose
test.md
--json
...
```

So the old wrapper signatures are not actually removed from the user-facing surface; they are just no longer Claudine-owned semantics. That is a confusing failure mode because users do not get a Claudine error or migration guidance.

The current regression coverage only checks that the old `compose-inline` subcommand is gone. There is no analogous rejection test for the retired wrapper flags.

Recommended fix:

- explicitly reject `--compose`, `--frontmatter-prompt`, and `--prompt-file` in wrapper passthrough with a migration error
- add integration coverage for those rejected signatures

### 3. The documentation still describes removed or incorrect composition flows

- Files:
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md:151`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md:153`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/validations-and-handlers.md:31`
- Severity: Medium

Acceptance criterion 8 says the docs must reflect the new contract. Two places still drift:

- `composition.md` claims `claudine <agent> --prompt-file <file>` is replaced by `claudine inline-compose --<agent> <file>`. That is not a behavioral replacement. `inline-compose` is a deterministic rewrite workflow that mutates the target document body; `--prompt-file` was a prompt-loading surface.
- `validations-and-handlers.md` still says the harness is active in `claudine <provider> --prompt-file`, `--frontmatter-prompt`, and `--compose`, which are exactly the retired wrapper entry points the refactor was supposed to remove.

Those docs will send users back to the old drifted mental model even though the implementation intent is now the two canonical top-level commands.

Recommended fix:

- update the harness doc to point at `claudine compose` and `claudine inline-compose`
- remove the `--prompt-file -> inline-compose` migration mapping
- audit remaining user-facing references to `frontmatter-prompt` terminology

## Coverage Gaps

- There is no integration test covering inline file-state `post_checks` after closure. The current harness retry coverage only exercises `response_includes` checks in [`wrap_commands.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs#L1604).
- There is no regression test that wrapper invocations reject `claudine <agent> --compose`, `--frontmatter-prompt`, or `--prompt-file`. The only retirement test in this area is for `compose-inline` in [`wrap_commands.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/tests/wrap_commands.rs#L2006).
- The design called for inline preflight validation against provider sandbox write constraints when determinable. Current top-level inline preflight only checks OS read/write permissions in [`compose.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L197), while provider-specific write assessment exists only inside the harness permission probe in [`wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L179). There is no coverage for that designed validation path.

## Ergonomics / Performance

- [`ComposeArgs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L75) and [`InlineComposeArgs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs#L109) still duplicate nearly every non-file field. A shared composition args struct would reduce drift.
- [`InlineClosurePlan`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/types.rs#L89) captures `original_frontmatter_hash` and `managed_fields`, but [`apply_inline_closure()`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/closure.rs#L43) and [`rewrite_inline_document()`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/closure.rs#L73) only use `original_body_hash` and hardcode `last_updated`. Either wire the extra fields into the rewrite/tamper-handling path or drop them until they are needed.
