# Error Architecture

How a typed error travels from the place it is raised to the place a human reads
it or a `when:` clause matches it — and the rules that keep those two answers the
same.

The **taxonomy** (the facet enums and the locked code catalog) is ratified in
[`error-catalog.md`](../../features/_completed/2026-06-28-real-errors/error-catalog.md);
its data model is [`error-structure.md`](../../features/_completed/2026-06-28-real-errors/error-structure.md).
This document is the **transport**: the seams, the selection rule, and what an
author of a new error must do. For the `err.*` surface those facets reach, see
[lifecycle.md](lifecycle.md#err-fields).

## The one-sentence version

A failure is a chain of typed errors. **One** function picks which link speaks
for it, and rendering, `err.*`, and machine output all call that function — so a
route cannot classify one cause while rendering another.

## Two traits, one chain

| Trait | Where | Answers |
|-------|-------|---------|
| `BlockError` | biscuit-terminal | "how does a *human* read this?" — the rendered `StatusBlock` |
| `Diagnostic` | `claudine::diagnostics` | "how does a *program* react to this?" — `category`, `code`, `disposition`, `origin`, `detail`, `severity` |

`Diagnostic` is a **supertrait** of `BlockError`, so an implementor is both. That
is deliberate: the same struct that supplies the "Did you mean `…`?" hint in a
block supplies `err.detail.suggestions` to a handler. If a field is worth showing
a human, it is worth exposing to a handler — one struct, both jobs.

### Where diagnostic identity lives

Claudine owns the facets; lower layers do not have any (**Option A**, ratified in
the feature's [decisions.md §D-1](../../features/2026-07-13-error-propogation/decisions.md)).
Every Darkmatter, biscuit-file, or `std::io` error that crosses into Claudine is
retained as a `#[source]` by a Claudine diagnostic wrapper. The cost is that
every ingress must be wrapped; what it buys is a dependency direction that does
not require a cross-area contract in biscuit-terminal.

## The discovery seam

Stable Rust cannot upcast an arbitrary `&dyn Error` to `&dyn Diagnostic`, so
`claudine::diagnostics::as_diagnostic` is a **hand-authored downcast allowlist**
over concrete types (`CompositionError`, `ClaudineError`, `HarnessError`, and
`RestoredDiagnostic` today).
It is `pub` because `claudine-cli` is a separate crate, and it is the CLI's *only*
Claudine allowlist.

Nothing in the language makes that list exhaustive. What does is
`registry_lists_every_diagnostic_impl` (`cli/tests/error_guards.rs`), which parses
every production `impl Diagnostic for …` out of the sources and fails in both
directions — an unregistered impl, or a downcast to a type no impl defines. A
missing arm here *is* the incident this architecture was built to fix: a
`Diagnostic` the walker cannot see renders as a bare `Error:` line.

## Effective-diagnostic selection

`select_effective_diagnostic` walks the chain **outer to inner**:

1. The first `Semantic` diagnostic **wins and stops**.
2. If the chain holds only `Transparent` wrappers (and lower-layer `BlockError`
   causes), the **deepest** candidate wins.
3. Nothing renderable → `None`, and the caller falls back to unstructured output.

### The role contract

`DiagnosticRole` is **data**, never inferred from an enum name or `Display` text:

- **`Semantic`** — this boundary deliberately classifies the operation and owns
  its facets. The default, because owning the classification is the norm and
  delegating is the deliberate act.
- **`Transparent`** — the wrapper delegates **both** facets *and* rendering to its
  cause, so it has no identity of its own. Selection continues through it.

The distinction that trips people: owning the identity does not mean rendering
*locally*. `CompositionError::ShellExpansionFailed` builds its `status_block`
from its cause's block while owning `composition.shell_expansion` — it is
`Semantic`. A wrapper carrying a typed Darkmatter cause is **always** `Semantic`
under Option A, because a Darkmatter cause supplies no facets, so delegating to
it would leave the failure unclassified.

> **Hazard.** A `Semantic` wrapper over a rich Darkmatter cause **must** delegate
> `status_block` to that cause. It wins selection now, and a `status_block` that
> falls through to the flat catch-all replaces Darkmatter's path/line/excerpt
> block with one line of `Display` text. The L2 capture suites are the detectors.

### `diagnostic_source` vs `Error::source`

The two transparent wrappers hold `inner: Box<CompositionError>` with **no
`#[source]`** — promoting it would make `color_eyre`'s cause-chain fallback print
the same `Display` text twice. So `Error::source()` returns `None` and the walk
would stop *at* the wrapper. `Diagnostic::diagnostic_source()` defaults to
`Error::source` and those two override it. Use it when a wrapper's meaningful
cause is not a `thiserror` source.

### Traversal guards

The walk terminates on **repeated error-object identity** and at
`MAX_SELECTION_DEPTH` (64 — far above any real chain; the deepest today is ~6).
Both guards **keep the best candidate already selected**: a malformed third-party
`source()` still reports the best cause seen before the chain went wrong. Identity
is the whole wide pointer (address *and* vtable), because a `#[from]` payload can
share its parent's address and treating that as a revisit would truncate a healthy
chain.

## The snapshot boundary

Concrete Rust error values never cross a process, wire, or persistence boundary.
At the last in-process boundary the selected diagnostic projects **once** into a
`DiagnosticSnapshot`, and every downstream consumer reads that shape.

Two rules make it survive version skew in both directions:

- **Facets are owned strings**, not the closed in-process enums. A newer producer
  may know a code an older consumer's enum cannot name; the older consumer must
  still read, store, and forward it. Re-narrowing to an enum is the *consumer's*
  choice, made where an unknown value can be handled — never a deserialization
  failure.
- **`detail` is an opaque value.** The catalog evolves additively, so a field
  added after a consumer shipped round-trips through it untouched.

`cause` is a **one-level** projection: `DiagnosticCause` carries no cause of its
own, so `cause.cause` is unrepresentable in v1 rather than merely undocumented.
Widening to recursion later is additive if it is ever ratified.

`schema_version` moves only for a **non-additive** change (a removed or re-typed
field). A new facet value, detail key, or code is additive by construction,
because both sides already tolerate values they do not know.

### Projecting from an erased boundary

A boundary that holds a `color_eyre::eyre::Report` rather than a concrete type is
**not** a boundary that lost its provenance. `Report` cannot be a `#[source]`
(it does not implement `std::error::Error`), but it *boxes* its source rather
than discarding it, so the typed diagnostic is still reachable by downcast.

`DiagnosticSnapshot::select(report.as_ref())` is the seam: it runs the same
`select_effective_diagnostic` walk the CLI renders through and returns the
projection, or `None` when the chain genuinely only ever held prose. A record at
such a boundary stores that projection **beside** its existing prose field —
never instead of it, because the prose is often load-bearing (`SequenceStepResult.error`
is compared against the `interrupted by SIGINT` sentinel to select exit code 130).

Storing the snapshot as data, rather than attaching a restored diagnostic as a
`#[source]`, is deliberate: a source field would change what
`next_registered_cause` finds and therefore make `err.cause.*` appear where it
was previously absent — an authored-matching-surface change §D10 reserves for a
versioned migration. It would also publish a `Box<RegisteredDiagnostic>` to the
chain, which the boxed-diagnostic guard exists to reject.

### Coming back the other way

Some snapshots are taken *early* and acted on much later. `CompositionPrepContext`
projects the launch-detection failure at prep time because the prep record is
`Clone` and a concrete error is not; `--repo` decides whether that failure is
fatal several stages further on. At that point the snapshot has to become an
error again, and lifting `snapshot.message` into an `eyre!` string is a **second
erasure** — at the one boundary that still held the code, category, disposition,
origin, detail, and cause.

`RestoredDiagnostic` is what such a boundary returns instead. Its facets are read
from the catalog row for the snapshot's `code`, so the failure re-enters
`select_effective_diagnostic` and renders a `StatusBlock` with the identity it
was projected with. Restoration is a fixed point: projecting a `RestoredDiagnostic`
yields the snapshot it was built from, cause included.

Two rules go with it:

- **Framing goes on `with_context`, not into the message.** "`--repo` requires
  startup repo detection" is context the snapshot cannot know; it prefixes the
  human message and leaves every facet untouched. Framing that deserves its own
  classification deserves its own typed error, not a restored one.
- **An unknown code degrades, it does not fail.** `Diagnostic::code` returns
  `&'static str`, so a code this build's catalog cannot name falls back to
  `internal.bug` while `detail` and `message` carry through. In-process
  restoration cannot reach that path; it exists so the seam is total.

### Evolving the machine surface

Every field added to the snapshot and to `err.*` is **intentionally additive**,
and that is the whole compatibility strategy:

- **Adding** a code, a detail key, a facet value, or a snapshot field is
  non-breaking. An older consumer round-trips what it does not recognize; an
  existing `when:` clause keeps matching.
- **Removing or renaming** one is breaking — it silently kills author `when:`
  clauses that match it — and is **out of scope** for any change that is not
  explicitly a versioned migration. `base_dir` and `fallback_dir` are retained
  as compatibility projections for exactly this reason, even though `candidates`
  supersedes them.

A field that exists but is not yet knowable is `null`, not absent. That is what
lets a producer declare the full shape before a resolver can fill it, which is
how `composition.invalid_file_reference` reserves its file-resolution fields
today.

## Rules for a typed wrapper

1. **Retain the typed cause.** Concrete typed error, `#[from]`, a `#[source]`
   field, or `wrap_err` — the last only where the concrete source stays in the
   chain and no structured context is needed. Never `format!("…{e}")`.
2. **Never box a registered diagnostic you need to reach.** A `#[source]` field
   typed `Box<E>` publishes **`Box<E>`** to the chain: `downcast_ref::<E>()`
   returns `None`, and `Box`'s own `source()` delegates to `E::source()`, so the
   walk **skips `E` at every depth**. Box the *context* instead, as
   `CompositionError::InvalidFileReference` does — boxing the context costs
   nothing; boxing the source costs discoverability. The same trap applies to a
   `Box<E>` reaching `Report::from`/`.into()`: unbox at the boundary
   (`Report::from(*error)`).
3. **A registered code always projects a catalog-shaped `detail` object.** Seed
   from `null_detail_for(code)` so every declared key is a *present* key, then
   overwrite what you can populate. An unavailable optional is `null`; a
   top-level `null` detail is a defect.
4. **Never invent a field you cannot source.** A field the current resolver
   cannot supply is `null` — not parsed out of `Display`, not back-derived from a
   neighbouring facet. See the `failure` field below for the worked example.
5. **One code per authoring mistake, not per surface.** `InvalidFileReference`
   owns `composition.invalid_file_reference` for proxying, expressions, schemas,
   and transclusion alike; `event` and `property` in structured detail tell the
   surfaces apart. That is what keeps one `when:` clause matching as new surfaces
   adopt it.
6. **`code → disposition` is 1:1 and stable.** If two failures would need
   different dispositions, they are different codes — not one code with a
   discriminant. (A `Category` is *not* disposition-uniform; only a code is.)

### Reasons a new code is (and is not) justified

`composition.shell_approval` is the worked example both ways. The approval family
— a user declining, a blacklist hit, a missing handler, `--dry-run` — earns one
code because every reason resolves the same way (change the document or the
approval configuration), which keeps the disposition stable while
`err.detail.reason` discriminates.

`CompositionError::PreFlightFailed` keeps the `composition.failed` catch-all,
because it is **prose** covering unrelated failures. Giving it the approval code
would mean parsing its own `Display` to discover which failure it is — the exact
defect this architecture removes. The fix for a prose error that deserves a code
is to *type it*, not to code the prose.

## Catalog additions

The `composition.invalid_file_reference` payload was extended additively. The
original five fields keep their names, order, and values:

| Field | Supplied today | Notes |
|-------|----------------|-------|
| `reference` | ✅ | the raw, unresolved reference |
| `kind` | ✅ | reference-kind slug: `not_found`, `malformed`, `found_elsewhere`, `remote_not_enabled` |
| `base_dir` | ✅ | **compatibility projection** — see below |
| `suggestions` | ✅ | the same did-you-mean list the human block shows |
| `fallback_dir` | ✅ | **compatibility projection** — see below |
| `source_path` | wrapper only | the document the reference was authored in |
| `property` | wrapper only | the authored property path |
| `event` | wrapper only | the lifecycle event that was running |
| `repository_root` | probe only | the resolver's plan root, projected when a probe ran; `null` otherwise |
| `candidates` | probe only | the ordered, provenance-carrying probe record, projected when a probe ran; `null` otherwise |
| `failure` | harness path | typed failure classification — `invalid_syntax`, `missing_context`, `no_match`, `permission_io`, `unsupported_remote` |

**`base_dir` and `fallback_dir` are compatibility projections.** They are the two
anchors the pre-`candidates` payload exposed, retained so an existing `when:`
clause keeps matching. `candidates` supersedes them.

**Two resolver paths today.** The shared `biscuit-file`/harness resolver reaches
`composition.invalid_file_reference` through two arms.
`HarnessError::PathResolutionFailed` ran a probe and retained its plan, so
`failure`, `kind`, `repository_root`, and the ordered `candidates` all project
from the typed probe record. `HarnessError::FileReferenceUnresolvable` failed
before any probe ran (e.g. a syntactically-invalid reference), so only `failure`,
`reference`, and `source_path` project; `kind`, `repository_root`, and
`candidates` stay `null` rather than being invented. The lower-layer legacy path
through `FileReferenceDiagnostic` (the markdown-interpolation arm) continues to
supply the original five fields and reserves the six additions as `null` —
exactly the case the additive catalog was designed to tolerate.

**`failure` is not `kind`.** They are different vocabularies, which is why both
are declared. The historical reason `failure` was reserved as `null` is that
Darkmatter's `FileRefFailure::classify` folds filesystem I/O, permission denial,
and missing context into `NotFound`, so mapping `NotFound → no_match` would
assert "no candidate matched" for a permission error that never probed a
candidate. The shared resolver replaces that lossy classifier with a typed
`PathResolutionFailure` on the probe arm (and a typed `FileReferenceError` →
slug mapping on the no-probe arm), so `failure` now projects from typed data —
never back-derived from `kind` or parsed out of `Display`.

## The lossy-boundary audit

The guards live in `cli/tests/error_guards.rs`, backed by a `syn` reader over
`lib/src`, `cli/src`, and `contract/src`. They run under `just test` and
`just lint-transport`.

| Guard | Fails when |
|-------|-----------|
| `no_unallowlisted_typed_error_collapses` | a typed error is flattened to prose — formatted report, `to_string()` `map_err`, prose `reason`/`message` field, pre-return `format!`, log-then-return-another, or a `DiagnosticSnapshot` facet re-erased into a report/prose field |
| `registry_lists_every_diagnostic_impl` | an `impl Diagnostic` is missing from `as_diagnostic`, or vice versa |
| `no_registered_diagnostic_is_reachable_only_through_a_box` | a registered diagnostic sits behind a `Box` in a `#[source]`/`#[from]` field or a `Result<_, Box<T>>` return |
| `detail_projections_write_only_declared_keys` | a `detail` projection writes a key its codes do not declare |
| `from_code_projects_a_catalog_shaped_detail_for_every_registered_code` | a registered code projects a top-level `null` detail |
| `every_diagnostic_in_the_corpus_projects_its_catalog_key_set` | a constructed diagnostic's detail keys ≠ its catalog key set |

### Why a scan and not a grep

The predecessor was a grep keyed on `map_err(|e| …)`. It could not tell an error
binding from an identifier, so it had to choose between missing real defects and
drowning in false ones — 13 of its inventory hits were not lossy at all. The scan
knows two things a grep cannot:

- **Provenance** — a binding is inspected only when the syntax tree proves it is
  an error (a `map_err`/`or_else`/`unwrap_or_else` closure parameter, or an
  `Err(e)` pattern).
- **Retention** — `Foo { message: e.to_string(), source: e }` stringifies `e`
  *and* keeps it. The chain is intact, so it is not a defect. A grep sees only
  the `to_string()`.

### Adding an exception

Both allowlists (`error_guards/transport-allow.toml`,
`error_guards/boxed-diagnostic-allow.toml`) key an entry to an **enclosing
symbol** — not a source line, which would match by text across files — and
require a `tag` and a written `reason`. `tag = "retained"` is permanent (no typed
source exists, or the boundary is downstream of the final render); any other tag
is a burn-down bucket a follow-up spec closes. A stale entry fails its own guard,
so the lists cannot rot.

An unregistered `Diagnostic` is deliberately **not** allowlistable. It is never
an acceptable exception — it is the motivating incident.

### What the guards cannot see

Worth knowing before trusting them. D-13's live defect passed *every* static
guard: the registry did list the type, no typed value was collapsed, and the
headless suite exercised a different route. The error entered a `Report` as a
`Box`, and only a real terminal driving the second proxy route end-to-end found
it. That is the argument for the L2 render captures
(`cli/tests/level2_typed_error_render_capture.rs`) existing at all.
