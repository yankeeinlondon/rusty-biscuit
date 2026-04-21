# FileTree component

- The [`Filesystem`](@biscuit-terminal/lib/src/components/filesystem.rs) component in `biscuit-terminal` is a production ready component for displaying a directory tree of files. 
- In this feature we will add a new `Renderable` component to Darkmatter called `FileTree` which resembles `Filesystem` in that it 
    - is reporting a _graph_ of information
    - However, where `FileSystem` is pointed to a directory, `FileTree` is pointed to a file
    - `FileTree` will then be able to express external dependencies of the Markdown file including:
        - hyperlinks
        - image references
        - script references
        - css references
        - _transclusions_
    - `FileTree` will also offer a builder method called `.follow_transclusions()` which when used will change the rendering of transclusions into a graph of FileTree views
        - each transclusion will represent the HEAD of another `FileTree` with the file and 

## Example Illustration

The kinds of rendering this component would provide to the terminal would look something like:

```txt
    | ̅ ̅ ̅ ̅ 🔗 https://somewhere.com
    | ̅ ̅ ̅ ̅ 🔗 https://google.com
    |
    | ̅ ̅ ̅ ̅ 📄 @docs/a-linked-document.md
    |
    | ̅ ̅ ̅ ̅ 📄 @style/some-linked-css-stylesheet.md
    |
📄 foobar.md (2 inline script blocks, 3 meta props)
    |
    |<--- 📄 @docs/a-linked-document.md inserted into the '## Some Section' section
    |<--- 📄 @docs/b-linked-document.md inserted into the '## Another Section' section
    |
    |---> 📄 @docs/a-toc-referenced-doc.md inserted TOC links into the '## Another Section' section
    |
    |<--- 🧠 summarize https://site.com into the '## Yet Another Section' section
```

> Note: this is just a low fidelity version; without color, without nerd fonts (if available), without proper line shapes

The key ideas are:

- the references go above the file being analyzed
    - similar items do not need any vertical padding (aka, all URL based hyperlinks)
    - when transitioning from one _kind_ of link to another (e.g., URL based hyperlinks to Markdown hyperlinks), there is a blank row separating
- the transclusions go below the file being analyzed
- inline HTML elements detected in page referenced on same line as file (in parenthesis)
- in this diagram we did NOT use the `.follow_transclusions()` option

## Use with the Darkmatter CLI

We will add a new `graph` command to the Darkmatter CLI:

- syntax: `md graph <file-ref>`
    - this will run the basic -- non-following -- report
- if we want to validate that all of the references are valid references we would add the `--validate` flag
    - this will validate all immediately links but in a non-following mode it will not recursively check all the transclusions
    - it will however, check that the resource involved in the transclusion does exist
- if we want to add recursion (aka, follow the transclusions) then we'll need a `--follow` flag for the CLI to indicate this
    - The `--follow` and `--validate` flags can be used together and when they are then we will validate the entire recursive tree of links.


## Nerdfonts

Just like we do with the `FileSystem` component, we should leverage biscuit-terminal's ability to detect if the terminal is using a nerdfont and then leverage that when it's available.
