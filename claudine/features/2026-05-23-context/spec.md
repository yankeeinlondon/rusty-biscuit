# Context subcommand

We are going to add a **context** subcommand to **claudine CLI**: `claudine context`.

- when run we will provide a full overview to the context variables which Darkmatter provides to Markdown documents
- review the document @darkmatter/docs/topics/context-variables.md to understand all the context variables available
- by default when someone types `claudine context` we will:
    - use the H3 and H4 headings in the context-variable.md document to organize information
    - under each H4 heading (or H3 heading if the section has no H4 sub-sections) add a table with the following columns:
        - "Property" (e.g., `ctx.today`, etc.)
        - "Type"
        - and "Description"

- at the bottom of the report -- regardless of whether `--values` flag was used -- we will add:
    - a StatusInfo::Info message to stderr which says "use <blue>--expressions</blue>" to see the expression engine's operations and functions"
    - a StatusInfo::Info message to stderr which says "use <blue>--side-effects</blue>" to see the available <i>safe</i> side effects that <b>Claudine</b> provides without need for being white listed."


## Values

- we will provide a CLI switch `--values` which renders the same report but removes the Description column and replaces it with a "Value" column which render's the current host's values

## Expressions

- the document @darkmatter/docs/topics/darkmatter-expressions.md describes the different:
    - explains where these expressions can be used
    - operations provided
    - utility functions (min(), is_string(), etc.)
- spend the time up front to create a well structured report that looks nice, conveys information clearly, and is not overly verbose

## Side Effects

- For now just output "not implemented yet" as this is going to be introduced soon but doesn't exist yet
