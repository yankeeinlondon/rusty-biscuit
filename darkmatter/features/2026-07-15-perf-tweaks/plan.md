---
agent: claude
created: 2026-07-16
phase: 1
total_phases: 4
yolo: true
---

# Plan — Arc-Backed Schema Baseline Performance Tweaks

Execution plan for [`spec.md`](./spec.md).

## Shape of the Work

Four phases. Phase 1 is an unconditional test/benchmark hygiene change with no
production wall-time claim. Phase 2 builds a measurement harness and issues a
**ruling** — a real gate that can legitimately terminate the feature. Phase 3
runs **only if Phase 2 authorizes it**. Phase 4 records the outcome either way,
including a no-material-win closure.

```
Phase 1 ──▶ Phase 2 ──▶ [RULING] ──▶ Phase 3 (conditional) ──▶ Phase 4
                            └────────── no-win path ─────────────┘
```

Phase 1 and Phase 2's harness authoring are independent and **may run in
parallel** — they touch disjoint files (Phase 1 edits existing read-only
callsites; Phase 2 adds a new bench target). Phase 2's *baseline capture* must
run after Phase 1 lands, so the saved baseline reflects the tree Phase 3 will be
measured against.

## Verified Ground Truth

Established by reading the tree at plan time; treat as fact, not assumption.

**Callsite inventory** — `darkmatter_base_json_schema()` has 9 executable calls
plus 1 rustdoc example, and the spec's table matches the tree exactly:

| Location | Function | Action |
|---|---|---|
| `lib/tests/base_schema_end_to_end.rs:64` | `base_schema_file_parses_and_converts` | migrate |
| `lib/tests/base_schema_end_to_end.rs:293` | `schema_document_transcludes_same_file_as_library_source` | migrate |
| `lib/tests/base_schema_end_to_end.rs:340` | `base_schema_ctx_is_darkmatter_owned_generated_context` | migrate |
| `lib/src/markdown/schemas/mod.rs:2270` | `darkmatter_base_json_schema_validates_known_samples` | migrate |
| `lib/src/markdown/schemas/mod.rs:2326` | `darkmatter_base_json_schema_allows_unknown_keys` | migrate |
| `lib/benches/effective_schema_ownership.rs:132` | `bench_default_baseline_initialization` setup | migrate (hygiene) |
| `lib/src/markdown/schemas/mod.rs:2301` | `darkmatter_base_json_schema_is_cached` | **retain** |
| `lib/benches/effective_schema_ownership.rs:149` | `owned_accessor` bench case | **retain** |
| `lib/benches/effective_schema_ownership.rs:202` | `bench_effective_schema_ownership` baseline input | **retain** |
| `lib/src/markdown/schemas/mod.rs:166` | rustdoc example | **retain** |

**Cache-identity equality is provable, not hoped for.** `BASE_JSON_SCHEMA` is
built as `Arc::new(to_json_schema(darkmatter_base_schema_ref()))`
(`schemas/mod.rs:123-129`), and `with_darkmatter_baseline_schema()` stores
`darkmatter_base_schema()` — which is `darkmatter_base_schema_ref().clone()`.
Today's encoder computes `canonical_json_sorted(to_json_schema(&that_clone))`.
Therefore `canonical_json_sorted(darkmatter_base_json_schema_ref())` is
**value-identical** to the current built-in encoding. Phase 3's cached canonical
hash rests on this identity, and Task 3.6 asserts it rather than assuming it.

**Two encoders touch the baseline, and they disagree today — preserve that.**
`options.rs` has two independent encoders over the same destructured fields:

- `options_hash` (`enc`, lines ~1809-1820) encodes the canonical baseline JSON
  **and** the `baseline_is_darkmatter_default` bool.
- The compose-id encoder (`cenc`, lines ~1976-1985) encodes **only** the
  canonical baseline JSON — no discriminant bool.

Consequence: a custom baseline structurally equal to the built-in currently
hashes *differently* under `options_hash` and *identically* under the compose-id
encoder. That asymmetry is existing behavior. Phase 3 must reproduce both
encoders' field sets exactly; it is not licensed to "fix" the asymmetry.

**Public API is untouched.** The only external consumer of the baseline builders
is `claudine/lib/src/system_prompt/prepare.rs:140`, which calls the public
`with_baseline_schema(...)`. Phase 3 changes crate-private representation only,
so Claudine should not need a source edit — Task 3.1 verifies this with impact
analysis rather than assuming it.

## Open Decisions (resolve in-phase, do not silently pick)

The spec's four review questions map to concrete tasks. Recommendations are
given so execution is not blocked, but each must be recorded in the results doc.

1. **Q1 (Phase 1 value)** — resolved by executing Phase 1: the cleanup is usage
   guidance, and the plan forbids presenting it as a production win.
2. **Q2 (extend vs. separate bench target)** — **recommend a separate
   `compose_options_schema_ownership` target** (Task 2.1). Rationale: the
   existing target measures `DarkmatterSchemas`/`EffectiveSchema`, not
   `ComposeOptions`; and `just bench-baseline <name>` saves across `--benches`,
   so a separate target keeps this feature's saved baseline from being coupled to
   unrelated bench churn.
3. **Q3 (semantic cache identity)** — **recommend preserving today's exact
   behavior** (Task 3.6), which means *not* adopting a new equality rule. See the
   two-encoder note above.
4. **Q4 (enum vs. sentinel)** — **recommend the `BaselineSource` enum** (Task
   3.2). It deletes the representable-but-invalid state (`_is_darkmatter_default
   == true` while `baseline_schema == None`) that the current two-field model
   allows.

---

## Phase 1 — Read-Only Callsite Cleanup

Unconditional. Test and benchmark hygiene; **no production wall-time claim may
be made from this phase**.

- [ ] **1.1** Run `impact({target: "darkmatter_base_json_schema", direction: "upstream"})` and record the blast radius in the results doc. Expect HIGH risk with **no production execution flow** — all callers are tests and the ownership benchmark. If impact reports a production caller, stop and reconcile against the spec's inventory before editing.
- [ ] **1.2** Add `darkmatter_base_json_schema_ref` to the `darkmatter::markdown::schemas::{...}` import list in `lib/tests/base_schema_end_to_end.rs:3-9`. Keep `darkmatter_base_json_schema` imported only if a retained caller still needs it; if not, drop it to avoid an unused-import lint.
- [ ] **1.3** `lib/tests/base_schema_end_to_end.rs:64` (`base_schema_file_parses_and_converts`) — `let json = darkmatter_base_json_schema_ref();`. Downstream reads are index/borrow only, so no other line changes.
- [ ] **1.4** `lib/tests/base_schema_end_to_end.rs:293` (`schema_document_transcludes_same_file_as_library_source`) — change to `assert_eq!(&file_json, darkmatter_base_json_schema_ref());`. The `&` on the left is required: `Value: PartialEq` compares `&Value` to `&Value`.
- [ ] **1.5** `lib/tests/base_schema_end_to_end.rs:340` (`base_schema_ctx_is_darkmatter_owned_generated_context`) — `let json = darkmatter_base_json_schema_ref();`. Indexing (`json["properties"]["ctx"]`) works unchanged through the borrow.
- [ ] **1.6** `lib/src/markdown/schemas/mod.rs:2270` (`darkmatter_base_json_schema_validates_known_samples`) — `let json = super::darkmatter_base_json_schema_ref();` and build the validator with `.build(json)` (drop the now-double-reference `&json`).
- [ ] **1.7** `lib/src/markdown/schemas/mod.rs:2326` (`darkmatter_base_json_schema_allows_unknown_keys`) — same borrow + `.build(json)` change as 1.6.
- [ ] **1.8** Update the two migrated tests' `///` docs (mod.rs:2265, and the analogous test doc comments) so they name the behavior under test — the compiled baseline validating/accepting frontmatter — rather than the owned accessor. Per repo comment discipline, this doc pass ships in the same change as the behavior edit.
- [ ] **1.9** `lib/benches/effective_schema_ownership.rs:132` — change `bench_default_baseline_initialization` setup to `let baseline = darkmatter_base_json_schema_ref();`. Then `property_count(baseline)` and `merge_baseline(black_box(baseline), ...)` both typecheck (`merge_baseline` already takes `&Value`). This is untimed setup: **benchmark hygiene, not a reported result**.
- [ ] **1.10** Verify the three retained owned callers are untouched and still compile: `mod.rs:2301` (`darkmatter_base_json_schema_is_cached`), `benches:149` (`owned_accessor` case), `benches:202` (`bench_baseline(c, "darkmatter_baseline", darkmatter_base_json_schema())` — takes `Value` by value, so it *must* stay owned). Leave the `mod.rs:166` rustdoc example owned.

### Phase 1 validation checkpoint

- [ ] **1.11** `grep -rn "darkmatter_base_json_schema()" --include='*.rs' darkmatter` returns exactly the four retained sites (mod.rs:166 rustdoc, mod.rs:2301, benches:149, benches:202) and nothing else.
- [ ] **1.12** `darkmatter_base_json_schema_is_cached` passes — it proves both halves of the contract in one test (`ptr::eq` on two borrows; owned value mutated without disturbing the borrow).
- [ ] **1.13** `cargo bench -p darkmatter --bench effective_schema_ownership -- --list` still lists a named `owned_accessor` case.
- [ ] **1.14** From `darkmatter/`: `just test`, `just test-l2`, `just lint` all pass. (`just test-l2` skips cleanly without a tmux/WezTerm harness; that is a pass.)
- [ ] **1.15** Run `detect_changes({scope: "compare", base_ref: "main"})` and confirm the affected-symbol set is limited to the migrated test/bench functions.

---

## Phase 2 — Measure the Outer `ComposeOptions` Path

Builds the harness and issues the ruling. **This phase can legitimately end the
feature**, and doing so is a result, not a deferral.

Harness authoring (2.1-2.5) is independent of Phase 1 and **may run in
parallel** with it. Baseline capture (2.6) must not start until Phase 1 is
merged.

- [ ] **2.1** Create `lib/benches/compose_options_schema_ownership.rs` and register it in `lib/Cargo.toml` as a new `[[bench]]` with `harness = false` (append after the `phase10_residuals` entry, matching the file's existing formatting). Rationale for a separate target over extending `effective_schema_ownership` is recorded above (Q2) — write it into the bench's `//!` header too.
- [ ] **2.2** Build the fixtures. Construct options via `ComposeOptions::new_with_context(context)` with a fixed/demand-driven **empty** context — the spec is explicit that eager runtime-context capture would otherwise swamp the schema-ownership signal, and the 2026-07-12 review already showed context capture dominating compose. Reuse `effective_schema_ownership.rs`'s `synthetic_baseline(512)` shape for the custom-baseline control so it is size-comparable to the built-in.
- [ ] **2.3** Implement the six required Criterion cases, each `black_box`-guarded: (1) `configure_builtin` — `new_with_context(ctx).with_darkmatter_baseline_schema()`; (2) `clone_builtin_options` — clone configured built-in options, modeling child-pipeline propagation; (3) `configure_custom` + `clone_custom_options` — equivalently sized custom-baseline control; (4) `compose_baseline_only`; (5) `compose_baseline_plus_document_schema`; (6) `compose_no_baseline` control so unrelated options/compose cost stays visible.
- [ ] **2.4** Add an `options_hash` case over built-in-baseline options. Not in the spec's list, but the tree justifies it: `options_hash` calls `to_json_schema(baseline)` + `canonical_json_sorted(...)` on **every** call for the built-in path, which is plausibly larger than the `SimplifiedSchema` clone Phase 3 targets. Measuring it now is what makes the Phase 3 ruling honest.
- [ ] **2.5** Match the existing ownership benchmark's rigor: `group.sample_size(100)`, `Throughput::Elements(property_count)`, and the same host-preflight discipline. Add a `just bench-schema-options` recipe to `darkmatter/justfile` alongside `bench-schema` / `bench-compose`.
- [ ] **2.6** *(after Phase 1 merges)* Capture the saved baseline: `cargo bench -p darkmatter --bench compose_options_schema_ownership -- --save-baseline perf-tweaks-pre`. Record host preflight, 100 samples/function, dispersion (std dev / MAD, not just the mean), and benchmark source identity (commit SHA of the bench file).
- [ ] **2.7** **Predeclare the numeric thresholds before reading Phase 3 results** — write them into the results doc now, so the gate cannot be rationalized after the fact. Declare (a) a target: the minimum improvement on `configure_builtin` / `clone_builtin_options` that justifies the change; (b) a regression ceiling for the custom-baseline and no-baseline controls. Anchor both to the measured dispersion, not to a round number picked by feel.
- [ ] **2.8** Write `results.md` next to this plan with the Phase 2 numbers, the declared thresholds, and the host preflight.

### Phase 2 ruling — hard gate

- [ ] **2.9** Compare the measured built-in configure/clone cost against harness noise. **Proceed to Phase 3 only if** the cost is repeatable beyond noise **and** the proposed representation change can plausibly improve it without regressing controls.
- [ ] **2.10** If no repeatable opportunity exists: record the **no-material-win ruling** in `results.md` with the numbers that support it, skip Phase 3 entirely, and go to Phase 4. Per the spec, measurement-based closure is a completed outcome — do not reword it as a deferral, a TODO, or a follow-up ticket.

---

## Phase 3 — Conditional `ComposeOptions` Representation

**Runs only if Task 2.9 authorized it.** If Phase 2 ruled no-win, skip this
phase in full.

- [ ] **3.1** Run `impact()` on `ComposeOptions::with_baseline_schema`, `with_darkmatter_baseline_schema`, and `options_hash`, and record the blast radius. Confirm the only external consumer is `claudine/lib/src/system_prompt/prepare.rs:140` via the unchanged public `with_baseline_schema`. **Warn before proceeding if impact returns HIGH/CRITICAL.**
- [ ] **3.2** In `lib/src/markdown/compose/context/options.rs`, replace the `baseline_schema: Option<SimplifiedSchema>` + `baseline_is_darkmatter_default: bool` pair with one crate-private `BaselineSource { None, DarkmatterBuiltIn, Custom(SimplifiedSchema) }`. This deletes the invalid `(None, true)` state the two-field model permits. Update the `Default` impl (options.rs:474-475) and the `Self { .. }` destructure in the hash encoder (options.rs:1586-1587).
- [ ] **3.3** Rewrite the two public builders against the new representation, **signatures unchanged**: `with_darkmatter_baseline_schema()` sets `BaselineSource::DarkmatterBuiltIn` and **must no longer call `darkmatter_base_schema()`** — that call is the deep clone this phase exists to delete. `with_baseline_schema(schema)` sets `BaselineSource::Custom(schema)`.
- [ ] **3.4** Update `lib/src/markdown/compose/schema_validation.rs:84-108`: `has_baseline` becomes a non-`None` check on the enum; the builder match maps `DarkmatterBuiltIn` → `builder.with_darkmatter_baseline_json_schema()` and `Custom(s)` → `builder.with_baseline(s.clone())`. Custom-baseline conversion and validation semantics stay byte-identical.
- [ ] **3.5** Update the `Debug` impl (options.rs:395-402) to distinguish configured-vs-absent without dumping schema contents. Prefer distinguishing built-in from custom in the debug string; do not print either schema's body.
- [ ] **3.6** **Cache identity — the highest-risk task.** Add a `LazyLock<String>` holding `canonical_json_sorted(darkmatter_base_json_schema_ref())` and encode it for the `DarkmatterBuiltIn` arm in **both** encoders. Reproduce each encoder's existing field set exactly: `options_hash` (`enc`) emits the canonical JSON **and then** the discriminant bool (`true` for built-in, `false` otherwise); the compose-id encoder (`cenc`) emits the canonical JSON **only**. Do not add the discriminant to `cenc` and do not remove it from `enc` — the asymmetry is existing behavior. Never hash the enum discriminant alone: an edit to the checked-in `darkmatter.yaml` must still invalidate compose artifacts.
- [ ] **3.7** Add a guard test asserting `*BUILTIN_CANONICAL == canonical_json_sorted(&to_json_schema(&darkmatter_base_schema()).unwrap())`. This pins the Ground-Truth equality that makes 3.6 safe, and fails loudly if the built-in schema's derivation path ever diverges.
- [ ] **3.8** Update the crate-internal tests that read the removed fields directly: options.rs:2189 (`options.baseline_schema.is_some()`), options.rs:2204, and options.rs:2511 (compose-id sensitivity). Rewrite against the enum without weakening what they prove.
- [ ] **3.9** Add cache-key regression tests: two different custom baselines still hash differently; a custom baseline structurally equal to the built-in retains **exactly today's** behavior (differs under `options_hash`, matches under the compose-id encoder); and a mutated built-in JSON Schema changes the canonical hash.
- [ ] **3.10** Add a zero-clone assertion for the built-in path. Prefer the crate's existing test-only instrumentation-counter feature (`lib/Cargo.toml` `[features]`) over a timing-based assertion — a wall-clock test will flake in CI.

### Phase 3 validation checkpoint

- [ ] **3.11** Re-run the Phase 2 bench against the saved baseline: `cargo bench -p darkmatter --bench compose_options_schema_ownership -- --baseline perf-tweaks-pre`. The built-in cases must clear the Task 2.7 target; the custom-baseline and no-baseline controls must show no statistically credible regression. Append the comparison to `results.md`.
- [ ] **3.12** Behavior-compatibility suite — all six spec-listed behaviors unchanged: baseline-only validation; document schema overriding a baseline property; trigger-schema assembly with a baseline; recursive transclusion inheriting the baseline; run-local and persistent cache separation between semantically different baselines; `DARKMATTER_NO_BASELINE_SCHEMA` + `--baseline-schema PATH` CLI behavior (`cli/src/commands/compose.rs:628-660`).
- [ ] **3.13** From `darkmatter/`: `just test`, `just test-l2`, `just lint` pass. Note `just test`/`just lint` cover all three crates (`darkmatter`, `darkmatter-cli`, `dmls`), so DMLS's `overlay/schema.rs` baseline usage is gated here.
- [ ] **3.14** Run the Claudine gates identified by Task 3.1's impact analysis (`just test claudine` from the repo root, at minimum covering `system_prompt/prepare.rs`).
- [ ] **3.15** `just build` in every package area selected by impact analysis. **Do not** substitute `cargo build --workspace` or `cargo check --workspace` — the repo forbids a workspace-wide run as a routine gate.
- [ ] **3.16** Record Windows and Linux compile/test evidence before closing. Per repo memory, this is achievable from the macOS host: Linux via Docker (real kernel) and Windows via the installed `x86_64-pc-windows-gnu` target. Do not close Phase 3 by declaring cross-OS evidence impossible.
- [ ] **3.17** Run `detect_changes({scope: "compare", base_ref: "main"})` and confirm the affected symbols and execution flows match the intended scope.

---

## Phase 4 — Documentation and Closure

Runs on **both** paths — Phase 3 completed, or Phase 2's no-win ruling.

- [ ] **4.1** Confirm `schemas/mod.rs` rustdoc still states plainly that `darkmatter_base_json_schema()` returns an independent owned value and `darkmatter_base_json_schema_ref()` is the read-only fast path. Both docblocks (lines ~152-190) already say this; correct only if Phase 1/3 drifted them.
- [ ] **4.2** Document that `with_darkmatter_baseline_schema()` uses the process-cached built-in schema — **without** promising the internal enum or Arc layout. The representation is a non-goal to expose.
- [ ] **4.3** Finalize `results.md`: Phase 2 numbers, predeclared thresholds, the ruling, and (if it ran) the Phase 3 comparison. If Phase 3 did not run, the no-material-win ruling is the terminal entry.
- [ ] **4.4** Per repo drift-maintenance rules, update `.claude/skills/darkmatter/SKILL.md` **only if** Phase 3 changed something a skill reader must know. Phase 1 alone changes no architecture and needs no skill edit. Public API is unchanged in every path, so the likely answer is no edit.
- [ ] **4.5** Set the spec's frontmatter `status` to reflect the outcome, and move `darkmatter/features/2026-07-15-perf-tweaks/` to `features/_completed/`.
- [ ] **4.6** Final gate: `just test`, `just test-l2`, `just lint` green from `darkmatter/`. Commit only if the prompt explicitly asks — per repo convention, committing is a separate operation.

---

## Risks

- **Phase 3's real payload is cache identity, not the clone.** Task 3.6 touches
  two encoders that already disagree with each other. Getting it subtly wrong
  silently corrupts compose cache identity — either colliding two different
  baselines, or failing to invalidate when `darkmatter.yaml` changes. Tasks 3.7
  and 3.9 exist specifically to make that failure loud.
- **The gate is real.** Phase 2 may well conclude that the remaining built-in
  clone is noise and the feature ends there. Task 2.7 predeclares thresholds
  before results are visible precisely so that conclusion cannot be
  rationalized away.
- **Phase 1 is not a performance win.** It is usage guidance. Tasks 1.9 and the
  phase header both say so; reporting it as a production improvement would be a
  false claim.
- **`options_hash` may dwarf the clone.** Task 2.4 measures it. If confirmed, the
  cached canonical string in 3.6 is the larger win and the Phase 3 narrative
  should lead with it.
