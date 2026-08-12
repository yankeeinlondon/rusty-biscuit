---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-30
start_phase: 1
yolo: "false"
packages:
    - darkmatter
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/functions.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/cli/tests/compose_schema_file_rewrite.rs
docs_updated_during_phase_4:
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .opencode/skill/darkmatter/SKILL.md
source_code:
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/functions.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/cli/tests/compose_schema_file_rewrite.rs
documentation:
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schema-definition.md
---

# Execution Plan — Rewrite `file`-Typed Schema Property Values to Their Resolved Relative Path

Implements [`spec.md`](spec.md). Converts the eager `darkmatter-file` format
validator's "validate-and-discard" behavior into "validate-and-rewrite" so an
eager `file(eager)`-typed frontmatter property is stored as the resolved
repo-relative path — the same projection `relative(value)`/`dirname(value)`
already produce.

The plan is layered so each phase is independently testable and reviewable.
Phase 1 unblocks Phases 2–4; Phases 2 and 3 are sequential (the API in 3 wraps
the engine in 2); Phase 4 depends on 3.

## Conventions Used Throughout

- **Trigger marker:** `format: darkmatter-file` (eager). The lazy
  `darkmatter-file-reference` marker is **never** a trigger (Decision #1).
- **Shared projection:** `resolve_arg(value, ctx)` →
  `make_relative(abs, base_dir)`, both currently private to
  `markdown/compose/expression/functions.rs` (`functions.rs:1054`,
  `functions.rs:1143`). Resolution is already shared via
  `resolve_ctx::resolve_file_ref_with_fallback`; only `make_relative` needs
  lifting.
- **Read-only preservation:** `EffectiveSchema::validate` and
  `validate_with_positions` keep their current contracts (coerce on a working
  copy, mutate no input). Normalization is a separate explicit API.
- **Anchor reuse:** Consume the same `ResolutionContext { base_dir,
  file_ref_fallback_dir }` already carried by `EffectiveSchema`
  (`schemas/mod.rs:285`) — no new anchor plumbing.
- **Tests** use `just test` (L1 unit) and `just test-l2` (L2 integration) in
  the `darkmatter` package area, per the `rust-testing` skill. CWD-sensitive
  tests follow the `#[serial_test::serial("darkmatter-file-cwd")]` convention
  already used in `schemas/format.rs`.

---

## Phase 1 — Extract the Shared Relativization Helper

**Goal:** Lift `make_relative` out of `functions.rs` into a small shared
module so the expression path and the new schema rewrite pass call the *same*
projection. No behavior change.

**Why first:** Every later phase depends on a single canonical projection
function. Doing this in isolation makes the rewrite a thin consumer and
guarantees Decision #2 (byte-identical to `relative(value)`/`dirname(value)`)
by construction rather than by parallel re-implementation.

**Touchpoints:** `markdown/compose/expression/functions.rs` (remove
definition, keep callers), new
`markdown/compose/expression/path_projection.rs` (or another shared home
agreed during implementation), `markdown/compose/expression/mod.rs` (re-export).

- [x] Create `markdown/compose/expression/path_projection.rs` housing the
      relocated `make_relative(abs: &Path, base_dir: &Path) -> String` plus a
      thin wrapper that performs the **separator normalization** the spec
      requires (`/` on every OS — see Risks: "Cross-platform path text drift"
      and the existing `.replace('\\', "/")` calls scattered through
      `functions.rs`). The normalized wrapper is the function the rewrite pass
      will call; existing expression functions keep using the raw
      `make_relative` plus their own per-call `.replace` *or* migrate to the
      normalized wrapper — implementation's call, but the migration must not
      change any current test output.
- [x] Re-export `make_relative` (and the normalized wrapper) from
      `markdown/compose/expression/mod.rs` so `schemas/` can depend on it
      without a cyclic reach.
- [x] Update `functions.rs:1143` to delete the local definition and import
      from `path_projection`. Update `relative_fn` (`functions.rs:1161`) and
      the six other `make_relative(...).replace('\\', "/")` call sites
      (`functions.rs:1285, 1355, 1384, 1513, 1571`) to use the shared symbols.
      Behavior must be byte-identical — verified by leaving every existing
      function-level test green.
- [x] Add unit tests in `path_projection.rs` covering: (a) git-root-relative
      when inside a repo, (b) `base_dir`-relative outside a repo, (c) `~/`
      home-aliased, (d) absolute fallback, (e) Windows separator normalization
      to `/`. Use `tempfile::TempDir` plus an explicit git root fixture where
      needed.

**Validation checkpoint (Phase 1):**
- `just test` in `darkmatter` — all existing `functions.rs` tests pass
  unchanged (proves the lift was behavior-preserving).
- New `path_projection` tests pass.
- `cargo check -p darkmatter` clean.
- No public API surface change observable from outside the crate.

**Parallelizable with later phases:** No. Phases 2–4 consume this module.

---

## Phase 2 — Implement the Rewrite Engine (`schemas/rewrite.rs`)

**Goal:** A pure, schema-shape-driven function that walks a compiled JSON
Schema, finds every present value under an **eager** `format:
darkmatter-file` marker, and rewrites it to the repo-relative projection.
Idempotent, anchor-driven, opt-in by marker presence.

**Why second:** Phase 1 supplies the shared projection; this phase builds the
schema walker that consumes it without yet wiring any public API or compose
hook, so it can be unit-tested in isolation.

**Touchpoints:** new `markdown/schemas/rewrite.rs`,
`markdown/schemas/mod.rs` (declare module, expose `NormalizationOutcome`).

- [x] Define `NormalizationOutcome { value: Value, changed: bool }` mirroring
      `coerce::CoercionOutcome`'s shape. Pure: takes owned/coerced instance,
      returns rewritten instance; never mutates caller input.
- [x] Implement the top-level dispatcher
      `rewrite_eager_file_values(json_schema, instance, ctx) -> NormalizationOutcome`
      that delegates to the root-union branch when `json_schema["anyOf"]`
      exists (mirroring `coerce_frontmatter_with_pending`'s top-level branch at
      `coerce.rs:248`) and to the object/property walk otherwise.
- [x] Implement the **object walk**: iterate the schema's declared
      `properties`, descend into each present instance key, and for any
      property whose effective schema carries `format: darkmatter-file`,
      rewrite the value. Mirror the nullable-`anyOf` unwrap that
      `coerce::coerce_object` performs (`coerce.rs:340`,
      `unwrap_nullable_arm`) so a `file(eager) | null` property still triggers
      on its present, non-null arm.
- [x] Implement the **nested-shape descent** (Decision #8): recurse through
      inline-object `properties` and `items` arrays so an
      array-of-`file(eager)` element and an inline-object sub-property are
      each rewritten. The walk shape mirrors the schema-shape descent the
      description-resolution and coercion passes already perform.
- [x] Implement the **per-value rewrite**: for a present, non-null string
      value under an eager `darkmatter-file` marker, run
      `resolve_file_ref_with_fallback` (reusing the path-resolution work
      already shipped) and apply the shared `make_relative`. Leave the value
      verbatim when:
      - the value is `null` or absent (Decision #7),
      - the value is a remote URL (`is_remote_url`, Decision #7),
      - the resolved path is `None` (non-local reference accepted by
        validation — Decision #7),
      - rewriting produces a byte-identical string (idempotence fast path,
        Decision #6).
      Apply separator normalization (Phase 1's normalized wrapper) before
      storing.
- [x] Implement the **root-union branch** (Decision #8/#9): reuse
      `validate::wrap_arm_as_root_schema` (`validate.rs:706`) and the same
      accepted-arm selection rule as `coerce::coerce_root_union`
      (`coerce.rs:255`) so the rewrite never normalizes against a different
      arm than the one used for type write-back. Rewrite only the committed
      arm's eager-file properties.
- [x] Implement the **property-union branch** (Decision #9): a property-level
      `anyOf` may rewrite only when exactly one validating arm identifies the
      value as eager-file-typed. Zero or multiple matching arms leave the
      original value unchanged. Mirror the no-guessing rule already in
      `coerce::coerce_property_union`.
- [x] Enforce the **pending-value skip** (Decision #4): a value still holding
      `$(` or unresolved `{{` anywhere in that value is left verbatim. The
      rewrite API takes the same `composition_pending: &HashSet<String>` shape
      `coerce_frontmatter_with_pending` takes and skips any pending key at the
      top level (and, for nested shapes, skips a pending scalar in place).
- [x] Wire `NormalizationOutcome` and the dispatcher entry point into
      `markdown/schemas/mod.rs`. Do **not** add an `EffectiveSchema` method
      yet (that is Phase 3) — keep this phase's surface as a free function
      so the engine is unit-testable in isolation.

**Phase 2 unit tests (in `rewrite.rs`, all gated as L1):** every behavior the
spec's "Rewrite unit tests" section enumerates, each as a focused test:

- [x] Top-level `file(eager)` property with a resolvable relative reference is
      rewritten to the git-root-relative path.
- [x] Rewritten value equals `relative_fn`'s output for the same input
      (consistency with the existing projection — Decision #2).
- [x] `string`-typed property holding a path-shaped value is **not** rewritten.
- [x] Raw JSON Schema property with `format: darkmatter-file` **is** rewritten;
      any other `format` is not (Decision #9 trigger contract).
- [x] Bare (lazy) `file` property — `format: darkmatter-file-reference` — is
      **not** rewritten even when its value resolves to an existing file
      (Decision #1).
- [x] Idempotence: `rewrite(rewrite(x)) == rewrite(x)` (Decision #6).
- [x] Unresolvable `file(eager)` value is never rewritten — and the rewrite
      pass must not be invoked on a failed-validation instance anyway, so this
      test double-belts the contract (Decision #4).
- [x] URL/remote-resolving eager value left verbatim (Decision #7).
- [x] Absent and `null` optional `file(eager)` properties unchanged
      (Decision #7).
- [x] Nested `file(eager)` (inline-object sub-property), array-of-`file(eager)`,
      and root-union / property-union `file(eager)` arms each rewritten via the
      shape walk (Decision #8).
- [x] Ambiguous property union (two validating eager-file arms) is **not**
      rewritten (Decision #9).
- [x] Composition-pending eager `file` value is skipped; a concrete sibling
      eager `file` value in the same accepted schema is still rewritten
      (Decision #4).

**Validation checkpoint (Phase 2):**
- `just test` in `darkmatter` — every rewrite unit test above passes.
- `cargo check -p darkmatter` and `cargo clippy -p darkmatter` clean.
- No changes to any existing test elsewhere (this phase is purely additive).

**Parallelizable work inside Phase 2:** The object walk, root-union branch,
and property-union branch are largely independent once the per-value rewrite
and the `NormalizationOutcome` type land. An implementer may draft them in
parallel branches and merge, provided every unit test above is added against
the merged result.

---

## Phase 3 — Public Normalization API + Compose Write-Back

**Goal:** Expose the engine behind an explicit `EffectiveSchema` API and wire
it into the compose frontmatter write-back so both surfaces (library opt-in
and compose) rewrite consistently. Validation-only APIs stay read-only.

**Why third:** Depends on Phase 2's engine. Sequencing the API after the
engine lets the API be a thin, well-documented wrapper that composes
coercion → validation gate → rewrite exactly as the spec's Behavior diagram
prescribes.

**Touchpoints:** `markdown/schemas/mod.rs` (new `EffectiveSchema` method),
`markdown/compose/schema_validation.rs` (call site at the existing coercion
write-back, `schema_validation.rs:139-153`).

- [x] Add `EffectiveSchema::normalize_frontmatter` (exact name is the
      implementer's call; the spec suggests `normalize_frontmatter`). Signature
      shape:
      `fn normalize_frontmatter(&self, frontmatter: &Value, composition_pending: &HashSet<String>) -> NormalizationOutcome`.
      Internally: build a `ResolutionContext` from `self.base_dir` and
      `self.file_ref_fallback_dir` (the same anchors
      `validate_with_positions` threads into `FileRefAnchors` at
      `mod.rs:314`), then delegate to Phase 2's dispatcher. The method
      **does not** validate; it assumes the caller has already validated
      (compose does so on its accepted effective schema). Document this
      precondition in the rustdoc.
- [x] Preserve the **read-only validation contract** (Reader's note,
      Decision #3): do **not** call `normalize_frontmatter` from `validate`
      or `validate_with_positions`. Add a rustdoc note on both methods
      reaffirming "no input mutation," and add a regression test that hands a
      known-raw eager `file` value to `validate_with_positions` and asserts
      the caller's `serde_json::Value` is byte-identical afterward.
- [x] Wire compose's write-back at `schema_validation.rs`: after the existing
      coercion write-back loop and **after** the validation gate has accepted
      the composition-independent instance, call
      `effective.normalize_frontmatter(&coerced_or_current_instance,
      &composition_pending)` and write the rewritten top-level properties back
      into `markdown.frontmatter_mut()` using the same skip-pending-keys loop
      shape already present at `schema_validation.rs:142-152`. The rewrite
      must run only on the success path — if `composition_independent` would
      fail, do not rewrite (Decision #4: "a document with final validation
      problems is never rewritten").
- [x] Maintain compose's existing key exclusions: `$schema` and every key in
      `options.exclude_keys` must remain outside both the coercion and the
      rewrite write-back (spec: "Implementation constraints"). Verify by
      reusing the same instance-construction loop that already excludes them
      at `schema_validation.rs:113-128`.
- [x] Add library-level tests:
      - [x] Calling `normalize_frontmatter` on an `EffectiveSchema` carrying
            an eager `file` property rewrites the value; the original input
            `Value` is unchanged.
      - [x] Calling `normalize_frontmatter` with a `composition_pending` set
            containing the eager-file key leaves that key verbatim and
            rewrites a non-pending sibling.
      - [x] `validate_with_positions` does not mutate its `frontmatter`
            argument even when the schema has eager `file` properties
            (Decision #3 regression test).

**Validation checkpoint (Phase 3):**
- `just test` and `just test-l2` in `darkmatter` — all green.
- The new read-only contract regression test fails the moment anyone wires
  rewrite into `validate_with_positions`, so it is a real guardrail.
- `cargo check -p darkmatter` and `cargo clippy -p darkmatter` clean.
- Manually run `md compose` on a synthetic doc with a `file(eager)` property
  and confirm the rewritten value appears in the effective frontmatter dump.

**Parallelizable work inside Phase 3:** The `EffectiveSchema` method body and
the compose write-back wiring can be drafted in parallel by two implementers
since both depend only on the Phase 2 engine's public entry point. Merge
conflicts are confined to import lines.

---

## Phase 4 — Integration, Cross-Platform, and Documentation

**Goal:** End-to-end coverage that proves the spec's user-visible scenarios,
cross-platform stability, and the public docs all reflect the new contract.

**Why last:** Integration tests exercise the full compose → re-compose
round-trip that only exists once Phases 1–3 have landed. Documentation is
deliberately last so it describes the shipped behavior, not a draft.

**Touchpoints:** L2 integration tests under `darkmatter/cli/tests/` (or the
lib's L2 home, per the `rust-testing` skill's area convention), and
`darkmatter/docs/topics/schema-definition.md`,
`darkmatter/docs/inline/schema-validation.md`.

- [x] L2 integration: `md compose` of a doc whose `file(eager)`-typed property
      started raw shows the resolved repo-relative value in the effective
      frontmatter dump (spec Integration bullet 1).
- [x] L2 integration: compose write-back persists the rewritten value —
      re-running `md compose` on the persisted file is a fixpoint (no further
      diff). This is the spec's idempotence guarantee at the file level
      (Decision #6, Integration bullet 2).
- [x] L2 integration: end-to-end **review-feature-shaped fixture** — a
      `$schema` with `spec: file(eager, required)` and a derived
      `review_file: "{{dirname(spec)}}/review-{{iteration}}.md"` (no
      `{{ctx.area}}/` prefix). Assert `dirname(spec)` and the derived
      `review_file` resolve to the same prefix and that `review_file` exists
      on disk when the file is created. This is the motivating bug reproduced
      and then structurally fixed (spec: Motivating Bug, Integration bullet
      3). `review_file` itself stays bare/lazy `file` — it may not exist yet.
- [x] L2 integration: existing validation-only library calls keep returning
      reports without mutating the caller's `serde_json::Value` (spec
      Integration bullet 4). This complements the Phase 3 unit-level
      regression with a wider call-site sweep.
- [x] CWD-independence test (L2, serial under
      `#[serial_test::serial("darkmatter-file-cwd")]`): with the process CWD
      mutated to an unrelated directory, compose still produces the
      git-root-relative rewritten value. Mirrors the convention already used
      in `schemas/format.rs`.
- [x] Windows separator stability: assert the stored string uses `/`
      separators. This cannot run on macOS/Linux as a cross-OS assertion, so
      implement it as a unit-level check in `path_projection.rs` (Phase 1)
      that constructs a `PathBuf` with `\` separators and verifies the
      normalized wrapper emits `/`. Add an accompanying note in the docs that
      committed Markdown is portable across OSes (Risks: "Persisting resolved
      paths into committed documents").
- [x] Update `darkmatter/docs/inline/schema-validation.md`:
      - In the **Type Coercion** / "Where coercion runs" section
        (`schema-validation.md:230-235`), add a sibling subsection describing
        the **eager-`file` normalization contract**: what triggers it
        (`file(eager)` / `format: darkmatter-file`), what it does (rewrites
        the stored value to the repo-relative projection), what is **not**
        rewritten (`string`, lazy `file`, remote/URL, absent/null), and the
        idempotence guarantee. Cross-reference the new `normalize_frontmatter`
        library API.
      - Update the "Limitations (v1)" section (`schema-validation.md:268-273`)
        if any item there is now resolved or amended by this feature.
- [x] Update `darkmatter/docs/topics/schema-definition.md`:
      - Extend the `file` row in the Types table
        (`schema-definition.md:97`) and the Files section to state that an
        eager `file(eager)` value is **rewritten** to its repo-relative
        resolved path at validation success, while bare `file` is left
        verbatim. Keep the existing document-first / launch-area fallback
        description intact — the rewrite consumes that resolution, it does
        not change it.
- [x] Add a rustdoc-level example on `EffectiveSchema::normalize_frontmatter`
      (per the CLAUDE.md rustdoc convention: summary → `## Examples` →
      `## Returns`; no H1) showing a minimal "raw reference → repo-relative"
      rewrite. Keep the fixture small (under 20 lines) per the comment-quality
      guidance.

**Validation checkpoint (Phase 4):**
- `just test-l2` in `darkmatter` — every L2 scenario above passes.
- `just lint` in `darkmatter` — clippy and rustdoc checks clean.
- Manual review: docs render correctly (run `md` docs preview if available;
  otherwise re-read the edited sections in place) and the doc claims match
  the shipped behavior exactly.
- Re-run `just test` once more to confirm the full L1+L2 suite is green after
  every doc/example tweak.

**Parallelizable work inside Phase 4:** The L2 integration tests, the
CWD-independence test, and the two doc updates are independent of each other
once Phases 1–3 have landed. They may be authored in parallel and merged
individually.

---

## Cross-Cutting Verification (run at every phase boundary)

- [x] `cargo metadata --no-deps --format-version 1` still parses (workspace
      membership unchanged — no new crate, only files inside `darkmatter`).
- [x] `cargo check -p darkmatter` clean on macOS (this host). Note in the PR
      description any behavior that warrants a Windows/Linux sanity check
      (the separator normalization and the CWD-independence tests are the two
      cross-platform-sensitive spots).
- [x] No `cargo fmt` write-mode runs (per AGENTS.md). Match surrounding style
      by hand; `cargo fmt --check` is fine for diagnosis.
- [x] No `std::env::current_dir()` introduced as an implicit anchor anywhere
      in `rewrite.rs` or the new `EffectiveSchema` method (spec:
      "Implementation constraints"). The rewrite consumes the
      `ResolutionContext` carried by `EffectiveSchema`; legacy ambient-CWD
      behavior stays confined to the existing resolver fallback.
- [x] Every behavior-changing edit includes a pass over the affected `///`
      and `//!` docs; stale inline `//` comments are fixed or deleted in the
      same change (AGENTS.md: Comment Quality).

## Out of Scope (tracked separately, do not do here)

- The claudine prompt edits (`prompts/review-feature.md` typing `spec` as
  `file(eager, required)` and dropping `{{ctx.area}}/` prefixes) — spec's
  Downstream Work section explicitly defers these to a separate claudine
  change so the Darkmatter library behavior can be reviewed and merged
  independently.
- Any change to read-side resolution (`file_exists`, `frontmatter`, the eager
  `file` validator's anchor order). That landed in
  `@claudine/fixes/_completed/2026-06-27-path-resolution/plan.md` and this
  feature only consumes its `ResolutionContext`.
