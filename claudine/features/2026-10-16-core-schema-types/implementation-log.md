---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-16-core-schema-types/spec.md"
plan: "claudine/features/2026-10-16-core-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
