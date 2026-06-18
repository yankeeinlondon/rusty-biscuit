---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 3

## Verdict

Not ready for production.

The main approval lifecycle is much improved: the CLI now runs document pre-flight before compose, Claudine merges document and harness commands, condition-blind collection covers false page blocks and false transclusions, dynamic command shapes are rejected early, and cache opt-out behavior is implemented across body, frontmatter, and shell blocks. I found one remaining production-safety gap in the library pipeline for shell-block-only compositions, plus one explicit design goal that is still only partially implemented.

## Findings

### High: shell-block-only compositions skip the up-front pre-approved-set validation

The design requires the pre-flight membership check to cover body `::shell`, frontmatter `$()`, and `::shell-block` commands before any shell execution begins. The collector does include shell-block commands in the approval set (`darkmatter/lib/src/markdown/compose/preflight/collect.rs:265`), but the root pipeline only calls `validate_pre_approved(...)` when `FrontmatterShellExpansion` or `ShellExpansion` is enabled:

- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:216`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:218`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:219`

`ComposeOperation::ShellBlocks` is missing from that gate. A caller that intentionally runs only shell blocks, or shell blocks plus transclusion, can supply `with_pre_approved_commands(...)` and still avoid the up-front graph-wide validation. The shell-block stage prepares every command inside one block before executing that block, but it does not prepare every shell block in the document graph before executing the first block:

- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:104`
- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:126`
- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:146`

So a first approved shell block can execute before a later sibling block, or a transcluded child block, fails with `NotPreApproved`. That violates the v2 invariant that pre-flight catches the incomplete approval set before any shell command runs.

Verification level: Level 1 is appropriate. Add an in-process test with `ComposeOptions::new().only(&[ComposeOperation::ShellBlocks])`, a first approved shell block that creates a sentinel, a later unapproved shell block, and `with_pre_approved_commands(...)` missing the later command. The expected result is a pre-execution `NotPreApproved` and no sentinel. A second test should include `BlockTransclusion` to prove the root validation walks child shell blocks before any root block executes.

Suggested fix: include `ComposeOperation::ShellBlocks` in the root pre-approved validation gate.

### Medium: pre-flight graph metadata is collected but still not reused by transclusion

The design says the pre-flight collection walk should cache graph metadata and reuse it to seed graph composition, avoiding a second directive parse / target-resolution pass. The implementation exposes `ComposePreflightReport::preflight_graph`, but its own docs frame this as something for a "future integration":

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:104`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:107`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:120`

`rg` shows no non-test consumer of `preflight_graph` outside `compose_preflight()`. The transclusion engine still resolves targets from directives during the final compose pass:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:462`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:473`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:492`

This is not an immediate approval safety failure, but it leaves a named v2 design goal incomplete and preserves a source of drift between the pre-flight graph walk and the execution graph walk.

Verification level: Level 1 is appropriate. Once wired, add a test proving a preflight-collected local or remote child is consumed by final transclusion without repeating target discovery. If reuse is intentionally deferred, update the feature design/plan so this is no longer part of the production-ready slice.

## Test Coverage Assessment

- Level 1 present: condition-blind collection for false page blocks, false local and remote transclusions, frontmatter ternary branches, shell blocks, dynamic command-shape rejection, execution subset checks including randomized conditions, CLI `--shell` discovery, pre-flight approval lifecycle, Claudine document + harness merge, cache defaulting, all three no-cache syntaxes, volatile warnings, and cache-key separation for chain operators/redirections.
- Level 1 gap: no test covers pre-approved validation when `ShellBlocks` is enabled without `ShellExpansion` or `FrontmatterShellExpansion`.
- Level 1 gap: graph metadata reuse is untested because it is not implemented.

No Level 2 or Level 3 tests are required for these findings. The reviewed requirements are compose orchestration and shell-command approval semantics, not terminal rendering, terminal input encoding, or OS keyboard behavior.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
cargo test --color=never -p darkmatter markdown::compose::shell_expansion::integration_tests --lib
cargo test --color=never -p darkmatter markdown::compose::shell_blocks --lib
```

All three passed.
