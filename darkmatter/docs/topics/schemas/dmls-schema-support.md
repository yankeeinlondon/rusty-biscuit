---
prompt: |-
    You are responsible for _documenting_ the DMLS support for SimplifiedSchema schemas.

    - use the darkmatter skill
    - read the document: @darkmatter/docs/topics/schemas/authoring-schemas.md for view on how SimplifiedSchema works
    
    Your documentation should match the style and level of detail of `{{dirname(ctx.self)}}/authoring-schemas.md`. Make sure that your document covers at least the following topics:
    
    - discuss what LSP features are implemented and which are not
    - discuss the extensibility that DMLS provides though configuration
    - discuss the extensibility that DMLS provides programatically

    If it can be made aesthetically tasteful it would also be nice to see a table that on one dimension shows the features an LSP can provide and on the other dimension shows the parts of the Darkmatter DSL which are supported. Regardless of how you present it, it's important to provide the reader with a view on how much "coverage" we currently provide to schemas in DMLS.

    If you want to visualize some aspects of your research you should feel free to use Mermaid code blocks.
---
