---
title: Formatting rules control
# Every `style.markdown` rule set to a non-default value, so each one is
# observable when a test removes or mutates it.
style:
    markdown:
        frontmatter: false
        table-layout: false
        fixed-width: 80
        unwrap: true
        indentation: 2
        ordered-lists: false
        list-padding: false
        list-item-padding: one
        list-interior-padding: all
        default-language: sh
---

Intro line one
intro line two.
- first
- second

```
echo unlabeled
```
