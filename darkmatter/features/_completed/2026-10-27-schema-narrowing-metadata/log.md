---
feature: 2026-10-27-schema-narrowing-metadata
implementation_1: "2026-10-27T21:58:41-07:00"
---

# Implementation Log: Schema Narrowing Metadata

## Implementation of Review Findings #1

> **started at:** 2026-10-27T21:58:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-27-schema-narrowing-metadata/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1 | Passive schema consumers skip narrowing semantic checks | fixed | rules moved to schema-owned `simplified/narrowing.rs`; every child row verified; full `just test` 12960/12960 |
| R1-F1.1 | Public type parser: `file`, `string(min(3))`, `enum(a,b)` with `true-iff` rejected | fixed | `meta_schema_phase1::public_type_parser_rejects_true_iff_narrows_on_a_refinement` |
| R1-F1.2 | Standalone schema parser: same targets plus overload disagreement (absence, target, kind, position) rejected | fixed | `meta_schema_phase1::standalone_schema_parser_rejects_invalid_narrows_declarations` (`\|` and YAML-list spellings) |
| R1-F1.3 | Runtime registration keeps rejecting those cases | clean | `flow_sensitive_narrowing::registered_facts::registration_rejects_invalid_facts`, `registration_rejects_overloads_that_disagree_on_the_fact` (text → `Grammar`, programmatic → `InvalidFact`) |
| R1-F1.4 | Host catalog loader keeps rejecting invalid exact-family targets | clean | new rows in `host_functions::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation`; `catalog::tests::overloads_must_agree_on_the_declared_fact` |
| R1-F1.5 | Meta-type validation: `string` control valid; `file`/`string(min(3))` `true-iff` invalid | fixed | `meta_schema_phase1::type_definition_rejects_narrows_facts_registration_rejects`; `schema` meta-type in `schema_consumers_reject_named_types_nested_in_narrows_targets` |
| R1-F1.6 | DMLS standalone authoring diagnoses invalid `true-iff` and mixed overload clauses; control clean | fixed | DMLS `schema_yaml_spellings::a_narrows_fact_registration_rejects_is_diagnosed_while_authoring` |
| R1-F1.7 | Grouped array `(string \| number)[]`: parser and registration agree | fixed | decision: accept grouped-union arrays as one-way targets in parser and registration (an array is one type; only top-level unions rejected; `true-iff` still rejects); `registered_facts::a_grouped_union_array_is_a_one_way_target`; added to `human_review_items` |
| R1-F1.8 | Distinct binding contracts with the same fact still accepted | clean | control cases accepted in registration, standalone, YAML-list, meta-type and catalog tests |
| R1-F2 | Compound narrowing targets admit unresolved named types | fixed | `narrows_target_problem` recursive over every `TypeExpr` variant (exhaustive match); all children verified |
| R1-F2.1 | `{a: Name}` rejected with located unsupported-target error (parser, registration, host loader) | fixed | `meta_schema_phase1::public_type_parser_rejects_named_types_nested_in_narrows_targets`, `schema_consumers_reject_named_types_nested_in_narrows_targets`, `registered_facts::registration_rejects_named_types_nested_in_a_target`, `host_functions::a_host_catalog_rejects_named_types_nested_in_a_fact_target` |
| R1-F2.2 | `tuple([Name])` rejected likewise | fixed | same four tests (`tuple([Name])`) |
| R1-F2.3 | `Name[]` still rejected | clean | same tests assert `Name[]` still rejected |
| R1-F2.4 | `(Name \| string)[]` rejected for the nested reference consistently | fixed | same tests; message names the type, not "union" |
| R1-F2.5 | Self-contained `{a: string}`, `tuple([string])`, `string(min(3))`, `enum(a,b)` one-way accepted | clean | controls accepted with fact kind checked in the same tests |
| R1-F2.6 | No usable fact exists for a target containing a name (programmatic path included) | fixed | programmatic `builder_from_types` with a nested name → `InvalidFact`, no registration; also union arm in object, dictionary value, tuple spread, nested function parameter, `Name@./other.yaml` |
| R1-F3 | Invalid predicate calls still grant narrowing facts | fixed | gate `call_is_rejected` at the one fact-granting arm of `scope/facts.rs::assume`; all children verified |
| R1-F3.1 | `probe_pred(foo, 1)` true side `file` | clean | `rejected_calls::a_rejected_call_grants_no_fact_on_either_side` (valid call narrows to `file`) |
| R1-F3.2 | `probe_pred(foo, {})` retains `unknown` | fixed | same test |
| R1-F3.3 | `!probe_pred(foo, {})` retains `unknown` | fixed | same test (negated and parenthesized forms) |
| R1-F3.4 | Short-circuit: following `frontmatter(foo, 'title')` gets no fact | fixed | `rejected_calls::a_rejected_guard_does_not_narrow_later_operands` (`&&`, `and(...)`, `?:`) |
| R1-F3.5 | `::block when=` bad guard gives body no fact | fixed | `rejected_calls::a_rejected_block_guard_does_not_narrow_its_body` |
| R1-F3.6 | Arity controls (missing / extra argument) give no fact | clean | `rejected_calls::a_wrong_argument_count_grants_no_fact` |
| R1-F3.7 | Omitted optional subject gives no fact | clean | existing `registered_facts::an_omitted_optional_fact_argument_narrows_nothing` |
| R1-F3.8 | DMLS branch projection shares TypeScope result | fixed | DMLS `expression_scopes::local_type_takes_no_fact_from_a_rejected_guard_call`; siblings: `every_implication_kind_withholds_a_rejected_calls_fact`, `an_argument_that_may_bind_keeps_the_fact` |
| R1-F4 | Narrowing failures lose required diagnostic provenance | fixed | `SchemaError::Grammar` gains structured `related`; projected through source mapping into DMLS `relatedInformation`; all children verified |
| R1-F4.1 | Grammar duplicate clause: structured primary + related location | fixed | `meta_schema_phase1::public_parser_relates_a_duplicate_narrows_clause_to_the_first`; grammar unit `a_duplicate_clause_is_located_at_the_second_and_relates_the_first` |
| R1-F4.2 | DMLS quoted scalar duplicate: `relatedInformation` at first clause | fixed | `schema_yaml_spellings::a_duplicate_narrows_clause_relates_the_first_while_authoring`, `disagreeing_narrows_overloads_relate_both_declarations_while_authoring` |
| R1-F4.3 | DMLS block scalar duplicate: `relatedInformation` at first clause | fixed | same two tests (block scalar); imported files: `named_type_references::a_rejected_narrows_declaration_in_an_imported_file_is_related_into_that_file` |
| R1-F4.4 | Host catalog semantic rejection carries located target range | fixed | `host_functions::a_host_catalog_locates_a_rejected_fact_in_its_source` (`HostCatalogError::span()`/`related()`) |
| R1-F4.5 | Overload mismatch retains both declaration origins when available | fixed | `registered_facts::registration_rejects_overloads_that_disagree_on_the_fact` (text spans; `InvalidFact { overloads }`); `meta_schema_phase1::public_parsers_relate_disagreeing_narrows_overloads_to_both_declarations` |
| R1-F4.6 | Single-token errors keep precise ranges | clean | `schema_yaml_spellings::an_invalid_narrows_clause_in_a_block_scalar_is_ranged_at_its_token` passes unchanged |
| R1-F4.7 | Source-free programmatic errors: typed, no invented range | clean | later-overload missing-parameter case: typed `InvalidFact`, no range, `overloads == [1]` |
| R1-F4.8 | Sibling: repeated `parameters`/`returns`/`fallible`/`category` and second `suggest` relate the first keyword | fixed | grammar unit `a_duplicate_function_constraint_relates_its_first_keyword`; suggest covered by existing suggest tests |
| R1-X1 | Nonblocking from review frontmatter: CLI `too_many_arguments` lint failure (`cli/src/commands/render.rs`) | fixed | `#[allow(clippy::too_many_arguments)]` per sibling CLI commands; `just lint` exit 0 |

### Progress

- starting the work on 'R1-F1: Passive schema consumers skip narrowing semantic checks' at 21:59:23
        - moved the declaration rules into schema-owned `lib/src/markdown/schemas/simplified/narrowing.rs` (`clause_problem`, `declaration_problem`, `overload_arms_problem`), together with the pure family predicates (`Family`, `families`, `exact_family`, `is_exact_fact_type`, `type_identity`, `has_ordinary_constraints`) that `scope/shape.rs` now imports; the grammar applies them per function and beside each `duplicate_overload` site (top-level string unions, inline-object members, YAML-list unions), and `compile_declared_fact` validates through them and only lowers
        - signature text with an invalid fact now fails as `RegistrationError::Grammar`; programmatic `builder_from_types` declarations keep `InvalidFact`; the two registration tests were updated to assert both paths
        - R1-F1.7 decision: accept `(string | number)[]` as a one-way target in both parser and registration (an array of a grouped union is one array type; only a top-level union is rejected). `Shape::suitable_for` honors it: `unknown` becomes the target and an array alternative becomes the target (`a_grouped_union_array_is_a_one_way_target`); `true-iff` still rejects it as not an exact family
        - catalog loading now fails a bad fact as `CatalogError::Schema` (the grammar rejects first) rather than `CatalogError::Invalid`; `catalog::tests::overloads_must_agree_on_the_declared_fact` and `a_clause_naming_a_parameter_the_overload_omits_fails_the_load` updated to match
        - added public-API and editor regressions: `meta_schema_phase1::{public_type_parser_rejects_true_iff_narrows_on_a_refinement, standalone_schema_parser_rejects_invalid_narrows_declarations, type_definition_rejects_narrows_facts_registration_rejects}`, host-loader rows in `host_functions::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation`, `flow_sensitive_narrowing::registered_facts::a_grouped_union_array_is_a_one_way_target`, and DMLS `schema_yaml_spellings::a_narrows_fact_registration_rejects_is_diagnosed_while_authoring`
        - docs: `docs/topics/schemas/definition.md` narrowing rules (one-way refinements, grouped-union arrays, overload agreement, one set of rules); skill `darkmatter/expressions.md` records the schema-owned validator
        - completed R1-F1: `just test narrows host_function shipped_schema_corpus meta_schema schema_yaml_spellings flow_sensitive file_predicates type_scope design_time_call_check overload grammar scope catalog` 1029/1029 passed; `cargo clippy -p darkmatter -p dmls --all-targets -- -D warnings` clean (`just lint` still stops on the existing CLI `too_many_arguments`)
        - orchestrator: subagent reports 1029/1029 focused tests passing; F1.7 decision = accept grouped-union arrays as one-way targets in parser and registration
- work completed for 'R1-F1: Passive schema consumers skip narrowing semantic checks' at 22:15:07
- starting the work on 'R1-F2: Compound narrowing targets admit unresolved named types' at 22:15:07
        - defect class: target validation checked only the outer `TypeExpr` node; `narrows_target_problem` (`simplified/types.rs`) now walks the whole target with an exhaustive `match` (no wildcard) over every `TypeExpr` variant: `Imported` and `Ref` are rejected; `InlineObject` members and pattern-key (dictionary) values, `Union` arms, `Tuple` positions and spread, and `Function` parameters/returns/nested narrows target are walked; `Primitive` is a leaf. The error names the type (`a narrowing target cannot use the named type \`Name\`, ...`)
        - swept sites: every consumer reaches the check through `narrowing::clause_problem` (grammar per function and overload groups, YAML-list unions in `simplified/mod.rs`, host catalog loading, registration's `compile_declared_fact`, which validates before lowering, so no `NarrowingFact` is built for a rejected target) plus the grammar's early check in `parse_narrows`, located at the target span; nested atoms carry no spans, so the target span is the location
        - tests: `meta_schema_phase1::{public_type_parser_rejects_named_types_nested_in_narrows_targets, schema_consumers_reject_named_types_nested_in_narrows_targets}`, `flow_sensitive_narrowing::registered_facts::registration_rejects_named_types_nested_in_a_target` (text and `builder_from_types`), `host_functions::a_host_catalog_rejects_named_types_nested_in_a_fact_target`; extra sibling shapes covered: union arm inside an object, dictionary value, tuple spread, function parameter inside an object, `Name@file` two levels deep; all four fail with the outer-only check restored
        - `schema` meta-type sibling: already rejects `narrows(x: file; true-iff)` (asserted in `schema_consumers_reject_named_types_nested_in_narrows_targets`); no code change needed; docs now name both meta-types
        - docs: `docs/topics/schemas/definition.md` Targets rule names nested named types as rejected
        - completed R1-F2: `just test narrows meta_schema host_function flow_sensitive shipped_schema_corpus schema_yaml_spellings catalog` 435/435 passed; `just test grammar type_scope` 319/319 passed; `cargo clippy -p darkmatter -p dmls --all-targets -- -D warnings` clean
- work completed for 'R1-F2: Compound narrowing targets admit unresolved named types' at 22:21:43
- starting the work on 'R1-F3: Invalid predicate calls still grant narrowing facts' at 22:21:43
        - defect class: `scope/facts.rs::assume` granted a predicate's fact after checking arity only (`narrowing_fact_for(args.len())`), never whether the passive call check accepts the argument bindings
        - repair at the one acceptance point: new `call_check::call_is_rejected(name, args, scope)` types each argument with `ArgumentType::of` in the current scope (which already holds the enclosing guards' facts, as `branch_scope` gives `check_expression_calls`) and asks `check_call_with_definitions(...).is_rejected()`; the `Expr::FunctionCall` arm grants the fact only when it is false. No binding rule is duplicated and no handler runs (literal arguments go through the binder, never the handler). Since `assume` works on span-erased `Expr`, a private `unlocated` re-spans the arguments with empty spans for the existing spanned typing rules
        - decision (unknown compatibility): only category 5 (arity, no overload, an argument that can never bind) withholds the fact; Unknown, Uncertain, and NullableMatch arguments keep it, because a guard whose branch runs at all has bound. Alternative rejected: withholding on anything below Compatible, which would drop valid narrowing for every guard whose subject is `unknown` (the subject itself is category Unknown against `value: unknown`, so `probe_pred(foo, 1)` would have lost its fact)
        - sweep: all three implication kinds pass through the gated arm before `predicate_fact`'s polarity rules; `!`, `?:`, `||` (`Fallback`), and `and`/`or` reach calls only by recursing into `assume`; the lazy `and`/`or` route checks arity only, which is complete because their `...values: unknown | null` parameters bind any value (comment added); `comparison` refines only stable subjects and a call is never one, so no call-granted fact there; `narrowing_fact_for` has no other production reader (`scope/tests.rs` reads only its index); `expression_type` (the call checker's typing) and DMLS `local_type` both reach facts only through `TypeScope::assume`
        - tests: `flow_sensitive_narrowing::rejected_calls::{a_rejected_call_grants_no_fact_on_either_side (F3.1-F3.3), every_implication_kind_withholds_a_rejected_calls_fact (true-iff, false-implies), an_argument_that_may_bind_keeps_the_fact (unknown/undeclared/uncertain/nullable count), a_wrong_argument_count_grants_no_fact (F3.6), a_rejected_guard_does_not_narrow_later_operands (F3.4: &&, and(), ?:, negated ?:), a_rejected_block_guard_does_not_narrow_its_body (F3.5)}`; F3.7 is the existing `registered_facts::an_omitted_optional_fact_argument_narrows_nothing`; F3.8 DMLS `expression_scopes::local_type_takes_no_fact_from_a_rejected_guard_call` (calls `dmls::overlay::expressions::local_type` on the test thread, gated on `effects-instrumentation` like the other test-hook users). Handler-entry counters assert 0. With the gate disabled, 4 of the 6 library tests and the DMLS test fail (the arity and may-bind tests are controls that pass either way)
        - docs: `docs/topics/darkmatter-expressions.md` ("Where a predicate's facts come from": accepted calls only, with a table), `docs/topics/schemas/definition.md` narrowing-facts paragraph; skill `darkmatter/expressions.md`
        - completed R1-F3: `just test flow_sensitive narrows design_time_call_check type_scope file_predicates context_function_contracts host_function expression_scopes rejected_calls local_type` 252/252 passed; `just test scope call_check guard narrow absence undeclared nullable lifecycle page_blocks when_ block predicate` 2147/2147 passed (a first run reported 1 timeout among 1842 started, cancelled; the rerun passed all 2147); `cargo clippy -p darkmatter -p dmls --all-targets -- -D warnings` and `cargo clippy -p dmls --all-targets --features effects-instrumentation -- -D warnings` clean
        - orchestrator: the subagent's first broad run hit one unidentified timeout and was cancelled; the rerun of the same filter passed 2147/2147 (recorded as load-sensitive; the phase gate below decides)
- work completed for 'R1-F3: Invalid predicate calls still grant narrowing facts' at 22:33:38
- starting the work on 'R1-F4: Narrowing failures lose required diagnostic provenance' at 22:33:38
        - defect class: declaration errors flattened provenance into message text (`(first at byte N)`) or dropped it (`InvalidFact`, `HostCatalogError`, `SchemaDocument` payload folding), so no consumer could relate the second declaration
        - library: `SchemaError::Grammar` gains `related: Vec<GrammarRelated { message, span, arm }>` (`arm` names another option of a YAML-sequence union); all 32 constructors updated, pattern matches use `..` (incl. claudine `schema/translate.rs`, `schemas_core_type_grammar.rs`); the `(first at byte N)` text is gone
        - duplicate `narrows`: primary = second keyword, related = whole first clause; sibling sweep: every repeated function constraint (`parameters`, `returns`, `fallible`, `category`) relates its first keyword, and a second `suggest` relates the first (string and YAML-list parsers); no other message embeds a byte offset (`grep "at byte"` clean; the status block's `Span:` line is a rendered field)
        - overloads (`narrowing.rs` `NarrowsProblem.related: RelatedOverload`): different facts = later clause, related = earlier clause; missing fact = declaring clause (unchanged primary), related = the whole overload without a clause (decision: it has no clause to point at; the grammar fills its span from `Parser::arm_spans`, the YAML-list parser from that option's scalar). Alternative rejected: primary on the non-declaring overload, which has no token to range
        - projection: `project_definition_failure` projects related spans through the same lockstep path into `LocatedDefinitionFailure.related: Vec<LocatedRelated>` (sibling option via `sibling_arm`); unprojectable entries are dropped, never invented
        - registration: text path `RegistrationError::Grammar { span, related }` (signature-text bytes); programmatic `InvalidFact { overloads: Vec<usize> }` (zero-based, no range); `HostCatalogError::span()/related()` located in the catalog source via `locate_standalone_definition_failure`
        - DMLS: standalone, inline `$schema`, and `type-definition` meta-type diagnostics emit `relatedInformation` from the located failure. Imports: a standalone file's payload failure reaches DMLS as `SchemaError::SchemaDocument` (message only), so the overlay locates it at assembly (`overlay::file_failures`, stored on `DocumentOverlay`/cached schema/standalone resolution) and diagnostics range into that file without reading (a first version read on the request path and failed `snapshot_tests::every_input_change_starts_a_snapshot_that_answers_from_its_own_inputs`, which forbids request reads)
        - tests: grammar `a_duplicate_clause_is_located_at_the_second_and_relates_the_first`, `a_duplicate_function_constraint_relates_its_first_keyword`; `meta_schema_phase1::{public_parser_relates_a_duplicate_narrows_clause_to_the_first (F4.1), public_parsers_relate_disagreeing_narrows_overloads_to_both_declarations (F4.5 text, string and YAML-list)}`; `host_functions::a_host_catalog_locates_a_rejected_fact_in_its_source (F4.4)`; `flow_sensitive_narrowing::registered_facts::registration_rejects_overloads_that_disagree_on_the_fact` (F4.5 text spans + programmatic `overloads == [0, 1]`; F4.7 later-overload missing parameter `overloads == [1]`); DMLS `schema_yaml_spellings::{a_duplicate_narrows_clause_relates_the_first_while_authoring (F4.2/F4.3), disagreeing_narrows_overloads_relate_both_declarations_while_authoring (quoted, block, absent, inline `$schema`, meta-type)}`, `named_type_references::a_rejected_narrows_declaration_in_an_imported_file_is_related_into_that_file` (referenced `$schema` file, `pred@./types.yaml` from Markdown and from a standalone schema); F4.6 unchanged `schema_yaml_spellings::an_invalid_narrows_clause_in_a_block_scalar_is_ranged_at_its_token`
        - docs: `docs/topics/schemas/dmls-schema-support.md` ("Definitions with two locations"), `docs/topics/schemas/definition.md` overload rule; skill `darkmatter/schema.md`
        - completed R1-F4: `just test narrows meta_schema host_function schema_yaml_spellings flow_sensitive catalog grammar diagnostics named_type schema relate` 3367/3367 passed; `cargo nextest run -p dmls` 1544/1544 and with `--features effects-instrumentation` 1546/1546; final `just test narrows meta_schema host_function schema_yaml_spellings flow_sensitive catalog grammar diagnostics named_type relate` 946/946; `cargo check --workspace --all-targets` clean; `cargo clippy -p darkmatter -p dmls --all-targets -- -D warnings` clean
- work completed for 'R1-F4: Narrowing failures lose required diagnostic provenance' at 23:17:50
- starting phase gates (full `just lint`, `just test`) at 23:17:50
        - full `just lint` first failed on the branch's existing CLI `too_many_arguments` (`cli/src/commands/render.rs`, brought in by a merge from `feat/pipeline`); fixed with `#[allow(clippy::too_many_arguments)]` like the sibling CLI commands, then `just lint` exit 0
        - full `just test`: 12960 passed, 10 skipped, exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - claudine (one `..` pattern in `composition/schema/translate.rs`): `just test schema` 1359/1359 passed
        - `just cross-check dmls --os windows`: pass (ran on the patched working tree, so no observation was published)

### Completion Report

- **outcome:** complete
- **elapsed:** 1 h 36 min (21:58:41 → 23:34:36)
- **parent findings:** total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 33 required child rows (F1.1–F1.8, F2.1–F2.6, F3.1–F3.8, F4.1–F4.7) resolved as fixed or clean; added sibling row R1-F4.8 (fixed) and nonblocking row R1-X1 (fixed)
- **verified repairs:**
        - R1-F1: every reader now applies the same `narrows` rules from schema-owned `schemas/simplified/narrowing.rs`: the public parser, standalone documents, the `type-definition` and `schema` meta-types, DMLS, the host catalog loader, and registration. Registration only lowers to a fact. Decision: grouped-union arrays are accepted as one-way targets in both the parser and registration
        - R1-F2: `narrows_target_problem` walks every `TypeExpr` variant with an exhaustive match, so a named type at any depth is rejected before any fact is built
        - R1-F3: `call_check::call_is_rejected` gates fact application in `TypeScope::assume`; only a proven rejection withholds the fact, so arguments of unknown type stay conservative
        - R1-F4: `SchemaError::Grammar.related`, `LocatedDefinitionFailure.related`, `HostCatalogError::span()/related()`, and `RegistrationError::InvalidFact.overloads` carry structured locations; DMLS emits `relatedInformation` for quoted scalars, block scalars, inline `$schema`, and imported files
- **open findings:** none
- **changed files:**
        - schema: `lib/src/markdown/schemas/simplified/{narrowing.rs (new), grammar.rs, mod.rs, source.rs, types.rs}`, `lib/src/markdown/schemas/{errors.rs, mod.rs, resolve.rs, resolve/recursive.rs}`
        - expression: `lib/src/markdown/compose/expression/{call_binding/registration.rs, call_check.rs, catalog/tests.rs, host/mod.rs, scope/facts.rs, scope/mod.rs, scope/shape.rs}`
        - DMLS: `dmls/src/diagnostics/{frontmatter.rs, named_types.rs}`, `dmls/src/overlay/mod.rs`
        - CLI: `cli/src/commands/render.rs` (lint allow only)
        - claudine: `claudine/lib/src/composition/schema/translate.rs` (`..` in one pattern)
        - tests: `lib/tests/l1/{flow_sensitive_narrowing.rs, host_functions.rs, meta_schema_phase1.rs, schemas_core_type_grammar.rs}`, `dmls/tests/l1/{expression_scopes.rs, named_type_references.rs, schema_yaml_spellings.rs}`
        - docs: `docs/topics/darkmatter-expressions.md`, `docs/topics/schemas/{definition.md, dmls-schema-support.md}`, `.claude/skills/darkmatter/{expressions.md, schema.md}`
- **final verification:**
        - `just lint` (darkmatter): exit 0 (all findings)
        - `just test` (darkmatter): 12960 passed, 10 skipped (all findings)
        - `just check-tier-coverage darkmatter`: 0 stranded (new tests)
        - `cargo check --workspace --all-targets`: clean (subagent, R1-F4's `SchemaError` shape change)
        - claudine `just test schema`: 1359/1359 (R1-F4 consumer)
        - `just cross-check dmls --os windows`: pass (R1-F4 import provenance path handling)
