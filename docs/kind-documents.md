# Kind Documents

Throughout this repo we utilize the `kind` to formalize what _kind_ of document the document is. This formalism is enforced with a schema defined for this repo at schemas/kind.yaml:

::code claudine/features/2026-09-21-schema-enhancements/drafts/kind-document.yaml

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
