# FileTree component

- The [`Filesystem`](@biscuit-terminal/lib/src/components/filesystem.rs) component in `biscuit-terminal` is a production ready component for displaying a directory tree of files. 
- In this feature we will add a new `Renderable` component to Darkmatter called `FileTree` which resembles `Filesystem` in that it 
    - is reporting a graph of information
    - However, where `FileSystem` is pointed to a directory, `FileTree` is pointed to a file
    - `FileTree` will then be able to express external dependencies of the Markdown file including:
        - hyperlinks
        - image references
        - script references
        - css references
        - _transclusions_
    - `FileTree` will also offer a builder method called `.follow_transclusions()` which when used will change the rendering of transclusions into a graph of FileTree views
        - each transclusion will represent the HEAD of another `FileTree`

## Use with the Darkmatter CLI

We will add a new `graph` command to the Darkmatter CLI:

- syntax: `md graph <file-ref>`
- 
