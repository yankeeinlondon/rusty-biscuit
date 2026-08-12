---
created: 2026-06-27
status: ready for planning and implementation
reviewed: true
review_iterations: 1
area: darkmatter
packages:
    - darkmatter
related_specs:
    - "@darkmatter/features/_completed/2026-05-11-schemas/spec.md"
    - "@darkmatter/features/_completed/2026-05-23-compose-schema/spec.md"
    - "@darkmatter/features/_completed/2026-05-28-schema-coercion/design.md"
    - "@claudine/fixes/_completed/2026-06-27-path-resolution/plan.md"
---

# Rewrite `file`-Typed Schema Property Values to Their Resolved Relative Path

A `$schema` property typed **`file(eager)`** validates that the caller-supplied
value is a resolvable, *existing* file reference (via the `darkmatter-file`
format → `resolve_file_reference`), but then **throws the resolved path away**:
the document's stored value stays the raw, caller-supplied reference. The eager
validator is a predicate that already resolves (document-first → launch-area
fallback) — `validate_file_reference(value, base_dir, fallback) -> bool`,
`schemas/format.rs:126` — it simply discards the resolved path it computed.

> **Eager vs lazy `file`.** Since this spec was drafted the `file` type split
> in two: bare `file` lowers to the **lazy** `darkmatter-file-reference` format
> (syntax-only via `FileReference::new`; a not-yet-existing path passes, nothing
> is resolved), while `file(eager)` lowers to the **eager** `darkmatter-file`
> format (resolve + existence check). This feature concerns *only* the eager
> arm: there is no resolved path to rewrite for a lazy reference, and rewriting
> one would be wrong (it may legitimately name a file that does not exist yet,
> e.g. a `review_file`). See Decision #1 and the Non-Goals.

This leaves the document in a **mixed-resolution state**: an eager `file`
argument remains a raw reference, while every path *function* that consumes it
(`dirname`, `basename`, `parent_dir`, `relative`, `file_exists`, `frontmatter`)
first **resolves** it. Authors are forced to reconcile the two by hand — and
they get it wrong, because a raw reference and a resolved path are not
interchangeable.

This feature closes that gap: when a property is `file(eager)`-typed, validation
**rewrites the stored value to the resolved, repo-relative path**, using the
exact projection the path functions already apply. After the rewrite the
document state is uniformly resolved, so `spec` and `dirname(spec)` agree by
construction and no author ever hand-prepends `{{ctx.area}}` again.

> **Why eager `file`, not `string` (and not lazy `file`).** This transformation
> is scoped *only* to the eager `file` type (`darkmatter-file`). A `string`
> property is verbatim text and must never be rewritten — a caller who declares
> `string` is asking for the literal value back. A **lazy** `file` resolves
> nothing and may name a not-yet-existing file, so it is also left verbatim. The
> declared type (eager) is the opt-in. (This is the property review-feature.md
> got wrong: it typed `spec: string(required)`, so `spec` stayed raw while
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

With this feature, `spec` is typed `file(eager)` and rewritten to
`claudine/fixes/2026-06-26-opencode-yolo/spec.md` at validation time. `dir`,
`review_file`, and every body reference are then derived from an
already-resolved value, the `{{ctx.area}}/` prefixes are removed from the
prompt, and the doubling is structurally impossible.

## Goals & Non-Goals

**Goals**

- When a frontmatter property is `file(eager)`-typed and its value resolves,
  rewrite the **stored** value to the resolved, repo-relative path.
- Use the **same projection** the existing path functions use — resolve via
  `FileReference`, then `make_relative(_, base_dir)` (git-root-first) — so the
  rewritten value is byte-identical to what `relative(value)` /
  `dirname(value)`'s prefix would produce. Consistency by construction, not by a
  parallel re-implementation.
- Apply the rewrite on **both** surfaces that already share schema write-back:
  an explicit library normalization API and the compose-stage frontmatter
  write-back (the same dual surface `coerce_frontmatter_with_pending` serves
  today). The existing `EffectiveSchema::validate*` methods remain read-only.
- Be **idempotent**: rewriting an already-resolved relative value yields the
  same string, so compose → re-validate is a fixpoint.
- Leave validation **outcomes** unchanged: an unresolvable eager `file` value
  still fails validation exactly as today; the rewrite only runs on the success
  path.
- Update the public schema docs (`darkmatter/docs/topics/schema-definition.md`
  and `darkmatter/docs/inline/schema-validation.md`) so the new eager-file
  normalization contract is documented alongside type coercion.

**Non-Goals (this feature)**

- **No change to read-side resolution.** How `file_exists`/`frontmatter`/the
  eager `file` validator resolve a reference (and against which fallback anchor)
  is governed by the now-landed path-resolution work
  (`@claudine/fixes/_completed/2026-06-27-path-resolution/plan.md`). This feature
  only normalizes the **stored value**; it consumes the resolver that work
  settled on (`ResolutionContext` with `base_dir` + `file_ref_fallback_dir`).
  The two must agree on the canonical form (repo-relative) — see Foundational
  Decision #5.
- **No rewriting of `string`, lazy `file`, or any non-eager-`file` property.**
  The declared **eager** `file` type (`darkmatter-file`) is the sole trigger. A
  bare lazy `file` (`darkmatter-file-reference`) is syntax-only — it resolves
  nothing and may name a not-yet-existing file — so it is left verbatim.
- **No relativizing of remote/URL references.** An eager `file` value that
  is remote-like is validated exactly as today and is never fetched or
  relativized by this feature. If the existing eager validator rejects it, it
  still fails validation; if a future resolver accepts a non-local reference,
  the rewrite pass must leave the stored value verbatim because there is no
  local path to project.
- **No new authoring syntax.** `file` and `file(eager)` already exist as
  simplified types and as root-union arms; this only changes what an eager
  `file`-typed *value* becomes.
- **No prompt edits in this feature.** Switching prompts to `file(eager)` types
  and deleting the `{{ctx.area}}/` prefixes is a required but separate follow-up
  (see Downstream Work) so the Darkmatter change can land and be reviewed on its
  own.
- **No implicit mutation from validation-only APIs.** Existing library callers
  that call `EffectiveSchema::validate` or `validate_with_positions` keep the
  documented contract that validation coerces on a working copy and does not
  mutate caller input. Callers that want normalized values must call the new
  normalization API explicitly.

## Foundational Decisions

- **Decision #1 — The eager `file` type is the opt-in; `string` and lazy `file`
  are never rewritten.** The rewrite fires iff a property's compiled fragment
  carries the eager `format: darkmatter-file` marker. Since the eager/lazy split
  (`file_fragment`, `simplified/convert.rs:554`), `SimplifiedType::File` compiles
  to **one of two** markers: bare `file` → lazy `darkmatter-file-reference`
  (syntax-only, *not* a trigger) and `file(eager)` → eager `darkmatter-file`
  (resolve + exists, *the* trigger). Keying on the eager marker means the rewrite
  only fires where validation has already proven the value resolves to an
  existing file. `string`/`number`/`enum`/lazy `file`/etc. are untouched.

- **Decision #2 — The canonical stored form is the git-root-relative projection,
  reusing existing code.** The rewrite is `resolve_arg(value, ctx)` →
  `make_relative(resolved, base_dir)` — exactly the body of the existing
  `relative_fn` (`functions.rs:1161`). `make_relative` (`functions.rs:1143`)
  strips the git root first, then `base_dir`, then `~`, else absolute. This is
  why `dirname(spec)` already yields `claudine/fixes/…`, so an eager `file`-typed
  `spec` rewrites to the *same* prefix its `dirname` would emit. No new
  relativization logic is introduced. Resolution is **already** shared — the
  eager validator calls `resolve_ctx::resolve_file_ref_with_fallback`
  (`schemas/format.rs:55`), the same helper `resolve_arg` uses — so only
  `make_relative` remains private to `functions.rs`; lift it into a small shared
  module so `expression` and `schemas` relativize identically rather than
  duplicating it.

- **Decision #3 — Rewrite is a dedicated post-validation pass, not the
  jsonschema format callback and not `coerce_frontmatter_with_pending`.** The
  `darkmatter-file` format callback returns `bool` and **cannot mutate** the
  instance. `coerce_frontmatter_with_pending` is intentionally **FS-agnostic and
  pure** (JSON type-shape coercion only); resolving a file reference needs
  filesystem access
  and a resolution context, which do not belong there. So the rewrite is a
  separate pass that walks `EffectiveSchema.json_schema` for
  `format: darkmatter-file` properties and rewrites the matching present values,
  carrying a resolution context. It runs at the **same integration points** as
  the coercion write-back so both the library and compose surfaces rewrite
  consistently (Decision #4).

- **Reader's note: validation remains read-only.** Earlier drafts said "library
  validation path" too loosely, which would violate the current
  `EffectiveSchema::validate_with_positions` contract: it coerces against a
  working copy and mutates no input. The implementation must add an explicit
  normalization surface such as
  `EffectiveSchema::normalize_frontmatter(&self, frontmatter, pending, ctx) ->
  NormalizationOutcome` (exact name left to implementation style) that returns
  the coerced-and-rewritten value plus `changed`. Compose calls that API and
  writes its result back; validation-only library callers remain unchanged.

- **Decision #4 — Runs after coercion and after the relevant validation gate.**
  Normal validation order is: coerce → validate → (on success) file-rewrite. A
  document with final validation problems is never rewritten. In compose's
  pre-shell stage, "success" means no **composition-independent** validation
  problems after the existing pending-value filtering. The rewrite pass must
  skip any value that is still composition-pending (`$(...)` or unresolved
  `{{ ... }}` anywhere in that value) and may rewrite non-pending eager-file
  values from the accepted effective schema. This preserves the current
  defer-until-post-shell behavior while still normalizing fields whose values
  are already concrete.

- **Decision #5 — Resolution context (and its anchor) is threaded in, reusing
  the now-landed path-resolution work.** The rewrite needs `base_dir` and the
  launch-area `file_ref_fallback_dir` to resolve and project. That plumbing has
  **shipped** (`@claudine/fixes/_completed/2026-06-27-path-resolution/plan.md`):
  `ResolutionContext { base_dir, file_ref_fallback_dir }` exists
  (`resolve_ctx.rs`), the eager `file` validator already resolves through it
  (`schemas/format.rs`, document-first → launch-area fallback), and the
  expression functions thread it. The rewrite pass must use the **same**
  `ResolutionContext` the read-side resolves through, so the stored value
  resolves identically at prepare time and at every later read. No anchor
  disagreement remains to design around — the two surfaces already share one
  resolver; the rewrite just consumes it.

- **Decision #6 — Idempotence is required and tested.** `resolve_arg` of an
  already-relative resolved value yields the same absolute path, and
  `make_relative` of that yields the same string. The pass must be a fixpoint:
  `rewrite(rewrite(x)) == rewrite(x)`. This guarantees the compose write-back
  (which persists the rewritten frontmatter and may be re-validated) is stable
  and never drifts a value across runs.

- **Decision #7 — Non-local and absent values pass through unchanged.** An eager
  `file` value that resolves to, or is otherwise classified as, a URL/remote
  reference is left verbatim if validation accepted it. An absent or `null`
  optional eager `file` property is left as-is. Only a present concrete value
  that resolves to a local path is rewritten.

- **Decision #8 — Root-union eager `file` arms participate; nested eager `file`
  properties participate.** Wherever the **eager** `format: darkmatter-file`
  appears in the compiled schema for a *present* value — a top-level property, an
  inline-object sub-property, an array element, or the winning arm of a
  root/property union — that value is rewritten. The lazy
  `darkmatter-file-reference` marker is skipped wherever it appears. The walk
  mirrors the schema-shape descent the description-resolution and coercion passes
  already perform (nullable `anyOf` unwrap, `items` for arrays).

- **Decision #9 — Raw JSON Schema uses the same marker contract, but ambiguous
  unions do not guess.** The trigger is the compiled schema marker
  `format: darkmatter-file`, not the source language that produced it. This
  means raw JSON Schema authors who intentionally use Darkmatter's eager format
  get the same normalization as SimplifiedSchema authors. For property-level
  unions, the rewrite may commit only when exactly one validating arm identifies
  the value as eager-file-typed; zero matching arms or multiple validating eager
  arms leave the value unchanged. For root unions, reuse the same accepted-arm
  selection rule as coercion (`coerce_root_union`) so the rewrite never normalizes
  against a different arm than the one used for type write-back.

## Behavior

```
caller passes:   spec = "fixes/2026-06-26-opencode-yolo/spec.md"   (file(eager)-typed)
                 │
coerce  ─────────┤  (JSON type-shape coercion; file value already a string → no-op)
                 │
validate ────────┤  eager darkmatter-file format: resolve_file_reference(value).is_ok()? 
                 │     └─ fail → ValidationProblem (unchanged); STOP, no rewrite
                 │     └─ ok  → continue
                 │
file-rewrite ────┘  for each present eager-file-typed value:
                       resolved = resolve_arg(value, ctx)        # Some(abs) | None(url) 
                       stored   = make_relative(abs, ctx.base_dir)
                       → "claudine/fixes/2026-06-26-opencode-yolo/spec.md"

document state now: spec = "claudine/fixes/2026-06-26-opencode-yolo/spec.md"
dirname(spec)      = "claudine/fixes/2026-06-26-opencode-yolo"     # agrees
```

## Module Layout & Touchpoints

**Modified / added (darkmatter/lib):**

```
markdown/compose/expression/functions.rs   # lift make_relative into a shared module
                                            #   (resolve is already shared via resolve_ctx)
markdown/compose/expression/path_projection.rs
                                            # or equivalent shared home for repo/base/home
                                            #   relative rendering; expression functions and
                                            #   schema rewrite both call it
markdown/schemas/
├── mod.rs        # file-rewrite pass invoked alongside the coercion write-back,
│                 #   after successful validation, in the same surfaces
├── rewrite.rs    # (new) walk json_schema for the EAGER format: darkmatter-file
│                 #   (skip lazy darkmatter-file-reference); rewrite present local
│                 #   values via the shared resolve+relative helper
markdown/compose/schema_validation.rs       # compose write-back applies the rewrite
                                            #   to effective_frontmatter on success
```

The compose pipeline already persists coerced frontmatter; the rewrite hooks the
same write-back so the normalized `file` values are what downstream
interpolation, lifecycle events, and `inline-compose` see. Library callers get
the same behavior by opting into the new normalization API; `validate` remains a
read-only report API.

**Implementation constraints**

- `rewrite.rs` must not call `std::env::current_dir()` as an implicit anchor.
  It consumes the `ResolutionContext` carried by `EffectiveSchema`/compose and
  uses the current-process CWD only inside existing legacy resolver behavior
  when no anchors were configured.
- The shared relative projection must normalize path separators to `/` before
  storing frontmatter. `Path::to_string_lossy()` would otherwise produce
  backslashes on Windows, making committed Markdown differ by platform.
- The pass must only write keys present in the validation instance. Compose must
  continue excluding `$schema` and `options.exclude_keys` from user schema value
  validation and from rewrite write-back.

## Testing Strategy

**Rewrite unit tests (`rewrite.rs`)**

- A top-level `file(eager)` property with a resolvable relative reference is
  rewritten to the git-root-relative path.
- The rewritten value equals `relative(value)` for the same input (consistency
  with the existing projection).
- A `string` property holding a path-shaped value is **not** rewritten.
- A raw JSON Schema property with `format: darkmatter-file` is rewritten, while
  a property with any other `format` is not.
- A bare (lazy) `file` property is **not** rewritten, even when its value happens
  to resolve to an existing file — only the eager `darkmatter-file` marker is a
  trigger (Decision #1).
- Idempotence: rewriting an already-rewritten value is a no-op (Decision #6).
- An unresolvable `file(eager)` value fails validation and is never rewritten
  (Decision #4) — no partial mutation.
- A `file(eager)` value resolving to a URL/remote reference is left verbatim
  (Decision #7).
- An absent / `null` optional `file(eager)` property is unchanged.
- Nested `file(eager)` (inline-object sub-property), array-of-`file(eager)`, and
  root/property union `file(eager)` arm are each rewritten via the shape walk
  (Decision #8).
- Ambiguous property unions are not rewritten: if more than one validating arm
  could explain the value, the original value is preserved (Decision #9).
- A composition-pending eager `file` value is skipped during pre-shell compose
  validation, while a concrete sibling eager `file` value in the same accepted
  schema can still be rewritten.
- Process-CWD independence: with the CWD mutated to an unrelated dir, the rewrite
  still produces the git-root-relative value (mirrors the
  `#[serial_test::serial("darkmatter-file-cwd")]` convention in `format.rs`).
- Windows separator stability: a rewrite performed on Windows stores
  `repo/relative/path.md`, not `repo\relative\path.md`.

**Integration**

- `md compose` of a doc whose `file(eager)`-typed property started raw shows the
  resolved relative value in the effective frontmatter dump.
- Compose write-back persists the rewritten value (re-`md compose` is a fixpoint).
- An end-to-end review-feature-shaped fixture: `spec: file(eager)`, no
  `{{ctx.area}}/` prefixes, `dirname(spec)` and a derived `review_file` agree and
  resolve.
- Existing validation-only library calls keep returning reports without mutating
  the caller's `serde_json::Value`.

## Downstream Work (separate, required follow-up — claudine)

Once this lands, claudine prompts stop compensating for the gap:

- `prompts/review-feature.md`: type `spec: file(eager, required)` and
  `design: file(eager)` — eager so the value both must exist and is rewritten;
  delete every manual `{{ctx.area}}/` prefix — `@{{spec}}`, `@{{design}}`, and
  `review_file: "{{dir}}/review-{{iteration}}.md"`. The double-prefix bug
  disappears structurally. (`review_file` itself stays bare/lazy `file` — it may
  not exist yet — and is built from the already-rewritten `dir`.)
- Audit sibling prompts (`prompts/implement-suggestions.md`, others using
  `dirname`/`@{{ctx.area}}/{{…}}`) for the same pattern.
- This is tracked as its own claudine change, not in this Darkmatter feature, so
  the library behavior can be reviewed and merged independently.

## Risks

- **Anchor disagreement with read-side resolution.** If the rewrite relativizes
  against one anchor (git root) while a later read resolves against another
  (post-`chdir` ambient CWD, launch area), the stored value could fail to
  resolve. Largely retired now that the path-resolution work has landed: the
  eager `file` validator and the expression functions already resolve through
  one shared `ResolutionContext` (Decision #5), so the rewrite need only consume
  the *same* context. The integration fixture that round-trips a rewritten value
  through a real read remains the backstop.
- **Resolution-context plumbing reaches a previously pure layer.** The rewrite
  pass needs FS access and a base dir where `coerce_frontmatter_with_pending`
  deliberately has none. Keeping the rewrite a *separate* pass (Decision #3)
  preserves that coercion's purity and confines the new dependency.
- **Validation API contract drift.** Existing docs and code promise
  `EffectiveSchema::validate_with_positions` does not mutate input. This spec
  preserves that contract by adding a separate normalization API and having
  compose opt into write-back explicitly.
- **Cross-platform path text drift.** Repo-relative paths are portable only if
  the stored string uses `/` separators. The projection helper must normalize
  separators before frontmatter write-back and tests must cover the Windows
  representation even if they run on macOS/Linux.
- **Persisting resolved paths into committed documents.** The rewritten value is
  repo-relative (portable across machines/OSes), not absolute, so committed
  prompt/spec state stays portable. A non-repo document falls back to
  `base_dir`/`~`/absolute via `make_relative`; absolute is the only
  non-portable outcome and only occurs outside any git root.
- **Behavior change for existing `file(eager)`-typed documents.** Any current
  document already using eager `file` properties will start seeing its stored
  values rewritten (bare/lazy `file` documents are untouched). Because the
  rewrite is the value those documents' own path functions already computed, the
  observable effect is convergence, not breakage; covered by the idempotence and
  consistency tests.

## Open Questions

- ~~Should the rewrite also normalize the value the **caller** sees echoed back
  (e.g. in `initialize`/`start` diagnostics), or only the persisted document
  state?~~ **Resolved:** normalize only document state. Diagnostics interpolate
  from the same state after compose write-back, so they inherit the normalized
  value without adding a second echo/diagnostic rewrite path.
- ~~Sequencing with the path-resolution plan.~~ **Resolved:**
  `@claudine/fixes/_completed/2026-06-27-path-resolution/plan.md` has landed, so
  the shared `ResolutionContext` (`base_dir` + `file_ref_fallback_dir`) already
  exists and the read side resolves through it. This feature now builds *on top
  of* that anchor rather than co-designing it.
