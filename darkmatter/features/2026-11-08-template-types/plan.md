---
total_phases: 8
created: 2026-11-09
phase: 4
agent: claude/sonnet
yolo: true
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- darkmatter-lsp
- claudine
- claudine-cli
- claudine-gen
source_files_during_phase_1:
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/view.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/grammar/slot_tests.rs
- simplified-schema/lib/src/simplified/lint.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/narrowing.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-constraints.yaml
- simplified-schema/lib/tests/l1/declaration_grammar.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/schema_object.rs
- darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
- darkmatter/lib/tests/l1/dynamic_schema_slots.rs
- darkmatter/cli/tests/l1/hash_kind_save_diff.rs
- claudine/cli/src/argv/partition.rs
- claudine/cli/src/argv/partition/tests.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
- claudine/cli/src/completion/schema_completion/candidates.rs
- claudine/cli/src/completion/schema_completion/tests.rs
- claudine/cli/src/completion/setter_value.rs
- claudine/cli/tests/common/owned_value_options.rs
- claudine/cli/tests/l1/completion_cli.rs
- claudine/cli/tests/l1/completion_ownership.rs
- claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
source_files_during_phase_2:
- claudine/gen/src/schema_compat.rs
- claudine/lib/src/composition/schema/classify.rs
- darkmatter/cli/src/commands/schema/assignment.rs
- darkmatter/dmls/src/overlay/named_types.rs
- darkmatter/dmls/src/overlay/suggestions.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/lib/src/lifecycle/schema_projection.rs
- darkmatter/lib/src/markdown/compose/context/catalog.rs
- darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/data/contract.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/frontmatter.rs
- darkmatter/lib/src/markdown/schemas/completion.rs
- darkmatter/lib/src/markdown/schemas/detect.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
- simplified-schema/lib/src/completion.rs
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/file_match.rs
- simplified-schema/lib/src/format.rs
- simplified-schema/lib/src/frontmatter_shape.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/phase.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/graph.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/schema/view.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/lint.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/narrowing.rs
- simplified-schema/lib/src/simplified/query.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/simplified/subtype.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/src/slots/unquoted.rs
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-types/properties.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-types/status-type.yaml
- simplified-schema/lib/tests/l1/quote_the_slot.rs
- simplified-schema/lib/tests/l1/template_types_passive.rs
source_files_during_phase_3:
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/host.rs
- simplified-schema/lib/src/slots/pending.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/src/slots/reads.rs
- simplified-schema/lib/src/slots/template_type.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/select.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/query.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/template_types_passive.rs
- simplified-schema/lib/tests/l1/template_type_results.rs
- simplified-schema/lib/tests/l1/template_type_imports.rs
- simplified-schema/lib/tests/l1/template_type_refusals.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
source_files_during_phase_4:
- darkmatter/lib/src/markdown/schemas/slots/host.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/src/markdown/schemas/slots/codes.rs
- darkmatter/lib/src/markdown/schemas/slots/cache.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
- darkmatter/lib/src/markdown/schemas/clean.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
source_files_during_phase_5:
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/commands/schema/assignment.rs
source_files_during_phase_6:
- darkmatter/dmls/src/slot_jobs.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/overlay/suggestions.rs
- darkmatter/dmls/src/diagnostics/slots.rs
source_files_during_phase_7:
- claudine/cli/src/completion/schema_completion/mod.rs
- claudine/cli/src/completion/schema_completion/keys.rs
- claudine/cli/src/completion/schema_completion/candidates.rs
- claudine/cli/src/completion/engine/mod.rs
docs_updated_during_phase_8:
- simplified-schema/docs/topics/language/slots.md
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/dmls/docs/autocomplete.md
- darkmatter/dmls/docs/hover.md
- claudine/docs/topics/completions/shell-completions.md
skills_files_updated_during_phase_8:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/cli.md
docs_updated_during_phase_1:
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- claudine/docs/topics/completions/shell-completions.md
- claudine/docs/topics/composition.md
- claudine/docs/topics/execution-flow.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
docs_updated_during_phase_2:
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/SKILL.md
docs_updated_during_phase_3:
- simplified-schema/docs/topics/language/slots.md
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
---

# Template types — implementation plan

Spec: `2026-11-08-template-types` (`spec.md` beside this file). Run every phase
in order; a phase ends only when its validation checkpoint passes. The whole
session tree is non-interactive: no phase has a human gate. Work a human will
want to judge is built, then listed in the implementation log for the review
that follows the final phase.

## Summary and Definition of Done

### What the work is

Today a `{{ … }}` slot can fill a *constraint* of a type the author wrote
(`enum(...)`, `default(...)`, …). This feature adds the second kind, the
**template type**: a slot that supplies a property's **whole type definition**
(`status: "{{ load_data('./status-type.yaml', 'type-definition') }}"`), with
the same optional suffix an imported named type accepts (`[]`, `(required)`,
`-> description`).

The work crosses four package areas:

| Area | What changes |
| --- | --- |
| `simplified-schema` | grammar, a new `TypeExpr::Slot` atom, every reader of the atom, the refused-position errors, the host boundary (`SlotHost` gains read hooks), substitution of returned definitions with import expansion, suffix merge, nested-slot / reachable-slot / pattern / `@this` checks |
| `darkmatter` (lib, cli, dmls) | host implementation of the read hooks, grants, cache and dependency inputs, trigger payloads, pending handling in compose/clean/phase/reference checks, `md schema validate <schema file>` now resolving, editor pending views and worker |
| `claudine` (lib, cli) | shell completion resolves a templated property's values on a value request with non-prompting grants |
| docs and skills | rename "value slot" / "whole-type slot" to **template constraint** / **template type** everywhere, and document the new behavior |

### Definition of done

1. A template type resolves to **the same schema and validation behavior as
   the equivalent static definition** (proved by paired tests: the same
   document judged against a slot schema and against the static schema
   gives identical reports, JSON Schema, and effective-schema queries).
2. Passive operations stay passive: parse, query, export, completion, lint
   and DMLS views evaluate nothing and read nothing. A template type reports a
   typed pending state; it is never `unknown`, an empty object, an optional
   property, or a missing declaration.
3. Every read introduced by a returned definition (imports, transitive
   imports, examples, path probes) is checked against the originating slot's
   grant **before** the file is opened, a buffer is used, or a memo is
   consulted, and is recorded as a dependency on success **and** failure.
4. `md schema validate <schema file>` resolves every slot (template
   constraints and template types) through the same grant gate as document
   mode; a refused or failed resolution exits 1; the `slots` count is gone.
5. Claudine completion: names never evaluate; a value request on a property
   holding a slot resolves it with non-prompting grants; ungranted or failed
   resolution offers nothing and prints nothing; static properties complete
   without evaluation.
6. The old terms ("value slot", "value-position slot", "value position" in the
   slot sense, "whole-type slot", `ValueSlotKind`, `Constraint::DynamicValue`)
   appear nowhere outside `_completed` specs (checked by the command in Phase 1
   and again in Phase 8).
7. `just test` and `just lint` pass in `simplified-schema`, `darkmatter`, and
   `claudine`, plus `just test-minimal` in `simplified-schema`. Nothing in the
   change is OS-specific; paths are built with `Path`/`PathBuf` and compared
   through the existing canonicalization helpers, so macOS, Linux, native
   Windows and WSL2 compile as before (load the `os` skill before touching
   any path comparison).

### Phase map

| Phase | Name | Waves |
| --- | --- | --- |
| 1 | Rulings and terminology rename | 1–2 |
| 2 | Grammar, `TypeExpr::Slot`, and passive readers (simplified-schema + downstream compile) | 3–5 |
| 3 | Resolution engine (host read hooks, substitution, checks) | 6–8 |
| 4 | Darkmatter library: host, grants, cache, triggers, pending surfaces | 9–11 |
| 5 | `md` CLI: schema validate and assignments | 12 |
| 6 | DMLS editor | 13 |
| 7 | Claudine shell completion | 14 |
| 8 | Docs, skills, and final verification | 15–16 |

Skills to load before starting: `darkmatter`, `simplified-schema`, `rust`,
`rust-testing`, `os`, `just`. Claudine phase also loads `claudine`. Prefer
`gitnexus` `impact`/`context` for blast-radius questions, never as a gate.

## Phase 1 — Rulings and terminology rename

Records every settled decision (below), then performs the mechanical rename so
that every later phase writes the new names only.

### Necessary Rules

Each rule is a decision followed by its evidence. They bind every later phase.

**Names and shapes**

- **R1 — Rename targets.** `ValueSlotKind` → `TemplateConstraintKind`;
  `Constraint::DynamicValue { kind, target, slot }` →
  `Constraint::TemplateConstraint { kind, target, slot }`; fixture
  `value-positions.yaml` → `template-constraints.yaml`; `slot_tests` test
  names containing `value_slot`/`value_position` → `template_constraint…`.
  `Constraint::Dynamic`, `DynamicList`, `DynamicKind`, `DynamicElement`, and
  `PendingSchema`/`Template` keep their names: they are not banned terms and
  describe list-shaped constraints, which the docs call template constraint
  lists. Evidence: spec "Terminology rename" lists `ValueSlotKind` and
  `Constraint::DynamicValue` only; `rg` shows the type at
  `simplified-schema/lib/src/simplified/types.rs:1248` and 14 consuming files
  (`slots.rs`, `slots/substitute.rs`, `resolve.rs`, `resolve/recursive.rs`,
  `grammar.rs`, `narrowing.rs`, `serialize.rs`, `source.rs`, `lint.rs`,
  `mod.rs`, `lib.rs`, tests `schema_object.rs`, `declaration_grammar.rs`,
  `dynamic_slots.rs`).
- **R2 — Claudine's unrelated wording.** "value slot" meaning the cursor
  position after `=` becomes **setter value** in:
  `claudine/cli/src/completion/schema_completion/candidates.rs:3`,
  `claudine/cli/src/completion/engine/tests.rs:315,317`,
  `claudine/cli/src/argv/partition.rs:27`,
  `claudine/cli/tests/common/owned_value_options.rs:7`,
  `claudine/cli/tests/l1/completion_ownership.rs:55,294,298`,
  `claudine/cli/tests/l1/completion_cli.rs:245,262,279`,
  `claudine/docs/topics/completions/shell-completions.md:459,510,739,924`,
  `claudine/docs/topics/composition.md:327`,
  `claudine/docs/topics/execution-flow.md:82`. The module
  `completion/setter_value.rs` already uses the target name.
  Historical text under `claudine/**/_completed/**` and
  `darkmatter/**/_completed/**` is never edited.
- **R3 — New public names.** `TypeExpr::Slot(Slot)` (the atom);
  `PropertyAtom::is_pending_type()`; `PropertyDef::has_pending_type()`;
  `enum Requiredness { Required, Optional, Pending }` returned by
  `PropertyDef::requiredness()`; `SlotPosition` (parser-internal);
  `slots::TemplateReads`, `slots::SchemaRead`, `slots::SchemaReadKind`,
  `slots::SchemaReadOutcome`; `SchemaError::SlotReadRefused`. A pending type
  is represented **by the `TypeExpr::Slot` atom itself**, not by a side table:
  every reader that matches `TypeExpr` is forced to answer, which is the
  spec's "explicit pending answer". Evidence: `PropertyDef::is_required`
  (`types.rs:160`) documents "expects a **resolved** definition"; spec
  "Pending types" forbids mapping to `unknown`.
- **R4 — Slot errors carry location and property.**
  `SchemaError::SlotResult` gains `property: Option<String>` (the property
  path) and `related: Vec<SlotRelated>` (`SlotRelated { message, file:
  Option<PathBuf>, span: Option<SourceSpan> }`); the existing constructors in
  `slots/substitute.rs` pass `None`/empty. New `SlotResultKind` variants (the
  enum is already `#[non_exhaustive]`, `errors.rs:231`):
  `NotATypeDefinition(Box<SchemaError>)` (wrong shape of the returned value or
  an ordinary definition error from `parse_type_definition_value`),
  `NestedSlot`, `ReachableSlot`, `LocalReference`, `SuffixConflict`,
  `InvalidExpansion`. Evidence: spec "Diagnostics" requires the slot error
  structure and typed causes to be kept, names of the originating slot and
  property path, and related information for an imported file and for the
  suffix.

**Grammar and positions**

- **R5 — Parsing.** In `Parser::parse_type_expr`
  (`grammar.rs:1885`), an atom that begins with `{{` is read with the existing
  `Parser::read_slot` (`grammar.rs:3095`, which already runs the balanced
  expression scanner `scan_expression_at`, rejects `{{ }}` and an unterminated
  slot, and parses the expression once). Its suffix is read by the same
  code as `parse_named_reference` (`grammar.rs:2147`): an optional
  `( … )` list parsed with `ListOwner::Reference`, an optional `[]` with an
  optional second `( … )`. The result is
  `PropertyAtom { ty: TypeExpr::Slot(slot), is_array, constraints,
  array_constraints, description: None }`; `-> description` and `|` are
  handled by the existing `parse_definition`. After the suffix, a byte that is
  an identifier character is the error "part of a type name" (R6). No new
  `}}` detector: a slot's end is `slot_end` (`grammar.rs:448`).
- **R6 — Refused positions.** `Parser` gains a field
  `slot_position: SlotPosition` set by the construct being parsed:
  `PropertyType` (default; includes nested object properties, named type
  bodies, and property-level union arms), `TupleElement` (allowed),
  `TupleSpread`, `FunctionParameter`, `FunctionReturn`,
  `PropertyDeclaration` (the `property(...)` argument, `grammar.rs:3067`):
  the last four refuse. A shared builder
  `slot_position_error(position, span)` produces the single message
  shape: it names the position (for example "a `function(...)` parameter"),
  says "template types are not allowed there", lists the allowed positions
  ("a property's type, including nested properties and named type bodies; a
  property-level union arm; a tuple element, with an optional suffix such as
  `[]` or `(required)`"), and appends the rewrite sentence: for a **property
  key, pattern key, root, an import path, or part of a type name** the
  rewrite is "write the name literally; to choose a type from data, give the
  property a template type: `name: \"{{ … }}\"`"; for **tuple spread,
  function parameter/return, `property(...)`** there is no rewrite sentence
  (no permitted equivalent exists). Non-grammar positions are detected where
  the text is first read: root `$schema` string scalar and root-union string
  arm in `parse_schema_declaration` (`simplified/mod.rs:100`); a property key
  whose trimmed text starts a balanced `{{ … }}` in `parse_schema_shape`
  (`mod.rs:307`); `{{` inside the text after `@` in `read_fileref`
  (`grammar.rs:2197`). Keys that merely contain brace text without a
  balanced leading slot stay literal. Evidence: spec table "Grammar and
  result contract" and "Diagnostics"; the six grammar entry points all call
  `parse_type_expr` through `parse_union_arms`.
- **R7 — "Quote the slot".** A new function
  `slots::unquoted_slot_hint(text: &str, error_line: Option<usize>) ->
  Option<SourceSpan>` scans the YAML text for a mapping value or list item
  whose first non-space bytes after `key:` / `- ` are `{{` outside quotes,
  comments, and block scalars, and returns the span of that value. It is
  called (a) where `parse_schema_shape` reports "property names must be
  strings" for a flow-mapping-in-flow-mapping value and the located source
  text shows an unquoted `{{` there, and (b) where a schema file or
  frontmatter fails YAML decoding (the decode error's line is passed as
  `error_line`; a hint is returned only when that line, or the line before it,
  matches). The replaced message is "quote the slot: write
  `key: \"{{ … }}\"`". A decode error with no matching line keeps its
  original error. Evidence: spec "Grammar" requires the hint "including when
  YAML decoding fails" and forbids labeling an unrelated mapping error; the
  mapping-key error is `mod.rs:309`. Sites for (b): the schema-file loader
  (`parse_standalone_schema_document`, `simplified/standalone.rs`) and
  Darkmatter's frontmatter decode failure (grep `serde_yaml_ng::from_str` in
  `darkmatter/lib/src/markdown/frontmatter*`).

**Result contract and substitution**

- **R8 — Result shapes.** A slot value reaches
  `simplified_schema::parse_type_definition_value(property, &yaml)`
  (`simplified/mod.rs:262`), after conversion JSON → `serde_yaml_ng::Value`
  (string → String; array → Sequence with each element converted the same
  way). JSON `null`, a mapping, a number, and a boolean are refused **before**
  the parser as `SlotResultKind::NotATypeDefinition` with
  `SchemaError::SlotResult::WrongShape`-style text ("a type definition: a
  string or an array"); the string `"null"` is the null type. Anything the
  parser rejects (empty array, `""`, malformed text, a numeric array element)
  is wrapped unchanged in `NotATypeDefinition`. Nothing is stringified or
  coerced. Evidence: spec "Grammar and result contract"; the parser already
  handles the string-encoded list form.
- **R9 — Where substitution happens.** In `slots::apply_active`
  (`slots/substitute.rs`), called at the lowering sites after import
  expansion: `resolve.rs:501` (`resolve_inline_mapping`), `resolve.rs:1445`
  (`resolve_standalone_schema`), and the arm sites `resolve.rs:785` and
  `resolve.rs:1962`. Type slots are substituted **first**, then the existing
  constraint substitution runs over the resulting tree (so template
  constraints in a use-site suffix, which survive the merge, resolve normally).
  `apply_active`/`apply_active_to_arm` gain a parameter
  `site: &ExpansionSite { base_dir, schema_roots, request_context, shipped }`
  (every call site already holds these) and return
  `LateDependencies { referenced_files: Vec<PathBuf>, searched:
  BTreeSet<PathBuf> }`, which the call sites merge into their
  `ResolvedSchema.referenced_files` / `searched`. First-pass expansion
  (`expand_atom`, `discover`) passes a `TypeExpr::Slot` through untouched, so a
  template stays a valid, passive schema.
- **R10 — Expansion of a returned definition reuses the import engine.**
  For one slot occurrence the substituter builds a synthetic root
  `SchemaRoot::Single` with one property whose atom is
  `TypeExpr::Imported { name: "__template_type", reference: "this" }`, and a
  local type table holding the parsed returned definition under that name,
  then calls the existing `expand_imports` (`resolve.rs:2517`) with:
  `base_dir` = the slot's defining directory (`slot.origin.parent()`, else
  `site.base_dir`), `key` = `DefNamespace::File(origin)` when the slot has an
  origin, else `DefNamespace::InlineRoot`, and the site's roots/context. The
  engine's own discovery, cycle, shared-definition and
  `apply_postfix` (named-type merge) logic therefore applies unchanged, the
  suffix is merged by the existing rules and never concatenated into text,
  `check_non_null_use` re-checks `non-null`, and conflicting defaults or
  constraints fail as static imports do. Stored definitions the expansion
  publishes are added to the shared `DefinitionTable`; to avoid borrowing it
  while iterating, `substitute` first takes the definition bodies out
  (`std::mem::take`), rewrites them and the root, then puts them back.
  Each occurrence runs **two** expansions: first with the bare synthetic atom
  (no suffix) so the checks of R12 see only what the returned data introduced,
  then with the written suffix applied. Evidence: spec "Parse the returned
  definition once … apply the written suffix through the existing named-type
  merge rules"; `expand_import`/`inline_base`/`apply_postfix` at
  `resolve.rs:2839–2942`; discovery at `resolve/graph.rs:106` cannot see a
  returned definition, so a nested `expand_imports` run is the only way its
  recursive types are admitted.
- **R11 — Invariants after substitution.** The union checks now inlined in
  `parse_property_definition_at_depth` (`simplified/mod.rs:560–620`: at most
  one `suggest`, duplicate `function` overloads, `overload_arms_problem`,
  `non_null_problem`) move into
  `PropertyDef::check_union_invariants(property) -> Result<(), SchemaError>`;
  the parser and the substituter both call it, and the substituter calls it on
  every property definition that contained a replaced atom. Tuple-element
  checks that read the element's type (every `self.err` in `parse_tuple`,
  `grammar.rs:995–1100`, that inspects `arms`/`atom`; the grouping and
  optional-order rules stay purely syntactic) move into
  `TupleType::check_element(&PropertyDef)`, called at parse for concrete atoms
  (skipped for a `Slot`) and again after substitution. A returned union
  flattens into the surrounding union through the existing
  `ExpandedDef::Union` arm of `expand_union_arms` (`resolve.rs:2661`); an
  array suffix makes `TypeExpr::Union` with `is_array`. Presence follows
  `PropertyDef::is_required` (`types.rs:160`) unchanged after substitution:
  `"null"`, `"unknown"`, and a union with a null-admitting arm stay optional
  under `(required)`.
- **R12 — Checks on the bare expansion.** Run in this order on the first
  (bare) expansion; each failure is `SchemaError::SlotResult` at the
  originating slot with `property` set:
  1. before expansion, on the parsed definition: any slot from
     `slots::definition_slots` → `NestedSlot` (a template constraint nested
     in the returned text included); any `TypeExpr::Imported` with
     `reference == "this"` (the `LOCAL_REFERENCE`, which covers bare `Name`
     and `Name@this`) → `LocalReference`;
  2. after expansion: any slot in the expanded definition or in a stored
     definition it published (`slots::collect_slots` over the result) →
     `ReachableSlot` with `related` naming the imported file and the slot's
     span when `PendingSchema::slot_text` proves it. Only definitions the
     expansion actually reached are walked, so an unused slot elsewhere in an
     imported file never disqualifies it;
  3. every `Constraint::Pattern` in the expanded definition and in those
     stored definitions passes `slots::check_slot_pattern`
     (`slots/substitute.rs`, 4096 bytes, no lookaround or backreference,
     linear-engine compile) → else `PatternRejected`.
  Evidence: spec "Decision: reject slots reachable through a returned
  definition's imports" and "All patterns introduced by a returned
  definition".
- **R13 — Import bases and nesting.** A returned definition's imports resolve
  against the slot's defining schema file (R10 `base_dir`); the data file is
  never the base. A file loaded by a returned import expands with its own
  base through the unchanged engine. Evidence: spec "Imports, reads, and
  resolution"; `Slot.origin` doc (`types.rs:1192`).

**The read boundary (simplified-schema ↔ host)**

- **R14 — New host hooks.** `SlotHost` gains one **required** method
  `fn template_reads(&self) -> Rc<dyn TemplateReads>`, where
  `trait TemplateReads` is `'static` and has
  `fn admit(&self, slot: &Slot, read: &SchemaRead<'_>) -> Result<(),
  SlotAdmissionError>` and
  `fn observe(&self, slot: &Slot, outcome: &SchemaReadOutcome<'_>)`;
  `SchemaRead { path: &Path, kind: SchemaReadKind }` with
  `SchemaReadKind::{Content, Probe}`;
  `SchemaReadOutcome { path, kind, bytes: Option<&[u8]>, exists: bool }`.
  The host returns an `Rc` of owned handles (the Darkmatter host clones its
  `Arc`s), so no lifetime is erased and no `unsafe` is needed (the crate has
  no unsafe today). `PendingSchema::resolve` installs it in a thread-local
  scope (`slots::with_template_reads`) for the replay, exactly as
  `with_values` scopes values. `SlotHost` has no install base: the
  test hosts in `simplified-schema/lib/tests/l1/dynamic_slots.rs`, the docs
  example in `docs/topics/language/slots.md`, and `DarkmatterSlotHost`
  (`darkmatter/lib/src/markdown/schemas/slots/host.rs:65`) are updated in the
  same phase that adds the method. `buffers::guarded`/`ReadGuard`
  (path-predicate, `buffers.rs:104`) is unchanged: it carries no refusal code
  and cannot gate a probe, so it cannot serve this contract.
- **R15 — Where the engine asks.** Only while expanding for a template type
  (an `ImportEngine` field `template_slot: Option<Slot>` set by the
  substituter). `admit(Probe)` is asked for **every planned candidate
  before anything is probed**: candidates come from
  `FileReference::candidate_plan_with_order(.., Resolution)`
  (already used by `first_planned_candidate`, `select.rs:215`), for a bare
  name once per schema root as the explicit `./name` it is probed as
  (`select_in_roots`, `select.rs:150`). `admit(Content)` is asked for the
  selected path in `ImportEngine::resolve_namespace` **before**
  `namespace_cache`, before `load_named_types`/the process memo
  (`resolve.rs:3332`), and before `buffers::read`. For an `example(...)`
  reached by the returned definition (`rehome_atom_examples`,
  `resolve.rs:3245`), each planned candidate is a `Probe` and the resolved
  path a `Content` ask at rehome time (the later read at
  `resolve.rs:4486` is the same path). After each read or probe the engine
  calls `observe` (bytes on success, `None` on failure, `exists` for a
  probe), **including when the read failed or the candidate is missing**.
  Evidence: probes happen inside `biscuit-file`'s `resolve_detailed`
  (`select.rs:136`), so a post-hoc check could not gate them.
- **R16 — A refusal is sticky and typed.** The first refusal from `admit` is
  recorded in the thread-local scope and returned as
  `SchemaError::SlotReadRefused { slot, refusal: SlotAdmissionError }`. The
  engine's discovery swallows resolution errors (`graph.rs:239`, `let Ok(target)
  = … else return`), so the substituter checks the scope for a recorded
  refusal **after every nested expansion** and returns it even if the
  expansion otherwise succeeded. In `PendingSchema::resolve`, the attempt
  closure maps `SlotReadRefused` to a new `Attempt::Refused(Vec<(Slot,
  SlotAdmissionError)>)`, which `resolve_with` turns into
  `SlotResolutionError::Refused`, so the refusal keeps its code and can never
  become a successful schema. Cycles stay `SchemaError::ImportCycle`, grants
  stay `Refused`, and everything else stays `SchemaError::*`: the three are
  distinguishable by variant, which is how Darkmatter already maps codes
  (`slots/host.rs:nested_resolution_code`).
- **R17 — Reads are shared and retained.** `PendingSchema::replay_with` runs
  under `buffers::with_reads_retained` (`buffers.rs:35`) so a shared import is
  read once per resolution, not once per property. The exact texts already in
  `PendingSchema::read_sources()` are served by the existing `buffers::with`
  snapshot; files first met after evaluation are read through the same
  `buffers::read` (the request's source-buffer reader), and `observe` hands the
  host the bytes actually read. Evidence: `pending.rs:replay_with`; the
  `work-counters` feature's `observed::Observed::opened` proves read-once in a
  test.

**Darkmatter**

- **R18 — Host implementation.** `DarkmatterSlotHost::template_reads`
  returns an owned `DarkmatterTemplateReads { gates: Arc<dyn SlotGates>, data:
  DataRuntime }`. `admit` builds
  `ReadTarget { function: "schema import", kind: Content|Probe, location:
  ReadLocation::Local(path), observed_hash: None }`, takes the gate with
  `gates.gate_for(defining)` where `defining = slot.origin` or the request
  context's `base_dir()` (the same expression as `host.rs` `resolve`), and
  maps `GateRefusal::Denied` → `SLOT_DENIED_CODE` and
  `ApprovalRequired` → `SLOT_APPROVAL_REQUIRED_CODE` in a
  `SlotAdmissionError`. The session already wraps gates in `RecordingGates`
  (`session.rs:660`), so an allowed read is recorded by
  `dependencies::note_read` with the defining file, which is what
  `SlotSession::serves` (`session.rs:520`) checks before rereading a cached
  input. `observe` calls `data.observe(Observation { entry, kind: File|Exists|
  MissingProbe, hash: blake3 of the bytes })` (hash with `biscuit-hash`),
  which `capture_observations` (`session.rs:459`) turns into the cache's
  `InputCheck`s (size/mtime recheck policy and its accepted same-size/
  same-mtime limitation are unchanged), replays on a warm hit
  (`session.rs:430`), and feeds DMLS watches (`slot_jobs.rs:474`). The gate
  prompts through the existing handler for a computed target and returns
  `ApprovalRequired` where there is none (data output, `ReferenceCheck`, DMLS
  jobs): "keep existing grant-only behavior" holds without new code.
  Pre-flight (`approve_reads`) is **not** extended: an import named by returned
  data is a computed target, asked at resolution time.
- **R19 — Cache key and failure.** The cache key
  (`cache.rs:resolution_key`) is unchanged: the first pass's
  `read_sources()`, slots, context, repository, home. Late imports are covered
  by the stored `InputCheck`s, so a changed import is a miss through
  `serves`. A denied or missing import is cached like any other cacheable
  failure (`caches_failure`) **except** `approval_required` and `denied`,
  which are already grant-dependent (`needs_grant`) and re-judged per
  session. Cache and observations are written on success and failure because
  `observe` runs before the outcome is known.
- **R20 — Triggers.** No Darkmatter change is needed for layering: a trigger
  payload names its property, `declared_by_payload` /
  `discarded_by_higher_layers` (`schemas/mod.rs:1109–1209`) work from names,
  and `PendingSchema::without_properties` removes a discarded property's
  slots before resolution. Library work is only that `omit_properties` and
  `def_refs`/`atom_refs` in `slots.rs` understand `TypeExpr::Slot` (Phase 2),
  so a replaced property's template type never evaluates and requests no read.
  Slots in trigger `match` conditions stay refused
  (`triggers/grammar.rs:493`). Root-level slots are refused by R6, so the
  decision is possible without evaluation.
- **R21 — Prohibited contexts stay prohibited without new code.**
  Kind definitions, the baseline, global catalogs, the editor extension layer
  and `kind: <yaml document kind>` definitions refuse through
  `collect_slots`-based checks (`codes.rs:slot_prohibition_code`,
  `refuse_slots_in_baseline`, `catalog.rs:1122`, `yaml_documents/select.rs:86`,
  `semantic.rs:159`). Because `collect_slots` (R25) reports a
  `TypeExpr::Slot`, a template type there is refused with the same code.
  Reaching a template type *through a static import* from these contexts is
  already covered by the same walk.
- **R22 — Pending in Darkmatter surfaces.** Compose, `md clean`, and phase
  projection already fail with `UnresolvedSlots` until the effective template
  is resolved (`compose/schema_validation.rs:810 defers_template`,
  `schemas/clean.rs`), because every lowering entry point runs
  `reject_unresolved`. The only changes are the Phase 4 tests proving a
  template type follows these paths, and that `md graph` /
  `md validate refs` (`SessionMode::ReferenceCheck`) leave a template type
  unresolved and deferred. No substitute type is ever used.
- **R23 — `md schema validate <schema file>`.** New library method
  `DarkmatterSchemas::resolve_schema_file(path: &Path, text: &str) ->
  Result<ResolvedSchema, SchemaError>` in `schemas/yaml_documents/mod.rs`:
  `simplified_schema::resolve::prepare_yaml_schema_text`
  (`resolve.rs:1602`); on `UnresolvedSlots { pending: Some }` it resolves
  through the instance's slot session with `resolve_template`
  (`schemas/mod.rs:498`). In `validate_yaml` (`validate.rs:647`), for a file
  that `parse_standalone_schema_document` recognizes, the CLI builds the
  pending templates, calls `api.approve_slot_reads` for them (prompting only
  on a terminal; `slot_session(request, is_data)` already passes no handler
  for data output), then calls `resolve_schema_file`. A failure is
  `FileOutcome::schema_error` (exit 1, "unavailable check"), reusing the
  document-mode rendering and `grants_needed` (`validate.rs:1085–1101`). The
  `slots: usize` field of `FileOutcome::Validated`, `template_slot_count`
  (`validate.rs:698–704`), the "checked as written, not resolved" note
  (`validate.rs:791–793`) and the JSON `slots` record (`validate.rs:1051`) are
  deleted. Passive syntax checking stays available to DMLS through the
  library and is not exposed by the command.
- **R24 — DMLS.** Pending is rendered from the `TypeExpr::Slot` atom: hover
  and key completion show the authored property and "type pending" and no
  nested keys, requiredness, defaults or value suggestions
  (`providers/frontmatter.rs` readers listed in Phase 6). Resolution stays on
  the existing background worker (`slot_jobs.rs`): `SlotJob` already uses
  `buffers::with`, `with_slot_pins_only(true)` for requests, supersession by
  `seq`, and `SlotView.refreshing` for stale data; a last-good schema keeps
  serving hover/completion marked stale, and `diagnostics/slots.rs` emits
  `dm.schema.slot_pending` and never a "valid" verdict while pending. Watches
  need no new code: R18 makes imports `Observation`s, which `slot_reads`
  (`slot_jobs.rs:474`) already collects, including missing candidates.
- **R25 — Slot collection.** `slots::collect_slots`, `definition_slots`,
  `reachable_slots`, `stamp_atom_origin`, `visit_atom_slots_mut`, `def_refs`,
  `atom_refs`, and `omit_properties` (`slots.rs`) each gain a
  `TypeExpr::Slot` arm: the slot is reported/stamped/visited; it names no
  `DefKey`. Origin stamping covers it, so `SlotRequest::defining_file` is the
  defining file.
- **R26 — Consumers of a type that may be pending.** Two classes.
  (A) *Template-aware readers* answer pending explicitly:
  `query.rs` (`suggestions_for_def` returns `SuggestionQuery { pending: true }`
  with no items; `flattened_alternatives` exposes the atom),
  `subtype.rs` (a pending atom is a subtype only of the identical slot, and
  has no subtype proof against a pending supertype arm; a union whose other
  arms are static still proves facts from those arms), `narrowing.rs` (a
  pending atom proves no narrowing fact), `lint.rs`, `completion.rs`,
  `serialize.rs` (writes `{{ expression }}` then the suffix), `source.rs`
  (span of the slot atom), `definitions.rs` (reachability), the DMLS and
  Claudine readers. (B) *Expression-typing readers*
  (`compose/expression/scope/*`, `call_binding/translate.rs`,
  `expression/catalog/mod.rs`, `context/catalog.rs`, `contract.rs`) map a
  `Slot` to the checker's existing untyped/`UNTYPED` outcome so no diagnostic
  is produced against a type that is not known; this is diagnostic
  suppression, not the schema type `unknown`. (C) *Lowering* (`convert.rs`,
  `signature.rs`) returns
  `SchemaError::UnresolvedSlots(UnresolvedSlots::new(vec![slot]))`; the guard
  `slots::reject_unresolved` already prevents arrival, so the arm is defensive
  and never panics. Every existing helper that answers a boolean from a
  concrete type (`PropertyDef::is_required`, `admits_null`, `has_default`,
  `marks_*`) keeps its meaning for resolved definitions; each call site that can
  receive a template (`claudine/cli/.../keys.rs:286`, DMLS) switches to
  `requiredness()`/`has_pending_type()`.
- **R27 — Audit method for "every reader".** Adding the variant makes every
  exhaustive `match` fail to compile. Wildcard arms hide readers, so the
  implementer lists them: `rg -n "TypeExpr::" --glob '*.rs'` (about 890 hits
  across the files in this plan's Phase 2 list) and reads every `match`/`if
  let`/`matches!` on `.ty` with a `_ =>` arm; each one is given an explicit
  decision per R26 (the implementation log records the count and the
  disposition of each wildcard).
- **R28 — Input handling for result shapes.** The single validator for a
  returned definition is `slots::template_type::definition_from_value`
  (R8 + R12 step 1). Field shapes are tested once through it; the written
  suffix and the data path each get a control and one invalid shape; document
  shapes (duplicate keys, trailing content, YAML specifics) belong to the
  existing loaders (`load_data`, schema-file and frontmatter decoding), which
  this feature does not change, and are tested only for the new
  "quote the slot" hint (once per loader: schema file and frontmatter).
  Default for every invalid shape: a located `SchemaError::SlotResult` at the
  slot, nothing resolved, nothing published. Exceptions: `[]` and `""` are the
  ordinary definition errors; the string `"null"` and `[null]` are valid.
- **R29 — No spike.** The spec states no performance spike is required and
  its shared-definition and per-resolution memo reuse answer the cost
  question. No measurement is scheduled, no multi-host benchmark, no wider
  measurement. A measured regression found later is a new fix, not part of
  this plan.
- **R30 — Scope limits.** No new feature-specific CI environment, no change to
  the approval model, grant store, or cross-command freshness policy. Docs
  never link to or name this feature's directory (CLAUDE.md "Features and
  Fixes"). No file is created under `_unscheduled`, `_completed` is never
  touched by an agent, and the `just complete` recipe is never run.

### Wave 1 — Rename (parallel; disjoint files)

- [x] **Rename simplified-schema** — apply R1 across code, tests, fixtures, docs, skill
    - `ValueSlotKind` → `TemplateConstraintKind`, `Constraint::DynamicValue` → `Constraint::TemplateConstraint` in every file listed under R1 (also `lib.rs` re-exports).
    - `git mv` `lib/tests/fixtures/dynamic_slots/value-positions.yaml` → `template-constraints.yaml`; fix every reference (`rg value-positions`), including `fixtures/dynamic_slots/README.md`.
    - Rename test functions in `grammar/slot_tests.rs` and `lib/tests/l1/*.rs` whose names or messages contain "value slot", "value position", "whole type".
    - In `docs/topics/language/slots.md`: replace the vocabulary ("value slot", "value-position slot", "A value slot is the whole argument") with **template constraint**; rewrite the section "Whole Types (Planned)" to **Template Types** and keep it marked **planned** (Phase 8 removes the marker); update the intro paragraph to name both kinds.
    - `.claude/skills/simplified-schema/SKILL.md` and `.claude/skills/darkmatter/schema.md`: same vocabulary change.
    - Risk: "value position" also appears in unrelated prose; only the schema-slot sense is renamed — read each hit.
- [x] **Rename darkmatter and claudine** — apply R1 (docs only) and R2
    - Darkmatter docs: `darkmatter/docs/topics/schemas/schema-slots.md`, `authoring-schemas.md`, `dmls-schema-support.md`, `darkmatter/dmls/docs/autocomplete.md`, `.claude/skills/darkmatter/dmls.md`; test names and comments in `darkmatter/dmls/tests/l1/recursive_schema_editor.rs`, `darkmatter/cli/tests/l1/hash_kind_save_diff.rs`, `schema_union_matrix.rs`, and `darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs` / `l1/dynamic_slots_fixture_repos.rs` where the old terms occur.
    - Claudine: apply R2 to every listed site; rename test identifiers (`engine/tests.rs`, `partition/tests.rs`) that embed "value slot".

### Wave 2 — Validate the rename

- [x] **Rename guard** — nothing banned remains
    - Run `rg -n -i "value[ -_]slot|value[ -_]position|whole[ -_]type[ -_]slot|ValueSlotKind|DynamicValue" --glob '!**/_completed/**' --glob '!darkmatter/features/2026-11-08-template-types/**'`; the only permitted hit is none. Add the one command to the implementation log.
    - Run `just test` and `just lint` in `simplified-schema`, `darkmatter`, `claudine`; run `just test-minimal` in `simplified-schema`.

**Checkpoint 1:** the guard prints nothing; all three areas are green; no behavior changed (diff outside docs/tests/names is only the two renamed symbols).

## Phase 2 — Grammar, `TypeExpr::Slot`, and passive readers

Everything here is passive: a template type parses, queries, exports, and
reports a typed pending state, but nothing resolves yet. At the end of the
phase the whole workspace compiles.

### Wave 3 — The atom and the grammar (single agent; every later wave depends on `types.rs`)

- [x] **Add the atom** — `TypeExpr::Slot(Slot)` and helpers
    - Add the variant (R3) with docs that state it is the pending type and never lowers; `PropertyAtom::is_pending_type`, `PropertyDef::has_pending_type`, `Requiredness` and `PropertyDef::requiredness()` (`Pending` when any atom is a slot at the top level of the definition; otherwise `is_required` → `Required`/`Optional`).
    - Extract `PropertyDef::check_union_invariants` and `TupleType::check_element` (R11); the parser keeps calling them; behavior of static schemas must not change (the whole existing suite is the proof).
    - Add the new `SlotResultKind` variants and the `SlotResult { property, related }` fields (R4) and the `SchemaError::SlotReadRefused` variant (R3/R16); update every constructor and the `miette`/`StatusBlock` rendering in `errors.rs` (`errors.rs:964–1000`).
- [x] **Parse and refuse** — R5, R6
    - `parse_type_expr` slot branch with the suffix reader factored out of `parse_named_reference` (`grammar.rs:2147`) so both call one function.
    - `SlotPosition` and `slot_position_error`; set the position in `parse_tuple` (element vs spread), `parse_function` (parameters/returns), and the `property(...)` parser (`grammar.rs:3060–3085`); detect the non-grammar positions at the sites named in R6; reject `{{` in an import path (`read_fileref`).
    - Update the grammar doc comment (`grammar.rs:62`: `slot := "{{" expression "}}"` is now also a type atom).
    - Update `slots.rs` for R25 (`collect_slots`, `def_slots`/`atom_slots`, `def_refs`/`atom_refs`, `stamp_*`, `visit_*_mut`, `omit_properties`) so a template type is a reported slot.
    - Risk: `slots/substitute.rs` matches `TypeExpr` exhaustively (`Substitution::atom`): add a pass-through `TypeExpr::Slot(_) => {}` arm now; Phase 3 replaces it.

### Wave 4 — Readers (parallel; each agent owns disjoint files)

- [x] **Source and export** — spans, serialization, "quote the slot"
    - `simplified/source.rs`: the slot atom's span through plain, `'…'`, `"…"`, `|`, `>` scalars, Markdown frontmatter and CRLF (reuse the machinery that positions template-constraint slots; add the atom position kind).
    - `simplified/serialize.rs`: write `{{ expression }}` plus suffix and description exactly as authored; exporting never evaluates.
    - R7: `slots::unquoted_slot_hint`, wired at both sites in `simplified/mod.rs` and `simplified/standalone.rs`; Darkmatter's frontmatter site is wired in Wave 5.
- [x] **Query-side readers** — pending answers per R26(A)
    - `query.rs`, `subtype.rs`, `narrowing.rs`, `lint.rs`, `definitions.rs`, `signature.rs`, `completion.rs`, `phase.rs`, `file_match.rs`, `frontmatter_shape.rs`.
    - Rules to implement and unit-test in each file: pending atom suggests nothing and reports `pending: true`; `is_subtype` per R26; narrowing proves nothing from a pending atom; `phase`/`file_match`/`frontmatter_shape` treat a pending property as present-by-name with no shape facts.
- [x] **Resolver and lowering passthrough** — R26(C)
    - `resolve.rs` (`expand_atom` pass-through, `atom_has_imports`, `rehome_atom_examples`, `check_non_null_use`), `resolve/graph.rs` (`discover_atom` ignores `Slot`), `resolve/recursive.rs` (summaries skip `Slot`), `simplified/convert.rs` (defensive typed error), `simplified/mod.rs`.
    - Lowering, `validate_strict/soft`, and `layer_under/layer_over` must produce the typed `SchemaError::UnresolvedSlots` through the existing `reject_unresolved` guard: add tests per entry point.

### Wave 5 — Downstream compile and explicit answers (parallel by package)

- [x] **Darkmatter readers** — R26(B) and R27
    - Files: `schemas/detect.rs`, `schemas/completion.rs`, `schemas/mod.rs`, `schemas/catalog.rs`, `schemas/triggers/matcher.rs`, `lifecycle/schema_projection.rs`, `compose/expression/data/contract.rs`, `scope/shape.rs`, `scope/iterable.rs`, `scope/projection.rs`, `call_binding/translate.rs`, `expression/catalog/mod.rs`, `context/catalog.rs`, `cli/src/commands/schema/assignment.rs` (resolved-only input: R26(C) outcome, plus a `debug_assert!`-free typed fall-through), `dmls/src/providers/frontmatter.rs`, `dmls/src/overlay/suggestions.rs` (compile-level pending arm: show nothing, claim nothing; Phase 6 does the views).
    - Wire the frontmatter YAML-decode "quote the slot" hint (R7 site b) and test it through frontmatter extraction.
- [x] **Claudine readers**
    - `claudine/lib/src/composition/schema/classify.rs` and `claudine/gen/src/schema_compat.rs` (a `Slot` is an unsupported shape for generation: return the existing "unsupported" outcome with a message naming a template type); `claudine/cli/.../schema_completion/keys.rs` keeps compiling (behavior is Phase 7).
- [x] **Passive tests** — simplified-schema `lib/tests/l1/dynamic_slots.rs` plus new `lib/tests/l1/template_types_passive.rs`
    - Fixtures: new `lib/tests/fixtures/dynamic_slots/template-types/` (schema files using template types at each allowed position with a plain data file).
    - Cover: string and list-form positions; property type, nested object property, named-type body, union arm (`|` and YAML list), tuple element; suffixes `[]`, `(required)`, `[](required)`, description; literal brace text in a quoted enum member and a default stays literal (no slot reported); passive `query`/`property`/`named_type`/`to_simplified_yaml` evaluate nothing (use a host that panics if called, never given); lowering / validate / layering entry points return typed `UnresolvedSlots`.
    - Refused positions (R6): one test per "No" row (root `$schema` scalar, root-union arm, property key, pattern key, import path, part of a type name, tuple spread, function parameter, function return, `property(...)`), each asserting the message names the position, lists the allowed positions, and carries the rewrite sentence only where R6 gives one.
    - Positioned errors: spans through plain, quoted, block scalar, Markdown frontmatter, and CRLF; "quote the slot" for both the mapping-key shape and a YAML decode failure, and the negative case (an unrelated mapping error keeps its message).

**Checkpoint 2:** `just test` and `just lint` pass in `simplified-schema`, `darkmatter`, `claudine` (they compile and the existing suites are unchanged); the `rg` audit of R27 is recorded in the implementation log; all passive tests above pass.

## Phase 3 — Resolution engine (simplified-schema)

Makes a template type resolve to the same schema as the static definition, and
adds the gated read boundary.

### Wave 6 — Boundary and plumbing (single agent)

- [x] **Host hooks** — R14
    - New `slots/reads.rs`: `TemplateReads`, `SchemaRead`, `SchemaReadKind`, `SchemaReadOutcome`, thread-local scope `with_template_reads` and the sticky-refusal recorder (R16). `SlotHost::template_reads` required; update the in-repo hosts: `lib/tests/l1/dynamic_slots.rs` helpers and the docs example. `DarkmatterSlotHost::template_reads` returns, in this wave, an object whose `admit` refuses every read with `SLOT_APPROVAL_REQUIRED_CODE` and whose `observe` does nothing (a safe default so the workspace compiles); Phase 4 replaces it with the real implementation of R18.
- [x] **Resolve plumbing** — R16, R17
    - `PendingSchema::resolve` installs the scope; `Attempt::Refused`; mapping of `SlotReadRefused` in `resolve_with`; `replay_with` under `buffers::with_reads_retained`.
    - `apply_active`/`apply_active_to_arm` signature with `ExpansionSite` and the `LateDependencies` return (R9), merged at the four call sites into `referenced_files`/`searched`.
    - `ImportEngine.template_slot` and the guarded paths of R15 (`resolve_namespace`, `select_schema_file`'s candidate pre-check via a new `select_schema_file_checked(…, &dyn Fn(&Path)->Result<(), SlotAdmissionError>)` used only by the guarded engine, `rehome_atom_examples`); with no scope installed the engine behaves exactly as before.

### Wave 7 — Substitution (single agent; the core)

- [x] **Template type substitution** — `slots/template_type.rs`, R8–R13
    - `definition_from_value` (R8 + R12 step 1), the synthetic-root double expansion (R10), checks 2–3 of R12, invariant re-check (R11), related information (R4), property path in every error.
    - Replace the pass-through arm in `Substitution::atom`; run type slots before constraint substitution (R9); handle a slot as a union arm, a tuple element, an array item, inside an inline object, inside a named type body, and inside a stored definition (`definitions.bodies_mut`).
    - Risk: shared/cyclic definitions: a returned definition importing a self-recursive type must produce `TypeExpr::Ref` to a published stored definition; test it (R10 evidence).
    - Risk: `SlotKey` is the evaluation identity, so a named type holding a template type used twice evaluates once and substitutes at both sites.

### Wave 8 — Resolution tests (parallel; one file each)

All tests run in-process against the library's public API (`resolve_slots`,
`PendingSchema::resolve`, `validate_strict`, `to_json_schema`) with a fixed-value
test host; no binary is spawned here. Every positive test asserts equivalence
with the static schema (same `to_json_schema` and same validation of one valid
and one invalid document).

- [x] **Result shapes** — `lib/tests/l1/template_type_results.rs` (R28 matrix)
    - Control: `"enum(draft, published)"` equals the static `status`.
    - Exceptions-only table: JSON `null`, mapping, number, boolean → `NotATypeDefinition` naming slot and property; `[]` and `""` → ordinary definition errors; `["string", 123]` and `[123]` → error (never filtered or empty); `"null"` and `[null]` → valid null type; string-encoded list `"[string, number]"` → union; array result → union that flattens into the surrounding union and into the YAML list form; array suffix over a union; `(required)` over `"null"`, `"unknown"`, and a null-admitting union stays optional; nullable array item does not make the array nullable; conflicting default and constraint with the suffix → `SuffixConflict` naming the slot and the suffix as related; invalid `non-null` combination after expansion → `InvalidExpansion`; template-constraint slot in the suffix still resolves.
    - Suffix and data path: one control and one invalid shape each.
- [x] **Imports and reads** — `lib/tests/l1/template_type_imports.rs`
    - Defining-file base: slot in the document, in a referenced schema file, and in a named type imported from a nested directory each resolve `Person@./types.yaml` against their own file, and a remote or other-directory data file never changes the base; local references inside an imported file (`Name@this` there) work; `Name@this` / bare `Name` in returned text → `LocalReference`; import cycle → `ImportCycle`, distinguishable from refusals; a returned definition importing a self-recursive type resolves; shared import read once per resolution (`work-counters`, `observed::Observed::opened`); recorded dependencies (`referenced_files`, `searched`) include imported files and missing candidates on success and on failure.
    - Guard: a host that refuses records the probe before anything is probed and before any open (assert no `opened` entry, memo untouched via `memo_holds`), the refusal keeps its code, a denial never yields a schema (including where discovery swallows the error), a static schema import's ability to read the same file does not authorize a slot-selected import, and an example reached by a returned import is gated.
- [x] **Slot and pattern refusals** — `lib/tests/l1/template_type_refusals.rs`
    - Nested active slot in returned text (template constraint inside) → `NestedSlot`; literal `{{ … }}` inside a quoted enum member and default stays literal and passes; slot reachable through an import: direct, transitive, and unused-slot-elsewhere (passes); a statically authored import of the same slot-bearing type still works; introduced patterns: 4096 bytes accepted, 4097 refused, lookaround, backreference, size-limit compile failure, directly through an import; a pattern written directly in the schema keeps its current rules; suffix patterns exempt.

**Checkpoint 3:** `just test`, `just lint`, `just test-minimal` in `simplified-schema` pass; grep for the code smells of the robustness section over the new files returns no hit that affects a load-bearing field: `rg -n "serde\(default\)|filter_map\(.*as_str|unwrap_or_default\(\)|\.ok\(\)" simplified-schema/lib/src/slots/template_type.rs simplified-schema/lib/src/slots/reads.rs` (each hit justified in the log).

## Phase 4 — Darkmatter library

### Wave 9 — Host and cache (single agent)

- [ ] **Host implementation** — R18, R19
    - `DarkmatterTemplateReads` and `DarkmatterSlotHost::template_reads` in `slots/host.rs`; new `SLOT_*` code mapping for the new `SlotResultKind` variants in `slots/codes.rs` (`slot_result_code`) and in `nested_resolution_code` (`SlotReadRefused` arrives as `Refused`, already handled); `SlotReadRefused` and the new `SchemaError` variants handled in the exhaustive matches the compiler lists (`schemas/slots/session.rs failure_of`, `compose/report_diagnostics.rs:344,801–829`, `triggers/finding.rs`, dmls `slot_jobs.rs is_slot_failure`).
    - Make sure `observe` runs inside `capture_observations`/`dependencies::collect` (the host is called from `pending.resolve` inside them, `session.rs:459`), including for nested contract resolutions that use `unguarded`.
- [ ] **Slot-pending reporting** — report pending context
    - `compose/report_diagnostics.rs` and `compose/schema_validation.rs:810 defers_template`: the unresolved-template report names the property with a pending type ("template type") beside template-constraint slots, using the slot's `property` path; embedding consumers keep the existing unresolved-template report (no new approval mechanism).

### Wave 10 — Surfaces (parallel; disjoint files)

- [ ] **Schema-file resolution** — R23 library half
    - `DarkmatterSchemas::resolve_schema_file`; unit tests with granted, undecided-without-handler, and denied grants.
- [ ] **Triggers and layering** — R20
    - Tests only plus any missing `omit_properties` case: a trigger payload `owner: "{{ … }}"` is pending and matches; when the document declares `owner`, the slot never evaluates and requests no read (assert via a counting evaluator and the gate log); a returned object's nested keys never become top-level properties; a root-union lower declaration replacement follows the existing every-arm rule; slots in `match` conditions, kind definitions, the baseline, global catalogs, the editor extension layer, and a `kind:` YAML document definition remain refused, including through imports.
- [ ] **Compose, clean, phase, reference checks** — R22
    - Tests only through the normal command paths in `darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs` (extend): composition, `md clean`, phase projection resolve before defaults, coercion, required-key decisions; a failed resolution is never reported valid or clean; `md graph` / `md validate refs` keep deferral with no grant and use no substitute type.

### Wave 11 — Darkmatter library tests (parallel; reuse `SlotRepo` in `darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs`)

- [ ] **Reads and grants** — `dynamic_schema_slot_template_types.rs`
    - Read approval before memo or buffer access (instrument the data memo and `buffers`); transitive imports; absent grants without prompts (`SessionMode::ReferenceCheck`, no handler); computed target asked through the handler once and remembered; denied keeps `dm.schema.slot_denied`; refusal never falls back to a schema.
- [ ] **Cache and dependencies** — same file
    - Cached read observations (a warm result reports the same reads as a cold one: compare `data` read sets); a write to the data file, to an imported file, to a deeper transitive import, and creation of a previously missing candidate each force re-resolution before the next validation; grants re-checked before a dependency is reread; the existing omitted-property key, request-context inputs and size/mtime policy unchanged (the existing cache tests stay green).
- [ ] **Equivalence** — static-vs-template paired reports through `DarkmatterSchemas` for string, list, union, nested, array results.

**Checkpoint 4:** `just test` and `just lint` pass in `darkmatter`.

## Phase 5 — `md` CLI

### Wave 12 — Commands (single agent, then tests)

- [ ] **Schema validate** — R23 CLI half
    - Implement as R23 states; delete `slots`, `template_slot_count`, the note and the JSON record; update the CLI's own help text if it mentions templates (`darkmatter/cli/src/commands/schema/`).
- [ ] **Type-aware assignments** — `assignment.rs`
    - Resolve through the grant gate before `apply_all`/`coerce_value`/`def_prefers_string` run, preserving the raw assignment text until the type is available; a refused resolution exits 1 as an unavailable check. `validate_one` (`validate.rs:541`) already calls `effective_for` when there are assignments; ensure the resolved effective schema is what the assignment code reads.
- [ ] **CLI tests** — `darkmatter/cli/tests/` (`dynamic_schema_slots.rs`, `schema_validate*.rs`)
    - Spawn the real binary (no terminal window gains focus; data output and `--output json` variants): document mode with a template type resolved, refused (`dm.schema.slot_approval_required`, exit 1), and approved via the existing non-interactive grant flags; assignment (`status=published`) coerced by the resolved type, and `status=…` with the grant absent; schema-file mode over a template-constraint slot and a template-type slot with granted, prompted (simulated through the existing approval-handler seam, not a TTY), and refused reads — each success is exit 0, each failure exit 1 with the located error and no `slots` field; a malformed returned definition no longer passes as a "valid template".

**Checkpoint 5:** `just test` and `just lint` in `darkmatter` (cli crate included) pass.

## Phase 6 — DMLS editor

### Wave 13 — Editor (parallel by file; then tests)

- [ ] **Pending views** — R24
    - `providers/frontmatter.rs`: hover (`:2266 schema_hover_details`, `:1935`, `:2173`), key completion (`shape_key_completions`, `nested_key_completions`, `top_level_key_completions`), value completion (`value_completions`, `def_value_completions`, `union_item_completions`, `array_item_value_completions`), `known_shape`/`nested_shape` (`:2554`, `:2607`): a pending atom shows the authored property and "type pending"; it never invents nested keys, requiredness, defaults, or value suggestions; static sibling information (other properties, static union arms) is preserved. `overlay/suggestions.rs` (`collect_reference_candidates` `:125`) and `overlay/schema.rs` get the explicit pending answer.
- [ ] **Worker and diagnostics** — `slot_jobs.rs`, `diagnostics/slots.rs`
    - Pending → resolved transition, failed state, stale marking of last-good completion/hover, no "valid" diagnostics while pending, cancellation/supersession publishing no partial schema (reuse `seq`/`canceled`); confirm the unsaved-buffer and import/missing-candidate watches via R18 (`watched_paths`).
- [ ] **Editor tests** — `darkmatter/dmls/tests/l1/schema_slots_editor.rs` and `slot_jobs.rs` inline tests
    - Pending-to-resolved transition after the worker finishes; cancellation by a newer edit; an unsaved edit to an imported file re-resolves; a created missing candidate re-resolves; a failed resolution shows failed status, last-good is stale.

**Checkpoint 6:** `just test` and `just lint` in `darkmatter` (dmls crate included) pass.

## Phase 7 — Claudine shell completion

### Wave 14 — Completion (single agent)

- [ ] **Resolve on value requests** — spec "Claudine shell completion"
    - `claudine/cli/src/completion/schema_completion/mod.rs`: `load_prompt_schema` (`:98`) builds `DarkmatterSchemas::new(prompt.context).with_slot_session(Arc::new(SlotSession::for_context(DataRuntime::new(), &context).with_mode(SessionMode::ReferenceCheck)))` and calls `effective_for` only for a **value request on a property whose authored definition holds a slot** (`slots::reachable_slots(def, definitions)` non-empty on the template's declaration). `ReferenceCheck` with no handler never prompts and uses only the automatic and persisted grants (`session.rs:69`, `run_grants` `:370`), exactly the non-prompting set; nothing is written to the terminal.
    - A successful resolution supplies the resolved property definition to `candidates.rs::property_value` (enum members, matched files). An unresolved or failed resolution returns no candidates so the shell fallback runs (`engine/mod.rs:270–274`), silently.
    - Names: `run_setter_name` keeps using the authored template declarations (`PromptSchema::declarations`), nothing evaluated; required-first ordering (`keys.rs:286`) uses the authored suffix via `requiredness()` with `Pending` ordered as optional-by-authored-suffix (a template type's requiredness is not resolved for ordering).
    - A property with no slot completes its values from the template declaration without evaluation, even in a template (`kind: enum(a, b)` → `kind='a'`, `kind='b'`).
    - Resolution is of the whole template through the existing resolver and cross-command cache; a slot elsewhere in the template that is ungranted fails the resolution, and the property then offers nothing (accepted: all-or-nothing keeps one code path and one cache entry).
    - `PromptSchema::Pending` keeps carrying the template; its producer at `mod.rs:106` stays; the consumers change as above.
- [ ] **Completion tests** — `claudine/cli/tests/l1/completion_committed_prompt_schema.rs`, `completion_setter.rs`, `schema_completion/tests.rs` (replace the Pending test at `:1167`)
    - `stat<TAB>` → `status=` with nothing evaluated (a counting file read proves it); `status=<TAB>` with a template-constraint slot and with a template type on a committed clean repo (automatic grant) offers the resolved enum members; with the grant absent offers nothing and prints nothing (assert empty stdout and stderr); `kind=<TAB>` in a template completes from the static enum without evaluation; a persisted user grant (fake home via `common::completion::fake_home`) is honored; a volatile or URL-reading resolution is not served cross-process and offers nothing. Run through the existing `run_complete*` helpers; no terminal windows.
    - Update `claudine/docs/topics/completions/shell-completions.md` (Phase 8 verifies).

**Checkpoint 7:** `just test` and `just lint` pass in `claudine` (cli and lib crates).

## Phase 8 — Docs, skills, and final verification

### Wave 15 — Documentation (parallel; every page written for a developer new to the repo, with a worked example per rule and a Mermaid diagram for the resolution flow and for the read-approval flow)

- [ ] **SimplifiedSchema docs** — `simplified-schema/docs/topics/language/slots.md`
    - Replace "Whole Types (Planned)" with **Template Types**: the grammar and suffix, the **slot-position table** (same text the diagnostics print: allowed rows and each refused row, R6), the result shape table (R8) and failure table, defining-file import bases, reachable-slot refusal and pattern limits, the host's `template_reads` hooks, what a pending template type reports through each operation (extend "What a Template Reports"), and `PendingSchema`/`omit` behavior. Remove every "planned" marker.
- [ ] **Darkmatter docs** — `darkmatter/docs/topics/schemas/schema-slots.md`, `authoring-schemas.md`, `schema-triggers.md` (trigger payload decision), `schema-validation.md`, the `md schema validate` command reference page under `darkmatter/docs/` (find with `rg -l "schema validate" darkmatter/docs`: schema-file mode now resolves, no `slots` count, exit 1 when unavailable), `dmls-schema-support.md`, `darkmatter/dmls/docs/autocomplete.md` and `hover.md` ("type pending", stale marking). Cache and grants: document that returned imports are computed targets asked at resolution time.
- [ ] **Claudine docs** — `claudine/docs/topics/completions/shell-completions.md`: replace the "switched off for every property" description with the new behavior, the table of name vs value requests, and the non-prompting grants; "setter value" vocabulary per R2.
- [ ] **READMEs** — touch `simplified-schema/README.md`, `darkmatter/README.md`, `claudine/README.md` only where an example or description changed (check with `rg -n "slot|whole" <README>`).
- [ ] **Skills** — `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`, `dmls.md`, `cli.md`: names, positions, host hooks, validate-schema-file behavior. Keep `SKILL.md` under 200 lines with links.
- [ ] **Docs never name this feature**: `rg -n "template-types" -g '!**/features/**' -g '!**/fixes/**'` prints nothing.

### Wave 16 — Final verification (sequential)

- [ ] **Terminology and drift** — rerun the Phase 1 guard; confirm no comment still says "value slot", "whole-type slot", or "planned" for template types; fix comment drift in every symbol this plan changed (CLAUDE.md "Code Comment Quality": code is right, comment is wrong).
- [ ] **Corpus and regression** — keep the passive shipped-schema corpus (`darkmatter/lib/tests/l1/shipped_schema_corpus.rs`) and static-schema regression checks green; static schemas take no new path.
- [ ] **Full runs** — `just test` and `just lint` in `simplified-schema`, `darkmatter`, `claudine`; `just test-minimal` in `simplified-schema`; from the repo root `just ci-local --plan` to confirm the affected-scope plan names only expected packages (informational; nothing is pushed).
- [ ] **Implementation log** — write `implementation-log.md` beside this plan: the R27 audit (count and each wildcard disposition), anything built that a human will want to judge (the "quote the slot" wording, refusal sentences, the all-or-nothing Claudine resolution, whole-template resolve for completion), and every departure from the spec. The terminal state is "implementation complete, ready for review"; do not move the directory, do not run `just complete`.

**Checkpoint 8 (done):** every item in "Definition of done" holds; all commands above are green; the implementation log exists.