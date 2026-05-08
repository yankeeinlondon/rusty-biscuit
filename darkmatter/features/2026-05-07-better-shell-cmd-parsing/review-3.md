---
ready: false
agent: codex
model: ""
---

# Review: Better Shell Command Parsing

## Findings

### High: `2>&1` and `>&2` are merged after process exit, so stream ordering is not preserved

The redirection executor does not wire one child stream to the other at process setup time. It pipes stdout and stderr separately, waits for the child, then concatenates one captured buffer after the other in `apply_redirection_to_output()`.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:558` to `executor.rs:585` keeps both streams as pipes for `2>&1` / `>&2`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:588` to `executor.rs:615` implements redirection by appending stderr after stdout for `2>&1`, and stdout after stderr for `>&2`.
- Existing redirection tests only assert that both strings are present, not that the merged output preserves the emitted sequence.

Impact: commands that intentionally use `2>&1` to preserve diagnostic order will be rendered out of order. For example, a process that writes `ERR` to stderr before `OUT` to stdout should produce `ERROUT` under `2>&1`, but this implementation reports stdout first and stderr second. This is observable in both body `::shell` output and frontmatter shell expansion stdout.

Verification level: Level 1 present but incomplete. Level 1 is appropriate for this feature, but the tests do not verify the user-facing ordering semantics of stream redirection.

### Medium: `md compose --shell` discovery still underreports chained commands

`collect_shell_commands()` parses body and frontmatter shell directives into pipelines, but then records only the first executable/args pair for each directive. That means `::shell echo ok && pwd` is reported as a single `echo ok` command, and a frontmatter value like `$(echo ok || pwd)` has the same problem.

Evidence:

- Body discovery uses `directive.executable` and `directive.args` once per directive at `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:136` to `discovery.rs:167`.
- Frontmatter discovery uses `candidate.executable` and `candidate.args` once per frontmatter candidate at `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:377` to `discovery.rs:401`.
- The fixed approval path now validates every action before execution, but the non-executing command report does not mirror that command surface.

Impact: the dry-run/reporting path is no longer an accurate preflight view of what will execute. This is not as severe as an execution bypass, because `prepare_directive()` now checks every action, but it still weakens the feature’s auditability and makes `--shell` misleading for the exact command chains added by this feature.

Verification level: Level 1 gap. Add tests for `collect_shell_commands()` covering body and frontmatter chains, with expected entries for every action or an explicit full-chain entry model.

### Medium: Persisting "allow command" for a chain only persists the first executable

When the approval handler returns `AllowCommandPersist`, `prepare_directive()` completes all pending normalized commands, but only appends a prefix whitelist entry for `effective.executable`, which is the first action in the chain.

Evidence:

- The request can represent a whole chain at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:245` to `mod.rs:256`.
- The persistence branch writes only the first executable at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:270` to `mod.rs:276`.
- The CLI prompt describes option 2 as persisting `request.executable` "with any args" at `darkmatter/cli/src/approval.rs:120` to `approval.rs:124`, which is ambiguous for a multi-command approval prompt.

Impact: the approved chain runs once, but the persisted policy does not match the user’s likely intent when approving an entire chain. On the next run, later actions will still require approval unless they are separately whitelisted. That is safer than over-whitelisting, but it is ergonomically surprising and should be made explicit or changed to persist each executable in the chain.

Verification level: Level 1 gap. Existing regression tests cover independent whitelist checking, but not the persisted policy contents after `AllowCommandPersist` on a chain.

## Requirement Verification Matrix

| Requirement | Strongest verification present | Status |
| --- | --- | --- |
| `> /dev/null` executes with no captured stdout | Level 1 compose test | ok |
| `2> /dev/null` suppresses stderr | Level 1 compose test | ok |
| `2>&1` merges stderr into stdout | Level 1 compose test, presence only | gap: order semantics broken/untested |
| `>&2` routes stdout to stderr | Level 1 compose test, presence only | gap: stream routing semantics incomplete |
| `A && B` executes `B` only if `A` succeeds | Level 1 compose tests | ok |
| `A || B` executes `B` only if `A` fails | Level 1 compose tests | ok |
| complex `A && B || C` chains | Level 1 compose test | ok |
| every command in a chain is policy checked before execution | Level 1 compose regression tests | ok |
| approval prompt presents the entire chain | Level 1 approval-request tests | ok for execution path |
| dry-run shell command report presents the command surface | Level 1 single-command tests only | gap for chains |
| unsupported `;`, `<`, bare `|`, arbitrary redirection remain rejected | Level 1 tokenizer/parser tests | ok |
| literal backticks are preserved | Level 1 tokenizer test | ok |
| frontmatter `$()` supports the new grammar | Level 1 scan/compose tests for OR chain | partial: no redirection/AND/discovery tests |

## Recommendations

- Implement `2>&1` / `>&2` using actual child stream wiring where possible, or document that darkmatter only concatenates captured streams and rename the behavior/tests accordingly.
- Expand `collect_shell_commands()` to emit every pipeline action, or introduce a chain-aware entry that carries all actions while preserving source/origin metadata.
- For `AllowCommandPersist` on chains, either persist a prefix rule for each action or make the CLI option explicitly say it persists only the first command.
- Add focused Level 1 tests for stream ordering, chain discovery in body/frontmatter, and persisted whitelist contents after `AllowCommandPersist`.

## Production Readiness

Not ready. The execution/security path is much improved from the previous review, but stream redirection semantics are still incomplete and the discovery/reporting surface does not yet reflect chained commands.
