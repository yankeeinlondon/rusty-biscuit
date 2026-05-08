---
ready: false
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### High: Partial `allow_once` overlap can approve an unapproved chain action

`prepare_directive()` correctly normalizes every action in a chain and checks each against the blacklist and whitelist first. The approval reservation path then becomes too broad: if `try_reserve_allow_once()` returns `false` for any normalized command, the implementation immediately treats the entire chain as approved.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:229` to `mod.rs:234` stops reserving on the first `false` from `try_reserve_allow_once()`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:236` to `mod.rs:241` returns `Ok(PreparedShellDirective { ... })` for the whole directive when any reservation was not made.
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:797` to `types.rs:803` documents that `false` means either the command is already allowed or another thread is handling it; it does not mean every command in the current chain is allowed.

Impact: a user can approve `echo allowed` once, then a later directive such as `echo allowed && make unapproved-target` can pass preparation without prompting for `make unapproved-target`, as long as `echo allowed` is encountered first in the chain. This violates the spec's "Every command in the chain must be checked against the policy engine" and "If any command in the chain requires user intervention ... the entire chain" must be approved upfront. The same early-approval path is also unsafe for a pending reservation from another thread: one pending action can cause a different unapproved action in the same chain to bypass the handler.

Verification level: Level 1 gap. This is in-process policy/approval behavior, so Level 1 is appropriate, but I did not find a regression test for partial `allow_once` overlap inside a multi-action chain.

Recommended fix: reserve only the commands that are not already whitelisted or already in `allow_once`, and after any failed reservation re-check that every command in the current directive is now authorized before returning success. If a command is merely pending in another thread, do not approve unrelated unreserved actions implicitly; either wait for the pending decision or return an approval-required/conflict error.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `> /dev/null` executes with no captured stdout | Level 1 parser and compose tests | ok |
| `2> /dev/null` suppresses stderr | Level 1 parser and compose tests | ok |
| `2>&1` and `>&2` merge/route streams with stable ordering | Level 1 compose tests | ok |
| combined accepted redirections have coherent left-to-right shell-style semantics | Level 1 parser and compose tests | ok |
| `A && B` and `A || B` execute conditionally | Level 1 compose tests | ok |
| complex `A && B || C` chains | Level 1 compose tests | ok |
| shell-block lines may be pipelines | Level 1 shell-block unit/stage/discovery tests | ok |
| shell-block pipeline timeout fallback emits a warning | Level 1 shell-block stage test | ok |
| body and frontmatter pipeline timeout fallback emits warnings | Level 1 compose/frontmatter tests | ok |
| every command in a chain is policy checked before execution | Level 1 tests for blacklist/whitelist/pre-approved paths; partial `allow_once` overlap untested and broken | gap |
| approval prompt presents the entire chain | Level 1 approval-request tests | ok |
| dry-run shell command report presents every body/frontmatter/shell-block chain action | Level 1 discovery tests | ok |
| unsupported `;`, `<`, bare `|`, arbitrary redirection remain rejected | Level 1 tokenizer/parser tests | ok |
| literal backticks are preserved | Level 1 tokenizer test | ok |
| frontmatter `$()` supports the new chain grammar and rejects interpolated executables after operators | Level 1 parser/compose/discovery tests | ok |

No Level 2 or Level 3 verification appears necessary for this feature as specified. The observable behavior is command parsing, policy/approval orchestration, process execution, captured output, and compose warnings rather than terminal emulator rendering or OS keyboard input.

## Notes

The previous review's two concrete findings appear addressed:

- Shell-block commands now parse and carry `ShellPipeline` through execution and discovery.
- Pipeline timeout fallback markers are propagated into compose warnings for body, frontmatter, and shell-block execution.

One non-blocking test-rigor note: the timing assertion in `execute_frontmatter_commands_concurrently` failed once while I was running multiple cargo test filters concurrently, then passed on a rerun by itself. I am not counting this as a product finding, but the test is sensitive to host contention.

## Validation Run

- `cargo test -p darkmatter shell_expansion` passed on rerun: 297 passed, 1 ignored.
- `cargo test -p darkmatter shell_block` passed: 82 unit-filtered tests plus 15 `shell_block_integration` tests passed.
- `cargo test -p darkmatter frontmatter_shell` passed: 43 passed.

## Production Readiness

Not ready. The parser and executor coverage is now in good shape, but the partial `allow_once` approval bypass is a policy/security blocker for chained commands.
