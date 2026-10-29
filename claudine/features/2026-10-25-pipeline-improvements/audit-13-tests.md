---
kind: implementation-log
---

# Test audit: 2026-10-23-shell-template-approval

Added **one genuinely new test function/Nextest identity** (gross and net
increase: **one**), strengthened **zero** existing tests, and modified
**zero** slow tests. No production behavior changed. All **584 distinct
selected L1 tests passed**, maximum **3.881 seconds** on this host.

## Coverage improvement

`sequence_shell_templates::a_nested_container_terminal_template_uses_the_current_value_as_one_argument`
closes a later container-integration gap in the original approval corpus.
Existing container argument tests used `--yolo`; the new test uses real
saved policy files and the normal noninteractive CLI. An exact rule for the
container terminal event's initial bytes is refused before the first task
runs. Replacing that rule with a template allows the same graph. An earlier
side effect changes the placeholder to a value containing whitespace,
semicolon, command substitution, and backticks. The native argument recorder
observes the changed value as exactly one argument after the nested child,
with no shell reinterpretation. The fixture also asserts no audio publication.

The test passed alone in **1.756 seconds** and in the final parallel scope in
**3.446 seconds**. The module introduction's stale late-root spelling was
corrected from `outputs` to `seq`; runtime behavior was unchanged.

## Requirement coverage and boundaries

| Acceptance criteria | Existing and new evidence |
| --- | --- |
| 1–2: missing values and no effects before rejection | Darkmatter discovery tests and Claudine `shell_missing_values`, sequence barrier, and referenced-document cases assert typed/located failures, no recorder/provider effects, and preservation of earlier task effects. Empty/zero/false/fallback controls remain. |
| 3–5: persisted rules, shape matching, type coverage | Darkmatter template policy/grammar suites exercise real store round trips, malformed rule warnings, whitelist/blacklist robustness matrices, argument/operator identity, and the complete covers table. |
| 6–7: argument boundaries, validation, secret omission | Real `md` and Claudine processes dispatch native recorder executables; cases include adjacent holes, metacharacters, empty arguments, Unicode, newlines, NUL/container rejection, declared constraints, and errors that omit rejected values. |
| 8–11: offered choices, later-task/late approval, loop re-audit | Scripted public handlers assert the actual offered choices and persisted policy, while no-terminal process cases prove no accidental prompt/provider launch. Native sequence/lifecycle cases distinguish first-task exact approval from later-task templates and reusable loop shapes from exact-iteration refusal. |
| 12–14: unchanged paths, runtime policy, direct targets | Exact/prefix controls, prohibited roots/initialization, filled-command built-in and user blacklists, later-chain rejection, curated redirect ordering, and interpreter-dependent target refusal remain covered. |
| 15: authorization versus results | Existing public composition and lifecycle tests check changed values, owner constraints, distinct exact/template modes, cached-result blacklist enforcement, and failed persistence releasing pending approvals. |
| 16: state, discovery, containers | Existing preflight/group matrices and container CLI tests cover lexical context, child source ownership, passive discovery, frozen membership, retry/proxy restrictions, and terminal event-time values. The new saved-policy/native-dispatch regression adds the missing non-`--yolo` nested terminal case. |
| 17: schema provenance | Named/local schema projections, fallback scalar typing, numeric/union broad approval with retained runtime validation, no numeric-to-alphanumeric coercion, and malformed persisted constraints are covered by policy/discovery and native CLI suites. |

In-process grammar, transformation, and policy checks are F1. Saved-file
round trips and the real CLI/native recorder cross the filesystem and OS
process-argument boundary (F2): the recorder itself reports the arguments it
received. Scripted approval is the boundary the specification requests; it
does not promise emulator styling or keyboard encoding. No requirement in
this slice needs a terminal/browser/device, so **L2 is not applicable** and
no windows were opened. Providers remain fixture executables, never live LLM
calls. Current macOS evidence is not a claim of a fresh cross-OS run; existing
portable recorder coverage remains available to the other OS environments.

## Verification

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(shell_template) | test(shell_missing) | test(shell_approval_prompt) | test(shell_expansion::site)' just test
just test sequence_shell_templates::a_nested_container_terminal_template
BISCUIT_TEST_FILTER='test(shell_template) | test(shell_missing) | test(shell_approval_prompt) | test(shell_expansion::site) | test(sequence_containers)' just test
```

Initial selection: **446 passed**, maximum **2.488 seconds**. Final expanded
parallel selection: **468 passed**, maximum **3.881 seconds**; test execution
took **51.014 seconds** overall. Log:
`/tmp/pipeline-s13-claudine-final.log`.

From `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(shell_template) | test(approval_prompt) | test(shell_expansion::site)' just test
```

**116 passed**, maximum **3.396 seconds**, parallel execution **4.144
seconds**. Log: `/tmp/pipeline-s13-darkmatter.log`. No selected test exceeded
five seconds. Compiler time is excluded from per-test timing. The compile
emitted the existing macOS linker warning; no new test compile warning was
introduced. `git diff --check` passed. No full-area suite, formatter, or
commit was run by this slice; the coordinator owns final package gates.
