# Tree Hugger Kickoff

Read the @tree-hugger/README.md file for some broad context on what we're aiming to do with this package. The plan we are building will be to build out the first build of both the Library code (`./tree-hugger/lib`) and the CLI (`./tree-hugger/cli`).


**IMPORTANT:** Always use the `tree-sitter` skill when working with this package.

## Library

### Core Structs

- we are going to build two core structs:
    - `TreeFile` - the tree file will be responsible for looking at a single file and providing the key static analysis we need at the file level
    - `TreePackage` - the tree package will be responsible for understanding all the files in a "package" (aka, a repo or a package within a monorepo)

### `TreeFile` struct

One of the nice benefits of tree-sitter is that it allows a very focused file-based view of projects. Some AST libraries are more focused naturally on the "project" then the "file" but "tree-sitter" is laser fast at evaluating files and _projects_ are more of an afterthought.


