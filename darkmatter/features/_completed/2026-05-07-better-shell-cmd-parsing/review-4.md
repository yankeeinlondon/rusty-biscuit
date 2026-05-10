---
ready: false
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### High: Frontmatter executable interpolation is still allowed after `&&` / `||`

The frontmatter shell parser documents that executable-position interpolation must be rejected not just for the first command, but also after chain operators. The implementation only checks `first_token_portion(inner)` and never walks the later pipeline executable positions.

Evidence:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:53` says the original pre-interpolation value is checked for executable-token interpolation.
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:58` explicitly says to check after pipe/chain operators.
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:142` to `frontmatter_shell_expansion.rs:155` only extracts and checks the first token.
- Existing tests only cover `$({{cmd}} arg)` at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:397`; there is no regression for `$(false || {{cmd}} arg)` or `$(true && {{cmd}} arg)`.

Impact: a frontmatter value can smuggle an interpolated executable into any non-first pipeline action. For example, an original value like `$(false || {{cmd}} ok)` can pass validation once interpolation rewrites it to `$(false || echo ok)`. Policy approval still sees the resolved command, so this is not an execution-without-policy bypass, but it violates the executable-audit rule the frontmatter scanner claims to enforce and newly matters because this feature added multiple executable positions inside one `$()`.

Verification level: Level 1 gap. This is parser/validation behavior, so Level 1 is sufficient, but the strongest current test only checks the first executable token. Add Level 1 tests for interpolated executables after both `&&` and `||`.

### Medium: Accepted combined redirections do not preserve their configured targets

`parse_pipeline()` accepts multiple redirection tokens on the same action by setting both `stdout` and `stderr` fields in `RedirectionConfig`. However, `configure_streams()` treats any merge redirection as dominant and wires both child streams to the merged pipe, ignoring a simultaneous null target.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:370` to `tokenize.rs:380` allows stdout and stderr redirection fields to be set independently on one command action.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:651` to `executor.rs:665` immediately returns from the merge branch and sets both stdout and stderr to the shared pipe.
- The redirection tests at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:1694`, `mod.rs:1717`, `mod.rs:1744`, `mod.rs:1771`, and `mod.rs:1805` cover single redirection forms, but not combined forms.

Impact: common accepted commands like `cmd > /dev/null 2>&1` will still capture stdout/stderr instead of suppressing both streams, because the merge branch overrides the stdout-null configuration. If combined redirections are intentionally out of scope, the parser should reject multiple redirections per command. If they are accepted, the executor needs deterministic semantics and tests for each supported combination/order.

Verification level: Level 1 gap. Redirection semantics here are process wiring/captured output, so Level 1 is appropriate; the missing coverage is for accepted combined redirection configurations.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `> /dev/null` executes with no captured stdout | Level 1 compose test | ok |
| `2> /dev/null` suppresses stderr | Level 1 compose test | ok |
| `2>&1` merges stderr into stdout and preserves order | Level 1 compose tests | ok for single redirection |
| `>&2` routes stdout to stderr and preserves order | Level 1 compose tests | ok for single redirection |
| Combined accepted redirections have coherent semantics | No targeted test | gap |
| `A && B` executes `B` only if `A` succeeds | Level 1 compose tests | ok |
| `A || B` executes `B` only if `A` fails | Level 1 compose tests | ok |
| complex `A && B || C` chains | Level 1 compose test | ok |
| every command in a chain is policy checked before execution | Level 1 compose regression tests | ok |
| approval prompt presents the entire chain | Level 1 approval-request tests | ok |
| dry-run shell command report presents every chain action | Level 1 discovery tests | ok |
| unsupported `;`, `<`, bare `|`, arbitrary redirection remain rejected | Level 1 tokenizer/parser tests | ok |
| literal backticks are preserved | Level 1 tokenizer test | ok |
| frontmatter `$()` supports the new chain grammar | Level 1 compose/discovery tests | partial: later executable interpolation is untested and currently allowed |

## Recommendations

- Rework `validate_no_executable_interpolation()` to tokenize the original `$()` content enough to identify every executable position after `&&` and `||`, then reject `{{...}}` in any of those positions.
- Decide whether multiple redirections on one command are supported. If not, reject them during `parse_pipeline()`. If yes, add explicit semantics and Level 1 tests for `> /dev/null 2>&1`, `2>&1 > /dev/null`, `2> /dev/null >&2`, and related accepted combinations.

## Production Readiness

Not ready. The review-3 issues appear addressed, but the new multi-command frontmatter surface leaves a documented validation rule unenforced, and accepted combined redirections have broken semantics without tests.
