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

## Text or File?

Each string is read in one of two ways, and every Darkmatter reader (`md compose`, `md validate refs`, `md graph`, the library's reference APIs, and the language server) decides it the same way.

A value is **inline Markdown text**, inserted as written, when it is empty, spans more than one line, contains a Markdown link (`](`), or is a plain word or phrase with no path separator and no file extension. Anything else is a **file reference**: a value with a `/` or `\`, a sigil such as `~/` or `@`, a URL, or a single `name.ext` word.

| Value                                    | Read as                                             |
| ---------------------------------------- | --------------------------------------------------- |
| `prologue: ./intro.md`                   | file `intro.md`                                     |
| `prologue: intro.md`                     | file `intro.md`                                     |
| `prologue: Read this first`              | text                                                |
| `prologue: README`                       | text (no extension), even if a `README` file exists |
| `prologue: "[Setup](./setup.md)"`        | text holding a link to `./setup.md`                 |
| `prologue: \|` followed by indented lines | text                                                |
| `prologue: ""`                           | text that inserts nothing                           |
| `prologue: "./intro.md#start"`           | file `intro.md#start` (a `#` is part of the name)   |
| `prologue: ./notes.txt`                  | file `notes.txt`, refused: not a Markdown file      |

Inline text is never checked as a file. The links inside it are checked as ordinary links, so `md validate refs` passes `prologue: "[Setup](./setup.md)"` when `setup.md` exists and reports a missing hyperlink when it does not. A file reference is checked as the whole filename composition opens, so `prologue: "./intro.md#start"` is reported missing even when `intro.md` has a `# Start` heading. A file reference must name a Markdown file (`.md` or `.markdown`), as it must for `::file`: composition and `md validate refs` both refuse `prologue: ./notes.txt` with an unsupported-file-type error at the `prologue:` key, even when `notes.txt` exists.

> **Propagation Exception:** Unlike other frontmatter properties (which propagate from parent to child during transclusion for use in replacement and interpolation), `prologue` and `epilogue` are scoped to the document that defines them. They are not inherited by child documents brought in via `::file` directives or frontmatter transclusion.
>
> **Note:** the "options" which we provide in [block transclusion](./block-transclusion.md#) -- including _conditional_ transclusion -- are not available for Frontmatter transclusion.

