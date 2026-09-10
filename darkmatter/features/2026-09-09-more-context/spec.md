

Add:

- `ctx.self` - a file path to the current file
- `ctx.hostname` - the hostname of the computer operating on
- `ctx.id` - a unique id for the document instance:
    - generated from an XX hash of the page plus:
        - an ms epoch timestamp
        - the hostname
        - the repo's name
- `ctx.sid` - uses the same underlying components as `id` but uses a cryptographically secure hasher:
    - this makes the ID impossible to map back to the page
    - but it also makes it slower to generate
