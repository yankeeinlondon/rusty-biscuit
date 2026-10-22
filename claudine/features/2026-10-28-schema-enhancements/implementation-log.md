---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-28-schema-enhancements/spec.md"
plan: "claudine/features/2026-10-28-schema-enhancements/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/mod.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/timing.shipped.yaml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/main.rs
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.toml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.lock
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-28-schema-enhancements/inventory/criteria-map.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/prerequisites.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/function-registry.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/ownership.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/reader-matrix.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/rulings.md
    - claudine/features/2026-10-28-schema-enhancements/behavior-ledger.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/findings.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/default.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/arbitrary-precision.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/preserve-order.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
human_review: true
human_review_items:
    - |-
      **Can a category name appear twice, or be empty? (needed before Phase 2's grammar work)**

      Phase 2 adds a `categories:` list to schema files, which groups functions for documentation. The spec does not say what happens when two entries have the same name, or when a name is blank. The grammar code written in Phase 2 has to pick one answer, and changing it later means rewriting that code and its tests.

      - **Option A — reject both (recommended).** A blank name, or a name that repeats an earlier one, makes the schema file fail to load with an error pointing at the entry. Names compare exactly ("Math" and "math" are different). List order is kept. *Pro:* matches how the old catalog already rejects blank categories and how the spec rejects duplicate parameter names; nothing is silently dropped. *Con:* a small authoring mistake stops the file loading.
      - **Option B — merge duplicates, first one wins.** *Pro:* forgiving. *Con:* no sensible rule when the two copies have different descriptions; hides mistakes.
      - **Option C — silently drop blank names.** *Con:* this is the kind of silent filtering the project's input rules forbid.

      I recommend **A**. Say if you would rather compare names case-insensitively, so "Math" and "math" count as the same name.
    - |-
      **Four edge cases in how numbers are read (needed before Phase 2's number work)**

      Phase 1's investigation picked how numbers will be checked for exactness: each file reader keeps today's values and also remembers how each number was written, so `9007199254740993` (too large to store exactly) can be rejected instead of silently rounded. A workspace-wide alternative (`serde_json`'s `arbitrary_precision` switch) was rejected because it would change number behavior in 67 crates and still not help YAML. Details: `spikes/numeric-ingress/findings.md`. Four smaller cases need your answer:

      1. **Huge whole numbers in YAML, such as `99999999999999999999`.** Today they make the *whole* frontmatter fail to parse. *Recommended:* keep it an error, but report it on that one field with its location. *Alternative:* report it as "cannot be represented exactly", like the other big numbers. That is more uniform, but it needs a custom YAML read path.
      2. **Expression literals such as `1e3` or `.5`.** These are not valid in expressions today. *Recommended:* keep it that way; the spec does not add that syntax. *Alternative:* add it (new syntax, more scope).
      3. **JSON `1e400` (too large to store).** The JSON reader already fails with "number out of range". *Recommended:* accept that load error as the spec's "not finite" outcome. *Alternative:* re-word the error to say "not finite" (cosmetic, extra work).
      4. **Text declared as `json` that contains `1e400`.** It is rejected today as malformed JSON. *Recommended:* keep that.
    - |-
      **Rulings needed later (not blocking Phase 2, but best decided now while you are reviewing)**

      Each has a recommendation in `inventory/rulings.md` or `inventory/function-registry.md`. Replying "approve the recommendations" settles them all.

      - **R1 — some functions cannot have a "converted input" test (blocks acceptance criterion 10, Phase 4).** 34 functions take any value unchanged (for example `is_number`, `file_exists`, `first`) or take no arguments, so a test showing the input being converted is impossible. *Recommended:* mark those cells "impossible", test instead that the input arrives unchanged, and cover conversion with test-only functions. *Alternatives:* weaken those functions' signatures so they convert (changes behavior; not recommended), or drop them from the criterion (forbidden by the plan).
      - **`trim` does not exist.** The spec's criterion 43 expects `trim(null)` to return `null`, but there is no `trim` function. Add `trim` (recommended, a small string function) or change the criterion.
      - **121 examples, not 117.** Three functions added four examples after the spec was written. Carry all 121 (recommended).
      - **R3 — duplicate keys in JSON.** Reject them in raw `.json` schema files and in `json`-typed text (recommended). Keep the editor-settings channel as it is, because an existing test depends on it.
      - **R4 — editor settings channel.** The spec says `workspace/configuration`, but the language server deliberately uses `didChangeConfiguration`. Read the spec as meaning the existing channel (recommended).
      - **R5 — some "should fail" cells in the plan contradict documented behavior.** For example `types.x: null` is documented as the `null` type. Keep the documented behavior for ordinary schema files, and apply the stricter rules only in the new function-catalog loader (recommended).
      - **R7 — two plan cells disagree with the JSON Schema standard (`prefixItems: []`, `enum: []`).** Follow the standard's validator (recommended).
      - **Ambiguous null behavior for about 20 functions** (for example `is_empty(null)`, `starts_with(null, …)`) and **`has_command(true)`'s draft signature**, which contradicts criterion 44. These are listed in `behavior-ledger.md`; the ledger review is a pre-merge requirement anyway.
message_to_agent: |-
  Phase 1 produced only inventory, a numeric spike, and matrix tests; no production code changed. Read these before starting Phase 2 (all under claudine/features/2026-10-28-schema-enhancements/):
  - inventory/rulings.md (R1-R8) and spikes/numeric-ingress/findings.md "Decisions for the author": check human_review_items / the author's reply for the decisions. Do not implement categories duplicate/empty handling or the four numeric edge cells until they are ruled; the recommendations are written there.
  - inventory/ownership.md: Grammar descriptors owns simplified/grammar.rs and convert.rs; Numeric foundations supplies the admission function that Grammar calls at grammar.rs:279 and :1948 (both parse numbers with bare parse::<f64>()). schemas/resolve.rs is unowned in the plan (proposed: Schema adapter, Wave 4); schemas/mod.rs proposed for the integrator. `expr/binding.rs` already exists, so pick another name for the call binder.
  - Numeric seam (selected): a JSON-Pointer -> written-number-text sidecar beside today's serde_json::Value, recorded only for numbers stored as f64 (modeled on MappingOrders). YAML: second guided serde_yaml_ng pass; JSON: RawValue pass; JSON5: json-five round-trip model; expression literals: lexer.rs:921 token text; schema metadata: grammar.rs:1941 Arg.lex. Do NOT enable serde_json/arbitrary_precision. Darkmatter must declare serde_json features raw_value and float_roundtrip itself (today they only arrive via feature unification).
  - Defects owned by Phase 2: standalone.rs:127-130 swallows YAML parse errors into Ok(None) (DMLS drops them at named_types.rs:344 and overlay/named_types.rs:231); coerce.rs:597 rounds large integers silently and its comment is wrong; YAML .inf/.nan and JSON5 1e400 become null, 1e-400 becomes 0, enum members saturate through i64, min(inf) is accepted.
  - Matrix tests live in darkmatter/lib/tests/l1/schema_enhancements_matrix/. When your task makes a blocked cell decidable (see inventory/reader-matrix.md "Cell coverage"), add that cell there; when you fix the AmbiguousReferenced mislabel, tighten the duplicate/trailing cells to assert the new error class.
  - Spec criterion 43 uses `trim`, which is not registered; the draft catalog lacks find_files, code_link, try_frontmatter and wrongly lists is_file_type (Phase 4 concerns).
---

# Implementation Log for 2026-10-28-schema-enhancements (6 phases)

## Phase 1

Phase 1 is inventory and a one-time numeric-ingress spike. It changes no
production code.

### What was produced

| Plan task | Output |
|---|---|
| Contract inventory: criterion → task/test map | `inventory/criteria-map.md`: all 61 criteria (1–59, 33a, 33b) mapped to a plan task, a proposed L1 test identity, and the public result the test observes |
| Contract inventory: prerequisites | `inventory/prerequisites.md`: checked in code, not by spec status. `core-schema-types`, `union-type-constraints`, `fm-namespace` and `type-narrowing` are all **supplied**; the inline `(A \| B)[]` spelling and the declared-type projection are this feature's own work (Phase 2 Grammar, Phase 5 Variable projection) |
| Contract inventory: registry and catalog | `inventory/function-registry.md`, enumerated from the registry source: 113 canonical functions (62 pure, 49 context, 2 lazy: `and`/`or`), 75 aliases; the current catalog matches the registry exactly |
| Contract inventory: behavior ledger | `behavior-ledger.md`: one draft row per canonical function with null family, fixture applicability and preserved/changed status. **Human review outstanding (pre-merge requirement)** |
| Contract inventory: API/file ownership | `inventory/ownership.md` |
| Contract inventory: reader fields and matrix expansion | `inventory/reader-matrix.md` (with a "Cell coverage" appendix) |
| Contract inventory: rulings | `inventory/rulings.md` (R1–R8, all pending the author) |
| Numeric ingress spike (ran once) | `spikes/numeric-ingress/findings.md`, throwaway crate `spikes/numeric-ingress/src/` (outside the workspace), captured runs in `spikes/numeric-ingress/runs/` |
| Matrix fixtures | `darkmatter/lib/tests/l1/schema_enhancements_matrix/` (5 L1 tests) and `darkmatter/lib/tests/fixtures/schema_enhancements_matrix/` with `PROVENANCE.md` |

### Key findings

- **Examples: 121, not 117.** 74 executable, 47 display-only. The spec's 117
  is the catalog at commit `78540226f` (2026-09-17); `find_files` and
  `try_frontmatter` have since added one executable example each and
  `code_link` two display-only ones. The 72-executable floor is still met.
  Recommendation: carry all 121.
- **`trim` is not a registered function**, but the spec's null table and
  criterion 43 (`trim(null)` → `null`) use it. Criterion 43 cannot pass as
  written without adding `trim` or changing the criterion.
- **`keys`/`values` are not registered yet** (planned by this feature).
- **The draft replacement catalog** (`claudine/docs/schemas/partials/functions.yaml`)
  is missing `find_files`, `code_link`, `try_frontmatter` and lists
  `is_file_type`, which is out of scope and unregistered.
- **Converted-input fixture impossible for 34 functions**: 24 whose only
  parameter is `unknown | null` (7 type predicates, 14 date predicates,
  `file_exists`, `and`, `or`) and 10 taking `any[]` (`first`, `last`, eight
  `as_*`). `and`/`or` have no rejected fixture either. This is ruling R1;
  **criterion 10 stays blocked until the author rules.**
- **Ownership conflict:** `expr/functions/mod.rs` (6,041 lines) holds the
  dispatch, the binding types and 27 context handler bodies, so the Wave 5
  Pure and Context handler tasks would collide. Proposed: before Wave 5, the
  integrator moves the context bodies into their family files (move only).
  Unowned files: `schemas/resolve.rs` (→ Schema adapter) and
  `schemas/mod.rs` (→ integrator). The name `expr/binding.rs` is taken, so
  the Call binder needs another module name. Nine catalog consumers the plan
  does not name exist, including `claudine/gen/src/inputs.rs:292` and
  Claudine's grouping in `claudine/cli/src/commands/context/mod.rs:67`.
- **ConfigState drift (plan rule 8):** the plan's claim is only partly right.
  The `ConfigState` docs (`dmls/src/config/mod.rs:457`) already say the file
  layer is kept, matching `recompute`. The drift is in the test
  `test_semantic_tokens_malformed_value_keeps_previous` (`:1079–1081`), whose
  comment says the last good config survives. Left for the Wave 8 config task,
  which owns that file. Also: one invalid editor value discards *every*
  non-formatting overlay section (existing contract, kept).
- **`workspace/configuration` is never requested by DMLS** (documented). The
  spec's criterion 40 wording is read as the existing `didChangeConfiguration`
  channel (ruling R4).
- **Prerequisite comment drift (not fixed; owned by Phase 5):**
  `PredicateFact::FileExists` (`facts.rs:28`) says the handler rejects
  non-strings; it returns `false`.

### Numeric ingress: selected seam

A **number-spelling sidecar**: each loader keeps building today's
`serde_json::Value` and also records a JSON Pointer → written-number-text map
for numbers stored as `f64`, like the existing `MappingOrders`. One admission
function checks exactness from the spelling. `serde_json/arbitrary_precision`
was rejected (no effect on YAML/JSON5, workspace-wide feature unification
into 67 crates, and it breaks `Value(1.0) == Value(1.00)`). Darkmatter must
declare `serde_json` `raw_value` and `float_roundtrip` itself. Spike
recovered every spelling except YAML `99999999999999999999`, which fails the
whole frontmatter today. Four cell-level decisions for the author are in
`findings.md`.

### Existing defects found (not fixed in Phase 1; each is owned by a later task)

| Defect | Owner task |
|---|---|
| `standalone.rs:127-130` turns a YAML parse error into `Ok(None)`; DMLS then drops it (`named_types.rs:344`, overlay `:231`) | Grammar (Wave 2) |
| Duplicate-key and trailing-content errors in schema files reported as `AmbiguousReferenced` with no detail (`resolve.rs:759`, `:1152`) | Schema adapter (Wave 4) |
| `coerce.rs:597` rounds big integers silently; its comment is wrong | Numeric foundations (Wave 2) |
| `grammar.rs:279`, `:1948` parse type-expression numbers with bare `parse::<f64>()` | Grammar + Numeric (Wave 2) |
| YAML `.inf`/`.nan` and JSON5 `1e400`/`Infinity` become `null`; `1e-400` becomes `0`; YAML integer > u64 fails the whole frontmatter; enum members saturate through `i64`; `min(inf)` accepted | Numeric foundations (Wave 2) |
| `Markdown::from(String)` swallows frontmatter parse errors (`markdown/mod.rs:999-1003`) | not in this feature's scope; reported |
| `catalog.rs:425-438` silently skips a malformed schema-root catalog | not in this feature's scope; reported |
| `md set` stores malformed JSON as a string without warning | only if the numeric seam is routed through it |

### Matrix fixtures

Positive controls come from real producers (shipped `schemas/partials/timing.yaml`
and a pure-envelope shipped schema, `md schema detect --format json` output
and its YAML twin, `md get --yaml` output); keyword fixtures that `detect`
does not emit are hand-written and labeled so in `PROVENANCE.md`.

Tests (all `darkmatter` `l1`, prefix `schema_enhancements_matrix::`):

- `envelope::shipped_tagged_envelope_matrix`
- `envelope::shipped_pure_envelope_matrix`
- `raw_schema::raw_json_schema_keyword_matrix` (48 keywords, `.json` branch)
- `raw_schema::raw_yaml_schema_keyword_matrix` (48 keywords, YAML raw branch)
- `frontmatter::frontmatter_value_matrix`

Only cells current code already decides correctly are asserted. Where the
outcome is right but the error class is mislabeled (`AmbiguousReferenced`),
the test asserts only that loading fails; the fixing task tightens it.
Every other cell is assigned to its task or ruling in
`inventory/reader-matrix.md` → "Cell coverage". DMLS cells wait for the
`expressions` field (Wave 8). Observed contract: an absent optional property
composes as `null`, so absent and explicit null differ only in the source.

`darkmatter/lib/tests/l1/schema_yaml_spellings.rs` was edited only to make
`Edit` and `apply` `pub(crate)` for reuse.

Targeted verification:

- `cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(/schema_enhancements_matrix/) | test(/schema_yaml_spellings/) | test(/test_layout/)'` → 30 passed (slowest new test 0.63 s)
- `cd darkmatter && just lint` → clean

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no behavior, so there is no regression test to add. The
behavior Phase 1 pins is the current decided matrix cells above. The plan's
criteria-to-test mapping for later phases is `inventory/criteria-map.md`.

### Gates

Scope calculator (run from the repository root after all edits):
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
→ `packages: [darkmatter, test-toolkit, claudine]`, `change_class: package`.
The gates were therefore required, and all were run serially with no
subagent running:

| Gate | Result |
|---|---|
| `claudine`: `just test` | 10,691 passed, 9 skipped, 0 failed (332 s) |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `darkmatter`: `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` (the binary this phase changed) | 2,073 passed, 0 failed |
| `darkmatter`: `just lint` | clean |

No failures, no load-sensitive reruns, no pre-existing failures observed.
Dependent packages: Phase 1 changed only test files and fixtures inside the
`darkmatter` `l1` binary, so no dependent package can observe the change.
Cross-OS check skipped: the new tests use in-memory fixtures and temp
directories, with no OS-specific path handling; CI covers the other OSes.
The `claudine` skill needed no update (no Claudine behavior changed).
