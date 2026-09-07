

In order for the _symbolic links_ (the preferred approach) to be valid across as many providers as possible, Claudine will first "upgrade" the agent skill by:

1. ensuring that both "name" and "description" Frontmatter are set

    - `name` -- _if not set already_ -- can always be inferred and set
    - `description` -- _if not set_ -- is not inferrable via a deterministic process

2. 


In addition to this primary role of ensuring that all skills are available everywhere, Claudine provides some reporting functionality that can be useful:

```sh
# list all Skills that are in scope based on the directory you are in
claudine skills
# filter down skills available to only those which match the filter string
claudine skills <filter-string>
```

If there are a large number of skills in scope then the skill names will be listed -- _grouped by scope_ -- but no additional metadata for the skill will be shown:

TODO: add image

> if there are either synchronization gaps or malformed skill files detected they will be listed below the skills in an **Exceptions** section.

If the number of skills in scope is more limited then more metadata for each skill will be provided for additional context.
