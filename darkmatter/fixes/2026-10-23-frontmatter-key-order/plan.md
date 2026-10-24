---
total_phases: 6
created: 2026-10-23
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
start_phase: 2
---

# Plan: Frontmatter key order is the contract, the hash ignores it

## Summary and Definition of Done

Merman turned on `serde_json/preserve_order` for every Darkmatter build by
accident. Three things must be true afterward:

1. **Order is explicit.** Darkmatter's own manifest requests `preserve_order`,
   and author key order survives parsing, serialization, `md frontmatter`
   edits, and every merge (base first, new keys appended).
2. **The non-strict frontmatter hash ignores key order at every depth**, using
   a canonical-JSON renderer that lives in `biscuit-hash` (`json` feature) and
   leaves recorded hash identities untouched.
3. **Other structured hashes are audited** (TOC normalized hash, schema
   validator cache key, DMLS schema cache key) and each is canonicalized or
   documented as order-sensitive.

Done means every acceptance item (spec 1-8) is observably met: the benchmark
manifest test passes unchanged, the pair-of-documents hash matrix passes,
biscuit-hash feature combinations build and test, `cargo tree` shows Darkmatter
requesting `preserve_order`, `just test` and `just lint` are green in both
package areas, and the implementation log records the audit, justified output
updates, the strict-baseline correction, and departures. The terminal state is
"implementation complete, ready for review" (never `just complete`).

The Input Robustness Matrix does not apply: the work adds a renderer/hasher and
changes merge ordering. It adds no parser or config reader (frontmatter parsing
itself is unchanged).

Scope notes: no performance spike (spec says none); other dependents of
`preserve_order` (claudine, messenger, ...) and `float_roundtrip` are out of
scope.

---

## Phase 1: Rulings, orientation, and caller trace

Read-only orientation plus the one risk-reducing spike; no production code.

### Necessary Rules

The spec marks `needs_rulings: false`. Every implementation choice the spec
leaves open is settled below from a code trace done at planning time
(2026-10-23), so none is deferred to the implementer or to author review.

- **N1. Renderer is infallible, and it already exists.** Darkmatter's
  `compose/cache/hashing.rs` has `canonical_json_sorted`, a borrowed
  `Serialize` wrapper (`CanonicalJson`) that sorts object keys by `str` order
  and emits compact `serde_json`. It already backs the compose-cache
  `effective_state_hash`, `context_hash`, and operation keys
  (`cache/operation.rs`). **Move that implementation into biscuit-hash** as the
  shared renderer (R-K4) and delete the Darkmatter copy. Its bytes do not
  change, so compose cache keys stay stable. Its `unwrap()`/`unwrap_or_default()`
  become one `expect` with an invariant message. Serializing a `Value` into a
  `Vec<u8>` cannot fail, because keys are always strings and the writer is
  in-memory. This is the spec's "infallible by construction" option, not a
  `Result`. Its existing unit tests move with it.
- **N2. Public names (implementer's choice, fixed here for consistency):**
  `biscuit_hash::canonical_json(&Value) -> String`,
  `xx_hash_json(&Value) -> u64`, `blake3_hash_json(&Value) -> <existing blake3
  return type>`. Hash conveniences are one-line calls over a shared renderer.
  The renderer may write into a caller-provided `String`/`Write` internally to
  avoid intermediate allocation.
- **N3. Sorted traversal without cloning.** Collect `Vec<&String>` keys per
  object, sort, emit. With `preserve_order` the input map is never reordered.
- **N4. TOC normalized hash** canonicalizes the whole frontmatter object
  (`canonical_json(Value::Object(map))`), keeps the empty-map sentinel (no
  assignment when frontmatter is empty) and the separate raw hash. Its stored
  value changes for any document whose nested order was previously not
  sorted-by-chance; this is intentional and noted in the log.
- **N5. `schema_content_hash`** switches to `xx_hash_json` (canonical bytes).
  Array order stays significant.
- **N6. DMLS `schema_cache_key`: no change.** `DmlsConfig::schema` is
  `SchemaConfig { strict: bool, extensions: BTreeMap<String,
  SchemaExtensionConfig { path, globs: Vec<String> }> }`. It has no free-form
  JSON object, and `BTreeMap` serializes sorted, so the key is already
  order-insensitive by construction. Record that in the audit table and add a
  one-line comment at `schema_cache_key` stating it. Document text stays
  byte-sensitive.
- **N7. Composition merge: base fixes order, a parameter picks the value.**
  `deep_merge(base, overlay)` and `deep_merge_override` already keep base
  positions and append overlay-only keys. The defect is the argument reversal
  used for document precedence: `EffectiveStateBuilder::build` with
  `PreferDocument` and `EffectiveState::new` both call
  `deep_merge(external, frontmatter)`, which makes external state the order
  base. Fix it by giving the recursive merge an explicit value-precedence
  parameter (which side wins a non-object conflict). The receiving-document
  argument is always `base` and always fixes key positions. Do not add a
  post-hoc reorder pass.
  - **Null rule, unchanged:** in `deep_merge`, a `null` on the winning side means
    "not set" and yields to the other side's value; `deep_merge_override` keeps
    `null` as a literal. Re-express the existing behavior exactly; do not change it.
  - `PreferExternal`, `ErrorOnConflict`, `merge_replace_maps` (doc replace is
    base, parent wins values), and `apply_set_overrides` (child frontmatter is
    base) already have the receiving document as `base`. They only switch to the
    new signature.
- **N8. Ordered maps through `EffectiveState`; Claudine updated in this fix.**
  Order is observable. `EffectiveState::data` is a `HashMap`, so bare `{{ fm }}`
  (`get_fm`, `FmPath::Whole`) renders top-level keys in random order. Transclusion
  inheritance (`markdown_child_options`) also re-collects that `HashMap` into the
  child's external state, so the parent's order is lost before the child's merge.
  - Change `EffectiveState`'s `data` field, `data()`, and `EffectiveState::new`
    to `serde_json::Map<String, Value>`.
  - Change `EffectiveStateBuilder::with_frontmatter` to the same type, and
    `markdown_child_options`'s `parent_data` parameter to `&Map`.
  - `presentation_values` stays a `HashMap`, because it is used only for lookup.
  - The public API change is contained. The callers outside Darkmatter are six
    Claudine sites (`cli/.../sequence/task_run.rs`,
    `cli/.../loop_control/tests/mod.rs`, `lib/.../interpolation_conformance.rs`,
    `lib/.../preflight.rs`, and `lib/.../lifecycle/executor.rs`, which has two).
    Each already holds an ordered `Map` and converts it to a `HashMap`, so the
    update deletes that conversion.
  - Update those sites in this fix. Claudine *behavior* differences from
    insertion order stay out of scope, per the spec. Compiling Claudine is in
    scope because this fix changes its API.
  - Darkmatter-internal `with_frontmatter` callers (`pipeline/mod.rs`
    `document_state_builder`, `subtree.rs`, `replacement.rs`, `conditions.rs`,
    and the CLI test `cli/tests/l1/compose_schema.rs`) switch to `Map` the same
    way.
- **N9. Strict hashes of nested maps may change** once author order is honored.
  Documented, not migrated; no timestamp bumps.
- **N10. Test-input CI.** Any test reading repo files uses the forms from the
  `rust-testing` skill (the benchmark manifest is read by an existing test).

### Spike (runs once, here only)

- [x] **S1. Confirm the planning trace (read-only).** N6–N8 are already decided.
  This spike only confirms that the call-site lists in N1 and N8 are still
  complete on the current tree; the decisions do not change. Grep for
  `with_frontmatter(`, `EffectiveState::new(`, `.data()` on an
  `EffectiveState`, and `canonical_json_sorted` across Darkmatter and Claudine.
  Grep Darkmatter and DMLS for other `serde_json::to_string`/`to_vec` calls that
  feed a hash. Record the lists in the implementation log. A newly found site
  takes the same treatment as its siblings; it does not reopen the decision.
  - Use `gitnexus` `context`/`impact` or text search; treat `risk: UNKNOWN` as
    "confirm with grep".

### Tasks

- [x] **Baseline run.** On the current tree run `just test` in `darkmatter/` and
  `biscuit-hash/`; save the failing-test list (the draft says 45) so later
  phases can show each failure resolved or deliberately updated.
- [x] **Read skills.** Load the `darkmatter`, `biscuit-hash`, `rust-testing`,
  and `os` skills; load `rust-devops` only if CI scope is touched (not expected).
- [x] **Create the implementation log** at
  `darkmatter/fixes/2026-10-23-frontmatter-key-order/implementation-log.md`
  with sections: Hash audit, Output updates, Strict-baseline correction,
  Departures. Seed it with the S1 findings.

**Checkpoint 1:** call-site lists confirmed in the log; baseline failure list
saved.

---

## Phase 2: biscuit-hash `json` feature

Self-contained library work; nothing else depends on it except Phase 3.

### Wave 1 (parallel)

- [ ] **Feature wiring.** (`biscuit-hash/lib/Cargo.toml`)
  - add `json = ["dep:serde_json"]` and optional `serde_json = "1"`
  - it must not enable `preserve_order`, `xx_hash`, or `blake3`
- [ ] **Docs skeleton.** Create `biscuit-hash/docs/` with a `json.md` topic
  (rendering, hash algorithms, feature combinations with a small table and one
  example per row). Audience: a developer new to the repo. Fill after the API
  lands.

### Wave 2 (after Wave 1 wiring)

- [ ] **Canonical renderer.** New `biscuit-hash/lib/src/json.rs`, gated on
  `feature = "json"`, re-exported from `lib.rs`.
  - port Darkmatter's `canonical_json_sorted`/`CanonicalJson` (N1) byte-for-byte;
    move its unit tests here
  - borrowed traversal, sorted key refs, recursive through objects in arrays
  - arrays/scalars preserved; no input mutation; infallible (N1)
  - must compile with `--no-default-features --features json`
- [ ] **Hash conveniences.** `xx_hash_json` under `json`+`xx_hash`
  (existing seed and return type), `blake3_hash_json` under `json`+`blake3`;
  both hash the bytes of the one renderer, no second recursive algorithm.

### Wave 3 (after Wave 2)

- [ ] **Unit tests** (nextest, in the crate): recursive object-order
  independence; array-order sensitivity; empty `{}`/`[]`; scalar roots; Unicode
  keys; escaped strings; exact canonical bytes for one representative fixture;
  equality with compact `serde_json::to_string` for already-sorted input;
  input iteration order unchanged (build the input with explicit insertion
  order and `preserve_order` enabled via a dev-dependency feature); hash
  conveniences equal `xx_hash_bytes`/blake3 helper of the canonical bytes.
  - the crate's own tests need `preserve_order` only as a dev-dependency
    feature, never as a normal feature
- [ ] **Docs.** biscuit-hash README (overview + link only), crate docs
  (`//!`), `docs/json.md`, and the `biscuit-hash` skill.

**Checkpoint 2 (validation):**

- [ ] `cargo check -p biscuit-hash --no-default-features --features json`
- [ ] same with `json,xx_hash`, `json,blake3`, and default features
- [ ] `just test` and `just lint` in `biscuit-hash/` (all-features recipe
  covers `json`; confirm lint covers the gated code)

---

## Phase 3: Darkmatter manifest, hash, and audited hashes

Requires Phase 2.

### Wave 1 (sequential, small)

- [ ] **Manifest.** `darkmatter/lib/Cargo.toml`: `serde_json` gains
  `features = ["preserve_order"]` with a contract comment; the `biscuit-hash`
  dependency gains `json` alongside `blake3`.
  - run `cargo tree -e features -i serde_json -p darkmatter` and save the
    output for the log (acceptance 7)

### Wave 2 (parallel; disjoint files)

- [ ] **`hash_frontmatter_map`** (`lib/src/markdown/hash/mod.rs`). Replace only
  the value rendering with the shared renderer. Preserve: top-level key sort,
  `key:` + canonical value, the literal two-character `\n` join (not a newline),
  `xx_hash` of the bytes, empty -> `xx_hash("")`, ignore-property filtering,
  strict path (YAML of the unsorted map), body and keys-only rules.
  - update the stale doc comment on `hash_frontmatter_keys` only if it implies
    sorted nested maps
- [ ] **TOC normalized hash** (`toc/mod.rs`, ~line 442): per N4.
- [ ] **Schema content hash** (`schemas/validate.rs:1446`): per N5.
- [ ] **DMLS cache key** (`dmls/src/overlay/mod.rs:1047`): per N6, add only the
  one-line comment; no code change.
- [ ] **Compose cache renderer** (`compose/cache/hashing.rs`,
  `cache/operation.rs`): per N1, replace `canonical_json_sorted` with
  `biscuit_hash::canonical_json` and delete the local wrapper. Existing compose
  cache tests must pass unchanged, which shows the keys are byte-identical.
- [ ] **Residual audit.** Search remaining Darkmatter and DMLS hash inputs for
  serialized JSON objects; canonicalize or justify each; record every audited
  hash (purpose, order matters?, action) in the log.

### Wave 3 (parallel tests, after Wave 2)

- [ ] **Fixed historical-format regression.** A small fixed expected-hash test
  for the entry format (`key:value` joined with literal `\n`), so a future
  delimiter or whole-map change fails.
- [ ] **Hash matrix tests** (parsed YAML fixtures, not value equality). Three
  pairs: top-level-only reorder, two-level nested reorder, array-element
  reorder. Each: equal non-strict frontmatter hashes, different strict hashes.
  Cover the shared helper and the frontmatter component of each option-based
  hash kind; nested-only pairs leave strict keys-only hashes unchanged; body-only
  output unaffected.
- [ ] **TOC/schema tests.** Normalized TOC hash ignores top-level, nested, and
  array-contained object key reordering but changes on array-element reorder;
  reordered schema object shares a validator cache identity, reordered array
  does not.

**Checkpoint 3 (validation):**

- [ ] `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`
  passes with the manifest **unchanged** (`replace_heavy` back to
  `9e3319d168b8b47a`)
- [ ] run the Darkmatter suite and compare against the Phase 1 baseline list

---

## Phase 4: Merge order

Requires Phase 3 manifest change; independent of hashing code, so Waves 1 and 2
may run concurrently with Phase 3 Wave 3 on separate files.

### Wave 1 (parallel)

- [ ] **Shallow merges** (`lib/src/markdown/frontmatter.rs`). Verify
  `merge_with` under every `MergeStrategy` and `set_defaults` keep base
  positions, append incoming-only keys in incoming order, never move an
  overwritten key, and replace nested objects wholesale (incoming internal
  order retained). `ErrorOnConflict` still errors; no rollback promise. Fix
  only what tests show is wrong; document as a stated contract in comments.
- [ ] **Recursive composition order** (`compose/context/effective_state.rs`).
  Per N7: receiving document is the base for document + external state; the
  child is the receiving document in transclusion; `apply_set_overrides` and
  `merge_replace_maps` keep base-first order. Remove the argument-reversal as
  an ordering policy while keeping value precedence identical.
  - preserve scalar/array/null, runtime `ctx`, and special `replace` rules;
    arrays remain leaves
- [ ] **Ordered maps through `EffectiveState`** per N8: change the field,
  accessor, constructor, builder, and `markdown_child_options` to
  `serde_json::Map`. Update the Darkmatter-internal callers and the six Claudine
  sites, deleting their `HashMap` conversions. Then run
  `cargo check --tests -p claudine -p claudine-cli`; it must be clean.

### Wave 2 (after Wave 1)

- [ ] **Merge tests.** Enumerate keys to assert order. Shallow: every strategy
  and defaults, overwrite-without-movement (`{z,a}` + `{a,b}` -> `z,a,b` whether
  `a` keeps or takes the incoming value), incoming key order, nested object
  replacement. Recursive: document/external merge with both value-preference
  choices, transclusion inheritance, `set` / `set.NAME` overrides; plus
  regression assertions that value/null/array/`ctx`/`replace` precedence is
  unchanged.
- [ ] **Metadata agreement.** Test that `Frontmatter::mapping_key_order` agrees
  with parsed value iteration order; update comments implying nested JSON maps
  must be sorted. Do not remove or redefine `mapping_key_order`.
- [ ] **Unordered-map audit.** Check paths that expose/persist authored
  mappings for conversions through `HashMap`; carry ordered maps instead.
  Record findings in the log.

**Checkpoint 4:** merge tests green; no new `HashMap` on an order-observable
path (grep `into_iter().collect()` on frontmatter objects).

---

## Phase 5: Failing-output review and CLI process tests

### Wave 1 (parallel by test file)

- [ ] **Review each candidate failure individually** (do not bulk-accept
  snapshots):
  - `frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`
  - `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`
  - `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`
  - `schemas_convert_snapshots::*`
  - `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`
  - for each: confirm new output follows author/deterministic construction
    order; randomized order means a code fix, not a snapshot update; for the
    legacy-command byte comparison confirm both routes agree before touching
    expected text; record justification in the log
  - order fixtures built from parsed YAML or explicit insertion
- [ ] **Remaining failures** from the Phase 1 baseline list: triage each as
  resolved by earlier phases, a justified update, or a code fix.

### Wave 2 (parallel, ordinary process tests via the test toolkit; no terminal or browser focus)

- [ ] **`md frontmatter` round trip.** read, edit, save, reread, repeat: nested
  and top-level order survives; replacing an existing key keeps its position;
  new keys append.
- [ ] **`md hash --save` / `--diff`.** Save a non-strict baseline, reorder
  nested keys only, then: no semantic change, baseline file not rewritten, no
  timestamp bump, empty diff.
- [ ] **Strict-baseline correction.** A test (or log entry if a test is not
  meaningful) showing strict hashes of nested maps follow author order; no
  automatic migration.

**Checkpoint 5:** `just test` and `just test-l2` (if the area has L2 for these
commands) green in `darkmatter/`, which includes the CLI and DMLS.

---

## Phase 6: Documentation, final gates, handoff

### Wave 1 (parallel docs)

- [ ] `darkmatter/docs/cli/hash.md`: recursive normalization; strict-hash
  compatibility and the need to refresh an affected strict baseline via the
  normal save path.
- [ ] New `darkmatter/docs/topics/frontmatter.md` (parsing, order, shallow merge
  behavior; compact example per rule; a Mermaid diagram for the merge flow).
- [ ] Update `darkmatter/docs/topics/frontmatter-recursion.md`:
  receiving-document-first recursive order with compact examples.
- [ ] Link from the relevant README overviews (overview only, no command
  reference).
- [ ] `docs/dependencies.md` (Darkmatter and biscuit-hash areas; repo-level if
  it lists features), `darkmatter` and `biscuit-hash` skills.
- [ ] Comment pass on every touched symbol (`///`, `//!`, inline): fix or delete
  drift; docs must not link to or name this fix.

### Wave 2 (sequential gates)

- [ ] `just lint` and `just test` in `darkmatter/` and `biscuit-hash/`
- [ ] `cargo check --tests -p claudine -p claudine-cli` (N8 API change)
- [ ] biscuit-hash feature matrix from Checkpoint 2 re-run once as the final
  gate (not a repeated spike)
- [ ] `cargo tree -e features -i serde_json -p darkmatter` shows `preserve_order`
  requested by darkmatter itself
- [ ] `git diff --stat` shows no `cargo fmt`-only churn

### Wave 3

- [ ] Finish the implementation log: hash audit table, justified output
  updates, strict-baseline correction, the N8 API change and the Claudine
  sites it touched, and any departures from the spec or from N1–N10.
- [ ] Set spec status to `implemented` per repo convention; **do not** move the
  fix to `_completed` or run `just complete`.

**Final checkpoint:** every acceptance item 1-8 mapped to a passing check or a
log entry; state "implementation complete, ready for review".
