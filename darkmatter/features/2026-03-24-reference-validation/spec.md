# Reference Validation

> **Note:** we have now implemented caching for the compose pipeline; details are available in [caching design](@darkmatter/docs/topics/caching.md).

## Reference Validation

The [`Markdown`](@darkmatter/lib/src/markdown/mod.rs) struct in Darkmatter provides a strong foundation for working with markdown files. From this struct we can:

1. run the [composition pipeline](@darkmatter/docs/darkmatter-compose-pipeline.md)
2. run implementations `links()`, `inline_html_links()`, `image_references()`, and `inline_html_image_references()`

However, both the link and image reference implementations do not report on the _composed_ markdown document which is really the question these implementations must report on.

Beyond that change, we also need to move beyond _just_ links and image references to a larger set of references as well as provide validation features on these references.

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

- get_transclusions() - reports on the transclusions in the local document
- get_transclusions_graph() - reports on the full graph of transclusion originating from the given document


### CSS, Scripts, and Fonts

This document update includes a need to be able to report on:

- inline CSS and CSS imports
- inline scripts and script imports
- Fonts imports

- get_inline_css() - inline css blocks in local file
- get_inline_css_graph() - inline css blocks in local file and transcluded files
- get_css_imports()
- get_css_import_graph() 

- get_inline_script() - inline scripts blocks in local file
- get_inline_script_graph() - inline scripts blocks in local file and transcluded files
- get_script_imports()
- get_script_import_graph() 


### Meta Tags

- get_meta_tags() - get all meta tags in the local document as a dictionary
- merge_meta_into_frontmatter(overwrite: bool) - merge the meta tags key/values into frontmatter properties
- set_meta_tag(key: string, value: MetaValue)
