---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: Compose PTY tests do not actually prove schema collection happens before provider launch

The spec requires schema-required collection to run before the provider session starts, regardless of whether the resolved session is interactive. The new Level 2 compose tests are named for that guarantee, but they only wait for the prompt label, submit an answer, and then assert that the provider marker exists afterward:

- `level2_pty_schema_prompt_precedes_provider_launch_with_interactive_flag` waits for `topic` and raw mode, writes `async\r`, then checks `marker.exists()` at [claudine/cli/tests/level2_schema_prompt_pty.rs:607](../../cli/tests/level2_schema_prompt_pty.rs:607) and [claudine/cli/tests/level2_schema_prompt_pty.rs:622](../../cli/tests/level2_schema_prompt_pty.rs:622).
- `level2_pty_schema_prompt_precedes_provider_launch_with_frontmatter_interactive` follows the same pattern at [claudine/cli/tests/level2_schema_prompt_pty.rs:662](../../cli/tests/level2_schema_prompt_pty.rs:662) and [claudine/cli/tests/level2_schema_prompt_pty.rs:677](../../cli/tests/level2_schema_prompt_pty.rs:677).
- `level2_pty_schema_prompt_appears_even_when_no_interactive_overrides_frontmatter` follows the same pattern at [claudine/cli/tests/level2_schema_prompt_pty.rs:737](../../cli/tests/level2_schema_prompt_pty.rs:737) and [claudine/cli/tests/level2_schema_prompt_pty.rs:752](../../cli/tests/level2_schema_prompt_pty.rs:752).

These tests would still pass if a regression launched the provider immediately after rendering the prompt but before the user answered, because none of them asserts `!marker.exists()` before sending input. The sequence L2 test already has the right shape: it asserts no provider launch after the prompt appears and before submission at [claudine/cli/tests/level2_schema_prompt_pty.rs:849](../../cli/tests/level2_schema_prompt_pty.rs:849).

Verification level: intended Level 2, but the assertion does not verify the required ordering. Because this is a user-observable session/TTY ownership guarantee, I would not mark the feature production-ready until these compose L2 tests assert no launch before prompt submission for all three resolved-mode cases.

### Medium: Early compose headers still render from the raw `--interactive` flag, not resolved interactivity

`compose` and `inline-compose` eagerly render the execution header before prepare/execution, but both pass `shared.interactive` into `emit_execution_header`:

- [claudine/cli/src/commands/compose.rs:493](../../cli/src/commands/compose.rs:493) through [claudine/cli/src/commands/compose.rs:505](../../cli/src/commands/compose.rs:505)
- [claudine/cli/src/commands/compose.rs:966](../../cli/src/commands/compose.rs:966) through [claudine/cli/src/commands/compose.rs:975](../../cli/src/commands/compose.rs:975)

The executor's fallback header uses `request.session_interactive` correctly at [claudine/cli/src/commands/wrap/composition/mod.rs:1152](../../cli/src/commands/wrap/composition/mod.rs:1152), but direct compose/inline-compose set `header_emitted = true`, so that corrected path is skipped. As a result, a document with `interactive: true` and no `-i` can run as interactive while the first status line describes the run as non-interactive. This contradicts the feature's goal of making the prompt's session intent visible and diagnosable.

Fix direction: either delay the eager header until after `prepare_*_with_schema` resolves `prepared.selection_hints.interactive`, or compute a raw-frontmatter provisional interactivity value for the early header and reconcile it with the final prepared value. Add a CLI/PTY-visible test for the header text on `interactive: true` without `-i`.

Verification level: currently no test covers this header path for frontmatter-driven interactivity. Because it is terminal-rendered status text, Level 2 capture would be the strongest fit if exact rendered text/style matters; a Level 1 process assertion on stripped stderr is acceptable if the contract is only the mode label.

## Verification Matrix

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| `interactive` parses only boolean/null and rejects wrong types | Level 1 unit tests in `prepare.rs` | OK |
| CLI precedence: `--no-interactive` > `--interactive` > frontmatter > default | Level 1 unit tests in `compose.rs` | OK |
| Frontmatter `interactive: true` drives timeout conflicts after resolution | Level 1 integration tests in `compose_interactive_timeout_cli.rs` | OK |
| `--no-interactive` overrides frontmatter and allows timeouts | Level 1 integration test | OK |
| `inline-compose` rejects unsupported interactive closure and names source | Level 1 tests present for error rendering and existing CLI path | OK, but frontmatter CLI coverage would be useful |
| `sequence` rejects authored `interactive: true`; allows false/null/absent | Level 1 unit and CLI tests | OK |
| Dry-run includes resolved session mode/source | Level 1 dry-run renderer tests | OK |
| Schema prompt appears in TTY for `-i`, frontmatter `interactive: true`, and `--no-interactive` override | Level 2 PTY tests | OK for prompt appearance and successful collection |
| Schema prompt completes before provider launch for compose | Level 2 PTY tests intended, but missing pre-submit no-launch assertion | Gap |
| Early execution header reflects resolved frontmatter interactivity | No coverage found | Gap |

## Tests Run

- `cargo test -p claudine-cli --test compose_interactive_timeout_cli --color=never`
- `cargo test -p claudine --lib parse_interactive_hint --color=never`

Both passed.
