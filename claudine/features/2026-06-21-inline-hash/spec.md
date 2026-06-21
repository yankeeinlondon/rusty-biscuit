# `inline-compose` Document Hashing

**Status:** Draft · **Author:** (pending) · **Date:** 2026-06-21

Stamp a Darkmatter content hash into the `hash:` frontmatter property of every
document an `inline-compose` run rewrites, computed from a **before** and
**after** snapshot of the document so we have a single, self-consistent signal
for changes to *both* frontmatter and body. Fold the existing
"body-unchanged" detection into that same before/after hashing instead of
maintaining a separate one-off hash.

## Goals

1. Every successful `inline-compose` closure writes a `hash:` frontmatter
   property describing the final on-disk document.
2. The `hash:` value is produced by Darkmatter's Markdown-aware hasher (xxHash),
   in the same format `md hash` writes, so `md hash --diff` round-trips against
   it with no false positives.
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
  off BLAKE3 — see [Open Decision 1](#open-decision-1--blake3-harness-fingerprints),
  which is deliberately held out of primary scope.

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
- `MdHashOptions::default()` uses property name `"hash"` and **excludes both
  `hash` and `last_updated`** from the frontmatter segment
  (`hash/options.rs::ignore_set`). This is what makes the stored value a stable
  fixed point: re-running the hash after stamping reproduces the same value
  (`compute.rs` test `ignored_keys_do_not_affect_frontmatter_hash`).

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
`source.markdown.compute_hash(MdHashKind::Simple, &MdHashOptions::default())`
instead of `hash_body(false)`.

> Note on equivalence: the Simple body segment under default (non-strict) options
> is the same underlying value as today's `hash_body(false)` — the migration
> preserves whitespace-normalizing semantics. Verify `MdHashOptions::default()`
> is non-strict during implementation; if not, pass an explicitly non-strict
> options value so detection behavior is unchanged.

### D2 — Body-unchanged detection reads the body segment

In `apply_inline_closure`, compute the replacement's Simple hash once and compare
**only the body segment** to the plan's body segment:

```rust
let post_hash = replacement_markdown.compute_hash(MdHashKind::Simple, &opts);
if post_hash.body_segment() == plan.original_hash.body_segment() {
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
let opts = MdHashOptions::default();
let stored = md.stored_hash();                       // existing value, if re-run
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

### D5 — Frontmatter-change signal (informational)

With both segments available before and after, the closure can report whether the
frontmatter segment changed (`plan.original_hash.fm_segment() != final fm
segment`). Because the closure reverts modified keys and merges only new keys, the
fm segment changes **iff a new frontmatter key was merged** — which the existing
`new_properties` reporting already surfaces. Recommendation: do not add new output
this round; the segment is captured and stored, and the existing property
reporting remains the user-facing signal. The stored fm segment is what makes
later `md hash --diff` tooling able to distinguish fm vs body drift.

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

- **Document already carries `hash:`** — read via `stored_hash()`; Darkmatter
  overwrites in place, preserving key position, and excludes it from computation.
- **Determinism** — `today` is already threaded through the closure; reuse it for
  `apply_hash_save` so tests are deterministic (no wall-clock call inside the
  hasher).
- **Inline targets are always Markdown** — an inline-compose target by definition
  has a `prompt` frontmatter, so Darkmatter's Markdown hasher is always
  appropriate here (unlike the harness file checks below).

## Open Decisions

### Open Decision 1 — BLAKE3 harness fingerprints

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

### Open Decision 2 — stamp on every inline run, or opt-in?

The goals assume **every** inline-compose run stamps `hash:`. If some inline
documents should opt out (e.g. authored docs that manage their own hash), we need
a frontmatter toggle. **Recommendation:** stamp unconditionally; revisit only if a
real opt-out case appears.

## Tests

L1 (library, `composition::closure`):

- Successful inline closure writes a `hash: "<16hex>-<16hex>"` Simple shorthand.
- Re-computing the hash on the written file (`md.compute_hash(Simple, default)`)
  reproduces the stored value — self-reference stability.
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
  stamp before the single atomic write (D3).
- Tests alongside `closure.rs`.
- `docs/topics/composition.md` — document the `hash:` stamping behavior.
- `.claude/skills/claudine/` — note the new behavior if architecture docs cover
  inline-compose output.
