---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-16-core-schema-types/spec.md"
plan: "claudine/features/2026-10-16-core-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/alphanumeric.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/fixtures/detect/arrays/expected.yaml
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/cli/src/commands/context/format.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/inline-compose.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/sequence.trigger.yaml
    - darkmatter/docs/schemas/claudine.yaml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_core_types.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - claudine/lib/src/composition/inline_prompt.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/src/completion/schema_completion/keys.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - darkmatter
    - claudine
    - claudine-cli
    - darkmatter-cli
    - dmls
---

# Implementation Log for 2026-10-16-core-schema-types (6 phases)

## Phase 1

Phase 1 is inventory and feasibility only. **No repository source file was
changed.** The regex spike ran as a throwaway crate in `/tmp`. Paths below are
relative to `darkmatter/` unless they start with `claudine/`; `S` means
`darkmatter/lib/src/markdown/schemas`.

### Baseline (before any Phase 2 edit), 2026-10-12, macOS host

| Gate | Result |
|---|---|
| `claudine`: `just test` | 8684 passed, 9 skipped |
| `claudine`: `just lint` | clean |
| `darkmatter` lib, `--test l1 --lib`, filter `schema\|trigger\|convert\|coerce\|phase\|about\|corpus\|spelling` | 1832 passed |
| `darkmatter-cli` + `dmls`, `--test l1`, filter `schema\|trigger\|overlay\|meta_schema` | 280 passed |

No baseline failures. Unrelated working-tree changes were present before this
phase and were left alone: the move of `2026-10-15-diagnostic-node-positions`
into `_completed/` (three `D` entries plus one untracked directory).

### Wave 1 — Contract inventory

**Test layout.** `lib`, `cli`, and `dmls` each set `autotests = false` and
have one consolidated L1 binary at `tests/l1/main.rs`. `tests/l1/test_layout.rs`
fails on any undeclared file, so every new file needs a `mod` line there.
Unit tests live in `#[cfg(test)]` blocks and in `S/tests/mod.rs`.

**Acceptance criteria → tests (existing tests to extend; new tests and where they go)**

| AC | Extend | Add |
|---|---|---|
| 1 | `lib/tests/l1/schemas_validate_table.rs::validate_table` (fixture dirs under `lib/tests/fixtures/validate/`); `cli/tests/l1/schema_validate.rs` | One fixture dir per Phase A row (null, unknown/any, inclusive and XOR root unions), plus an `md schema validate` end-to-end test in `cli/tests/l1/schema_validate.rs` |
| 2 | `convert.rs`: `any_is_empty_object`, `required_scalar_atoms_are_byte_for_byte_unchanged` (row `("any(required)", vec![])` at ~2230), `optional_scalar_atoms_emit_null_wrap` | `convert.rs` unit tests for `null`, `unknown(required)`, and `shell` |
| 3 | `lib/tests/l1/schemas_named_type_merge.rs` (`union_arms_hoist_inherited_required`, `each_presence_combination_is_inherited_as_declared`) | `[person, null]` with inherited `required`; `[string(required), unknown]` |
| 4 | `lib/tests/l1/schema_yaml_spellings.rs`; `S/simplified/source.rs` tests | Native `null`/`~` at property position, with a span |
| 5 | `grammar.rs::parses_each_type_keyword` (~2038); named-merge conflict tests | New `lib/tests/l1/schemas_alphanumeric.rs`; option-set conflict in `schemas_named_type_merge.rs` |
| 6 | none | Unit tests beside the new subtype function (new `S/simplified/subtype.rs`, or in `types.rs`) |
| 7 | `lib/tests/l1/schemas_literal_expression.rs` `expression_*` (~435–530) as the template; `dmls/tests/l1/lsp_session.rs` | `shell_*` tests; a lifecycle-shaped fixture through `md schema validate` and DMLS |
| 8 | `cli/tests/l1/schema_triggers.rs::dialect_family_has_identical_compose_validate_and_trace_activation`; `dmls/src/overlay/mod.rs` (~1783–1817); `S/triggers/lint.rs` vacuous tests; `shipped_schema_corpus.rs`; `convert.rs::optional_any_accepts_null_and_required_any_is_presence_only` (~2417) | A malformed-`steps` registry-decision test; side-effect partial tests |
| 9 | `dmls/src/providers/frontmatter.rs::meta_schema_completion_catalog_is_descriptor_driven` (~4246); `lsp_session.rs::meta_schema_phase7_inline_hover_completion_and_diagnostics` | Option completion before `;`; hover with the `any` alias and the effective character set |
| 10 | `about.rs` (`simplified_type_keyword_set_matches_descriptor_set`, `descriptor_keywords_are_parseable`); `cli/tests/l1/schema_about.rs::schema_about_lists_every_supported_type_keyword` | Alias and options parity |
| 11 | `shipped_schema_corpus.rs::shipped_schema_artifacts_parse_passively` | Corpus cases plus the end-to-end test |
| 12 | `lib/tests/l1/schema_phase_validation.rs` (eager/generated/raw-JSON tests); `compose/schema_validation.rs::document_optional_bindings_materialize_before_coercion_and_are_idempotent` (~1640); `triggers/matcher.rs` tests | One cross-surface presence matrix test in `lib/tests/l1/` |
| 13 | `coerce.rs` tests; `lib/tests/l1/schemas_union_coerce.rs`; `serialize.rs` round trips | Alphanumeric edges and no coercion; a serializer test that emits `unknown` and `"null"` |
| 14 | `schemas_literal_expression.rs::literal_bare_null_rejected` (stays an error); `meta_schema_phase*.rs` | Null named type through local and imported refs; a null-default error with a source location |

**Code map**

- `SimplifiedType` (`S/simplified/types.rs:~367`) is `Copy`; `Any` is at ~418,
  with `as_keyword`/`from_keyword` beside it. Presence helpers are
  `PropertyDef::is_required` (~154, `pub(crate)`) and `PropertyAtom::is_required` (~308).
- **Hazard:** `SimplifiedType::Any` doubles as a "target type not yet known"
  placeholder for inline-object postfixes (`grammar.rs` ~557, ~577),
  named-reference postfixes (~1080, ~1095), and `deferred_import`. The new
  `Unknown` semantics must not reuse this placeholder. Give the placeholder its
  own representation, or make sure presence and lowering never see it.
- Grammar: `parse_type_expr` (~840) checks an explicit `@` first, then a keyword,
  otherwise a bare reference. `parse_constraint_list` (~1151) already parses
  enum members (~1170) and literal values (~1182) positionally before `;`; that
  is the precedent for alphanumeric options. `min`/`max` map to string length
  only for `String` (~1497, ~1522), and `suggest` is limited to String/Number
  (~1551). Options on a named reference (`slug(allow-dashes; max(10))`) are
  parsed before the target type is known, so the parser must keep them
  until resolution.
- YAML shape: `S/simplified/mod.rs::parse_property_definition_at_depth` (~310)
  rejects YAML null with "got null" (~375–383), and sequence arms reject it
  (~347). Root parsing is `parse_yaml_schema` (~113); root null must stay an
  error.
- Lowering (`S/simplified/convert.rs`): `object_body_from_shape` (~651) builds
  `required` (~662–668, ~722). `union_property_to_schema` (~195) computes required
  at ~205–208 and adds the null arm at ~248. `wrap_optional_null` (~304) is called
  from `atom_to_schema` (~308; decision at ~318). `atom_fragment_without_null_wrap`
  is at ~390. `email_fragment` (~1301) is the analog for alphanumeric,
  `expression_fragment` (~1369) for shell, and `any_fragment` is at ~1409.
- Shell analog: `DARKMATTER_EXPRESSION_FORMAT` (`S/format.rs:~111`) is registered
  in `register_darkmatter_formats_with` (~213). The closure decodes through
  `literal_token::decode_leaf`, then defers pending values via
  `is_pending_expression_value`. `coerce.rs::coercion_target` (~200) already
  sends any non-yaml/json `type: string` to `ToString`, so `shell` needs no
  coercion change.
- **Alphanumeric coercion hazard:** that same `ToString` rule would coerce `3`
  to `"3"`, which then passes the pattern. AC5 and AC13 require a type
  mismatch, so `coercion_target` must decline coercion for an alphanumeric
  target. That covers scalars, array items, and union arms (`coerce_property_union`, ~482).
  The decision should come from the typed atom or the `x-darkmatter-type`
  annotation the lowering emits.
- Other keyword lists: `serialize.rs` (via `as_keyword`); `about.rs`
  descriptors (~100–131; the descriptor struct has no alias or options field
  yet); `completion.rs` (`is_completable` ~240); `detect.rs` (~146/153/182,
  infers null as `Any`); `cli/src/commands/schema/assignment.rs:~196`
  (`atom_prefers_string`). `compose/expression/catalog` `DataType::Any` is the
  function-parameter type and is out of scope.
- **Exhaustive matches in Claudine** that the new variants will break at
  compile time: `claudine/lib/src/composition/schema/classify.rs` (~343, ~366)
  and `claudine/cli/src/commands/context/format.rs:~29`.

**Validator builders**

| Builder | Registers Darkmatter formats? |
|---|---|
| `S/validate.rs::build_validator_with`, used by `build_structural_validator`, `build_validator_in_context`, `build_validator_with_callers`, and `ValidatorCache::{validator_for, structural_validator_for}`; reached by coerce, completion, example, `S/mod.rs`, and `file_match` | yes |
| `S/format.rs` tests (public `register_darkmatter_formats`) | yes |
| `compose/expression/catalog/parser.rs:~174` `jsonschema::validator_for` (production) | **no**: it already misses the expression/yaml/datetime formats, and would miss `darkmatter-shell` |
| Test-only builders: `S/mod.rs` ~2635/~2671, `S/tests/mod.rs` ~904/~960, `S/simplified/mod.rs:~932`, `S/resolve.rs:~3228`, `lib/tests/l1/suggest_constraint_phase3.rs:~35` | no (they only check that schemas compile) |

Claudine, DMLS, and the CLI build no validators of their own. Seam: register
`darkmatter-shell` in `register_darkmatter_formats_with`. Phase 3 should decide
whether the catalog parser routes through `ValidatorCache` (it validates
function-signature schemas whose types could include `shell`).

**Independent presence checks found**

1. `convert.rs`: union (~205–208), atom wrapper decision (~318), and arm required (~400).
2. `triggers/matcher.rs:~488` has its own `is_required`. It ignores `array_constraints`,
   so it already disagrees with `PropertyAtom::is_required`. `property_matches`
   (~203) treats a present null as absent.
3. `triggers/lint.rs:~96` `is_gate` reads only the value constraints.
4. `phase.rs:~120` `project_atom` re-adds Required per union arm (~133–137).
5. `compose/schema_validation.rs:~551` uses `PropertyDef::is_required`
   (optional-property materialization).
6. `completion.rs`: `add_keys` (~552), and `alternative_accepts` (~417) pushes
   Required to suppress the wrapper.
7. DMLS: `dmls/src/providers/frontmatter.rs:~2425` `is_required` (`declares_constraint`), used at ~696, ~1483, ~1729.
8. Claudine: `claudine/cli/src/completion/schema_completion/keys.rs:~273`,
   `claudine/lib/src/composition/inline_prompt.rs:~84` `def_is_required_at_completion`,
   `claudine/lib/src/composition/schema/classify.rs:~196`.
9. `detect.rs` (~441–465) edits Required during inference. That is authoring,
   not a presence decision, so it is out of scope.

**Intended shared seam:** one public presence calculation on the typed model in
`S/simplified/types.rs`. Make `PropertyDef::is_required` public (or add
`PropertyDef::presence()`), plus `admits_null_whole_value()`, both over
`PropertyDef::alternatives()` so a `Union` and a postfixed `TypeExpr::Union`
are both covered. A whole value admits null when it is not an array and its
scalar type, or any arm, is `null`/`unknown`. Such a value is never required,
whatever `required`/`eager`/`generated` says. The function is documented as
needing **resolved** input. Every site 1–8 calls it. `convert` passes the
property-level answer down instead of re-deriving it per atom, and skips the
optional-null wrapper for null-admitting values, so `unknown` → `{}` and
`null` → `{"type":"null"}` (this changes
`snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap`, which today shows
`anyOf[{type:null},{}]` for optional `any`).

**Consumers**

- DMLS type-keyword completion iterates `schema_type_descriptors()`
  (`frontmatter.rs:~415`). Constraint completion's `accepts_constraint` (~547)
  **splits the `accepted_constraints` display string**, which is the
  display-string reconstruction the spec forbids. Phase 4 must expose typed
  accepted constraints and option keywords from the library.
  `SchemaCursorRole::Constraint` (`S/simplified/cursor.rs:~38`) does not track
  "before the first `;`", so option completion needs that cursor state.
- Named-type diagnostics project through `dmls/src/diagnostics/named_types.rs:~131`,
  and each `SchemaConstraintOrigin` becomes `relatedInformation`.
- Option-set conflict seam: `S/resolve.rs::apply_import_postfix` (~1873;
  single-atom path ~2035–2058), before `merge_constraints`. Add a
  `SchemaConstraintConflictKind::Options` (`S/simplified/source.rs:~152`) raised
  through `MergeSite::conflict`. `locate_origin_spans` fills spans. DMLS related
  locations come for free if the option list is recorded as one origin keyword.
  `SchemaSourceMap::constraint_span` will need to recognize it.

### Wave 1 — Migration inventory

**`any(required)` sites** (no spacing variants, no `any[](required)`; spec
snapshots under `claudine/features/**` and `claudine/fixes/_completed/**` are left
untouched)

| Site | Decision |
|---|---|
| `tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml:7` (`none`) | `steps: unknown[](required)` |
| `.../inline-compose.trigger.yaml:7` (`none`) | same |
| `.../sequence.trigger.yaml:4` (the only positive gate, so it would be vacuous if left) | same |
| `docs/topics/schemas/definition.md:~2322` (trigger example) | `unknown[](required)` |
| `docs/topics/schemas/definition.md:~99` (`any` row, "presence-only") | rewrite: `unknown` canonical, `any` alias, `required` has no effect |
| `S/simplified/convert.rs:~2230` table row | keep the row (still `{}`, not in `required`); add an `unknown(required)` row |
| `S/simplified/convert.rs:~2411–2428` | drop the "rejects absent" assertion, keep explicit-null acceptance, rename the test |
| `claudine/docs/schemas/partials/side-effect.yaml:10,16` | see below |
| **`docs/schemas/claudine.yaml:64`**: `state: "any(generated; required) …"` (not in the spec's list) | `object(generated; required)`. Claudine always injects `state` as an object (`claudine/lib/src/composition/sequence/model.rs` `StepState::to_value`, and the `state` insert ~318). Otherwise `state` would silently become optional at completion. `previous_state`/`next_state`/`compatibility` are optional `any` and stay as the alias spelling (or become `unknown`). |

Other wording sites: `about.rs:~127–130` (the `any` descriptor says "only care
that a property exists"; this becomes false), `docs/inline/schema-validation.md`
(~54 type list, ~333 coercion table), `docs/topics/schemas/local-type-references.md:~57`.
`schema_phase_validation.rs::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
pins exact `definition.md` sentences, so update it together with the doc.

**Undeclared-variable `any` wording:** none found. Diagnostics and docs already
say "of unknown type" (`dmls/docs/diagnostics.md` ~183/~213,
`docs/inline/interpolation.md:~151`). `docs/inline/doc-list.md:~32` already
says "`any`/`unknown`". No change is required; optionally put `unknown` in
backticks.

**Obsolete rejection expectations** (null at a property/arm/named-definition position):

- `lib/tests/l1/schema_yaml_spellings.rs` ~136–137 (`title null`/empty), ~164–165
  (`status null arm`/empty arm), ~226–227 (`level null`/empty).
- `lib/tests/l1/schemas_union_schema_matrix.rs` ~174 (named `policy: null`),
  ~238–239 (`content_policy: null`/empty).

These become "loads as the null type" (the witness instance then fails or
passes on its value). Expectations that **stay errors**: `kind`/`types` null,
root `$schema` null, null root-union arm (`schema_yaml_spellings.rs` ~99–112,
~201–202, ~259–260; `schemas_union_schema_matrix.rs` ~124–125, ~149, ~202–203),
`literal(null)` (`grammar.rs:~2471`, `schemas_literal_expression.rs:~130`),
`$path` null (`schema_roots.rs:~497`), and the `$constraints` null rows. A bare
`unknown` was never tested as a type: today it fails as `UnknownNamedType`, and
nothing pins that. No `.snap` asserts these rejections.

**Local types named `unknown`/`null`/`alphanumeric`/`shell`:** none in shipped
schemas or fixtures; they appear only in spec spike files. Watch
`claudine/docs/schemas/partials/stack.yaml:17` `shell@./shell.yaml`. It is
explicitly qualified (and already a Draft corpus entry), but the grammar must
still read `shell@…` as an import once `shell` is a keyword.

**`steps` domain.** None of the three trigger target schemas declares `steps`.
The only `steps` value any test or shipped artifact uses is `steps: []`
(`tests/fixtures/schema-triggers/docs/sequence.md`); `claudine.md`,
`inline-compose.md`, and `plain.md` have none. No test uses a malformed value,
and there are no shipped `steps:` producers (Claudine's real key is
`sequence:`). The matcher already treats a present null as absent, so today
`any(required)` means "present, non-null, any type". `unknown[](required)` keeps
the `[]` match and the null/absent behavior, and stops matching non-list
non-null values. The resulting registry decisions, which Phase 4 must assert:

| Input | Today | After |
|---|---|---|
| `steps: []` alone | sequence | sequence (unchanged) |
| `steps: {a: 1}` or `steps: oops` alone | sequence | no trigger (no gate matches) |
| `prompt: x` + `steps: oops` | none (excluded by the `none` guard) | **inline-compose** (the `none` guard no longer excludes it; `output` is then required by the target) |
| `kind: claudine` + `steps: 5` | none | **claudine** (`provider` is then required) |
| `steps: null` / absent | as absent | as absent (unchanged) |

Decision: `unknown[](required)`. No evidence supports narrowing the items. The
intentional selection change above is the documented migration policy and must
be tested as a registry decision, including the target-schema failure.

**Side-effect partial** (`claudine/docs/schemas/partials/side-effect.yaml`). It
is not embedded; its only references are `stack.yaml:18` and the corpus. **It
does not load today.** `set_frontmatter`/`merge_frontmatter` are written as
`|-` block strings of pseudo-YAML, and the corpus classifies the file as
`Draft { "unexpected character ':'" }` (`shipped_schema_corpus.rs:~226`).
Per-field decisions:

| Field | Operation (`darkmatter/lib/src/effects/verbs.rs`) | Decision |
|---|---|---|
| `set_frontmatter.value` | `set_frontmatter(file, prop, value)` stores a supplied null verbatim (~88–106) | null **has a defined meaning** (store null) → null-admitting union of every accepted type including `null`; `required` has no effect |
| `merge_frontmatter.prop`, `.value` | `merge_frontmatter(file, obj)`; a non-object fails `PropertyType` (~109–129); a null inside the object is inserted | remove both; add `obj: object(required)`. The argument name `obj` matches `claudine/lib/src/composition/lifecycle/signatures.rs:~154`. Also fix the copy-pasted `$description`. |

There are no other `any` fields. Lines 19–28 (`delete_frontmatter: null` …) are
`null` **placeholders**, and so are `claudine/docs/schemas/partials/lifecycle.yaml:4–9`
(`initialize: null` …). Under this spec they become the real `null` type ("must
be null or absent"), which is not what they mean. Phase 4 decides; see the
message to the next agent. Claudine's action parser rejects a literal YAML null
argument (`action_shape.rs:~377`), so null reaches `value` only through
interpolation.

**Corpus scanner.** `lib/tests/l1/shipped_schema_corpus.rs` walks eight schema
directories (skipping `fixtures`). Every file is `Loadable` unless its
`EXCEPTIONS` entry (~78–249, 25 entries) says Placeholder / NotASchema /
EnvelopeRejected / Draft{substring}, and a stale exception fails the test.
Entries affected by this spec:

- `claudine/docs/schemas/partials/side-effect.yaml` (Draft "unexpected
  character"): once the two definitions become real mappings, this error
  disappears. Expect `EnvelopeRejected` (top-level `description` key) or
  `Loadable`. AC8 requires the partial to load, so the end state must be
  asserted, not re-excepted.
- `claudine/docs/schemas/partials/lifecycle.yaml` (Draft "got null"): its
  `initialize: null` siblings will now parse. The empty `types:` may still fail
  with a message that also contains "got null", which would keep the exception
  green by accident. Re-check what it matches and pin the precise error.
- `partials/stack.yaml`: its spread syntax still fails, so it probably stays.

### Wave 2 — Regex spike

Engine facts: `S/validate.rs::build_validator_with` uses
`PatternOptions::regex()` (the linear `regex` 1.12.3 crate), and switches to
`fancy-regex` only when `schema_uses_lookaround` finds lookaround (pattern-key
literal precedence). Under fancy-regex, a pattern without fancy features is
delegated to `regex` unchanged, so a lookaround-free alphanumeric pattern stays
linear in both modes. `jsonschema` 0.55 `pattern` is a search, so the pattern
must anchor itself. Rust `regex` `^`/`$` without the `m` flag anchor to the
haystack ends only (no implicit trailing-newline match).

Construction. `C` is the allowed class: `[A-Za-z0-9` plus the escaped allowed
extras `\-`, `_`, `/`, `\.`, space, then `]`. `F` is the first-character class:
`[A-Za-z]` for `leading-alpha`, otherwise `C` minus `-`/`/` when blocked.

- without `block-double-periods` (or without `allow-periods`): `^(?:F C*)?$`
- with both: let `N = C \ {.}` and `FN = F \ {.}`. Then
  `^(?:LEAD(?:N+\.?)*)?$`, where `LEAD = (?:FN\.?|\.)` if `.` ∈ F, otherwise `FN\.?`.

Each pattern is one ordinary regex with no lookaround and no backreference; it
compiles under `PatternOptions::regex()`. The spike also proved that the linear
engine rejects `^(?!\.)`, so the run was on the linear engine.

Samples checked through `jsonschema` with `PatternOptions::regex()` (all as expected):

| Options | Accepted | Rejected |
|---|---|---|
| base | `""`, `abc1` | `a.b`, `abc\n`, `\nabc`, `a\tb`, `é`, `a b`, U+00A0, `ａ` |
| `allow-periods, block-double-periods` | `""`, `a.b`, `.`, `.a.b.` | `a..b`, `..`, `a...b`, `a.b..`, `a.b\n`, `\n` |
| `allow-slashes, allow-periods, block-double-periods, block-leading-slash` | `""`, `a/b.c`, `./a` | `/a`, `../a`, `a/../b`, `a\b` |
| `leading-alpha, allow-underscores, allow-periods, block-double-periods` | `""`, `abc`, `a_1.b` | `1abc`, `_a`, `.a`, `a..b` |
| `allow-dashes, block-leading-dash` | `""`, `a-b`, `a-` | `-a` |
| `allow-whitespace` | `a b`, `" "` | `a\tb`, `a` U+3000 `b`, `a\nb` |
| `allOf[periods+no-dd pattern, pattern("^a"), minLength 1]` | `a.b` | `""`, `b.a`, `a..b` |
| base + `x-darkmatter-type`/`x-darkmatter-options: []` | `abc` | `a-b` (annotations are inert) |

Typed API contract (settled for Phase 2):

- an `AlphanumericOptions` value is a `Copy` bitset over the nine keywords, so
  it fits the `Copy` `SimplifiedType`
- `canonical()` drops redundant narrowing options: `block-leading-dash` without
  `allow-dashes`, `block-double-periods` without `allow-periods`,
  `block-leading-slash` without `allow-slashes`, and both leading blocks under
  `leading-alpha`
- `keywords()` yields table order
- `allowed_chars()`/`first_chars()`/`forbids_double_period()` are the semantic
  sets the subtype relation compares
- `pattern()` emits the construction above from the canonical form

No blocker; the specified language needs no engine change. The sample table
above is the case list Phase 2 must keep as ordinary L1 tests against the real
lowering (the spike crate itself is not retained).

### Requirement → test mapping for Phase 1

Phase 1 changes no behavior, so it adds no tests. The regex samples above
become required Phase 2 test cases (AC5, AC13). Broader gates run: see the
baseline table. Nothing was skipped, and there were no pre-existing failures.

## Phase 2

Phase 2 implements the shared type model: the four types in the AST and
keyword catalog, typed `alphanumeric` options with one canonicalizer, the
option grammar, native YAML null at property positions, and option identity
through named references. Paths below are relative to `darkmatter/lib/src/`
unless stated; `S` means `markdown/schemas`.

### Decisions

- **`Any` is renamed `Unknown`.** `SimplifiedType::Unknown` has the keyword
  `unknown`; `from_keyword` accepts `unknown` and `any`;
  `SimplifiedType::aliases()` returns `["any"]` for it. Serialization emits
  `unknown`. New variants: `Null`, `Alphanumeric`, `Shell`.
- **The "target not yet known" placeholder is gone.** `grammar.rs` used
  `SimplifiedType::Any` (plus a `defer_suggestion_target` flag) for the lists
  of inline objects and named references. A private `ListOwner { Type(ty),
  InlineObject, Reference }` now names what a constraint list belongs to, so
  `unknown` never doubles as a placeholder. Error text for a misplaced
  `suggest` changes from "on `any`" to "on an inline object" / "on a
  named-type reference".
- **Options are a positional constraint.** Following the `enum`
  (`Constraint::Members`) and `literal` (`Constraint::LiteralValue`)
  precedent, options ride in `Constraint::Options(Vec<AlphanumericOption>)`
  in authored order. That keeps `SimplifiedType` `Copy`, and it lets a named
  reference (`slug(allow-dashes; max(10))`) carry options until resolution,
  when the target is known. `Constraint::keyword()` for options is the
  **first authored option**, which makes `SchemaConstraintOrigin` and
  `SchemaSourceMap::constraint_span` locate the option group with no new span
  plumbing. `constraint_span` now also accepts `,` after the keyword, because
  several options form one `;`-separated group.
- **One canonicalizer** (new `S/simplified/alphanumeric.rs`):
  `AlphanumericOption` (nine variants, table order), `AlphanumericOptions` (a
  `Copy` `u16` bitset), and `AsciiCharSet` (a `u128` bitset). API:
  `canonical()`, `options()`/`keywords()` (table order),
  `allowed_characters()`, `first_characters()`, `forbids_double_period()`,
  `accepts(&str)`, and `pattern()` (the Phase 1 spike construction).
  `PropertyAtom::alphanumeric_options()` returns the canonical set for an
  `alphanumeric` atom. All are re-exported from `S` for DMLS and the subtype
  relation.
- **Option grammar** (`parse_option_prefix`): a list owned by `alphanumeric`
  or a reference that opens with an option keyword reads options up to `;`
  or `)`. A list that opens with anything else is constraints only.
  Exceptions: an unrecognized word in an `alphanumeric` list is "unknown
  `alphanumeric` option"; a repeated option, an option with arguments, an
  option after `;`, and a list without `;` that mixes options and
  constraints (in either order) are grammar errors. The mixed-list message
  shows the `alphanumeric(allow-dashes; min(1))` form. An option keyword on
  any other type is rejected as "an `alphanumeric` option and is not valid on
  `<type>`". `min`/`max` count characters on `alphanumeric` as on `string`.
  `suggest` is accepted on `alphanumeric`, with string candidates.
- **Named-reference option identity** (`S/resolve.rs::check_use_site_options`,
  called from `apply_import_postfix` before constraint merging): use-site
  options are removed from the constraint list. An equal canonical set is a
  repetition and is dropped. A different set is a `ConstraintConflict` with
  the new `SchemaConstraintConflictKind::Options`, whose origins are the
  definition's option group (span filled for file sources) and the use site.
  When the base type writes no options, the definition origin uses keyword
  `alphanumeric` at the declaration path, with no span. Options on a
  non-`alphanumeric` target are a `Convert` error. On a union target, the
  existing `UnionReference` rejection applies. Use-site `min`/`max` on an
  `alphanumeric` target are reinterpreted as lengths, and `suggest` on an
  imported `alphanumeric` target is accepted.
- **Native null at property positions** (`S/simplified/mod.rs`): `YamlValue::Null`
  is `PropertyAtom::bare(Null)` at a property definition, a property-union
  arm, a nested mapping property, and a named definition (named definitions
  go through the same parser). Root `$schema`, root-union arms, and
  `literal(null)` are untouched and still errors.
- **Source projection of null** (`S/simplified/source.rs`): a `Null` atom
  whose scalar is a YAML null spelling (`""`, `~`, `null`, `Null`, `NULL`)
  records `Atom` and `TypeKeyword` spans over the whole scalar. The strict
  locator now models an empty mapping value (`key:`) and an empty block
  sequence item (`-`) as an empty scalar; before this phase both were
  projection errors in strict mode, which would have made a newly legal
  `review:` fail to load.
- **Serializer**: `render_constraints` writes the canonical options first,
  joined with `,`, and drops redundant ones. `alphanumeric(block-leading-slash)`
  therefore serializes to bare `alphanumeric`. The YAML-level `"null"`
  quoting is left to `serde_yaml_ng`, which quotes the string `null`.
- **Meta-types**: `type-definition` values parse through
  `parse_property_definition`, so `[string, null]` and `"null(required)"`
  are valid definitions. A whole-value native null for a `type-definition`
  property still means absence, because that meta-type's JSON
  `type: [string, object, array]` excludes null. Recorded rather than
  changed, since the spec scopes meta-type parity to parsing rules.

### Landed early (Phase 3 must finish, not redo)

Adding the variants forced match arms in the converter, the trigger matcher,
the expression catalog, detection, and Claudine. These were implemented with
real behavior rather than stubs, so that `null` loads end to end and the
obsolete rejection tests could become acceptance tests:

- `S/simplified/convert.rs`: `null` → `{"type":"null"}`, and a non-null
  `default` is a `Convert` error naming the property (it has **no source
  span yet**). `unknown` → `{}`. `alphanumeric` →
  `string_fragment_with_base` with the canonical pattern first and user
  patterns/`not-empty` as further `allOf` entries, plus `x-darkmatter-type`
  and `x-darkmatter-options` (`[]` for the base). `shell` →
  `{type: string, format: darkmatter-shell}`, with only the universal
  constraints accepted.
- `S/format.rs`: `DARKMATTER_SHELL_FORMAT` is registered in
  `register_darkmatter_formats_with`, with literal-token decoding and
  pending-value deferral mirroring `expression`.
- `S/triggers/matcher.rs`: `Null` matches null, `Shell` matches non-blank
  text, and `alphanumeric` is checked through `accepts`. `Constraint::Options`
  is match-safe (`triggers/grammar.rs`).
- `compose/expression/catalog/mod.rs`: `Unknown` → `DataType::Any`; the new
  types have no function-parameter type (`None`).
- Claudine `classify.rs`: `alphanumeric`/`shell` prompt as plain text;
  `unknown`/`null` are not prompted. Claudine CLI `context/format.rs`:
  colors.
- `S/about.rs`: minimal `unknown`/`null`/`alphanumeric`/`shell` type
  descriptors, an `<options>` constraint descriptor, and an `<options>`
  match-safe descriptor, so the descriptor parity tests pass. The `<options>`
  form is `alphanumeric(opt, ...)` because a longer form widened the
  constraint table's first column and wrapped the `min`/`max` rows that
  `schema_about_uses_readable_terminal_components` asserts on. There is still
  no alias or option metadata field (Phase 4, catalog alignment).

**Not done (by plan):** presence is unchanged. `null(required)` and
`unknown(required)` still enter `required`, and the optional wrapper still
wraps `null` as `anyOf[null, {type:null}]` (Wave 4). There is no
`alphanumeric` coercion opt-out, so `3` → `"3"` still passes the base pattern
(Wave 5). There is no `alphanumeric` default/suggest value check, no shell
default conversion or structured-default rejection, and the catalog parser's
`jsonschema::validator_for` still lacks the Darkmatter formats (Wave 5). The
subtype relation is not started (Wave 5). No docs and no shipped-schema
migration (Phases 4–5).

### Requirement → test mapping (Phase 2 scope)

| Requirement | Tests |
|---|---|
| Keywords, `any` alias, canonical `unknown` | `types.rs::any_is_an_alias_of_canonical_unknown`, `simplified_type_keyword_round_trip`; `grammar.rs::parses_each_type_keyword`, `any_parses_as_unknown_and_new_keywords_take_constraints`; L1 `schemas_core_type_grammar::any_is_unknown_and_serializes_canonically`; `schemas_detect_table` (fixture `detect/arrays/expected.yaml` now `unknown[]`) |
| Keyword collisions need `@this`; `shell@./shell.yaml` stays an import | `grammar.rs::keyword_collisions_need_an_explicit_local_reference`; L1 `shipped_qualified_shell_import_stays_an_import` |
| Options, canonicalizer, regex construction (Phase 1 sample table, all 512 option subsets, linear engine) | `alphanumeric.rs::{keywords_round_trip_in_table_order, canonical_order_is_table_order_regardless_of_input_order, canonical_drops_blocks_whose_character_is_not_allowed, canonical_drops_leading_blocks_under_leading_alpha, canonical_preserves_meaning, character_sets_reflect_options, regex_class_escapes_class_syntax}`; `types.rs::alphanumeric_options_are_canonical_and_only_on_alphanumeric` |
| Option grammar: before `;`, constraint-only shorthand, unknown/repeated/post-`;`/mixed/argument errors, spans | `grammar.rs::{alphanumeric_options_precede_the_separator, alphanumeric_without_options_reads_constraints_only, alphanumeric_options_on_an_array_bind_to_items, alphanumeric_option_grammar_errors, alphanumeric_option_errors_point_at_the_option, alphanumeric_takes_string_constraints_and_suggestions, named_reference_keeps_options_for_resolution}`; L1 `option_grammar_errors_surface_through_schema_loading` |
| Named option identity: equal spellings merge constraints only; different sets conflict with related locations; every route; non-alphanumeric/union targets | L1 `equal_option_sets_at_a_use_site_resolve`, `a_different_option_set_at_a_use_site_is_a_conflict_with_both_locations`, `a_conflict_against_a_base_type_without_options_names_its_declaration`, `every_route_to_a_different_option_set_conflicts`, `options_on_a_target_that_is_not_alphanumeric_fail` |
| Native null spellings at property, union arm, nested, named positions; root/root-arm/`literal(null)` stay errors | L1 `every_native_null_spelling_is_the_null_type_at_a_property`, `native_null_is_a_union_arm_and_a_nested_property`, `root_null_null_root_arms_and_literal_null_stay_errors`, `schemas_source_projection::property_definition_parser_matches_schema_property_matrix` |
| Native null spans, empty value and empty item | L1 `a_native_null_definition_keeps_its_authored_span`, `an_empty_value_and_an_empty_union_item_project_as_null` |
| Local and imported null named types | L1 `a_null_named_type_resolves_through_local_and_imported_references` |
| Null default | L1 `a_null_default_must_be_null` (message only; the source location is a Wave 5 gap) |
| Meta-types use the same rules | L1 `type_definition_values_follow_the_same_null_rules` |
| Parse/serialize/parse meaning, canonical options, `unknown`, `"null"` | L1 `serialization_writes_canonical_options_and_round_trips_meaning`; `schemas_grammar_proptest` now generates `unknown`, `null`, `shell`, and `alphanumeric` with canonical options |
| Lowered annotations | L1 `lowering_records_the_canonical_options` |
| Obsolete rejection expectations flipped to the null type | `schema_yaml_spellings` matrices (`title null`, new `title tilde`, `title empty value`, `status null arm`, `status empty arm`, `level null`, `level empty value`, each with a witness proving null passes and a non-null value fails); `schemas_union_schema_matrix` (`policy: null`, `content_policy: null`/empty now validate as the null type, so the control list is `Invalid`) |

The input robustness matrix's YAML declaration side runs through the existing
`schema_yaml_spellings` and `schemas_union_schema_matrix` drivers. Their null
cells now assert the new outcome (null type) with witnesses. The duplicate
key, trailing-content, wrong-type, and empty cells are unchanged and green.
The full Phase 1 matrix (instance side, new-type columns) is Phase 5's
acceptance work.

Changed-reader smell audit (`S/simplified/mod.rs`, `source.rs`, `grammar.rs`,
`resolve.rs` edits): no `#[serde(default)]`, `unwrap_or_default()` on a
load-bearing parse, `filter_map(.. as_str())`, or `.ok()` was added.
`YamlValue::Null` is matched explicitly and kept distinct from absence (an
absent key is never parsed).

### Gates (2026-10-12, macOS host)

| Gate | Result |
|---|---|
| `darkmatter`: `just test` | exit 0, 9547 passed, 11 skipped |
| `darkmatter`: `just lint` | exit 0 |
| `claudine`: `just test` | 8684 passed, 9 skipped (same as baseline) |
| `claudine`: `just lint` | exit 0 (a pre-existing `ld` `__eh_frame` linker warning only) |

Failures met and fixed during the phase: nine in the first library run
(expected: obsolete null-rejection cells, `any[]` → `unknown[]` detection
output, the `suggest` target message, and the strict locator's empty
sequence item), plus one CLI `schema about` layout failure caused by the long
`<options>` form. There were no pre-existing failures.

OS: the change is pure parsing, resolution, and serialization with no paths,
processes, or terminal I/O. New L1 tests use `TempDir` and
`biscuit_file::canonicalize_simplified`, like their neighbors. No cross-OS
rig run was needed; CI covers Linux and macOS on the pull request.

## Phase 3

Phase 3 integrates presence, value semantics, and the subtype relation. Paths
are relative to `darkmatter/lib/src/` unless stated; `S` means
`markdown/schemas`.

### Wave 4 — Effective presence

- **One calculation** in `S/simplified/types.rs`: `PropertyDef::is_required`
  and `PropertyAtom::is_required` (both now `pub`) return *effective*
  presence: a `required` marker (written, inherited, hoisted from a union arm,
  or after `[]`) **and** no whole-value null admission. `admits_null()` (on
  both types) is true for a non-array `null`/`unknown`, or a non-array union
  (property-level or postfixed `TypeExpr::Union`) with such an arm. The raw
  marker check is the crate-private `PropertyAtom::marks_required`. Both
  helpers expect resolved input; an unresolved reference never admits null.
- **Lowering** (`S/simplified/convert.rs`): `atom_to_schema` returns a
  null-admitting atom's bare fragment (`null` → `{"type":"null"}`, `unknown` →
  `{}`), never in `required`. `union_property_to_schema` computes the union's
  null admission first; a null-admitting union is optional and gets no extra
  `null` arm.
- **Consumers routed through it**: trigger matcher (`triggers/matcher.rs`; its
  local `is_required` used to ignore `array_constraints`, so a
  `unknown[](required)` condition would have been a guard — now a gate), the
  vacuous-arm lint (`triggers/lint.rs`), phase projection (`phase.rs`, via
  `PropertyAtom::is_required`), completion keys (`completion.rs::add_keys`),
  composition materialization (`compose/schema_validation.rs`) and directive
  targets (`compose/directive_targets.rs`, both already on
  `PropertyDef::is_required`), DMLS (`dmls/src/providers/frontmatter.rs::is_required`),
  Claudine (`claudine/lib/src/composition/inline_prompt.rs::def_is_required_at_completion`,
  `claudine/lib/src/composition/schema/classify.rs::is_required`,
  `claudine/cli/src/completion/schema_completion/keys.rs::is_required`).
- **Migrations pulled forward from Phase 4** because the presence change
  broke them immediately: the three trigger fixtures
  (`darkmatter/tests/fixtures/schema-triggers/schemas/*.trigger.yaml`) now
  declare `steps: unknown[](required)` (Wave 1 decision; `sequence.trigger.yaml`
  was otherwise rejected as vacuous by the corpus test), and
  `darkmatter/docs/schemas/claudine.yaml` `state` is `object(generated; required)`
  (Wave 1 decision; otherwise it silently became optional). Phase 4 still owns
  the registry-decision tests for malformed `steps`, the side-effect partial,
  and the docs.
- Snapshot `l1__schemas_convert_snapshots__snapshot_atom_any.snap` now shows
  optional `any` as `{}` (predicted in Phase 1).

### Wave 5 — Value semantics

- **`null` default location**: the grammar (`S/simplified/grammar.rs`, the
  `default` arm of `parse_one_constraint`) rejects a non-null `default(…)` on
  `null` as a `SchemaError::Grammar` whose span covers the argument (`1` in
  `null(default(1))`, bytes 13..14). The converter's `null_fragment` check
  stays as the backstop for a default written at a use site of a `null` named
  type (`gone: nothing(default(1))`), which is only judged after resolution and
  carries the property name but no span.
- **`alphanumeric` defaults and suggestions** (`convert.rs::check_alphanumeric_values`):
  every `default(…)` and `suggest(…)` value is validated against the lowered
  fragment (option pattern plus every authored string constraint) with the
  structural validator, without coercion; a failure is a `Convert` error naming
  the value. A numeric `default(12)` therefore fails (write `default('12')`).
  Other types' suggestions remain lint-only (tested).
- **`shell` defaults** (`convert.rs::shell_fragment`): a string, boolean, or
  number default is converted to text the way a supplied value is coerced and
  must contain a non-whitespace character; a null, array, or object default is
  a load error. String constraints and `suggest` were already rejected via
  `reject_unsupported` (Phase 2).
- **Coercion** (`S/coerce.rs::coercion_target`): a `type: string` fragment
  annotated `x-darkmatter-type: alphanumeric` has no coercion target, so
  numbers and booleans stay native in scalars, array items, and union arms
  (other arms keep theirs: `[alphanumeric, string]` turns `3` into `"3"`
  because only the `string` arm validates). The annotation key is now the
  shared constant `format::DARKMATTER_TYPE_ANNOTATION`. `unknown` (`{}`) has no
  coercion target, so native values, including nested nulls, are preserved.
  `shell` needed no change (plain `ToString`, as `expression`).
- **Validator builders**: `compose/expression/catalog/parser.rs` built its
  catalog validator with a bare `jsonschema::validator_for`, which neither
  registered Darkmatter formats nor asserted formats. It now uses
  `validate::build_structural_validator` (made `pub(crate)`). Every remaining
  builder outside tests goes through `build_validator_with`, which registers
  `darkmatter-shell`.
- **Subtype relation** (new `S/simplified/subtype.rs`, exported as
  `darkmatter::markdown::schemas::is_subtype`): over resolved atoms. Identity
  is the primitive type plus canonical `alphanumeric` options, `enum` members
  (as a set), and the `literal` value; inline objects and resolved unions
  compare structurally; an unresolved reference equals nothing. Arrays are
  subtypes only when identical. `alphanumeric(X) ⊆ alphanumeric(Y)` compares
  `allowed_characters`, `first_characters` (`AsciiCharSet::is_subset`), and
  `forbids_double_period`. Always-text types (`string`, `date`, `datetime`,
  `time`, `file`, `enum`, `url`, `email`, `yaml`, `json`, `expression`,
  `alphanumeric`, `shell`, and a string-valued `literal`) are subtypes of
  `string`; `numberlike`, `boolish`, the meta-types, `unknown`, `null`, and
  numbers are not.
- **Docs touched minimally** because they described behavior that no longer
  exists: `darkmatter/docs/topics/schemas/definition.md` (the `any` row becomes
  `unknown` with the new presence rule; the trigger example uses
  `steps: unknown[](required)`), and the `unknown`/`null` descriptors in
  `S/about.rs` now say `required` has no effect. The rest of the docs pass is
  Phase 5.

### Requirement → test mapping (Phase 3 scope)

| Requirement | Tests |
|---|---|
| One effective presence calculation; arrays/children keep presence; concrete types still required | `types.rs::null_admitting_whole_values_are_never_required` |
| Lowering: `null`/`unknown` bare, never in `required`, no double null arm, nullable union keeps arm constraints (AC2) | `convert.rs::unknown_accepts_null_and_omission_whatever_its_presence_marker`, `null_admitting_properties_lower_bare_and_never_enter_required`, `required_scalar_atoms_are_byte_for_byte_unchanged` (new `unknown(required)`/`null(required)` rows); snapshot `snapshot_atom_any` |
| Phase A table end to end through `md schema validate`, incl. inclusive and XOR root unions, `[code, null]` with inherited `required` + `min(3)`, `[string(required), unknown]`, `null[]`/`unknown[]`/nullable child containers (AC1, AC3, AC12) | `darkmatter-cli` L1 `schema_validate_core_types::null_admitting_presence_matches_the_phase_a_table_end_to_end` (one `md` run, ~60 documents) |
| Validation, launch, and completion agree; `eager`/`generated` cannot restore presence; inherited and postfixed `required`; raw JSON Schema keeps `required` for `{}`/`{"type":"null"}` (AC12) | L1 `schemas_effective_presence::{null_admitting_whole_values_are_optional_at_every_phase_whatever_their_markers, inherited_and_postfixed_required_is_dropped_by_a_null_arm, nullable_items_and_children_leave_their_container_required, ordinary_concrete_types_keep_their_presence_and_timing, raw_json_schema_keeps_authored_required_for_nullable_properties}` |
| Trigger matching and vacuous-arm lint use the same presence; `unknown[](required)` is a gate that rejects null/scalar/object | L1 `schemas_effective_presence::trigger_conditions_use_the_same_presence`; `matcher.rs::presence_follows_the_shared_effective_calculation`; `lint.rs::null_admitting_required_condition_is_a_guard_but_a_required_list_is_a_gate` |
| Composition materializes null-admitting properties as null; a required list of nulls still fails when absent | L1 `schemas_effective_presence::composition_materializes_null_admitting_properties_and_requires_containers` |
| Claudine consumers follow the shared presence | `claudine` `inline_prompt::tests::null_admitting_whole_values_are_optional_at_completion`, `composition::schema::tests::null_admitting_whole_values_are_not_required` |
| `alphanumeric` per-option positive/negative, empty string, `min(1)`/`not-empty`, `a..b`, `1abc`, `é`, edge newlines, tabs, Unicode whitespace, overlapping constraints (AC5, AC13) | L1 `schemas_core_type_values::{each_alphanumeric_option_has_a_positive_and_a_negative_value, alphanumeric_edges_and_overlapping_constraints}` |
| Optional null accepted, required null rejected (AC13) | L1 `optional_alphanumeric_accepts_null_and_required_rejects_it` |
| Exported schema (plain `jsonschema` validator, no Darkmatter registrations), Darkmatter validator, and `AlphanumericOptions::accepts` agree; canonical annotations (AC5, AC13) | L1 `exported_schema_and_typed_options_agree` |
| No number/boolean coercion for `alphanumeric` in scalars, items, union arms; other arms keep theirs; composition rejects `3` (AC5, AC13) | L1 `alphanumeric_never_coerces_numbers_or_booleans` |
| `alphanumeric` default/suggest must be valid; other types' suggestions stay advisory | L1 `alphanumeric_defaults_and_suggestions_must_be_valid_values` |
| `shell`: blank/whitespace rejected (incl. Unicode), pipes and `;` accepted, no syntax check, optional null (AC7) | L1 `shell_accepts_any_non_blank_command_without_parsing_it` |
| `shell` scalar coercion to text, items included (AC7) | L1 `shell_scalars_coerce_to_text_like_expression` |
| `shell` constraint applicability and default rules mirror `expression` | L1 `shell_constraints_and_defaults_mirror_expression` |
| Lifecycle-shaped schema: `shell` action rejected under `initialize`, accepted under `start`; mixed and empty action objects rejected (AC7, `md schema validate`) | `darkmatter-cli` L1 `schema_validate_core_types::a_shell_action_is_rejected_under_initialize_and_accepted_under_start` |
| `unknown` preserves native scalar/object/array values and nested nulls; `null` never coerces (AC14) | L1 `unknown_values_keep_their_native_types`, `null_is_never_coerced` |
| Non-null `null` default fails with a source location; reference route still fails (AC14) | L1 `schemas_core_type_grammar::a_null_default_must_be_null` (asserts `SchemaError::Grammar` and the argument span) |
| Subtype relation: every table row, redundant options, option order, implied restrictions, constraints ignored, identical non-string types and arrays, always-text ⊆ `string`, `unknown`/numbers/arrays ⊄ `string` (AC6) | `subtype.rs` unit tests (6), doctest on `is_subtype`, L1 `subtype_relation_is_reachable_through_the_public_schema_api` |

Mutation check: disabling the `alphanumeric` coercion opt-out and the
default/suggest check each turned the corresponding L1 test red; both were
restored before the gates.

The Phase 1 instance-side input robustness matrix as one data-driven driver
remains Phase 5's acceptance work; the cells this phase changed (`A`, `S`,
`N`, `X`, `U`, `L` absent/null/wrong-type/empty) are covered by the tests
above. Changed-reader smell audit (`grammar.rs` default arm): no
`#[serde(default)]`, `unwrap_or_default()`, `filter_map(.. as_str())`, or
`.ok()` added; null default and absent default stay distinct.

### Gates (2026-10-12, macOS host)

| Gate | Result |
|---|---|
| `darkmatter`: `just test` | exit 0, 9578 passed, 11 skipped |
| `darkmatter`: `just lint` | exit 0, no warnings |
| `darkmatter`: `cargo test --doc -p darkmatter is_subtype` | 1 passed |
| `claudine`: `just test` | exit 0, 8686 passed, 9 skipped (baseline 8684 plus the two new presence tests) |
| `claudine`: `just lint` | exit 0 (pre-existing `ld` `__eh_frame` linker warning only) |

Failures met and fixed during the phase: the corpus test
(`shipped_schema_and_trigger_corpus_parses_passively`) rejected
`sequence.trigger.yaml` as vacuous once `any(required)` stopped being a gate
(fixed by migrating the fixtures), and the optional-`any` snapshot. No
pre-existing failures.

OS: the changes are typed-model logic, lowering, coercion, and validation with
no new paths, processes, or terminal I/O. New tests use `TempDir`,
`CliProcessFixture`, and relative file names. No cross-OS rig run was needed.

Unrelated working-tree changes left alone: `.claudine/memory/commits.md` and
`.claudine/non-interactive.md` were modified by something outside this phase.
