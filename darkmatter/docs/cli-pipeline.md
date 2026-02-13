# Markdown Pipelining in CLI

Currently the Darkmatter CLI is first and foremost a Markdown renderer for the Terminal. This has been it's primary focus to date but as we start to build out the full Markdown pipelining features that will become less and less it's primary function.

To understand the pipelining process review the [Darkmatter Pipeline](./darkmatter-pipeline.md) which give a good overview of all the various things which take place in a full Markdown pipeline transform. One of the key observations is that the last stage is "rendering" and currently the rendering which has gotten the most attention is rendering for the terminal.

## Changes To Make

The current number of switches is cluttered so we need to rationalize these.

### Output Format:

- Currently we have `--json`, `--html`, `--html-show`, and `--ast` which all relate to the output format
- All of these switches should be removed and replace with a single `--output <output>` format
- the valid "output" options are:
    - `auto` - this is the default and it will output to the terminal when it detects a TTY terminal, otherwise it will report as just Markdown text
    - `text` _or_ `markdown` - will export the content as Markdown plain text
        - Note: this includes exporting the Frontmatter if the file has frontmatter
    - `ast` _or_ `json` - will export the Markdown file as a md.AST JSON file
    - `html` - will export as HTML (with inline JS)

### Images:

- The options regarding images are largely dependant on the output format
    - the default output is based on whether we're in a TTY session and whether the user has overridden the default output with the `--output` switch discussed above ... but it should always be available to the CLI
- **Terminal Output:**
    - we currently render images to the terminal based on capability (e.g., if the terminal is detected as supporting images we will render it)
    - this is a good default and will remain AS IS
    - if you want to explicitly override this there is no command line switch but it will respond to the `TERMINAL_IMAGES` environment variable:
        - when not set the default is used
        - when set to `false` then images are never set
        - when set to `true` then images are always rendered (regardless of terminal support)
    - there is currently a `--no-images` CLI switch but this should be removed
- **Markdown Output:**
    - In Markdown documents we typically have images referenced using the Markdown style of:

      ```md
      ![alt text](./path/to/image.png)
      ```

    - But we cannot forget that Markdown documents _can_ and often are added via HTML because HTML provides more control
        - there is nothing wrong with having inline HTML to reference images in Markdown, that is fully allowed
        - the support in Markdown readers to render inline images is reasonably good but it's not 100% by any means
    - In an ideal world we would always want Markdown documents to use Markdown syntax to render images. It is more idiomatic, it ensures better reader support, is less cluttered for AI readers, and far easier to author and edit.
    - To solve this we must broadly categorize the "extra features" which an inline image provides over a Markdown image reference:

        - **Image Width** - by far and away the most common thing that authors want control over is specifying the width of an image. Either by some fixed unit or as a percentage of the viewable viewport.
        - **Popover Text** - far less common is the need to have a popover dialog appear to provide some additional information when a user hovers over an image.
        - **Everything Else** - there are undoubtedly various

  There are two nuances here worth calling out:

    - The 2nd Parameter
        - the official CommonMark spec actually allows for a _second_ parameter to specified within the parenthesis (after the image's filepath).
        - many, maybe even _most_, Markdown readers do not support this at all or not very well
        - in the specification, this second parameter is intended for setting the `title` attribute when rendering the Markdown to HTML. This has the effect of providing some "popover text" when a user hovers over the image (not immediately but eventually).
        - In modern terms though this is a poor UI experience compared to either JS or now modern browser support for a more full featured "popover" effect via HTML/CSS.

    - Image Width in the Alt Text

        - We have already implemented a way to specify the image's width by adding it into the "alt text":

          ```md
          ![alt text|15%](./path/to/image.png)
          ```

        - The design and testing of this has been overly focused on outputting to the Terminal but now we must take a broader view:
            - When we are rendering to "Markdown" we should simply remove the `|` character
