---
ready: false
agent: codex
model: ""
---

# Review: Inline Compose Sequence Mismatch

## Findings

### High: Non-TTY plain output still contains SGR and OSC 8 escapes

The plain-output tests normalize the evidence away before asserting it:

- [inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:28)
- [error.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1661)

A direct redirected run with `NO_COLOR=1` emitted 44 escape bytes, including
SGR styling and an OSC 8 file link. This violates acceptance criterion 16 and
contaminates CI/log output. The CLI test must inspect raw stderr and assert that
it contains no escape byte; the render test must make the same assertion
without calling `strip_escape_codes`.

### High: The TTY diagnostic still has no real-terminal verification

The command chooses the YAML branch from the actual stderr descriptor at
[compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs:812),
but the only TTY-path test constructs the error with `stderr_is_tty: true` and
calls the renderer directly
([error.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1611)).
The `assert_cmd` tests use piped stderr and are Level 1 process tests, despite
being described as Level 2
([inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:6)).

This does not verify that the real command enters the TTY branch, that the
top-level error walker preserves the appended YAML, or that styling and the
OSC 8 document link render correctly in a terminal emulator. Add a Level 1 PTY
test for branch selection and verbatim YAML, plus a Level 2 real-terminal
capture for the rendered diagnostic and link. No Level 3 test is required.

### Medium: Observable validation precedence remains partially tested

Acceptance criterion 7 requires empty and wrong-type non-null prompts combined
with a non-null sequence to produce the mismatch diagnostic before prompt
validation. Those cases exist only as library predicate tests; the CLI suite
does not prove the externally observable precedence.

The malformed-frontmatter test at
[inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:223)
only proves that the mismatch marker is absent. It does not positively assert
that the existing frontmatter-parse diagnostic was retained. Add Level 1 CLI
cases for empty, numeric, collection, and mapping prompts, and assert the
specific parse-error identity for malformed YAML.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Non-null authored `prompt` + `sequence` rejection | Level 1 unit/process | Adequate |
| Null/absent-key behavior and override isolation | Level 1 unit/process | Adequate |
| Parse and prompt-validation precedence | Level 1 process/predicate | Partial |
| Fail-fast shell/provider/source behavior | Level 1 process | Adequate |
| Non-TTY YAML withholding | Level 1 process | Content covered; plain fallback broken |
| TTY YAML inclusion and fidelity | Level 1 synthetic render | Gap: no CLI PTY or Level 2 capture |
| Styling and OSC 8 document link | Level 1 synthetic render | Gap: requires Level 2 |
| Keyboard-driven behavior | Not applicable | No Level 3 needed |

## Verification

The prebuilt integration-test binary passed all 13 tests. A direct redirected
CLI run returned exit code 1 with the expected mismatch and withholding prose,
but also emitted the escape sequences described above. `git diff --check`
passed.

Current-source Cargo tests could not be rebuilt because rustup has no configured
default toolchain. The existing binaries therefore provide behavioral evidence
but not a clean-build verification of the reviewed source.

## Verdict

Not ready for production. Detection and fail-fast placement match the
specification, but the plain-output contract is still broken and the TTY
diagnostic remains verified at the wrong test level.
