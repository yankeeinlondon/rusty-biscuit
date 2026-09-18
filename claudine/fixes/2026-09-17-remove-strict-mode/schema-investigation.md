# Schema and editor investigation

Read-only source investigation for the technical design checkpoint, 2026-09-17.
The functional specification remains authoritative. Every proposed design in
this document is a recommendation awaiting human agreement, not a confirmed
decision. No implementation, prototype, or implementation plan was produced.

## Evidence quality

Read the complete specification, repository instructions, Claudine and
Darkmatter skills, Darkmatter schema/DMLS topics, and OS guidance. Graph-first
queries targeted schema discovery, editor refresh, constraints, and schema
generation in the explicitly bound `better-static-analysis` repository.
The graph reported 46 commits behind HEAD and returned corrupted symbol/path
associations alongside empty process results. Context queries for
`DeferredValue` and `load_extension_schema` returned not found. Consequently,
graph results were navigation attempts only; none establishes current callers,
blast radius, or risk. Current source was inspected directly after those
unsuccessful graph queries. A refreshed graph still needs verification.

## Verified current architecture

| Source | Finding and design consequence |
| --- | --- |
| `darkmatter/lib/src/markdown/schemas/triggers/envelope.rs:29` | The actual envelope discriminator is `trigger-schema`; known keys are only `kind`, `match`, `$schema`. This differs from the supplied kind catalog and current skill's `schema-trigger`. A naming/migration ruling is necessary before designing shipped activation artifacts. Do not silently change the spec or infer support for the newer spelling. |
| `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs:112` | Discovery walks ancestor `schemas/` directories up to a supplied boundary. It avoids symlinks, orders filenames by UTF-8 bytes, rejects case-fold collisions, and shadows farther files by filename. This already isolates automatic nested discovery by document ancestry. |
| `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs:300` | `scan` is transactional and resolves every unshadowed trigger payload before matching documents. A broken inactive payload currently invalidates the scan. New failure isolation cannot be layered only into editor error rendering. |
| `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs:1` | Merge order is caller baseline, matching triggers in registry order, then document schema. Registry order is nearest-first, but later layers win property conflicts. Preserve the real merge order unless explicitly changed; nearest-file shadowing and payload precedence are distinct rules. |
| `darkmatter/lib/src/markdown/schemas/resolve.rs:1729` | `merge_baseline` retains document properties wholesale on conflict. Merely attaching the new prohibition to a lower schema property's JSON cannot guarantee non-relaxation. |
| `darkmatter/dmls/src/overlay/mod.rs:415` | On scan error DMLS explicitly retains the last-good registry. Errors are projected to trigger-authoring files. This violates the new required consumer-facing incomplete-validation and no-stale-assistance contract. |
| `darkmatter/dmls/src/overlay/mod.rs:47` | Schema state is presently either `Ready` or `Failed`. It cannot describe current valid independent definitions alongside unavailable dependent definitions. |
| `darkmatter/dmls/src/overlay/mod.rs:390` | Cache identity combines document/config and registry debug hashes; successful schema dependencies are content-hashed. Failed outcomes have no dependency list, so recovery requires explicit invalidation rather than trusting successful-dependency tracking. |
| `darkmatter/dmls/src/overlay/schema.rs:941` | `trigger_boundary` selects the nearest containing workspace. Its documentation claims Git-root narrowing that the function does not implement. Resolve this documentation drift against actual code and the new root-discovery requirements. |
| `darkmatter/dmls/src/overlay/schema.rs:958` | Configured extension baselines merge over the Darkmatter baseline using the shared merge operation. Paths are resolved relative to the first workspace root; activation is glob-based. DMLS owns this assembly plumbing today. |
| `darkmatter/dmls/src/workspace/watch.rs:47` | Watches include configured document globs and `**/schemas/*.yaml` / `*.yml`. Client watch and server-rescan modes already exist. Explicit external directories, non-YAML dependencies, and workspace markers need corresponding watch/invalidation coverage. |
| `darkmatter/dmls/src/router.rs:1089` | Watched changes skip open buffers and refresh all diagnostics after applying disk changes. Schema inputs remain disk-backed; saving an open schema must still invalidate its consumers even if document indexing favors its open buffer. |
| `darkmatter/lib/src/markdown/schemas/simplified/types.rs:401` | `Constraint` has no `no-shell-expansion` variant. Existing schema shapes carry object constraints, and property atoms distinguish item and array constraints. |
| `claudine/lib/src/composition/lifecycle/mod.rs:110` | Deferred lifecycle expressions are currently raw JSON values retained through excluded top-level keys, not a typed deferred-value object retaining schema restrictions. |
| `darkmatter/lib/src/markdown/compose/schema_validation.rs:235` | Deferring the schema verdict can suppress preparation errors. New mandatory feature checks must not disappear merely because lifecycle values or ordinary schema verdicts are deferred. |
| `claudine/gen/Cargo.toml:8` | The generator explicitly forbids dependencies on Claudine library/CLI and already depends on Darkmatter. It is a suitable cycle-free location for schema generation. |
| `claudine/gen/src/main.rs:1` | Existing generator supports deterministic generation/check workflows; the CLI invokes its binary rather than linking it. |
| `claudine/justfile:116` | Normal package test/lint workflows include `claudine-gen`; schema drift can join those gates without a new CI cell. |
| `darkmatter/lib/src/markdown/schemas/mod.rs:135` | Darkmatter embeds its baseline YAML with `include_str!` and parses it; this is not an existing schema generation pipeline. |

## Recommendations to discuss serially

### Shared declarative bindings

Prefer generic binding metadata associated with the same schema extension
source as lifecycle property schemas. Darkmatter parses this payload into a
shared descriptor catalog; Claudine generates runtime declarations from it and
supplies actual eager/lazy values. DMLS dynamically loads the same source.
Descriptors should express root name, static type/documentation, schema
location or opaque scope association, and `available`, `unavailable(reason)`,
or `execution-dependent` availability. They contain no providers or executable
predicates. Claudine maps its event identity to these declarations; Darkmatter
knows no lifecycle event meanings.

Reasonable alternatives are metadata in the schema envelope, a referenced
companion descriptor file, or descriptors nested in trigger payloads. Prefer
schema-associated metadata over trigger-only declarations: mandatory runtime
bindings must exist independently of optional editor activation. Whether the
metadata is inline or referenced can be decided with the generated bundle
format. Avoid inventing another kinded document category without a ruling.

### Generation and distribution

Prefer an explicit schema generation/check command in the existing
`claudine-gen`, producing a checked-in runtime artifact beneath Claudine's
library schema module. The generator uses Darkmatter's parser/resolver and
does not depend on Claudine. Claudine consumes the artifact through Darkmatter
types. DMLS consumes the original repository YAML dynamically. Compilation
does not need a runnable Claudine binary or source-tree schema files.

Alternatives: a build script regenerates at every build, or a separate schema
generator crate. Build-time generation complicates bootstrap/build graphs and
hides reviewable artifacts; a new crate adds little while the existing
generator already has the necessary independent dependency direction.

The generated representation must preserve resolved schema semantics,
binding descriptors, restriction metadata, and source identity. A JSON Schema
blob alone is insufficient if it discards SimplifiedSchema-specific presence,
typing, or provenance metadata. Decide between generated Rust constructors
and a versioned compiled bundle consumed by Darkmatter; do not assume the
current SimplifiedSchema AST is serializable (its current derives are
`Debug`, `Clone`, and `PartialEq`). Include source-relative identities, never
developer-machine absolute paths. Keep generation deterministic and test both
byte drift and runtime/editor semantic equivalence.

### Restriction preservation

Prefer one Darkmatter-owned restriction projection accumulated from applicable
schema layers, independently of ordinary type-property replacement. The
effective prohibition at a location is the union of restrictions on its
ancestors and participating layers. Deferred evaluation carries a captured
restriction context and schema/source identity alongside its raw value.
Before any expansion stage, recompute the destination's applicable policy and
combine it with retained restrictions. Generated children inherit their
container policy, and moved deferred values must not lose their origin policy.

The consequential choice is how far origin restrictions follow moved values:
metadata on every JSON node is comprehensive but invasive; policy on prepared
evaluation units plus destination checks is smaller. Prefer the latter when
the movement audit proves all deferred units cross a typed boundary. Do not
claim this is settled until movement paths are inventoried. Ordinary literal
output is never reparsed merely to enforce this mechanism. Retain Claudine's
independent shell-action backstop before preflight.

### Activation and source ordering

Extend existing generic trigger matching with a passive workspace-fact
condition, evaluated from a host-supplied snapshot. A condition such as marker
existence should use an explicit root anchor and a source-relative portable
path, not a hardcoded Claudine filename or an expression executed during
matching. Darkmatter owns predicate grammar/evaluation; DMLS owns observing
filesystem facts and invalidating dependent documents. Root anchor and exact
syntax remain consequential decisions. No prototype is warranted by current
source evidence.

Automatic sources retain ancestry scope. Explicit `SCHEMA_DIR` is additive
and has workspace-wide possible scope, regardless of its source location.
Represent source origin and applicability scope separately so imports retain
their real origin without accidentally inheriting its directory's automatic
scope. Preserve existing automatic filename shadowing and payload merge
order; explicitly decide where the additional explicit source sits and how
duplicate automatic/explicit discovery is deduplicated. Restrictions must
survive whatever ordinary-property order is selected.

### Refresh and failure isolation

Prefer current-generation per-source load states and a dependency graph over
the current transactional registry. A source is loaded, inactive, absent, or
failed; a failed state retains source/cause and possible scope, not a usable
last-good definition. Parse activation before requiring its payload. An active
rule's broken payload/import/descriptor suspends dependent checks; a failed
activation rule makes applicability uncertain across its possible scope.
An intentionally removed optional root is different from a missing dependency
still referenced by an active definition.

Diagnostics, hover, and completion must consume the same current-generation
availability mask. When a failed rule's contribution is unknown, schema-aware
claims it might override are not demonstrably independent; withhold those
claims rather than treating the baseline as proof of a final effective type.
Independent Markdown syntax, expression parsing, and unaffected definitions
remain available. Successful refresh rebuilds affected state and clears the
failure and dependent suppression together.

Track both successful and missing dependencies, candidate schema directories,
and marker facts so create/update/delete can recover failures or discover new
directories. Extend existing watcher/rescan machinery; do not require eager
whole-workspace scanning. Coalesce changes, use immutable generation snapshots,
and discard obsolete refresh results. Record qualitative responsiveness
observations without numerical acceptance thresholds.

## Migration and verification considerations

- Apply the specification's exact schema-file inventory. Preserve
  `claudine/schemas/review.yaml`; empty/draft files in `claudine/docs/schemas`
  must not replace populated sources. `err.yaml` currently references the empty
  placeholder; repair its authoritative source rather than embedding a broken
  dependency.
- Cover parser constraints, lowering metadata, restriction inheritance, schema
  merge behavior, excluded lifecycle values, deferred execution, generated
  values, and typed error projection through the common Darkmatter authority.
- Exercise startup and successful/failed refresh through actual DMLS protocol
  paths, checking diagnostics and completion/hover together. Include uncertain
  applicability, sibling isolation, external explicit sources, missing imported
  files created later, marker changes, and newly created schema directories.
- Preserve portable paths, deterministic filename ordering, case-fold collision
  behavior, and watcher fallback across macOS, Linux, native Windows, and WSL2.
- Drift checking belongs to existing generator/package gates. No execution or
  responsiveness tests ran during this document-only investigation.

## Remaining blockers for this investigation

Human decisions are still needed for descriptor payload, generated artifact
format, deferred/moved policy retention, activation grammar/root anchor, explicit
source precedence, and failure-state granularity. The trigger-kind mismatch
needs an explicit scope/compatibility ruling. A trustworthy refreshed graph is
still required before reporting current impact findings. None of these open
items should be delegated to the future implementation plan.
