---
kind: canonical-documentation
classification: schema/types
---

# Simplified Schema Types

The _types_ which `SimplifiedSchema` provides will be broken up into the following categories:

1. **Basic Types** - _the basic primitive types you'd expect in a schema grammar_
2. **Ergonomic Helper Types** - _types added due to their high reuse potential, their ability to express a type pattern that would otherwise be hard to express, or both_
3. **Container Types** - 
4. **Metadata Types** - _types used to define types or schemas_
5. **Functions** - _being able to express a type grammar for functions is rarely a need for Markdown authors but there are edge cases and being able to do so really rounds out the grammar's capabilities_

In this document we will go category by category and describe all the _types_ that `SimplifiedSchema` provides while also expressing any **constraints** which are unique to this type.

> **Note:** 
> 
> - _all types_ have a set of constraints that can be used and those are found in [**Global Type Constraints**](./global-type-constraints.md) (e.g., `required`, `eager`, etc.)
> - _all types_ can describe properties in the schema by using the `-> {description}` grammar at the end of properties definition; for more on this see [**Using Prose Descriptions in Schemas**](./using-prose-in-schemas.md)

## Basic Types

::file _basic-type-summary.md

### `string`

The **string** type exists in every typed programming language as well as any schema grammar worth it's salt and let's be clear `SimplifiedSchema` is nothing if it isn't a bit _salty_ (in attitude not functionality).

We will not cover the constraints that are common to _all_ types but let's make sure you know those constraints that are unique (or at least partial) to the **string** type:

- `suggest(v1,v2,v3,...)`
    - we all know what an **enumeration type** is -- _and if you don't, have a look below for an example_ -- and where an enumeration expresses a discrete set of literal types as variant and then REQUIRES that the value be one of the stated variants ... `suggest` offers a slightly less opinionated variation on that theme
    - the `suggest` constraint allows you list a set of variants which represent good examples of what this **string** type might be but these are just _suggestions_ and do not actually narrow the string type to require that one of these variants be used.
    - this can be useful as metadata to downstream consumers like [DMLS](darkmatter/docs/topics/dmls.md) which can now auto-complete these suggestions for users but also knows that _any_ **string** is a valid type for the property with this type
    - this kind of _constraint_ doesn't strictly "constrain" the type but it provides context and examples to consumers when a downstream system leverages this metadata
- `min_length(#)` _and_ `max_length(#)`
- `starts_with(prefix)`, `ends_with(postfix)`, `pattern(regex)`
- `not-empty`


### `number`

### `boolean`

### `enum(v1,v2,v3,...)`

### `null`

## Ergonomic Types


## Container Types


## Meta Types
