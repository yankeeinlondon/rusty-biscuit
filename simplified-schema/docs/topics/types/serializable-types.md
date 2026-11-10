# Serializable Types in Simplified Schemas


There are a number of very well recognized serialization formats which _could_ be considered a constrained/narrowed type of "string" because they are all serialized into a string form. However, being able to express the serialized data format as a **type** is not only ergonomic but in effect a stronger guarantee than could be achieved by even the finest RegEx craftsman out there.


## Types


### `json`

- to be a valid `json` type the value must be a string and that string must be a valid JSON serialization

### `jsonl`

- the JSONL format is an array of JSON serializable values where the _array_ is represented in string form and each element of the array is delimited by a `\n`

### `json5`

- to be a valid `json5` type the value must be a string and that string must be a valid JSON5 serialization
- **Note:** a valid "json5" type is a superset of "json" as JSON5 allows for all the awkward key quoting that JSON requires but allows for bare key names too

### `yaml`

- to be a valid `yaml` type the value must be a string and that string must be a valid YAML serialization

### `toml`

- to be a valid `toml` type the value must be a string and that string must be a valid TOML serialization



## Constraints

All of the serializable data types offer a "type" constraint to express what shape the serialized data is expected to be in. This constraint is entirely optional.

```yaml
$schema:
    any_json: json
    specific_json: json(string[])
    people: json(person[])
types:
    person:
        name: string(required)
        age: number
```

## Related Docs

- [Types Overview for Simplified Schema](../types.md)
