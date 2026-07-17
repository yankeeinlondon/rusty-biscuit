---
created: 2026-07-16
phase: 3
status: ratified
---

# Decisions — End-to-End Typed Error Propagation

Rulings taken during execution of [`plan.md`](./plan.md). Each records what was
decided, the authority for the decision, and the condition that would reverse it.

---

## D-1 — Cross-crate diagnostic identity lives in Claudine (Option A)

**Decision.** Claudine owns semantic adapters. Every lower-layer error
(Darkmatter, biscuit-file, `std::io`) that crosses into Claudine is retained as
`#[source]` by a Claudine diagnostic wrapper. That wrapper either owns its
facets and `BlockError` rendering (a *semantic* boundary that deliberately
reclassifies the operation) or transparently delegates both to its cause (a
*transparent* wrapper). Effective-diagnostic selection consults the Claudine
registry before falling back to Darkmatter's `as_block_error` registry for
lower-layer causes.

**Authority.** [`spec.md`](./spec.md) §"Open Questions" → "Where should
cross-crate diagnostic identity live?". The spec names Option A as its own
recommendation, states plainly that "D2–D10 assume Option A", and records that
Option B "should be ratified as a separate cross-package contract before
implementation". Option C is explicitly excluded by the spec ("Option C should
not be used").

**Consequences accepted.**

- Every lower-layer ingress into Claudine must be wrapped. This is the cost that
  buys the dependency direction.
- Direct Darkmatter callers *outside* Claudine do not gain Claudine facets. This
  is acceptable: they already render through Darkmatter's own `BlockError`.
- Careless adapters could duplicate detail across the wrapper and its cause. The
  catalog-parity test (Phase 6) is what prevents this, not reviewer diligence.

**Reversal condition (Option B exit).** If maintainers prefer Option B — moving a
provider-neutral diagnostic facet trait down beside `BlockError` in
`biscuit-terminal` and implementing it directly in Darkmatter and biscuit-file —
then this plan **stops at Phase 1**. Option B invalidates D2's crate-local
registry, turns a Claudine transport fix into a cross-area public API change in
`biscuit-terminal`, and forces a ruling on who owns the category/code catalogs.
That is a separate specification, not a phase of this one. Nothing in Phases 2–8
should be started while Option B is under consideration, because Phase 2's
registry is precisely the artifact Option B deletes.

---

## D-2 — The two proxy routes do not share a resolver (discovered, not decided)

This is recorded here because it materially changes what Phase 7's cross-route
parity criterion can assert, and it was not visible from the spec.

**Finding.** Spec Acceptance Criterion 5 requires the same proxy failure to have
"identical diagnostic identity, headline, hint, and available typed resolution
detail regardless of which supported lifecycle route initiated the handoff".
Characterization against `HEAD` shows the two routes do not currently fail in the
same *place*, let alone with the same identity:

| Route | Resolver | Where it fails on a missing target |
|---|---|---|
| `initialize` proxy | `claudine::composition::resolve_proxy_target` | At resolution — the resolver probes existence and returns `HarnessError::PathResolutionFailed` |
| terminal/recovery proxy | `claudine::harness::resolve_harness_path` | **Not at resolution.** The path resolves successfully; the run announces "flow control redirected to …" and then fails later in pre-flight with an `io::Error` read failure |

Their observed `HEAD` surfaces are therefore unrelated:

```text
# initialize route
Error: lifecycle initialize proxy: path resolution failed for "…": proxy target does not exist: …

# failure route
Error: pre-flight shell approval failed: proxy target pre-flight: failed to read '…': No such file or directory (os error 2)
```

**Implication for Phase 7.** Typing the errors is necessary but *not sufficient*
for AC5. Two routes that fail at different stages against different resolvers
cannot be made to agree on code/headline/hint by wrapping alone — the terminal
route would still be reporting a pre-flight read, not a resolution miss.

**Ruling.** Converging the two routes onto one resolver is a **routing change**,
and D10 requires that any routing change discovered mid-audit is split into a
separate spec rather than fixed here. Phase 4/5 must therefore preserve both
routes' current failure *stages*. If Phase 7 finds AC5 unsatisfiable without
converging the resolvers, that is the trigger to raise the separate spec — not a
license to change routing inside this feature.

**Note.** The downstream [`2026-07-13-file-resolution`](../2026-07-13-file-resolution/spec.md)
feature unifies file-reference resolution and is the natural home for this
convergence. Sequencing it after this feature keeps the dependency edge acyclic,
exactly as this spec's reader's note requires.

---

## D-3 — A wrapper is `Transparent` only if it delegates code, detail, *and* rendering

**Decision.** `DiagnosticRole::Transparent` is assigned from the projections a
variant actually forwards, never from what it wraps. Exactly two variants
qualify today — `CompositionError::WithFrontmatter` and
`CompositionError::LifecycleEvaluationAlreadyEmitted` — which is the same pair
the `BlockError` dispatcher has already called "transparent wrappers" since the
real-errors work. Everything else defaults to `Semantic`.

The load-bearing consequence is for the variants that carry a **typed Darkmatter
cause** (`CompositionError::ComposeFailed`, `FrontmatterParse`,
`InlineHashMalformed`, `PreFlightDiscoveryFailed`, `ShellExpansionFailed`,
`ClaudineError::SystemPromptComposition`). Their doc comments say they exist "so
the CLI's top-level walker renders Darkmatter's rich block", which reads like
transparency — but they are **`Semantic`**. Under Option A a Darkmatter cause
supplies no facets, so a wrapper that delegated to it would leave the failure
unclassified. Owning the code and *rendering through* the cause is not a
contradiction: `ShellExpansionFailed` already builds its `status_block` from
`error.status_block(term)` while owning `composition.shell_expansion`. Role is
about who owns the identity; where the pixels come from is `status_block`'s
business.

**Authority.** [`spec.md`](./spec.md) §D4 — "**transparent** — it delegates both
rendering and facets to its source" — read against §"Open Questions" Option A,
which is what makes a Darkmatter cause facet-less.

**⚠️ Hazard this hands to Phase 5.** Today the walker cannot see
`ClaudineError`/`HarnessError` at all, and `CompositionError::ComposeFailed` &
friends lose to the deepest Darkmatter block. Once Phase 5 points the walker at
`select_effective_diagnostic`, those `Semantic` wrappers **win** — and their
`status_block` currently falls through to the flat catch-all arm
(`render/provider.rs`), which would replace Darkmatter's rich block (path, line,
excerpt) with one line of `Display` text. **Phase 5 must make each of those
variants' `status_block` delegate to its inner cause's block, the way
`ShellExpansionFailed` does.** The L2 capture suites
(`level2_invalid_file_reference_capture`, `level2_malformed_frontmatter_capture`)
are the detectors.

**Reversal condition.** If a Claudine diagnostic ever wraps *another Claudine
diagnostic* purely to add context, that wrapper is `Transparent` and this ruling
does not constrain it.

---

## D-4 — Two supporting seams the spec did not name

Both were forced by the code, and both are additive.

**`Diagnostic::diagnostic_source`.** The two transparent wrappers hold `inner:
Box<CompositionError>` with **no `#[source]`** — promoting it would make
`color_eyre`'s cause-chain fallback print the same `Display` text twice. So
`Error::source()` returns `None` and the selection walk would stop *at* the
wrapper, never reaching the error it delegates to. The trait therefore exposes
`diagnostic_source()`, defaulting to `Error::source`, which those two override.
This is also the primitive D9's "one-level next registered cause" needs.

**`select_with(error, discover)`.** The registry is a closed downcast list over
concrete types, and every Claudine diagnostic that can *terminate* a chain today
is `Semantic`. The deepest-transparent rule and "a guard keeps the best
candidate" are therefore unreachable through the public `as_diagnostic` — not
untrue, just not constructible from shipped types. The walk takes its discovery
function as a parameter so tests can supply a probe registry and exercise the
rules the spec ratified. Production always passes `as_diagnostic`.

---

## D-5 — `failure` stays `null`; it is not back-derived from `kind`

**Decision.** The new `composition.invalid_file_reference` `failure` field
projects `null` for every error this feature can construct, even though a
`FileRefFailure` value is sitting right there in the `FileReferenceDiagnostic`.

**Authority.** [`spec.md`](./spec.md) §D3 — "fields unavailable from the current
private resolver are `null`, not invented or parsed from `Display`".

**Why it looks available but is not.** Darkmatter's `FileRefFailure::classify`
is a three-arm match with a catch-all: `InvalidSyntax → Malformed`,
`RemoteNotLocal → RemoteNotEnabled`, and **everything else** — filesystem I/O,
permission denial, missing env/git/workspace context, relative-path computation
— `→ NotFound`. Mapping `NotFound → no_match` would therefore assert
"no candidate matched" for a permission error that never probed a candidate.
That is exactly the invented classification D3 forbids, and it is the specific
distinction the file-resolution feature exists to make
(`permission_io` / `missing_context` vs `no_match`).

**Consequence.** `kind` (`not_found`, `malformed`, `found_elsewhere`,
`remote_not_enabled`) and `failure` (`invalid_syntax`, `missing_context`,
`no_match`, `permission_io`, `unsupported_remote`) are *different vocabularies*
and both are declared. `kind` keeps its current values unchanged, which is what
keeps this catalog change additive.

**Reversal condition.** When file-resolution lands a resolver that distinguishes
absence from permission/I/O failure, `failure` is populated from that typed
result — never from `kind`.

---

## D-6 — The snapshot's one-level cause is a distinct type

**Decision.** `DiagnosticSnapshot::cause` is `Option<DiagnosticCause>`, where
`DiagnosticCause` carries no `cause` field of its own — rather than
`Option<Box<DiagnosticSnapshot>>` with a convention that the nested value's
cause is always `None`.

**Authority.** [`spec.md`](./spec.md) §D4 — "It is a one-level projection in v1;
`err.cause.cause` is not exposed."

**Rationale.** A self-referential type would make the v1 boundary a rule
reviewers must remember and tests must police. A separate type makes
`err.cause.cause` *unrepresentable*: the constraint is discharged by the type
checker at every future call site, including ones this feature never sees.
Widening to recursion later is additive if it is ever ratified.

**Consequence accepted.** Seven facet fields are declared twice. That is the
price of the structural guarantee, and the two types are asserted equal
field-for-field by the round-trip suite.

---

## D-7 — `#[source] Box<ConcreteError>` makes the concrete error undowncastable

**Finding (discovered in Phase 4, proven by probe).** A `#[source]` field typed
`Box<E>` publishes **`Box<E>`** to the cause chain, not `E`. thiserror's
`AsDynError` blanket impl applies to `Box<E>` itself (`Box<E: Error>` is itself
`Error`), so `Error::source()` yields `&dyn Error` whose `TypeId` is
`Box<E>` — and `downcast_ref::<E>()` returns `None`. Worse, `Box<E>`'s own
`Error::source()` delegates to `E::source()`, so the walk **skips past `E`
entirely**: `E` is never a chain member at any depth.

```text
CompositionError::AtomicWriteFailed  (#[source] source: Box<ClaudineError>)
  Error::source() -> Some(&dyn Error)
      downcast_ref::<ClaudineError>()       => None      // ← invisible
      downcast_ref::<Box<ClaudineError>>()  => Some(..)   // ← what is actually there
```

**Why it matters.** D2's `as_diagnostic` is a **downcast allowlist over concrete
types**. Any diagnostic reached only through a `#[source] Box<Concrete>` field is
undiscoverable by construction — the registry can list it and the source-parity
test can pass, and it still never resolves at runtime.

**Ruling for this feature.** `CompositionError::InvalidFileReference` carries its
`HarnessError` **unboxed** and boxes its *context* instead
(`context: Box<FileReferenceContext>`). Boxing the context costs nothing;
boxing the source costs discoverability. The unboxed source alone pushed the
variant to ~200 bytes and tripped `clippy::result_large_err` across 135 sites,
which is what forced the context-boxing shape — record it so a future author
does not "simplify" it back.

**Pre-existing instances (NOT fixed here — out of Phase 4's scope).**

| Variant | Field |
|---|---|
| `CompositionError::AtomicWriteFailed` | `#[source] source: Box<ClaudineError>` |
| `CompositionError::ShellExpansionFailed` | `#[source] error: Box<ShellExpansionError>` |
| `MarkdownLoadCause::Parse` | `#[from] Box<MarkdownError>` |

`AtomicWriteFailed` is the load-bearing one: it wraps a **`ClaudineError`**, a
registered Claudine diagnostic, so `io.write_failed`'s inner cause is invisible
to `as_diagnostic` today. **Phase 6's source-parity test cannot catch this** — it
proves the registry lists every `impl Diagnostic`, not that every registered type
is *reachable*. Phase 6 should add a companion check: for each `#[source]` /
`#[from]` field whose type is `Box<T>` where `T` is a registered diagnostic,
fail. Phase 5 should decide whether `AtomicWriteFailed` is worth the same
context-boxing treatment.

---

## D-8 — `escape_prose_path` over-escapes body text

**Finding.** `escape_prose_path` (`composition/error/render/mod.rs`) escapes
`\ < > { "`. The `"` is there because the helper's primary use is interpolating
a path into an `<a href="…">` attribute. But Prose's own body-text escape set
(`Prose::escape_text`) is `< > { * _ [ ] ( ) \` — **no `"`** — so a `"` fed
through `escape_prose_path` into *body* prose renders as a literal `\"`.

This was latent because paths rarely contain quotes. Phase 4 surfaced it:
`HarnessError`'s `Display` quotes the reference (`path resolution failed for
"nope.md"`), and rendering that string through `escape_prose_path` leaked
`\"nope.md\"` into the user-facing block.

**Ruling.** Body text uses `Prose::escape_text`; `escape_prose_path` is for href
attributes and path labels. `InvalidFileReference`'s renderer does this and
`invalid_file_reference_body_does_not_leak_escape_backslashes` locks it.

**Not swept here (Rule 3).** Other renderers pass messages through
`escape_prose_path` (e.g. `lifecycle::status_block`'s `LifecycleInvalid` arm).
They are only safe because their messages happen not to contain `"`. Phase 8's
comment/behavior-drift pass is the natural home for auditing the helper's call
sites, or splitting it into `escape_href` / `escape_body`.
