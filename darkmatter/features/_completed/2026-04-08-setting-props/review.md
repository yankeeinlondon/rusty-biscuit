# Review: Compose Setter Shorthand

**Reviewed against**: `spec.md`, `tech-design.md`, `plan.md`

## Implementation Completeness

### Fully Implemented

- **args.rs**: `input: Option<PathBuf>` replaced with `args: Vec<String>` and `num_args = 0..` clap annotation. Markdown file completer removed. Existing args.rs tests updated and new clap-parsing tests added (`compose_args_captures_positional_tokens`, `compose_args_empty_when_no_positionals`).
- **commands.rs**: `ParsedComposeArgs` struct, `parse_compose_setter`, `parse_shorthand_value`, and `parse_compose_positionals` helpers added. `run_subcommand` dispatches through the classifier. `run_compose` accepts `shorthand_setters` and merges them into `override_map` after `--set` and before `with_set_overrides`.
- **Integration tests (cli.rs)**: All 11 planned tests implemented (tests 1-11 from plan Step 6).
- **docs/cli/compose.md**: Shorthand syntax section, key grammar, value parsing, precedence section, and filename escape hatch all documented.

### Gaps

#### 1. CLI README not updated

Tech design explicitly lists `darkmatter/cli/README.md` as a documentation target (line 355). The Compose Pipeline section of `darkmatter/cli/README.md` does not mention shorthand setters anywhere. It still only shows `--state` and `--set` usage examples.

**Recommendation**: Add shorthand examples to the Compose Pipeline section of `darkmatter/cli/README.md`, e.g.:

```bash
# Shorthand overrides
md compose doc.md iteration=1 draft=false name=Alice
```

#### 2. No unit tests for the three parsing helpers

The tech design (line 243) states the helpers should be "directly unit-testable." The `#[cfg(test)] mod tests` block in `commands.rs` contains zero tests for `parse_compose_setter`, `parse_shorthand_value`, or `parse_compose_positionals`. All test coverage is integration-level (subprocess `assert_cmd` tests in `cli.rs`).

Unit tests would be more targeted, faster, and provide better failure diagnostics for edge cases like:

- Empty key: `=value` returns `Some(Err(..))`
- Invalid first char: `9key=val` returns `None` (not a setter)
- Hyphenated key: `my-key=val` returns `Some(Ok(..))`
- Key with underscore: `_private=val` returns `Some(Ok(..))`
- Key with path separator: `path/key=val` returns `None`
- Value parsing: `true` becomes `Value::Bool(true)`, `hello` becomes `Value::String("hello")`, `null` becomes `Value::Null`
- `parse_compose_positionals` with empty vec returns empty setters and no input
- `parse_compose_positionals` with setter-only tokens returns no input

**Recommendation**: Add a dedicated test module in `commands.rs` covering the key grammar boundary, value parsing boundary, and full positional classification. This is the most important test gap.

#### 3. Missing edge-case integration tests

Three scenarios from the tech design are not covered by integration tests:

| Scenario | Tech design ref | Current coverage |
| --- | --- | --- |
| `=value` (empty key error) | Error handling item 4 | No test |
| Invalid key like `9key=val` treated as input path | Key grammar section | No test |
| Setter before file path: `md compose iteration=1 doc.md` | Input resolution section, line 147 | No test |

**Recommendation**: Add these three integration tests to `cli.rs`.

#### 4. Shell completion regression

The compose positional previously had `ArgValueCompleter::new(complete_markdown_files)`. This was removed without replacement. The tech design (line 209-214) acknowledges this and specifies a compose-specific completion strategy:

> if the current token already contains `=`, do not suggest files
> otherwise, suggest markdown files and `-`

The current implementation provides **no file completion** for the compose positional. This is a user-experience regression for anyone relying on tab completion to select compose input files.

**Recommendation**: Add a custom completer for the compose `args` field that suggests markdown files and `-` when the current token does not contain `=`, or document this as a known limitation to be addressed in a follow-up.

## Code Quality

### Precedence correctness

The merge order is correct. `override_map` is built from `--set` first, then shorthand setters overwrite via `insert`:

```rust
for (key, value) in shorthand_setters {
    override_map.insert(key, value);
}
```

This correctly implements the precedence: `--state` < `--set` < shorthand.

### Key grammar implementation

The grammar check in `parse_compose_setter` (commands.rs:112-123) correctly implements `[A-Za-z_][A-Za-z0-9_-]*`. Tokens with `/`, `:`, or other path-like chars in the key portion return `None` and fall through to input-path classification. This correctly handles the `./foo=bar.md` escape hatch.

### Value parsing

`parse_shorthand_value` correctly:
- Returns empty string for empty RHS
- Tries JSON5 parse first
- Falls back to string on JSON5 failure

### Error message quality

The "multiple input paths" error message reconstructs all non-setter tokens, which is helpful:

```rust
args.iter()
    .filter(|t| parse_compose_setter(t).is_none())
    .map(|t| t.as_str())
    .collect::<Vec<_>>()
    .join(", ")
```

### Ergonomic concern: `run_compose` parameter count

`run_compose` already had `#[allow(clippy::too_many_arguments)]` at 16 parameters. The feature adds one more (17). This is a pre-existing concern, not introduced by this feature, but it continues a trend. A future refactor could consider grouping related parameters into a struct (e.g., `ComposeOverrides { state, set, shorthand_setters }`).

## Summary

| Category | Status |
| --- | --- |
| Functional correctness | Complete and correct |
| Plan test coverage (11 tests) | All 11 implemented |
| Unit test coverage for parsing helpers | Missing |
| CLI README documentation | Not updated |
| Shell completion | Regressed (removed, not replaced) |
| Edge-case integration tests | 3 missing (empty key, numeric-first key, setter-before-path) |

### Priority Recommendations

1. **Add unit tests** for `parse_compose_setter`, `parse_shorthand_value`, and `parse_compose_positionals` in `commands.rs` (high priority — core parsing logic has no direct tests).
2. **Update `darkmatter/cli/README.md`** with shorthand examples in the Compose Pipeline section (medium priority — documentation gap per tech design).
3. **Add 3 edge-case integration tests** for empty key error, numeric-first key, and setter-before-path ordering (medium priority — boundary behavior coverage).
4. **Restore or replace shell completion** for compose file arguments (low priority — UX regression but can be a follow-up).
