# Kind Documents

Throughout this repo we utilize the `kind` to formalize what _kind_ of document the document is. This formalism is enforced with a schema defined for this repo at schemas/kind.yaml:

::code ^schemas/kind.yaml

> Note: this schema uses [SimplifiedSchema](^darkmatter/docs/topics/schemas/index.md) grammar




## Schemas (`schema`)

**Schemas** use [SimplifiedSchema](^darkmatter/docs/topics/schemas/index.md) grammar to define enforceable schemas.

> Note:
>
> Darkmatter, DMLS, and (through them) Claudine find a schema by bare name or apply a schema trigger only when the file is in one of the document's five schema roots: the `SCHEMAS_DIR` folder, the document's package `schemas/`, its package area's `schemas/`, the repository's `schemas/`, and `~/schemas`. See [Schema roots](^darkmatter/docs/topics/schemas/definition.md#schema-roots) for the search order and scope rules.

::doc-list kind=schema group-by=package-area

### Schema Triggers

A _dynamic_ schema definition which only activates when it's match expressions match.

A trigger, like a `types`-only `kind: schema` library, may carry a kebab-case `name` that diagnostics show beside its path, as in `titled-notes (schemas/titled.yaml)`. The label must be unique within one schema root: when two files of a root declare the same `name`, both are reported and neither applies. It is only a label; every reference still names the file by its path. See [Trigger labels and local types](^simplified-schema/docs/topics/language/schema-documents.md#trigger-labels-and-local-types).

::doc-list kind=schema-trigger group-by=package-area

### YAML Document Kinds (`yaml-schema`)

A `kind: yaml-schema` document formalizes a kind of **YAML** document. Its `$schema` declares the `kind` it claims with a required literal, and every `.yaml`/`.yml` document whose top-level `kind` has that value is validated against it, by `md schema validate` and in the editor through DMLS. A YAML document without a claimed `kind` falls back to its root `$schema`, if it has one.

```yaml
# schemas/roster.yaml
kind: yaml-schema
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
```

A definition is found the same way as a schema: in one of the document's five schema roots. `schema`, `schema-trigger`, `yaml-schema`, `namespace-schema`, `expression-schema`, and `mutation-schema` are themselves `yaml-schema` kinds that ship with SimplifiedSchema, and Darkmatter and DMLS check files of those kinds against them. See [YAML documents](^darkmatter/docs/topics/schemas/definition.md#yaml-documents) for selection, discovery, and failure rules.

::doc-list kind=yaml-schema group-by=package-area

### Host Namespaces (`namespace-schema`)

A `kind: namespace-schema` document declares one **host namespace**: a global that a host's template expressions read, such as Claudine's `loop` or `seq`. Its `$schema` is one object whose properties are the namespace's members, and an optional `types` table holds named types private to the file:

```yaml
# claudine/registration/loop.yaml (shortened)
kind: namespace-schema
namespace: loop
description: "The nearest enclosing loop's current pass and the previous pass's result."
$schema:
    count: "number(integer; min(1)) -> The one-based iteration."
    is_first: "boolean -> Whether this is the first iteration."
```

A namespace file is read only by a host's global catalog, typically as a member of a published host registration, where it imports other files by `./` or `../` path alone. It is never a `$schema` target, a `Name@file` import source, or a YAML document kind selector. See [`kind: namespace-schema`](^simplified-schema/docs/topics/language/schema-documents.md#kind-namespace-schema) for every rule.

::doc-list kind=namespace-schema group-by=package-area

### Callable Catalogs (`expression-schema` and `mutation-schema`)

A `kind: expression-schema` document declares value-producing expression functions; a `kind: mutation-schema` document declares signature-backed lifecycle actions. Both carry required `categories` and `types` (with optional `description`), export no `$schema`, and keep all helper types private to the file:

```yaml
# example expression-schema (shortened)
kind: expression-schema
categories:
    - Text -> Transform text.
types:
    Label: string(min(1))
    kebab_case:
        - |-
            function(
                parameters(x: Label);
                returns(string);
                category(Text);
            ) -> Converts a label to kebab-case.
        - |-
            function(
                parameters(x: null);
                returns(null);
            ) -> Passes null through unchanged.
```

Simplified Schema checks the envelope shape and type syntax strictly; Darkmatter runs the callable semantic checks (required category, overload agreement, local-only helpers, name collisions, and eager binding) in `md schema validate`, DMLS, and host registration. Neither kind can be referenced as a `$schema` target or `Name@file` import source, and host registrations accept `expression-schema` (paired with a derived `.facts.json` companion) while refusing `mutation-schema`. See [Callable catalogs](^simplified-schema/docs/topics/language/schema-documents.md#callable-catalogs-kind-expression-schema-and-kind-mutation-schema) and [Host functions](^darkmatter/docs/topics/host-functions.md).

::doc-list kind=expression-schema group-by=package-area

::doc-list kind=mutation-schema group-by=package-area

## Index Documentation (`index`)

In the `docs` folders of each package area in this monorepo you'll find documents that broadly cover important functionality exposed in the 

::doc-list kind=index group-by=package-area
