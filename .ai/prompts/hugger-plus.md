# Hugger Plus

The various _types_ of symbols found by the `tree-hugger` packages must have good metadata captured. I can see that for functions, for instance, we currently capture the name, language, file, positional info, and "kind" but a function has:

- parameters
    - those parameters have types
- a return type

A function -- or really any symbol -- may be annotated with comments.

These attributes need to be captured.

## CLI

- When reporting with the `--json` flag set the goal is to report with completeness in JSON format.
- When _not_ reporting with `--json` flag we are targeting the terminal and should be aiming to have a clear output for the user. This includes escape codes for colors, OSC8 links for files, etc.
- We should add a `--plain` flag which skips adding any of the terminal escape codes


## Testing

Testing is woefully inadequate at the moment.

- always use `rust-testing` skill when testing
- make sure we have far better test coverage
- we need fixtures for various languages which we can test against


