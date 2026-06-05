---
ready: false
agent: codex
model: ""
---

# Review 3

## Findings

### High: directory hashing silently hashes unreadable or malformed files as empty documents

- Location: `darkmatter/cli/src/commands.rs:1430`
- Requirement: `md hash` operational errors such as malformed YAML or unreadable files must exit `1`; directory mode recursively hashes Markdown files, but it is still part of the same `md hash` user-facing contract.
- Current behavior: `run_hash_directory` maps each path through `Markdown::try_from(p.as_path()).unwrap_or_else(|_| Markdown::from(String::new()))`. Any per-file read or frontmatter parse error is discarded and replaced with an empty document, so the command can exit successfully with an aggregate hash that omits the real failing file content.
- Why this matters: a CI or release check using `md hash <dir>` can pass even when one Markdown file is unreadable or has malformed frontmatter. That directly violates the exit-code contract and creates a false baseline.
- Verification level: missing Level 1 CLI coverage. Add an integration test that creates a directory containing a `.md` file with malformed frontmatter and asserts `md hash <dir>` exits `1`; add a second test for an unreadable file if the platform can support it reliably, or cover the load-error branch at the library/helper level.
- Suggested fix: collect per-file results instead of swallowing them. Let `Markdown::try_from` errors propagate with the path context, and only compute the aggregate after every file has loaded and parsed successfully.

## Test Rigor

This feature is file, stdout, stderr, and exit-code behavior. Level 1 tests are the appropriate minimum; no Level 2 or Level 3 terminal verification is required because the spec does not define terminal rendering, key input, paste, mouse, glyph width, or terminal-emulator encoder behavior.

The prior Review 2 findings appear addressed at Level 1: literal heading source is now used for structured/detailed heading fingerprints, and detailed stored section levels are validated with CLI coverage for malformed levels.

The remaining finding is a Level 1 CLI gap: directory aggregate hashing does not preserve the operational-error contract for per-file load/parse failures.

## Production Readiness

Not ready for production. The core hashing paths are close, but directory hashing can currently report success while silently replacing a failing Markdown file with an empty document.

## Verification Notes

I attempted focused Level 1 test runs:

- `cargo test -p darkmatter hash:: --color=never`
- `cargo test -p darkmatter-cli test_hash_ --test cli --color=never`

Both commands were still compiling or waiting on Cargo locks after about 60 seconds, so I terminated the Cargo processes per the non-interactive session guidance. No passing test result is claimed from those runs.
