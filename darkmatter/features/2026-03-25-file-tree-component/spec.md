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
    |
    |
    |
```

> Note: this is just a low fidelity version; without color, without nerd fonts (if available), without proper line shapes

The key ideas are:

- the references go above the file being analyzed
- the transclusions go below the file being analyzed
- inline HTML elements detected in page referenced on same line as file (in parenthesis)
- in this diagram we did NOT use the `.follow_transclusions()` option

## Use with the Darkmatter CLI

We will add a new `graph` command to the Darkmatter CLI:

- syntax: `md graph <file-ref>`
- 
