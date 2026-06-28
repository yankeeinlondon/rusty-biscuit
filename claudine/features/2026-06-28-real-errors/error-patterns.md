# Error Patterns — Brainstorm

> Working notes for the `real-errors` feature. The goal here is **not** to fix the
> one error in [`spec.md`](./spec.md) — it is to name the *recurring patterns* that
> manufacture dense, hard-to-read errors across **Darkmatter** and **Claudine**, so
> we can attack them strategically rather than one message at a time.
>
> Each pattern below is named, grounded in real `file:line` evidence, and paired with
> "what good looks like" drawn from the target error in the spec.

---

## The reference failure, annotated

```
 MarkdownError: transform failed                          ← (1) headline names the MACHINERY, not the problem
┃
┃ frontmatter key 'iteration': Interpolation evaluation   ← (2) three layers of prose concatenated
┃ failed for 'frontmatter(spec, ...) ? ... : 1':              with ": " — reader must parse to find cause
┃ frontmatter() invalid file path: "features/.../spec.md" ← (3) the REAL problem, buried last & smallest
┃
┃ Review the transform pipeline inputs and any            ← (4) generic hint; same string for every
┃ configured rules.                                          Transform error regardless of true cause
```

The root cause ("a file you referenced does not exist") is the *deepest, smallest,
last* thing on screen, framed by two layers of implementation vocabulary and capped
with a hint that tells the user nothing. Every pattern below contributes to one of
those four failures.

---

## Pattern catalog

### P1 — Headline names the mechanism, not the problem
**Symptom.** The first line a user reads describes the subsystem that threw
("transform failed", "Interpolation evaluation failed"), not the thing they did
wrong ("a referenced file doesn't exist").

**Evidence.**
- `darkmatter/.../markdown/errors/blocks.rs:205` — `transform_block()` hard-codes the
  header `("MarkdownError", "transform failed")` for *every* transform failure.
- `darkmatter/.../markdown/compose/interpolation/rewrite.rs:122` &
  `markdown/transform/mod.rs:376` — `"Interpolation evaluation failed for '{}': {}"`.

**Why it's bad.** Errors are written from the *implementer's* vantage (which module
blew up) instead of the *author's* vantage (what's wrong with my document). The user
has to reverse-engineer the subsystem map to understand a typo.

**Good looks like.** Headline = `invalid file path`. The mechanism ("transform") is
demoted to context or dropped entirely.

---

### P2 — Stringly-typed errors collapse structure at the source
**Symptom.** The richest information exists only at the throw site; by the time the
error surfaces it is an opaque `String` that cannot be re-focused, matched, linked,
or suggested-against.

**Evidence.**
- The entire expression/interpolation engine returns `Result<_, String>`:
  `functions.rs:1498` `frontmatter_fn(...) -> Result<Value, String>`;
  `resolve_arg(...) -> Result<Option<PathBuf>, String>` (`functions.rs:931`). The error
  type is `String` *at the engine boundary* — everything is pre-flattened before it can
  reach a typed layer.
- Claudine repeats the collapse when bridging Darkmatter:
  - `resolve.rs:42,47,62` — `map_err(|e| CompositionError::InvalidReference(format!("{file_ref}: {e}")))`
  - `sequence.rs:106,115,220` — `SequenceExternalLoad(format!("\`{raw}\`: {e}"))`
  - `closure.rs:147` — `AtomicWriteFailed(e.to_string())`
  - `lifecycle_control.rs:245` — `.map_err(|e| e.to_string())`
- Opaque `String`-only variants that can never be pattern-matched downstream:
  `CompositionError::InvalidReference(String)`, `FileNotFound(String)`,
  `MarkdownLoad(String)`, `TemplateError(String)`, `LinkingError(String)`;
  `MarkdownError::Transform(String)`.

**Why it's bad.** This is the *root enabler* of P1, P3, P4, P7, P8. Once a cause is a
string you cannot ask "is this a missing file?", recover "which path?", attach an OSC8
link, or run a did-you-mean. Every richer behavior downstream becomes impossible.

**Good looks like.** A typed cause that travels intact: e.g.
`InvalidFileReference { reference, assigned_to_key, prompt_file, base_dir, candidates }`,
where every field the renderer needs is still a real value, not embedded in prose.

---

### P3 — Nesting prose instead of nesting data
**Symptom.** Each layer that catches an error wraps it by string-concatenating a
human prefix, so depth = more prose to read, not more structure to render.

**Evidence.**
- `frontmatter_interpolation.rs:216` `key_scoped_error()` only knows how to do
  `Transform(format!("frontmatter key '{key}': {msg}"))` — it prepends prose to a
  string and re-wraps as the same opaque variant.
- The reference failure is literally three of these concatenations stacked:
  `key '{key}': ` + `Interpolation evaluation failed for '{expr}': ` + `frontmatter() invalid file path: {raw}`.

**Why it's bad.** The "scope" each layer adds (which key, which expression) is genuinely
useful — but as a *field*, not as a sentence fragment. As prose it lengthens the dense
blob and forces the reader to locate the `:`-delimited tail.

**Good looks like.** Layers attach fields (`key = "iteration"`, `expr = "..."`) to a
structured error; the renderer decides what to show and how to lay it out.

---

### P4 — Generic, per-wrapper-variant hints
**Symptom.** The trailing hint is keyed to the *outer wrapper variant*, not the true
cause, so it's identical and useless across wildly different failures.

**Evidence.**
- `blocks.rs:205-211` — every `Transform` error gets
  `"Review the transform pipeline inputs and any configured rules."`
- Hints are hard-coded one-per-`*_block()` function (`blocks.rs` `transform_block`,
  `frontmatter_parse_block`, `file_load_block`, ...). There is no mapping from *cause*
  to hint; the hint is decoration on the *mechanism*.

**Why it's bad.** A hint is supposed to be the single most actionable next step. Bound
to the wrapper, it can only be generic. A missing-file failure and a malformed-expr
failure (both `Transform`) get the same advice.

**Good looks like.** The hint is a property of the *root cause* (missing file →
"Did you mean? ..."), generated from the typed fields, not a static per-variant string.

---

### P5 — All-or-nothing context display
**Symptom.** Document context is shown as *nothing* or *everything* — never the
relevant slice.

**Evidence.**
- Transform/interpolation errors render **no** YAML at all (`transform_block` body is
  just the flattened message).
- Where context *is* shown, Claudine's `FrontmatterExcerpt`
  (`frontmatter_excerpt.rs`) dumps the **entire** frontmatter block (delimiters + all
  keys) and highlights one line.

**Why it's bad.** The spec calls this out directly: the fix is to show the *relevant
variables* plus their parent `$schema` line for shape — not zero context, not a full
dump. Neither current behavior does that.

**Good looks like.** Extract only the keys involved in the failure (`spec`, `iteration`)
plus the structural parent (`$schema:`), rendered as a focused, line-numbered YAML
excerpt.

---

### P6 — Missing locus/actor context at the throw site
**Symptom.** The deepest error literally *cannot* describe where it happened or offer
help because the throwing function was never given the context.

**Evidence.**
- `functions.rs:931-947` `resolve_arg` knows the raw path and `ctx.base_dir`, but the
  `frontmatter()` error string carries neither the **prompt file** that contained the
  expression, the **schema key** it was assigned to, nor the **sibling files** that
  could power a suggestion.
- `resolve_arg` returns `Ok(None)` (file valid but absent) vs `Err(...)` (path
  malformed) — but both collapse into the same user-facing "invalid file path", erasing
  the distinction the user most needs.

**Why it's bad.** You can't render what you didn't capture. P2/P3 ensure context is
lost in transit; P6 is the case where it was never gathered at all.

**Good looks like.** Resolution context carries (or can lazily produce) the base dir's
sibling listing, the owning prompt file, and the assigning key, so the error can be
built rich at the source.

---

### P7 — "Did you mean?" exists, but not for the most common failure (files)
**Symptom.** Fuzzy suggestion machinery is in the codebase, yet the single most common
authoring mistake — a mistyped file path — gets none.

**Evidence.**
- Suggestions exist for **unknown ctx vars** (`evaluator.rs:304` via
  `suggest(CONTEXT_VARIABLE_DESCRIPTORS, ...)`), **unknown expression functions**
  (`expression/mod.rs`), **lifecycle verbs** (`lifecycle.rs:2183` via
  `darkmatter::catalog::levenshtein`), and **loop keys** (`loop_config.rs:308`).
- **No** suggestion path for missing files in `frontmatter()` / `resolve_arg`, despite
  the spec's observation that a missing file is *almost always a typo*.

**Why it's bad.** The capability is proven and present; it's just not wired to the
highest-frequency error class. Pure missed leverage.

**Good looks like.** When a file reference fails, list the closest sibling filenames by
edit distance ("Did you mean `features/2026-06-21-opencode-log-fix/spec.md`?").

---

### P8 — OSC8 links are manual, opt-in, and inconsistent
**Symptom.** Clickable file links appear on some error paths and not others, because
every link is a hand-written `<a href>` the author must remember to emit.

**Evidence.**
- Darkmatter renders links only for **schema validation** errors
  (`blocks.rs:267`) and frontmatter-parse errors (`ctx.linked_path_prose()`);
  Transform errors render **no** links.
- Claudine has `render_file_link()` (`error.rs:2186`) called manually at ~15 sites
  (`error.rs:1484,1553,1605,...`). Easy to forget; a path that isn't passed through it
  renders as dead plain text.

**Why it's bad.** Linkability becomes a per-call-site discipline instead of a property
of "this value is a path". Coverage is therefore patchy and drifts.

**Good looks like.** Paths in typed error fields are linked automatically by the
renderer; authors don't decide per-message.

---

### P9 — Two error vocabularies, lossy at the boundary
**Symptom.** Darkmatter (`MarkdownError`) and Claudine (`CompositionError`) are
separate error worlds; some bridges preserve structure, many flatten it.

**Evidence.**
- Structure-preserving bridges exist and are the model to follow:
  `ClaudineError::SystemPromptComposition(#[from] MarkdownError)`,
  `CompositionError::ComposeFailed(#[source] MarkdownError)`,
  `FrontmatterParse(#[source] MarkdownError)`.
- But many sites take a `MarkdownError` and stuff it into a `String`
  (P2 evidence: `resolve.rs`, `sequence.rs`). The boundary is the place structure most
  often dies.

**Why it's bad.** Inconsistency means a user's experience depends on *which code path*
threw, not on the nature of their mistake. The good paths prove the bad paths are
fixable.

**Good looks like.** A uniform rule: cross-crate error transport always uses
`#[from]` / `#[source]`, never `to_string()`.

---

### P10 — Two render boundaries with divergent quality
**Symptom.** The *same* underlying failure renders well or badly depending on whether
it surfaces through Claudine's CLI or Darkmatter's own renderer.

**Evidence.**
- Claudine has a thoughtful render boundary: `error_walker.rs` finds the **deepest**
  `BlockError`, and `enrich_frontmatter` appends a `FrontmatterExcerpt` at the CLI
  edge.
- Darkmatter's own path (`status_block()` → `transform_block`) produces the dense
  reference failure and is what the `md` CLI shows standalone.

**Why it's bad.** Quality is a function of entry point, not of the error. Investment in
one renderer doesn't help the other.

**Good looks like.** Rich rendering driven by the *typed error*, shared by both crates'
render boundaries.

---

### P11 — Lossy distinctions: absent vs malformed vs wrong-directory
**Symptom.** Genuinely different failure modes are folded into one generic message.

**Evidence.**
- `resolve_arg` (`functions.rs:931`) distinguishes "path string malformed" (`Err`) from
  "valid path, not found in base_dir or CWD" (`Ok(None)`) — then the caller renders both
  as `"invalid file path"`.

**Why it's bad.** The correct next action differs: malformed → fix syntax; not found →
check the name / did-you-mean; found-elsewhere → fix the relative root. Collapsing them
hides the one fact that determines the fix.

**Good looks like.** Distinct typed causes (or a discriminant field) so the renderer and
hint can branch.

---

## Cross-cutting structural root causes

Three foundations underlie the surface patterns:

1. **String is the error type at subsystem boundaries (P2).** The expression engine's
   `Result<_, String>` is the upstream dam; nothing typed can flow past it. This is the
   highest-leverage thing to change — most other patterns are downstream of it.

2. **Wrapping is prose-concatenation, not field-attachment (P3).** Every catch layer
   reaches for `format!("{prefix}: {msg}")`. Until "add context" means "add a field",
   depth will keep producing density.

3. **Rendering is decoupled from cause, coupled to wrapper (P1, P4, P8, P10).** Headline,
   hint, links, and excerpt are all decided by the *outermost variant / call site*
   instead of the *typed root cause*. A cause-driven renderer would fix several patterns
   at once.

---

## Assets already in the codebase (build on, don't reinvent)

- `darkmatter::catalog::levenshtein` + `suggest(...)` — fuzzy matcher, already used for
  vars/functions/verbs. Reuse for filenames (P7).
- `CompositionError::WithFrontmatter` + `FrontmatterExcerpt` + `error_walker` deepest-
  block walk — a working render-boundary pattern (P5/P10) to generalize.
- `render_file_link()` and the `<a href>` Prose convention — the linking primitive (P8),
  needs to become automatic.
- `#[from]` / `#[source]` bridges already used for some `MarkdownError` transport (P9) —
  the correct pattern, needs to become the *only* pattern.
- `StatusBlock` / `Prose` / `CodeBlock` renderables — the display vocabulary for the
  target design already exists.

---

## Candidate strategic directions (to validate, not yet decided)

- **Typed file-reference error as the pilot.** Replace the `frontmatter()` string error
  with a structured cause carrying `{ reference, assigned_key, prompt_file, base_dir }`
  and lazily-computed `candidates`. It exercises P1, P2, P5, P6, P7, P8 end-to-end and
  is exactly the spec's example.
- **Cause-driven rendering trait.** A renderer that takes the *deepest typed cause* and
  produces headline + focused excerpt + auto-links + cause-specific hint — shared by both
  crates' boundaries (kills P4, P8, P10 structurally).
- **A "context excerpt" extractor** that, given the involved keys, emits only those keys
  + their `$schema` parent (P5) instead of full-dump or nothing.
- **Lint the anti-patterns.** Treat new `Variant(String)` error variants and
  `map_err(|e| ...to_string())` across the DM↔Claudine boundary as code smells to grep
  for in review (P2, P9).

---

## Open questions

- How far up does the expression engine's `Result<_, String>` need to become typed
  before the benefit is realized — just `frontmatter()`/`resolve_arg`, or the whole
  engine? (Scope vs. payoff.)
- Should the rich "focused YAML excerpt" live in Darkmatter (so the `md` CLI benefits)
  or Claudine? The spec wants *both* libraries improved — argues for Darkmatter-owned.
- Do we need a shared `BlockError`-style trait spanning both crates, or does
  `#[source]` transport + a single CLI-edge renderer suffice?
- Candidate file lists for did-you-mean: directory siblings only, or recurse? How do we
  bound cost on large trees?
