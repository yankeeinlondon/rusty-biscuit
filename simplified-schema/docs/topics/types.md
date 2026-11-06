---
kind: canonical-documentation
classification: schema/types
---

# Simplified Schema Types

The _types_ provided by `SimplifiedSchema` are organized into categories and in this document we will introduce each category, provide the inventory of types in that category, and provide links to get further details on this type category including .



provides will be broken up into the following categories:

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

Basic types provide an informal 80/20 rule for the type system. They aren't the sexiest types but they tend to be the most common types found in the wild. These types include:

- `string`
- `number`
- `numberlike`
- `boolean`
- `unknown` _or_ `any` (_both are the same type_)
- `enum`
- and `null`

For details on these types including the specific constraints these types expose you should read: [Basic Types in Simplified Schema](./types/basic-types.md)

## Serialized Data Types

There are a number of very well recognized serialization formats which _could_ be considered a constrained/narrowed type of "string" because they are all serialized into a string form. However, being able to express the serialized data format as a **type** is not only ergonomic but in effect a stronger guarantee than could be achieved by even the finest RegEx craftsman out there.

These types include:

- `json`
- `json5`
- `jsonl`
- `yaml`
- and `toml`

For details on these **serializable** data types you should read: [Serialized Data Types in Simplifed Schema](./types/serializable-types.md).


## Date/Time Types

Being able to have date, time, or datetime _types_ is a convenience worth having rather than trying to constrain strings to fit the representative ISO formats with RegExp expressions. These types include:

- `date`
- `time`
- `datetime`

For details on these **date/time** types you should read: [Date/Time types in Simplifed Schema](./types/date-time-types.md).

## Other Ergonomic Types

- `email`
- `phone`
- `country_code`, `country`
- `zip`, `zip5`, `zip5-4`

## Container Types

Array, object, and tuple.

## Union Types


## Functions


## Meta Types

- `expression`
- `type-definition`
- `schema`
