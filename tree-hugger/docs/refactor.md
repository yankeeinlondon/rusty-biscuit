The tree-hugger Library and CLI are working to some degree but they still pretty broken and this feature request is a major refactor of both.

## CLI

In many/most we have a `--all` flag which will select all the source files from the current repo/package. This switch should be removed and iterating over all source files should be the default!

> syntax: `hug <subcommand> <filter[]>`

- Subcommands honoring this syntax include:
    - functions, classes, types, symbols
    - the `exports` subcommand is redundant and should be removed
    - we will for now ignore the `lint` and `imports` subcommands as these are different
- there can be 0:M parameters which act as filters
- filtration can take place at:
    - the file level
    - and the symbol name level
- if a parameter is file reference (e.g., ends with a file extension) then we eliminate all files except those which are explicitly stated in the parameters (by default we search ALL source files).
- if a parameter is not a file reference it is assumed to be a glob pattern for symbol names
    - all symbol based filters will be combined in a logical AND operation
    - example:
        - `hug functions foo bar`
        - this expression will scan all source files
        - will only return symbols who's names contain "foo" OR "bar"
    - by default a symbol glob pattern has the `*` wildcards added to the front and back of what the user expresses
        - this allows for easy string subset matching which is most commonly what people want
        - if a user wants to only have the wildcard at the start or end of their glob pattern they must explicitly state it:
            - 'foo*' will only match on symbols which START with foo
            - '*bar' will only match on symbols which END with bar
        - as soon as the user has added one or more `*` wildcard symbols then we will no longer add any wildcards automatically:
            - 'foo*bar' will only match on symbols which start with **foo** and end with **bar**

### Filtering Switches

While parameters are a common form of filtering, we also provide some CLI switches which act as filtering devices too.

- `--exported` filters symbols to only those which are exported or public
- `--prelude` 
    - For Rust projects this maps directly to the symbols which a user has deemed to be of high utility and has therefore re-exported them under the prelude domain.
        - When this flag is used it doesn't mean we want the "re-exports", it means we want those symbols which were "blessed" as part of the prelude to be included in the result set but not others
    - For non-Rust projects the concept of a "prelude" is likely not 

### Output Switches

By default we output for the terminal, which means:

- leveraging the renderable components from `biscuit-terminal` to present visually impressive reports on the symbols which are returned (Table, Prose, TwoColumn, UnorderedList, etc.)
- the information in this mode may be slightly lossy as we do not want to overwhelm the user
- however, we don't want to be too minimal. A well designed report can be information dense and still understandable to the user in the terminal.

The only other output format we provide is JSON:

- using the `--json` switch indicates output should be valid JSON
- the JSON output format should never be "lossy"

### Sorting and Grouping

The underlying static analysis is done with tree-sitter and it's approach is very "file" focused. From that technical analysis standpoint it might seem logical that we'd be interested in seeing the _results_ of this analysis also from a file based perspective but that is not always the case.

- by default our reporting will **not** be sorted or grouped by files, instead the default behavior is to report on the symbols in alphabetical order.
- using the `--group-by-file` switch will group each file's symbols separately:
    - the filenames will be sorted alphabetically
    - the symbols inside each group will follow the normal alphabetical sorting order unless a command line switch changes this (e.g. `--sort-by-kind`, etc.)
- use the `--sort-by-kind` can be used regardless of whether a grouping effect was chosen
    - the symbols will first be sorted by their "kind" and then alphabetically within items of the same kind
- use the `--group-by-module` switch will group by modules:
    - if this is used with `--group-by-file` then we will group FIRST by module and then by file
- use the `--sort-by-module` will sort by modules first and then whatever sort order was defined by the other switches


## tree-hugger Library

One of the problems we keep on bumping into time and again is that we're not aiming high enough in our Symbol Schema (e.g., the metadata we capture when parsing the source code) and then find that the thing we want to present in our reporting is not available.

Our goal should be a high-fidelity model for every symbol in a project.

### Remodeling the Symbol Schema

The starting point for this refactor really needs to reside in a remodelling of the symbol schema.

- there is a strong design built for this purpose: [Symbol Schema Design](./symbol-schema-design.md)
- once we finalize on this design we'll need to make sure that we update our static analysis so that this much more complete schema will be able to be populated

### Consider Caching

- review and incorporate the thinking of this new caching design document: [cache design](./cache-design.md).

## Task

Create a high fidelity plan to refactor the CLI and Library based on these inputs.
