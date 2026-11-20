# Ergonomic Sequences

We have improved sequences but today we use the `prompt` directive to perform a "compose" or "inline-compose" operation
on a Markdown or YAML file and that might be ok but being more explicit would be better. In addition, today, a sequence
can't call another sequence!

In this specification we will add the following operations:

- `compose` - composes the referenced document and uses as a prompt to an agent; produces an error if the referenced document has a "prompt" or "sections"
  frontmatter property (indicating that it is intended to be executed with `inline-compose`)
- `inline-compose` - same as compose but with the intention of executing an inline-compose operation; error if the neither the `prompt` or `sections`
  frontmatter properties are set
- `sequence`

All three operations will have the format of:

- short form: `{operation}: <file-reference>`
- long form:

    ```yaml
    $schema:
        compose: fn
        inline-compose: fn
        sequence:
    ```

All three operation will accept the `with` parameter in the long form which allows for explicit setting of properties
of Frontmatter state prior to implementing. Note, however, that _by default_ the sequence's state is passed into each
child operation.
