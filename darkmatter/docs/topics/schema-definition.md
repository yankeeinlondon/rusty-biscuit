# Schema Definitions in Darkmatter

Today YAML has no _formal_ mechanism for referencing a **schema** for the YAML data structure but you will still see many cases where the key `$schema` is being used to reference a [JSON Schema](https://json-schema.org/) definition. Unfortunately most editors ignore this but there are some efforts to get this formalized in the future.

Darkmatter, however, is impatient and has decided not to wait for standards to implement it's own solution. That said, some attempt at being somewhat in line with what is informally being used today is a goal. The high level structure and feature set of Darkmatter's implementation is:

- use the `$schema` property to define the schema
    - The schema can reference a local file or an external URL
    - It can also be defined _inline_ as part of the Frontmatter of a document
- allow for two definition formats:

    1. JSON Schema
          - the same informal _referencing_ of JSON Schema definitions which are seen today in the wild
    2. Simplified Schema
          - a YAML based definition that can be easily defined, understood, and converted to JSON Schema (if so desired)

## JSON Schema Syntax

::file @darkmatter/docs/topics/json-schema-primitives.md 

Frontmatter is always modelled as a key/value object, so we could define frontmatter in the most generic way possible as just:

```json
{
    "type": "object"
}
```

However, Darkmatter has a "base" schema that it uses which looks like:

```json
{
    
}
```

Beyond these primitives JSON Schema introduces the concept of "constraints" which can be placed on 

## Simplified Schema Syntax

The simplified syntax 
