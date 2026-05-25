# Feature Specification: YAML Code Block and Frontmatter Generation

## Summary

This feature adds two convenience methods to the `Yaml` struct in `biscuit-file` to facilitate the generation of Markdown-compatible YAML representations. These methods handle proper formatting, validation for specific use cases (like frontmatter), and collision avoidance for nested Markdown syntax.

## Method Signatures

```rust
impl Yaml {
    /// Returns the YAML content as a Markdown code block with appropriate language tagging.
    ///
    /// Handles collision avoidance by dynamically adjusting the outer code fence
    /// if the content itself contains triple backticks.
    pub fn as_yaml_code_block(&self) -> Result<String, YamlError>;

    /// Returns the YAML content formatted as a Markdown frontmatter block.
    ///
    /// Strictly enforces that the YAML content is a mapping (key-value pairs).
    pub fn as_frontmatter_block(&self) -> Result<String, YamlError>;
}
```

## Requirements

### 1. Frontmatter Validation (`as_frontmatter_block`)
- **Strict Mapping Requirement**: Frontmatter must be a valid set of key-value pairs to be structurally sound for most Markdown parsers.
- **Validation**: The method MUST check if the underlying YAML value is a mapping (object/map).
- **Error Handling**: If the YAML content is a scalar (e.g., a single string, number, or boolean) or a sequence (array), the method MUST return a `YamlError`.
- **Formatting**: The output must be enclosed between triple-dash delimiters (`---`) on their own lines.

### 2. Dynamic Code Fences (`as_yaml_code_block`)
- **Language Tagging**: The block must start with ` ```yaml ` and end with ` ``` ` by default.
- **Collision Avoidance**: If the serialized YAML content contains triple backticks (```), the method MUST increase the length of the outer fence to ensure the block remains valid Markdown.
  - The number of backticks in the outer fence must be at least one greater than the longest sequence of backticks found within the serialized content.
  - Example: If the content contains ` ``` `, the outer fence should use four backticks ` ```` `.
- **Standard Formatting**: The serialized YAML should be indented and formatted according to standard `biscuit-file` conventions (using `serde_yaml_ng`).

## Examples

### Success: YAML Code Block
**Input YAML:**
```yaml
key: value
nested:
  foo: bar
```

**Output of `as_yaml_code_block()`:**
````md
```yaml
key: value
nested:
  foo: bar
```
````

### Success: YAML Code Block with Collision
**Input YAML (containing backticks in a string):**
```yaml
snippet: "Check out this code: ```rust\nfn main() {}\n```"
```

**Output of `as_yaml_code_block()` (using 4 backticks for outer fence):**
`````md
````yaml
snippet: "Check out this code: ```rust\nfn main() {}\n```"
````
`````

### Success: Frontmatter Block
**Input YAML:**
```yaml
title: My Page
author: Jane Doe
```

**Output of `as_frontmatter_block()`:**
```md
---
title: My Page
author: Jane Doe
---
```

### Failure: Frontmatter Validation
**Input YAML (Scalar):**
```yaml
"just a string"
```

**Result of `as_frontmatter_block()`:**
`Err(YamlError)` (Validation failure: expected mapping, found scalar).

**Input YAML (Sequence):**
```yaml
- item 1
- item 2
```

**Result of `as_frontmatter_block()`:**
`Err(YamlError)` (Validation failure: expected mapping, found sequence).
