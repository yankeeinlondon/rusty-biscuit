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
