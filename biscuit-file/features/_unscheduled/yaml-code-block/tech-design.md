# Technical Design: YAML Markdown Blocks

This document complements the functional specification for adding Markdown-oriented YAML rendering helpers to `biscuit-file`.

## Goals

- Add `Yaml::as_yaml_code_block()` for rendering a parsed YAML document inside a valid fenced Markdown code block.
- Add `Yaml::as_frontmatter_block()` for rendering a parsed YAML mapping as a Markdown frontmatter block.
- Preserve existing `serde_yaml_ng` formatting behavior used elsewhere in `biscuit-file`.
- Keep the feature in the existing `yaml` feature surface with no new dependencies.
- Return precise `YamlError` values instead of panicking or emitting structurally invalid Markdown.

## Non-Goals

- No CLI flags are required for this feature.
- No alternate frontmatter delimiters are required.
- No custom YAML serializer or configurable formatting options are required.
- No support for Markdown frontmatter extraction changes is required.

## Public API

Add the following inherent methods to `Yaml` in `biscuit-file/lib/src/yaml/types.rs`:

```rust
impl Yaml {
    /// Returns the YAML content as a Markdown fenced code block.
    ///
    /// ## Errors
    ///
    /// Returns an error if the YAML value cannot be serialized.
    pub fn as_yaml_code_block(&self) -> Result<String, YamlError>;

    /// Returns the YAML content as a Markdown frontmatter block.
    ///
    /// ## Errors
    ///
    /// Returns an error if the YAML value is not a mapping or cannot be serialized.
    pub fn as_frontmatter_block(&self) -> Result<String, YamlError>;
}
```

These methods should be available whenever the `yaml` feature is enabled because `Yaml` itself is only compiled behind that feature.

## Error Model

`serde_yaml_ng::to_string()` already returns `serde_yaml_ng::Error`, and `YamlError::Parse(#[from] serde_yaml_ng::Error)` currently accepts that error type. The implementation can therefore use `?` for serialization failures without adding a separate serialization variant.

For frontmatter shape validation, add a dedicated variant to `YamlError`:

```rust
#[error("Frontmatter must be a YAML mapping, found {found}")]
FrontmatterNotMapping { found: &'static str },
```

This keeps the failure distinct from schema validation and conversion errors. The crate is still `0.1.0`, so adding a public enum variant is acceptable. If the crate later stabilizes the enum, it should be made `#[non_exhaustive]` before adding more variants.

Add a small helper for diagnostic names:

```rust
fn yaml_kind(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "null",
        serde_yaml_ng::Value::Bool(_) => "boolean",
        serde_yaml_ng::Value::Number(_) => "number",
        serde_yaml_ng::Value::String(_) => "string",
        serde_yaml_ng::Value::Sequence(_) => "sequence",
        serde_yaml_ng::Value::Mapping(_) => "mapping",
        serde_yaml_ng::Value::Tagged(_) => "tagged",
    }
}
```

`as_frontmatter_block()` should validate the top-level value before serialization:

```rust
if !matches!(self.value, serde_yaml_ng::Value::Mapping(_)) {
    return Err(YamlError::FrontmatterNotMapping {
        found: yaml_kind(&self.value),
    });
}
```

Tagged mappings should not be accepted implicitly. A tagged value is not itself a mapping at the top level, and preserving tags inside Markdown frontmatter is not consistently supported by downstream parsers.

## Serialization

Both new methods should serialize from the parsed value:

```rust
let yaml = serde_yaml_ng::to_string(&self.value)?;
```

This matches existing `biscuit-file` behavior in the CLI and conversion code. It also avoids returning the original raw source, which may have inconsistent indentation, comments, or formatting unrelated to the normalized parsed value.

The implementation should normalize only the wrapper boundary, not the YAML body:

- Do not trim leading whitespace from the serialized YAML.
- Do not trim or rewrite scalar styles.
- Preserve the serializer-provided trailing newline.
- If the serializer ever returns a string without a trailing newline, insert one before the closing delimiter.

A small helper keeps wrapper formatting consistent:

```rust
fn ensure_trailing_newline(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
}
```

## Code Block Fence Selection

`as_yaml_code_block()` must choose a fence length that cannot collide with backtick runs in the serialized YAML body.

Algorithm:

1. Serialize the YAML value with `serde_yaml_ng::to_string`.
2. Scan the serialized YAML for the longest contiguous run of backticks.
3. Use `max(3, longest_run + 1)` backticks for the outer fence.
4. Emit the opening fence with the `yaml` language tag.
5. Emit the serialized YAML body.
6. Ensure exactly one line boundary before the closing fence.
7. Emit the closing fence.

```rust
fn longest_backtick_run(input: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;

    for ch in input.chars() {
        if ch == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }

    longest
}
```

Example output when the body has no collision:

````markdown
```yaml
key: value
```
````

Example output when the body contains a triple-backtick run:

`````markdown
````yaml
snippet: "```rust\nfn main() {}\n```"
````
`````

## Frontmatter Formatting

`as_frontmatter_block()` should use the canonical Markdown frontmatter delimiter:

```text
---
<serialized yaml>
---
```

Implementation outline:

```rust
pub fn as_frontmatter_block(&self) -> Result<String, YamlError> {
    if !matches!(self.value, serde_yaml_ng::Value::Mapping(_)) {
        return Err(YamlError::FrontmatterNotMapping {
            found: yaml_kind(&self.value),
        });
    }

    let yaml = serde_yaml_ng::to_string(&self.value)?;
    let mut output = String::from("---\n");
    output.push_str(&yaml);
    ensure_trailing_newline(&mut output);
    output.push_str("---\n");
    Ok(output)
}
```

This allows nested sequences and mappings inside frontmatter as long as the top-level YAML value is a mapping.

## Module Placement

Keep the implementation local to `biscuit-file/lib/src/yaml/types.rs`:

- Add the two public methods inside the existing `impl Yaml` block near `as_json()` and `as_toml()`.
- Add private helpers near the existing private conversion helpers.
- Add tests in the existing `#[cfg(test)] mod tests` in the same file.

No changes are needed in `biscuit-file/lib/src/yaml/mod.rs` or `biscuit-file/lib/src/lib.rs` because the methods are inherent on the already exported `Yaml` type.

## Test Plan

Add focused unit tests in `biscuit-file/lib/src/yaml/types.rs`.

### Code Block Tests

- `as_yaml_code_block_basic`
  - Input: a mapping with nested values.
  - Assert output starts with ```` ```yaml\n ````.
  - Assert output includes the serialized keys.
  - Assert output ends with ```` ```\n ````.

- `as_yaml_code_block_increases_fence_for_triple_backticks`
  - Input: mapping with a string containing ```` ``` ````.
  - Assert output starts with ```` ````yaml\n ````.
  - Assert output ends with ```` ````\n ````.

- `as_yaml_code_block_uses_one_more_than_longest_run`
  - Input: string containing four contiguous backticks.
  - Assert the opening and closing fences contain five backticks.

- `as_yaml_code_block_allows_scalar_and_sequence`
  - Input: scalar and sequence documents.
  - Assert both produce fenced YAML blocks.

### Frontmatter Tests

- `as_frontmatter_block_mapping`
  - Input: a top-level mapping.
  - Assert output starts with `---\n`.
  - Assert output includes serialized mapping content.
  - Assert output ends with `---\n`.

- `as_frontmatter_block_allows_empty_mapping`
  - Input: `{}`.
  - Assert output is structurally valid frontmatter.

- `as_frontmatter_block_rejects_scalar`
  - Input: `"just a string"`.
  - Assert `matches!(err, YamlError::FrontmatterNotMapping { found: "string" })`.

- `as_frontmatter_block_rejects_sequence`
  - Input: `- one`.
  - Assert `matches!(err, YamlError::FrontmatterNotMapping { found: "sequence" })`.

- `as_frontmatter_block_rejects_tagged_value`
  - Input: a tagged top-level value if `serde_yaml_ng` parses it as `Value::Tagged`.
  - Assert the error reports `found: "tagged"`.

## Documentation Updates

When implementing the feature, update:

- `biscuit-file/lib/README.md`
  - Add the two methods to the `Yaml` example.
- `biscuit-file/README.md`
  - Mention Markdown YAML block generation in the functional overview if this becomes part of public behavior.
- `.claude/skills/biscuit-file/references/api.md`
  - Add `Yaml::as_yaml_code_block()` and `Yaml::as_frontmatter_block()` to the YAML conversion section.
- `.claude/skills/biscuit-file/references/format-conversion.md`
  - Mention Markdown wrapper generation as YAML rendering, not as a new data-format conversion.

No dependency documentation changes are needed because the design adds no crates.

## Verification Commands

Run from `biscuit-file/`:

```sh
just test
just lint
```

If a narrower loop is needed during implementation, run from the repository root:

```sh
cargo test -p biscuit-file yaml::types
```

## Implementation Notes

- Do not use `unwrap()` or `expect()` in production code.
- Keep helpers private until another module needs them.
- Mark the two new methods `#[instrument(level = "trace", skip(self), fields(source = ?self.source))]` only if the added tracing is useful. Existing conversion methods use tracing on public conversion entry points, so matching that style is reasonable.
- Use rustdoc `## Errors` sections and avoid an H1 inside doc comments.
- Avoid adding CLI behavior unless a later specification explicitly asks for it.
