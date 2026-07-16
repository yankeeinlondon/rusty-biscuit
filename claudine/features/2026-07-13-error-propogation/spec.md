---
created: 2026-07-13
status: draft
reviewed: false
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

## Goals

- Preserve the concrete error and its complete `Error::source()` chain across
  every Claudine library, CLI, lifecycle, and harness boundary.
- Add semantic context with typed wrappers that retain their source, never by
  interpolating an error's `Display` value into a new opaque report.
- Ensure every Claudine error family implementing `Diagnostic`/`BlockError` is
  discoverable by the top-level renderer after type erasure into
  `color_eyre::Report`.
- Use the same deepest meaningful typed cause for terminal rendering and the
  lifecycle `err.*` projection.
- Preserve structured detail, source-document context, diagnostic identity,
  disposition, origin, and remediation hints through every route.
- Render all ordinary terminal failures through `BlockError` and
  `TerminalRenderable` components, including plain/no-color output.
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
surface, reference kind, resolution bases/candidates, and the typed lower-level
cause where one exists.

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

The Claudine library owns one public or crate-visible downcast seam for its
complete diagnostic family, analogous to Darkmatter's `as_block_error`:

```rust
fn as_diagnostic(error: &(dyn Error + 'static)) -> Option<&dyn Diagnostic>
```

The exact function name may differ, but it must:

- recognize every concrete Claudine type that implements `Diagnostic`;
- return a `BlockError`-capable diagnostic from the same value;
- be the only Claudine concrete-type allowlist used by CLI rendering;
- compose with Darkmatter's discovery registry for lower-layer causes;
- have exhaustive tests that fail when a diagnostic type is implemented but
  not registered.

The CLI error walker performs one outer-to-inner cause-chain traversal and
selects the deepest meaningful diagnostic according to the delegation rules.
It must not maintain a second partial type list.

### D3 — Semantic wrappers own context, causes own mechanics

When a lower-level error does not know the user operation, its caller adds a
typed semantic wrapper. For example, a filesystem resolver may know that no
candidate exists; the composition layer knows that the authored
`initialize.stack[*].proxy` property requested it.

A semantic wrapper carries, where applicable:

- source document `SourceContext`;
- lifecycle event and action/property path;
- raw authored value;
- normalized reference kind;
- ordered resolution candidates or bases;
- underlying typed source;
- hints based on the actual authoring contract.

The wrapper owns the appropriate diagnostic code/origin when the operation's
meaning differs from the generic cause. Its `cause` remains available for
lower-level inspection.

### D4 — Rendering and `err.*` select the same cause

The function that builds `LifecycleErrorInfo` and the function that renders a
top-level report must use the same meaningful-cause selection contract:

- `err.category`, `err.code`, `err.disposition`, `err.origin`, and
  `err.detail.*` come from the same diagnostic that supplies the rendered
  headline/body/hint;
- `err.msg` is that diagnostic's presentation string, not a classifier input;
- `err.cause.*` may expose the next typed diagnostic in the chain;
- transparent wrappers delegate both projections;
- deliberate semantic wrappers own both projections.

No route may classify one cause while rendering a different unrelated cause.

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
`Diagnostic::detail()`, with absent optional values represented as `null`.
Structured values must remain structured; arrays, objects, paths, candidate
lists, and typed causes must not be flattened into one message field.

Serialization/logging may add a human `message`, but it cannot replace the
facets or detail payload.

### D8 — Lossy-boundary inventory and enforcement

Implementation starts with a production-code inventory of:

- `eyre!("...{e}...")` and equivalent formatted-report construction;
- `.map_err(|e| ... e.to_string() ...)`;
- error-bearing `reason: String` / `message: String` fields populated from a
  typed error;
- manual `format!` context immediately before returning an error;
- error logging followed by returning another error;
- concrete `BlockError`/`Diagnostic` implementations absent from discovery.

Each occurrence is classified as:

1. typed provenance defect — replace it;
2. genuinely unstructured external text — retain with an explicit reason;
3. presentation-only conversion after the final render boundary — retain.

A source-level drift test scans the high-risk production modules for the known
lossy patterns. Exceptions use a narrow allowlist with a comment explaining why
no typed source exists. This is a regression guard, not the authority for
correctness; typed-chain and L2 tests remain mandatory.

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
detail rather than a terminal-rendered string.

## Testing Strategy

### L1 — Typed propagation and registry

- Every contextual wrapper exposes its original concrete error through
  `Error::source()`.
- Every Claudine `Diagnostic` implementation is discoverable through the
  central registry after erasure to `dyn Error`.
- Nested Claudine → Darkmatter → biscuit-file chains select the expected
  meaningful cause.
- Semantic wrappers and transparent wrappers project facets according to D4.
- Structured detail retains every registered field and type.
- Frontmatter enrichment remains transparent to typed matching.

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
their output contracts differ.

### Regression enforcement

- The lossy-pattern inventory test fails on a new unapproved production site.
- A registry parity test fails when a new diagnostic type is omitted.
- Snapshot tests assert actionable content, not only `to_string()` substrings.
- Lifecycle tests assert both rendered identity and `err.*` facets for the same
  failure.

## Acceptance Criteria

1. No known typed Claudine/Darkmatter/biscuit-file error is flattened into a
   string while crossing production orchestration boundaries.
2. All Claudine `Diagnostic` implementations are discoverable through one
   Claudine registry, and the CLI walker uses it.
3. Top-level rendering and lifecycle `err.*` select the same meaningful typed
   diagnostic.
4. The motivating proxy-resolution failure renders as a source-aware
   component block with actionable context and never as generic `Error:` text.
5. The same proxy failure renders identically regardless of which supported
   lifecycle route initiated the handoff.
6. Frontmatter excerpts, no-color behavior, and exactly-once rendering retain
   their current contracts.
7. Registered diagnostic detail remains structured and conforms to the locked
   code catalog.
8. L1 registry/chain tests, L2 CLI snapshots, the lossy-boundary drift guard,
   `just test`, `just test-l2`, and `just lint` pass in the Claudine package
   area.

## Documentation and Maintenance

- Update the Claudine error-architecture documentation with the central
  discovery seam and typed-wrapper rules.
- Update the error catalog when a semantic operation requires a new code or
  richer detail fields; do not silently reuse a code with the wrong origin.
- Add the lossy-boundary audit procedure to the Claudine skill and contributor
  guidance.
- When behavior-changing error work touches a symbol, review its rustdoc and
  inline comments for stale claims about rendering or propagation.
