# Basic Types in Simplified Schema

## Types

### `string`

The **string** type exists in every typed programming language as well as any schema grammar worth it's salt and let's be clear `SimplifiedSchema` is nothing if it isn't a bit _salty_ (in attitude not functionality).

We will not cover the constraints that are common to _all_ types but let's make sure you know those constraints that are unique (or at least partial) to the **string** type:

- `suggest(v1,v2,v3,...)`
    - we all know what an **enumeration type** is -- _and if you don't, have a look below for an example_ -- and where an enumeration expresses a discrete set of literal types as variant and then REQUIRES that the value be one of the stated variants ... `suggest` offers a slightly less opinionated variation on that theme
    - the `suggest` constraint allows you list a set of variants which represent good examples of what this **string** type might be but these are just _suggestions_ and do not actually narrow the string type to require that one of these variants be used.
    - this can be useful as metadata to downstream consumers like [DMLS](darkmatter/docs/topics/dmls.md) which can now auto-complete these suggestions for users but also knows that _any_ **string** is a valid type for the property with this type
    - this kind of _constraint_ doesn't strictly "constrain" the type but it provides context and examples to consumers when a downstream system leverages this metadata
- `default(val)`
    - the `default(val)` constraint, like `suggest`, is not _strictly speaking_ a "constraint" but instead metadata for downstream consumers. In this case the metadata indicates what the default value for a variable that is not defined (e.g., `null`)
- `min(#)` _and_ `max(#)`
    - we often want to constrain how long (or short) a string is allowed to be (in characters)
    - the `min` and `max` constraints allow us to use one or both to apply these constraints.
- `starts_with(prefix)`, `ends_with(postfix)`, `pattern(regex)`
    - all three of these constraints narrow the string type by making some portion of the string be a "string literal"
    - because we often want to ensure that a string _starts with_ or _ends with_ a string literal we have an ergonomic way to express this
    - example:

        ```yaml
        $schema:
            secret: string(starts_with(_); ends_with(. Tell no one!))
        ```
    
      > the `secret` property must now be a type of `_${string}. Tell no one!` (using Typescript nomenclature)
    - if you want to create a union type for the string literal, this too can be done by leveraging the `|` operator symbol:

        ```yaml
        $schema:
            name: string(starts_with(Mr.|Mrs.|Ms.))
        ```

        > In this example the type of the `name` property is 
        > 
        > `${"Mr." | "Mrs." | "Ms."}${string}`

    - if the `starts_with(prefix)` and `ends_with(postfix)` constraints don't give you enough precision on the constrained string you want to create you can leverage the `pattern(regex)` constraint:
        - this constraint allows for any valid regex expression to be used to establish 

- `not-empty`
    - a normal "string" type can be any string and that includes an _empty_ string
    - adding the `not-empty` constraint makes an _empty_ string invalid
    - it also invalidates a string that _only_ contains whitespace

### `number`


### `numberlike`

When we are interacting with HTML, the terminal, and other forms of input it is not uncommon to get numeric values expressed as strings (e.g., "4", "999", etc.). The `numberlike` type expresses that both explicit numeric types and numeric strings are valid.

### `boolean` _and_ `boolish`



### `enum(v1,v2,v3,...)`

An enumeration offers a discrete set of literal variants to choose from and at runtime a value assigned to the enum type must match exactly one of these variants.

### `unknown` / `any`

When a variable is not assigned a _type_ then it's type defaults to `unknown` (or `any`). Both `unknown` and `any` types are identical and represent the union of ALL types. In addition to _defaulting_ to the `unknown` type, schemas can explicitly define a property to have the `unknown` type.

### `null`


## Related Docs

- [Types in Simplified Schema](../types.md)
