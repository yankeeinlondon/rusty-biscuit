# Local Type References

A schema can define reusable types under `types` and use them in its exported
`$schema`. See [External Schemas](./authoring-schemas.md#external-schemas) for an
authoring example. Bare local names are intended shorthand for explicit `@this`
references; the current parser still requires `@this`.

## Resolving a Name

Built-in keywords retain their existing meaning. Otherwise, a bare type name
refers to the `types` section of the schema where that reference was written.
For example, `code` and `code@this` select the same local definition. An explicit
reference can also disambiguate a local name that matches a built-in keyword:
`string` means the built-in type, while `string@this` selects a local definition.

There is no search through other active schemas, frontmatter properties or
runtime globals. A missing name is a resolution error. Forward references use
the complete local type table, and cycles follow the existing bounded,
cycle-checked named-import resolution rules.

Importing a definition does not change what its local references mean. If
`address@./contacts.yaml` uses a bare `postcode` type, that type comes from
`contacts.yaml`, not from the importing document.

The parser should preserve the name and authored span, then use the existing
named-import resolver with the defining namespace. Bare and explicit forms
must share constraint, array, union and description handling. The same resolved
definitions serve runtime generation, validation and DMLS assistance.

## Inheriting Constraints

A named type carries its constraints wherever you use it. You should not have
to repeat `required`, `generated`, or another constraint just because the type
has a name.

For example, given:

```yaml
types:
    code: string(required;generated)
```

The following property inherits both constraints:

```yaml
$schema:
    err:
        code: code
```

Writing `code@this` or importing the same definition from another file preserves
the same constraints. This also applies when one named type references another.
You can add constraints where you use the type, but leaving them out does not
remove the constraints already defined on it.

For example, a reusable label can set a minimum length while the property using
it adds a maximum:

```yaml
kind: schema
types:
    label: string(min-length(3))
$schema:
    title: label(max-length(80))
```

The resulting `title` accepts strings between 3 and 80 characters long. If the
property instead adds `min-length(5)`, the stricter minimum wins: at least 5
characters. Maximum bounds work the other way: the smaller maximum wins.

Contradictory constraints are schema errors. A minimum length of 10 combined
with a maximum length of 5 cannot be satisfied, so Darkmatter reports the
conflict instead of silently dropping either constraint.

This is different from [Schema Layering](./authoring-schemas.md#schema-layering).
Reusing a type combines its constraints with the property's additional
constraints. Layering separate schemas chooses the whole property definition
from the higher-precedence schema.

Constraints inside nested objects and on array elements or containers keep
their respective meanings. The `generated` constraint retains its existing
validation behavior; inheriting it does not create a runtime value provider.

> **Implementation status:** The current resolver incorrectly strips top-level
> `required` and `generated` when a named type is reused. Correcting that bug,
> along with adding bare local names, is pending implementation. The rule above
> describes the intended behavior for all reference forms.
