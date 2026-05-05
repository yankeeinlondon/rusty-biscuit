We need to add a new CLI command `sniff repo language` which will return the "primary programming language" found in the repo.

We also need to fix a regression where all sniff commands which involve the filesystem are supposed to expose the `--base <dir>` CLI switch. It is a part of the global help system for sniff but this switch does not appear to be working with any of hte `sniff repo` subcommands!
