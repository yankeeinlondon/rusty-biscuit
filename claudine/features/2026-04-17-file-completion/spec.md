# File Completions

We already have SOME file completions for Claudine but to make it easy to use we are going to need to do more. This feature adds an important completion feature. It adds "file completion" for claudine's `compose`, `inline-compose`, and `sequence` commands.

- each of these commands receives a "file reference" to the Markdown file they will be using
- these file references all leverage the file referencing provided by `biscuit-file`'s **FileReference** struct
- we need to make sure that our shell completions will auto-complete the valid markdown files so the user doesn't have to type them out manually
