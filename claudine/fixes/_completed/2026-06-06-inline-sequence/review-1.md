---
ready: false
agent: codex
model: ""
---

# Review: Inline Compose Sequence Mismatch

## Findings

### High: Plain and redirected diagnostics still emit terminal control sequences

The specification requires the diagnostic to remain plain and readable when styling or
hyperlinks are unavailable. The current tests cannot enforce that contract because they
strip SGR and OSC 8 sequences before asserting:

- [error.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1661)
- [inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:28)

A direct run with stderr redirected and `NO_COLOR=1` still emitted SGR color/reset
sequences and an OSC 8 file hyperlink. This contaminates CI/log output and contradicts
acceptance criterion 16. The plain-render test should assert that the unmodified output
contains no escape byte, and the CLI non-TTY test should inspect raw stderr before
normalization.

### High: TTY rendering requirements have no actual TTY or real-terminal verification

The TTY branch is selected using `std::io::stderr().is_terminal()` in
[compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/compose.rs:809),
but every new CLI test uses `assert_cmd` with piped stderr. The TTY tests construct the
error with `stderr_is_tty: true` and render it directly
([error.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/error.rs:1611)).
That proves string assembly only; it does not prove the command enters the TTY branch,
the top-level error walker preserves the custom YAML append, or the terminal displays
the OSC 8 link and YAML block correctly.

The test file also incorrectly describes its piped `assert_cmd` runs as Level 2
([inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:6)).
They are Level 1 process tests under the review taxonomy.

Required coverage:

- Level 1 PTY: run the real binary with stderr attached to a PTY and verify the TTY-only
  YAML branch, paragraph order, delimiter exclusion, and LF/CRLF payload handling.
- Level 2 real-terminal capture: verify the styled diagnostic and linked document render
  correctly through a supported terminal emulator.

No Level 3 coverage is required because this feature has no keyboard-input behavior.

### Medium: Several precedence acceptance criteria are only partially exercised

Acceptance criterion 7 requires empty and wrong-type non-null prompts with a non-null
sequence to produce the mismatch before prompt validation. Those cases only test the
library predicate in [mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/mismatch.rs:150);
there is no CLI test proving the observable diagnostic precedence.

The malformed-frontmatter test only checks that the mismatch marker is absent
([inline_compose_sequence_mismatch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/inline_compose_sequence_mismatch.rs:223)).
It does not assert that the existing frontmatter-parse diagnostic was retained.

Add Level 1 CLI cases for empty, numeric, collection, and mapping prompts paired with a
non-null sequence, and assert the positive parse-error identity for malformed YAML.

## Verification Levels

| Requirement | Strongest verification | Assessment |
|---|---|---|
| Non-null authored `prompt` + `sequence` rejection | Level 1 unit/process | Adequate |
| Null/absent-key behavior and override isolation | Level 1 unit/process | Adequate |
| Parse-error precedence | Level 1 process | Partial: only mismatch absence asserted |
| Fail-fast shell/provider/source behavior | Level 1 process | Adequate for tested surfaces |
| Non-TTY YAML withholding | Level 1 process | Content covered; raw plain-output fallback broken |
| TTY YAML inclusion and fidelity | Level 1 synthetic render | Gap: no real CLI PTY |
| Styling and OSC 8 document link | Level 1 synthetic render | Gap: requires Level 2 |
| Keyboard-driven behavior | Not applicable | No Level 3 needed |

## Verification

The prebuilt mismatch integration test binary passed all 13 tests. A direct redirected
CLI run returned exit code 1 and showed the expected mismatch/withholding prose, but also
showed the SGR and OSC 8 leak described above. A PTY smoke run showed the TTY YAML branch
and authored YAML content.

Current-source Cargo tests could not be compiled because this environment has no
installed rustup toolchain. The available binaries were built after the reviewed source
changes, but they do not replace a clean source build.

## Verdict

Not ready for production. The core mismatch detection and fail-fast placement are
correct, but the plain-output behavior is broken and the user-visible TTY rendering
contract lacks the required verification level.
