---
status: ready for planning and implementation
created: 2026-06-21
area: claudine
packages:
- claudine
- darkmatter
reviewed: true
review_iterations: 3
hash: 6526f38bd6583936-d7e76324b6b6d6d6
last_updated: 2026-06-25
---

# `inline-compose` Document Hashing

Stamp a Darkmatter content hash into the `hash:` frontmatter property of every
document an `inline-compose` run rewrites, computed from a **before** and
**after** snapshot of the document so we have a single, self-consistent signal
for changes to *both* frontmatter and body. Fold the existing
"body-unchanged" detection into that same before/after hashing instead of
maintaining a separate one-off hash.

## Goals

1. Every successful `inline-compose` closure writes a `hash:` frontmatter
   property describing the final on-disk document.
2. The `hash:` value is produced by Darkmatter's Markdown-aware hasher (xxHash)
   as a forced `MdHashKind::Simple` value (`<frontmatter>-<body>`), so
   `md hash --diff` round-trips against it with no false positives.
3. The body-unchanged error (`InlineBodyUnchanged`) is derived from the same
   before/after hash rather than its own bespoke `u64` — single source of truth.
4. The before/after snapshot exposes a frontmatter-change signal in addition to
   the body-change signal, **but only the body segment gates the error.**

## Non-Goals

- Hashing for `compose` (it never mutates the target file) or for wrapper
  passthrough runs.
- A new "frontmatter changed" *error*. The frontmatter signal is informational
  this round (it can feed the existing new/reverted-property reporting); promoting
  it to a gating check is a separate decision.
- Migrating the harness `file_changed` / `file_unchanged` post-check fingerprints
  off BLAKE3 — see [Deferred Decision 1](#deferred-decision-1--blake3-harness-fingerprints),
  which is deliberately held out of primary scope.
- Honoring `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`, or `--kind`-style hash
  selection for inline-compose. This path is library-owned and deterministic:
  it always writes the standard `hash:` property, ignores only `hash` and
  `last_updated`, and stores a Simple hash.

## Background — two hashers, not one

A point of confusion worth nailing down up front, because it changes what
"switch the detection from BLAKE3 to Darkmatter" actually means:

- The `inline-compose` **body-unchanged detection already uses Darkmatter
  hashing**, not BLAKE3. It is `replacement_markdown.hash_body(false)` compared
  against `plan.original_body_hash` at `lib/src/composition/closure.rs:67`, where
  `original_body_hash` was captured by `source.markdown.hash_body(false)` in
  `lib/src/composition/prepare.rs:310`.
- The **BLAKE3** code is a *separate* facility: the harness `file_changed` /
  `file_unchanged` post-checks, which fingerprint **arbitrary** files (any type,
  not just Markdown) via `biscuit_hash::blake3_hash_bytes` in
  `lib/src/harness/validate/compare.rs:8-27`.

So this spec's primary work is **additive** (stamp a `hash:` property, and DRY
the existing Darkmatter body check). The BLAKE3 swap is a genuinely different
code path and is treated as an open decision, not a foregone change.

## Relevant Darkmatter API

Darkmatter already solves the hard part — the self-reference problem — so we
reuse its high-level API rather than hand-rolling.

- `Markdown::compute_hash(kind: MdHashKind, &MdHashOptions) -> ComputedHash`
  — computes a hash for the requested kind. `MdHashKind::Simple` (the default)
  yields the two-segment `<frontmatter>-<body>` form, each segment 16 hex digits.
- `Markdown::plan_hash_save(stored: Option<&StoredHash>, &MdHashOptions) -> MarkdownResult<SaveDecision>`
  and `Markdown::apply_hash_save(&SaveDecision, &MdHashOptions, today: &str) -> Option<String>`
  — plan then serialize a document with the `hash:` property stamped in place,
  returning `Some(text)` to write or `None` when nothing changed.
- `StoredHash::parse(value: &serde_json::Value, property: &str) -> MarkdownResult<StoredHash>`
  — parse the existing frontmatter `hash:` value before planning a save. The
  current Darkmatter API does **not** expose a `Markdown::stored_hash()` helper;
  inline-compose should follow the same pattern as `darkmatter/cli/src/commands/hash.rs`.
- `MdHashOptions::default()` uses property name `"hash"` and **excludes both
  `hash` and `last_updated`** from the frontmatter segment
  (`hash/options.rs::ignore_set`). This is what makes the stored value a stable
  fixed point: re-running the hash after stamping reproduces the same value
  (`compute.rs` test `ignored_keys_do_not_affect_frontmatter_hash`).

Reader note from review: the draft originally implied `MdHashOptions::default()`
was enough to guarantee a Simple stored hash. It is not: `plan_hash_save`
selects the existing stored kind when one exists. Inline-compose must pass
`MdHashOptions { forced_kind: Some(MdHashKind::Simple), ..MdHashOptions::default() }`
for both baseline capture and final stamping so a document with an older
`structured` or `detailed` hash is deliberately normalized to the Simple format
this feature promises.

### The self-reference subtlety (why this is safe)

Storing a hash *inside* the frontmatter that the hash supposedly covers is
circular. Darkmatter breaks the cycle by filtering `hash` and `last_updated` out
of the frontmatter map before hashing. Consequence the implementer must respect:
**never compute the stored value from the raw frontmatter** — always go through
`compute_hash` / `plan_hash_save`, which apply the ignore-set. The frontmatter
*segment* therefore reflects "every frontmatter key except the two managed ones,"
which is exactly the semantics we want for a change signal.

## Current flow (for reference)

`InlineClosurePlan` (`lib/src/composition/types.rs:539`) carries:

```rust
pub struct InlineClosurePlan {
    pub original_document_text: String,
    pub original_body_hash: u64, // captured via hash_body(false)
}
```

`apply_inline_closure` (`lib/src/composition/closure.rs:52`) runs **after** the
agent exits 0 and **before** post-checks (`harness_orch/loop_control.rs:499`):

1. Reject empty replacement body.
2. `replacement_markdown.hash_body(false) == plan.original_body_hash` →
   `InvalidInlineResponse("replacement body is unchanged")` (surfaced as
   `InlineBodyUnchanged`).
3. `compare_frontmatter` → new keys merged, modified keys reverted.
4. `rewrite_inline_document` assembles the final text **in memory** (preserves
   original frontmatter, merges new props, upserts `last_updated`).
5. `atomic_write(target_path, doc_string)` — one atomic write.

No `hash:` property is read or written anywhere in this path today.

## Design

### D1 — Capture a Darkmatter "before" snapshot in the plan

Replace the bare `original_body_hash: u64` with the full pre-run computed hash so
both segments are available downstream:

```rust
pub struct InlineClosurePlan {
    pub original_document_text: String,
    /// Pre-run Simple hash (`<fm>-<body>`) of the source document, computed with
    /// `MdHashOptions::default()` so the `hash`/`last_updated` keys are excluded.
    pub original_hash: ComputedHash, // MdHashKind::Simple
}
```

Built in `prepare_inline` (`prepare.rs:310`) by calling
`source.markdown.compute_hash(MdHashKind::Simple, &inline_hash_options())`
instead of `hash_body(false)`, where `inline_hash_options()` returns:

```rust
MdHashOptions {
    forced_kind: Some(MdHashKind::Simple),
    ..MdHashOptions::default()
}
```

> Note on equivalence: the Simple body segment under default (non-strict) options
> is the same underlying value as today's `hash_body(false)` — the migration
> preserves whitespace-normalizing semantics. `MdHashOptions::default()` is
> currently non-strict; the helper above must leave `strict: false` unless this
> feature intentionally changes unchanged-body semantics.

Implementation detail: `ComputedHash` currently exposes the Simple segments as
enum fields rather than accessor methods. Add a small local helper or upstream
`simple_segments()` accessor if that keeps the closure code clearer; do not
string-split `flat_string()` to recover the segments.

### D2 — Body-unchanged detection reads the body segment

In `apply_inline_closure`, compute the replacement's Simple hash once and compare
**only the body segment** to the plan's body segment:

```rust
let post_hash = replacement_markdown.compute_hash(MdHashKind::Simple, &opts);
if simple_body(&post_hash) == simple_body(&plan.original_hash) {
    return Err(CompositionError::InvalidInlineResponse(
        "replacement body is unchanged".into(),
    ));
}
```

Semantics are identical to today: frontmatter differences never trip this error.
(`ComputedHash` segment accessors may need a small helper; the `Simple` variant
already holds `{ fm, body }`.)

### D3 — Stamp `hash:` on the final document (single atomic write)

`rewrite_inline_document` already produces the final document **as an in-memory
string** before the write at `closure.rs:96`. Slot the stamp between assembly and
write so there is still exactly **one** atomic write:

```rust
let doc_string = rewrite_inline_document(/* ... as today ... */)?;

// Stamp the Darkmatter hash into `hash:` in place.
let md: darkmatter::markdown::Markdown = doc_string.into();
let opts = inline_hash_options();
let stored = parse_inline_stored_hash(&md, &opts)?;  // existing value, if re-run
let decision = md.plan_hash_save(stored.as_ref(), &opts)?;
let final_text = md
    .apply_hash_save(&decision, &opts, today)
    .unwrap_or_else(|| md.to_string());              // None ⇒ no change to stamp

crate::config::atomic::atomic_write(target_path, final_text.as_bytes())
    .map_err(|e| CompositionError::AtomicWriteFailed(e.to_string()))?;
```

The stored format for `MdHashKind::Simple` with no extra-ignored keys is the
shorthand string, e.g.:

```yaml
hash: "9f1c0a2b3d4e5f60-71a2b3c4d5e6f708"
```

`parse_inline_stored_hash` should mirror Darkmatter CLI behavior:

```rust
fn parse_inline_stored_hash(
    md: &Markdown,
    opts: &MdHashOptions,
) -> MarkdownResult<Option<StoredHash>> {
    match md.frontmatter().as_map().get(opts.property.as_str()) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => StoredHash::parse(value, &opts.property).map(Some),
    }
}
```

If the existing `hash:` property is malformed, fail the closure with a typed
composition error that preserves the Darkmatter `MalformedStoredHash` reason.
Do not silently overwrite a malformed hash: `md hash --save` treats that as an
operational error, and inline-compose should not weaken the stored-hash contract.

### D4 — `last_updated` ownership

`rewrite_inline_document` already upserts `last_updated` to `today`, and
`apply_hash_save` also manages `last_updated`. These do **not** conflict because
the hash computation excludes `last_updated`, so order is irrelevant to the
stored value. Behavior:

- First-ever stamp (no prior `hash:`): `apply_hash_save` writes `hash:` and, by
  Darkmatter's "first baseline doesn't bump `last_updated`" rule, leaves the date
  alone — which is fine, the closure already set it to `today`.
- Re-run (prior `hash:` present): content changed (new body), so the date resolves
  to `today` from both writers — same value, idempotent.

Keep the closure's existing `last_updated`/frontmatter-merge logic; do **not**
refactor `last_updated` ownership into Darkmatter in this change (out of scope,
Rule 3).

### D4.1 — Existing non-Simple hashes are downgraded intentionally

Because this feature promises a Simple `hash:` value, inline-compose forces
`MdHashKind::Simple` even when the existing stored hash is `structured` or
`detailed`. Side effects:

- A document with a valid non-Simple stored hash is rewritten to Simple on the
  next successful inline-compose closure.
- `last_updated` should bump only when Darkmatter's lower-resolution comparison
  reports content drift; a kind downgrade by itself must not bump the date.
- After the rewrite, ordinary `md hash --diff` should compare against the stored
  Simple value and exit 0.

This is an intended standards change for inline-compose outputs, not an
accidental loss of Darkmatter capability. The mitigation is explicit scope:
manual `md hash --kind structured --save` / `--kind detailed --save` remains
available for authored documents, but inline-compose owns its generated output
baseline and always stores the cheaper two-segment signal.

### D5 — Frontmatter-change signal (informational)

With both segments available before and after, the closure can report whether the
frontmatter segment changed (`simple_fm(&plan.original_hash) != simple_fm(&final_hash)`).
Compute `final_hash` from the final stamped document with the same forced-Simple
options; the managed `hash`/`last_updated` keys are ignored, so the signal is not
polluted by the stamp itself. Because the closure reverts modified keys and
merges only new keys, the fm segment changes **iff a new frontmatter key was
merged** — which the existing `new_properties` reporting already surfaces.
Recommendation: do not add new output this round; the segment is captured and
stored, and the existing property reporting remains the user-facing signal. The
stored fm segment is what makes later `md hash --diff` tooling able to
distinguish fm vs body drift.

## Scope of behavior

- Applies wherever the **inline closure** runs: a direct `claudine inline-compose`
  run, and any `claudine sequence` step that is an inline-compose step (same
  closure path).
- `compose` is unaffected (no file mutation).
- Stamping is a **file mutation, not terminal output** — it happens regardless of
  `--silent`, `show_checks`, or TTY state.
- Skipped exactly when the closure is skipped today: non-zero agent exit, user
  interrupt, empty body, or unchanged body (all short-circuit before the write).

## Edge cases

- **Document already carries `hash:`** — parse the existing value with
  `StoredHash::parse`; Darkmatter overwrites in place, preserving key position,
  and excludes it from computation.
- **Document carries malformed `hash:`** — fail before the atomic write with a
  typed composition error derived from `MarkdownError::MalformedStoredHash`.
  This matches `md hash --save` and prevents inline-compose from silently
  erasing evidence of a corrupted baseline.
- **Document carries non-Simple `hash:`** — valid `structured`, `detailed`, `fm`,
  or `body` stored hashes are accepted as the comparison baseline but the closure
  writes back Simple. This is the deliberate normalization described in
  [D4.1](#d41--existing-non-simple-hashes-are-downgraded-intentionally).
- **Frontmatter-less intermediate document** — `replacement_markdown` used for
  unchanged-body detection may have no frontmatter. That is fine: absent and
  empty frontmatter hash to the same stable fm segment, and only the body segment
  participates in the error check.
- **Determinism** — `today` is already threaded through the closure; reuse it for
  `apply_hash_save` so tests are deterministic (no wall-clock call inside the
  hasher).
- **Inline targets are always Markdown** — an inline-compose target by definition
  has a `prompt` frontmatter, so Darkmatter's Markdown hasher is always
  appropriate here (unlike the harness file checks below).

## Deferred and Resolved Decisions

### Deferred Decision 1 — BLAKE3 harness fingerprints

`file_changed` / `file_unchanged` post-checks fingerprint arbitrary files with
BLAKE3 (`validate/compare.rs`). These are cooperative correctness checks, not an
adversarial integrity boundary, so the cryptographic property buys nothing and is
inconsistent with the xxHash used everywhere else. Three ways forward:

- **(a) Defer** — leave BLAKE3; ship only the inline-stamp work. Lowest risk.
- **(b) Uniform xxHash** — swap to `biscuit_hash` xxHash bytes for all files.
  Byte-exact change detection, minimal semantic change, kills the inconsistency.
- **(c) Markdown-aware** — Darkmatter Simple/Body hash when the target is `.md`
  (ignoring cosmetic whitespace **and** the managed `hash`/`last_updated` keys),
  `biscuit_hash` xxHash otherwise. Most "correct," but it **changes the meaning**
  of `file_changed`/`file_unchanged` on Markdown: after this spec stamps a
  `hash:`/`last_updated`, a `file_unchanged: @doc` check would *not* flag the
  stamp as a change. That interaction is arguably desirable but must be decided
  and tested deliberately.

**Recommendation:** (a) for this spec; track (b)/(c) as a follow-up so the BLAKE3
decision isn't smuggled in alongside the inline-stamp change.

### Resolved Decision 2 — stamp on every inline run

Every successful inline-compose closure stamps `hash:` unconditionally. No
frontmatter opt-out is added in this feature.

Rejected alternative: add a `hash: false` or `inline_hash: false` opt-out. It
would preserve more author control, but it also creates a second class of
inline-compose output that cannot participate in the same `md hash --diff`
workflow. No current use case needs that split, so unconditional stamping is the
simpler contract.

## Tests

L1 (library, `composition::closure`):

- Successful inline closure writes a `hash: "<16hex>-<16hex>"` Simple shorthand.
- Re-computing the hash on the written file (`md.compute_hash(Simple, default)`)
  reproduces the stored value — self-reference stability.
- A document with an existing valid `structured` or `detailed` hash is normalized
  to Simple and still passes `md hash --diff`.
- A document with malformed `hash:` fails with a typed composition error and does
  not perform the final atomic write.
- Body-unchanged still errors via the body-segment comparison (port the existing
  `apply_inline_closure_rejects_unchanged_body` test to the new path).
- Adding a new frontmatter key changes the fm segment; reverting a modified key
  does not.
- Idempotency: stamping an already-stamped, otherwise-unchanged document does not
  perpetually bump `last_updated`.
- Determinism: fixed `today` ⇒ byte-stable output.

L2 (CLI, optional): a real `claudine inline-compose` run against a fixture leaves
a valid `hash:` that `md hash --diff` reports as unchanged (exit 0).

## Touch list

- `lib/src/composition/types.rs:539` — `InlineClosurePlan` field change.
- `lib/src/composition/prepare.rs:310` — capture `compute_hash(Simple)` baseline.
- `lib/src/composition/closure.rs:52-103` — body-segment detection (D2) + hash
  stamp before the single atomic write (D3), including local helpers for forced
  Simple options, stored-hash parsing, and Simple segment extraction.
- Tests alongside `closure.rs`.
- `docs/topics/composition.md` — document the `hash:` stamping behavior.
- `.claude/skills/claudine/` — note the new behavior if architecture docs cover
  inline-compose output.