# Real Errors — Design 2

> A long-term design for replacing dense, mechanism-first error messages across
> **Darkmatter** and **Claudine** with cause-driven, author-facing diagnostics.
> Grounded in the pattern catalog of [`error-patterns.md`](./error-patterns.md) and
> the example in [`spec.md`](./spec.md), but driven by what the code actually is
> today, not by the brainstorm's candidate directions.

---

## 1. Executive summary

The single most important thing this design got from reading the code: **the
cause-driven renderer the spec asks for already exists.** Darkmatter and Claudine
both render errors through one shared abstraction:

- `BlockError` (trait) — `biscuit-terminal/lib/src/errors/block_error.rs:66` — owns
  `status_block(&Terminal) -> StatusBlock`, a `severity()`, and a `block_source()`
  cause hook.
- `StatusBlock` (renderable) — `biscuit-terminal/lib/src/components/status_block.rs`
  — header + multi-`Prose` body + hint + border, projected to the render tree
  (terminal / browser / markdown).
- `SourceContext` — `biscuit-terminal/lib/src/errors/source_context.rs:14` — already
  carries `{ absolute, display, content: Arc<str>, frontmatter: Range }`, and already
  produces an OSC8 file link (`linked_path_prose`) and gutter-marked excerpts
  (`excerpt_prose`).
- `as_block_error()` — `darkmatter/lib/src/markdown/errors/mod.rs:57` — a downcasting
  registry; **both** the `md` CLI (`darkmatter/cli/src/main.rs:75`) and Claudine's
  `error_walker` (`claudine/cli/src/output/error_walker.rs:94`) walk the cause chain
  through it to find the deepest renderable block.
- `levenshtein` + `suggest` + the `Described` catalog trait — `darkmatter/lib/src/catalog/mod.rs:70`
  — a working did-you-mean already wired to ctx-vars, functions, lifecycle verbs.

So P1 (headline), P4 (hints), P8 (links), P10 (two boundaries) are **not** missing
capabilities — they are *structurally solved at the render layer already*. The
`md` CLI and Claudine CLI converge on the same renderer whenever a typed error
reaches it.

The reason the spec's error is still terrible is **upstream of the renderer**, and
it has exactly one root: **typed causes never arrive.** They are flattened to
`String` at two dams:

1. **The expression engine dam (Darkmatter).** Evaluation is `Result<Value, String>`
   (`expression/mod.rs:359`) and the evaluator's own result is
   `EvalResult::Error { message: String, original: String }`
   (`interpolation/evaluator.rs:91`). Everything below — `frontmatter()`,
   `resolve_arg` (`functions.rs:941`) — is pre-flattened to a string *before* it can
   become a typed `MarkdownError`. The interpolation layer then re-wraps that string
   into the catch-all `MarkdownError::Transform(String)` (`rewrite.rs:122`,
   `transform/mod.rs:376`), prefixing more prose (`key_scoped_error`,
   `frontmatter_interpolation.rs:216`).

2. **The cross-crate bridge dam (Claudine).** Where Claudine *could* keep a typed
   `MarkdownError`, ~6 sites re-flatten it into opaque `String` variants
   (`resolve.rs:42,47,62`, `sequence.rs:106,115,220`, `closure.rs:147`,
   `lifecycle_control.rs:245`, `preflight.rs:180`).

**The design is therefore not "build a cause-driven renderer." It is "let typed
causes survive to the renderer that already exists."** That reframing makes the
work tractable: type the engine, stop flattening at boundaries, and add three small
renderer affordances (focused excerpt, file did-you-mean, auto-linked path fields).

A secondary finding sharpens the scope: stringly-typed errors don't only damage
*display*, they damage *control flow*. `is_fatal_eval_error` (`rewrite.rs:20`)
decides whether an eval failure aborts composition or is swallowed into a
`ComposeWarning` **by string-prefix-matching the message** against
`UNKNOWN_FUNCTION_PREFIX`. So the `String` boundary is also a correctness boundary:
a missing-file failure is currently classified as non-fatal-in-lenient-mode purely
because its message text doesn't start with the magic prefix. Typing the error lets
this decision become a real `match`, and lets each cause declare its own fatality.

---

## 2. Design goals (success criteria)

A diagnostic is "good" when, from the typed cause alone, the shared renderer can
produce all four of:

1. **A cause-named headline** — "invalid file path", never "transform failed".
2. **A focused context excerpt** — only the involved keys plus their structural
   parent (`$schema:`), never zero and never a full dump.
3. **A cause-specific, actionable hint** — "Did you mean `…`?", derived from the
   cause's own fields, not from the outer wrapper variant.
4. **Auto-linked paths** — every path-typed field is an OSC8 link because it is a
   path, not because a call site remembered to wrap it.

…and this must hold identically whether the error surfaces through the `md` CLI or
Claudine's CLI. The renderer is the *only* place display decisions are made; every
layer above it only *attaches structure*.

---

## 3. The design, in five layers

The layers map onto the three cross-cutting root causes the brainstorm identified
(string-at-boundary, prose-concatenation, cause-decoupled-rendering), but reorder
them by leverage: type the bottom first, because everything else is downstream.

### Layer 0 — A typed expression-engine error (`ExpressionError`)

Replace `Result<Value, String>` in the expression engine with
`Result<Value, ExpressionError>`. This is the upstream dam and the highest-leverage
change; P2 calls it "the highest-leverage thing to change," and the code confirms it.

```rust
// darkmatter/lib/src/markdown/compose/expression/error.rs  (new)
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExpressionError {
    #[error("invalid file path: {reference}")]
    FileReference {
        reference: String,
        kind: FileRefFailure,          // Malformed | NotFound | FoundElsewhere  (P11)
        base_dir: PathBuf,
        fallback_dir: Option<PathBuf>,
        #[source] source: biscuit_file::FileReferenceError,
    },
    #[error("unknown function: {name}")]
    UnknownFunction { name: String, arity: usize },
    #[error("{function}() expects {expected} argument(s), got {actual}")]
    Arity { function: &'static str, expected: ArityBound, actual: usize },
    #[error("{function}() argument {index}: expected {expected}, got {actual_type}")]
    ArgType { function: &'static str, index: usize, expected: &'static str, actual_type: &'static str },
    #[error("parse error: {0}")]
    Parse(String),            // last-mile only; parser typing is a later phase
    // …binary-op / comparison / arithmetic variants as needed
}

impl ExpressionError {
    /// Type-driven replacement for `is_fatal_eval_error`'s string prefix match.
    pub fn is_authoring_fatal(&self) -> bool { /* UnknownFunction, Arity, … */ }
}
```

Key properties this buys, each traceable to a pattern:

- **P6 (locus captured at source).** `resolve_arg` (`functions.rs:941`) *already has*
  `ctx.base_dir` and `ctx.file_ref_fallback_dir` in scope — today it throws them away
  into a `format!`. The typed variant simply keeps them. No new plumbing.
- **P11 (absent vs malformed vs wrong-dir).** `resolve_arg` already distinguishes
  `Err` (malformed) from `Ok(None)` (valid-but-absent); `FileRefFailure` promotes that
  distinction into the type instead of collapsing it into one "invalid file path".
- **Fatality becomes typed.** `is_fatal_eval_error` (`rewrite.rs:20`) becomes
  `err.is_authoring_fatal()`; the `EvalResult::Error { message, original }`
  (`evaluator.rs:91`) gains a typed sibling carrying `ExpressionError` so the lenient
  vs. fail-fast branch (`rewrite.rs:119-133`) matches on a variant, not a prefix.

**Scope decision (open question in the brainstorm, answered here):** type the engine
*to the function-dispatch boundary* — `evaluate`, `evaluate_function`, the `require_*`
helpers, and the filesystem builtins (`resolve_arg`, `load_markdown`, `frontmatter_fn`).
**Do not** type the recursive-descent parser in the first cut; keep `Parse(String)` as a
single last-mile variant. Rationale: the parser's errors are already syntactic and rarely
the *author's* confusion in the observed failures; the filesystem/function path is where
the dense, misdirected messages come from. This bounds the diff to the layer that pays.

### Layer 1 — `MarkdownError` carries typed causes (retire `Transform(String)`)

`MarkdownError::Transform(String)` (`types.rs:74`) is the catch-all that the engine's
flattened string lands in, and the renderer's `transform_block` (`blocks.rs:239`) is
why every transform failure gets the same headline and the same dead hint. Replace it
with cause-carrying variants:

```rust
// darkmatter/lib/src/markdown/types.rs
#[error("interpolation failed")]
Interpolation {
    key: Option<String>,          // frontmatter key, when scoped (was prose in key_scoped_error)
    expression: String,           // the {{ … }} span text
    span: Option<SourceSpan>,     // for the focused excerpt (Layer 3)
    #[source] cause: ExpressionError,
},
```

- **P1.** `BlockError for MarkdownError` (`types.rs:202`) gains an arm that, for an
  `Interpolation` whose `cause` is a `FileReference`, delegates to the *cause's* block:
  headline = "invalid file path". The mechanism word "transform" never reaches the user.
- **P3.** `key_scoped_error` (`frontmatter_interpolation.rs:216`) stops doing
  `Transform(format!("frontmatter key '{key}': {msg}"))` and instead *sets the `key`
  field*. Scope becomes data, not a sentence fragment.
- **P4.** The hint is generated from the typed `cause` (missing file → did-you-mean),
  not hard-coded per `*_block` function.

`Transform(String)` does not have to vanish in one commit — it can remain for genuinely
opaque internal failures while the interpolation/file path moves to `Interpolation`. But
the *interpolation* path, which is the spec's example, must be fully typed.

### Layer 2 — Cross-crate transport discipline (one rule, lint-enforced)

**Rule: a `MarkdownError` (or any `BlockError`) crossing into Claudine travels by
`#[from]`/`#[source]`, never through `.to_string()`/`format!("{e}")`.**

The good bridges already exist and prove it works:
`CompositionError::ComposeFailed(#[source] MarkdownError)`,
`FrontmatterParse(#[source] MarkdownError)`,
`ClaudineError::SystemPromptComposition(#[from] MarkdownError)` (P9). The fix is to
convert the ~6 flattening sites to typed variants:

| Site | Today | Becomes |
|------|-------|---------|
| `resolve.rs:42,47` | `InvalidReference(format!("{file_ref}: {e}"))` | `Reference { file_ref, #[source] FileReferenceError }` |
| `resolve.rs:62` | `MarkdownLoad(format!("{path}: {e}"))` | `MarkdownLoad { path, #[source] io::Error }` |
| `sequence.rs:106,115,220` | `SequenceExternalLoad(format!(...))` | `SequenceExternalLoad { raw, #[source] … }` |
| `closure.rs:147` | `AtomicWriteFailed(e.to_string())` | `AtomicWriteFailed { path, #[source] … }` |
| `lifecycle_control.rs:245` | `.map_err(|e| e.to_string())` | propagate the typed error |

**Enforcement (answers "lint the anti-patterns").** A repo-level review guard
(grep-based test or clippy lint allowlist) flags two smells at the DM↔Claudine
boundary: new `Variant(String)` error variants, and `map_err(|e| … e.to_string() …)`
on a value whose type implements `BlockError`. This keeps the boundary from
re-collapsing over time — the brainstorm's "treat as code smell" made executable.

### Layer 3 — A focused-context excerpt, owned by Darkmatter

P5's "all-or-nothing" is the one place the existing render assets genuinely fall
short. Two extremes exist today:

- `transform_block` renders **no** YAML.
- Claudine's `FrontmatterExcerpt` (`frontmatter_excerpt.rs`) and Darkmatter's
  `SourceContext::excerpt_prose` render a **contiguous** slice (a line ± context), and
  `FrontmatterExcerpt::capture_frontmatter_block` dumps the **entire** block.

Neither can render "just `spec`, `iteration`, and their `$schema:` parent." Add a
**non-contiguous, structure-aware** excerpt to `SourceContext` (Darkmatter-owned, so
the `md` CLI benefits too — answering the brainstorm's placement question):

```rust
// SourceContext
pub fn focused_yaml_excerpt(&self, keys: &[YamlKeyPath], term: &Terminal) -> Prose;
```

- Parses the frontmatter once, resolves each `YamlKeyPath` (dotted, indentation-aware
  — `FrontmatterExcerpt::locate_property_line` already does this) to its line range,
  **unions** those ranges with their structural ancestors (the `$schema:` parent line),
  and renders a gutter-numbered, syntax-highlighted excerpt with elision markers (`…`)
  between non-adjacent regions.
- The involved keys come *from the typed error*: `Interpolation.key` plus any
  file-reference arguments the expression touched (e.g. `spec`). This is the data the
  renderer needs that only a typed cause can supply (closes the P5↔P2 loop).

This generalizes `FrontmatterExcerpt`/`SourceContext::excerpt_prose` rather than
replacing them; the contiguous path stays for parse/fence errors where a single
locus is the right focus.

### Layer 4 — Did-you-mean for files, and auto-linked path fields

**Did-you-mean (P7).** The capability exists but cannot be reused as-is: `suggest`
requires the `Described` trait whose `key()` returns `&'static str`
(`catalog/mod.rs:40`), and filenames are runtime strings. So:

- Expose `levenshtein` (already `pub`, already Unicode-correct, `catalog/mod.rs:112`)
  and add a thin `suggest_strings(candidates: &[String], key: &str, max) -> Vec<&str>`
  that applies the *same quality gate* (`max(2, len/3)`) without the `Described` bound.
- Populate candidates lazily from the `FileReference`'s base directory: when a
  `FileRefFailure::NotFound` is rendered, list sibling entries of the *expected* parent
  dir and rank by edit distance against the leaf name. **Cost bound (open question):
  siblings of the target directory only, non-recursive, capped (e.g. first 1–2k
  entries), computed lazily at render time** — never during evaluation, so the hot path
  is untouched and large trees can't blow up.

**Auto-linking (P8).** Make "path → OSC8 link" a property of the *type*, not the call
site. `SourceContext::linked_path_prose` already produces the link; the change is that
the cause-driven blocks read path-typed fields (`reference`, `base_dir`) and link them
automatically when building the `StatusBlock` body. Claudine's 15 manual
`render_file_link()` sites (`error.rs:2229`) collapse into the shared block builder, so
new error variants get links for free and can't forget.

### Layer 5 (cross-cutting) — the renderer stays the sole decision-maker

No new top-level renderer is introduced. The existing `BlockError` →
`as_block_error` → deepest-block walk is the shared boundary (P10), and it *already
converges* the two CLIs. The only renderer-side additions are the three affordances
above (focused excerpt, string did-you-mean, auto-linked fields), all expressed as
methods on existing types (`SourceContext`, the block helpers). Investment lands in
one place and both CLIs inherit it — by construction, not by discipline.

---

## 4. The spec's error, end to end (worked trace)

| Stage | Today | Under this design |
|-------|-------|-------------------|
| `resolve_arg` fails | `Err("invalid file path \"features/…/spec.md\": …")` | `Err(ExpressionError::FileReference { reference, kind: NotFound, base_dir, … })` |
| `frontmatter_fn` | re-`format!`s the string | propagates the typed `ExpressionError` |
| evaluator | `EvalResult::Error { message: String }` | typed-error sibling carries `ExpressionError` |
| interpolation rewrite | `Transform("Interpolation evaluation failed for '…': …")` | `MarkdownError::Interpolation { key: "iteration", expression, cause: FileReference{…} }` |
| key scoping | prepends `"frontmatter key 'iteration': "` | sets `key = "iteration"` field |
| Claudine bridge | (would re-flatten) | `#[source]` — structure intact |
| render | `transform_block`: "transform failed" + dead hint | cause block: **headline** "invalid file path"; **focused excerpt** of `$schema`/`spec`/`iteration`; **hint** "Did you mean `features/2026-06-21-opencode-log-fix/spec.md`?"; **OSC8 link** to the prompt file |

This is exactly the spec's target rendering, produced from the typed cause with no
message string parsing anywhere.

---

## 5. Pattern coverage

| Pattern | Addressed by | Mechanism |
|--------|--------------|-----------|
| P1 headline names mechanism | L1 + L5 | cause block sets headline from the *cause* variant |
| P2 stringly-typed at boundary | **L0** | `Result<Value, ExpressionError>` replaces `Result<_, String>` |
| P3 nesting prose not data | L1 | `key_scoped_error` sets a field instead of concatenating |
| P4 generic per-wrapper hint | L1 + L4 | hint derived from typed `cause` fields |
| P5 all-or-nothing context | **L3** | `focused_yaml_excerpt(keys)` |
| P6 locus not captured | L0 | `resolve_arg` keeps `base_dir`/`fallback_dir` it already holds |
| P7 no did-you-mean for files | L4 | `suggest_strings` + lazy sibling listing |
| P8 manual opt-in OSC8 links | L4 | path-typed fields auto-linked by the block builder |
| P9 lossy cross-crate boundary | **L2** | `#[from]`/`#[source]` only, lint-enforced |
| P10 two render boundaries | L5 | already shared; converges once causes survive |
| P11 absent vs malformed | L0 | `FileRefFailure` discriminant |

P2 and L0 are load-bearing: six of eleven patterns are downstream of the engine
string boundary, which is why the design front-loads it.

---

## 6. Key technical challenges

1. **The engine's `String` reaches further down than the top-level signature.**
   It is not just `evaluate() -> Result<_, String>`; it is also `EvalResult::Error`
   (`evaluator.rs:91`) and every `require_*` leaf (`functions.rs`). Typing has to go
   down to the leaves to avoid a typed-then-re-stringified seam. This is the largest
   single mechanical change and touches the hot evaluation loop.

2. **Fatality classification is currently a string contract.** `is_fatal_eval_error`
   (`rewrite.rs:20`) and the lenient/fail-fast branch (`rewrite.rs:119-133`) encode
   *semantics* in a prefix match. Re-expressing this as `err.is_authoring_fatal()`
   risks subtly changing which failures abort vs. warn. Specifically: **should a
   missing file reference be fatal in lenient (non-`fail_fast`) body interpolation?**
   Today it is swallowed to a warning (only unknown-function is fatal). The whole-value
   frontmatter span in the spec is fatal because it's `fail_fast`. The design must
   preserve the existing lenient/strict matrix exactly unless we *deliberately* decide
   to promote missing-file to fatal — that is a behavior decision, not a refactor, and
   needs its own test matrix.

3. **Error types in a hot path want to stay cheap.** `evaluate` runs per `{{ … }}` span,
   per rescan pass (up to `MAX_INTERPOLATION_DEPTH` = 10). `ExpressionError` carries
   `PathBuf`s and `String`s. The error path is cold (only built on failure), so this is
   fine — *provided* we don't compute expensive fields (sibling listings, did-you-mean)
   at throw time. The design defers all of that to render time. The one thing to watch:
   `ExpressionError` is in the `Err` arm of a `Result` returned by value through a deep
   call stack; box it if `Result` size regresses the success path.

4. **`Described`/`suggest` cannot be reused for filenames without a new entry point.**
   The `&'static str` bound is real. `suggest_strings` is small but must replicate the
   quality gate semantics exactly so file suggestions feel as calibrated as the existing
   var/function ones; otherwise we get confident-but-wrong "did you mean" noise — the
   very thing the gate exists to prevent.

5. **The focused excerpt needs key→span resolution that survives composition.** To show
   `spec`/`iteration`/`$schema`, the error must carry enough to locate those keys in the
   *source* frontmatter. `locate_property_line` exists but works on the captured block
   string; threading a `SourceSpan`/`YamlKeyPath` from the throw site (where the key is
   known) to the renderer is new plumbing. Late-binding lifecycle interpolation
   complicates this: a lifecycle string is resolved at event-time via DM2, far from the
   original document parse, so the "source" to excerpt may be the effective frontmatter,
   not the file on disk. The focused excerpt may need a fallback for the late-binding
   case (render the resolved value, not a file slice).

6. **Cause-block delegation vs. scope preservation.** `BlockError` deliberately avoids
   double-rendering (`block_source()` returns `None`; the wrapper returns the inner
   block). If `MarkdownError::Interpolation` delegates to its `FileReference` cause's
   block, the *scope* fields (`key`, `expression`) must still surface — i.e. the
   delegation has to *merge* the cause's headline/hint with the wrapper's focused
   excerpt, not just forward. This is a small but real change to the delegation contract
   in `as_block_error`/`status_block`.

7. **`md` CLI vs. Claudine CLI excerpt source.** The `md` CLI renders a standalone
   document; Claudine renders a *prompt file in a compose pipeline*. The focused excerpt
   wants the original prompt file path. `SourceContext` already carries `absolute`/
   `content`, so this is mostly available — but Claudine's `FrontmatterExcerpt` and
   Darkmatter's `SourceContext` are *parallel* implementations today. They should
   converge on `SourceContext::focused_yaml_excerpt`, which means migrating Claudine's
   bespoke excerpt path — a non-trivial but high-value consolidation.

---

## 7. Benefits

- **Fixes the class, not the message.** Because the renderer is cause-driven and the
  fix is "let causes arrive typed," *every* error routed through the engine and the
  DM↔Claudine boundary improves at once — not just `frontmatter()`.
- **Both CLIs improve together, for free.** The `md` and `claudine` CLIs already share
  `as_block_error`; once causes survive and the three render affordances land in
  Darkmatter, neither CLI needs bespoke work (kills P10 structurally).
- **Correctness, not just cosmetics.** Typing the engine error removes a string-prefix
  control-flow decision (`is_fatal_eval_error`), making fatal-vs-warn a checked `match`.
- **Drift-resistant by construction.** Auto-linked path fields and lint-enforced
  transport mean coverage can't silently regress the way 15 hand-placed
  `render_file_link()` calls and ad-hoc `to_string()` bridges do today.
- **Leverages, doesn't reinvent.** No new renderer, no new diagnostic framework —
  `BlockError`, `StatusBlock`, `SourceContext`, `levenshtein` all stay; we extend them.
- **Breaking changes are affordable now.** The spec notes no installed user base, so
  retiring `Transform(String)` and changing the engine signature is acceptable for the
  long-term gain.

---

## 8. Limitations & areas needing more exploration

- **Engine-typing scope is a judgment call.** I recommend typing to the
  function-dispatch boundary and keeping `Parse(String)` as a last-mile variant. If
  parser errors turn out to be a meaningful share of author confusion, the parser needs
  a second typing pass — out of scope here, flagged as a likely follow-on.
- **The fatal-vs-warn semantics decision (challenge #2) is unresolved.** Whether to
  promote missing-file to fatal in lenient mode is a product decision with a test
  matrix, not something this design should silently change. Needs a deliberate call.
- **Late-binding lifecycle interpolation (challenge #5) is the hardest corner.** The
  focused-excerpt-of-source model assumes the error maps to a document on disk. DM2
  event-time resolution breaks that assumption. The fallback (render the resolved
  value/expression instead of a file slice) is sketched but not designed in detail.
- **Excerpt convergence is a migration, not a green-field.** Claudine's
  `FrontmatterExcerpt` and `WithFrontmatter`/`enrich_frontmatter` boundary work today;
  folding them into `SourceContext::focused_yaml_excerpt` must preserve the existing
  TTY-gating, `FORCE_COLOR` override, and non-TTY ANSI-stripping behavior. Risk of
  regressing those if done carelessly.
- **Did-you-mean calibration for files is unproven.** The quality gate works for short
  catalog keys; filenames are longer and more similar to each other (`spec.md` vs
  `specs.md` vs dated dirs). The gate threshold may need tuning specifically for paths,
  and we should decide leaf-name-only vs. full-relative-path matching.
- **Cost-bounding the sibling listing on huge trees** is bounded by "siblings only,
  capped, lazy," but the exact cap and whether to skip on directories above N entries
  needs measurement, not a guess.
- **`Result` size regression** from a fat `ExpressionError` in the hot eval loop is a
  *possible* (not confirmed) perf concern; needs a before/after check and likely
  `Box<ExpressionError>` in the `Err` arm.

---

## 9. Suggested phasing (sequenced by leverage, each independently shippable)

1. **L0 pilot — typed `ExpressionError` for the file path, behind the existing
   signatures.** Type `resolve_arg`/`load_markdown`/`frontmatter_fn` and the evaluator
   error, preserving the current `Display` strings byte-for-byte so behavior is
   unchanged. Pure refactor; provable by snapshot tests staying green.
2. **L1 — `MarkdownError::Interpolation` cause variant + cause-driven block.** First
   *visible* improvement: the spec's error gets its real headline and a typed hint.
3. **L4 — file did-you-mean + auto-linked path fields.** Highest user-visible payoff
   per line of code; reuses `levenshtein`.
4. **L3 — `SourceContext::focused_yaml_excerpt`.** The P5 fix; benefits `md` CLI too.
5. **L2 — boundary transport cleanup + the anti-pattern lint.** Convert the ~6
   flattening sites; lock the boundary so it can't re-collapse.
6. **Excerpt convergence + lifecycle late-binding fallback.** The hardest corners,
   done last when the typed substrate is in place.

Phases 1–2 alone resolve the literal spec example; 3–5 generalize it to the whole
error class; 6 closes the remaining corners.
