---
ready: true
agent: codex
model: ""
---

# Review 4

## Findings

No blocking findings.

The iteration-3 blocker is resolved: the Level 2 helper now evaluates the
WezTerm Level 2 gate before resolving the `md` shim, and shim creation no
longer depends on symlink privileges. `run_md_env_bin` takes a lazy binary
closure, `run_md_env` passes `md_shim` without invoking it early, and
`link_or_copy` falls back from symlink to hard link to copy. The integrity
check now uses `is_same_binary`, so the same assertion works for symlink,
hard-link, and copied shims.

## Requirement Coverage

- God-file decomposition: implemented. The original CLI source and test
  god-files have been split into responsibility-focused modules. Remaining
  over-cap files are explicitly documented exceptions in the spec and are
  reported by `just lint-files` as accepted rather than silently ignored.
- Library-leak extraction: implemented for the specified surfaces. The CLI
  now uses library/renderable-owned color parsing, Tailwind lookup, TOC,
  delta, validation view, style claims, and reference JSON serialization
  surfaces instead of re-owning those behaviors in the CLI.
- JSON compatibility: Level 1 integration tests compare full normalized JSON
  values for `md validate refs --format json` and `md graph --json` against
  captured baselines. Covered cases include local paths, remote URLs,
  fragments, data URIs, inline records, validation errors, graph follow, and
  graph validation output.
- Delta text compatibility: Level 1 golden tests cover no-change,
  frontmatter-only, preamble-only, section add/remove/modify/move,
  whitespace-only, code-block content/language changes, and verbose visual
  diff output.
- Real-terminal rendering behavior: existing and split `level2_*` tests remain
  at Level 2, using WezTerm capture through the shared harness. The helper now
  invokes the Cargo-built `md` shim instead of a host `PATH` binary.
- Harness integrity: Level 1 structural tests cover shim identity, valid shim
  acceptance, foreign shim rejection, and absolute temp-dir shim paths. These
  are correctly outside the `level2_` filter because they test filesystem
  helper behavior rather than real-terminal rendering.
- Level 3: not required. This feature does not specify keyboard, mouse,
  paste, IME, or terminal input-encoder behavior.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---:|---|
| CLI JSON shape preservation | Level 1 | Appropriate: subprocess integration plus full JSON baseline comparison. |
| Delta terminal text shape | Level 1 | Appropriate for deterministic CLI stdout bytes; no real terminal encoder behavior is required. |
| Layout/styling/disclosure/table/image/list real-terminal rendering | Level 2 | Appropriate: real WezTerm capture verifies rendering through a terminal emulator. |
| L2 harness runs the built binary | Level 1 structural plus Level 2 consumers | Appropriate: structural tests pin shim identity, and all L2 helpers consume that shim. |
| L2 skip behavior before shim setup | Level 1 structural/code review | Appropriate: the lazy closure makes the ordering explicit and testable without requiring an absent WezTerm host. |

## Verification Run

- `cargo test -p darkmatter-cli --test level2_harness_integrity --color=never`
  passed on macOS: 4 passed.
- `cargo test -p darkmatter-cli --test validate_refs --test graph --color=never`
  passed on macOS: 26 passed.
- `just lint-files` passed; all over-cap files were documented accepted
  exceptions.
- `cargo nextest list -p darkmatter-cli -E 'test(/level2_/)' --color=never`
  did not include `level2_harness_integrity`.
- `cargo nextest list -p darkmatter-cli -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))' --color=never`
  included the four `level2_harness_integrity` structural tests.

## Residual Risk

I did not run the full workspace suite or `just test-l2`; the focused checks
above cover the iteration-4 changes and the prior review blocker. Cross-platform
behavior was reviewed from the code path, but only macOS was executed in this
session.
