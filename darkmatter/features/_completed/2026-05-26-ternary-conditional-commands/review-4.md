---
ready: true
agent: codex
model: ""
---

# Review 4

## Findings

No blocking findings.

The implementation now matches the spec's core invariant: every command a frontmatter ternary could execute is statically anchored to the authored branch shape, validated against executable-position interpolation, and prepared through the existing shell policy path before the selected branch runs.

Prior review items appear resolved:

- Condition parsing now uses `parse_condition`, so condition-mode `&&`, `||`, `!`, comparisons, and parenthesized expression ternaries are supported by Level 1 execution tests.
- Shell-command discovery now walks every reachable ternary branch through `directive_reachable_pipelines`, so approval/preflight sees both branches, chained branch actions, and resolved branch arguments.
- Ternary separator detection is now explicitly whitespace-padded in the spec and implementation, with parser tests locking in that contract.
- Branch boundaries are pinned to the pre-interpolation snapshot, and shape-preservation checks reject argument interpolation that tries to introduce new `&&` / `||` actions or change executable tokens.

## Test Rigor

All feature requirements are parsing, composition, expression evaluation, shell-command discovery, allowlist/preapproval validation, and process execution through the existing shell runtime. These are correctly covered at Level 1. The spec does not add terminal rendering, keyboard input, mouse/paste/IME, or real terminal encoder/decoder behavior, so Level 2 and Level 3 tests are not required for production readiness.

Requirement-to-level mapping:

| Requirement | Strongest verification present | Appropriate? |
| --- | --- | --- |
| Plain frontmatter `$()` behavior remains unchanged | Level 1 unit/integration | Yes |
| Ternary parser splits only top-level whitespace-padded `?` / `:` and respects quotes/parentheses | Level 1 parser tests | Yes |
| Nested ternaries in branches are rejected | Level 1 parser tests | Yes |
| Then/else branch selection works, including empty branch short-circuit | Level 1 execution tests | Yes |
| Condition interpolation and condition-mode expression grammar work | Level 1 execution tests | Yes |
| Executable-position interpolation remains rejected in plain pipelines and ternary branches | Level 1 parser/discovery tests | Yes |
| Argument interpolation inside branches is allowed but cannot change pipeline shape | Level 1 execution tests | Yes |
| Both reachable branch command sets are surfaced to discovery/approval | Level 1 discovery tests | Yes |
| Off-allowlist or unapproved commands fail even when in the unselected branch | Level 1 execution tests | Yes |
| Motivating workflow works through the full compose pipeline | Level 1 compose integration tests | Yes |

## Verification

Completed:

```bash
cargo test --color=never -p darkmatter frontmatter_shell_expansion --lib
```

Result: 85 passed, 0 failed.

Completed:

```bash
cargo test --color=never -p darkmatter frontmatter_ternary --lib
```

Result: 5 passed, 0 failed.

## Production Readiness

Ready for production.
