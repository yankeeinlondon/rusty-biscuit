---
replace:
    frontmatter: "[Frontmatter](https://dev.to/dailydevtips1/what-exactly-is-frontmatter-123g)"
last_updated: 2026-02-14
---

## Preparation Tasks

While the main workhorse in the Markdown pipeline is _transclusion_ we offer a number of preparatory steps that can help shape the document before we get to transclusion:

- **Text Replacement**

    We describe the text replacement process in the design document [Text Replacement](@darkmatter/docs/text-replacement.md), however, the short version is that we treat the `replace` property in frontmatter as a special property and if it's defined as a dictionary then we use it to replace all the _keys_ with the associated _value_ throughout the document (like a document find-and-replace operation).

- **Frontmatter Interpolation**

    Any property defined either _in_ the documents frontmatter or _passed into_ the document's frontmatter during the the pipelining process can be used to replace "handlebars" based template syntax on the page.

    In addition we provide built-in context which is made available in frontmatter via the `ctx` and `env` properties. For more details refer to the design document: [Frontmatter Interpolation](@darkmatter/docs/interpolation.md).

- **Cleaning**

    A lot of Markdown is delivered in a somewhat dirty or non-standards based format. By _cleaning_ the document we attempt to move it closer to being fully compliant while being careful not to change any semantic meaning.

- **Normalization**

    The structure of a document is determined by `H1` -> `H6` level headings and there are some basic rules (and in some cases just _strong suggestions_) that Markdown provides:

    1. Only one `H1` per document; this represents the "title" of the document
    2. Never skip heading levels, going from `H2` to `H3` is normal but you should not go from `H2` directly to `H4`
    3. Headings always have one (_not more, not less_) blank lines _before_ and _after_ them
    4. Ordered and Unordered lists always have one (_not more, not less_) blank lines _before_ and _after_ them
    5. etc.


