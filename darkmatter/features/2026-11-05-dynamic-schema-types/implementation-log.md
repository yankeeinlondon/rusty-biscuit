---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/spec.md"
plan: "darkmatter/features/2026-11-05-dynamic-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/enum_item_tests.rs
    - darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs
    - darkmatter/lib/tests/l1/dynamic_slots_fixture_repos.rs
    - darkmatter/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/fixtures/dynamic_slots/value-positions.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/revision.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/minimum.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/maximum.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/default-retries.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/slug-pattern.yaml
docs_updated_during_phase_1:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/autocomplete.md
    - simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter-lsp
    - darkmatter
    - simplified-schema
decisions_for_review:
    - |-
        Array-item completion offers every scalar candidate of the item type, not only enum members.
        The plan asked for enum members inside a list such as the sandboxing `os[]`. The fix routes a
        `- ` item and a flow-list element of a one-type array through the same code that completes a
        plain value, so `boolean[]` items now offer true/false, `file[]` items offer paths, and
        `literal(...)` item types offer their value, besides enum members. The alternative was an
        enum-only branch, which is smaller but leaves a second, narrower copy of the value logic.
        Reverting means replacing `array_item_value_completions` with an enum-members-only lookup.
    - |-
        An unclosed flow list (`os_support: [` with no `]`) still offers nothing.
        The frontmatter no longer parses at that point, which stops every completion, not only enums;
        `suggest(...)` lists behave the same today. Editors close the bracket as it is typed, and the
        tests use that closed form. Making it work would mean a tolerant YAML recovery for the whole
        frontmatter provider, which is a separate change; the limit is documented in the DMLS docs.
    - |-
        The fixture repository offers five states of the data file: clean, dirty, untracked, ignored,
        and no repository. The approval matrix (Phase 5) also needs symlink-escaping and other-repository
        cells; they are left for Phase 5 to add on top of `SlotRepo` (it exposes the root and paths),
        because their shape depends on the grant rule built there. Adding them now would only move work.
    - |-
        A template's lowering error carries only its slots for now. The plan's `UnresolvedSlots` also holds
        a `pending` replay handle; that handle needs the resolution entry points, which Phase 3 builds, so
        the struct is `#[non_exhaustive]` with `slots` alone and Phase 3 adds the field. Adding a placeholder
        field now would have meant a type with nothing to put in it.
    - |-
        Validating a document against an unresolved template returns the existing "could not prepare" failure
        (`ValidationFailure::Unprepared`) whose error is the typed `UnresolvedSlots`. The plan asked for the new
        `SimplifiedSchemaError::Unresolved` from validation too, but validation returns a `ValidationFailure`,
        not a `SimplifiedSchemaError`; adding a new failure variant would ripple through every host that
        matches on it. `to_json_schema`, `layer_under`, and `layer_over` do return `Unresolved`. Reverting
        means adding a `ValidationFailure::Unresolved` variant and updating its matches.
    - |-
        A Markdown document's `$schema`, read by the `SimplifiedSchema` factory, now reports suggestion and slot
        positions in the Markdown document (as a standalone schema file already did); before, a Markdown
        suggestion's position was relative to its type string. No production caller read those positions.
        The alternative was to project only slots, which would leave the two kinds of position in different
        coordinates. Reverting removes one call in `schema/layer.rs::from_frontmatter`.
    - |-
        When a named type is reused with a slot bound or default beside an inherited one (`Name(min({{ x }}))`
        over a type with `min(3)`), both are kept until resolution; the "stricter bound wins" and "defaults must
        agree" merge rules cannot be applied before the slot's value is known. Phase 3's substitution must
        re-apply them after it replaces the slot. The alternative, rejecting a slot beside an inherited bound,
        would forbid a reasonable schema. A slot bound on a reference that turns out to be a string is
        retargeted to a length bound; on a list it stays in the list's constraints.
    - |-
        The subtype check keeps returning yes/no. The plan asked for an "unresolved" result where a template can
        reach a subtype or coercion check; changing `is_subtype`'s return type is a wide public change, and the
        hosts that call it work on resolved schemas. Instead an `enum` or `literal` with a slot is a subtype of
        nothing but itself (its members are not known), which fails closed. This also fixed a hazard: two
        different template enums would otherwise have compared as the same type ("no members" each).
    - |-
        Suggestion queries and union completion gained a `pending` flag (`SuggestionQuery.pending`,
        `AlternativeCompletion.pending`) instead of a separate pending list: the literal elements are offered
        and the flag says the list is incomplete. Each slot itself is available from `pending_slots()` on the
        schema or the type. The structs are public with public fields, so a host building them by hand must
        now set the flag; none does today.
    - |-
        The lower-level `parse_yaml_schema_with_source` still cannot project positions through a `|` or `>`
        block scalar (it never could for suggestions); the factories and standalone loader, which every
        schema file goes through, can, and the block-carrier tests run there. Extending the lower-level scanner
        was left out as unrelated to slots.
    - |-
        The shipped-schema corpus exception for Claudine's sandboxing schema was removed in Phase 2, not Phase 7.
        Once slots parse, that schema passes its passive `kind: schema` admission, and the exception's own guard
        failed the suite with "remove this carve-out"; the spec says to remove it when admission works. The
        schema file itself still has its old `as_csv(...)` slot text: changing it to the `load_data` spelling
        (N40) and the resolved-validation compose test stay in Phase 7. Keeping the exception would have meant
        rewriting its guard to expect success, which is the same as deleting it.
source_files_during_phase_2:
    - simplified-schema/lib/src/completion.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/expression/scan.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/schema/error.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/layering.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/schema/tests.rs
    - simplified-schema/lib/src/schema/view.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/grammar/slot_tests.rs
    - simplified-schema/lib/src/simplified/lint.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/query.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/subtype.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/slots.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/expression_scanner.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_object.rs
    - simplified-schema/lib/tests/l1/slot_spans.rs
    - simplified-schema/lib/tests/l1/slot_templates.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
docs_updated_during_phase_2:
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/simplified-schema-struct.md
docs_created_during_phase_2:
    - simplified-schema/docs/topics/language/slots.md
skills_files_updated_during_phase_2:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
    Phase 2 is complete and green. What Phase 3 needs to know:
    - Templates parse to `Constraint::Dynamic(DynamicList)` / `Constraint::DynamicValue { kind, target, slot }`
      (simplified-schema/lib/src/simplified/types.rs). Static lists never do. `Slot.origin` is always `None`
      today: N12 origin stamping in `resolve.rs::expand_imports` is yours.
    - `simplified-schema/lib/src/slots.rs` already exists (collect_slots, has_dynamic, definition_slots,
      reachable_slots, reject_unresolved). Extend it with SlotKey/SlotValues/with_values/substitute rather than
      creating another module.
    - Every lowering refuses a template up front: `convert.rs::to_json_schema_with_definitions` calls
      `slots::reject_unresolved`, and `atom_fragment_without_null_wrap` rejects an atom holding a slot. So the
      substitution must run before lowering (after import expansion), exactly as N10 says.
    - `SchemaError::UnresolvedSlots(Box<UnresolvedSlots>)` has only `slots` (`#[non_exhaustive]`, built with
      `UnresolvedSlots::new`); add N10's `pending: Option<PendingSchema>` there.
    - Value-slot targets: `target` is the type keyword, `Literal` for `literal(...)`, and `Unknown` on a
      reference (except a reference reinterpreted as a string length bound, which becomes `String`). Whether a
      Min/Max is a count is decided by which list holds it: `array_constraints` means item counts; a `string`
      target means lengths; otherwise numeric. A slot bound/default beside an inherited one is NOT merged by
      `resolve.rs::merge_constraints`; substitution must re-apply the stricter-bound and equal-default rules.
    - In a dynamic suggest list, literal duplicates are kept (no grammar error); N7's first-occurrence dedupe is
      for substitution to do across literals and slot values.
    - `SimplifiedSchema` keeps a template layer with `lowered = Err(UnresolvedSlots)`; a whole-file `$schema`
      reference or root-union file arm that reaches a template is kept as a layer with `typed` = the authored
      root (no expansion) and the slots taken from the error (`schema/layer.rs::resolve_authored`).
      `pending_slots()` reads `Definition.pending`, computed in `Definition::new`.
    - Darkmatter compiles against all of this; the Darkmatter changes were a match arm in
      `triggers/finding.rs` and deleting the shipped-corpus `awaiting_enum_slots` carve-out in
      `lib/tests/l1/shipped_schema_corpus.rs` (the sandboxing schema now passes passive admission and the
      carve-out's guard demanded its removal). Phase 7 still owns the sandboxing.yaml `load_data` edit (N40)
      and its resolved-validation compose test; mark only that part of its task done.
    - Carried from Phase 1: the host sweep (Sun/Wed 04:00) can delete target/ mid-build, and an outside process
      may commit the working tree during the run.
---

# Implementation Log for 2026-11-05-dynamic-schema-types (9 phases)

## Baseline

Taken 2026-11-08 on macOS (Apple silicon), package-area recipes, serially.

| Area | `just test` | `just lint` |
| --- | --- | --- |
| `simplified-schema` | pass: 1550 (lib) + 1362 (`l1`), 0 skipped | pass |
| `sniff` | pass: 3545, 33 skipped | pass |
| `darkmatter` | pass: 15254, 10 skipped, 1 slow (see below) | pass |

- `simplified-schema` was measured before any Phase 1 edit; `sniff` and
  `darkmatter` after them (a host sweep corrupted the first attempt, see
  Phase 1). The Phase 1 edits to `darkmatter` add tests, fixtures, the N42
  completion fix, and two doc paragraphs, so any later red test outside
  those is attributable to later phases.
- Pre-existing, load-sensitive: `darkmatter-cli::l1
  compose_shell_template::every_suggested_rule_approves_the_command` took
  5.23 s in the full run (load average about 16) and 2.96 s alone.
- Pre-existing, environmental: the macOS linker warns
  `__eh_frame section too large` for `darkmatter-cli`; not a test or lint
  failure.

## Departures

The plan was reconciled on 2026-11-08 with the investigation outcomes and
Rulings Q8–Q20 (see the plan's "Reconciliation" section). The grant-store
location (now Q8) and the breadth staging (now Q16: value positions delivered,
whole types deferred) are rulings, no longer departures.

- **N35, mutation mechanism.** `DataRuntime::invalidate_memo` records no paths,
  so per-path invalidation of a pinned snapshot does not exist to reuse. The
  plan replaces it with a stricter check: a memo-generation counter plus a
  re-hash of the snapshot's recorded observations on each use after the counter
  moves. Grants decided earlier in the run are reused (Q17).

## Decisions applied

Phase rules N1–N42 (with N6a, N12a, N12b) are applied as written unless a later entry here says
otherwise.

## Measurements

None yet. Phase 9 measures the implementation once on one host against the S1
budgets (plan N1); the simulated baseline is in
`investigations/s1-s2-s3-s6-performance.md`.

## Phase 1

### What was done

- **Baseline** recorded above.
- **Fixtures.** Reviewed the first run's files against N4/N6.
  - `simplified-schema/lib/tests/fixtures/dynamic_slots/`: the list-position
    fixtures (`schema-with-slot.yaml`, `suggest-slot.yaml`,
    `imported-types.yaml` with `nested/`, `os.yaml`) match the plan and were
    kept. Added the Q16 value positions: `value-positions.yaml` (one
    whole-argument slot each in `literal`, `min`, `max`, `default`,
    `pattern`) and its data under `values/`; README table extended.
  - `darkmatter/lib/tests/fixtures/dynamic_slots/claudine/` (sandboxing
    shape, `os.yaml`, `provider.md`) was kept. The README named a builder,
    `tests/dynamic_slots_fixtures/mod.rs`, that did not exist; it is now
    written: `SlotRepo::build(source, RepoState)` copies the tree into a
    fresh `git init` repository (fixture identity, no host Git config) with
    the data file `claudine/docs/os.yaml` in one of `Clean`, `DirtyData`,
    `UntrackedData`, `IgnoredData` (committed `.gitignore`), or
    `NoRepository`. Declared in `tests/l1/main.rs`; its own tests
    (`l1/dynamic_slots_fixture_repos.rs`) prove each state with
    `git status --porcelain --ignored` and that the schema-relative slot
    path `../../../os.yaml` names the data file.
- **DMLS enum completion (N42, Q20).**
  - Cause: on a `- ` item line, `union_item_completions` handles only
    union/object item arrays and `block_array_suggestions` only
    `suggest(...)`; a flow `[…]` element consulted only `suggest(...)`.
    Nothing offered the members of a single-arm enum item type, named
    (`os[]`) or inline (`enum(...)[]`). A scalar `os` value already worked.
  - Fix (`dmls/src/providers/frontmatter.rs`): the scalar candidate logic
    of `value_completions` moved into `def_value_completions(def, …)`; a new
    `array_item_value_completions` builds the item definition with the
    existing `array_item_def(array_item_atoms(..))` and calls it. Block
    items (`block_array_suggestions`, renamed `block_array_item_completions`)
    and flow elements fall back to it when there is no `suggest(...)`.
  - Docs: `docs/topics/schemas/dmls-schema-support.md` (enum values bullet)
    and `dmls/docs/autocomplete.md` (new "Items of an array of one scalar
    type" section). Skill: `.claude/skills/darkmatter/dmls.md`.

### Requirement-to-test mapping

| Behavior | Test (boundary, tier) |
| --- | --- |
| Empty `- ` item of named `os[]` offers every member | `darkmatter-lsp providers::frontmatter::enum_item_tests::empty_block_item_of_named_enum_array_offers_every_member` (schema file on disk + overlay + completion provider, F2, L1) |
| Bare `-` item | `…::bare_dash_item_of_named_enum_array_offers_every_member` |
| Partial item filters and its text edit replaces exactly the partial | `…::partial_block_item_filters_members_and_replaces_the_partial` |
| Flow elements `[]`, `[macOS, ]`, `[macOS, W]` | `…::flow_array_elements_offer_members` |
| Inline `enum(fit, fill)[]` | `…::inline_enum_array_item_offers_members` |
| Control: scalar `os` value unchanged | `…::scalar_enum_value_keeps_offering_members` |
| Fixture states | `darkmatter::l1 dynamic_slots_fixture_repos::*` (5 tests, real `git`) |

Regression proof: with the two fallbacks reverted, 5 of the 6
`enum_item_tests` fail (each `left: []`) and the scalar control passes;
with the fix all 6 pass. Targeted commands:
`cargo nextest run -p darkmatter-lsp --lib -E 'test(enum_item_tests)'` and
`cargo nextest run -p darkmatter --test l1 -E 'test(dynamic_slots_fixture_repos) | test(test_layout)'`.

The Input Robustness Matrix does not apply to this phase: no loader or
parser was added (the two loaders it names arrive in Phases 3 and 5).

### Gates

- `just test` and `just lint` in `darkmatter/`, `sniff/`, `simplified-schema/`:
  all pass (counts in Baseline). `just check-tier-coverage darkmatter`:
  0 stranded.
- Not skipped: the scope calculator was not used to skip anything, since
  the phase changes compiled code.
- Dependents: no `darkmatter` library source changed (only its tests); the
  DMLS change is internal to the `darkmatter-lsp` binary crate, which no
  package depends on. No dependent tests were run.
- Cross-OS: not run. The DMLS change has no OS-specific code; the fixture
  builder uses `canonicalize_simplified`, `Path::join`, and Git's `/`
  porcelain paths, and sets `GIT_CONFIG_GLOBAL` to `NUL` on Windows. The
  nightly Windows/WSL2 cells prove it after merge.

### Incidents

- A scheduled host sweep (`~/.local/bin/rusty-biscuit-sweep.sh`, Sunday
  04:00, `cargo sweep --maxsize 120GB`) ran during the first baseline
  attempt and removed 505 GiB from this worktree's `target/` mid-build;
  `sniff`'s test build and lint failed with missing `build/*/output` and
  `.rmeta` files. The attempt was stopped, the sweep allowed to finish
  (not interrupted: it is not this session's process), and every gate
  rerun from scratch.
- While this phase ran, a process outside this session committed the
  working tree several times (`dffd6b0e8`, `2fe27e5ea`, `705fa8f55`,
  `4c8fa60cd`, by the author identity). This session staged and committed
  nothing; the committed content is this phase's work.

## Phase 2

### What was done

All in `simplified-schema` except one match arm in Darkmatter.

- **AST (N3).** `Slot`, `DynamicKind`, `DynamicElement`, `DynamicList`,
  `ValueSlotKind`, `Constraint::Dynamic`, `Constraint::DynamicValue` in
  `simplified/types.rs`, re-exported from `simplified` and the crate root.
  `Constraint::slots()`, `PropertyAtom::{suggestion_candidates,
  has_pending_suggestions, has_pending_values}`. `keyword()` for `Dynamic`
  is `"{{"` for a leading slot, else `"<members>"`/`"suggest"` (it returns
  `&'static str`, so "first element's authored text" is approximated).
- **Scanner (N5).** `expression::scan::scan_expression_at`, the one closing
  rule, used by the grammar (`read_slot`), the grammar's own
  `argument_list_end`/`find_top_level`, and `source.rs::scan_expression`
  (argument lists skip a slot whole, so `split_top_level`,
  `scan_constraint_groups`, `constraint_call_arguments` read it as one
  argument).
- **Grammar (N4, N7).** `enum(...)` members and `suggest(...)` arguments take
  a slot as a complete element; `literal`, `default`, `min`, `max`, `pattern`
  take a whole-argument slot. `Dynamic`/`DynamicValue` are built only when a
  slot is present. Positioned errors: complete element, whole argument,
  unterminated slot, empty slot, expression parse error (offset mapped into
  the expression). One-`suggest`-per-definition and the exact-type rule hold
  for dynamic lists; a dynamic suggest list keeps literal duplicates (N7
  dedupe happens at substitution). `null(default({{…}}))` and value slots on
  `type-definition`/`schema` are rejected.
- **Deferred references.** `reinterpret_dynamic_suggestions` retargets a
  reference's dynamic suggest list; resolve.rs's suggest checks use
  `is_suggestion_list`/`suggestion_list_span`.
- **Serialize.** Literals keep their quoting; slots are written back as
  `{{ expr }}`; export → parse → export is stable.
- **Spans.** `project_atom_suggestions` projects slot `span`/`expression_span`
  and dynamic candidate spans in both value and array lists. The Markdown
  frontmatter factory path now projects candidate and slot spans into the
  document (`project_keyed_declaration_spans`), as the standalone path did.
- **Lowering (N10, passive half).** `SchemaError::UnresolvedSlots(Box<
  UnresolvedSlots { slots }>)` (Display + terminal status block).
  `to_json_schema_with_definitions` rejects a template before lowering
  (`slots::reject_unresolved`); `atom_fragment_without_null_wrap` rejects an
  atom with slots, so any other route into lowering is closed too.
  New module `slots.rs`: `collect_slots`, `has_dynamic`, `definition_slots`,
  `reachable_slots`.
- **Object (N13, passive half).** `is_resolved()`, `pending_slots()`,
  `SimplifiedSchemaError::Unresolved`; `to_json_schema`, `layer_under`,
  `layer_over` return it; validation returns `ValidationFailure::Unprepared`
  holding `UnresolvedSlots`; a whole-file `$schema` reference or root-union
  file arm that reaches a template is kept as an unlowered layer.
  `TypeDefinition::pending_slots()`.
- **Consumers.** `SuggestionQuery.pending` and `AlternativeCompletion.pending`
  (literal elements offered, list flagged incomplete); lint judges only
  literal candidates, and none when a value slot shapes the target;
  `is_subtype` gives an unresolved enum/literal no identity (it was about to
  compare `enum({{a}})` and `enum({{b}})` equal as "no members");
  `format.rs` `forms(enum)` admits a template's literal members; narrowing
  markers; recursive-definition use sites with a slot are always inlined;
  `is_array_container_constraint` and `reinterpret_bounds` know value slots;
  `has_default` counts a slot default.
- **Darkmatter.** `triggers/finding.rs::related_path` classifies the new
  `SchemaError` variant (names an imported slot's defining file). Trigger
  match conditions already exclude slots (`is_match_safe_constraint` is an
  allow-list).
- **Docs.** New `simplified-schema/docs/topics/language/slots.md` (resolution
  marked **planned**); `language/index.md`, `simplified-schema-struct.md`,
  `diagnostics.md`, `language-core.md` updated. Skills:
  `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`.

### Requirement-to-test mapping

Boundary for all rows: the `simplified-schema` crate's public API, in-process
(F1 for the grammar unit tests, F2 for factory and file-backed tests over the
shipped fixtures in `tests/fixtures/dynamic_slots/`), tier L1.

| Behavior | Test |
| --- | --- |
| `scan_expression_at`: same end as the whole-text scan; quoted `}}`/`{{`, nested braces; unclosed, empty, `{{{`, literal token, wrong byte; escaped opener | `l1 expression_scanner::scan_expression_at_*` (4) + doctest |
| enum slot with literals and `; required`/`default` tail; mixed order; multiple slots; `||` inside a slot in a union and array | `lib simplified::grammar::slot_tests::{an_enum_slot_mixes…, slots_and_literals_alternate…, a_slot_inside_a_union…}` |
| static lists stay `Members`/`Suggest`/`LiteralValue` (static parity) | `slot_tests::a_static_list_never_builds_a_dynamic_constraint`, `l1 declaration_grammar::slots::a_static_list_keeps_its_static_constraint`, `schema_object::templates::a_static_schema_is_resolved_and_lowers_its_lists_as_before` |
| quoted `'{{ x }}'` stays literal; `abc{{x}}` keeps today's behavior; data-shaped literal beside a slot | `slot_tests::a_brace_that_does_not_open_an_element_is_text`, `declaration_grammar::slots::data_shaped_text_beside_a_slot_stays_literal` |
| complete-element, unterminated, empty, parse-error-offset errors | `slot_tests::{a_slot_must_be_a_complete_list_element, an_unterminated_or_empty_slot…, a_slot_expression_parse_error…}`, `declaration_grammar::slots::slot_errors_are_located_in_the_expression` |
| suggest slots: target, deferred reference, duplicates kept only when dynamic, one list per definition, exact-type rule | `slot_tests::{a_suggestion_slot_keeps…, duplicate_literals…, a_definition_still_holds_one…, suggestion_slots_follow_the_exact_type_rule}` |
| value positions: literal/min/max/default/pattern, array-level, reference target; whole-argument errors; `null` default, `type-definition` | `slot_tests::{value_positions_take…, a_value_slot_must_be_the_whole_argument, value_slots_keep_the_type_rules…}` |
| serialize round trip, literal quoting kept | `declaration_grammar::slots::templates_round_trip…`, `schema_object::templates::{export_keeps_every_slot…, export_keeps_literal_quoting…}` (all three list fixtures + value-positions fixture) |
| spans through plain, `'…'`, `"…"`, `\|`, `>`, CRLF, frontmatter offset, Markdown factory, imported defining file; slot as one argument in the source map | `l1 slot_spans::*` (8) |
| factories admit templates; `is_resolved`/`pending_slots`; `to_json_schema` → `Unresolved`; validation → `Unprepared(UnresolvedSlots)`; layering either side fails atomically | `schema_object::templates::*` (7), `slot_templates::validating_against_a_template_never_reports_success` |
| query/suggestion/lint/completion/subtype on templates | `l1 slot_templates::*` (6) |
| parse, query, export evaluate nothing and read nothing (counters + missing data files + provenance) | `lib schema::tests::context_boundary::a_template_reads_and_evaluates_nothing_but_its_schema_files` |

The Input Robustness Matrix does not apply: the change is to the
type-expression text grammar; no file-format or configuration loader was
added or changed. The loader code-smell grep over the new code found one
`.ok()?` on `str::from_utf8` of a `&str`'s bytes (cannot fail; not a
load-bearing parse).

### Gates

Run on macOS after all edits, with no subagent running (this phase used none).

- `simplified-schema`: `just test` pass (1596 with `work-counters,terminal`;
  1406 in the no-`file` minimal build), `just lint` pass (clippy with
  features and `--no-default-features`), doctests pass (41, including the
  new `scan_expression_at` and `is_resolved` examples),
  `just check-tier-coverage simplified-schema`: 0 stranded.
- `darkmatter`: `just test` pass (15254 passed, 10 skipped), `just lint`
  pass, `just check-tier-coverage darkmatter`: 0 stranded. On the first full
  run `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry`
  failed on its own carve-out guard ("…validates now…; remove this
  carve-out"); the carve-out was removed (see decisions) and the rerun was green.
  `darkmatter-cli::l1 compose_shell_template::every_suggested_rule_approves_the_command`
  ran 5.08 s in one full run: the pre-existing load-sensitive test from the
  baseline, not touched here, and under 5 s on the final run.
- Dependents (compile): `cargo check --all-targets -p darkmatter
  -p darkmatter-cli -p darkmatter-lsp -p claudine -p claudine-cli -p messenger`
  passes.
- Dependents (targeted tests, serially; each package has more than 200
  tests, so only tests reading schemas through the changed symbols ran):
  `claudine` `test(composition::schema) | test(claudine_schemas) |
  test(container_schema_recursion) | test(host_catalog)` 999 passed;
  `claudine-cli` `test(schema_completion)` 53 passed; `claudine-gen`
  `test(schema_compat) | test(model_config_research)` 3 passed;
  `messenger` `test(research)` 1 passed.
- Gate skipping: not used; this phase changed compiled code, so
  `affected_scope.py` was not consulted to skip anything.
- Cross-OS: not run. The change adds no path or OS-specific code; the one
  file-backed test joins `CARGO_MANIFEST_DIR` with `Path::join`, and the CRLF
  span case runs on every OS. The nightly Windows/WSL2 cells cover it.
