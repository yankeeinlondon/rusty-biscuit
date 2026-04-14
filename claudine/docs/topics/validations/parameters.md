# Parameters in Claudine

**Claudine** provides a simple grammar for _documents_, _headless sequences_, and _headless splits_ to define parameters so they can formally declare how to pass state into them.


> **Note:** _headless splits_ do not yet exist

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
- `Array<T>`

Each of these types also allows a `Option<T>` variant which expresses an optional parameter:

- `Option<String>` represents an optional string
