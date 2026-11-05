# Simplified Schema Types

The _types_ which `SimplifiedSchema` provides will be broken up into four discrete categories:

1. Basic Types - _the basic primitive types you'd expect in a schema grammar_
2. Ergonomic Helper Types - _types added due to their high reuse potential, their ability to express a type pattern that would otherwise be hard to express, or both_
3. Metadata Types - _types used to define types or schemas_
4. Functions - _being able to express a type grammar for functions is rarely a need for Markdown authors but there are edge cases and being able to do so really rounds out the grammar's capabilities_

In this document we will go category by category and describe all the _types_ that `SimplifiedSchema` provides while also expressing any **constraints** which are unique to this type.

> **Note:** 
> 
> - _all types_ have a set of constraints that can be used and those are found in [**Global Type Constraints**](./global-type-constraints.md) (e.g., `required`, `eager`, etc.)
> - _all types_ can describe properties in the schema by using the `-> {description}` grammar at the end of properties definition; for more on this see [**Using Prose Descriptions in Schemas**](./using-prose-in-schemas.md)

## Basic Types

::file _basic-type-summary.md

### `string`

### `number`
