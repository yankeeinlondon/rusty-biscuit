---
ready: true
agent: codex
model: ""
---

# Review 3 — Frontmatter-Driven Interactive Sessions

## Findings

No production-blocking findings.

The prior review gaps appear to be addressed:

- The resolved interactive timeout conflict now checks both `--timeout` and `--step-timeout` against explicit CLI, frontmatter, and environment timeout sources in `claudine/cli/src/commands/wrap/composition/mod.rs`.
- The eager compose/inline-compose header now resolves the session badge from raw frontmatter, so `interactive: true` without `-i` no longer renders as non-interactive.
- The Level 2 PTY schema tests now assert provider non-launch after the prompt is visible but before input is submitted, which closes the earlier ordering hole for `compose`.
- `sequence` has both unit and CLI coverage for hard-rejecting authored `interactive: true` before any provider step launches.

## Verification Matrix

| Requirement | Strongest observed verification | Assessment |
|---|---:|---|
| `interactive` accepts only boolean/null and rejects other types with `InteractiveHintWrongType` | Level 1 unit tests in `claudine/lib/src/composition/prepare.rs` | OK |
| CLI precedence is `--no-interactive` > `--interactive` > frontmatter > default | Level 1 unit tests in `claudine/cli/src/commands/compose.rs` | OK |
| `-i` and `--no-interactive` are mutually exclusive | Level 1 clap parser test | OK |
| Frontmatter `interactive: true` selects interactive mode for `compose` | Level 1 CLI launch/header tests and Level 2 PTY schema tests | OK |
| `--no-interactive` overrides frontmatter `interactive: true` | Level 1 CLI test and Level 2 PTY schema test | OK |
| Resolved interactive mode conflicts with CLI `--step-timeout` | Level 1 CLI test in `compose_interactive_timeout_cli.rs` | OK |
| Resolved interactive mode conflicts with frontmatter `timeout` | Level 1 CLI test in `compose_interactive_timeout_cli.rs` | OK |
| `--no-interactive` + frontmatter `interactive: true` allows timeout use again | Level 1 CLI test in `compose_interactive_timeout_cli.rs` | OK |
| Eager header shows the interactive badge for frontmatter-driven interactive mode | Level 1 CLI rendered-output test | OK |
| Dry-run metadata includes resolved session mode/source | Level 1 render tests in `wrap/composition/dry_run.rs` | OK |
| `inline-compose` rejects unsupported interactive closure and names the resolved source | Level 1 tests for the typed error/rendered message | OK |
| `sequence` hard-rejects authored `interactive: true`, allows false/null/absent, and launches no provider on rejection | Level 1 unit and CLI integration tests | OK |
| Non-TTY missing schema values on `interactive: true` still report `MissingProperties` without launching | Level 1 CLI test in `compose_schema_cli.rs` | OK |
| TTY schema collection is independent of session mode and precedes provider launch for `compose -i` | Level 2 PTY test with pre-submit no-launch assertion | OK |
| TTY schema collection is independent of frontmatter-driven interactive mode and precedes provider launch | Level 2 PTY test with pre-submit no-launch assertion | OK |
| TTY schema collection still runs when `--no-interactive` overrides frontmatter | Level 2 PTY test with pre-submit no-launch assertion | OK |

## Notes

I did not find a mismatch requiring Level 3 verification. This feature resolves session mode from flags/frontmatter and verifies schema prompt ordering; it does not introduce a UX requirement about the terminal emulator's keyboard encoder, bare modifier presses, paste, mouse, or IME behavior.

One edge remains outside the explicit spec table: `--set` can alter effective frontmatter, while the eager header is based on raw frontmatter. I did not mark this as a finding because this feature's precedence contract names CLI flags, authored frontmatter, and default only. If `--set interactive=true` is intended to be supported as an override, it should get its own documented precedence rule and header/dry-run regression tests.

## Tests Run

- `cargo test -p claudine-cli --test compose_interactive_timeout_cli --color=never`
- `cargo test -p claudine-cli --test sequence_cli sequence_rejects_interactive_true_frontmatter_via_cli --color=never`
- `cargo test -p claudine --lib parse_interactive_hint --color=never`

I did not run the Level 2 suite locally; per `.claude/skills/rust-testing/SKILL.md`, those should be run via `just test-l2` because they depend on shared real-terminal harness setup.
