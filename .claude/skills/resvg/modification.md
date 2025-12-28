# Dynamic SVG Modification

Techniques for modifying SVG content before rendering, useful for theming, data injection, and dynamic content.

## Two Approaches

1. **String Manipulation**: Easier for simple changes
2. **Tree Manipulation**: More robust for complex logic

## String Manipulation

For simple color swaps or text replacements, treat SVG as string before parsing.

### Color Swapping

```rust
let mut svg_string = std::fs::read_to_string("icon.svg")?;

// Simple color swap
let customized_svg = svg_string.replace("#0000FF", "#FFD700");

// Multiple replacements
let customized_svg = customized_svg
    .replace("#primary", "#FF5733")
    .replace("#secondary", "#3498DB");

let tree = usvg::Tree::from_str(&customized_svg, &opt, &fontdb)?;
```

### Hiding Elements by ID

```rust
// Hide element by adding display="none"
let customized_svg = svg_string.replace(
    r#"id="secret-layer""#,
    r#"id="secret-layer" display="none""#
);
```

### Data Injection (Templates)

```rust
fn render_template(svg_template: &str, data: &HashMap<String, String>)
    -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut svg = svg_template.to_string();

    // Replace template variables
    for (key, value) in data {
        let placeholder = format!("{{{{{}}}}}", key); // {{KEY}}
        svg = svg.replace(&placeholder, &html_escape::encode_text(value));
    }

    // Parse and render
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let opts = usvg::Options {
        fontdb,
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(svg.as_bytes(), &opts)?;

    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)?;

    resvg::render(&tree, usvg::FitTo::Original,
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}

// Usage
let template = r#"<svg><text>{{TITLE}}</text></svg>"#;
let mut data = HashMap::new();
data.insert("TITLE".to_string(), "Hello World".to_string());
let png = render_template(template, &data)?;
```

### Dark Mode Toggle

```rust
fn apply_dark_mode(svg: &str, dark_mode: bool) -> String {
    if dark_mode {
        svg.replace("#FFFFFF", "#1E1E1E")
           .replace("#000000", "#FFFFFF")
           .replace("#F0F0F0", "#2D2D2D")
    } else {
        svg.to_string()
    }
}

// Usage
let svg = std::fs::read_to_string("icon.svg")?;
let themed_svg = apply_dark_mode(&svg, true);
```

## Tree Manipulation

For complex logic, traverse and modify the `usvg` tree.

### Changing Fill Colors

```rust
use usvg::{NodeExt, Fill, Paint, Color};

// Parse SVG
let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb)?;

// Traverse and modify
for node in tree.root.descendants() {
    if let usvg::NodeKind::Path(ref mut path) = *node.borrow_mut() {
        // Change red shapes to green
        if let Some(ref mut fill) = path.fill {
            if let Paint::Color(c) = fill.paint {
                if c == Color::new_rgb(255, 0, 0) { // Red
                    fill.paint = Paint::Color(Color::new_rgb(0, 255, 0)); // Green
                }
            }
        }
    }
}
```

### Hiding Elements by ID

```rust
for node in tree.root.descendants() {
    if node.id() == "background-layer" {
        if let usvg::NodeKind::Path(ref mut path) = *node.borrow_mut() {
            path.visibility = usvg::Visibility::Hidden;
        }
    }
}
```

### Modifying Stroke Width

```rust
for node in tree.root.descendants() {
    if let usvg::NodeKind::Path(ref mut path) = *node.borrow_mut() {
        if let Some(ref mut stroke) = path.stroke {
            // Double all stroke widths
            stroke.width *= 2.0;
        }
    }
}
```

### Finding and Replacing Text

```rust
for node in tree.root.descendants() {
    if let usvg::NodeKind::Text(ref mut text) = *node.borrow_mut() {
        for chunk in &mut text.chunks {
            for span in &mut chunk.spans {
                // Replace text content
                if span.text == "Old Text" {
                    span.text = "New Text".to_string();
                }
            }
        }
    }
}
```

## Comparison of Approaches

| Task | Recommended Method | Why? |
|------|-------------------|------|
| **Simple Color Swap** | String `.replace()` | Extremely fast, low overhead |
| **Conditional Hiding** | Tree Traversal | Precision; allows logic like "hide all circles" |
| **Dark Mode** | String Replace or CSS Injection | SVGs often use single theme color |
| **Data Injection** | String Templates (`format!()`) | Best for changing text labels/chart values |
| **Complex Logic** | Tree Manipulation | Type-safe, handles structure correctly |

## Advanced: Combining Both Approaches

```rust
fn render_customized_svg(
    svg_path: &str,
    color_map: HashMap<String, String>,
    hide_elements: Vec<String>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 1. String manipulation for colors
    let mut svg = std::fs::read_to_string(svg_path)?;
    for (old_color, new_color) in color_map {
        svg = svg.replace(&old_color, &new_color);
    }

    // 2. Parse tree
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let opts = usvg::Options {
        fontdb,
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(svg.as_bytes(), &opts)?;

    // 3. Tree manipulation for hiding elements
    for node in tree.root.descendants() {
        if hide_elements.contains(&node.id().to_string()) {
            if let usvg::NodeKind::Path(ref mut path) = *node.borrow_mut() {
                path.visibility = usvg::Visibility::Hidden;
            }
        }
    }

    // 4. Render
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)?;

    resvg::render(&tree, usvg::FitTo::Original,
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}
```

## CSS Class-Based Theming

```rust
fn apply_theme_classes(svg: &str, theme: &str) -> String {
    match theme {
        "dark" => {
            svg.replace(r#"class="primary""#, r#"class="primary" fill="#FFFFFF""#)
               .replace(r#"class="secondary""#, r#"class="secondary" fill="#CCCCCC""#)
        },
        "light" => {
            svg.replace(r#"class="primary""#, r#"class="primary" fill="#000000""#)
               .replace(r#"class="secondary""#, r#"class="secondary" fill="#333333""#)
        },
        _ => svg.to_string(),
    }
}
```

## Performance Considerations

### String Manipulation
- **Pros**: Fast, simple, no tree parsing overhead
- **Cons**: Fragile, can break with malformed replacements
- **Best for**: Known, simple replacements

### Tree Manipulation
- **Pros**: Type-safe, handles structure correctly
- **Cons**: Requires parsing overhead, more complex code
- **Best for**: Complex logic, conditional modifications

### Recommendation

Use **string manipulation** for:
- Color swapping
- Simple text replacement
- Template variable injection

Use **tree manipulation** for:
- Conditional hiding based on element properties
- Modifying stroke/fill based on element type
- Complex structural changes
