# Tree Hugger Kickoff

Read the @tree-hugger/README.md file for some broad context on what we're aiming to do with this package. The plan we are building will be to build out the first build of both the Library code (`./tree-hugger/lib`) and the CLI (`./tree-hugger/cli`).

**IMPORTANT:** Always use the `tree-sitter`, `rust`, and `rust-testing` skills when working with this package.

- The [`nvim-treesitter`](https://github.com/nvim-treesitter/nvim-treesitter) repo is a gold mine for abstracting useful queries across language grammars. Take the time to research it and understand what can be leveraged.
    - Read an analysis of this repo here: [nvim-treesitter](../../tree-hugger/docs/nvim-treesitter.md)
    - Then do any additional analysis and research to ensure you understand how to use this repo as a resource
- The [sniff](../../sniff/README.md) library also has some discovery features that will likely be useful for the `TreePackage` struct if nothing else

## Library

### Core Structs

- we are going to build two core structs:
    - `TreeFile` - the tree file will be responsible for looking at a single file and providing the key static analysis we need at the file level
    - `TreePackage` - the tree package will be responsible for understanding all the files in a "package" (aka, a repo or a package within a monorepo)

### `TreeFile` struct

One of the nice benefits of tree-sitter is that it allows a very focused file-based view of projects. Some AST libraries are more focused naturally on the "project" then the "file" but "tree-sitter" is laser fast at evaluating files whereas _projects_ are more of an afterthought.

- the @tree-hugger/lib/file/tree_file.rs file has a basic starter implementation to work from
- Implement all the `todo!()` macros currently in the source.

### `TreePackage` struct

- the @tree-hugger/lib/package/tree_package.rs file has a basic starter implementation to work from
- Implement all the `todo!()` macros currently in the source.

### Testing

Make sure to take the time to build up a good set of testing helper primitives and fixtures before implementing any of the code. Having a good set of testing tools and fixtures to start will help the project reach a successful outcome.

