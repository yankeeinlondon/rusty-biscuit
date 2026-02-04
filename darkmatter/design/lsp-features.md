# LSP Features

The baseline functionality we need is a fully functioning Markdown LSP server. The focus for this module is to establish that baseline by reusing an existing LSP implementation and then _extend_ in the ways that would help with the [Darkmatter DSL](./darkmatter-dsl.md).

To understand how we plan to hook into an existing implementation of a Markdown LSP as a baseline please refer to [LSP Technical Strategy](../design/lsp-technical-strategy.md). The remainder of this document will focus on _additional_ features that we would like to layer on top of the baseline LSP.

## Features

1. File Reference Autocomplete.

    When a user types the `@` symbol either at the start of the line or after a whitespace character or break character like `(`, we want to bring up a list of files for the user to choose from.

    - the list to start will consist of popular file choices used in the given [project scope](../reference/project-scope.md) and which have [valid file extensions](../valid-file-extensions.md) for the given [document scope](../reference/doc-scope.md)
    - as soon as the user starts typing characters after the `@` symbol those files which fuzzy match

2. Interpolation Autocomplete.

    When a user is in an `unscoped` [document scope](../reference/doc-scope.md) and the type `{{` a list of frontmatter variables are provided as list. These variables include:
        - any **state** which was passed in during parsing,
        - any key/values defined on the page or from parent pages passed down to the given page.
        - the utility variables defined in [utility frontmatter](../reference/utility-frontmatter.md)

3. Interpolation Styling.

    When an interpolated frontmatter property is used in the body of the page with `{{variable}}` the inline block of characters should be highlighted with a badge like styling which converts the background color to make it stand out from the rest of the page.
