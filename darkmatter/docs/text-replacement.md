# Text Replacement

In Darkmatter we treat the `replace` frontmatter property with special meaning:

- to be considered appropriate for a Darkmatter text replacement, `replace` property is a dictionary type; for example:

    ```md
    ---
    replace:
        foo: bar
        one: two
    ---

    Some prose about foo.
    ```

- Calling `.transform()` on a Markdown struct will kick off the Markdown pipelining features

