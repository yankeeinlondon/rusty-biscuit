# Integrated Design — Real Errors

> The proposed design for replacing dense, mechanism-first error messages across
> **Darkmatter** and **Claudine** with cause-driven, author-facing diagnostics.
> Synthesizes [`design-1.md`](./design-1.md) and [`design-2.md`](./design-2.md),
> grounded in [`error-patterns.md`](./error-patterns.md) and the target in
> [`spec.md`](./spec.md). Every code claim below was verified against the tree.
>
> Companions: [`design-transcript.md`](./design-transcript.md) records *why* each piece
> was kept, dropped, or invented; [`error-structure.md`](./error-structure.md) designs the
> *handleability* axis (classifying errors so callers/authors can react to them);
> [`error-catalog.md`](./error-catalog.md) is the **ratified, locked** contract — the final
> facet enums, dotted-code list, and `detail` schemas. Their impacts are folded into this
> document (§2 principle 6, §3, §5, §5.5, §9, §13).

---

## 1. The one-sentence thesis

**The cause-driven renderer the spec asks for already exists; the bug is that typed
causes are flattened to `String` before they reach it — and that same flattening also
silently drives a control-flow decision. So the work is to let typed causes *survive*,
not to build a renderer.**

This reframing (the strongest idea in Design 2, verified below) sets the whole shape:
type the bottom, preserve across boundaries, add three small render affordances. We do
**not** adopt `miette` or any new diagnostic framework.

### Verified facts the design rests on

| Claim | Evidence |
|-------|----------|
| Renderer is shared by both CLIs | `darkmatter::markdown::errors::as_block_error` walked by `darkmatter/cli/src/main.rs` *and* `claudine/cli/src/output/error_walker.rs:42` (`deepest_block_error`). `biscuit_terminal::errors::as_block_error` is a no-op; Darkmatter's registry is the real one. |
| Engine is stringly-typed end to end | `functions.rs` — ~80 functions return `Result<Value, String>`; `resolve_arg:931 -> Result<Option<PathBuf>, String>`; `frontmatter_fn:1498`. |
| File-ref failure is a *repeated* shape | identical `"invalid file path"` `format!` at `resolve_arg:931/934/940/945`, `absolute():958`, `relative():1024`, `load_markdown:1481/1491`. |
| The catch-all wrapper | `MarkdownError::Transform(String)` (`types.rs:65`) → `transform_block` (`types.rs:217`) = one headline + one dead hint for every transform failure. |
| **String boundary is also a control-flow boundary** | `rewrite.rs:20-21`: `fn is_fatal_eval_error(message) -> bool { message.starts_with(UNKNOWN_FUNCTION_PREFIX) }`, used at `:120` to decide abort-vs-warn. |
| Suggest can't take filenames as-is | `suggest<D: Described>` (`catalog/mod.rs:70`); `Described::key() -> &'static str` (`:42`); `levenshtein` is `pub` (`:112`). |

The control-flow fact is the pivotal one: **this is a correctness fix wearing a
cosmetics costume.** Typing the error is not optional polish — it removes a semantic
decision currently made by string-prefix-matching.

---

## 2. Design principles (the contract every layer obeys)

1. **Causes survive typed; wrappers add fields, never prose.** Adding scope (the
   frontmatter key, the expression) means *setting a field*, never `format!("{prefix}: {msg}")`.
2. **The renderer is the sole display decision-maker.** Headline, hint, links, excerpt
   are derived from the *deepest typed cause*, never from the outer wrapper or call site.
3. **Darkmatter owns the diagnostic substrate.** Types, the focused excerpt, and file
   suggestions live in Darkmatter so the `md` CLI and `claudine` CLI both inherit them.
   Claudine only *preserves and transports* (settled — not an open question).
4. **Expensive help is computed at render time, never at throw time.** Sibling listings
   and did-you-mean run only when a block is actually rendered; the hot eval loop stays cold.
5. **Behavior changes are gated by characterization tests, not assumed safe.** Especially
   the fatal-vs-warn decision (§5).
6. **One taxonomy serves rendering *and* handling.** The same typed error that renders is
   the source of truth for *classification* (§5.5). Handlers bind to a projection of the
   typed error (`err.*`), never to a parallel string-matched layer — building one would
   recreate the exact control-flow-by-string bug (`is_fatal_eval_error`) this effort exists
   to remove. See [`error-structure.md`](./error-structure.md).

---

## 3. The type model — two layers, not three

This is where the integrated design **chooses between** the two proposals rather than
merging them. Design 1 proposes three enums (`ExpressionError` + `ExpressionCause` +
`InterpolationError`); Design 2 proposes one (`ExpressionError`) folded into
`MarkdownError::Interpolation`. We take a middle path justified by Rule 2 (Simplicity
First): **two layers total**, keeping Design 1's *reusable file-ref struct* and its
*catch-all for incremental migration*, but dropping its redundant where/what bifurcation.

### Layer A — `ExpressionError` (the *what*): flat, in Darkmatter

```rust
// darkmatter/lib/src/markdown/compose/expression/error.rs  (new)
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExpressionError {
    #[error("invalid file path: {}", .0.reference)]
    FileReference(FileReferenceDiagnostic),          // reusable — see below

    #[error("unknown function: {name}")]
    UnknownFunction { name: String },                // drives fatality (§5)

    #[error("{function}() expects {expected}, got {actual}")]
    Arity { function: &'static str, expected: ArityBound, actual: usize },

    #[error("{function}() argument {index}: expected {expected}, got {actual_type}")]
    ArgType { function: &'static str, index: usize, expected: &'static str, actual_type: &'static str },

    #[error("parse error: {0}")]
    Parse(String),                                   // last-mile; parser stays string-typed (§4)

    /// Migration catch-all for the long tail of pure builtins not yet individually typed.
    /// Carries the function name so it is never *less* informative than today's string.
    #[error("{function}(): {message}")]
    Other { function: &'static str, message: String },
}
```

Why this shape:

- **Reusable `FileReferenceDiagnostic` (kept from Design 1).** The verified evidence
  shows the *same* file-resolution failure in `frontmatter()`, `absolute()`, `relative()`,
  and `load_markdown()`. A reusable struct (vs. inlining fields into one enum variant, as
  Design 2 did) means all four sites — and future ones (`include`, `transclude`) — get the
  full diagnostic, links, and did-you-mean for free.

  ```rust
  #[derive(Debug, Clone)]
  pub struct FileReferenceDiagnostic {
      pub function: &'static str,        // "frontmatter" | "absolute" | …
      pub reference: String,             // raw arg, e.g. "features/…/spec.md"
      pub kind: FileRefFailure,          // P11 — the distinction code already has, today thrown away
      pub base_dir: PathBuf,             // resolve_arg ALREADY holds ctx.base_dir (P6)
      pub fallback_dir: Option<PathBuf>, // …and ctx.file_ref_fallback_dir
      pub source: biscuit_file::FileReferenceError,
  }

  #[derive(Debug, Clone)]
  pub enum FileRefFailure { Malformed, NotFound, FoundElsewhere, RemoteNotEnabled }
  ```
  `resolve_arg` (`functions.rs:931`) already distinguishes `Err` (malformed) from
  `Ok(None)` (valid-but-absent) and already has `base_dir`/`fallback_dir` in scope — it
  currently `format!`s them away. The typed variant simply *keeps what it already has*.
  No new plumbing for P6/P11.

  **Dual role (handleability, §5.5):** `FileReferenceDiagnostic` is also the `detail`
  payload for the diagnostic code `composition.invalid_file_reference`. Its fields project
  directly to `err.detail.reference`/`.kind`/`.suggestions` — the render fields and the
  handle fields are *the same fields*, captured once.

- **`Other` catch-all (kept from Design 1, missing in Design 2).** Verified: ~80 builtins
  return `Result<Value, String>`. Typing all of them in one change is a giant, risky diff.
  `Other { function, message }` lets the filesystem path (the spec's actual problem) become
  fully typed immediately while the pure-function long tail migrates opportunistically into
  `Arity`/`ArgType` — and is *never less informative* than today because it keeps the
  function name. This is the pragmatic seam that makes L0 shippable as a pure refactor.

- **Flat, not `Error`-wraps-`Cause` (dropped from Design 1).** Design 1's
  `ExpressionError::Evaluation { source: Box<ExpressionCause> }` is an extra indirection
  that carries no field the flat enum can't. Dropping it removes a `Box` and a match arm
  with no information loss.

### Layer B — scope, on `MarkdownError` (the *where*)

```rust
// darkmatter/lib/src/markdown/types.rs
#[error("interpolation failed")]
Interpolation {
    key: Option<String>,          // frontmatter key — was prose in key_scoped_error
    expression: String,           // the {{ … }} span text
    source: SourceRef,            // §6 — on-disk file OR effective (late-binding) value
    #[source] cause: ExpressionError,
},
```

This is Design 2's single-scope approach (simpler than Design 1's separate
`InterpolationError` enum with `FrontmatterKey`/`Body` variants). The frontmatter-vs-body
distinction Design 1 modeled with variants becomes `key: Option<String>` (`Some` =
frontmatter, `None` = body) — same information, one fewer type. `key_scoped_error`
(`frontmatter_interpolation.rs:216`) stops prepending prose and *sets the `key` field*.

`Transform(String)` is **not deleted in one commit** — it remains for genuinely opaque
internal failures while the interpolation/file-reference path (the spec's example) moves
to `Interpolation`. Retiring it fully is a later cleanup, not a prerequisite.

---

## 4. Engine-typing scope (the answer, with rationale)

Adopt **Design 2's scoping decision**: type **to the function-dispatch boundary** —
`evaluate`, `evaluate_function`, the `require_*` helpers, the filesystem builtins
(`resolve_arg`, `load_markdown`, `frontmatter_fn`, `absolute`, `relative`), and the
`EvalResult::Error` carrier in the evaluator. **Keep the recursive-descent parser
string-typed** behind the single `Parse(String)` variant.

Rationale: the parser's errors are syntactic and rarely the *author's* confusion in the
observed failures; the filesystem/function path is where every dense, misdirected message
in the catalog comes from. This bounds the diff to the layer that pays. The `Other`
catch-all (§3) covers the pure-function tail so the boundary is typed *everywhere* even
before every function is individually classified.

---

## 5. The correctness gate — fatality must not drift

**This is the part neither original design treated as a hard gate, and it is the single
most important rigor in this plan.** `is_fatal_eval_error` (`rewrite.rs:20`) currently
decides abort-vs-warn by `message.starts_with(UNKNOWN_FUNCTION_PREFIX)`. Replacing it with
`cause.is_authoring_fatal()` *changes a semantic contract that is currently encoded in
string text*.

Mandatory sequence (do not reorder):

1. **Characterize first.** Before any typing, add a characterization test matrix that
   pins the *current* fatal/warn outcome for:
   `{ unknown-function, missing-file, malformed-path, arity, arg-type, parse }`
   × `{ fail_fast, lenient }`
   × `{ frontmatter-whole-value span, body interpolation }`.
   Today only `unknown-function` is fatal in lenient mode; everything else (incl.
   missing-file) is swallowed to a `ComposeWarning`. Lock that green.
2. **Type the engine** (L0) preserving `Display` byte-for-byte so the matrix stays green —
   proving the refactor is behavior-neutral.
3. **Re-express fatality as a checked `match`:** `ExpressionError::is_authoring_fatal()`
   returns `true` only for the variants the matrix says are fatal today (`UnknownFunction`).
   The matrix re-run proves no drift.

**Fatality is a projection of `disposition` (§5.5), not a bespoke predicate.** "Fatal in
lenient compose mode" ≡ "this error's `disposition` halts composition." `UnknownFunction`
is `Correctable`+`origin=Author` and halts; missing-file is *also* `Correctable`+`Author`
but warns — so the matrix above is literally measuring two same-disposition errors with
different halting behavior, which is exactly the drift to pin. Define
`is_authoring_fatal()` *in terms of* the disposition facet rather than alongside it, so the
correctness gate and the handling taxonomy can never disagree.

**Product decision to surface, not silently make:** *should a missing-file reference be
fatal in lenient (non-`fail_fast`) body interpolation?* Today it is a warning. Promoting it
to fatal is defensible (a missing file is almost always a real mistake) but it is a
*behavior change with a blast radius*, not a refactor. Recommendation: **preserve current
behavior in the typing phase; raise the promotion as its own deliberate, separately-tested
phase.** Flag for Ken's call.

---

## 5.5. Handleability — errors are classified, not only rendered

Full treatment in [`error-structure.md`](./error-structure.md); the impacts on *this*
design are:

**A `Diagnostic` supertrait of `BlockError`.** Every handleable error gains, alongside
`status_block()`, five classification facets:

```rust
pub trait Diagnostic: BlockError {
    fn category(&self) -> Category;        // closed enum — coarse domain (unifies SemanticErrorKind ∪ BadgeCategory)
    fn code(&self) -> &'static str;        // stable dotted id, e.g. "composition.invalid_file_reference" — API contract
    fn disposition(&self) -> Disposition;  // Transient | Throttled | Correctable | NeedsInput | Unrecoverable
    fn origin(&self) -> Origin;            // Provider | Author | Caller | Environment | Internal
    fn detail(&self) -> ErrorDetail;       // typed instance payload → err.detail.*
    fn severity(&self) -> Severity { /* default from disposition */ }
}
```

These are *not* a new system: they unify the three partial taxonomies already in the tree
(`SemanticErrorKind`, `BadgeCategory`+`BadgeSeverity`, `RateLimitInfo`) and expose them
uniformly. The throttle-timing data Ken wants (*"when the cap lifts"*) already exists as
`RateLimitInfo.reset_at` — it just needs surfacing on `err.detail`. **The concrete enum
values, the full dotted-code list, and each code's `detail` schema are ratified and locked
in [`error-catalog.md`](./error-catalog.md)** (12 categories, 5 dispositions, 5 origins,
3 severities, ~38 codes); this section is the trait shape, that file is the contract.

**Same chain, one walk.** Classification delegates through transparent wrappers to the
meaningful cause by the *identical* rule as rendering (§9). So the deepest-cause the
renderer picks is the same error a handler binds to — `Diagnostic: BlockError` makes this
structural, not a second registry.

**Impacts already absorbed elsewhere in this doc:**
- `ExpressionError`/`FileReferenceDiagnostic` (§3) double as the `detail` payload for
  `composition.*` codes — render fields = handle fields, captured once.
- `is_authoring_fatal()` (§5) becomes a projection of `disposition`.

**New constraint this introduces: codes are a public contract.** Unlike `Display` strings
(free to change), `category`/`code`/`disposition`/`origin` values and `detail` field names
become versioned API the moment an author writes `when: err.code == "…"`. This needs a
single-source registry (modeled on the `Described` catalog) and additive-only evolution —
a discipline the render-only work did not require. The boundary lint (§10) extends to also
flag a new error variant that does not implement `Diagnostic`.

---

## 6. Source model for excerpts — designing away the hardest corner

Design 2 honestly flagged that late-binding lifecycle interpolation (DM2 event-time
resolution) breaks the "excerpt a file on disk" assumption but left the fallback only
sketched. We promote it to a designed type:

```rust
#[derive(Debug, Clone)]
pub enum SourceRef {
    /// Compose-time: the error maps to a real frontmatter region in a file.
    OnDisk(SourceContext),
    /// Late-binding (DM2 event-time): no stable on-disk locus; show the resolved text.
    Effective { rendered: String, origin_key: Option<String> },
}
```

The focused-excerpt renderer branches on this: `OnDisk` → a real, line-numbered YAML
slice; `Effective` → render the resolved value/expression with its origin key, not a file
slice. This turns "may need a fallback" into a total function and keeps the lifecycle path
honest instead of crashing or fabricating line numbers.

---

## 7. The focused-context excerpt (P5) — Darkmatter-owned

The one place existing render assets genuinely fall short. Today: `transform_block` shows
**no** YAML; `SourceContext::excerpt_prose` and Claudine's `FrontmatterExcerpt` show a
**contiguous** slice or the **whole** block. None can show "just `spec`, `iteration`, and
their `$schema:` parent."

Add a **non-contiguous, structure-aware** excerpt to `SourceContext` (Design 2's idea,
Design 1's `FrontmatterFocus` shape):

```rust
// SourceContext
pub fn focused_yaml_excerpt(&self, keys: &[YamlKeyPath], term: &Terminal) -> Prose;
```

- Resolve each `YamlKeyPath` (dotted, indentation-aware — `FrontmatterExcerpt::locate_property_line`
  already does this) to its line range, **union** with structural ancestors (the `$schema:`
  parent), render gutter-numbered + syntax-highlighted with elision markers (`…`) between
  non-adjacent regions.
- The involved keys come *from the typed error*: `Interpolation.key` plus the file-reference
  arguments the expression touched (e.g. `spec`). This is the data only a typed cause can
  supply — closing the P5↔P2 loop.
- **Fallback:** when keys can't be confidently sliced (aliases, complex sequences), fall
  back to the existing contiguous/whole-block excerpt rather than guessing.

**Convergence (flagged migration, not green-field):** Claudine's `FrontmatterExcerpt` +
`WithFrontmatter`/`enrich_frontmatter` boundary should migrate onto
`SourceContext::focused_yaml_excerpt`, **preserving** its TTY-gating, `FORCE_COLOR=1`
override, and non-TTY ANSI-stripping. This is high-value consolidation with real regression
risk — done last (§10).

---

## 8. File did-you-mean + auto-linked paths (P7, P8)

**Did-you-mean for files.** `suggest` can't be reused as-is (`Described::key()` is
`&'static str`; filenames are runtime). Add a thin sibling that reuses the *same quality
gate*:

```rust
// darkmatter/lib/src/catalog/mod.rs
pub fn suggest_strings(candidates: &[String], key: &str, max: usize) -> Vec<&str>;
//  same max(2, len/3) threshold as `suggest`, no `Described` bound
```

Candidates are listed **lazily at render time** from the expected parent directory:
non-recursive, capped (siblings only, first ~1–2k entries), ranked by edit distance against
the leaf name. Never computed during evaluation (Principle 4). For
`FileRefFailure::NotFound` whose parent dir is missing, suggest from the nearest existing
ancestor instead.

**Open calibration (measure, don't guess):** leaf-name-only vs. full-relative-path
matching, and whether the threshold needs path-specific tuning (`spec.md` vs `specs.md` vs
dated dirs are all close). Start leaf-name-only; tune against real repos.

**Auto-linking.** Make "path → OSC8 link" a property of the *field type*, not the call
site. The cause-driven block reads path-typed fields (`reference`, `base_dir`) and links
them via `SourceContext::linked_path_prose` when building the `StatusBlock`. Claudine's ~15
manual `render_file_link()` sites collapse into the shared builder, so new variants get
links for free and can't forget (kills P8 by construction).

---

## 9. Rendering contract — how the cause and scope combine

Design 2 flagged (challenge #6) that pure delegation loses the wrapper's scope. We specify
the merge precisely, so it isn't left ambiguous:

For `MarkdownError::Interpolation { key, expression, source, cause }` whose `cause` is a
renderable `ExpressionError`:

- **Headline** ← the *cause* (`invalid file path`). The mechanism word "transform" never
  appears.
- **Hint** ← the *cause* (missing file → "Did you mean `…`?"; malformed → "Fix the path
  syntax."; remote-disabled → "Enable remote reads or use a local path."). Derived from
  cause fields, never a per-`*_block` constant (kills P4).
- **Body** ← the *wrapper* contributes scope: "while evaluating `{{ … }}` for property
  `iteration`", the OSC8-linked prompt file, and the focused excerpt from `source`.
- **Links** ← auto, from path-typed fields (§8).

So the block is *composed from cause + scope*, not pure-forwarded. This is a small,
explicit change to the delegation contract in `as_block_error`/`status_block` and gets its
own test so a generic wrapper never shadows the useful leaf.

**The same delegation governs classification (§5.5).** A transparent wrapper forwards its
cause's `category`/`code`/`disposition`/`origin`/`detail`; a layer that deliberately
classifies (e.g. the provider layer deciding "this is `cap.plan_limit`") owns its facets
and does not delegate. One cause-chain walk serves both render and handle.

No new top-level renderer is introduced; `BlockError` → `as_block_error` → deepest-block
walk stays the shared boundary, and it *already converges* the two CLIs (verified). Once
causes survive, both improve by construction (kills P10).

---

## 10. Cross-crate transport discipline (P9) — one rule, lint-enforced

**Rule: a `MarkdownError` (or any `BlockError`) crossing into Claudine travels by
`#[from]`/`#[source]`, never through `.to_string()`/`format!("{e}")`.**

The good bridges already exist (`ComposeFailed(#[source] MarkdownError)`,
`SystemPromptComposition(#[from] MarkdownError)`). Convert the flattening sites:

| Site | Today | Becomes |
|------|-------|---------|
| `resolve.rs:42,47` | `InvalidReference(format!("{file_ref}: {e}"))` | `Reference { file_ref, #[source] FileReferenceError }` |
| `resolve.rs:62` | `MarkdownLoad(format!("{path}: {e}"))` | `MarkdownLoad { path, #[source] io::Error }` |
| `sequence.rs:106,115,220` | `SequenceExternalLoad(format!(…))` | `SequenceExternalLoad { raw, #[source] … }` |
| `closure.rs:147` | `AtomicWriteFailed(e.to_string())` | `AtomicWriteFailed { path, #[source] … }` |
| `lifecycle_control.rs:245` | `.map_err(\|e\| e.to_string())` | propagate the typed error |

**Enforcement.** A repo-level review guard (grep-based test in CI) flags two smells at the
DM↔Claudine boundary: new `Variant(String)` error variants, and `map_err(|e| … e.to_string())`
on a `BlockError` value. Makes "treat as code smell" executable so the boundary can't
re-collapse.

---

## 11. The spec's error, end to end

| Stage | Today | Under this design |
|-------|-------|-------------------|
| `resolve_arg` fails | `Err("invalid file path \"…/spec.md\": …")` | `Err(ExpressionError::FileReference(FileReferenceDiagnostic{ kind: NotFound, base_dir, … }))` |
| `frontmatter_fn` | re-`format!`s | propagates typed error |
| evaluator | `EvalResult::Error { message }` | typed sibling carries `ExpressionError` |
| interpolation rewrite | `Transform("Interpolation evaluation failed for '…': …")` | `MarkdownError::Interpolation { key: Some("iteration"), expression, source: OnDisk(ctx), cause: FileReference{…} }` |
| key scoping | prepends `"frontmatter key 'iteration': "` | sets `key` field |
| fatality | `is_fatal_eval_error` string-prefix | `cause.is_authoring_fatal()` checked `match` (§5) |
| Claudine bridge | re-flattens | `#[source]` — intact |
| render | "transform failed" + dead hint | headline **"invalid file path"**; focused excerpt of `$schema`/`spec`/`iteration`; hint **"Did you mean `features/…/spec.md`?"**; OSC8 link to prompt file |

Exactly the spec's target, produced from the typed cause with **no message-string parsing
anywhere**.

---

## 12. Pattern coverage

| Pattern | Layer | Mechanism |
|--------|-------|-----------|
| P1 mechanism headline | §3B, §9 | cause sets headline; "transform" never reaches the user |
| P2 stringly-typed boundary | **§3A, §4** | `Result<Value, ExpressionError>` + `Other` catch-all |
| P3 nesting prose not data | §3B | `key_scoped_error` sets a field |
| P4 generic hint | §9 | hint from typed `cause` fields |
| P5 all-or-nothing context | **§7** | `focused_yaml_excerpt(keys)` non-contiguous union |
| P6 locus not captured | §3A | keep `base_dir`/`fallback_dir` already in scope |
| P7 no file did-you-mean | §8 | `suggest_strings` + lazy sibling listing |
| P8 manual links | §8 | path-typed fields auto-linked by shared builder |
| P9 lossy cross-crate | §10 | `#[from]`/`#[source]`, lint-enforced |
| P10 two render boundaries | §9 | already shared; converges once causes survive |
| P11 absent vs malformed | §3A | `FileRefFailure` discriminant |
| **(correctness)** fatal-vs-warn by string | **§5** | `is_authoring_fatal()` gated by characterization matrix |

---

## 13. Phasing (by leverage; each independently shippable)

1. **Characterization gate (§5).** Pin current fatal/warn matrix green. *Prerequisite to
   touching the engine.*
2. **L0 pilot — typed `ExpressionError` for the file path, `Display` byte-for-byte.** Type
   `resolve_arg`/`load_markdown`/`frontmatter_fn`/`absolute`/`relative` + the evaluator
   carrier, with `Other` catch-all for the rest. Pure refactor; matrix + snapshots stay green.
3. **L1 — `MarkdownError::Interpolation` + cause-composed block (§9).** First *visible*
   win: the spec's error gets its real headline and typed hint.
4. **File did-you-mean + auto-linked fields (§8).** Highest user-visible payoff per line.
5. **`SourceContext::focused_yaml_excerpt` (§7).** The P5 fix; `md` CLI benefits too.
6. **Boundary transport cleanup + anti-pattern lint (§10).** Lock the boundary.
7. **`Diagnostic` trait + `err.*` projection + taxonomy unification (§5.5).** Implement the
   facets on the typed errors (folding in `SemanticErrorKind`/`BadgeCategory`/
   `RateLimitInfo`), the code registry, and the introspection surface (`claudine errors`).
   Needs the typed substrate from phases 2–3, so it lands here. The *enum/code design is
   already ratified and locked* in [`error-catalog.md`](./error-catalog.md) — implement to
   that contract; evolve it additively only.
8. **Excerpt convergence + `SourceRef::Effective` late-binding path (§6, §7).** Hardest
   corners, last, on a typed substrate.

Phases 1–3 resolve the literal spec example; 4–6 generalize the *rendering* to the whole
class; 7 adds *handleability*; 8 closes the corners.

**Downstream of phase 7 (handler-side, out of scope here):** consuming the facets in
recovery requires a control-action **`when` dimension** — `defer`/`retry`/`resume` accepting
`until: <timestamp>` (absolute) alongside today's relative `delay`, so a `throttled` error
can "resume at `err.reset_at`." This is a handler/rendezvous concern, not error structure;
the structural prerequisites it depends on (a serializable absolute `reset_at`; a `detail`
schema rich enough to author a corrective `resume` message now or hand it to a human later)
are specified in [`error-structure.md`](./error-structure.md) §11.

---

## 14. Risks & open questions (carried forward honestly)

- **Fatality promotion (§5)** — product decision for Ken; default to preserving current
  behavior.
- **`Result` size regression** — `ExpressionError` carries `PathBuf`/`String`; if the
  success path regresses, `Box<ExpressionError>` in the `Err` arm. Measure, don't assume.
- **Did-you-mean calibration (§8)** — leaf-only vs full-path, threshold tuning; measure on
  real repos.
- **Excerpt convergence (§7)** — must preserve Claudine's TTY/`FORCE_COLOR`/ANSI-strip
  behavior; real regression risk.
- **Parser typing** — deferred behind `Parse(String)`; revisit only if parser errors prove
  a meaningful share of author confusion.
- **Taxonomy ratification (§5.5)** — ✅ **resolved.** The `category`/`disposition`/`origin`/
  `severity` enums, the dotted-code list, and each code's `detail` schema are ratified and
  locked in [`error-catalog.md`](./error-catalog.md) (all 8 decisions confirmed); evolution
  is additive-only. Detail representation is recommended serde→Value (error-structure §2.3),
  with the typed-enum alternative noted but not chosen.

---

## 15. Success criteria

- The reference failure renders with a root-cause headline naming the invalid file
  reference, in **both** `md compose` and `claudine compose`.
- The error names the receiving frontmatter key, links the prompt file (OSC8 when capable),
  shows a *focused* excerpt (`$schema`/`spec`/`iteration`), and suggests likely files.
- Fatal-vs-warn behavior is provably unchanged by the typing refactor (characterization
  matrix green).
- No new string-only lower-layer error variants; the boundary lint passes.
- The win generalizes: `absolute()`/`relative()`/`load_markdown` failures inherit the same
  diagnostic from the shared `FileReferenceDiagnostic`.
- Each handleable error exposes stable `category`/`code`/`disposition`/`origin`/`detail`
  facets via `Diagnostic`, projected to `err.*`, so a handler can tap a pattern
  (`err.disposition == "throttled"`), target a code (`err.code == "cap.plan_limit"`), or
  target an instance (`err.detail.property == "status"`) — with no string-message parsing.
