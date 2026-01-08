# Content Interpolation

The `interpolate` module provides flexible content interpolation for strings, regex patterns, markdown, and HTML.

## Module Structure

```rust
use shared::interpolate::{
    // String interpolation
    interpolate,

    // Regex-based interpolation
    interpolate_regex,

    // Type-safe interpolation context
    InterpolateContext,

    // Markdown-specific interpolation
    md_interpolate,

    // HTML-specific interpolation
    html_interpolate,
};
```

## Basic String Interpolation

### Simple Placeholders

```rust
use shared::interpolate::{interpolate, InterpolateContext};
use std::collections::HashMap;

// Create context
let mut context = HashMap::new();
context.insert("name", "Alice");
context.insert("language", "Rust");

// Interpolate
let template = "Hello, {{name}}! Welcome to {{language}}.";
let result = interpolate(template, &context);
assert_eq!(result, "Hello, Alice! Welcome to Rust.");
```

### Missing Values

```rust
// Missing placeholders are left unchanged
let template = "Hello, {{name}}! Your ID is {{id}}.";
let result = interpolate(template, &context);
assert_eq!(result, "Hello, Alice! Your ID is {{id}}.");
```

## Regex-Based Interpolation

For custom placeholder patterns:

```rust
use shared::interpolate::interpolate_regex;
use regex::Regex;

// Custom pattern: ${variable}
let pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
let template = "Hello, ${name}! Welcome to ${language}.";

let result = interpolate_regex(template, &context, &pattern);
assert_eq!(result, "Hello, Alice! Welcome to Rust.");
```

## Type-Safe Context

```rust
use shared::interpolate::InterpolateContext;
use serde_json::json;

// From HashMap
let mut map = HashMap::new();
map.insert("user".to_string(), "Bob".to_string());
let context: InterpolateContext = map.into();

// From JSON
let json_context = json!({
    "user": "Bob",
    "count": 42,
    "active": true
});
let context = InterpolateContext::from_json(json_context);

// Access values
assert_eq!(context.get("user"), Some("Bob"));
assert_eq!(context.get("count"), Some("42"));
assert_eq!(context.get("active"), Some("true"));
```

## Markdown Interpolation

Preserves markdown structure while interpolating:

```rust
use shared::interpolate::md_interpolate;

let markdown = r#"# Welcome, {{name}}!

You have {{count}} new messages.

## Your Stats
- Language: {{language}}
- Level: {{level}}
"#;

let mut context = HashMap::new();
context.insert("name", "Alice");
context.insert("count", "5");
context.insert("language", "Rust");
context.insert("level", "Expert");

let result = md_interpolate(markdown, &context);
// Preserves markdown formatting while replacing placeholders
```

### Features

- Preserves heading structure
- Maintains code blocks unchanged
- Handles inline code correctly
- Preserves link formatting

## HTML Interpolation

Safe HTML interpolation with escaping:

```rust
use shared::interpolate::html_interpolate;

let html = r#"
<h1>Welcome, {{name}}!</h1>
<p>You have <strong>{{count}}</strong> notifications.</p>
<div class="user-{{role}}">
    {{content}}
</div>
"#;

let mut context = HashMap::new();
context.insert("name", "Alice");
context.insert("count", "3");
context.insert("role", "admin");
context.insert("content", "<script>alert('XSS')</script>");

let result = html_interpolate(html, &context);
// Automatically escapes HTML in interpolated values
// <script> becomes &lt;script&gt;
```

### Safety Features

- Automatic HTML escaping
- Preserves existing HTML structure
- Safe for user-generated content
- Prevents XSS attacks

## Advanced Patterns

### Nested Objects

```rust
use serde_json::json;

let context = InterpolateContext::from_json(json!({
    "user": {
        "name": "Alice",
        "email": "alice@example.com"
    },
    "stats": {
        "posts": 42,
        "karma": 1337
    }
}));

// Access nested values with dot notation
let template = "{{user.name}} ({{user.email}}) - Posts: {{stats.posts}}";
// Note: Implementation may require custom handling for nested access
```

### Custom Transformers

```rust
// Apply transformations during interpolation
let mut context = HashMap::new();
context.insert("price", "99.99");
context.insert("date", "2026-01-08");

// Can combine with custom processing
let template = "Price: ${{price}} | Date: {{date}}";
let result = interpolate(template, &context);
```

## Error Handling

The interpolation functions are designed to be forgiving:

```rust
// Missing values are preserved
let result = interpolate("Hello, {{name}}!", &HashMap::new());
assert_eq!(result, "Hello, {{name}}!");

// Invalid placeholders are ignored
let result = interpolate("Hello, {{}}!", &context);
assert_eq!(result, "Hello, {{}}!");

// Unclosed placeholders are preserved
let result = interpolate("Hello, {{name!", &context);
assert_eq!(result, "Hello, {{name!");
```

## Performance Considerations

1. **Pre-compile regex**: For repeated use, compile regex once
2. **Reuse contexts**: Convert to `InterpolateContext` once
3. **Batch processing**: Process multiple templates with same context
4. **Memory efficiency**: Contexts use `Cow<'static, str>` for zero-copy

## Use Cases

### Configuration Templates

```rust
let config_template = r#"
database:
  host: {{db_host}}
  port: {{db_port}}
  name: {{db_name}}

api:
  endpoint: https://{{api_domain}}/v1
  key: {{api_key}}
"#;

let result = interpolate(config_template, &env_context);
```

### Email Templates

```rust
let email_template = r#"
Dear {{recipient_name}},

Thank you for your order #{{order_id}}.

Items:
{{order_items}}

Total: ${{order_total}}

Best regards,
{{company_name}}
"#;
```

### Documentation Generation

```rust
let doc_template = r#"
# {{project_name}} Documentation

Version: {{version}}
Updated: {{date}}

## Installation

```bash
{{install_command}}
```

## Configuration

{{config_section}}
"#;
```

## Testing

```rust
#[test]
fn test_interpolation_escaping() {
    let mut context = HashMap::new();
    context.insert("xss", "<script>alert('xss')</script>");

    // HTML interpolation escapes
    let html_result = html_interpolate("<div>{{xss}}</div>", &context);
    assert!(html_result.contains("&lt;script&gt;"));

    // Regular interpolation doesn't escape
    let text_result = interpolate("Message: {{xss}}", &context);
    assert!(text_result.contains("<script>"));
}
```