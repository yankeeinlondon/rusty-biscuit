# HTML Output

Darkmatter converts Markdown to HTML with inline CSS and optional JavaScript for advanced features.

## Inline Syntax Preservation

### Dim Text (`⌄text⌄`)

The `⌄text⌄` dim syntax is a terminal-first inline formatting extension. In HTML output, the `⌄` delimiters are preserved as literal characters rather than being converted to a semantic HTML tag.

**Example:**

```markdown
This is ⌄dimmed⌄ text.
```

**HTML output:**

```html
<p>This is ⌄dimmed⌄ text.</p>
```

This preserves the original Markdown source intent while acknowledging that HTML has no standard `<dim>` element. Consumers who want dim styling in HTML should apply CSS to a custom wrapper or use an alternative approach.
