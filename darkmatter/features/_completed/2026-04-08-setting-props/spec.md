Currently when we want to set the initial state of the frontmatter before kicking off the [compose pipeline](@claudine/docs/darkmatter-compose-pipeline.md) we use the `--set <obj>` notation. This can be handy but it also can get awkward when you just want set a single key/value.

This feature will provide a secondary means of setting initial state of the frontmatter when using the `md compose` command:

- any `key=value` parameters will be treated as setters
- for example, `md compose foobar.md iteration=1` will kick off the compose function with frontmatter property `iteration` set to `1`.


