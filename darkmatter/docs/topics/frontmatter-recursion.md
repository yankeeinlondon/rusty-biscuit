# Frontmatter and Recursion

During transclusion:

- the frontmatter of the parent document is passed to the child documents
- if the child document has the properties defined then
    - if the property is a scalar value or a list value then the child document's value is retained _over_ the parents
    - if the property is a dictionary value then we merge the object's keys, giving the child's values priority over the parent's in the case where the parent

## Inherited Values Are Data

The child receives the parent's **composed** values, and it treats them as data: it never evaluates a `{{ … }}`, converts a `{{{ … }}}`, or runs a `$( … )` it finds in them. The parent already evaluated what its author wrote, so evaluating it again in every child would let text produced by an expression, a file read, or a command become a new instruction.

```md
<!-- parent.md -->
---
area: claudine
note: "fixed {{{ area }}}"
---
::file child.md

<!-- child.md -->
Child: {{ note }}
```

The child composes to `Child: fixed {{ area }}`, exactly as the parent's own body would.

Values a transclusion directive sets (`::file child.md set.x="…"`) are authored when the parent wrote them literally in the directive, and data when any part of the directive's options came from interpolation (the parent evaluates `set.x="{{ v }}"` before the child sees it). The child's own frontmatter is authored and scanned as usual. See [Inserted Text Is Data](../inline/interpolation.md#inserted-text-is-data).

## Exceptions

- `ctx` is a property which is provided to each document by Darkmatter; it is recommended that document authors **not** use this property themselves but it is supported. When a document _does_ have a `ctx` property then we will use the normal recursion process described above.
