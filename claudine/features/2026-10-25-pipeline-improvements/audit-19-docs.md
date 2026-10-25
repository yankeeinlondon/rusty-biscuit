---
kind: implementation-log
---

# Documentation audit: 2026-10-29-recursive-schema-types

Updated **4 existing canonical documents**; created **0 new paths**. One
updated path, `schema-triggers.md`, was an empty placeholder and now contains
the missing trigger-payload guide. Eight canonical references are recorded in
the completed spec. Every referenced page has
`kind: canonical-documentation`; its snapshot body is unchanged.

| Canonical document | Functionality covered | Change in this audit |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/definition.md` | Child-value cycle rule, definition identity and use-site metadata, phases, conversion, JSON Schema lowering, merge safety, library APIs | Clarified finite-value/resource limits and that acyclic types remain inlined; the detailed recursive contract was already present. |
| `darkmatter/docs/topics/schemas/authoring-schemas.md` | Practical tree example, finite leaves, supported cycles, references and trigger authoring | Reviewed current example and links; no edit needed. |
| `darkmatter/docs/topics/schemas/local-type-references.md` | Local/cross-file names, inheritance, recursive source namespaces and serialization | Reviewed current recursive reference guide; no edit needed. |
| `darkmatter/docs/topics/schemas/schema-triggers.md` | Recursive trigger payloads, layer precedence and separate definition identities, safe raw `$defs`, phases and passive behavior | Filled the existing empty page with a complete trigger/document example, explanation and links to the deeper references; added canonical kind. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Lazy cursor-directed completion/hover, navigation across cycles, positioned deep failures and cycle diagnostics | Reviewed complete existing recursive section; no edit needed. |
| `darkmatter/docs/inline/schema-validation.md` | CLI/library timing reports, cold/warm benchmarking, cache correctness and context-sensitive validation | Added reproducible schema-only benchmark commands and explained frozen workloads, process-cache reset, separate resolution/I/O/compile measurements, and context/diagnostic isolation. |
| `darkmatter/docs/lsp/configuration.md` | Slow-validation threshold, defaults, zero/invalid values and settings precedence | Reviewed current configuration contract; already canonical and accurate. |
| `darkmatter/docs/lsp/indexing.md` | DMLS stage event, cache states, failures, request scope and privacy | Added canonical kind; existing timing-field and lifecycle explanations remain current. |

The empty trigger page was a documentation coverage gap, not a missing public
API. It now gives an unfamiliar developer an obvious starting point for
applying recursive schemas automatically. The detailed recursive reference
and editor pages already describe the grammar, finite leaves, child-value
admission, positions, and lazy descent, so they are linked rather than copied.

## Validation

Compared the recursive contract with the resolver graph and shared-definition
implementation, the merge/reference scope rules, and the current reference-
aware tests. Checked the benchmark explanation against
`lib/benches/schema_validation.rs` and the area's `bench-schema` recipe:
frozen cold/warm cases reset process caches outside timed work, and the
recursive/unrolled groups separate resolution, I/O, and compilation. Checked
CLI timing fields and the DMLS configuration/logging topics against their
implemented contracts and the implementation log.

All eight implicit repository-relative references exist under package-area
`docs/` folders and carry the requested kind. The spec body is byte-for-byte
unchanged from HEAD. Relative Markdown targets in the four edited docs exist,
and `git diff --check` passes. No feature/fix links or names were introduced
into canonical docs. No stale skill statement needed changing: the Darkmatter
schema skill already documents recursive graph admission, typed references,
definition storage, scoped merge rewriting, and timing ownership. Tests and
timing measurement belong to the concurrent test agent; no test suite or
benchmark was run by this documentation slice.
