---
agent: open_code/zai-coding-plan/glm-5.2
phases: 6
created: 2026-06-22
start_phase: 1
yolo: 'true'
source_files_during_phase_1:
- claudine/lib/src/composition/error.rs
- claudine/lib/src/composition/types.rs
- claudine/lib/src/composition/closure.rs
- claudine/lib/src/composition/prepare.rs
- claudine/cli/src/commands/wrap/inline.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- claudine/lib/src/composition/closure.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- claudine/lib/src/composition/closure.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- claudine/lib/src/composition/closure.rs
- claudine/cli/tests/inline_compose_hash.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
- claudine/docs/topics/composition.md
- claudine/docs/topics/frontmatter-properties.md
- claudine/docs/topics/execution-flow.md
- .opencode/skill/claudine/timeline.md
- .opencode/skill/claudine/validations-and-handlers.md
- .opencode/skill/claudine/SKILL.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .opencode/skill/claudine/timeline.md
- .opencode/skill/claudine/validations-and-handlers.md
- .opencode/skill/claudine/SKILL.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
- claudine/features/2026-06-21-inline-hash/spec.md
- claudine/features/2026-06-21-inline-hash/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
- claudine/lib/src/composition/error.rs
- claudine/lib/src/composition/types.rs
- claudine/lib/src/composition/closure.rs
- claudine/lib/src/composition/prepare.rs
- claudine/cli/src/commands/wrap/inline.rs
- claudine/cli/tests/inline_compose_hash.rs
documentation:
- claudine/docs/topics/composition.md
- claudine/docs/topics/frontmatter-properties.md
- claudine/docs/topics/execution-flow.md
- .opencode/skill/claudine/timeline.md
- .opencode/skill/claudine/validations-and-handlers.md
- .opencode/skill/claudine/SKILL.md
hash: f5ebbd71478c385c-88fa126c6d8d6b36
packages:
- claudine
- claudine-cli
last_updated: 2026-06-22
---

# Execution Plan — `inline-compose` Document Hashing

Implements [`spec.md`](spec.md): stamp a Darkmatter `Simple` content hash into
the `hash:` frontmatter property of every document an `inline-compose` run
rewrites, and fold the existing body-unchanged detection into the same
before/after `ComputedHash` instead of a bespoke `u64`.

## Plan summary

| Phase | Scope                                                  | Spec refs      |
| ----- | ------------------------------------------------------ | -------------- |
| 1     | Capture full pre-run `ComputedHash` + migrate D2 check | D1, D2         |
| 2     | Stamp `hash:` on the final atomic write               | D3, D4, D4.1   |
| 3     | Capture frontmatter-change signal (informational)     | D5             |
| 4     | Comprehensive L1 + L2 validation                       | Tests section  |
| 5     | Documentation, frontmatter catalog, skill updates     | Touch list     |
| 6     | Final close-out and plan integrity                      | Plan + spec    |

**Dependency shape:** Phase 1 is the atomic foundation (type change touches 10
sites). Phases 2 and 3 are sequential (both edit `apply_inline_closure`).
Phase 5 (docs) can start once Phase 2 lands and may run in parallel with
Phases 3-4. Phase 4 tests are layered: idempotency/determinism depend only on
Phase 2; the fm-signal tests depend on Phase 3.

## Decisions settled by this plan

- **D-Helpers placement:** add `inline_hash_options()`, `parse_inline_stored_hash()`,
  `simple_body()`, `simple_fm()` as **local private helpers in
  `lib/src/composition/closure.rs`** (per spec touch list and Rule 3 — no
  cross-crate refactor). Do **not** add `simple_segments()` to Darkmatter in
  this change; the local match-on-`ComputedHash::Simple { .. }` is two lines.
  If a third caller appears later, revisit.
- **New error variant:** add `CompositionError::InlineHashMalformed {
  #[source] source: MarkdownError }` in `lib/src/composition/error.rs`, rendered
  through the existing `BlockError`/`StatusBlock` walker so Darkmatter's
  `malformed_stored_hash_block` shows. Matches the
  `FrontmatterParse(#[source] MarkdownError)` precedent.
- **Atomic type swap, no shim:** Phase 1 changes the `InlineClosurePlan` field
  type and updates all 10 construction/read sites in one commit. A temporary
  compat field would only delay the same mechanical sweep.
- **`MdHashOptions` is forced to `Simple`:** `inline_hash_options()` returns
  `MdHashOptions { forced_kind: Some(MdHashKind::Simple), ..MdHashOptions::default() }`
  so a pre-existing `structured`/`detailed` stored hash is normalized to
  `Simple` on the next inline-compose run (D4.1).
- **`last_updated` ownership unchanged:** `rewrite_inline_document` keeps its
  existing upsert; `apply_hash_save` also manages it but both converge on
  `today`. No refactor into Darkmatter (out of scope, Rule 3).

## Source-of-truth references (verified against `main`)

- Type: [`InlineClosurePlan`](../../lib/src/composition/types.rs:539) holds
  `original_document_text: String, original_body_hash: u64`.
- Capture: [`prepare_inline`](../../lib/src/composition/prepare.rs:310) sets
  `original_body_hash = source.markdown.hash_body(false)`.
- Closure: [`apply_inline_closure`](../../lib/src/composition/closure.rs:53)
  reads `plan.original_body_hash` at L67, assembles `doc_string` at L88-94,
  and atomic-writes at L96-97.
- Darkmatter API: `Markdown::compute_hash`, `plan_hash_save`,
  `apply_hash_save`, `StoredHash::parse` are all re-exported from
  [`darkmatter::markdown::hash::*`](../../../darkmatter/lib/src/markdown/hash/mod.rs:23)
  and `MarkdownError` from `darkmatter::markdown`.
- CLI pattern to mirror: [`parse_stored_hash`](../../../darkmatter/cli/src/commands/hash.rs:115).

---

## Phase 1 — Capture full pre-run `ComputedHash` and migrate body detection

**Goal:** replace the bare `u64` body hash with a full pre-run
`ComputedHash::Simple { .. }`, and read the **body segment** for the
unchanged-body check. Behavior must be byte-identical to today.

**Why first:** every later phase reads the new field. Splitting the type change
from the reader migration would leave the crate uncompilable, so D1 + D2 ship
together as one atomic foundation.

**Files:**

- `lib/src/composition/types.rs`
- `lib/src/composition/prepare.rs`
- `lib/src/composition/closure.rs`
- `lib/src/composition/error.rs` (new variant only; rendering wiring optional
  here, mandatory before Phase 2 closure)
- `cli/src/commands/wrap/inline.rs` (test call sites only)

### Tasks

- [x] Add `CompositionError::InlineHashMalformed { #[source] source: MarkdownError }` to `lib/src/composition/error.rs` with a `#[error("malformed stored `hash` property: {0}")]` message, and wire it through the existing `BlockError`/`StatusBlock` rendering walker (render `MalformedStoredHash` via Darkmatter's `malformed_stored_hash_block` — mirror `FrontmatterParse`'s pattern at error.rs:60).
- [x] In `lib/src/composition/closure.rs`, add private helpers near the top of the "Private helpers" section:
  - `fn inline_hash_options() -> MdHashOptions` returning `MdHashOptions { forced_kind: Some(MdHashKind::Simple), ..MdHashOptions::default() }`.
  - `fn simple_body(hash: &ComputedHash) -> &str` and `fn simple_fm(hash: &ComputedHash) -> &str` that match on `ComputedHash::Simple { body, .. }` / `{ fm, .. }` and panic-on-unreachable for non-Simple (the closure only ever stores Simple; document that invariant in the doc comment).
- [x] In `lib/src/composition/types.rs:539`, replace `pub original_body_hash: u64` with `pub original_hash: ComputedHash`. Update the doc comment to note it is always `MdHashKind::Simple` and computed via `inline_hash_options()` so `hash`/`last_updated` are excluded.
- [x] In `lib/src/composition/prepare.rs:310`, replace `let original_body_hash = source.markdown.hash_body(false);` with `let original_hash = source.markdown.compute_hash(MdHashKind::Simple, &inline_hash_options());` and update the `InlineClosurePlan { ... }` literal at L319-322. Import `compute_hash` / `MdHashKind` via the existing `darkmatter::markdown::...` use group; import `inline_hash_options` from `super::closure` (or `crate::composition::closure`).
- [x] In `lib/src/composition/closure.rs:67`, migrate the body-unchanged check:
  ```rust
  let replacement_markdown: darkmatter::markdown::Markdown = replacement_body.to_string().into();
  let post_hash = replacement_markdown.compute_hash(MdHashKind::Simple, &inline_hash_options());
  if simple_body(&post_hash) == simple_body(&plan.original_hash) {
      return Err(CompositionError::InvalidInlineResponse(
          "replacement body is unchanged".into(),
      ));
  }
  ```
  Remove the now-dead standalone `hash_body(false)` call on `replacement_markdown`.
- [x] Update every `InlineClosurePlan { original_document_text: ..., original_body_hash: ... }` construction in tests to use `original_hash: <md>.compute_hash(MdHashKind::Simple, &inline_hash_options())`:
  - `lib/src/composition/closure.rs:378-381, 512-514, 717-719, 765-767, 791-793` (5 sites in closure tests)
  - `lib/src/composition/prepare.rs:609, 1015` (2 sites — these read `plan.original_body_hash`; rewrite to assert on `simple_body(&plan.original_hash)` or assert `!= ""`/length 16)
  - `cli/src/commands/wrap/inline.rs:266-268, 309-311` (2 sites)
- [x] Verify `cargo check -p claudine` and `cargo check -p claudine-cli` compile cleanly.

### Validation checkpoint

- [x] `just test claudine` (or `cargo nextest run -p claudine`) — all existing tests green.
- [x] Specifically confirm `apply_inline_closure_rejects_unchanged_body` still fires with the "replacement body is unchanged" message (port its assertions verbatim; the spec lists this as a required port).
- [x] `just lint claudine` — no new warnings from dead `hash_body` imports.

---

## Phase 2 — Stamp `hash:` on the final atomic write

**Goal:** every successful inline closure writes a forced-`Simple`
`hash: "<fm>-<body>"` frontmatter property in the **same** atomic write that
persists the body. Malformed existing hashes fail before the write. Existing
non-Simple stored hashes are normalized to Simple.

**Files:**

- `lib/src/composition/closure.rs` (primary)
- `lib/src/composition/error.rs` (rendering of `InlineHashMalformed` — complete
  if not finished in Phase 1)

### Tasks

- [x] Add private `fn parse_inline_stored_hash(md: &Markdown, opts: &MdHashOptions) -> MarkdownResult<Option<StoredHash>>` in `closure.rs`, mirroring [`darkmatter/cli/src/commands/hash.rs:115`](../../../darkmatter/cli/src/commands/hash.rs:115) exactly: `None | Value::Null` → `Ok(None)`, otherwise `StoredHash::parse(value, &opts.property).map(Some)`.
- [x] In `apply_inline_closure`, slot the stamp between `rewrite_inline_document` (L88-94) and `atomic_write` (L96-97). Keep **exactly one** `atomic_write` call. Sketch from spec D3:
  ```rust
  let doc_string = rewrite_inline_document(/* ... unchanged ... */)
      .map_err(CompositionError::InvalidInlineResponse)?;

  let md: darkmatter::markdown::Markdown = doc_string.into();
  let opts = inline_hash_options();
  let stored = parse_inline_stored_hash(&md, &opts)
      .map_err(CompositionError::InlineHashMalformed)?;
  let decision = md.plan_hash_save(stored.as_ref(), &opts)
      .map_err(CompositionError::InlineHashMalformed)?;
  let final_text = md.apply_hash_save(&decision, &opts, today)
      .unwrap_or_else(|| md.to_string()); // None ⇒ no change to stamp

  crate::config::atomic::atomic_write(target_path, final_text.as_bytes())
      .map_err(|e| CompositionError::AtomicWriteFailed(e.to_string()))?;
  ```
  Note: `apply_hash_save` returns `Option<String>`; `None` means the decision
  was "leave file untouched" — but because the closure already mutated the body
  (D2 guaranteed a body change), the realistic path is `Some`. The
  `unwrap_or_else(|| md.to_string())` is a defensive fallback that writes the
  already-stamped document verbatim.
- [x] Confirm `today` is threaded through unchanged (closure signature already takes `today: &str`); do **not** introduce a wall-clock call inside the stamp path.
- [x] Audit `last_updated` semantics per spec D4 + D4.1:
  - First-ever stamp (no prior `hash:`): `apply_hash_save` writes `hash:`, leaves date alone (closure already set `today`). ✅ already correct.
  - Re-run with body change: both writers set `today` — idempotent. ✅ already correct.
  - **Kind downgrade** (valid `structured`/`detailed` stored hash present): `MdHashKind::relate` returns `KindRelation::Lower`, `plan_hash_save` writes the new Simple baseline with `bump_last_updated: false` (verified at `save.rs:95,113`). Confirm via test in Phase 4.
- [x] Do **not** refactor `rewrite_inline_document`'s `last_updated` upsert — out of scope (Rule 3, D4).

### Validation checkpoint

- [x] L1 test: successful inline closure writes `hash: "<16hex>-<16hex>"` (regex `^[0-9a-f]{16}-[0-9a-f]{16}$`).
- [x] L1 test: re-computing `md.compute_hash(MdHashKind::Simple, &inline_hash_options())` on the written file reproduces the stored `hash:` value byte-for-byte (self-reference stability).
- [x] L1 test: a document with a pre-existing valid `structured` or `detailed` `hash:` is rewritten to the Simple shorthand and `md hash --diff` (via `md.compare_hash(...)`) reports unchanged.
- [x] L1 test: a document with a malformed `hash:` (e.g. `hash: "not-a-hash"`) fails with `CompositionError::InlineHashMalformed` and **the file on disk is unchanged** (assert byte-equal to the pre-run snapshot — proves the failure path runs before `atomic_write`).
- [x] `just test claudine` green; `just lint claudine` clean.

---

## Phase 3 — Capture frontmatter-change signal (informational)

**Goal:** with both segments available before and after, compute the
fm-segment-change signal and stash it on `InlineClosureResult` so later
tooling can distinguish fm drift from body drift. **No new user-facing output
this round** (spec D5 explicit).

**Files:**

- `lib/src/composition/closure.rs`

### Tasks

- [x] Extend `InlineClosureResult` (`closure.rs:43`) with `pub frontmatter_changed: bool`. Derive `Default` continues to hold.
- [x] In `apply_inline_closure`, after the stamp produces `final_text`, compute the final Simple hash and compare the fm segment:
  ```rust
  let final_md: darkmatter::markdown::Markdown = final_text.clone().into();
  let final_hash = final_md.compute_hash(MdHashKind::Simple, &opts);
  let frontmatter_changed = simple_fm(&final_hash) != simple_fm(&plan.original_hash);
  ```
  Return it in the `InlineClosureResult`. The managed `hash`/`last_updated`
  keys are excluded by `inline_hash_options()`, so the stamp itself cannot
  pollute the signal.
- [x] Audit existing `InlineClosureResult` consumers — `new_properties`/`reverted_properties` remain the user-facing signal. `frontmatter_changed` is a future-facing field; do not surface it in CLI output this phase.

### Validation checkpoint

- [x] L1 test: adding a **new** frontmatter key in the post-run map flips `frontmatter_changed = true`.
- [x] L1 test: **reverting** a modified key (the existing `apply_closure_merges_new_and_reports_reverted` fixture already exercises this) leaves `frontmatter_changed = false` — because the closure restores the original value, so the fm segment matches the baseline.
- [x] L1 test: a body-only change (same frontmatter) leaves `frontmatter_changed = false`.
- [x] `just test claudine` green.

---

## Phase 4 — Comprehensive L1 + L2 validation suite

**Goal:** land every test enumerated in the spec's "Tests" section, plus the
port of the existing unchanged-body test to the new code path (if not already
covered in Phase 1).

**Files:**

- `lib/src/composition/closure.rs` (co-located `#[cfg(test)] mod tests`)
- `cli/tests/` (new L2 if added)

### Tasks (parallelizable with Phase 5)

The idempotency/determinism group depends only on Phase 2; the fm-signal group
depends on Phase 3. Both may proceed in parallel once their respective phase
lands.

**Depends on Phase 2:**

- [x] Port `apply_inline_closure_rejects_unchanged_body` explicitly onto the body-segment comparison (if the port in Phase 1 was only mechanical, add an assertion that the body segment of `plan.original_hash` is non-empty and 16 hex chars).
- [x] Idempotency: stamping an already-stamped, otherwise-unchanged document does not perpetually bump `last_updated`. (Construct a plan whose `original_hash` already matches a stored `hash:` value; assert `apply_hash_save` returns `bump_last_updated: false`.)
- [x] Determinism: fixed `today` ⇒ byte-stable output across two `apply_inline_closure` invocations on the same inputs.

**Depends on Phase 3:**

- [x] Adding a new frontmatter key changes the fm segment; reverting a modified key does not (covered in Phase 3 checkpoint — confirm still green here in the consolidated suite).

**L2 (CLI, optional but recommended):**

- [x] Add `cli/tests/inline_compose_hash.rs` (or extend an existing inline-compose L2) that runs a real `claudine inline-compose` against a fixture and asserts `md hash --diff` on the written file exits `0`. Use the `biscuit-test-harness` patterns from `.claude/skills/rust-testing/SKILL.md`; gate behind `just test-l2`.

### Validation checkpoint

- [x] `just test claudine` (L1) green; `just test-l2 claudine` (if L2 added) green.
- [x] `just lint claudine` clean — no dead code, no unused imports.
- [x] Full cargo workspace check: `cargo check --workspace` (the type change in Phase 1 can ripple in unexpected places; this is the final safety net).

---

## Phase 5 — Documentation, frontmatter catalog, and skill updates

**Goal:** every external surface a user might read to understand inline-compose
output mentions the new `hash:` stamping behavior.

**Files:**

- `claudine/docs/topics/composition.md`
- `claudine/docs/topics/frontmatter-properties.md`
- `claudine/docs/topics/execution-flow.md` (Step 9 — Inline Closure section)
- `.claude/skills/claudine/timeline.md`
- `.claude/skills/claudine/SKILL.md` (only if architecture/output docs reference inline-compose write semantics)
- `.claude/skills/claudine/validations-and-handlers.md` (the "inline-compose is the most stateful mode" paragraph mentions `last_updated`; add `hash:`)

### Tasks (parallelizable with Phases 3-4)

- [x] In `composition.md`, add a subsection under the inline-compose closure description (near L123-131 where `last_updated` is documented) titled **"`hash` property (auto-stamped)"** covering: forced Simple kind, `<fm>-<body>` 16-hex-per-segment format, exclusion of `hash`/`last_updated` from the fm segment (self-reference stability), `md hash --diff` round-trip, the deliberate non-Simple → Simple downgrade (D4.1), and the malformed-hash failure mode.
- [x] In `frontmatter-properties.md`, add a row for `hash` after the `last_updated` row (L14): "Auto-stamped Darkmatter `Simple` content hash (`<fm>-<body>`, 16 hex per segment) written on every successful `inline-compose` closure. Computed with `hash` and `last_updated` excluded, so re-running `inline-compose` on an already-stamped unchanged document is a fixed point. Forced to `Simple` regardless of any pre-existing `structured`/`detailed` value." Link to `composition.md` and to `InlineClosurePlan` / `apply_inline_closure`.
- [x] In `execution-flow.md` Step 9 (L499-548), insert a bullet after the `last_updated` bullet (L526): "Stamps a Darkmatter `Simple` content hash into the `hash:` frontmatter property (see [Composition — `hash` property](composition.md#hash-property-auto-stamped))". Update the "Frontmatter `last_updated`" row in the side-effects table (L548) to add a parallel "Frontmatter `hash`" row.
- [x] In `.claude/skills/claudine/timeline.md`, add a dated entry (top of list) summarizing: forced Simple hash stamp on every inline-compose closure, single atomic write, malformed-hash failure mode, non-Simple downgrade, DRY of the body-unchanged detection onto the body segment. Link to this plan and to `composition.md#hash-property-auto-stamped`.
- [x] In `.claude/skills/claudine/validations-and-handlers.md` (L479 paragraph), append: "and stamps a Darkmatter `Simple` hash into `hash:` (see [Composition — `hash` property](../../../claudine/docs/topics/composition.md#hash-property-auto-stamped))."
- [x] Audit `.claude/skills/claudine/SKILL.md` — if its composition blurb (L85, L155) mentions `last_updated` as the only auto-managed property, extend to mention `hash:`. If it only references composition at a high level, no change needed.
- [x] **Hash the changed Markdown docs.** Per repo convention (AGENTS.md "Hashing Content"), run `md hash --save` on every doc file edited in this phase so its `hash:` frontmatter reflects the new content. This is also a live smoke test of the feature: after editing, `md hash --diff` should report drift; after `--save`, it should exit 0.

### Validation checkpoint

- [x] Every edited doc's `hash:` frontmatter value matches `md hash --diff` (exit 0).
- [x] Cross-link audit: every "see composition.md" link resolves to an anchor that exists.
- [x] `just test claudine` still green (docs changes should not affect compilation, but confirm the workspace check from Phase 4 still passes).

---

## Phase 6 — Final close-out and plan integrity

**Goal:** finalize the feature's planning documents, ensure every Markdown file
 touched by the feature carries a current Darkmatter `hash:`, and run the final
 verification suite.

**Files:**

- `claudine/features/2026-06-21-inline-hash/spec.md`
- `claudine/features/2026-06-21-inline-hash/plan.md`

### Tasks

- [x] Hash `spec.md` with `md hash --save` so its `hash:` frontmatter reflects the
  final reviewed content.
- [x] Update this plan's frontmatter: set `phases: 6`, add `source_files_during_phase_6`,
  `docs_updated_during_phase_6`, `docs_created_during_phase_6`, and
  `skills_files_updated_during_phase_6`; refresh `last_updated`.
- [x] Hash this `plan.md` with `md hash --save` so its `hash:` frontmatter reflects
  the Phase 6 additions.
- [x] Run `just test claudine` and confirm all tests pass.
- [x] Run `just lint claudine` and confirm no new warnings.

### Validation checkpoint

- [x] `md hash --diff` exits 0 for `spec.md` and `plan.md`.
- [x] `just test claudine` green.
- [x] `just lint claudine` clean.

---

## Out-of-scope reminders (do not do)

- Do **not** migrate the harness `file_changed` / `file_unchanged` BLAKE3 fingerprints (spec Deferred Decision 1 — explicitly held out).
- Do **not** add a `hash: false` or `inline_hash: false` frontmatter opt-out (spec Resolved Decision 2).
- Do **not** honor `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`, or `--kind` for inline-compose (spec Non-Goals).
- Do **not** add a "frontmatter changed" *error* — `frontmatter_changed` in Phase 3 is informational only.
- Do **not** refactor `last_updated` ownership into Darkmatter.
- Do **not** run `cargo fmt` in write mode (AGENTS.md formatting rule).