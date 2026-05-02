---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 9)

## Summary

The Review 8 hotkey fixes are in place: forced badge modes now survive modifier events, and multi-character/empty hotkey specs are rejected. The default package test suite passes, including the currently enabled keyboard/completion PTY-style tests.

I found two remaining correctness gaps. Both are scoped, but they affect public CLI behavior, so I would not mark the feature production-ready yet.

## Findings

### 1. `--file` still accepts unsupported/plain-list files instead of enforcing structured array sources

The spec limits `--file` to JSON, JSONL, NDJSON, YAML, CSV, or TOML, and says the file must be structured as an array or an error is raised. The current parser accepts any unknown extension, sniffs the body, and for ordinary text falls back to `parse_list`.

That means `question choose-one --file options.txt` containing:

```text
Red
Green
Blue
```

is accepted as a valid option source, even though it is neither one of the supported formats nor an array-shaped source. This also leaves a test gap: existing file tests cover happy paths and non-array JSON/TOML, but not unsupported extensions or plain text fallback.

Evidence:

- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:185` lists `--file`.
- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:187` restricts file formats and requires array-shaped content.
- `biscuit-tui/cli/src/option_sources.rs:209` dispatches known extensions.
- `biscuit-tui/cli/src/option_sources.rs:215` through `:223` accepts unknown extensions and plain text by returning `Ok(parse_list(&body))`.

Recommendation: remove the unknown-extension plain-list fallback. Return a clear unsupported-format error for extensions outside the spec, or at minimum require sniffed unknown files to parse as JSON/YAML arrays and reject plain text. Add CLI/unit tests for `.txt` plain-list input and unsupported extensions.

### 2. Explicit CLI hotkeys can be shadowed by earlier default Ctrl hotkeys without a duplicate error

The design requires duplicate hotkeys to be rejected at CLI parsing time. The CLI duplicate check only looks at parsed explicit/numeric hotkeys. Later, library construction calls `ChoiceOption::effective_hotkey`, which gives every non-disabled option a default `Ctrl+<first alphanumeric label char>` when no explicit hotkey is set. The effective map is first-wins.

This creates a real collision path that the CLI cannot diagnose:

```text
question choose-one "Red" "[CTRL+R] Rose"
```

`Red` has no parsed hotkey, so it passes the CLI duplicate check. In the library, `Red` receives default `Ctrl+r`, then `Rose`'s explicit `Ctrl+r` is ignored by `or_insert`. Pressing `Ctrl+R` selects `Red`, not the explicitly configured `Rose`.

Evidence:

- `biscuit-tui/features/2026-04-28-choose-one-improvements/tech-design.md:339` through `:341` requires duplicate hotkeys to be rejected at CLI parsing time.
- `biscuit-tui/cli/src/choice_normalize.rs:319` through `:333` checks only `ParsedOption.hotkey`.
- `biscuit-tui/lib/src/components/choose.rs:136` through `:148` creates default Ctrl hotkeys from labels.
- `biscuit-tui/lib/src/components/choose_one.rs:852` through `:857` inserts effective Ctrl hotkeys first-wins; `ChooseMany` uses the same helper.

Recommendation: make the CLI duplicate check operate on effective hotkeys, not just explicit hotkeys, or change precedence so explicit hotkeys win over default label-derived hotkeys. Add tests covering an implicit/explicit collision for both `choose-one` and `choose-many`.

## Test Results

Passed:

```text
cargo test -p tui-chrome -p tui-chrome-cli
```

## Production Readiness

Not ready for production. The broad feature is close, but the remaining CLI source-contract gap and hotkey collision behavior should be corrected before release.
