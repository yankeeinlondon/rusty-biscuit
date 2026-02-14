# `ImageRef` Struct

The `ImageRef` struct -- defined in [`image_ref.rs`](../../lib/src/render/image_ref.rs) -- is meant to be a ergonomic and feature rich way of capturing, parsing, and transforming an image reference and outputting it multiple output targets. This functionality mirrors the functionality which the [`Link`](./Link.md) struct provides for hyperlinking and like it it supports the following input and output formats:

1. **HTML** `<img src="..." alt="..." />`
2. **Markdown** `![alt-text](image-url "optional-title")`
3. **Terminal** - OSC8 link (with fallback) and allowing escape codes for formatting

## Core State

To fully represent an image reference we must look at our most feature rich output client -- the HTML `img` tag -- to understand the various attributes used to represent it's "state":

| Attribute  | Description                          | Type | Default Value |
| ---------  | ---------------------                | -------------- | ------------- |
| `style`    | direct CSS style definitions          | CSS key/values | _undefined_      |
| `class`    | indirect CSS styling                 | string         | _undefined_      |
| `title`    | a text string which most browsers will render as a poor mans popover to the image | string | _undefined_ |
| `decoding` | provides hints to browsers on whether decoding in addition to rendering is necessary | sync, async, auto | auto |
| `fetchpriority` | provides a hint of the relative priority to use in fetching the image | high, low, auto | auto |
| `height`   | the intrinsic height of the image in pixels | Integer | _undefined_    |
| `width`    | the intrinsic width of the image in pixels  | Integer | _undefined_    |
| `loading`  | indicates how the browser should load the image | eager, lazy | eager |
| `referrerpolicy` | indicating which referrer to use when fetching the resource | no-referrer, no-referrer-when-downgrade, origin, origin-when-cross-origin, same-origin, strict-origin, strict-origin-when-cross-origin, unsafe-url | _undefined_ |
| [`sizes`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/img#sizes)      | allows different source images to be mapped to media queries | one or more source image sizes or the `auto` keyword | _undefined_ |
| [`src`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/img#src) | The image URL (_relative or absolute_) | string | undefined |
| [`srcset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/img#srcset) | One or more strings separated by commas, indicating possible image sources for the user agent to use. | string | undefined |

> **Note:** for an image reference to be valid it must have either `src` or `srcset` defined

- In addition a `img` tag can have any number of `data-xxx` tags.

The `ImageRef` struct must have all of these properties and populate them with valid values without requiring a user to manual specify everything (aka, good ergonomics).

### What about Terminal and Markdown links?

We modelled the "state" of an image reference on what HTML can do, what about Terminal output? What about Markdown links?

- Escape Codes in Alt Text
    - The only conflict resulting from a potential is Terminal output is that the `alt` property may use escape codes which would only work well for the terminal; for both Markdown and HTML the alternative text is always just plain text.
    - However, we don't need to worry about this when inputting the state as it's only during the output stage that we'll need to manage this in a smart way
- Markdown's Idiomatic Image Reference Syntax
    - A Markdown document can reference images in one of two ways:
        - The idiomatic Markdown syntax of: `![alt text](url "optional-title")`
        - An inline HTML syntax
    - There is nothing technically wrong with the _inline HTML_ image reference other than it's awkward to create and some Markdown readers might not render the image.
    - The upside is that it solves the problem of how to represent an image with rich metadata
    - Based on these trade-offs, the `ImageRef` struct will the following logic when rendering an image reference to a Markdown output:
        - if an image reference consists of only "alt text" and "image source" or even if it also has a "title" attributed defined, we will use the idiomatic Markdown syntax 100% of the time
        - if an image reference consists of more than these core attributes we have a decision to make:
            - if the environment variable IMAGE_REF_METADATA is set to `inline` (casing of value doesn't matter) then we will instead use the inline image references
            - if the environment variable IMAGE_REF_METADATA is set to `strip` then we'll use idiomatic Markdown
            - the default decision is to STILL use the idiomatic Markdown syntax but we must do this in a lossless manner:
                - A lossless representation of an image reference is achieved by using the "title" property of a Markdown image reference in a way which was not intended
                - The good news is that the "title" property of a Markdown image reference (as Markdown links) is rarely used today. It was added to the CommonMark spec so that readers/renders could add a title attribute which a browser will use to produce a minimal popover effect.
                - Instead of that goal we provide a base64-encoded JSON serialization of metadata properties
                - Only defined metadata fields are serialized; undefined fields are omitted
                - We are still following the CommonMark spec in that we're adding a string value to the 'title' property
                - Normal Markdown parsers should work fine
                - Markdown renderers will either display nothing (because they don't support the title feature) or they will provide a meaningless text string when hovered over in the browser


## Creating and working with an `ImageRef` struct

### Creating

Initial creation of an `ImageRef` can be done via any of the following methods:

- The public **new** function:
    - `new<T: Into<String>, U: Into<String>>(url: T, alt_text: U) -> Result<ImageRef,ImageRefError>`
- Leveraging the `ImageRef`'s implementation of the **From** trait for the following types:
    - (&str, &str) - takes a tuple representing the URL and Alt Text
    - (&String, &String) - takes a tuple representing the URL and Alt Text
    - (&str, &Prose) - takes a tuple representing the URL and Alt Text
    - (&String, &Prose) - takes a tuple representing the URL and Alt Text
- Leveraging the `ImageRef`'s implementation of the **TryFrom** trait for the following types:
    - String, &String, &str - tries to parse content as either a HTML or Markdown link

> the `Prose` struct is found in the `biscuit-terminal` package in this monorepo, is already being used in Darkmatter, and is a simple way for someone to generate Alt Text for the terminal

Of the methods for getting started with a `ImageRef`, the `TryFrom` implementations are the most involved because unlike the other methods which just need to check that the URL and Alt Text are valid these methods need to _import_ a full image reference in one of two formats:

- importing from an HTML `img` tag:
    - The importing and mapping of an HTML tag into the struct is fairly straight forward because we've in part modelled the struct on the HTML representation
    - Mainly it's just making sure that properties provided are validly typed
    - If a property is of an invalid type (typically an enumerated type) then we just drop that key/value pair but continue on with the import
- importing from a Markdown image reference:
    - An idiomatic Markdown image reference in most cases will just map the Alt Text and Url into struct after validating them
    - For Markdown links with the `title` defined we must discern whether the title property is an actual title or instead a base64 serialized metadata package
    - In Darkmatter we offer a convenience to Markdown authors of specifying the image's width as an _addon_ to the Alt Text using the `|` delimiter only:
        - The following image reference: `![hi|15%](./my-image.png)` would have:
            - an Alt Text of: `hi`
            - the image would be expected to be rendered at a width of 15% of the viewport, during the import process this would be achieved by setting the `style`

### Working with `ImageRef`

- we will use a series of builder methods which will help a user build up the ImageRef to the state they want
- we will provide a `to_terminal()`, `to_markdown(with_inline: bool)`, and `to_html()` method to output the image reference to various targets
    - **HTML:**
        - this is quite straightforward and has a largely one-to-one mapping between the structs properties and the attributes on a `<img>` tag
        - NOTE: we will strip all ANSI escape codes from the title if it's set
    - **Markdown:**
        - renders a simple idiomatic Markdown image reference when the only metadata present is alt-text, image-source, and title
        - if the passed in `with_inline` is set to `true` then all images with additional metadata will be rendered as an inline HTML image reference.
        - if the passed in `with_inline` is set to `false` then we will:
            - serialize all metadata (outside of the alt-text and source)

### Styles that Matter

The goal for the `ImageRef` struct is not to be _lossy_ whenever possible
