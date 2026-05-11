# Execution Plan: Set Key/Value Shorthand

Source: `claudine/features/_unscheduled/set-key-value/spec.md` and `tech-design.md`

## Phase 1 — Shared Parser (no CLI changes yet)

All work in `claudine/cli/src/commands/compose.rs`.

### Step 1.1: Add `parse_shorthand_value`

Add the function below the existing `parse_set_json`:

```rust
fn parse_shorthand_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::String(String::new());
    }
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) => parsed.value().clone(),
        Err(_) => serde_json::Value::String(raw.to_string()),
    }
}
```

Mirror of `darkmatter/cli/src/commands.rs:130-138`.

### Step 1.2: Add `parse_compose_setter`

```rust
fn parse_compose_setter(token: &str) -> Option<Result<(String, serde_json::Value), String>> {
    // Split on first '=', validate key regex, call parse_shorthand_value
}
```

Mirror of `darkmatter/cli/src/commands.rs:102-128`. Returns:
- `None` → not a setter (pass through as file candidate)
- `Some(Err)` → empty key (hard error)
- `Some(Ok)` → valid setter pair

### Step 1.3: Add `ParsedCompositionPositionals` struct and `parse_composition_positionals`

```rust
pub(crate) struct ParsedCompositionPositionals {
    pub file_ref: Option<String>,
    pub shorthand_setters: serde_json::Map<String, serde_json::Value>,
}
```

Iterate tokens, classify via `parse_compose_setter`, accumulate setters (last-write-wins), allow at most one file-ref candidate. Error on `=foo` (empty key). Mirror of `darkmatter/cli/src/commands.rs:140-172`.

### Step 1.4: Add `merge_set_overrides`

```rust
pub(crate) fn merge_set_overrides(
    raw_set: Option<&str>,
    shorthand: serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Value>>
```

1. Call existing `parse_set_json(raw_set)?` to get the `--set` base map
2. Insert shorthand pairs on top (shorthand wins)
3. Return `None` when empty, `Some(Value::Object(...))` otherwise

### Checkpoint 1

`cargo build -p claudine-cli` compiles. No behavioral changes yet — the new functions are unused.

---

## Phase 2 — Wire Compose Commands

### Step 2.1: Convert `ComposeArgs` to variadic positionals

In `compose.rs`, change:

```rust
// FROM
pub file: String,

// TO
#[arg(value_name = "ARG", num_args = 1..)]
pub args: Vec<String>,
```

Same change for `InlineComposeArgs`.

### Step 2.2: Update `run_compose_inner`

1. Replace `let ComposeArgs { shared, file } = args;` with destructuring `args` vec
2. Call `parse_composition_positionals(&args.args)?`
3. Require `file_ref.ok_or_else(|| eyre!("missing file reference: ..."))?`
4. Call `merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?`
5. Use the merged overrides where `set_overrides` was used before
6. Use `file_ref` where `file` was used before

### Step 2.3: Update `run_inline_compose_inner`

Same pattern as Step 2.2 but for inline-compose.

### Checkpoint 2

`cargo build -p claudine-cli` compiles. Manual smoke test:

```sh
claudine compose @prompts/some-test.md key=value --dry-run
```

Verify key appears in the composed frontmatter context.

---

## Phase 3 — Wire Sequence Command

### Step 3.1: Convert `SequenceArgs` to variadic positionals

In `sequence.rs`, change `file: String` to `args: Vec<String>` with the same `#[arg(...)]` attributes.

### Step 3.2: Update `run_sequence_inner`

1. Call `parse_composition_positionals(&args.args)?`
2. Require exactly one file ref
3. Call `merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?`
4. Pass merged overrides into `execute_sequence(...)` where `set_overrides` was used

Existing sequence overlay merge (`SequenceStepOverlay::as_set_overrides`) stays untouched — it already applies on top of user overrides, preserving reserved-key authority.

### Checkpoint 3

`cargo build -p claudine-cli` compiles.

---

## Phase 4 — Unit Tests

All tests in `claudine/cli/src/commands/compose.rs` (inline `#[cfg(test)]` module).

**Steps 4.1–4.4 are parallelizable** — they test independent functions.

### Step 4.1: Tests for `parse_shorthand_value`

| Case | Input | Expected |
|------|-------|----------|
| empty string | `""` | `String("")` |
| number | `"3"` | `Number(3)` |
| boolean | `"true"` | `Bool(true)` |
| array | `"[\"a\",\"b\"]"` | `Array(...)` |
| object | `"{mode:\"fast\"}"` | `Object(...)` |
| plain string | `"review.md"` | `String("review.md")` |
| URL with equals | `"https://x/?a=b"` | `String("https://x/?a=b")` |

### Step 4.2: Tests for `parse_compose_setter`

| Case | Token | Expected |
|------|-------|----------|
| valid setter | `"review=review.md"` | `Some(Ok(("review", String("review.md"))))` |
| underscore key | `"_private=true"` | `Some(Ok(("_private", Bool(true))))` |
| hyphen key | `"my-key=value"` | `Some(Ok(("my-key", String("value"))))` |
| empty value | `"key="` | `Some(Ok(("key", String(""))))` |
| first-eq split | `"url=https://x/?a=b"` | `Some(Ok(("url", String("https://x/?a=b"))))` |
| empty key | `"=foo"` | `Some(Err(...))` |
| digit-start key | `"9key=value"` | `None` |
| dot-path key | `"foo.bar=baz"` | `None` |
| slash key | `"/path=val"` | `None` |
| no equals | `"file.md"` | `None` |

### Step 4.3: Tests for `parse_composition_positionals`

| Case | Tokens | Expected |
|------|--------|----------|
| file only | `["file.md"]` | `file_ref=Some("file.md"), setters={}` |
| setter only | `["key=val"]` | `file_ref=None, setters={key:val}` |
| file + setter | `["file.md", "key=val"]` | `file_ref=Some("file.md"), setters={key:val}` |
| setter + file | `["key=val", "file.md"]` | same as above |
| multiple setters | `["a=1", "file.md", "b=2"]` | `file_ref=Some("file.md"), setters={a:1, b:2}` |
| duplicate setter | `["k=old", "k=new"]` | `setters={k:new}` (last wins) |
| two files | `["a.md", "b.md"]` | `Err` (multiple file candidates) |
| empty-key setter | `["=foo"]` | `Err` (empty key) |
| dot-path token | `["foo.bar=baz"]` | `file_ref=Some("foo.bar=baz"), setters={}` (classifier fallback) |

### Step 4.4: Tests for `merge_set_overrides`

| Case | `--set` | Shorthand | Expected |
|------|---------|-----------|----------|
| both empty | `None` | `{}` | `None` |
| set only | `Some(r#"{"a":"b"}"#)` | `{}` | `Some({"a":"b"})` |
| shorthand only | `None` | `{k:v}` | `Some({k:v})` |
| shorthand wins | `Some(r#"{"k":"old"}"#)` | `{k:new}` | `Some({k:new})` |
| disjoint merge | `Some(r#"{"a":"1"}"#)` | `{b:2}` | `Some({a:"1",b:2})` |

### Checkpoint 4

`cargo test -p claudine-cli -- compose` — all new unit tests pass.

---

## Phase 5 — Integration Tests

Work in existing test files. **Steps 5.1 and 5.2 are parallelizable.**

### Step 5.1: Extend `wrap_commands.rs`

Add tests that exercise the full CLI binary:

1. `compose` with shorthand overrides reaches the composition pipeline (verify via `--dry-run` output or exit code)
2. `inline-compose` with shorthand overrides (same approach)
3. Mixed `--set` and shorthand, verify shorthand wins
4. `=foo` produces a clear error message
5. Missing file ref produces a clear error message

### Step 5.2: Extend `sequence_cli.rs`

1. `sequence` with shorthand overrides
2. Verify reserved overlay keys (if testable at integration level) still beat user overrides

### Step 5.3: Update existing tests that assert on help text or `FILE` positional

Search for tests that match against the old `<FILE>` positional name and update them to expect `<ARG>...` or equivalent.

### Checkpoint 5

`cargo test -p claudine-cli` — all tests pass (new and existing).

---

## Phase 6 — Documentation

**Steps 6.1–6.4 are parallelizable.**

### Step 6.1: Update `claudine/docs/topics/composition.md`

Add a section on inline setters:
- Syntax and examples (`key=value` before or after file ref)
- Value parsing (JSON5 then string fallback)
- Precedence: inline > `--set` > frontmatter
- Key validation regex
- Note that `sequence` reserved overlay keys still win over both

### Step 6.2: Update `claudine/docs/cli/sequence.md`

Add inline setter examples. Note that reserved overlay keys are authoritative over both `--set` and shorthand.

### Step 6.3: Update `claudine/cli/README.md`

Update command synopsis from `<FILE>` to `<ARG>...` for compose, inline-compose, and sequence. Add setter examples.

### Step 6.4: Update doc comments in source

- `ComposeArgs` and `InlineComposeArgs` doc comments in `compose.rs`
- `SequenceArgs` doc comment in `sequence.rs`
- Command descriptions in `help.rs` only if the one-liner changes

### Step 6.5: Fix typos

The spec calls out correcting `inline-compse` and `sequences` typos in all user-facing docs and help text touched by this feature. Grep for these misspellings and fix any found.

### Checkpoint 6

`cargo build -p claudine-cli` (doc comments compile). Visual review of updated markdown files.

---

## Phase 7 — Final Validation

### Step 7.1: Full test suite

```sh
just -f claudine/justfile test
```

### Step 7.2: Lint

```sh
just -f claudine/justfile lint
```

### Step 7.3: Manual smoke tests

```sh
claudine compose @some/test.md review=review.md --dry-run
claudine compose review=review.md @some/test.md --dry-run
claudine inline-compose draft=false @some/test.md --dry-run
claudine sequence @some/seq.md topic="async" retries=3 --dry-run
claudine compose =foo  # expect error
claudine compose key=val  # expect missing-file error
claudine compose a.md b.md  # expect multiple-file error
```

---

## Dependency Graph

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 7
                                        │            │
                                     (4.1-4.4       (5.1-5.2
                                      parallel)      parallel)
                                        │
                                     Phase 6 ──────────────────► Phase 7
                                     (6.1-6.4
                                      parallel)
```

Phases 4 (unit tests) and 6 (docs) can run in parallel after Phase 3 completes. Phase 5 (integration tests) depends on Phase 4. Phase 7 gates on both 5 and 6.

## Files Modified

| File | Change |
|------|--------|
| `claudine/cli/src/commands/compose.rs` | Parser functions, struct changes, command wiring, unit tests |
| `claudine/cli/src/commands/sequence.rs` | Variadic args, positional parsing, merge wiring |
| `claudine/cli/tests/wrap_commands.rs` | Integration tests for compose/inline-compose |
| `claudine/cli/tests/sequence_cli.rs` | Integration tests for sequence |
| `claudine/docs/topics/composition.md` | Setter syntax, precedence, examples |
| `claudine/docs/cli/sequence.md` | Setter examples, overlay precedence note |
| `claudine/cli/README.md` | Updated synopsis and examples |
| `claudine/cli/src/commands/help.rs` | Only if command one-liners change |

## Risk Notes

1. **Darkmatter drift** — Mitigate by copying darkmatter's edge-case tests verbatim into Step 4.2/4.3.
2. **Existing test breakage** — The `<FILE>` → `<ARG>...` change will break any test asserting on help text. Step 5.3 addresses this.
3. **Clap no longer enforces file positional** — Command-level validation in Steps 2.2/2.3/3.2 replaces clap's structural enforcement with a clearer error message.
