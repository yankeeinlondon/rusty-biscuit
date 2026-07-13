# Data Modeling

The canonical data model — the CRDT document taxonomy (fact logs / state registers / multi-writer), the single-writer rule, the ephemeral presence layer, document addressing, and the DuckDB star-schema projection — lives in the shared design doc:

→ [rendezvous data model](../../rendezvous/docs/crdt.md)

This file is reserved for *feature-specific* modeling notes that fall out of the logging refactor's open decisions (in particular D1, the canonical `ClaudineAgenticLog` envelope, and D3/S3, session correlation). Author those here once the decisions in [spec.md](./spec.md) are ratified, rather than duplicating the shared taxonomy.
