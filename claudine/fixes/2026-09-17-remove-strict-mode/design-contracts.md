# Remove Strict Mode: Concrete Contracts

Technical completion draft, 2026-09-19. Companion to [design.md](design.md).
D1–D22 remain the human decision record. This document supplies reviewable API,
grammar, data-flow and recovery contracts under the request to complete design
steps 1–3. Names and signatures below describe the proposed implementation;
they are not claims that these APIs already exist.

The final discussion clarified host-owned globals (C3) and confirmed automatic
time-based activation refresh (C6). Neither remains a pending human ruling.
Markdown cannot declare globals or switch event bindings. Layering follows the
human's existing rules in
[Schema Layering](../../../darkmatter/docs/topics/schemas/authoring-schemas.md#schema-layering)
and D20; the proposed restriction to one root-union source is withdrawn.
Independent review is a separate step, assigned
by the human to another agent. Do not start replacement implementation planning
from this draft or the superseded plan.

## C1 — Additive lookup and checked runtime association

Use the existing `EvaluationLookup` trait and evaluator. The public contract is
the following additive shape; existing context accessors remain:

```rust
pub enum ResolvedBinding {
    Document { value: Option<ValueEnvelope> },
    Namespace { value: Option<ValueEnvelope> },
    Global { value: ValueEnvelope },
}

pub trait EvaluationLookup {
    fn get(&self, path: &str) -> Option<Value>;
    fn resolve(&self, path: &str) -> Result<ResolvedBinding, BindingError>;
    fn binding_view(&self) -> Option<&BindingView>;
    fn format_resolved(&self, path: &str, value: &Value) -> String;
    // Existing get_string and resolution/context descriptor methods remain.
}

impl BindingEnvironment {
    pub fn associate<'a>(
        view: Arc<BindingView>,
        document: &'a dyn EvaluationLookup,
        runtime: RuntimeBindings<'a>,
    ) -> Result<EvaluationSession<'a>, BindingConfigurationError>;
}

pub enum RuntimeBinding<'a> {
    Eager(ValueEnvelope),
    Lazy(&'a (dyn Fn() -> ValueEnvelope + Send + Sync)),
    Unavailable(UnavailabilityReason),
}
```

`resolve` defaults to wrapping `self.get(path)` as terminal ordinary data in
`Document`, `binding_view` to `None`, and `format_resolved` to existing
scalar/JSON interpolation formatting.
Lightweight ordinary-data implementations remain source-compatible. This
default is not a declaration of globals. Production namespace/document adapters
must enforce the contract before using it; it does not repair a legacy `get`
that itself falls back to context.

The evaluator always calls `resolve`. Both document absence and namespace-field
absence evaluate as null, but classification remains available to diagnostics.
Available-null is a Global envelope containing `Value::Null`; unavailable is a typed
`BindingError::Unavailable`, never an absent map entry. Once a root is classified
as global or namespace, missing descendants stay within that root. Exact `doc`
selects the effective document; explicit `doc.doc`, `doc.ctx`, and `doc.err`
remain ordinary properties. Preserve each namespace's existing whole-root value
semantics; this change does not invent an aggregate `ctx` where a context-only
adapter currently supplies only `ctx.*`.

Reserved namespaces are resolved first, registered globals second, document
properties last. Names exactly `doc`, `ctx`, or `env` cannot be registered.
Registration accepts a valid root identifier, not a dotted path. Duplicate
registration, unknown scope, unknown global, omitted declared global, or a
registration contradicting a definite declaration is a configuration error.
Association validates the complete registration map before evaluating an AST or
invoking a lazy provider. An execution-dependent declaration must become an
explicit available or unavailable entry at association. Definitely unavailable
entries must carry the declared structured reason; they cannot be rebound to
eager data. A declaration promising availability requires eager/lazy data.

The envelope-aware runtime map uses infallible providers. Existing
`InjectedGlobal::lazy(Fn() -> Value)` remains an adapter that wraps its result as
terminal data; an explicit envelope-aware constructor supports transferred
deferred values. No provider failure protocol or catch-unwind policy is
introduced. Each session caches a
global's root once, including a null value; field reads project from that cached
root. Cache lifetime is one direct expression operation or one recursive
subtree composition, not one string, action stack, event, or prepared document.
New operations get new sessions. Avoid holding the cache mutex while invoking
user code; do not translate poisoned synchronization state into missing data.
The session is not a cross-event provider store.

Existing `get_string` stays available to external callers. The evaluator and
interpolator format the already-resolved value through `format_resolved`; they
must not call `get_string` and perform a second lookup that could erase an error
or observe another value. EffectiveState and seed adapters retain name-coercion
behavior through that hook; forwarding wrappers forward it.

Internal structural expression results (path/index selection, object/array
construction and selected conditional branches) carry the envelope using C4's
operations. Arithmetic and string computation produce a completed value with
the executing unit's policy. The legacy public evaluator may project a terminal
result to JSON, but internal composition must use the envelope result so that
`{{ doc.deferred }}` does not lose the selected value's metadata. A transfer of
a still-deferred child preserves that child's remaining stages; completing the
selecting expression does not falsely complete the selected child.

`BindingView` is immutable and provider-free: namespace descriptors, declared
document shape, global root types/descriptions/availability, scope identity,
restriction policy and a semantic-bundle identity. It is shared by passive
validation and runtime association. Unknown bare properties have unknown static
type; a schema-declared absent property retains its declared type. A known
runtime null does not change the schema type catalog.

## C2 — Preparation and typed errors

```rust
pub enum AuthoredMode { Expression, InterpolatedValue, Subtree }

pub fn prepare_value(
    input: &ValueEnvelope,
    mode: AuthoredMode,
    context: &PreparationContext,
) -> Result<PreparedValue, PreparationError>;

pub fn validate_prepared(
    prepared: &PreparedValue,
    view: &BindingView,
) -> Vec<ValidationDiagnostic>;

pub fn evaluate_prepared(
    prepared: &PreparedValue,
    session: &EvaluationSession<'_>,
) -> Result<ValueEnvelope, EvaluationFailure>;
```

PreparationContext contains the immutable semantic schema/view, expected result
schema if any, source identities/spans, schema phase, and declared execution
policy. It contains no effect engine, runtime provider map, captured event
values, or ambient `ResolutionContext`. Preparation reuses existing interpolation
scanners and `SpannedExpr`; it records authored form, parsed ASTs, interpolation
span boundaries, referenced roots, explicit context requirements and function
capabilities. It does not interpolate, read environment/host facts, resolve
runtime file functions, or invoke lazy values.

All authored branches are parsed and checked for definite violations, including
unknown functions, unavailable scopes, and prohibited features. Runtime-dependent
values/availability are deferred. Ordinary eligible arithmetic/function errors
retain evaluator short-circuit semantics. Do not evaluate inactive branches to
discover errors. Runtime-generated spans inside the existing mixed-string pass
are parsed and checked by the same Darkmatter machinery immediately before
their evaluation; preparation cannot claim to have parsed text not yet produced.

PreparedValue stores its schema/binding/policy identity. Association/evaluation
rejects a mismatched identity with `PreparedContextMismatch`; the caller
reprepares from the retained envelope under the new view. Do not silently use
cached descriptors from an earlier schema generation. Runtime values and caches
are never part of that identity.

Whole-value interpolation evaluates once and preserves non-string JSON results.
Mixed strings retain the current bounded interpolation behavior inside one
operation. Completion of that operation is terminal for its interpolation stage,
even if the output contains braces. Do not add another parse/evaluate pass in
Claudine's message or typed-value handling. Expected result shape is checked by
Darkmatter against the supplied schema after evaluation and before an effect.

Extend existing error enums, retaining owned typed causes rather than creating
parallel string error systems:

| Error family | Structured fields and responsibility |
| --- | --- |
| Binding configuration | Code; root; scope ID; catalog/source span; reserved/duplicate/missing/contradictory registration details. |
| Unavailable binding | Root and requested path; opaque scope ID; structured reason code and parameters; authored span. |
| Feature violation | Feature ID, value path, authored span, originating policy sources and destination/execution restrictions. |
| Parse/evaluation/result failure | Existing typed parser/evaluator/schema cause with authored location and expected result context. |
| Schema source failure | Source ID, import/definition route, typed I/O/parser/resolution cause and possible applicability scope. |
| Activation failure | Predicate span, requested fact, observation/computation/configuration cause; no false-valued substitution. |
| Prepared mismatch | Prepared and current semantic identities; no execution attempted. |

UnavailabilityReason has a stable namespaced code, optional source span and
structured parameters; human-readable description comes from the catalog.
Darkmatter does not match on strings such as `initialize` or `outside-group`.

Claudine keeps `LifecycleExprError` as its owned contextual wrapper, with typed
Darkmatter arms. Remove undefined-variable and surviving-span variants when
their obsolete checks disappear. Result-shape failures are shared schema errors.
`CompositionError`, stack outcomes and proxy/recovery transport must retain an
owned/`Arc` cause inspectable through typed accessors and `Error::source`.
`LifecycleErrorInfo` JSON remains the authored `err` projection; it must not
replace the original cause. CLI presentation is the final formatting boundary.

## C3 — Binding grammar and complete lifecycle scopes

The D6 optional envelope describes the host application's binding catalog in
its external schema data. It is not Markdown syntax for defining globals.
The host schema associates known document properties with its event descriptors;
the host runtime selects the actual execution event. Illustrative excerpt (the
generated source must fill the full matrix below; paths assume a catalog in
`claudine/schemas`):

```yaml
kind: schema
bindings:
  globals:
    err:
      type: "err@../../darkmatter/schemas/partials/error.yaml"
      description: Error for the current lifecycle operation.
      availability: { state: unavailable, reason: claudine.outside-lifecycle }
    tracking:
      type: "tracking(required)@../../darkmatter/schemas/partials/tracking.yaml"
      description: Durations already captured by the caller.
      availability: { state: unavailable, reason: claudine.outside-lifecycle }
    current:
      type: "context(required)@../../darkmatter/schemas/partials/context-variables.yaml"
      description: Context captured for the current event.
      availability: { state: unavailable, reason: claudine.outside-lifecycle }
    group:
      type: "object(required)"
      description: Variables of the enclosing sequence group.
      availability: { state: execution-dependent }
  scopes:
    initialize:
      globals:
        err: { state: unavailable, reason: claudine.event-has-no-error }
        tracking: { state: available }
        current: { state: available }
    failure:
      globals:
        err: { state: available }
        tracking: { state: available }
        current: { state: available }
    sequence-shell-approval:
      globals:
        err: { state: unavailable, reason: claudine.preflight-unavailable }
        tracking: { state: unavailable, reason: claudine.preflight-unavailable }
        current: { state: unavailable, reason: claudine.preflight-unavailable }
        group: { state: unavailable, reason: claudine.preflight-unavailable }
  locations:
    - path: [initialize]
      scope: initialize
    - path: [failure]
      scope: failure
$schema:
  initialize: "lifecycle-event(no-shell-expansion)@./claudine-types.yaml"
  failure: "lifecycle-event@./claudine-types.yaml"
```

`type` uses the existing schema type/reference grammar, including imported named
types and property unions. The optional `lifecycle-error@this` atom permits
null through existing optional-property semantics; the three `@this` types must
be declared in the complete catalog's `types` section. Registration presence
is separate from type nullability. Scope IDs are
opaque to Darkmatter. A scope's missing root override inherits that catalog's
root default. Missing reason for `unavailable`, unknown fields/states/scopes,
duplicate IDs or undeclared override roots are configuration errors. Eager/lazy
is a runtime delivery choice and is not a fourth static availability state.

Catalog identity qualifies scope IDs. Repeated references to the same catalog
coalesce; independently declared globals with the same root must have equivalent
type and availability contracts or produce a structured binding conflict.
Property precedence does not resolve global-registration conflicts. Claudine's
typed construction of blocked/failure bindings requires a non-null error even
though the shared `err` type also serves nullable finalize/teardown scopes.

Markdown authors supply frontmatter properties, accessible directly or through
`doc`. They cannot register globals, provide global values, override availability,
or select another event scope through inline schemas or nested content. A
frontmatter property named `bindings` is ordinary document data, not a runtime
registration. An attempted global declaration in an inline schema is rejected
as unsupported configuration rather than installed.

Reusable type imports do not activate globals. Host-owned external catalog
metadata supplies passive editor descriptions, not runtime values or permission
to override mandatory host bindings. Runtime association always uses the host's
catalog and actual event. The proposed `binding-scope` constraint and generic
nested event-scope replacement mechanism are withdrawn. Nested conditions in an
event stack retain that event's bindings. The `locations` paths are typed
property/index paths in host metadata for passive checking; they do not dispatch
events. Contradictory host associations are configuration errors. Ancestor
feature restrictions and approval-phase prohibitions remain cumulative.

| Surface | `err` | `tracking` | `current` | `group` |
| --- | --- | --- | --- | --- |
| initialize | Unavailable: event-has-no-error | Available snapshot | Available snapshot | Lexical rule below |
| start | Unavailable: event-has-no-error | Available snapshot | Available snapshot | Lexical rule |
| success | Unavailable: event-has-no-error | Available snapshot | Available snapshot | Lexical rule |
| loop | Unavailable: event-has-no-error | Available snapshot | Available snapshot | Lexical rule; current loop callers are outside a group |
| blocked | Available required error | Available snapshot | Available snapshot | Lexical rule |
| failure | Available required error | Available snapshot | Available snapshot | Lexical rule |
| finalize | Available error or explicit null | Available snapshot | Available snapshot | Lexical rule |
| task setup | Unavailable: event-has-no-error | Owning execution snapshot | Owning execution snapshot | Lexical rule |
| task teardown | Available primary error or explicit null | Owning execution snapshot | Owning execution snapshot | Lexical rule |
| Early lifecycle shell approval | Unavailable: preflight-unavailable | Unavailable: preflight-unavailable | Unavailable: preflight-unavailable | Available only if already established and permitted by the existing approval context; otherwise unavailable |
| Sequence-wide shell approval, including referenced prompts and task stacks | Unavailable: preflight-unavailable | Unavailable: preflight-unavailable | Unavailable: preflight-unavailable | Unavailable: preflight-unavailable |
| Non-lifecycle location under the activated Claudine catalog | Unavailable: outside-lifecycle | Unavailable: outside-lifecycle | Unavailable: outside-lifecycle | Lexical rule |

Lexical rule: an established enclosing group supplies an available object,
including an empty object. Known absence is unavailable with `outside-group`.
A reusable task/prompt whose membership is unknown is execution-dependent during
passive checking and resolved explicitly at invocation. A group's own variable
definitions do not receive that new group's binding; any already established
outer scope follows the existing lexical execution context. Do not infer group
membership from a file's location. `doc.group` always selects document data.

`outputs` remains unavailable for sequence shell approval under its existing
policy; express it as an additional declared binding/restriction on that surface,
not a fifth lifecycle event global or a Darkmatter namespace. Loop `_loop_*`
bindings and hook metadata retain their separate caller-owned catalogs.

Existing timing measurements are supplied through `tracking`: `document_ms`,
`total_ms`, and `step_ms` retain their runtime values; absent measurements are
not fabricated. The initial schema permits an object without claiming field-level
constraints. `current` has the same shape as `ctx`, without a nested wrapper.
Capture happens at
the existing event boundary; laziness delays serialization only. Task setup and
teardown use their semantic scope even if a reused StackExecutionContext still
has another signal enum value. Production callers supply snapshots today;
snapshotless library/test contexts must supply the declared values or receive
D7's missing-registration error, not downgrade promised availability to null.

Preparing future lifecycle content selects its future event descriptors without
constructing runtime values. Actually evaluating a shell command for approval
selects the approval scope. Initialization's `no-shell-expansion` and existing
shell-action/bootstrap bans remain in force on applicable recovery routes;
recovery event dispatch is host orchestration, not Markdown scope switching.

## C4 — Provenance carrier and executable stages

```rust
pub enum PathSegment { Key(String), Index(usize) }
pub struct ValuePath(Vec<PathSegment>);
pub struct ValueEnvelope { /* private JSON + sparse metadata + origins */ }

impl ValueEnvelope {
    pub fn select(&self, path: &ValuePath) -> Result<Self, EnvelopeError>;
    pub fn replace(&mut self, path: &ValuePath, value: Self)
        -> Result<Option<Self>, EnvelopeError>;
    pub fn remove(&mut self, path: &ValuePath) -> Result<Option<Self>, EnvelopeError>;
    pub fn array(values: Vec<Self>) -> Self;
    pub fn object(values: IndexMap<String, Self>) -> Self;
    pub fn restrict(&mut self, path: &ValuePath, policy: RestrictionSet);
    pub fn snapshot(&self) -> Self;
    pub fn as_json(&self) -> &Value;
    pub fn into_terminal_json(self) -> Result<Value, EnvelopeError>;
}
```

Sparse records carry inherited feature prohibitions, source identity/spans,
file-origin references where applicable, and evaluation-stage state. Typed path
segments distinguish a key `a.b`, key `0`, and index 0. No JSON string prefix,
sentinel key, hidden authored frontmatter property, or mutable external sidecar
represents metadata.

Selection materializes all inherited ancestor restrictions at the selected
root and rebases descendant paths. Replacement removes old source metadata
under that destination and combines new origin policy with destination policy.
Return the prior envelope before applying new destination metadata to it.
Array/object reconstruction moves metadata with the selected values; array
indices are not permanent identities. Restriction application is monotonic
union; it never consumes stages. Failed operations leave their input unchanged.

Stage records identify the actual existing pipeline work, not an instruction to
run the full pipeline again: ordinary frontmatter interpolation, frontmatter
shell expansion where already supported, and deferred subtree interpolation
are distinct stage IDs with existing order and applicability. SubtreeCompose
does not gain a shell-expansion stage merely because a value contains shell
syntax. Body composition remains on its existing pipeline. Each deferred unit
records which stages remain, and completed output is explicitly terminal for
its completed stage. New authored input receives its stage plan from its source
boundary, never by scanning stored output for braces.

An operation evaluates a candidate, advances stages only in its successful
result, and leaves its retained input intact on failure. An active mixed-string
operation may run its existing bounded internal passes; completion ends those
passes permanently for that unit. Copying a terminal expression result through
set, proxy, snapshot or refresh must not schedule another interpolation pass.
Structural selection preserves provenance; this is not general taint tracking
through arbitrary arithmetic or string computations. Computed output belongs to
the executing authored unit's destination/execution policy and completion state.

Effective restrictions are the union of origin, destination and execution
restrictions. A pending prohibited stage fails before dispatch. The
initialization action runner remains a separate backstop even when no expression
expansion is involved. `as_json` is a borrowed data view, not an executable
transfer API. `into_terminal_json` rejects pending stages; restriction metadata
may be discarded only when no later internal executable consumer remains.
Already-completed output and provider/error/event data enter as terminal data.

Mutation APIs in Claudine store this carrier inside their existing atomic state.
See the complete [transfer inventory](migration-inventory.md#executable-value-transfer-inventory).
Do not add a persisted envelope format speculatively; any discovered resumable
serialization boundary must preserve versioned metadata before migration.

The provenance spike establishes the carrier's viability at `set_batch`, not
all these integrations. Its triple-brace negative control makes stage retention
mandatory: the first result `{{ secret }}` really executes if composed again.

## C5 — Shared semantic bundle and generation

### Unified global schema entry point

`darkmatter/schemas/darkmatter.yaml` is the single authoritative entry point
for global shapes. Its `$schema` declares `doc`, `ctx`, `current`, `env`, `err`,
and `tracking`, importing named definitions from `partials/`. Every global uses
the same schema parsing, resolution, validation, and DMLS projection machinery.
`ctx` and `current` reference the same context type. Bare `foo` resolves to
`doc.foo`; this shorthand belongs to expression resolution, not schema generation.
There is no `current_env` global.

The built-in document baseline is the resolved `doc` definition, not the entire
entry point. Document schema layering replaces properties within that definition
under the existing precedence rules. It cannot redefine global bindings. An
authored `doc.ctx` property remains independent of the host's `ctx` global.

Register this entry point explicitly as the global catalog. Exclude that
registered source from automatic document-schema contributions by resolved source
identity, including when discovered again through another path. Its `kind: schema`
envelope remains intact. Imported types-only partials never auto-apply. Do not
infer the global catalog role merely from property names or a directory scan.

Generation resolves the complete transitive import graph with the shared semantic
compiler and emits the global definitions, including constraints, descriptions,
and provenance. Runtime and DMLS consume equivalent definitions; neither rebuilds
the context shape by extracting `ctx` from the document baseline. Generated
artifacts must work without source YAML on the deployed filesystem. Keep the
Darkmatter artifact separate from Claudine's generated host policies and avoid
a generator/library dependency cycle.

Schema knowledge does not imply runtime availability. In this fix Claudine still
supplies lifecycle values and event availability through the host-binding API;
Darkmatter validates them and DMLS uses the same declarations without invoking
providers. Lifecycle execution remains in Claudine until the separate migration.
Expose event-captured context directly as `current.cwd`, for example, with no
`current.ctx` or `current.env` compatibility aliases. This shape correction does
not introduce reference-time freshness: lazy materialization retains the existing
event snapshot timing. `tracking` may initially be an unconstrained object;
field-level assistance requires a populated definition.

Acceptance checks must cover transitive import resolution, generated/loaded
schema parity for every global, identical `ctx`/`current` shapes, independent
`doc.ctx` and `ctx` values, document-only layering, exclusion of the registered
catalog from automatic document application, passive global hover/completion,
and event-specific availability. Verify `current.cwd` against the captured event
snapshot and reject the removed nested form under its schema.


The human-authored `err.yaml` now exports an `err` property and retains local
field types under `types`. Preserve both. Bare named types such as `code` are
requested shorthand for `code@this`; resolve them in the defining schema's
namespace through the existing named-import resolver. Built-in keywords retain
priority, explicit imports remain available, and unknown names remain errors.
The human also requested fixing constraint inheritance: remove the resolver's
stripping of top-level `required` and `generated`. Preserve the complete named
definition through local, explicit and cross-file references. Merge use-site
constraints with inherited constraints under the human-confirmed rule:
different constraints apply together, repeated bounds keep the stricter value,
and contradictory constraints produce a typed schema error with both source
locations. This does not replace D20's whole-property layering precedence.
Keep cycle detection and defining-file
origins unchanged. Verify required/generated behavior in resolved schema output,
runtime phase validation, generated bundles and editor assistance. See the
[local type reference contract](../../../darkmatter/docs/topics/schemas/local-type-references.md).
The parser, generated bundle and editor must agree on both spellings.

```rust
pub struct SemanticBundle {
    pub sources: Vec<SourceDefinition>,
    pub definitions: Vec<SchemaDefinition>,
    pub bindings: Vec<BindingCatalog>,
    pub triggers: Vec<ActivationDefinition>,
}

pub fn assemble_schema(
    contributions: &[SchemaContribution],
    document: &DocumentSchemaContext,
) -> Result<EffectiveSemanticSchema, SchemaAssemblyError>;
```

SourceDefinition contains a stable bundle-relative source ID, portable original
relative path, source-map spans and dependency IDs. A filesystem-loaded source
also has an anchored FileReference resolution context outside the portable
bundle. Generated sources use bundle-relative import IDs rather than absolute
build-machine paths. The schema-definition tree retains exported root arms,
named types, property/pattern-key/object constraints, descriptions, presence and
schema-phase behavior, binding use sites, restrictions and source provenance.
No compilation step discards the information needed for hover or completion.

Semantic identity covers definitions, scope/policy and dependency versions, not
runtime values. Use the repository hashing facilities when identity uses content
hashes: Darkmatter for Markdown and biscuit-hash xxHash for other schema content.
There is no independent hashing algorithm or hash-based source precedence.

The generator emits checked-in Rust constructors at
`claudine/lib/src/composition/schema/generated.rs` from `claudine/schemas`.
`claudine-gen` depends on Darkmatter, never Claudine. Normal builds consume the
artifact without source YAML on disk or a generation build script. Compare
generated and loaded bundles semantically, including metadata and origins, and
check deterministic artifact drift in existing generator tests. DMLS reads YAML
through the same parser and semantic compiler; it never links the Claudine
artifact. File-based imports and generated bundle references meet at the same
resolved definition interface.

EffectiveSemanticSchema is the single view for validation, known-shape queries,
hover, completion and binding preparation. Each contribution records definition
identity, tier, applicability, consuming scope, resolution origin, dependency
routes and same-tier ordering. Do not retain today's split between merged
trigger JSON validators and explicit-document-only simplified assistance.

## C6 — Activation syntax, facts and refresh

The canonical envelope accepts `kind`, optional `name`/`description`, required
nonempty `match`, and `$schema` plus optional `types`/`bindings` using the schema
envelope contracts. A trigger with no exported payload is invalid rather than a
silently ignored activation rule. Unknown keys are errors. `name` is display
metadata, not definition identity or a requirement to rename existing files.

`match` is a sequence whose entries are ANDed. An entry is either one predicate
mapping or an explicit `group:` mapping containing a nonempty list of predicate
mappings whose members are ORed. Each predicate mapping has exactly one
discriminator. Anonymous nested lists are not the group syntax.

The human confirmed that the kind rename is the grammar boundary: every
`kind: schema-trigger` uses this model; `kind: trigger-schema` is rejected with
migration guidance. Do not add an alias or heuristics to detect legacy OR lists.

```yaml
kind: schema-trigger
name: Task documents in a configured workspace
match:
  - expression: 'doc.kind == "task"'
  - file_exists: { base: workspace, path: './.example/config.yaml' }
  - group:
      - os: macos
      - os: linux
$schema: './task.yaml'
```

| Predicate | Payload and meaning |
| --- | --- |
| `expression` | Ordinary expression string using document data and activation-eligible functions. No template delimiters or shell expansion. |
| `document` | Existing pure match-expression mapping: property type guards, `$path`, `all`/`any`/`none`/`min-match`. Retains existing guard versus required-gate semantics and path glob matching. This explicit wrapper avoids collisions with predicate names. |
| `file_exists`, `file_absent` | Literal FileReference string or `{base: workspace, path: './relative'}`. Test a regular file after ordinary symlink resolution; missing is false/true, unreadable or unresolved is an observation failure. A directory is not a file. |
| `file_contains` | `{file: <same file operand>, text: <literal string>}`. Case-sensitive literal substring over UTF-8 contents; no regex, interpolation or parsing. Missing file is false; invalid UTF-8/read failure is an observation error. |
| `can_execute`, `cannot_execute` | Literal executable name, resolved using focused Sniff availability discovery and captured process search environment. Never launch the binary. Missing is false/true; discovery failure is not absence. |
| `in_repo`, `not_in_repo` | Literal `true` operand. Membership of the consuming document anchor, not schema source or ambient CWD. A failed probe is an error. |
| `os` | One of `macos`, `linux`, `windows`; WSL2 reports the Linux execution environment. Uses focused Sniff OS observation. |
| `timezone` | Literal IANA zone name, matching the observed local zone identifier; no abbreviation guessing. Failure to identify a zone is distinct from nonmatch. |
| `local_time`, `utc_time` | `{from: 'HH:MM:SS', until: 'HH:MM:SS'}`; half-open daily interval, start inclusive/end exclusive. Overnight intervals wrap midnight. Equal endpoints are invalid rather than ambiguous. |

`document` retains the old pure matcher as a predicate implementation, not a
second expression evaluator. Its `$path` uses existing normalized document path
semantics. For example, the old `all`/`none` fixture becomes one `document`
predicate with the same mapping. An old top-level OR list becomes one explicit
`group:` of translated `document` predicates. Legacy bare property mappings under `match`
are rejected with a typed migration diagnostic, not silently reinterpreted.
`trigger-schema` is rejected before matching and points to `schema-trigger`.

The normal expression registry exposes
`ActivationCapability::{SuppliedData, CapturedClock, Forbidden}`. Every function
must declare one, defaulting to Forbidden for new/unclassified functions.
CapturedClock functions receive the pass's clock explicitly and cannot consult
the host clock. SuppliedData functions receive only arguments/document data.
File, remote, Git and process-discovery functions remain forbidden in expression
predicates; their literal declarative counterparts above perform observations.
The existing EvaluationMode::Pure is not an authorization decision.

Activation `doc` is the supplied consuming document. Bare names address that
document only. `ctx` and `env` remain reserved namespaces but are unavailable on
this activation surface; explicitly using them is a preparation error. All
host-dependent activation supported here is represented by the declared
predicates and captured-clock functions, avoiding a second implicit host fact
catalog. Ordinary runtime `ctx`/`env` behavior is unchanged.

```rust
pub fn prepare_activation(source: &ActivationDefinition)
    -> Result<PreparedActivation, ActivationPreparationError>;
pub fn requested_facts(rules: &[PreparedActivation]) -> FactRequests;
pub fn observe_activation(requests: &FactRequests, context: &ObservationContext)
    -> ActivationSnapshot;
pub fn match_activation(rule: &PreparedActivation, snapshot: &ActivationSnapshot,
    document: &Value) -> Result<bool, ActivationError>;
```

Requests contain literal operands and declaring source origins. Snapshot stores
each requested fact as Present/Absent/Failed as appropriate, one clock instant,
timezone snapshot, and dependency/watch identities. Observation uses focused
Sniff APIs and biscuit-file FileReference resolution, not a broad host scan.
Fact reads may be collected from later untaken branches; matching itself has no
I/O or providers. Preparation errors apply to every branch. Eligible predicate
matching is left-to-right short-circuit: an observation failure matters only if
its predicate is reached before a decisive result. Do not replace failure with
false and do not reorder conditions in a way that changes error visibility.

Source-relative strings retain FileReference `&`/`@` behavior. Explicit workspace
paths must be explicit-relative and use one anchor with no search fallback.
Workspace anchor is the consuming document's owning repo/worktree root. Only a
confirmed absence of a repository permits nearest containing editor workspace
folder or captured CLI CWD fallback. An editor document outside all folders and
without a repo has an unresolved anchor, not the server's CWD. Missing anchors
produce typed fact failures when needed, with the rule's possible scope intact.
Paths stay platform-native internally; URI conversion and display normalization
do not define filesystem identity. Reuse FileReference/OS identity behavior;
never lowercase paths globally or require POSIX separators on Windows.

**Human decision:** confirmed 2026-09-19. DMLS refreshes time-dependent rules
while a document remains open without edits, at the next relevant time boundary,
and rechecks after sleep or clock changes. CLI evaluates activation once per
invocation. This is the starting policy; later performance work may adjust
scheduling. The table records implementation details under that decision, not
additional human-approved timer intervals.

| Dependency | Invalidation and freshness |
| --- | --- |
| Document data/path | On document version change, rename, open, and relevant workspace-folder configuration change. |
| Source YAML/imports and file facts | Watched create/update/delete, including missing targets and nearest existing parent of missing directories. Subscribe to discovery roots before relying on child directory existence. |
| Executable availability | Capture search environment when the consumer starts; watch relevant search directories/candidates and refresh on focus/request fallback. Configuration restart/refresh recaptures environment. Native Windows uses platform executable discovery semantics; Unix uses executable permissions. |
| Repository/package scope | Relevant repository/worktree metadata and manifests, folder changes and focus/request reconciliation through focused Sniff discovery. |
| OS | Immutable for the process. No periodic OS scan. |
| Zone/local clock | Refresh on timezone/clock change notification where available, and reconcile on focus/request. Evaluate ranges against actual instants: skipped local times do not occur; repeated times satisfy the same wall-time predicate in both occurrences. |
| Declarative time ranges | Schedule the next interval boundary or timezone offset transition for active documents; capture one new clock for the pass. Recompute after sleep/wake or backward/forward clock jumps. |
| Captured-clock expression functions | The shared capability descriptor supplies its next observable change boundary; if not derivable, reevaluate before each consumer request and on a conservative one-second timer while an open document depends on it. Preserve full captured precision within each evaluation. |

The one-second fallback is a scheduling choice, not a latency acceptance budget
or new language clock precision. Coalesce work and avoid timers for rules with
no clock dependency. CLI captures facts per requested composition/validation
pass; it does not introduce a persistent watcher service. DMLS can use client
watched-file registration or an existing native watcher adapter; if neither
reports a dependency, reconcile its captured facts at the next request/focus
event and report watch/observation failure rather than promise a current view.

## C7 — Ordering and root-union assembly

D20's property precedence is fixed: document > trigger > always-on > base.
Within a tier, nearest-directory-first and UTF-8 filename order are preserved,
explicit sources follow automatic sources, and later definitions win. Filename
shadowing is tier-local. Deduplicate only the same definition with equivalent
applicability/resolution context; retain both discovery dependency routes and
the explicit occurrence's priority. Distinct triggers remain distinct.

Implementation model under the settled layering rules: treat a single shape as
a one-arm union. This model is subject to technical review, not another request
to approve precedence or restrict the number of authored union sources.
Conceptually select one arm from each applicable contribution, assemble that
combination by D20's whole-property replacement, and accept a document if any
assembled combination validates. Do not flatten correlated alternatives into
independent optional properties, intersect overridden property validators, or
reject multiple union exports as an implementation shortcut. A higher-priority
property replaces that property within each combination; an arm that omits it
does not delete a lower-tier property's definition. Authored object constraints
remain conjunctive; ordinary property overlap alone is not a conflict error.

Keep this as a factored assembly graph with ordered overlay nodes and authored
alternative nodes; do not eagerly expand a Cartesian product into stored schema
copies. Validator and assistance queries traverse the same graph. Restrictions
are accumulated independently from every applicable source/ancestor and never
weakened by property replacement. For branch-dependent policies, accumulate bans
from every still-viable alternative. Narrow only using the shared validator's
existing supplied-data/discriminant rules; do not invoke providers to select a
permissive branch. With unresolved alternatives the union of their bans applies
before executing a feature. A branch ruled out by supplied data contributes no
branch-local policy, but enclosing/source restrictions still apply.

For hover/completion, use the existing discriminant/narrowing rules against the
effective alternatives. If several remain viable, show their possible property
types, distinguish branch-specific requirements, and retain each alternative's
description rather than invent a merged description. For a given arm's property
winner, missing description stays missing. Unknown-typed absent document values
do not select an arm. Preserve existing Launch/Completion required-property
semantics; this work does not make non-eager required inputs eagerly mandatory
or extend optional-null materialization to trigger schemas.

Required tests: two correlated unions plus base properties; same property
overridden at every tier; one branch omitting an overriding property; identical
arms coalesced without losing dependency routes; object/pattern constraints;
restrictions under unresolved alternatives; and consistent validation, hover and
completion after discriminant edits. Include the repository's feature/fix union
as a preserved export, without strengthening its authored optional properties.

## C8 — Dependency failure, suppression and publication

Each source/definition has `Pending`, `Ready`, `Failed` or intentionally
`Removed` state, a version, dependency routes and possible discovery scope.
Track negative dependencies too: a missing import or `schemas/` directory can
become present. Failure keeps source identity/cause and scope information, not
last-good semantic definitions. Optional source removal deactivates its
contribution; deleting a source still referenced by an active definition is a
required dependency failure.

Invalidate dependent contributions immediately when a dependency changes. One
immutable document view supplies diagnostics, hover and completion together;
no surface can keep an earlier dependent contribution after another suppresses
it. Independent Markdown/base checks and demonstrably independent definitions
continue. A failed higher-priority definition does not justify exposing a lower
definition as the current winner for possibly overridden properties. If the
failed source's shape or applicability is unknown, conservatively suppress
potentially dependent schema assistance within its possible scope while keeping
checks that are demonstrably unaffected. Do not label partial results complete.

```rust
pub struct RefreshTicket {
    pub document: DocumentIdentity,
    pub document_version: DocumentVersion,
    pub configuration_version: ConfigurationVersion,
    pub dependencies: Vec<(DependencyId, DependencyVersion)>,
}
```

Publication checks the entire dependency version vector plus document/config
identity. Discard a stale success or failure and ensure replacement work remains
scheduled; never strand a contribution Pending. Include discovery scope and
missing-source dependencies, not just successfully loaded files. Independent
source B changing does not invalidate source A's work unless A's ticket actually
depends on B. No single global source epoch cancels unrelated results. The
current synchronous refresh path can retain synchronous execution; this design
does not require an asynchronous worker rewrite.

When a rule cannot be parsed or observed well enough to decide applicability,
report incomplete validation for checked documents in its possible discovery
scope. Automatic nested scope excludes siblings; SCHEMA_DIR's possible scope
is workspace-wide. Failure diagnostics identify source, cause and dependency
route. Repair rebuilds the common view, restores all dependent surfaces and
clears the corresponding failure atomically. With no failure and an optional
rule absent/inactive, normal baseline validation is complete for that selected
set; mandatory Claudine runtime policies still come from the generated bundle.

## C9 — End-to-end integration flows

**Prepare and execute.** Claudine assembles existing caller/document layers with
their source origins and envelopes; Darkmatter loads/assembles the required
semantic view and prepares ordinary/deferred values in their existing stages.
Passive checking uses target-event descriptors and does not capture future
globals. At the existing event boundary Claudine captures snapshots, selects the
actual event/task and lexical scope, supplies all runtime entries and obtains a
checked fresh session. Darkmatter evaluates and validates the candidate result.
Claudine commits the whole operation or dispatches its effect only on success,
retaining the envelope for any later executable transfer.

**Approval and execution.** Prepare command expressions with the early approval
scope. Reject late globals and sequence lexical group references before any
approval request. Resolve command bytes once through Darkmatter, store a terminal
approved artifact tied to the task/command site, and use exactly those bytes at
primary/setup/teardown execution. The existing command whitelist remains the
authorization authority. Non-command stack fields remain prepared for their own
event. Refresh/proxy preparation audits newly authored commands under existing
policy; it does not re-evaluate a stored approved command at dispatch.

The Claudine-owned artifact interface is concrete:

```rust
pub struct ApprovedCommand { /* private command-site identity and exact bytes */ }

impl ApprovedCommand {
    pub fn command(&self) -> &str;
    pub fn site(&self) -> &CommandSiteId;
}

pub fn approve_resolved_command(
    site: CommandSiteId,
    resolved: ValueEnvelope,
    approvals: &mut CommandApprovals,
) -> Result<ApprovedCommand, CompositionError>;
```

CommandSiteId identifies source, prepared task instance and command field path;
primary/setup/teardown sites cannot overwrite each other's artifacts. The
constructor requires a terminal string and uses the existing approval/whitelist
policy. Store artifacts alongside prepared stacks and transfer them through task
construction; command dispatch accepts the artifact's bytes rather than its
authored expression. The whitelist can still reuse an exact previously approved
command under existing policy. This is provenance and byte-parity enforcement,
not a new authorization prompt or shell launcher.

**Mutation, handoff and recovery.** Evaluate a batch or proxy overlay against
the same pre-write view. Validate all candidates and commit value plus metadata
atomically. Select/rebase envelopes when moving values to child documents or
private task state. Repreparation combines retained origin policy with the new
destination policy, without resetting stages. On failure keep the typed cause
and unchanged retained snapshot; recovery uses its own scope plus enclosing
restrictions and the existing effect-routing policy.

**Editor refresh.** Discover scoped definitions and dependencies, parse rules,
collect literal fact requests, observe once and match the snapshot. Assemble
ready contributions with D20/C7, mark uncertain dependent contributions
unavailable, and publish the common current view only under a valid ticket.
Diagnostics, hover and completion read that view. Watch/focus/request/time events
invalidate only their dependency closure; successful repair restores assistance
without last-good fallback. No lifecycle providers or effect engine participate.

See [migration-inventory.md](migration-inventory.md) for concrete source
dispositions, transfer coverage, DRY ownership and all 22 acceptance mappings.
The separate lifecycle-extraction draft is not incorporated into these flows.
