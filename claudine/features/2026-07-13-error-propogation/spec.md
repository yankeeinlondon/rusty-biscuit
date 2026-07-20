---
created: 2026-07-13
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-16
review_iterations: 4
depends_on:
    - ../_completed/2026-06-28-real-errors/spec.md
related:
    - ../2026-07-13-file-resolution/spec.md
---

# End-to-End Typed Error Propagation

## Motivating Incident

A lifecycle `initialize` proxy failed to resolve its target and surfaced as:

```text
Error: lifecycle initialize proxy: path resolution failed for
"prompts/implement/implement-suggestions.md": proxy target does not exist: ...
```

The path resolver had already produced a typed `HarnessError::PathResolutionFailed`.
That error implemented both Claudine's `Diagnostic` contract and
`biscuit_terminal::errors::BlockError`, but the composition orchestration layer
converted it into a formatted `eyre!` string before returning it. The top-level
error walker could therefore see only an opaque message and used the generic
`Error:` fallback.

This was not an isolated missing renderer. It exposed a pipeline failure:

> Errors are typed where they originate, erased while crossing orchestration
> boundaries, and only partially rediscovered at the presentation boundary.

The completed real-errors work established the correct architecture: the typed
error is the single source of truth for rendering and handling, and Claudine
must preserve typed sources rather than flattening them to `String`. This
feature makes that architecture enforceable across the CLI and library instead
of relying on each call site to remember it.

> Reader's note: the inline review preserves unified file-reference resolution
> as a related downstream workstream because that specification depends on the
> typed transport established here; reversing the edge would create a cycle.
> This feature reserves the richer invalid-reference detail shape, while the
> file-resolution feature supplies the reference kind, candidate/probe plan,
> and resolution context. This revision also distinguishes an in-process Rust
> source chain from the serializable diagnostic snapshot used at process and
> persistence boundaries; a concrete Rust error type cannot survive
> serialization.

## Goals

- Preserve the concrete error and its complete `Error::source()` chain across
  every in-process Claudine library, CLI, lifecycle, and harness boundary.
- Project a versioned diagnostic snapshot that losslessly retains the public
  facets and detail at process, wire, and persistence boundaries instead of
  pretending that a concrete Rust type can cross those boundaries.
- Add semantic context with typed wrappers that retain their source, never by
  interpolating an error's `Display` value into a new opaque report.
- Ensure every Claudine error family implementing `Diagnostic`/`BlockError` is
  discoverable by the top-level renderer after type erasure into
  `color_eyre::Report`.
- Use the same effective diagnostic for terminal rendering and the lifecycle
  `err.*` projection, with explicit rules for transparent and semantic
  wrappers.
- Preserve structured detail, source-document context, diagnostic identity,
  disposition, origin, and remediation hints through every route.
- Render every registered ordinary terminal diagnostic through `BlockError`
  and `TerminalRenderable` components, including plain/no-color output.
- Add structural and end-to-end tests that prevent lossy error boundaries from
  silently returning.
- Audit existing production conversions and close every confirmed typed-error
  flattening site in the Claudine package area.

## Non-goals

- Replacing `thiserror`, `color-eyre`, `BlockError`, `StatusBlock`, or
  `SourceContext` with a new diagnostics framework.
- Redesigning the visual language of every existing error block.
- Changing lifecycle failure/finalize routing, retry policy, proxy placement,
  or provider execution behavior except where typed propagation is currently
  lost.
- Changing file-reference resolution precedence. That behavior belongs to the
  related file-resolution specification.
- Converting intentionally user-authored prose, provider stderr, or remote API
  text into invented structured causes when no typed source exists.
- Making every internal error public. Stability applies to the documented
  `Diagnostic` facets and registered codes, not private Rust enum layouts.
- Reworking error systems in the standalone `claudine-gen` or rendezvous
  binaries unless one of their errors crosses the core `claudine` CLI boundary.
  Their typed sources are included in the lossy-boundary inventory, but their
  independent presentation contracts are not redesigned here.

## Existing Contracts

This feature extends rather than replaces these ratified contracts:

1. `Diagnostic: BlockError`: handling and human rendering are two projections
   of the same typed error.
2. Transparent wrappers delegate classification and rendering to their
   meaningful cause; a semantic boundary that deliberately reclassifies an
   error owns its facets while retaining the underlying source.
3. `Display` is presentation only. It is not a durable transport format.
4. Frontmatter-rooted errors are enriched at the render boundary so upstream
   control-flow matching still sees the original typed variant.
5. Terminal output honors TTY detection, `NO_COLOR`, `FORCE_COLOR`, OSC 8
   hyperlinks, and ANSI stripping through the shared component stack.
6. A lifecycle evaluation error's intentional early-emission behavior remains
   exceptional and exactly-once; it does not create a general license for
   subsystem-local rendering.
7. Lifecycle `err.msg` is a concise notification headline: escape-free,
   single-line, non-empty, and capped at approximately 240 characters. Provider
   attempt failures retain the ratified `harness::failure_message` precedence;
   the full error remains in structured detail, source chains, and logs.

## Failure Pattern

### 1. String wrapping destroys provenance

These shapes are lossy when `e` is a typed error:

```rust
.map_err(|e| eyre!("operation failed: {e}"))?
Err(eyre!("{e}"))
SomeError { reason: e.to_string() }
format!("operation failed: {e}")
```

They retain the text a human might read, but discard the concrete type,
`source()` chain, `Diagnostic` facets, structured detail, and `BlockError`
renderer.

Semantic context must instead be represented by a typed wrapper with a source:

```rust
#[derive(Debug, thiserror::Error)]
enum CompositionError {
    #[error("lifecycle `{event}` proxy target could not be resolved")]
    LifecycleProxyResolution {
        source_path: PathBuf,
        event: LifecycleSignal,
        target: String,
        #[source]
        source: HarnessError,
    },
}
```

The exact enum ownership is an implementation decision. The invariant is not:
every contextual wrapper carries its typed source and the structured fields
needed for classification and rendering.

### 2. Renderer discovery is incomplete

Stable Rust cannot generically upcast `&dyn Error` to `&dyn BlockError`.
Concrete downcast registration is therefore unavoidable. Today the CLI walker
uses Darkmatter's registry and a direct `CompositionError` check; it does not
provide a Claudine-owned discovery seam covering `ClaudineError`,
`HarnessError`, and every other registered Claudine diagnostic family.

Preserving a typed source is necessary but insufficient if the renderer cannot
rediscover it after it becomes `dyn Error`.

### 3. Generic variants lose semantic context too early

A typed enum is not automatically a useful diagnostic. Variants such as
`PathResolutionFailed { raw, detail: String }` still collapse structured facts
into a generic message and may classify an authoring failure as an environment
failure. File-origin failures need source context, the authored property or
surface, available typed resolution context, and the typed lower-level cause
where one exists. The downstream file-resolution feature adds the normalized
kind and candidate/probe plan rather than this feature reverse-engineering them.

The domain boundary that understands the operation must add those facts before
returning the error. A renderer must not reverse-engineer them from `Display`.

### 4. Parallel orchestration routes drift

Lifecycle proxying, retries, loop execution, target initialization, and
terminal recovery have multiple orchestration entry points. Some retain typed
errors while others create `eyre!` strings. The same underlying failure can
therefore render differently depending on which lifecycle event reached it.

Shared operations must return the same typed error shape on every route.

## Required Design

### D1 — Typed transport at every boundary

Production functions that can return a known error family must preserve it by
one of these mechanisms:

- returning the concrete typed error;
- converting with `#[from]` into a typed wrapper;
- wrapping with a variant carrying `#[source]`;
- using `Report::wrap_err`/equivalent only when the concrete source remains in
  the report chain and no domain-specific structured context is required.

Formatted strings are allowed only for genuinely unstructured external text or
for final `Display` rendering. They are not allowed as a replacement for a
typed error already in hand.

### D2 — One Claudine discovery registry

The Claudine library owns one public downcast seam for its complete diagnostic
family, analogous to Darkmatter's `as_block_error`:

```rust
fn as_diagnostic(error: &(dyn Error + 'static)) -> Option<&dyn Diagnostic>
```

The function may be `#[doc(hidden)]`, but it cannot be `pub(crate)`: the
`claudine-cli` package is a separate crate and must be able to call it.

The exact function name may differ, but it must:

- recognize every concrete Claudine type that implements `Diagnostic`;
- return a `BlockError`-capable diagnostic from the same value;
- be the only Claudine concrete-type allowlist used by CLI rendering;
- compose with Darkmatter's discovery registry for lower-layer causes;
- pair its runtime tests with a Rust-aware source parity test comparing every
  production `impl Diagnostic for ...` against the registry. Rust provides no
  reflection that can make a hand-authored downcast list exhaustive by itself.

The CLI error walker performs one outer-to-inner cause-chain traversal and uses
the effective-diagnostic selection contract in D4. It must not maintain a
second partial type list.

### D3 — Semantic wrappers own context, causes own mechanics

When a lower-level error does not know the user operation, its caller adds a
typed semantic wrapper. For example, a filesystem resolver may know that no
candidate exists; the composition layer knows that the authored
`initialize.stack[*].proxy` property requested it.

A semantic wrapper carries, where applicable:

- source document `SourceContext`;
- lifecycle event and action/property path;
- raw authored value;
- normalized reference kind, when supplied by the lower resolver;
- ordered resolution candidates or bases, when supplied by the lower resolver;
- underlying typed source;
- hints based on the actual authoring contract.

For the lifecycle proxy miss in this specification, the wrapper owns
`composition.invalid_file_reference`; it does not introduce a proxy-specific
code. The lifecycle event and property path distinguish the surface in
structured detail, while the lower resolution or I/O diagnostic remains its
typed source. This keeps one public identity for the same authoring mistake
across proxying, expressions, schemas, and transclusion.

The locked catalog evolves additively for this richer context. The existing
`composition.invalid_file_reference` fields remain present, including
`fallback_dir`, and the implementation adds `source_path`, `property`,
`event`, `repository_root`, `candidates`, and `failure`. `kind` continues to
mean the reference kind; `failure` uses stable snake_case slugs for invalid
syntax, missing context, no match, permission/I/O, or unsupported remote.
`candidates` is the ordered structured record sequence from the file-resolution
specification, including root provenance and probe disposition rather than
formatted path strings. `base_dir` and `fallback_dir` remain compatibility
projections.

This feature adds the catalog fields and always projects the full object shape;
fields unavailable from the current private resolver are `null`, not invented
or parsed from `Display`. The related file-resolution feature replaces those
nulls with its typed classification, candidate/root provenance, and probe
dispositions. Error propagation must land first so that migration does not
flatten the richer resolver result on arrival.

### D4 — Rendering and `err.*` select the same effective diagnostic

The function that builds `LifecycleErrorInfo` and the function that renders a
top-level report must use the same effective-diagnostic selection contract.
Each registered diagnostic value is explicitly one of:

- **semantic/owning** — the boundary deliberately reclassifies the operation;
  it is selected and traversal stops for the primary diagnostic;
- **transparent** — it delegates both rendering and facets to its source;
  traversal continues.

Selection walks outer-to-inner. The first semantic/owning diagnostic wins. If
the chain contains only transparent registered diagnostics, the deepest one is
selected. This prevents a generic transparent wrapper from hiding a rich cause
without allowing a low-level filesystem error to steal identity from a
semantic authoring error. The role must be represented as data or an
object-safe trait method; it must not be inferred from enum names or `Display`
text.

The walk terminates on repeated error-object identity and enforces a generous
maximum depth so a malformed third-party `source()` implementation cannot hang
error reporting. Reaching either guard is recorded in diagnostic logs and does
not replace the best candidate already selected.

The selected value obeys these projection rules:

- `err.category`, `err.code`, `err.disposition`, `err.origin`, and
  `err.detail.*` come from the same diagnostic that supplies the rendered
  headline/body/hint;
- `err.msg` is that diagnostic's concise message projection after the existing
  notification-hygiene pass, not its multiline terminal block and not a
  classifier input;
- `err.cause.*` exposes the next typed diagnostic in the chain when present;
- transparent wrappers delegate both projections;
- deliberate semantic wrappers own both projections.

No route may classify one cause while rendering a different unrelated cause.
`err.cause.*` exposes exactly the next registered diagnostic below the primary
one when present. It is a one-level projection in v1; `err.cause.cause` is not
exposed. The complete Rust `Error::source()` chain remains available to
in-process callers and logs.

### D5 — One ordinary render boundary

Ordinary command failures return typed errors to the top-level CLI renderer.
Subcommands do not print an error and then return a second error for the same
failure.

The existing early lifecycle-evaluation emission remains because catch events
must run after the original crash is visible. It stays explicitly marked as
already emitted and covered by exactly-once tests. Any additional early render
boundary requires a separate design ruling.

When no registered diagnostic exists anywhere in the chain, the generic
fallback remains valid for truly unstructured errors. A registered diagnostic
falling through to that path is always a defect.

### D6 — Frontmatter and source enrichment survive wrapping

Frontmatter enrichment remains a transparent render-boundary wrapper. It must
locate the meaningful inner diagnostic through arbitrary typed source wrappers,
append the source excerpt once, and leave control-flow matching unchanged.

Errors originating from prompt frontmatter must carry enough location metadata
to highlight the relevant property or action. If exact span data is not
available, the nearest stable property path is required.

### D7 — Machine-readable detail remains lossless

Every registered diagnostic code's declared detail fields must be present in
`Diagnostic::detail()`, with absent optional values represented as `null`. A
registered code with declared fields must return an object with those keys,
never a top-level `null`. Undeclared ad hoc fields are rejected by parity tests
until the catalog is extended.
Structured values must remain structured; arrays, objects, paths, candidate
lists, and cause snapshots must not be flattened into one message field.

Serialization/logging may add a human `message`, but it cannot replace the
facets or detail payload.

The current `LifecycleErrorInfo::from_action_failure(error_kind, message)`
shape is a legacy lossy boundary when its caller already has typed provider,
cap, timeout, runaway, or harness data. Those callers must pass the selected
diagnostic or a diagnostic snapshot. A genuinely prose-only lifecycle action
failure may remain facet-less; it must not claim a registered code while
projecting an empty or top-level-null detail payload.

### D8 — Lossy-boundary inventory and enforcement

Implementation starts with a production-code inventory of:

- `eyre!("...{e}...")` and equivalent formatted-report construction;
- `.map_err(|e| ... e.to_string() ...)`;
- error-bearing `reason: String` / `message: String` fields populated from a
  typed error;
- manual `format!` context immediately before returning an error;
- error logging followed by returning another error;
- concrete `BlockError`/`Diagnostic` implementations absent from discovery.

The inventory covers production Rust sources in `claudine/lib`,
`claudine/cli`, and `claudine/contract`, plus crossings from `claudine-gen` and
the rendezvous crates into the core CLI. Generated sources, test fixtures, and
snapshot literals are excluded structurally rather than by broad substring
exceptions.

Known migration anchors are not deferred to discovery: the inventory must
include `cli/src/output/error_walker.rs` (its direct `CompositionError`
allowlist), `lib/src/composition/lifecycle/context.rs`
(`from_action_failure`), `cli/src/commands/wrap/harness_orch/loop_control.rs`
(typed errors converted with `to_string()`), `lib/src/harness/error.rs`
(`PathResolutionFailed { detail: String }`), and the pre-flight wrapper in
`cli/src/commands/wrap/harness_orch/prompt.rs`. These are starting points, not
an allowlist of the only affected symbols.

Each occurrence is classified as:

1. typed provenance defect — replace it;
2. genuinely unstructured external text — retain with an explicit reason;
3. presentation-only conversion after the final render boundary — retain;
4. deferred authored-matching-surface change — a typed error *is* in hand, but
   typing it here would move a surface author-written `when:` clauses select on
   (`err.kind`/`err.variant`/`err.code`), which D10 reserves for a separate
   versioned migration. Retain, tagged distinctly from category 2/3, with a
   reason naming the surface that would move.

Category 4 is a closed set, not a burn-down. It exists because D10's
behavior-neutrality constraint and D1's typed-transport requirement genuinely
conflict at a site whose flattened string feeds lifecycle routing: honoring D1
there would silently stop an authored rule from matching. The set does not
grow — a *new* collapse with a typed source in hand is a category-1 defect and
must be fixed, never tagged. The known category-4 site is
`cli/src/commands/wrap/inline.rs::try_inline_closure`, whose flattened
`CompositionError` feeds `LifecycleErrorInfo::from_action_failure`.

A Rust-aware source-level drift test scans the complete in-scope production
roots for the known lossy patterns. Exceptions use a narrow allowlist tied to
an enclosing symbol and explain why no typed source exists — or, for category
4, why typing it would move a matching surface. This is a regression guard, not
the authority for correctness; typed-chain and L2 tests remain mandatory.

### D9 — Serializable diagnostic snapshots

Concrete Rust error values never cross process, wire, or persistence
boundaries. At the last in-process boundary they are projected once into a
versioned `DiagnosticSnapshot` (the exact type name may differ) containing:

- schema version;
- category, code, disposition, origin, and severity;
- structured detail with the catalog-declared shape;
- concise, notification-safe presentation message;
- the one-level next registered cause from D4, when present.

`LifecycleErrorInfo`, machine output, recovery records, and cross-process
consumers use this shared shape rather than independently rebuilding facets.
Deserialization preserves unknown additive codes and detail fields so a newer
producer does not become unreadable to an older persistence/reporting path.
Wire-facing facet values are therefore preserved as owned strings at the
snapshot boundary even when the in-process API uses closed enums.
The snapshot does not attempt to recreate a private Rust enum on the receiving
side. It is lossless for the public diagnostic projection, not for arbitrary
private source types or deeper unregistered prose causes.

### D10 — Behavior-neutral transport migration

Typed propagation must not accidentally alter process exit status, lifecycle
event ordering, retry/resume/proxy decisions, or whether a failure is emitted
once. Before changing a route, characterize its exit code, selected lifecycle
events, and emission count. The intended behavior changes in this feature are
limited to richer rendering, richer machine detail, correction of a diagnostic
identity that was previously lost, and the documented `err.msg` change to the
effective diagnostic's concise message projection.

The existing provider-attempt message cascade remains authoritative for those
failures; it is not replaced by a generic `Display` call. Repo-owned lifecycle
examples and tests that consume `err.msg` are inventoried to verify the message
remains useful for TTS and messaging. No compatibility field preserves the old
flattened wrapper string because matching belongs on `err.code` and the full
context is available in structured detail.

Any routing or retry-policy change discovered during the audit is split into a
separate specification. Additive fields in JSON/machine output are intentional
and must be documented; removing or renaming an existing field is out of scope.

## Open Questions

### Where should cross-crate diagnostic identity live?

Darkmatter and biscuit-file cannot implement Claudine's current `Diagnostic`
trait without reversing the dependency graph, yet their `BlockError` values
can be deeper in the same source chain. The draft's original phrase "deepest
meaningful typed cause" therefore did not define an implementable same-value
contract.

#### Option A — Claudine semantic adapters (recommended)

Every lower-layer error crossing into Claudine is retained as `#[source]` by a
Claudine diagnostic wrapper. That wrapper owns or transparently delegates both
facets and `BlockError` rendering, and D4 selects it before consulting the
lower-layer `BlockError` registry.

**Pros:** preserves the dependency direction and Claudine-owned code catalog;
keeps this feature scoped; gives lifecycle and terminal output one selected
value; allows operation-specific context at the boundary.

**Cons:** every lower-layer ingress must be wrapped; direct Darkmatter callers
outside Claudine do not gain Claudine facets; careless adapters could duplicate
detail unless parity tests enforce a single projection.

#### Option B — Move the diagnostic trait to a shared lower crate

Move a provider-neutral diagnostic facet trait beside `BlockError` in
`biscuit-terminal` (or another dependency leaf) and implement it directly in
Darkmatter, biscuit-file, and Claudine.

**Pros:** the deepest concrete value can truly render and classify itself;
discovery composes naturally across crates; other binaries can reuse the
facets.

**Cons:** expands a Claudine transport fix into a cross-area public API change;
requires deciding who owns category/code catalogs; forces lower libraries to
adopt Claudine-oriented taxonomy or a new abstraction.

#### Option C — Pair separate render and classification selections

Keep lower `BlockError` discovery and Claudine `Diagnostic` discovery separate,
then build an `EffectiveDiagnostic` projection containing a render source and
a facet source.

**Pros:** smallest change to existing error types; no dependency movement.

**Cons:** structurally permits the two identities to drift, contradicts the
ratified one-taxonomy goal, and makes source enrichment and `err.cause.*`
ambiguous.

**Recommendation:** Option A. Claudine is the operation-aware boundary and owns
the locked lifecycle code catalog, so it is the right place to adapt a
mechanical lower-layer cause into a semantic diagnostic. D2–D10 assume Option A.
If maintainers prefer Option B, that decision should be ratified as a separate
cross-package contract before implementation; Option C should not be used.

## Error Rendering Contract

For a correctable, source-authored error, the rendered block should provide:

1. a domain-specific headline;
2. the source document as an OSC 8 link when supported;
3. the authored value/property;
4. structured resolution or validation context;
5. the meaningful lower-level cause when it adds information;
6. one concrete corrective hint;
7. a frontmatter/source excerpt when available.

Plain and piped output contains the same information without ANSI or OSC 8
control sequences. JSON/machine surfaces expose the same diagnostic facets and
detail rather than a terminal-rendered string. A route-specific lifecycle event
or property path may differ, but the diagnostic code, available typed
resolution detail, headline, and corrective guidance for the same underlying
failure do not drift by route.

## Testing Strategy

### L1 — Typed propagation and registry

- Every contextual wrapper exposes its original concrete error through
  `Error::source()`.
- Every Claudine `Diagnostic` implementation is discoverable through the
  central registry after erasure to `dyn Error`, and the source-parity test
  proves that every production implementation is registered.
- Nested Claudine → Darkmatter → biscuit-file chains select the expected
  effective diagnostic under both owning and transparent wrapper cases.
- A repeated or over-depth `Error::source()` chain terminates and preserves the
  best diagnostic selected before the guard.
- Semantic wrappers and transparent wrappers project facets according to D4.
- Structured detail retains every registered field and type, including the
  additive invalid-reference fields and present-null optional values.
- Frontmatter enrichment remains transparent to typed matching.
- `DiagnosticSnapshot` round-trips every facet, structured detail, message,
  and one-level typed cause; unknown additive code/detail values survive a
  read/write cycle.
- Every `err.msg` remains escape-free, single-line, non-empty, and within the
  established length cap; provider-attempt cascade precedence is unchanged.
- The same selected diagnostic produces terminal rendering,
  `LifecycleErrorInfo`, and the serialized snapshot.

### L2 — Real CLI rendering

Run representative failures through the actual `claudine` process and assert
the terminal/plain snapshots for at least:

- lifecycle proxy resolution from `initialize`;
- proxy resolution from a terminal/recovery event;
- composition source lookup;
- schema/file-reference failure;
- Darkmatter transclusion failure;
- harness pre-flight failure;
- a deliberately unstructured fallback error.

Each typed case must render a `StatusBlock`, never the generic `Error:` line.
TTY, `NO_COLOR`, `FORCE_COLOR`, and piped stderr variants are covered where
their output contracts differ. The real-terminal cases use the package's
existing L2 process harness and remain target-gated or portable across macOS,
Windows, and Linux.

For the proxy cases, assert identical code, headline, and hint across routes,
plus parity for any typed resolution detail supplied by the current resolver.
The downstream file-resolution feature owns tests for the actual candidate and
probe order. Assert the event/property context separately so intentional
route-specific detail is not mistaken for rendering drift. Each route also
asserts its pre-migration exit code, lifecycle event order, and exactly-once
emission count.

### Regression enforcement

- The lossy-pattern inventory test fails on a new unapproved production site.
- A registry parity test fails when a new diagnostic type is omitted.
- Catalog parity fails for a missing declared detail key, an undeclared key, or
  a registered code whose detail is top-level `null`.
- Snapshot tests assert actionable content, not only `to_string()` substrings.
- Lifecycle tests assert both rendered identity and `err.*` facets for the same
  failure.

## Acceptance Criteria

1. No known typed Claudine/Darkmatter/biscuit-file error is flattened into a
   string while crossing an in-process production orchestration boundary, except
   at a ratified D8 category-4 site where typing it would move an authored
   `when:` matching surface that D10 reserves for a separate versioned
   migration; a versioned diagnostic snapshot is used at process, wire, and
   persistence boundaries. Category-4 sites are enumerated in the allowlist and
   do not grow.
2. All Claudine `Diagnostic` implementations are discoverable through one
   Claudine registry, the CLI walker uses it, and source parity proves the
   registry is complete.
3. Top-level rendering, lifecycle `err.*`, and serialized machine output select
   the same effective diagnostic and one-level registered cause.
4. The motivating proxy-resolution failure renders as a source-aware
   component block with actionable context and never as generic `Error:` text.
5. The same proxy failure has identical diagnostic identity, headline, hint,
   and available typed resolution detail regardless of which supported
   lifecycle route initiated the handoff; only documented event/property
   context may differ.
6. Frontmatter excerpts, no-color behavior, and exactly-once rendering retain
   their current contracts.
7. Registered diagnostic detail remains structured and conforms to the locked
   code catalog, including the additive invalid-file-reference fields and
   present-null optionals.
8. Existing exit codes, lifecycle ordering, and retry/resume/proxy decisions
   remain unchanged by the transport refactor, and `err.msg` retains its
   notification-hygiene and provider-message precedence contracts.
9. L1 registry/chain/snapshot tests, L2 CLI snapshots, the lossy-boundary drift
   guard, `just test`, `just test-l2`, and `just lint` pass in the Claudine
   package area.

## Documentation and Maintenance

- Update the Claudine error-architecture documentation with the central
  discovery seam, effective-diagnostic selection, snapshot boundary, and
  typed-wrapper rules.
- Add the specified fields to `composition.invalid_file_reference` in the
  error catalog and document `fallback_dir` as a compatibility projection;
  do not silently reuse a code with the wrong origin.
- Update lifecycle documentation for the effective `err.msg`, the one-level
  `err.cause.*` projection, and the rule that a registered code always carries
  a catalog-shaped detail object.
- Add the lossy-boundary audit procedure to the Claudine skill and contributor
  guidance.
- When behavior-changing error work touches a symbol, review its rustdoc and
  inline comments for stale claims about rendering or propagation.
