# Reference Validation and Document Caching

## Reference Validation

The [`Markdown`](@darkmatter/lib/src/markdown/mod.rs) struct in Darkmatter provides a strong foundation for working with markdown files. However, we're just update the corresponding documentation for this struct in @darkmatter/docs/structs/Markdown.md to reflect a more detailed understanding of "Document References".

### Hyperlinks

Hyperlinks are an important part of every Markdown document and while most take the form of a **Markdown Link** they can also be inline HTML links with the `<a>` tag.

We currently have both a `.links()` and a `.inline_html_links()` implementation on the **Markdown** struct but they are limited in scope because they only report statically on the local document and _not_ the composed document. In order to address this we will do the following:

- add a `.has_transclusions(): bool` implementation on `Markdown` which tells us whether the current document has any transclusion elements. 
    - this not only provides a useful boolean flag on whether this node is an end node or if the composition graph continues with children of the current document, 
    - but it also serves as a useful initial check for the current `.links()` and `.inline_html_links()` implementations:
        - if the document is an end node, then the current implementations are enough
        - if the document _does_ have child transclusions then we must first resolve the 
- add a `.compose_without_report()` which is identical to `.compose_mut` but only returns the Markdown 

### Image References

The current implementations of `.image_references()` and `.inline_html_image_references()` mimic the limitations of the current hyperlink implementations: they do not consider the impact of transclusion operations. We will need to take the same actions here to enable this as we do with hyperlinks.

### Transclusions

We need to be able to query any Markdown struct what transclusions it includes.

- the key information we need back is 


### CSS, Scripts, and Fonts

This document update includes a need to be able to report on:

- inline CSS and CSS imports
- inline scripts and script imports
- Fonts imports

None of the above are common in a Markdown file but we should be able to detect and preserve this information. While the `Markdown.md` file documents some of the methods it expects it is NOT currently implemented.

### Meta Tags

In HTML `<meta>` tags are common but not in Markdown. In one regard, we want to be able to detect and preserve inline meta tags in Markdown but with meta tags we will go a bit further than we do with CSS, scripts, and fonts. This is described in the Markdown.md file in greater detail (this is described but not implemented).


