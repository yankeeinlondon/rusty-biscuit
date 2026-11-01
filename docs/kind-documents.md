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

A definition is found the same way as a schema: in one of the document's five schema roots. `schema`, `schema-trigger`, and `yaml-schema` are themselves `yaml-schema` kinds that ship with Darkmatter. See [YAML documents](^darkmatter/docs/topics/schemas/definition.md#yaml-documents) for selection, discovery, and failure rules.

::doc-list kind=yaml-schema group-by=package-area

## Index Documentation (`index`)

In the `docs` folders of each package area in this monorepo you'll find documents that broadly cover important functionality exposed in the 

::doc-list kind=index group-by=package-area
