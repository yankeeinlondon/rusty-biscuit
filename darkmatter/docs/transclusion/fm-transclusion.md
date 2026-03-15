# Frontmatter Transclusion

The most common and capable way of doing transclusion is via [block transclusion](./block-transclusion.md) within the body of the Markdown document. However, there are some use cases where you want to provide transclusion instructions in the frontmatter of a document and that is possible too.

With frontmatter transclusion we imbue two frontmatter properties special meaning:

- `prologue`
    - When this property is assigned either a **string** or **string array** value will be parsed as an instruction to transclude to the beginning of the document (before any content in the current body of the document)
    - string values are evaluated and determined to be one of the following:
        - local file reference
        - URL/remote content reference
        - String content
        - invalid reference
    - if an invalid reference is found then the pipelining process will stop and return a meaningfully descriptive error message.
        - We must try to distinguish an invalid attempt to reference a file and just string based content
    - if the caller wants invalid references to just be _ignored_ but not block the pipelining process then the caller must either:
        - set the environment variable IGNORE_INVALID to `true`, or
        - start the pipelining call with the frontmatter's `ignore_invalid` set to `true`
- `epilogue`
    - The **epilogue** property behaves exactly the same as **prologue** except that the document or documents referenced are added _after_ the body's content not before.

> **Propagation Exception:** Unlike other frontmatter properties (which propagate from parent to child during transclusion for use in replacement and interpolation), `prologue` and `epilogue` are scoped to the document that defines them. They are not inherited by child documents brought in via `::file` directives or frontmatter transclusion.
>
> **Note:** the "options" which we provide in [block transclusion](./block-transclusion.md#) -- including _conditional_ transclusion -- are not available for Frontmatter transclusion.

