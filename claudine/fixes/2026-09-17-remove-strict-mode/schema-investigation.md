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

## D5 follow-up: deferred, generated, and moved values

Investigation dated 2026-09-18. D1–D4 were reported confirmed by the design
orchestrator: additive resolution, immutable binding declarations with runtime
sessions at existing lookup-operation boundaries, preservation of current
`current.ctx`/`current.env` snapshots, and shared immutable prepared source.
The recommendations below address D5 only and remain unconfirmed.

Graph-first queries used the exact absolute worktree path rather than a
potentially unstable alias. Queries for movement flows and contexts for
`resolve_typed_value`, `commit_proxy`, `compose_step`, and `DocumentOverlay`
provided useful source locations. They do not establish clean current impact:
the previously reported graph inconsistency has not been independently cleared,
and some responses were truncated. All findings below were checked in source.

### Concrete movement boundaries

| Boundary | Current source evidence | Required policy treatment |
| --- | --- | --- |
| Deferred lifecycle extraction | `claudine/lib/src/composition/prepare.rs:238` excludes lifecycle keys from normal composition; `:482` extracts effective frontmatter; `:694` parses lifecycle configuration from those raw values. | Capture restrictions and source location when prepared source is extracted. Exclusion from evaluation must not exclude passive feature checks or discard restriction provenance. |
| Lifecycle shell preflight | `claudine/lib/src/composition/prepare.rs:707` resolves shell commands in parsed lifecycle structures and stamps approved bytes back. | Policy must accompany prepared and replaced command content; replacement is not permission to lose origin restrictions. Preserve byte parity and the runtime initialization backstop. |
| `proxy.with` evaluation | `claudine/lib/src/composition/lifecycle/executor.rs:1739` captures fallback context once; `:1771` resolves each mapping entry; `:1801` recursively resolves arrays/maps; `:1843` resolves scalar expressions. | Validate/evaluate using source-location restrictions before committing any handoff. Return complete typed values with stage/provenance metadata where subsequent existing composition stages can consume them. Do not add a new expression pass. |
| Proxy transport and refresh | `claudine/lib/src/composition/coordinator/handoff.rs:145` stores the evaluated overlay as `IndexMap<String, Value>`; `coordinator/document.rs:23` stores the immutable document overlay; `:104` adopts it into the prepared target. Existing coordinator tests at `tests.rs:548` preserve the overlay across refresh. | Keep relevant metadata beside immutable overlay values across target adoption and retry/resume refresh. Recompute target-location restrictions and union with retained origin restrictions for any remaining executable stage. Preserve value redaction. |
| Batched lifecycle `set` | `lifecycle/executor.rs:1457` resolves values against one pre-write snapshot before `RuntimeState::set_batch`; `runtime_state.rs:71` stores mutations and outputs as JSON; `:143` validates all keys and atomically commits a clone. | Values and metadata must commit atomically together. A rejected batch publishes neither. Snapshot cloning and parallel-task state cloning must carry both. |
| Runtime re-entry layering | `runtime_state.rs:235` merges user setters, runtime mutations, outputs, then reserved overlay into plain JSON. `prepare.rs:245` passes resulting overrides into Darkmatter. | Layering must move a winning value's provenance with that value and discard overwritten value provenance. A single policy bit on the whole merged object would incorrectly restrict unrelated siblings. |
| Sequence params and overrides | `sequence/task/mod.rs:810` resolves each parameter; `:831` layers params/setters/mutations/overlay; `:863` resolves authored values through `SubtreeCompose`. `claudine/cli/src/commands/wrap/sequence/jit.rs:89` folds step overrides and `:148` prepares the target. | Preserve policy on transferred values still eligible for a normal later stage; apply fresh destination policy at the target. Step/runtime data must not become newly executable merely because it contains braces. |
| Schema layering | `darkmatter/lib/src/markdown/schemas/resolve.rs:1729` merges schemas with whole-property replacement; `schemas/mod.rs:733` retains effective schema and origin information. | Accumulate restrictions independently of replacement, including ancestor container restrictions. Effective type replacement does not grant permission to remove restrictions. |

The inspected paths primarily clone and merge JSON in memory; they do not
already provide a typed feature-policy carrier. No claim is made that every
JSON serialization site in the monorepo was audited. The design must close the
listed crossings rather than rely on an unverified belief that JSON is always
terminal data.

### Recommended alternative: prepared source plus sparse value-tree metadata

Use D4's immutable prepared representation for executable source. Associate
each prepared unit with a Darkmatter-owned policy context: source/schema
identity, the applicable restriction set, and the existing evaluation-stage
state. When values cross an overlay, mutation, or deferred subtree boundary,
carry a value-tree envelope with sparse subtree metadata. Ordinary JSON nodes
stay ordinary JSON; metadata records only boundaries where policy or
evaluation state differs from the inherited context.

Illustrative responsibilities, not a final public signature:

```rust
struct CompositionValue {
    value: serde_json::Value,
    provenance: ValueProvenance,
}

struct ValueProvenance {
    // Sparse subtree records; paths are segments, not dotted strings.
    entries: Vec<SubtreeProvenance>,
}

struct SubtreeProvenance {
    path: ValuePath,
    source: SourceIdentity,
    restrictions: FeatureRestrictions,
    evaluation: EvaluationStageState,
}
```

Names are illustrative. Do not add a generic feature catalog for this change:
the restriction currently required is only `no-shell-expansion`. Use segment
paths or escaped JSON pointers so dots in property names and array indices
cannot alias. Darkmatter owns the operations that select a subtree, remap it
into a destination, merge layers, and propagate stage/policy provenance;
Claudine supplies lifecycle and action identities but does not reconstruct
those mechanics with its own expression traversal.

For executable content at destination `d`, the governing prohibition is the
union of retained source restrictions, applicable destination/ancestor schema
restrictions, and any enclosing execution prohibition. The initialization
execution backstop remains independent of this data. A newly generated child
inherits the generating unit's applicable context plus its destination policy.
Moving one subtree moves only that subtree's records. Replacing one value does
not transfer restrictions from discarded content into unrelated new content;
schema-location restrictions still apply independently to the replacement.

The carrier lives as long as the value can participate in a remaining normal
composition stage: across deferred lifecycle execution, a proxy overlay's
document lifetime, and runtime mutation snapshots across sequence/loop/retry
preparations. Immutable source identities may be retained without retaining a
lazy binding session. D2's session lifetime is unchanged. A new preparation
gets a fresh destination schema generation; it combines that generation's
restrictions with retained source restrictions rather than treating an old
generation number as authorization.

Construction from authored input, fully evaluated data, and moved/deferred
input must be distinct operations. The carrier must not silently serialize as
bare `Value` and then be accepted as equivalent executable input. The listed
in-memory paths should keep it intact. If a required transfer actually crosses
serialization, encode its provenance in an internal envelope/sidecar with the
value or finish the allowed stages before exporting plain data. Generic
presentation/persistence APIs may export plain JSON intentionally; export
does not preserve an invocation's executable provenance or authorize automatic
re-entry as deferred source. No new persistent public format is recommended
without evidence that this feature needs one.

### Literal data and existing evaluation stages

`resolve_typed_value` currently performs a string-follow-up operation and then
rejects surviving spans. The specification removes the rejection and places
ordinary interpolation mechanics in Darkmatter. Metadata must record what
Darkmatter actually finished, rather than deducing completion from text that
contains `{{` or `$(`. Escaped braces and expression-produced template text
remain valid literal data. A complete value is never reparsed merely to
propagate restrictions or because it moved to another JSON property.

This is not a blanket promise to skip existing ordinary composition stages.
If a value is intentionally still pending a stage that Darkmatter already
performs, that stage runs at its existing timing with origin-plus-destination
restrictions. The prepared representation must distinguish that state from
completed output. In particular, an existing shell expansion stage must check
policy before executing newly materialized shell syntax; it must not be
invented merely because a completed lifecycle result contains shell-looking
text. No extra evaluation pass, blanket brace ban, eager lazy-provider capture,
or per-document memoization is introduced.

### Alternative: provenance on every internal value node

A complete annotated value tree gives every scalar/container a source,
restriction set, and evaluation state. Selection and movement naturally carry
metadata, making accidental loss harder; heterogeneous generated containers
are straightforward. It is a reasonable choice if the existing value pipeline
already needs broad provenance beyond this fix.

Its cost here is substantial: all `Value`/frontmatter operations, effect
bridges, mutation cells, and overlay interfaces must accommodate another tree
representation, with conversion adapters at most boundaries. Unrelated plain
data consumers pay complexity. It still needs explicit serialization rules and
cannot determine whether string output is executable by inspecting braces.

Prefer sparse subtree metadata because restrictions inherit through containers
and the concrete movements are concentrated at prepared-source and overlay
boundaries. Simplicity is a strong factor, but not a reason to retain bare JSON
at a boundary that can resume evaluation. A carrier only on the original
prepared expression, with all output metadata discarded, is not a sufficient
third alternative for the current re-entry paths.

### D5 verification implications

- Move a restricted deferred subtree into an unrestricted location and retain
  the prohibition; move unrestricted executable content into a restricted
  destination and acquire it. Include arrays and same-named nested keys.
- Preserve unrelated sibling behavior when one overlaid subtree is restricted;
  preserve non-relaxation under descendant schemas and whole-property overrides.
- Prove a generated value is checked immediately before its existing expansion
  stage and no prohibited shell runs, including initialization recovery routes.
- Exercise proxy refresh, batched set failure/commit, runtime snapshot cloning,
  sequence parameter layering, and deferred lifecycle extraction with provenance.
- Prove resolved whole values and all supported literal escapes survive those
  transfers without added interpolation, brace rejection, or type loss.
- If any covered transfer serializes, test value-plus-metadata round trips and
  reject the accidental loss of provenance at that executable boundary.

The unresolved technical ruling is carrier granularity and transfer discipline.
This investigation recommends the sparse envelope; it does not record agreement
or modify the functional specification.

## D6 follow-up: declarative binding catalog

Investigation dated 2026-09-18. D5's sparse policy/stage provenance was reported
confirmed by the orchestrator. This section proposes the next catalog decision;
its syntax and recommendations are not confirmed.

### Source evidence and limitations

Graph-first queries used the absolute worktree and returned lifecycle injection,
standalone-envelope, and group-variable locations. Context for
`lifecycle_injected_globals` and `resolve_group_variables` reported an index
three commits behind HEAD; a broader query still contained unrelated symbol
associations. No current risk assessment is claimed. Relevant source was read.

- `darkmatter/lib/src/markdown/schemas/simplified/standalone.rs:143` accepts
  tagged documents with exactly `kind` and `types`; pure documents have exactly
  one `$schema` key. Neither currently permits binding metadata. Extending the
  envelope must change the shared parser/source mapping, not merely a loader.
- `StandaloneSchemaDocument` currently retains a declaration, path, structural
  source map, and suggestion lints, but no extra descriptor payload.
- `darkmatter/lib/src/markdown/schemas/resolve.rs:1286` imports named types by
  returning `shape.properties`. Descriptor metadata would be lost if carried
  only on a parsed envelope and then discarded during resolution.
- `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs:35` defines
  a small function-parameter `DataType`; it intentionally excludes schema
  unions, literals, and other schema types. Function descriptors contain
  static strings and slices and are not a suitable dynamic binding-type wire
  format. Use shared schema property/type definitions for globals instead.
- `claudine/lib/src/composition/lifecycle/validate.rs:707` allows `err` in
  error-capable events and rejects it elsewhere; context documentation names
  `blocked`, `failure`, and optional-error `finalize`. The current constructor
  at `context.rs:566` only injects `err` when an actual error exists.
- `lifecycle/executor.rs:813` injects `group` only when the host has group
  variables. `sequence/task/group.rs:434` resolves group variables once at
  group start; `:475` additionally projects them into member frontmatter and
  the member overlay. Editor analysis of an independently opened prompt cannot
  infer whether it will later be invoked by a group.
- Current `darkmatter/docs/schemas/err.yaml` contains `$schema`, `name`, and
  `description` beside tagged `types`, violating the actual parser's allowed
  keys. Its placeholder dependency is already identified in the specification.
  Correct this artifact during the approved schema migration; do not copy it
  into generated data and assume it is valid.

### Recommended alternative: metadata in the schema envelope

Extend the existing `kind: schema` envelope with a `bindings` payload. Keep
`types`' existing interpretation; do not introduce a new kind or silently
change whole-file/named-import semantics. Claudine's whole-document schema can
use tagged `types` for the same root-property mapping its current pure
`$schema` contains. Helper type files retain their existing role. Runtime and
editor loaders both receive a resolved schema-definition bundle carrying the
schema and binding catalog together.

Use one catalog of named globals, an explicit finite table of opaque scope
IDs, and use-site document-location assignments. Each global has a shared
schema type, documentation, and an availability declaration for each relevant
scope (a default plus explicit scope overrides is sufficient). Availability
is one of `available`, `unavailable` with a stable reason code and explanatory
metadata, or `execution-dependent` with a stable reason for the runtime check.
No expressions, callbacks, filesystem conditions, or event-name semantics are
part of that grammar.

Illustrative excerpt, intentionally incomplete rather than a proposed final
catalog (the named `error-context` type would be supplied by the migrated type
source):

```yaml
kind: schema
types:
    initialize: lifecycle-event(no-shell-expansion)@./claudine-types.yaml
    failure: lifecycle-event@./claudine-types.yaml
bindings:
    globals:
        err:
            type: error-context@./err.yaml
            description: The active lifecycle failure.
            availability:
                default:
                    state: execution-dependent
                    reason:
                        code: claudine.error_presence
                        description: Availability depends on the active failure.
                scopes:
                    lifecycle.initialize:
                        state: unavailable
                        reason:
                            code: claudine.event_has_no_error
                            description: This event cannot carry an error.
    scopes:
        lifecycle.initialize: {}
        lifecycle.failure: {}
    locations:
        /initialize: lifecycle.initialize
        /failure: lifecycle.failure
```

Scope IDs are opaque to Darkmatter; their readable spelling is Claudine data.
Claudine selects a scope from the generated catalog for an execution boundary.
Darkmatter uses the location table to select that same scope during passive
document checks. Location entries identify instance-path subtrees, including
their descendants, with longest explicit ancestor selection. The catalog must
reject ambiguous duplicate assignments and unknown scope IDs. A document root
default supplies the scope outside more-specific assignments. Do not use
dotted path strings: literal keys and array indices must remain unambiguous.

The final syntax needs to cover location assignments within reusable imported
object/array schemas. Prefer assigning a scope at a schema use site, propagated
structurally to matching instance descendants, rather than enumerating every
array index or building a second path-pattern language. In the example, the
root property `initialize` determines the scope; importing the reusable
`lifecycle-event` type does not inherently mean initialization. A shared event
type used under `failure`, `start`, task setup, and task teardown keeps its
shape while the consuming location selects its scope. Exact source syntax for
such use-site annotations remains a detail to settle with D6, not an implicit
permission to add new expression grammar.

`err` is definitely unavailable for the existing non-error event scopes.
Error-capable scopes allow passive references but use runtime availability
where an error may be absent. `group` is execution-dependent whenever the
document alone cannot prove invocation membership; host-known non-group
preparation may select a definitely unavailable scope. DMLS must not infer
group availability from document properties named `group` or from filesystem
placement. `timing` and `current` follow actual host availability and existing
capture timing. Under D3 this catalog describes `current.ctx` and
`current.env`; it must not introduce the future `current_env` root.

The runtime supplies eager/lazy values against these descriptors and resolves
execution-dependent entries to available/unavailable without reparsing policy.
Available `null` stays available. Runtime construction checks registrations,
types/association invariants, and missing required provider associations
through shared Darkmatter validation. The enum of states does not itself
execute providers. Generation may provide typed scope/global identifiers to
prevent Rust string spelling drift; it must not create a second manually
authored policy table.

Loading a schema definition activates its associated catalog only when that
definition applies. Importing a helper named type resolves its type and origin;
it must not accidentally activate unrelated global catalogs belonging to the
helper's entire file. Bindings attached to the selected schema definition are
explicitly retained in the resolved bundle. Type references resolve relative
to the declaring catalog file through the same source-aware file mechanism as
schema imports. Missing imports and invalid descriptors are typed required
dependency failures of an active definition, including source span and cause.
The mandatory runtime bundle bypasses optional trigger activation entirely.

### Alternative: referenced companion catalog

Keep the schema and catalog in separate YAML files, with an explicit reference
from the schema envelope. The companion is parsed as a generic binding payload;
it needs no new `kind`. Runtime generation and DMLS follow the same reference
and dependency identity. The companion may be useful if several independent
schemas genuinely share the same large catalog or separate ownership.

Benefits: smaller schema documents, catalog reuse, and focused diffs. Costs:
another required file/import edge, another missing-dependency failure mode,
more origin bookkeeping, and the same shared schema-envelope extension needed
to connect it. It does not avoid defining the catalog grammar. Prefer inline
metadata initially because the catalog is small and owned beside Claudine's
schema; add a reference form only if demonstrated reuse justifies it.

Trigger-only metadata is not a viable equivalent alternative: Claudine must
enforce the same catalog even when editor activation is absent or disabled.
Embedding a catalog only in Rust or teaching DMLS Claudine event names would
likewise violate the confirmed ownership/source contract.

### Open details before calling D6 complete

- Choose inline metadata versus a required companion reference.
- Settle use-site scope annotation syntax for imported object/array types;
  the simple location table above demonstrates the concept but alone does not
  specify dynamic array descendants or imported nested layouts.
- Define descriptor conflict behavior when multiple applicable schemas claim
  the same global. Silent replacement can erase reservation/availability; an
  explicit duplicate/conflicting declaration error is the simplest initial
  rule, while identical declarations can be deduplicated by source identity.
- Finish the complete host scope/provider matrix, especially definite
  early-shell unavailability and execution-dependent group/error membership.
- For generation, the existing schema AST and function descriptor structures
  do not derive a general serialization contract. A generated runtime artifact
  requires a deliberate resolved-bundle representation rather than assuming
  JSON Schema or current static function descriptors contain everything.

No prototype is needed: source inspection establishes the present parser and
runtime seams. This follow-up changes no implementation or functional scope.

## D9 follow-up: schema generation and embedded artifacts

Investigation dated 2026-09-18. D6 inline binding metadata and subsequent
scope/availability decisions are confirmed in the current design; this section
does not replace them with earlier provisional examples. D9 remains unconfirmed.

### Verified integration points

- `claudine/gen/Cargo.toml:8` forbids depending on Claudine library/CLI and
  already includes Darkmatter and biscuit-file. The generator can use new shared
  schema APIs without introducing a reverse dependency or another crate.
- `claudine/gen/src/main.rs:39` has explicit generation/check subcommands;
  generation supports report-only and explicit write behavior. Schema commands
  can be their own family without coupling regeneration to provider research.
- `claudine/gen/src/emit/mod.rs:1` emits checked-in Rust source from validated
  data. Formatting is deliberately deterministic and hand-rolled; no formatter
  invocation is part of the emitter contract.
- `claudine/gen/tests/drift.rs:1` shares production generation/check functions
  with CLI checks. The existing tests compare generated bytes against committed
  artifacts. They are a direct model for schema drift coverage.
- `claudine/justfile:113` includes the generator in normal `just test`; `:219`
  includes it in lint. No new CI cell or build dependency is necessary.
- `darkmatter/lib/src/markdown/schemas/simplified/types.rs:22` and
  `resolve.rs:52` expose semantic schema types without Serde derives. Rust
  constructors and a compiled blob both require deliberate new output support;
  there is no existing general schema serialization format to reuse.
- `darkmatter/lib/src/markdown/schemas/mod.rs:733` shows `EffectiveSchema`
  includes process-local validators and captured file-resolution context.
  Neither belongs in generated definitions. Origin records currently contain
  paths, so generation must normalize source identity explicitly.

The graph-first generation query used the absolute worktree path and returned
empty processes plus visibly corrupted unrelated paths. The findings above
come from current source inspection after that failed query. No current impact
claim is made.

### Recommended option: checked-in Rust constructors

Extend the existing generator with `schemas generate` and `schemas check`.
Emit `claudine/lib/src/composition/schema/generated.rs`, with a generated-file
header and a function constructing a Darkmatter-owned semantic schema-definition
bundle. The artifact contains resolved schema definitions and the associated
binding catalog, scopes, restrictions, and source-relative origin records.
It does not contain runtime binding providers, evaluator sessions, filesystem
observations, compiled validators, or developer-machine paths.

The bundle should be the same semantic product the disk-backed resolver hands
to the shared schema engine. Preserve complete schema semantics: typed literal
values, null/presence information, property/root unions, pattern dictionaries,
constraints, authored declaration ordering, type information, and declaration
provenance. Preserve D6's inline bindings and the agreed scope declarations.
Named imports are resolved during generation; runtime loading the mandatory
bundle must not read the repository schema tree. Shared Darkmatter lowering
derives JSON Schema, feature-policy projections, and process-local validators
from these definitions. Do not maintain an independently generated policy
interpretation in Claudine.

Claudine's hand-written schema module caches the immutable definition bundle
using the existing once-initialization pattern and supplies it as mandatory
runtime input to Darkmatter. Runtime values/providers remain constructed at
the confirmed D2/D3 boundaries. DMLS continues loading original repository YAML
through Darkmatter and never links or embeds this artifact. Equivalent resolved
definitions produce equivalent runtime/editor semantics.

Proposed developer commands, not commands run during this investigation:

```sh
cargo run -p claudine-gen -- --area claudine schemas generate --yes
cargo run -p claudine-gen -- --area claudine schemas check
```

Package-area convenience recipes may be `just schemas-generate` and
`just schemas-check`, calling the same functions/commands. A generator L1 drift
test calls `check_schemas` and therefore runs under existing `just test`.
`just test-gen` should include it automatically through the generator suite.
Do not attach schema generation to provider-only regeneration or regenerate
silently in `build.rs`.

Generation inputs are explicit runtime entry definitions plus their transitive
schema/type/binding dependencies. Enumeration and output ordering are stable.
The entry-point inventory is loader configuration, not a second hand-authored
schema. The mandatory runtime schema must not merge every file in
`claudine/schemas` indiscriminately: `review.yaml` remains an independently
applicable schema. Validate the shipped schema corpus separately, including
files that are not mandatory runtime roots. Record all actual generation
dependencies so a changed imported definition cannot evade drift detection.

Source origins use logical package/repository-relative identities and portable
separators, plus retained spans where available. An embedded origin is
diagnostic identity, not a host filesystem path to probe. DMLS retains real
disk origins for dynamic imports. Equal declarations should compare after
normalizing this transport difference, without dropping origin attribution.
No generated timestamps or ambient machine state enter output bytes.

Benefits: the generated artifact is compiler-checked, follows an established
repository pattern, and introduces no new serialized compatibility protocol.
Costs: the emitter must cover the schema-definition variants exhaustively and
generated constructors are more verbose than compiled data. Keep that emitter
mechanical: it prints shared parsed values, never reimplements grammar,
matching, restriction inheritance, or type resolution. Ordinary Rust source
API changes and generator drift checks keep constructor output synchronized.

### Alternative: embedded compiled data bundle

The same generator could emit deterministic
`claudine/lib/src/composition/schema/generated.json`, embedded with
`include_bytes!` and decoded by Darkmatter. Darkmatter must own the codec and
semantic validation, since Claudine and DMLS cannot independently reconstruct
schema meaning. Store the same resolved semantic definitions, not merely
lowered JSON Schema and not an `EffectiveSchema` object. Validators are built
after decoding.

Benefits: simpler emission, compact data, and easy round-trip tests. Costs:
new serialization coverage over the complete semantic representation, runtime
decode failures, and a format identity/version check so mismatched generated
data fails explicitly. This can remain an internal build artifact with no
backward compatibility or migration machinery; there is no justification for
designing a general portable wire protocol. It is reasonable if the shared
semantic bundle soon needs serialization for another concrete consumer.

Either option satisfies generated runtime definitions from authoritative YAML.
Prefer Rust constructors here when minimizing new formats is the primary
simplicity criterion; prefer the data bundle if mechanical emitter maintenance
proves more costly. Source inspection does not establish a need for a spike.

### Source bundle and build-time generation

A generated bundle of original YAML strings is technically another embedding
strategy, but it retains runtime YAML parsing and requires embedded import
resolution/source tracking. A source-only `include_str!` change is not the
specified generation pipeline. A real source-bundle generator would still need
validation, deterministic dependency bundling, and runtime/editor equivalence
checks, providing less benefit than either resolved option above.

A build script could generate artifacts into `OUT_DIR`, but invoking a
Darkmatter-dependent schema generator from Claudine's build dependencies
enlarges the build graph and makes output less directly reviewable. Explicit
generation plus checked-in artifacts preserves the established independent
bootstrap tool: broken Claudine generated code cannot stop the generator from
building and repairing it.

### Verification contract

Use the same production resolver for generation and disk-backed editor schemas.
Check deterministic regenerated bytes and compare the generated semantic bundle
against the YAML-resolved bundle after source-origin normalization. Exercise
actual validation, descriptor classification, and restriction inheritance
through both forms; byte equality alone cannot prove that the emitter retained
every field. Ensure mandatory runtime behavior works without the source-tree
schema directory and remains independent of editor activation. Unknown variants
or invalid bindings fail generation; malformed generated definitions fail typed
initialization rather than silently dropping policy. No new CI matrix cell,
new public artifact compatibility promise, formatter run, or implementation
change is proposed by this investigation.

## D11 follow-up: filesystem facts in activation rules

Investigation dated 2026-09-18. D10 confirms `schema-trigger` as the canonical
kind and structured rejection of the obsolete spelling. Examples below use
that confirmed spelling. D11 syntax and anchor recommendations remain open.

### Existing grammar and boundaries

Graph-first queries for trigger matching/discovery returned empty processes and
unrelated symbol associations; current source was inspected directly afterward.
No reliable current graph-impact claim is made.

`triggers/grammar.rs:32` explicitly reserves future non-property predicates to
dollar-prefixed names. `MatchExpr` already supports `all`, `any`, `none`,
`min-match`, frontmatter property tests, and `$path`. A `match` sequence is
outer OR; structural sibling keys are AND; mixing structural and property
conditions in one mapping is rejected (`grammar.rs:193`). `matcher.rs:58`
evaluates those nodes without I/O. `lint.rs` requires a positive presence/path
gate in every viable OR arm, preventing accidental universal activation.

`schemas/mod.rs:411` accepts explicit document and discovery-boundary paths;
the library forbids ambient-CWD discovery. Composition supplies its captured
boundary (`compose/schema_validation.rs:153`). The CLI schema validator uses a
captured repository root (`cli/src/commands/schema/validate.rs:145`). DMLS
currently selects the nearest containing opened workspace folder
(`dmls/src/overlay/schema.rs:941`); its comment about repository narrowing is
not implemented. Root selection and marker anchors therefore must be specified
separately instead of inferred from that stale comment.

Claudine configuration guidance/source references include repository
`.claudine/config.json` (`claudine/lib/src/config/`), but this investigation
does not select it as an activation marker. Presence of a particular Claudine
file remains data in an eventual consumer-owned trigger, never engine policy.

### Recommended grammar: one reserved match leaf

Add a single `$exists` predicate to the existing match tree:

```yaml
kind: schema-trigger
match:
    $exists:
        root: workspace
        path: .example/config.yaml
$schema: ./example.yaml
```

The path is one literal relative path, not an expression or glob. Both `root`
and `path` are explicit. Reject absolute paths, URI/reference shorthand, and
parent traversal; marker facts are anchored queries, not schema import search.
Use portable slash-separated authored paths and the existing shared
source-aware path authority when the host resolves them. The passive parser
only validates syntax and records the fact request; it does not resolve or
probe filesystem state.

Presence of one filesystem entry is sufficient; directory/file content is not
read and marker content does not execute. A precise final-entry/symlink policy
must be pinned in the host fact observer and cross-platform tests rather than
hidden behind `Path::exists`, which also discards I/O errors. The minimal
interpretation is final directory-entry presence, including a symlink entry,
without reading/following its final target. If the human instead wants regular
file existence, name the predicate `$file-exists` and specify that explicitly;
do not silently make the two meanings interchangeable.

Existing frontmatter-only rules are unchanged. Conjunction uses existing `all`:

```yaml
match:
    all:
        - $exists: { root: workspace, path: .example/config.yaml }
        - kind: literal(task; required)
```

OR arms retain their current meaning:

```yaml
match:
    - $exists: { root: workspace, path: .example/config.yaml }
    - kind: literal(task; required)
```

The latter deliberately means either condition, not marker AND all document
arms. Multiple marker requirements use ordinary `all`/`any`; there is no
marker-specific list shorthand or second condition language. The positive
`$exists` leaf counts as a gate for vacuity checking. `none` of marker presence
alone should retain the current prohibition on unguarded universal activation;
combine it with a positive document/path gate where needed.

### Anchor recommendation and exact scope

Separate these identities in the trigger's captured host context:

1. **Schema source origin:** actual file location; relative `$schema` and named
   imports stay relative to this origin, including external `SCHEMA_DIR`.
2. **Workspace anchor:** the nearest containing opened LSP workspace folder for
   this document. With multiple/nested opened folders, choose the most specific
   containing folder deterministically. A library/CLI host supplies its explicit
   equivalent root; the matcher never derives it from CWD.
3. **Applicability scope anchor:** for automatic discovery, the directory owning
   that `schemas/` directory; for explicit `SCHEMA_DIR`, the selected document's
   workspace anchor. This determines the meaning of `root: scope`.
4. **Discovery boundary:** the upper limit of the ancestor search, independently
   chosen according to repository/opened-tree discovery policy.

Permit exactly `root: workspace` and `root: scope`. Workspace-root markers can
enable rules across a workspace, but they never widen a nested automatic rule's
document scope. A package rule using `root: scope` checks that package's marker.
An explicit external source using `root: scope` checks the workspace, not the
external directory and not its containing package. The same source discovered
automatically and explicitly can therefore have distinct applicability/anchor
contexts even when its parsed bytes are shared.

For documents outside all opened workspaces, retain no automatic discovery.
For a non-repository opened tree, the opened root is the discovery boundary and
workspace anchor. For an opened repository root, both roots coincide. For an
opened subdirectory inside a repository, a consequential difference remains:
current DMLS stops at the opened folder, while R12 requires repository-root
schema discovery. Recommend allowing the captured repository root to be the
discovery boundary, while keeping `root: workspace` anchored to the actual
opened folder. An automatically discovered repository-level rule can use
`root: scope` to check the repository root. The checked document must still
belong to the opened workspace, and automatically discovered nested rules
cannot affect siblings. This root-discovery clarification needs agreement;
do not conflate the two roots under a misleading single `workspace` field.

### Pure evaluation and failure facts

Darkmatter parses the rule and enumerates marker requests. The host observes
those requests into a snapshot: `Present`, `Absent`, or a structured observation
failure. Successful nonexistence is false; permission/I/O failure is unknown,
not false. Darkmatter combines those facts with document match conditions and
returns matched/not-matched/undetermined plus dependent failures. For example,
`all(false, unknown)` is definitively false, while `all(true, unknown)` is
undetermined. The existing OR/combinator equivalents follow the same bounded
three-valued logic. A malformed rule still has unknown applicability across its
possible scope because no trustworthy parsed tree exists.

The snapshot is request/generation-scoped and contains no callable providers.
Filesystem observation performs metadata reads only; matching executes no
actions, shell expansion, lazy providers, content expressions, or network I/O.
Facts include missing entries so create/delete events can invalidate them.
Marker changes and ancestor directory creation/removal refresh dependent
documents through the agreed loader/watch strategy. A marker failure cannot
silently deactivate an active extension or retain stale dependent assistance.

### Alternative: separate top-level activation gate

An optional top-level `activate` (or `workspace`) payload could carry marker
facts, implicitly ANDed with the existing `match` document expression.
Benefits: clear visual separation between filesystem facts and document tests,
and no additional `MatchExpr` leaf. Costs: marker-only rules need optional
`match`, combined OR rules need another composition convention, and matching,
explanation, vacuity checking, dependency extraction, and errors now cross two
condition surfaces. It also expands the trigger envelope beyond the kind change.

Prefer the reserved leaf because the current grammar explicitly anticipates
this extension, existing boolean constructs express all required examples, and
`$` avoids collisions with ordinary unprefixed document property conditions.
No arbitrary expression language, marker-content parser, extra root aliases, or
hardcoded Claudine configuration filename is needed.
