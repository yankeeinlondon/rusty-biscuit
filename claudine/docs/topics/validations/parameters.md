# Parameters in Claudine

**Claudine** provides a simple grammar for _Markdown documents_ and _headless sequences_ to define parameters so they can formally declare how to pass state into them.

So if a markdown document -- `example.md` -- wants to define a single parameter `name` as a required string parameter:

```json5
{
    parameters: {
        name: "String"
    }
}
```

Now the Claudine CLI will help reinforce this contract:

```sh
# ❌ results in an error because we did not pass in the required `name` parameter
claudine compose example.md
# ❌ results in an error because `name` must be a string not a number
claudine compose example.md name=42
# ✅ the `name` parameter passed in correctly, the `example.md` document is composed
claudine compose example.md name=Bob
```

## Parameter Types

The valid _types_ a parameter can be defined include:

- `String` - any string based value
- `Filepath` - a valid file path (_must be valid at the time of execution/composition_)
- `Boolean` - a boolean true/false
- `Number` - a numeric value (integer or float)
- `Enum<foo,bar,baz>`
- `Dictionary` - a key/value object type

### Variants

There are two variants that can be applied to a type:

- `Option<T>`
    - `Option<String>` is an _optional_ string
- `Array<T>`
    - `Array<String>` is an _array_ of strings

### Default Values

If a Markdown document or [Sequence](../agent-flows/sequences.md) wants to set a _default value_ for a parameter they 
can make the value of the parameter definition a tuple and set it as the second element:

```json
{
    "parameters": {
        "iteration": ["Number", 1]
    }
}
```

In this example we're declaring a parameter `iteration` which a caller _can_ set when calling the document or sequence but because it has a _default value_ the caller is not obligated to set it. Even though the caller is given latitude to set or not set, the **type** remains a required type because either the caller or the _default value_ will set the parameter.

## The `describe` Command

With the introduction of parameter schemas, we get a new command `describe`:

```sh
claudine describe example.md
```

- reports:
    - `<blue-500><b>example.md</b></blue-500>`
    - if `description` Frontmatter property is set:
        - `<dim><i>{description}</i></dim>\n`
    - `  - <i>requires</i> <inverse>name</inverse> <i>as</i> <b>String</b>`
- you can use the optional `--json` CLI switch to get a JSON payload:

    ```json
    {
        "description": {description},
        "parameters": {
            "name": "String"
        }
    }
    ```

## Shell Completions

One of the nice things about explicitly declaring your parameter types is that **Claudine**'s **shell completions** can provide type-aware completions for you.

- defined keys are tab completable
- `Filepath` types provide tab completion to valid file references

