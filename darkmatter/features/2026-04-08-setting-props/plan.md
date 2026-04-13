# Implementation Plan: Compose Setter Shorthand

**Spec**: `darkmatter/features/2026-04-08-setting-props/spec.md`
**Tech Design**: `darkmatter/features/2026-04-08-setting-props/tech-design.md`

## Summary

Add `key=value` positional shorthand setters to `md compose` that translate into the existing `set_overrides` path. This is a pure CLI-layer change with zero library modifications.

## Confidence: High

- The library already supports `set_overrides` via `ComposeOptions::with_set_overrides(Value)`.
- The `--set` flag already parses JSON/JSON5 objects and feeds them into the same path.
- The compose pipeline applies `set_overrides` after `external_state` and before frontmatter interpolation — exactly the semantics we need.
- All that's required is changing how clap parses compose positionals and adding a classification step.

## Affected Files

| File | Change Type |
| --- | --- |
| `darkmatter/cli/src/args.rs` | Modify `Compose` variant |
| `darkmatter/cli/src/commands.rs` | Add helpers + modify `run_compose` dispatch |
| `darkmatter/cli/tests/cli.rs` | Add ~11 new tests |
| `darkmatter/docs/cli/compose.md` | Document shorthand syntax and precedence |

**No changes to**: `darkmatter/lib/` (compose library), `darkmatter/lib/src/markdown/compose/types.rs`, `darkmatter/lib/src/markdown/compose/mod.rs`.

## Step-by-Step Plan

### Step 1: Change compose positional to accept raw tokens

**File**: `darkmatter/cli/src/args.rs` (lines 80-145)

Change the `Compose` variant's `input: Option<PathBuf>` field to accept a raw vector of positional tokens:

```rust
Compose {
    /// Positional arguments: input path and/or key=value setters
    #[arg(value_name = "ARGS", num_args = 0..)]
    args: Vec<String>,

    // ... remaining flags unchanged
}
```

Key details:
- Remove `#[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]` from `input`.
- Replace `input: Option<PathBuf>` with `args: Vec<String>`.
- The `num_args = 0..` allows zero or more positionals (file optional for stdin).
- Remove the markdown file completer from this field (custom completion addressed in a follow-up if needed).

### Step 2: Add positional classification helpers

**File**: `darkmatter/cli/src/commands.rs`

Add three new functions in a dedicated module section (or at module level):

#### `parse_compose_setter(token: &str) -> Option<Result<(String, serde_json::Value)>>`

- Returns `None` if the token does not contain `=`.
- Returns `Some(Err)` if the key is empty or fails the grammar `[A-Za-z_][A-Za-z0-9_-]*`.
- Otherwise returns `Some(Ok((key, value)))`.
- Key grammar: first char `[A-Za-z_]`, subsequent chars `[A-Za-z0-9_-]`. Tokens with `/`, `:`, or other path-like chars in the key portion are **not** classified as setters.

#### `parse_shorthand_value(raw: &str) -> serde_json::Value`

- Try `biscuit_file::Json5::from_str(raw)`. If it succeeds, return `parsed.value().clone()`.
- Otherwise return `serde_json::Value::String(raw.to_string())`.
- Empty string (`foo=`) returns `Value::String("")`.

#### `parse_compose_positionals(args: &[String]) -> Result<ParsedComposeArgs>`

```rust
struct ParsedComposeArgs {
    input: Option<PathBuf>,
    shorthand_setters: serde_json::Map<String, serde_json::Value>,
}
```

- Iterate each token in `args`.
- For each token, call `parse_compose_setter`. If `Some(Ok(..))`, insert into the map (last-write-wins on duplicate keys). If `Some(Err(..))`, propagate the error.
- If `None` (not a setter), classify as an input path candidate. If an input was already found, return an error: "expected at most one input path, but got multiple: ...".
- Return `ParsedComposeArgs`.

### Step 3: Update `run_subcommand` to call the classifier

**File**: `darkmatter/cli/src/commands.rs` (lines 110-147)

In the `CliCommand::Compose` match arm:

1. Call `parse_compose_positionals(&args)` to get `ParsedComposeArgs`.
2. Pass `parsed.input.as_ref()` instead of the old `input.as_ref()`.
3. Pass the `shorthand_setters` map to `run_compose` (or merge inline).

### Step 4: Merge shorthand setters into `set_overrides` in `run_compose`

**File**: `darkmatter/cli/src/commands.rs` (around line 389-415)

After parsing `--set` into the override map (or creating an empty map), overlay the shorthand setters:

```rust
let mut override_map: serde_json::Map<String, serde_json::Value> = if let Some(json_str) = set_json {
    let parsed = biscuit_file::Json5::from_str(json_str)?;
    let set = parsed.value().clone();
    if let serde_json::Value::Object(map) = set {
        map
    } else {
        return Err(eyre!("Invalid --set argument: expected a JSON object"));
    }
} else {
    serde_json::Map::new()
};

for (key, value) in shorthand_setters {
    override_map.insert(key, value);
}

if !override_map.is_empty() {
    options = options.with_set_overrides(serde_json::Value::Object(override_map));
}
```

This ensures:
- `--set` keys are preserved.
- Shorthand keys overwrite `--set` keys on collision (shorthand wins).
- No change to `--state` semantics.

### Step 5: Add `shorthand_setters` parameter to `run_compose`

**File**: `darkmatter/cli/src/commands.rs` (line 332)

Add `shorthand_setters: serde_json::Map<String, serde_json::Value>` to the `run_compose` function signature and thread it through from `run_subcommand`.

### Step 6: Add CLI integration tests

**File**: `darkmatter/cli/tests/cli.rs`

Add the following tests (numbered per tech design):

1. **Basic shorthand on file input**: `md compose doc.md iteration=1` — verify `{{iteration}}` resolves to `1`.
2. **Basic shorthand with stdin**: `cat doc.md | md compose iteration=1` — same with piped input.
3. **Multiple setters, mixed types**: `iteration=1 draft=false name=Alice` — verify all resolve.
4. **JSON5 value**: `meta={author:"Alice"}` — verify parsed as object.
5. **Shorthand participates in validation**: Create template with `::file features/{{plan}}`, use `plan=my-plan.md` shorthand, verify transclusion resolves.
6. **Shorthand wins over `--state`**: `--state '{"iteration":0}' iteration=1` — result is `1`.
7. **Shorthand wins over `--set`**: `--set '{"iteration":1}' iteration=2` — result is `2`.
8. **Duplicate keys last-write-wins**: `iteration=1 iteration=2` — result is `2`.
9. **Empty value**: `empty=` — resolves to empty string.
10. **Multiple non-setter tokens error**: `md compose doc.md other.md` — expect failure with clear error.
11. **Path escape hatch**: `./foo=bar.md` treated as input, not setter.

### Step 7: Update documentation

**File**: `darkmatter/docs/cli/compose.md`

1. Add shorthand syntax examples to the Usage section.
2. Add a Precedence section: `--state` (fills defaults) < `--set` (overrides) < shorthand (overrides both).
3. Document top-level-key limitation.
4. Document filename escape hatch for ambiguous `foo=bar.md` paths.
5. Document value parsing: JSON5 with string fallback.

### Step 8: Update unit tests in `args.rs`

**File**: `darkmatter/cli/src/args.rs` (around line 620-635)

Update the existing `compose_perf_flag_sets_true` and `compose_without_perf_defaults_false` tests to use the new `args: Vec<String>` field instead of `input: Option<PathBuf>`.

## Precedence (confirmed from code)

1. Document frontmatter
2. `--state` defaults (fills missing/null keys)
3. `--set` overrides (unconditional overwrite)
4. Shorthand `key=value` overrides (unconditional overwrite, highest CLI priority)

This matches the existing `--set` behavior because shorthand values are merged into the same `set_overrides` map.

## Risk Assessment

- **Low risk**: No library changes. The compose pipeline already handles `set_overrides` correctly.
- **Clap migration**: Changing from `Option<PathBuf>` to `Vec<String>` is a well-understood clap pattern.
- **Backward compatibility**: All existing `--state`, `--set`, and positional-input invocations continue to work identically.
- **Edge cases**: The `./foo=bar.md` path disambiguation is handled by the key grammar (rejects keys containing `/`).

## Verification Checklist

- [ ] All existing tests pass (including compose `--state` and `--set` tests)
- [ ] New shorthand tests pass
- [ ] `cargo fmt --package darkmatter-cli` clean
- [ ] `cargo build -p darkmatter-cli` clean
- [ ] Documentation updated
