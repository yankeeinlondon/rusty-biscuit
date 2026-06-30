---
created: 2026-06-27
status: draft
reviewed: false
area: darkmatter
packages:
    - darkmatter
related_specs:
    - "@darkmatter/features/_completed/2026-05-11-schemas/spec.md"
    - "@darkmatter/features/_completed/2026-05-23-compose-schema/spec.md"
    - "@darkmatter/features/_completed/2026-05-28-schema-coercion/design.md"
    - "@claudine/fixes/2026-06-27-path-resolution/plan.md"
---

# Rewrite `file`-Typed Schema Property Values to Their Resolved Relative Path

A `$schema` property typed **`file`** validates that the caller-supplied value
is a resolvable file reference (via the `darkmatter-file` format →
`resolve_file_reference`), but then **throws the resolved path away**: the
document's stored value stays the raw, caller-supplied reference. Validation is
a pure predicate today (`validate_file_reference(value) -> bool`,
`schemas/format.rs:70`).

This leaves the document in a **mixed-resolution state**: a `file` argument
remains a raw reference, while every path *function* that consumes it
(`dirname`, `basename`, `parent_dir`, `relative`, `file_exists`, `frontmatter`)
first **resolves** it. Authors are forced to reconcile the two by hand — and
they get it wrong, because a raw reference and a resolved path are not
interchangeable.

This feature closes that gap: when a property is `file`-typed, validation
**rewrites the stored value to the resolved, repo-relative path**, using the
exact projection the path functions already apply. After the rewrite the
document state is uniformly resolved, so `spec` and `dirname(spec)` agree by
construction and no author ever hand-prepends `{{ctx.area}}` again.

> **Why `file`, not `string`.** This transformation is scoped *only* to the
> `file` type. A `string` property is verbatim text and must never be rewritten
> — a caller who declares `string` is asking for the literal value back. The
> declared type is the opt-in. (This is the property review-feature.md got
> wrong: it typed `spec: string(required)`, so `spec` stayed raw while
> `dirname(spec)` resolved.)

## Motivating Bug

`prompts/review-feature.md` builds a review-file path two ways that must agree:

```yaml
$schema:
    spec: string(required)          # <- raw reference, NOT resolved
dir: "{{dirname(spec || design)}}"  # <- dirname RESOLVES -> "claudine/fixes/…"
review_file: "{{ctx.area}}/{{dir}}/review-{{iteration}}.md"
success:
    stack:
        - when: "frontmatter(review_file,'ready') == true"   # ready branch
          action: [ { message: "✅ …" }, … ]
        - when: "frontmatter(review_file,'ready') != true"   # not-ready branch
          action: [ { message: "⚠️ …" }, … ]
```

Because `dirname(spec)` is already repo-relative (`claudine/fixes/…`) but the
author models `dir` as area-relative and prepends `{{ctx.area}}`, `review_file`
**doubles the area**:

```
dir          = claudine/fixes/2026-06-26-opencode-yolo
review_file  = claudine/claudine/fixes/2026-06-26-opencode-yolo/review-3.md   # doubled
```

The doubled path does not exist. At event time `frontmatter(review_file,'ready')`
cannot read it and **throws**; a throwing `when:` on the *first* stack item
aborts the entire `success` event (`claudine/.../lifecycle_executor.rs:589`), so
the second (not-ready) item never runs — **nothing is sent to Discord**, even
though the agent set `ready: true`. The reported symptom ("`== true` is never
true") was a red herring: the comparison is correct; the path it reads is wrong.

Empirically (verified via `md compose`):

| stored value | `file_exists` | `… == true` |
|---|---|---|
| `claudine/fixes/…/review-2.md` (single) | `true` | `true` |
| `claudine/claudine/fixes/…/review-2.md` (doubled) | `false` | *throws* |

With this feature, `spec` is typed `file` and rewritten to
`claudine/fixes/2026-06-26-opencode-yolo/spec.md` at validation time. `dir`,
`review_file`, and every body reference are then derived from an
already-resolved value, the `{{ctx.area}}/` prefixes are removed from the
prompt, and the doubling is structurally impossible.

## Goals & Non-Goals

**Goals**

- When a frontmatter property is `file`-typed and its value resolves, rewrite
  the **stored** value to the resolved, repo-relative path.
- Use the **same projection** the existing path functions use — resolve via
  `FileReference`, then `make_relative(_, base_dir)` (git-root-first) — so the
  rewritten value is byte-identical to what `relative(value)` /
  `dirname(value)`'s prefix would produce. Consistency by construction, not by a
  parallel re-implementation.
- Apply the rewrite on **both** surfaces that already share schema write-back:
  the library validation path and the compose-stage frontmatter write-back
  (the same dual surface `coerce_frontmatter` serves today).
- Be **idempotent**: rewriting an already-resolved relative value yields the
  same string, so compose → re-validate is a fixpoint.
- Leave validation **outcomes** unchanged: an unresolvable `file` value still
  fails validation exactly as today; the rewrite only runs on the success path.

**Non-Goals (this feature)**

- **No change to read-side resolution.** How `file_exists`/`frontmatter`/the
  schema `file` validator resolve a reference (and against which fallback
  anchor) is governed by the path-resolution work
  (`@claudine/fixes/2026-06-27-path-resolution/plan.md`). This feature only
  normalizes the **stored value**; it consumes whatever resolver that work
  settles on. The two must agree on the canonical form (repo-relative) — see
  Foundational Decision #5.
- **No rewriting of `string` (or any non-`file`) properties.** The declared
  type is the sole trigger.
- **No relativizing of remote/URL references.** A `file` value that resolves to
  a non-local reference (http(s)/url) is validated as today and stored verbatim;
  there is no local path to relativize.
- **No new authoring syntax.** `file` already exists as a simplified type and as
  a root-union arm; this only changes what a `file`-typed *value* becomes.
- **No prompt edits in this feature.** Switching prompts to `file` types and
  deleting the `{{ctx.area}}/` prefixes is a required but separate follow-up
  (see Downstream Work) so the Darkmatter change can land and be reviewed on its
  own.

## Foundational Decisions

- **Decision #1 — The `file` type is the opt-in; `string` is never rewritten.**
  The rewrite fires iff a property's compiled fragment carries the
  `format: darkmatter-file` marker (how `SimplifiedType::File` compiles,
  `simplified/convert.rs:446`). `string`/`number`/`enum`/etc. are untouched.

- **Decision #2 — The canonical stored form is the git-root-relative projection,
  reusing existing code.** The rewrite is `resolve_arg(value)` →
  `make_relative(resolved, base_dir)` — exactly the body of the existing
  `relative()` function (`functions.rs:1015`). `make_relative`
  (`functions.rs:998`) strips the git root first, then `base_dir`, then `~`,
  else absolute. This is why `dirname(spec)` already yields `claudine/fixes/…`,
  so a `file`-typed `spec` rewrites to the *same* prefix its `dirname` would
  emit. No new relativization logic is introduced; if a single shared helper is
  needed by both `expression` and `schemas`, lift `make_relative`/`resolve_arg`
  into a small shared module rather than duplicating them.

- **Decision #3 — Rewrite is a dedicated post-validation pass, not the
  jsonschema format callback and not `coerce_frontmatter`.** The
  `darkmatter-file` format callback returns `bool` and **cannot mutate** the
  instance. `coerce_frontmatter` is intentionally **FS-agnostic and pure** (JSON
  type-shape coercion only); resolving a file reference needs filesystem access
  and a resolution context, which do not belong there. So the rewrite is a
  separate pass that walks `EffectiveSchema.json_schema` for
  `format: darkmatter-file` properties and rewrites the matching present values,
  carrying a resolution context. It runs at the **same integration points** as
  the coercion write-back so both the library and compose surfaces rewrite
  consistently (Decision #4).

- **Decision #4 — Runs only on validation success, after coercion.** Order:
  coerce → validate → (on success) file-rewrite. A document that fails
  validation is never rewritten (its `file` value did not resolve, by
  definition of the `darkmatter-file` failure). This keeps "invalid input" and
  "normalized output" cleanly separated and means the rewrite never has to
  invent a value for an unresolvable reference.

- **Decision #5 — Resolution context (and its anchor) is threaded in, coupling
  this to the path-resolution work.** The rewrite needs `base_dir` (and, once it
  exists, the launch-area `file_ref_fallback_dir` from
  `@claudine/fixes/2026-06-27-path-resolution/plan.md`) to resolve and project.
  Whatever fallback anchor that plan threads into read-side resolution, the
  rewrite pass must use the **same** `ResolutionContext`, so the stored value
  resolves identically at prepare time and at every later read. Until that plan
  lands, the rewrite uses the existing `base_dir`/git-root projection (the same
  "works before the chdir" basis the current `dirname` relies on); the two
  efforts should be sequenced or co-designed so they never disagree on the
  canonical form.

- **Decision #6 — Idempotence is required and tested.** `resolve_arg` of an
  already-relative resolved value yields the same absolute path, and
  `make_relative` of that yields the same string. The pass must be a fixpoint:
  `rewrite(rewrite(x)) == rewrite(x)`. This guarantees the compose write-back
  (which persists the rewritten frontmatter and may be re-validated) is stable
  and never drifts a value across runs.

- **Decision #7 — Non-local and absent values pass through unchanged.** A `file`
  value that resolves to a URL/remote reference (which `resolve_arg` rejects for
  local projection) is left verbatim — validated, not rewritten. An absent or
  `null` optional `file` property is left as-is. Only a present value that
  resolves to a local path is rewritten.

- **Decision #8 — Root-union `file` arms participate; nested `file` properties
  participate.** Wherever `format: darkmatter-file` appears in the compiled
  schema for a *present* value — a top-level property, an inline-object
  sub-property, an array element, or the winning arm of a root/property union —
  that value is rewritten. The walk mirrors the schema-shape descent the
  description-resolution and coercion passes already perform (nullable `anyOf`
  unwrap, `items` for arrays).

## Behavior

```
caller passes:   spec = "fixes/2026-06-26-opencode-yolo/spec.md"   (file-typed)
                 │
coerce  ─────────┤  (JSON type-shape coercion; file value already a string → no-op)
                 │
validate ────────┤  darkmatter-file format: resolve_file_reference(value).is_ok()? 
                 │     └─ fail → ValidationProblem (unchanged); STOP, no rewrite
                 │     └─ ok  → continue
                 │
file-rewrite ────┘  for each present file-typed value:
                       resolved = resolve_arg(value, ctx)        # Some(abs) | None(url) 
                       stored   = make_relative(abs, ctx.base_dir)
                       → "claudine/fixes/2026-06-26-opencode-yolo/spec.md"

document state now: spec = "claudine/fixes/2026-06-26-opencode-yolo/spec.md"
dirname(spec)      = "claudine/fixes/2026-06-26-opencode-yolo"     # agrees
```

## Module Layout & Touchpoints

**Modified / added (darkmatter/lib):**

```
markdown/compose/expression/functions.rs   # make_relative/resolve_arg: extract a
                                            #   shared resolver if schemas can't reach them
markdown/schemas/
├── mod.rs        # file-rewrite pass invoked alongside the coercion write-back,
│                 #   after successful validation, in the same surfaces
├── rewrite.rs    # (new) walk json_schema for format: darkmatter-file; rewrite
│                 #   present local values via the shared resolve+relative helper
markdown/compose/schema_validation.rs       # compose write-back applies the rewrite
                                            #   to effective_frontmatter on success
```

The compose pipeline already persists coerced frontmatter; the rewrite hooks the
same write-back so the normalized `file` values are what downstream
interpolation, lifecycle events, and `inline-compose` see.

## Testing Strategy

**Rewrite unit tests (`rewrite.rs`)**

- A top-level `file` property with a resolvable relative reference is rewritten
  to the git-root-relative path.
- The rewritten value equals `relative(value)` for the same input (consistency
  with the existing projection).
- A `string` property holding a path-shaped value is **not** rewritten.
- Idempotence: rewriting an already-rewritten value is a no-op (Decision #6).
- An unresolvable `file` value fails validation and is never rewritten
  (Decision #4) — no partial mutation.
- A `file` value resolving to a URL/remote reference is left verbatim
  (Decision #7).
- An absent / `null` optional `file` property is unchanged.
- Nested `file` (inline-object sub-property), array-of-`file`, and root/property
  union `file` arm are each rewritten via the shape walk (Decision #8).
- Process-CWD independence: with the CWD mutated to an unrelated dir, the rewrite
  still produces the git-root-relative value (mirrors the
  `#[serial_test::serial("darkmatter-file-cwd")]` convention in `format.rs`).

**Integration**

- `md compose` of a doc whose `file`-typed property started raw shows the
  resolved relative value in the effective frontmatter dump.
- Compose write-back persists the rewritten value (re-`md compose` is a fixpoint).
- An end-to-end review-feature-shaped fixture: `spec: file`, no `{{ctx.area}}/`
  prefixes, `dirname(spec)` and a derived `review_file` agree and resolve.

## Downstream Work (separate, required follow-up — claudine)

Once this lands, claudine prompts stop compensating for the gap:

- `prompts/review-feature.md`: type `spec: file(required)` and `design: file`;
  delete every manual `{{ctx.area}}/` prefix — `@{{spec}}`, `@{{design}}`, and
  `review_file: "{{dir}}/review-{{iteration}}.md"`. The double-prefix bug
  disappears structurally.
- Audit sibling prompts (`prompts/implement-suggestions.md`, others using
  `dirname`/`@{{ctx.area}}/{{…}}`) for the same pattern.
- This is tracked as its own claudine change, not in this Darkmatter feature, so
  the library behavior can be reviewed and merged independently.

## Risks

- **Anchor disagreement with read-side resolution.** If the rewrite relativizes
  against one anchor (git root) while a later read resolves against another
  (post-`chdir` ambient CWD, launch area), the stored value could fail to
  resolve. Mitigated by Decision #5 — share one `ResolutionContext` with the
  path-resolution work — and by the integration fixture that round-trips a
  rewritten value through a real read.
- **Resolution-context plumbing reaches a previously pure layer.** The rewrite
  pass needs FS access and a base dir where `coerce_frontmatter` deliberately
  has none. Keeping the rewrite a *separate* pass (Decision #3) preserves
  `coerce_frontmatter`'s purity and confines the new dependency.
- **Persisting resolved paths into committed documents.** The rewritten value is
  repo-relative (portable across machines/OSes), not absolute, so committed
  prompt/spec state stays portable. A non-repo document falls back to
  `base_dir`/`~`/absolute via `make_relative`; absolute is the only
  non-portable outcome and only occurs outside any git root.
- **Behavior change for existing `file`-typed documents.** Any current document
  already using `file` properties will start seeing its stored values rewritten.
  Because the rewrite is the value those documents' own path functions already
  computed, the observable effect is convergence, not breakage; covered by the
  idempotence and consistency tests.

## Open Questions

- Should the rewrite also normalize the value the **caller** sees echoed back
  (e.g. in `initialize`/`start` diagnostics), or only the persisted document
  state? Leaning: only document state; diagnostics interpolate from the same
  state and inherit the normalized value for free.
- Sequencing with `@claudine/fixes/2026-06-27-path-resolution/plan.md`: land the
  fallback-anchor threading first (so the shared `ResolutionContext` exists), or
  co-design both and land together? The anchor must be agreed before either is
  considered done.
