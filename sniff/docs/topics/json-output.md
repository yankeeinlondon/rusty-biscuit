# JSON output

At any point of the `sniff` CLI's command structure you can add `--json` and get back a JSON payload instead of text intended (and often formatted) for the terminal.
To ensure that the JSON you get back we follow a strict set of rules to ensure that people "get what they expect":

- it might be _falsely_ assumed that `--json` provides the information that is provided to the terminal and nothing more
- this is incorrect because we are optimizing the targetted audiences
    - structured data is typically targetted for programatic usage 
    - whereas the default output is targetting humans
- the default screen output is therefore a _subset_ of the full data at that level of the command structure
    - The global `--verbose`, `-v` flags in many cases will provide additional information intended for human consumption in the terminal
    - The verbose output might extend to include all data attributes but it depends
- in contrast, JSON always includes all metadata for that level of the command structure
    - subcommands will never use data outside the parent command structure's scope
    - the inverse is also true: a _leaf_ subcommand's JSON returns ONLY its own data — never fields borrowed from a parent or sibling scope
- some parent nodes have a "default subcommand" so that `sniff X` (with no subcommand typed) behaves like `sniff X foo`
    - this dispatch is a convenience for terminal output only
    - `--json` always returns the scope of the node you typed, regardless of any default subcommand at that node

The Golden Rules:

- at every level of the command structure of the `sniff` CLI, it is the JSON output that determines the "scope" of metadata
- JSON output always includes all the metadata in scope (excluding the exception below)
- a parent node's JSON is the aggregate of its children's scopes, keyed by each child's subcommand name — so the JSON keys at a parent map back 1:1 to the subcommands you could drill into
- a leaf node's JSON contains only that leaf's own data
- default subcommands are a terminal-output convenience only; they have no effect on `--json` dispatch
- Terminal output is a design decision which relies on keeping the information easily digested

The Golden Exception:

- **sniff** is a host detection library and so in the few cases where we supplement host data using network requests this supplemental data is treated differently
- supplemental data is ONLY included -- to the terminal or to JSON -- when the `--with-network` CLI switch is used

## Example

Given a parent node `repo` with child subcommands `name`, `packages`, and `areas`:

```sh
sniff repo name --json
# { "name": "rusty-biscuit" }
# leaf scope only — no `is_monorepo`, no `package_count`, no siblings

sniff repo --json
# {
#   "name": "rusty-biscuit",
#   "packages": [ ... ],
#   "areas": [ ... ]
# }
# aggregate of every child scope, keyed by subcommand name
# the keys (`name`, `packages`, `areas`) match the subcommands you could invoke directly

sniff repo
# (with no --json: dispatches to the default subcommand `name` and prints the name)
```
