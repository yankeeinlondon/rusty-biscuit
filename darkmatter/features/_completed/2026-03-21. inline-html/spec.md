The `Markdown` struct in the darkmatter library already has `image_references()` and `links()` function which detect Markdown styles image references and links. We now need to a complimentary `inline_html_image_references()` and `inline_html_links()` functions.

- these functions have the same functional goals as the existing functions but instead of looking for Markdown styled variants they are looking for inline HTML variants inside the Markdown content.
- the existing functions leverage the `pulldown-cmark` crate to parse out the links and image references but this will likely not be the best strategy for the HTML based variants
- Darkmatter already is able to export MDAST ASTs via the `markdown` crate. Between the `markdown` crate and/or it's AST output this will hopefully serve as a basis for how we parse out the inline HTML variants.
    - Note: if there's a different way to do this you feel is better that's entirely fine but I wanted this option to be considered first
- I think we should also consider having a high performance way of checking if there is ANY inline HTML in the content and provide this to callers as `has_inline_html(): bool`. With this in place we can quickly check this first to determine if the parsing for links and image references is needed.


