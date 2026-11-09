# S7 (Extraction boundary) and S8 (Breadth): design report

Spec: `darkmatter/features/2026-11-05-dynamic-schema-types/spec.md` (draft, Rulings Q1–Q7 in force).
Read-only investigation, 2026-11-07. No builds or tests were run. GitNexus's index is
2864 commits behind HEAD (it predates the extraction), so `impact` was not usable.
Blast radius below comes from text search.

---

## 0. What the code does today (the facts the design rests on)

| Fact | Evidence |
|---|---|
| Enum members are parsed as a list of strings. The lexer reads a word until `,;()'"` or whitespace, so `{{` turns into a word and the parse then fails on the next token (this is the motivating error). | `simplified-schema/lib/src/simplified/grammar.rs:2353` (`parse_enum_members`), `:273` (`read_word`) |
| `Constraint::Members(Vec<String>)` carries no spans. `Constraint::Suggest(Vec<SuggestionCandidate>)` carries a span for each candidate. | `simplified/types.rs:984`, `:980`, `:1212` |
| Suggestion spans are relative to the grammar string. They are projected into the YAML or Markdown document through `DecodedScalar::project`, which handles plain, quoted, and block carriers. | `simplified/source.rs:2750` (`project_suggestion_spans`), `:2862`; the carrier boundary is described at `source.rs:1-15` |
| The scanner's one closing-delimiter rule is private: `scan_expression(content, start)`. | `expression/scan.rs:444` |
| `Constraint` and `TypeExpr` are **not** `#[non_exhaustive]`. A new variant breaks every exhaustive match, at compile time. | `simplified/types.rs:917`, `:239` |
| A `SimplifiedSchema` layer already keeps "parsed, but cannot be lowered" as state. `Layer.lowered: Result<Arc<Value>, Arc<SchemaError>>` holds a function-typed property or an in-memory reference error, and validation reports `ValidationFailure::Unprepared`. | `schema/layer.rs:36`, `:316`; `schema/mod.rs:248-271`; `simplified/convert.rs:1101` |
| Factories expand named types and **lower eagerly**. | `schema/layer.rs:290` (`resolve_authored`) |
| Layering (`layer_under`/`layer_over`) merges **lowered** JSON through `merge_baseline`. | `schema/layering.rs:77-90`; `resolve.rs:4185` |
| Darkmatter resolves the document `$schema` through `resolve_schema_with_roots`, which returns `ResolvedSchema { json_schema, .. }` already lowered. It then merges lowered layers in `merged_trigger_schema`. | `darkmatter/lib/src/markdown/schemas/mod.rs:716`, `:2078`; `simplified-schema/lib/src/resolve.rs:73`, `:169` |
| Trigger payloads are resolved, lowered, and compiled when installed (`enforce_compiles`). They are memoized by `Arc` identity, and the downstream memos depend on that identity. | `triggers/assemble.rs:267`, `:298`, `:821`, memo docs at `:355-395` |
| Imported definitions are inlined as copies. Only cyclic or shared ones keep a `DefKey` whose `DefNamespace` names the file. An inlined copy keeps no origin. | `resolve.rs:435`; `simplified/definitions.rs:32`, `:60` |
| `resolve::buffers::with_reads_retained` already keeps the exact text of every schema file read during one host operation. | `resolve/buffers.rs:98` |
| `load_data` failures are a Darkmatter type (`DataError`, `dm.data.*` codes, `DataSourceLocation`). The data contract runs `expand_named_types` and `to_json_schema_with_definitions` itself. | `darkmatter/lib/src/markdown/compose/expression/data/error.rs:21-31`, `:53`, `:67`; `data/contract.rs:144`, `:170` |
| Darkmatter's reserved roots are `fm`, `ctx`, `env`, `current`, `local`. A **bare** unbound identifier reads the document (`x` is `fm.x` unless a callback binds `x`). | `compose/expression/catalog/roots.rs:67`; `.claude/skills/darkmatter/expressions.md:166`, `:346` |
| `SchemaError::Unresolved` already exists. It means a `$schema` file reference failed. | `simplified-schema/lib/src/errors.rs:244` |

---

## S7. Extraction boundary

### S7.1 Template representation

**Recommendation: one `SimplifiedSchema` type with a template state, not a separate facade type.** Ruling Q2
says `SimplifiedSchema` "holds literal members and unresolved slots" and returns a typed error on lowering.
The layer already models "parsed, not lowerable" (`Layer.lowered: Err(..)`). A second facade type would have to
duplicate `query`, `properties`, `to_simplified_yaml`, and `select` for no gain. The type-level guarantee the
spec asks for comes from three things together:

1. the AST variants that carry slots,
2. a lowering entry point that cannot accept them (typed error), and
3. a resolve method that returns a **new** `SimplifiedSchema` whose layers contain no slot.

```rust
// simplified-schema/lib/src/simplified/slot.rs  (new; re-exported from `simplified`)

/// One `{{ … }}` slot as authored. Passive: never evaluated by this crate.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaSlot {
    /// The parsed expression; spans are relative to `text`.
    pub expression: SpannedExpr,
    /// The expression source between the delimiters, verbatim (for YAML
    /// export and as part of the cache key).
    pub text: Arc<str>,
    /// Byte span of the whole `{{ … }}` (delimiters included), first relative to
    /// the grammar string, then projected to the defining document like a
    /// suggestion candidate's span.
    pub span: SourceSpan,
    /// The source that authored the slot. Stamped by the loader, so it survives
    /// named-type inlining and alias chains. `InlineDocument` for frontmatter.
    pub origin: SchemaSourceId,
    /// What the slot must produce; fixed by its grammatical position.
    pub kind: SlotKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotKind {
    EnumMembers,                               // M1
    Suggestions { target: SimplifiedType },    // M2 (Reference => String until resolved, as today)
    // S8, if admitted:
    LiteralValue,
    DefaultValue,
    Bound { keyword: BoundKeyword, measure: BoundMeasure }, // min/max → Min, MinLen, MinItems …
    Pattern,
    TypeDefinition,
}

/// A list where slots and literal elements alternate (Q1).
#[derive(Debug, Clone, PartialEq)]
pub enum ListElement<T> { Literal(T), Slot(SchemaSlot) }
```

**Where slots live in the AST.** Add new variants. Do not change the type of `Members` or `Suggest`:

```rust
pub enum Constraint {
    // unchanged: concrete only (static, or produced by resolution)
    Members(Vec<String>),
    Suggest(Vec<SuggestionCandidate>),
    // new: present only in a template
    TemplateMembers(Vec<ListElement<Spanned<String>>>),
    TemplateSuggest(Vec<ListElement<SuggestionCandidate>>),
    // S8 (if admitted): TemplateLiteral(SchemaSlot), TemplateDefault(SchemaSlot),
    //                   TemplateBound(SchemaSlot), TemplatePattern(SchemaSlot)
    ...
}
// S8 whole-type slot (if admitted): TypeExpr::Slot(Box<SchemaSlot>)
```

Why new variants rather than `Members(Vec<ListElement<String>>)`:

- `Members` and `Suggest` are read by 28 files across simplified-schema, Darkmatter, DMLS, and Claudine. Most of
  them use `find_map` or `match c { Members(m) => …, _ => … }` (for example
  `claudine/lib/src/composition/schema/classify.rs:294`, `claudine/gen/src/schema_compat.rs:333`). Changing the
  payload type rewrites all of them.
- Those consumers only ever see a concrete `EffectiveSchema`, because a template cannot become one (S7.4).
  Keeping `Members` meaning "concrete" leaves them correct without edits.
- **Risk to name:** a wildcard consumer that is handed a *template* would read an enum that has no `Members`.
  The guard is structural: the only template-facing readers are the library's own query, completion, and
  serialize code, plus the DMLS pending projection. Add a library debug assertion
  (`SchemaRoot::contains_slots()`) to `to_json_schema_with_definitions` and to the `EffectiveSchema`
  constructor.

**Grammar.** In `parse_enum_members` and the `suggest` argument list, peek for an active `{{` (one not escaped
by `is_escaped_opener`) before `next_token`. When one is found, call a crate-visible
`scan::scan_expression(src, pos)`; that function is today's private `scan.rs:444`, given `pub(crate)` and no
other change. If it yields a span that parses, record a `SchemaSlot`. If not, report a positioned grammar error
(M3) using the parser's `ParseError` text and its `diagnostic_code`. This keeps one closing rule, as the spec
requires.

Quoted text is lexed by `read_quoted` before this check runs, so `'{{ x }}'` stays a literal member.

A list that contains at least one slot is stored as `TemplateMembers` or `TemplateSuggest`. A purely literal
list keeps today's representation and its static duplicate errors. The rule "enum requires members"
(`grammar.rs:1913`) is satisfied by any element, slot or literal.

Duplicate checking for suggestions in a list that contains slots moves to resolution (Q1, "Required
semantics").

**Spans and carriers.** `Spanned<String>` gives literal members spans, which DMLS needs anyway. Extend the
`Projector` (`source.rs:2750-2880`) so it projects slot spans, slot expression spans, and member spans the same
way it projects suggestion spans today. `atom_has_suggestions` becomes `atom_has_projected_spans`.
`scalar_matches_atom` keeps working because it compares unprojected atoms. Block, folded, and quoted carriers
are already handled by `DecodedScalar::project`.

The projection must also map the **expression's** inner spans, not only the outer `{{ … }}`. Otherwise a
positioned `load_data` argument error inside a double-quoted YAML carrier (escapes shift offsets) lands on the
wrong column. Map the outer span through `project`, then map each inner `SpannedExpr` span the same way. Where
an offset cannot be proven, use null, never a placeholder (M3's position rule).

**Origin.** `SchemaSlot::origin` is stamped once per parsed source by whoever knows the file:
`classify_yaml_schema_text(path, …)` (`resolve.rs:1065`), the named-type file memo, and
`Layer::from_frontmatter` (`InlineDocument`). Inlining a definition copies the atom, and the origin travels
with it. This is what makes "imported definitions keep their defining file's context" and M3's "primary source
is the authored slot in its defining file" hold through alias chains. `DefKey` covers only stored definitions,
so it is not enough.

**Template versus concrete, per operation.**

| Operation | On a template |
|---|---|
| `try_from`, `try_from_yaml`, `try_from_markdown`, `resolve` (the file factory) | Succeeds. Named types are expanded (passive, through the bound context). `Layer.lowered = Err(Arc::new(SchemaError::UnresolvedTemplate { slots }))`. |
| `query`, `property`, `properties`, `named_type`, `select` | Works. `TypeDefinition` gains `pending(&self) -> Option<PendingInfo>` (slot count, slot spans, literal elements). A list containing slots is never reported as complete. |
| `to_simplified_yaml` | Works. Serialize writes `{{ text }}` verbatim (`simplified/serialize.rs`). Round-trip parse is an identity. |
| `to_json_schema`, `validate_strict`, `validate_soft`, `layer_under`, `layer_over`, coercion | `SimplifiedSchemaError::Unprepared(Arc<SchemaError::UnresolvedTemplate{..}>)`, or `ValidationFailure::Unprepared`. This follows the existing function-type precedent, so no new failure path is needed. |
| `is_template() -> bool`, `slots() -> impl Iterator<Item = SlotSite<'_>>` | New, passive. |

`SchemaError::UnresolvedTemplate` must not reuse the name `Unresolved`, which already means a failed file
reference (`errors.rs:244`).

```rust
#[derive(Debug)]
pub struct SlotSite<'a> {
    pub slot: &'a SchemaSlot,
    /// Structural path from the declaration root (SchemaSourcePath); for an
    /// inlined import this is the *use* path, the slot's origin is the
    /// defining file.
    pub path: SchemaSourcePath,
    /// Literal elements beside the slot (Q1: visible before resolution).
    pub siblings: &'a [ListElement<…>],
}
```

### S7.2 Host-neutral resolver trait

```rust
// simplified-schema/lib/src/schema/resolution.rs (new)

/// The host's evaluator. simplified-schema never evaluates; it asks.
pub trait SlotResolver {
    /// The host's typed failure (Darkmatter: an enum wrapping `DataError`,
    /// `ApprovalRequired`, evaluation errors). Never stringified here.
    type Error: std::error::Error + Send + Sync + 'static;
    /// One input the evaluation observed (file bytes hash, missing probe,
    /// context value fingerprint, grant). Opaque to this crate; the host hashes
    /// with biscuit-hash and builds its own cache key.
    type Observation: Clone + Send + Sync + 'static;

    fn resolve_slot(
        &self,
        request: &SlotRequest<'_>,
    ) -> Result<SlotOutcome<Self::Observation>, Self::Error>;

    /// Polled between slots and before each nested resolution.
    fn is_canceled(&self) -> bool { false }
}

pub struct SlotRequest<'a> {
    pub slot: &'a SchemaSlot,          // expression, text, span, origin, kind
    pub expected: SlotKind,            // == slot.kind; duplicated for clarity of contract
    /// The defining file's directory and its derived resolution context:
    /// `context.for_cwd(defining_dir)`, derived from the captured request,
    /// never recaptured (spec "Evaluation context is the schema's").
    pub base_dir: &'a Path,
    pub context: &'a biscuit_file::FileResolutionContext,
    /// The active resolution stack, for nested resolution through the host
    /// (e.g. a `load_data` contract importing a dynamic named type).
    pub stack: &'a ResolutionStack,
}

pub struct SlotOutcome<O> {
    pub value: serde_json::Value,      // checked against `expected` by the library
    pub observations: Vec<O>,
}

/// Persistent (Arc-linked) stack of schema identities being resolved.
#[derive(Clone, Default)]
pub struct ResolutionStack(Option<Arc<Frame>>);
impl ResolutionStack {
    /// `Err(Cycle)` when `id` is already on the stack.
    pub fn push(&self, id: SchemaIdentity) -> Result<ResolutionStack, CycleError>;
    pub fn contains(&self, id: &SchemaIdentity) -> bool;
}
/// Defining source + template revision digest (host-neutral; the digest is
/// xxhash of the authored template text, computed in-crate is fine because
/// it is not a cache fingerprint of inputs, only an identity).
pub struct SchemaIdentity { pub source: SchemaSourceId, pub template_digest: u64 }
```

**Errors.** Keep the host type generic at the call boundary. Erase it only where it must cross a
non-generic `SchemaError`.

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SlotResolutionError<E> {
    /// The host failed; `cause` is the host's own typed error.
    #[error("…")] Host { site: OwnedSlotSite, #[source] cause: E },
    /// Wrong shape: scalar/null/mapping/nested array for a list slot, a
    /// non-string enum member, a non-number bound, an unparsable pattern or
    /// type-definition. `index` is the offending array element, if any.
    #[error("…")] Shape { site: OwnedSlotSite, expected: SlotKind, found: JsonKind, index: Option<usize> },
    /// Final enum/suggest list is empty after flattening.
    #[error("…")] EmptyList { site: OwnedSlotSite },
    /// The substituted schema failed lowering (e.g. invalid regex, min > max),
    /// positioned at the slot that supplied the value.
    #[error("…")] Lowering { site: OwnedSlotSite, #[source] cause: SchemaError },
    #[error("…")] Cycle { stack: Vec<SchemaIdentity> },
    #[error("resolution canceled")] Canceled,
}

/// The erased form stored in `SchemaError::SlotResolution(Arc<…>)` so
/// Darkmatter's `SchemaError`-returning paths (`effective_for`, triggers,
/// compose) carry it unchanged. Darkmatter downcasts to its `DataError`
/// to keep `dm.data.*` and `data_location` (M3).
pub type ErasedSlotError = SlotResolutionError<Arc<dyn HostCause>>;
pub trait HostCause: std::error::Error + Send + Sync + 'static {
    fn code(&self) -> Option<&'static str> { None }
    fn as_any(&self) -> &dyn std::any::Any;
}
```

`OwnedSlotSite { origin: SchemaSourceId, span: Option<SourceSpan>, path: SchemaSourcePath }` is the M3 primary
source. Data positions (`data_location`) stay inside the host cause as secondary locations, which are
Darkmatter's to project.

**Schema-file observations.** The library still reads imports and referenced schemas itself. Add
`resolve::buffers::with_reads_recorded(|| …) -> (T, Vec<SchemaRead>)`, which is `with_reads_retained` plus a
returned list `SchemaRead { path, text: Arc<str> }` of exactly the bytes that were used. The host hashes those
with biscuit-hash, which satisfies "hash bytes while reading" for transitive imports. Probes that found nothing
already come back as `SchemaProvenance::searched`. Hashing stays in the host. simplified-schema gains no
biscuit-hash dependency (the identity digest above can be biscuit-hash in the host too, if preferred).

### S7.3 The resolve step

```rust
impl SimplifiedSchema {
    pub fn is_template(&self) -> bool;
    pub fn slots(&self) -> impl Iterator<Item = SlotSite<'_>>;

    /// Evaluates every slot through `resolver` and returns a new, immutable,
    /// concrete schema. `self` is never changed. A static schema returns a
    /// clone (shares Arc'd definition and prepared state: fast path kept).
    pub fn resolve_with<R: SlotResolver>(
        &self,
        resolver: &R,
        stack: &ResolutionStack,
    ) -> Result<Resolved<R::Observation>, SlotResolutionError<R::Error>>;
}

pub struct Resolved<O> {
    pub schema: SimplifiedSchema,        // concrete; `is_template() == false`
    pub observations: Vec<O>,            // flattened in slot order
    pub schema_reads: Vec<SchemaRead>,   // from with_reads_recorded during expansion
}
```

Algorithm:

1. Push `SchemaIdentity`. If it is already on the stack, return `Cycle`.
2. Clone the *expanded* typed root from the template layer, `Typed { root, definitions }`. The authored form is
   kept for export.
3. Walk it in declaration order. For each slot, check `is_canceled`, call the resolver, then check the value's
   shape against `SlotKind`.
4. Build the replacement value: flatten one level, keep the first occurrence after interpretation, and
   reinterpret suggestions with `interpret_suggestion_parts` (`grammar.rs:3137`). Substitute `Members`,
   `Suggest`, or the S8 variant **as typed values**, never as reparsed text.
5. Substitute inside stored `DefinitionTable` bodies too. These are copied, so the template's `Arc` is never
   mutated.
6. Lower with `to_json_schema_with_definitions`. A failure becomes `Lowering`, positioned at the slot that
   supplied the value, when the value can be attributed to one.
7. Build a new `Layer` with `lowered: Ok(..)` and provenance plus the resolved inputs, and return it.

Atomicity follows from construction: nothing is shared mutably, and a failure drops the clone.
Re-resolution always starts from the template's `Layer`, because the concrete layer does not retain a link to
reuse.

Also provide the low-level form for Darkmatter's `ResolvedSchema` path:

```rust
pub fn resolve_slots<R: SlotResolver>(
    root: &SchemaRoot, definitions: &DefinitionTable, base_dir: &Path,
    context: &FileResolutionContext, resolver: &R, stack: &ResolutionStack,
) -> Result<(SchemaRoot, Arc<DefinitionTable>, Vec<R::Observation>), SlotResolutionError<R::Error>>;
```

In Darkmatter, `resolve::resolve_schema_with_roots` must also stop lowering eagerly when the result contains
slots. Add `ResolvedSchema::template: bool`, or split the result into
`ResolvedTemplate { simplified, definitions, provenance… }` plus a `lower()` step. Every caller of
`ResolvedSchema.json_schema` (30 files, see S7.7) then either sees a concrete schema or a typed
`UnresolvedTemplate`.

The least disruptive form: `ResolvedSchema.json_schema` stays a plain `Value` for static schemas.
`resolve_schema_with_roots` returns `Err(SchemaError::UnresolvedTemplate{..})` for a template. A new
`resolve_template_with_roots` returns the unlowered form, and Darkmatter's resolution-aware path calls it.
Existing callers keep their behavior: static schemas are unchanged and templates are a typed error. That is
the Q2 contract.

### S7.4 Merge: keep `merge_baseline` on lowered layers

**Recommendation: keep `EffectiveSchema`'s merge exactly as it is.** Resolve each layer to a concrete snapshot
first, lower it, then merge with `merge_baseline` in `merged_trigger_schema` (`mod.rs:2078`) and in
`SimplifiedSchema::layer` (`layering.rs:77`). Do not move the merge up to `SimplifiedSchema`. Reasons:

- Layer precedence is whole-property replacement (`resolve.rs:4185`). A slot sits inside one property's
  definition, so no slot result can change what wins a property. Resolving before the merge cannot change
  precedence.
- The merge handles `$defs` moves, re-keying, raw JSON Schema layers, and root-union documents
  (`move_baseline_definitions`). A typed merge would have to reproduce all of that for raw JSON layers, which
  have no typed form.
- The `MERGES` and phase memos and the validator cache are keyed by the `Arc` identity or hash of lowered JSON
  (`mod.rs:2065`, `assemble.rs:355`). They keep working unchanged, **provided the resolution cache returns the
  same `Arc<Value>` on a warm hit**. Make that a stated requirement of S2's cache.

Two adjustments are needed. Neither moves the merge.

1. **Triggers.** `resolve_file_payload` lowers and runs `enforce_compiles` when the payload is installed
   (`assemble.rs:298-350`, `:821`). For a template payload (Q6):
   - judge merge compatibility passively from the typed root (single object, no root union),
   - store `ResolvedPayload { json_schema: Pending(template) }`,
   - run `enforce_compiles` only after selection and resolution, positioned at the slot in the trigger file.
2. **Shadowed slots.** Whole-property precedence means that if the document redeclares `os`, a trigger
   payload's `os` slot contributes nothing. Resolving it anyway reads files and may raise approval prompts for
   a value that is discarded. `layer::declares` (`schema/layer.rs:~555`) gives the winning top-level key set
   passively, so resolution and the S5 pre-flight walk can skip slots under shadowed top-level properties.
   This does change the cache key of a layer's snapshot (the key set it resolved). See the question for the
   author (A1).

### S7.5 Passive rejection (Q3) and the Q4 prohibition through imports

**Q3, rejecting document and runtime bindings in every branch.** The library cannot hard-code Darkmatter's
root names (that would break host neutrality), so it should provide a generic walker and leave the policy to
the host:

```rust
// simplified-schema::expression::free  (new, passive)
/// Every free variable root (first `.` segment of `Variable`, base of a
/// member/index chain) and every called function name, over *every* branch
/// (both ternary arms, `||` right side, all args, callback bodies), with
/// callback-pattern names removed inside their bodies.
pub fn free_references(expr: &SpannedExpr) -> FreeReferences; // Vec<(name, span)> roots + calls
```

Darkmatter's `SchemaSlotPolicy::check(&SchemaSlot) -> Result<(), SlotPolicyError>` rejects the following,
each positioned at its own span:

- the `fm` root,
- **any bare free identifier**, because a bare `x` reads the document (`expressions.md:166`, `:346`),
- `local` outside a callback,
- lifecycle and document-state functions, using the function catalog's own classification, and
- shell syntax (Q5 "Excluded").

`ctx` and `env` stay, meaning the derived schema request context. `current` is volatile and mirrors `ctx`;
admit it or not according to S4's decision.

The check runs **at every point that admits a template**, so it applies to every template regardless of
whether it is resolved:

- trigger registry install (Q6's "pending" requires a *valid* template),
- `md schema validate <schema>`,
- DMLS diagnostics,
- named-type file loading on the Darkmatter side, and
- the start of `resolve_with` as a guard.

Because the check is passive and per-slot, it also covers slots in branches that evaluation would never reach.
The bare-identifier point is listed as clarification A2.

**Q4, no slots in the baseline or in kind definitions, including through imports.** Check after named-type
expansion, not on the authored text. Expansion is passive and inlines every imported definition, and the slot
keeps its `origin`, so one walk finds slots that came from imports. Add
`SchemaRoot::contains_slots()` / `DefinitionTable::contains_slots()` in the library, and apply it in
Darkmatter at these points:

- **The baseline.** `with_baseline_from_file`, `BASELINE_SCHEMA`, `--baseline-schema`, and the DMLS
  extension layer, which is assumed to be baseline-class (A3). The error is `SchemaError::SlotForbidden {
  site: Baseline, origin, span }`. Add an L1 guard asserting the shipped `fm.yaml` closure has no slot.
- **Kind definitions** (`yaml_documents/registry.rs`). Mark the claim broken with the same typed error.
  `claimed_values` (`claims.rs:87`) must also reject `TemplateMembers`. That should be explicit, not a
  wildcard miss.
- **Trigger match conditions** (`triggers/grammar.rs`, `matcher.rs`). Matching must stay passive (Q6), so a
  slot in a `$match`/`$path` condition is a schema error. Only the `$schema` payload may hold a template.

An ordinary `kind: schema` document such as `sandboxing.yaml` is allowed to hold slots.

### S7.6 Darkmatter integration (outline)

- `DarkmatterSchemas::with_slot_resolution(resolution: SlotResolution)`, where `SlotResolution` holds:
  - the data runtime (it shares the command's `DataRuntime` and read set),
  - the policy and approval handler (Q5),
  - the cancellation token,
  - the pinned-snapshot map for the command (spec "A command pins one concrete snapshot").
- Without it, `effective_for` on a template returns `SchemaError::UnresolvedTemplate`. That gives
  `md schema validate` its "unavailable check" (exit 1), and gives DMLS its pending state.
- The resolver impl in Darkmatter: `struct SchemaSlotEvaluator<'a> { … } impl SlotResolver for … { type Error =
  SlotHostError; type Observation = InputObservation; }`. `SlotHostError` wraps `DataError`, `ApprovalRequired`,
  and `ExpressionError`, and implements `HostCause::code()` so it returns `dm.data.*`.
- Nested resolution: a `load_data` contract that imports a dynamic named type (`data/contract.rs:144-170`)
  must call `resolve_slots` with the `request.stack` it was given. Before it waits on an in-flight cache entry,
  the host checks `stack.contains(id)` and reports `Cycle`, so the cache never waits on itself.

### S7.7 Blast radius (text search; GitNexus index stale)

| Change | Who is affected |
|---|---|
| New `Constraint` variants (`TemplateMembers`, `TemplateSuggest`, plus S8 variants) | Exhaustive `Constraint` matches. The proxy count (files matching `Constraint::Operands`) is 9 files: `simplified/{convert,grammar,serialize,types}.rs`, `resolve/recursive.rs`, Darkmatter `lifecycle/schema_projection.rs`, `call_binding/translate.rs`, `schemas/about.rs` (constraint descriptor catalog and its parity test `tests/l1/schema_grammar_descriptors.rs`), and `lifecycle_generated_operands.rs`. |
| `Members`/`Suggest` semantics unchanged (concrete) | No edit is needed in the 28 reader files, which include `claudine/lib/src/composition/schema/classify.rs:294,425`, `claudine/gen/src/schema_compat.rs:333`, `darkmatter/dmls/src/providers/frontmatter.rs`, and `triggers/matcher.rs`. Each must only ever receive concrete schemas, which is the S7.1 assertion. |
| New `TypeExpr::Slot` (S8 whole type only) | 44 files match `TypeExpr::Ref` arms: 18 in simplified-schema, 24 in Darkmatter lib/cli/dmls including tests, and 2 in Claudine (`classify.rs`, `gen/schema_compat.rs`). This is the main cost difference between S8 positions. |
| `SchemaError` new variants (`UnresolvedTemplate`, `SlotResolution`, `SlotForbidden`); it is not `non_exhaustive` (`errors.rs:193`) | Exhaustive matches in `errors.rs` (StatusBlock projection, `:814`, `:1024`) and Darkmatter's diagnostic projection. Also the DMLS diagnostic code mapping. |
| `resolve_schema_with_roots` / `ResolvedSchema` contract (template → typed error; new `resolve_template_with_roots`) | Resolve entry points: 15 Darkmatter-lib files, 4 DMLS, 1 CLI, 1 Claudine lib. `ResolvedSchema` users: 21 Darkmatter-lib, 1 DMLS, 8 library. |
| `effective_for` can return `UnresolvedTemplate` | 78 Darkmatter-lib files, 4 CLI, 4 DMLS, 15 Claudine (lib 10, cli 4, gen 1), 3 Messenger. Claudine's CLI completion uses `effective_for(&markdown).ok()?` (`claudine/cli/src/completion/schema_completion/mod.rs:56`), which would silently show no completions for a template. It should project pending, or attach a `SlotResolution`. |
| Trigger payload pending state | `triggers/assemble.rs` (`ResolvedPayload`, `enforce_compiles`, `file_payload_memo`), `resolve_trigger_layers` (`mod.rs:~800-860`), `payload_memo_tests.rs`. |
| `scan_expression` → `pub(crate)` | None outside the crate. |
| `resolve::buffers::with_reads_recorded` | Additive. |
| Passive check entry points | `md schema validate` (`cli/src/commands/schema/validate.rs`), the kind registry, trigger discovery, DMLS overlay (`dmls/src/overlay/schema.rs`, `suggestions.rs`). |
| Guards | `schema_language_ownership_guard.rs`: the owner table needs the new library-owned names (`SlotResolver`, `SchemaSlot`, `free_references`). The dependency-direction guard stays satisfied: no Darkmatter types are involved. |
| Shipped corpus | `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:1710` carve-out removed (M4). `simplified-schema/lib/tests/l1/shipped_corpus.rs` must admit templates. |

---

## S8. Breadth

The assessments below are relative to the enum/suggest baseline: list positions, `Constraint`-level variants,
and the `Projector` extension.

### `literal(...)`

- **Result shape:** one scalar (string, number, or boolean) inserted as `LiteralValue(Value)` with its native
  JSON type, so no text typing is involved. Null is rejected, matching the static rule (`grammar.rs:2524`). An
  array, a mapping, or a non-finite number is a `Shape` error.
- **Admission:** the slot is the whole positional value (`literal({{ … }}; required)`). Mixing is impossible,
  because literal takes exactly one value.
- **Diagnostics:** wrong shape at the slot. A `default(...)` that disagrees fails at lowering, which is
  attributed to the slot (`convert.rs:4162` behavior).
- **Editor:** completion offers nothing until resolved; hover shows "pending literal".
- **Interactions to watch:** `discriminant.rs` and root-union narrowing read literals passively. A slot there
  makes discrimination pending, which is fine because unions are only judged on concrete snapshots. Kind claims
  (`claimed_values`) are already excluded by Q4.
- **Cost:** low, about the same as enum. One `TemplateLiteral(SchemaSlot)` variant and `parse_literal_value`
  peeks for `{{`.
- **Recommendation: include.**

### `default(...)`

- **Result shape:** the same domain as a static default. A scalar comes from `arg_to_json` (`grammar.rs:3296`);
  null only on `null`; a string that parses as a type definition or schema on `type-definition`/`schema`
  (`grammar.rs:1971`).
- **Admission:** one slot as the sole argument.
- **Diagnostics:** shape errors, plus the lowering-time checks that already exist (union default collapse or
  conflict, literal mismatch, expression-default parse), attributed to the slot.
- **Editor:** a default affects completion insert text and hover. Show pending.
- **Hidden consequence:** `PropertyDef::has_default()` drives optional-property materialization (the
  compose seam, darkmatter `schema.md` "Composition and validation"). The *presence* of a default is still
  known passively because a slot exists, so materialization rules are unaffected. Only the value is
  deferred.
- **Cost:** low.
- **Recommendation: include.** M1 already requires the `default(...)` tail to coexist with slots, so the parser
  path is touched regardless.

### Numeric bounds `min`/`max`

- **Result shape:** one number admitted by the numeric model (`admit_number_spelling` / `numeric_arg`,
  `grammar.rs:3050`). The measure is decided passively by position: `Min`/`Max` (f64) for numbers, `MinLen`/
  `MaxLen` for character-counting owners, `MinItems`/`MaxItems` at array level. The latter two require a
  non-negative integer (`number_to_usize`, `grammar.rs:3273`). String numerals are rejected, with no implicit
  conversion, consistent with the enum rule.
- **Diagnostics:** shape and range errors at the slot. `min > max` fails at lowering, attributed to the slot.
- **Editor:** nothing is offered; hover shows pending.
- **Cost:** low. `BoundMeasure` is computed in `parse_one_constraint`, which already branches on
  `is_array_level` and `counts_characters`.
- **Recommendation: include.**

### Pattern constraints (`pattern(...)`)

- **Result shape:** one string. It must compile under the same rule as static patterns (`regex`, or
  `fancy_regex` when lookaround is present; `convert.rs:1576-1595`).
- **Diagnostics:** a compile failure is a `Lowering` error positioned at the slot, carrying the regex error.
- **Editor:** nothing is offered.
- **Cost:** low.
- **Safety note (reported separately, not a performance argument):** a data-supplied regex reaches
  `fancy_regex`, which backtracks. Validation of every document then runs author-external regex. `fancy_regex`
  has a backtrack limit, so this is bounded, but the Q5 "work limit for pure evaluation" does not cover
  *validation* time. Combined with Q5's automatic grant (any clean file in the repo), a pathological pattern can
  slow DMLS diagnostics. Recommend a size cap on slot-supplied patterns (or `regex`-only, no lookaround) and the
  existing backtrack limit made explicit. This is design guidance, not narrowing.
- **Inline pattern keys** (`PatternKey::Pattern`, a regex *key* in an inline object) are a different position,
  and slot keys change the property set. **Do not include** them. They are not on the S8 list.
- **Recommendation: include `pattern(...)`** with the cap.

### Whole type expressions (a slot yielding a `type-definition`)

- **Result shape:** a `type-definition` value, the same domain as the `type-definition` type, parsed with
  `parse_type_definition_value` (`simplified/mod.rs:262`), which accepts a string or a YAML list form. It
  produces a `PropertyDef`.
- **Placement:** wherever a property definition or union arm stands. The slot is the whole YAML value or
  arm. Postfix `{{ … }}(required)` / `{{ … }}[]` can reuse the existing use-site merge of
  `Name@file(postfix)`, treating the slot like a named reference whose body is computed. That is the cleanest
  rule, and it means `TypeExpr::Slot` behaves like `TypeExpr::Imported` until resolution.
- **Admission:**
  - the returned definition must contain no `{{ … }}`, so returned data never introduces another evaluation
    (spec "Reader's note"),
  - it must pass `non_null_problem` and the other checks, and
  - it is expanded with the normal named-type resolver.
- **Ill-defined point (needs a ruling, A4):** which file a returned `Name@./x.yaml` import is relative to.
  - Option (a): the slot's defining schema file, consistent with Q3 "schema's context".
  - Option (b): the file the data came from.
  - The library cannot know (b), because the resolver returns a value, not a provenance-tagged definition.
  - Recommendation: (a), and reject `Name@this` in a returned definition, mirroring `ThisImportRejected`
    (`data/error.rs:25`).
- **YAML hazard:** an unquoted value that starts with `{{` is a YAML flow mapping, not a scalar. The parser
  must report "quote the slot" when the YAML value is a mapping whose shape is `{{…}: null}`. The position map
  already falls back for unquoted `{{ }}` (`schema.md` positions notes).
- **Editor:** much weaker than the other positions. Until resolution the property's type, presence
  (`is_required`), structure, and nested keys are unknown, so key completion, hover, `md schema` assignment
  (`cli/src/commands/schema/assignment.rs`), and phase projection must all handle "pending type" everywhere.
  Q1's "literals stay visible" has no analog here.
- **Cost:** high. A new `TypeExpr` variant hits about 44 files of exhaustive matches, including two in
  Claudine. It also touches subtyping, narrowing, frontmatter shape, and lint, which all need a "pending"
  answer.
- **Recommendation: defer to a follow-up feature.** It is well defined once A4 is ruled, so this is about
  **implementation cost**, not budget or safety. Under the spec's own rule ("narrowing is justified only by
  performance") this is a request to *sequence* the work, not to narrow, and it needs the author's agreement
  (A5).

### Summary

| Position | Shape | Cost vs enum/suggest | Safe and well defined? | Recommend |
|---|---|---|---|---|
| enum members | `string[]`, flattened (M1) | baseline | yes | this feature |
| suggest | scalar[] (M2) | baseline | yes | this feature |
| `literal(...)` | one non-null scalar | ≈ same | yes | this feature |
| `default(...)` | static default domain | ≈ same | yes | this feature |
| `min`/`max` | finite number; int ≥ 0 for len/items | ≈ same | yes | this feature |
| `pattern(...)` | compilable regex string | ≈ same | yes, with a size/backtrack cap | this feature |
| whole type | `type-definition` value | high (~44 files) | needs A4 (import base) | follow-up (A5) |

---

## Questions for the author (not reopening rulings)

- **A1 (shadowed slots).** With whole-property precedence, must a slot under a property that a higher layer
  redeclares stay unread and unprompted? That is the strong form, in the spirit of Q6's "a trigger that no
  document selects reads nothing". The weak form resolves it anyway. The strong form makes a layer snapshot
  depend on the winning key set.
- **A2 (Q3 scope clarification).** In Darkmatter a bare free identifier reads the document, as `fm.x` does.
  Q3's "`fm.*` … and other document/runtime-only bindings" is taken to include bare names, `local` outside a
  callback, and lifecycle-state functions. Please confirm. Also: is `current.*` (volatile, mirrors `ctx`)
  admitted? That overlaps with S4.
- **A3.** Is the DMLS extension-schema layer (built-in → extensions → triggers → document) "baseline" for Q4?
  This report assumes yes.
- **A4 (whole-type slots).** Relative to what does an import inside a returned `type-definition` resolve?
  Recommended: the slot's defining file, with `Name@this` rejected.
- **A5 (sequencing).** Whole-type slots are recommended for a follow-up because of implementation cost, not
  budget. Under the spec's rule that narrowing needs a ruling, please confirm that this counts as sequencing
  rather than narrowing.
