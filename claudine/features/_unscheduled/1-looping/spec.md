While a lot of the flow control in Claudine is managed by **sequence**'s in this feature we'll add an atomic
flow control feature to the individual prompt called "looping" and defined by the `loop` Frontmatter property:

- it is a very common requirement to need to _loop over_ a certain prompt file and that is the primary utility that this feature will provide
- the `loop` property must consist of both a "conditional" (to determine when to stop looping) and 99% of the time a "action mutator" that will change the Frontmatter state in some way during the iteration.

## Conditionals

Conditionals are defined by one of the operators (as a frontmatter property under `loop`) defined in the next section.

### Operators

- `while`
    - receives a boolean expression to evaluate and so long as the expression evaluates to `true` it will continue to iterate
- `until`
    - receives a boolean expression to evaluate and so long as the expression evaluates to `false` it will continue to iterate

### Conditional Expressions

The conditional expressions allowed are those defined in Darkmatter in the @darkmatter/docs/topics/boolean-conditional-expressions.md document.

## Action Mutators

The action mutator(s) are defined by the `action` frontmatter property under the `loop` property. A looping prompt can have 0:M actions on each loop but these actions are only _applied_ after the completion of a prompt (aka, no action taken when the prompt is first executed).

### Action/Mutation Operations

- `increment(prop)`

    Increments the specified property by 1.

    > **Note:**
    >
    > - if the property specified is empty/null/undefined then incrementing will make it **1**
    > - if the property specified is not a numeric (or a string representation of a number) property then this will cause a `InvalidIncrementType` error to be returned; stopped execution immediately.

- `decrement(prop)`

    Decrements the specified property by 1.

    > **Note:**
    >
    > - if the property specified is empty/null/undefined then incrementing will make it **1**
    > - if the property specified is not a numeric (or a string representation of a number) property then this will cause a `InvalidDecrementType` error to be returned; stopped execution immediately.

- `set(prop, value)`

    Sets a property in the next iteration of the Frontmatter.

    > **Note:**
    >
    > - you can not set the "loop" or "replace" properties and if you do then it will cause a `InvalidAction` error; stopped execution immediately.

- `append(prop, value)`

    Appends a value `value` to the Frontmatter property `prop`.
    - this is intended to be used with a Frontmatter property with string content, but:
        - if the value is numeric or boolean it will be converted to a string equivalent before being appended
        - if the ${value} is an object/dictionary then it will be serialized to JSON (in a compact form with no new lines) and then a "\n" character placed at the front before being appended to the frontmatter property
            - this will result in a JSONL variable being built up if this "value" is consistently a dictionary
        - if the ${value} is a list/array then it will be serialized to a JSON array and appended to the property as `\n${json-array}`
        - because the object and array types will lead to the formation of a JSONL based string, if we get a `value` which is either an empty string or undefined/null we will preserve the JSONL pattern by:
            - if the first line in the Frontmatter[`prop`] is a JSON object then we'll add '{}' otherwise '[]'

- `prepend(prop, value)`

    Prepends a value `value` to the Frontmatter property `prop`.

    > Note: behaves the same as append but in reverse; that includes putting the `\n` character _after_ the new line instead of before

- `merge(prop, value)`

    This assumes that the Frontmatter `prop` is either empty/null/undefined or an object shaped property. If it is not then we will immediately stop execution with a `InvalidAction` error.
