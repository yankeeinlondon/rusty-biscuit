# Kind Documents

Throughout this repo we utilize the `kind` to formalize what _kind_ of document the document is. The formal values are listed in a schema at [`schemas/kind.yaml`](^schemas/kind.yaml):

::code ^schemas/kind.yaml

The schema documents the values; it is not applied to documents automatically. A document is checked against it only when it names the file as its `$schema`. Many documents in this repo use other `kind` values that the list does not include yet.

Two of these values are formal YAML document kinds that Darkmatter validates by their `kind` alone: a YAML file whose top-level `kind` is `schema` or `schema-trigger` is checked against a definition that ships with Darkmatter (see [External Grammar Files](^darkmatter/docs/topics/schemas/authoring-schemas.md#external-grammar-files)).

> Note: this schema uses [SimplifiedSchema](^darkmatter/docs/topics/schemas/index.md) grammar




## Schemas (`schema`)

**Schemas** use [SimplifiedSchema](^darkmatter/docs/topics/schemas/index.md) grammar to define enforceable schemas.

> Note:
> 
> In order to have a schema used by Darkmatter, DMLS, and Claudine by extension, it must be in one the following directories:
>
::file ^darkmatter/docs/topics/schemas/

::doc-list kind=schema group-by=package-area

### Schema Triggers

A _dynamic_ schema definition which only activates when it's match expressions match.

::doc-list kind=schema-trigger group-by=package-area

## Index Documentation (`index`)

In the `docs` folders of each package area in this monorepo you'll find documents that broadly cover important functionality exposed in the 

::doc-list kind=index group-by=package-area
