---
implementation_1: "2026-10-05T15:49:34-07:00"
---

# Deferred Kind Precision — Review/Implement Log

## Implementation of Review Findings #1

> **started at:** 2026-10-05T15:49:34-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-05-deferred-kind-precision/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the working tree already held uncommitted edits from a concurrent session (doc-comment extraction, `EXTRACTOR_REVISION` = 15, cache-upgrade row for revision 14); they are preserved and built upon
- starting the work on 'Grouped Scala fields disappear or lose their type annotations' at 15:49:56
        - defect class: extraction paths assume a declaration has one name, silently omitting grouped definitions and dropping the shared type from later names in grouped declarations
        - class sweep: extraction paths assume a declaration has one name; sites checked: Scala `locals.scm` `val`/`var` definition and declaration patterns, `extract_scala_body_fields`, the `classify.rs` Scala Field arm, `resolve_declaration_node`, header `keyword.rs` (Scala `val`/`var`), the `signature_text` cut, doc comments, Scala visibility and static readers, `symbol_scope`, `align_field_metadata`, Scala class-parameter/enum-case/binding/lambda patterns, every `child_by_field_name("name"/"pattern")` reader in `type_metadata.rs`/`signatures.rs`/`qualified_name.rs`/`visibility`/`static_flags`, and the Go/C/C++/Java/C#/PHP/Swift/Rust/Python/TS/JS field readers; fixed here: Scala `locals.scm` (definitions under `identifiers`), `extract_scala_body_fields` (every name, shared type); clean: classification and declaration resolution (they climb through `identifiers`), header keyword and `signature_text` (the shared header `val a, b: Int` contains each name, like C `int a, b`), docs, visibility, static, scope, `align_field_metadata`, and every other language's field reader (each reads a node that carries one name, or already enumerates)
        - initialized grouped definitions (`val a, b: Int = 1`) were never captured in class, trait, object or enum bodies, for `val` or `var`; grouped declarations without an initializer gave the type only to the first name
        - fixed: the Scala query captures every `identifier` under `identifiers`; `extract_scala_body_fields` lists every name with the declaration's type; provenance note updated
        - object fields are static; class, trait and enum fields are instance; each name has its own name span and shares the keyword, header and doc comment; tuple patterns (`val (a, b) = …`) are still not captured
        - behavior change: a method-local grouped `val x, y = 1` used to produce no symbols; each name is now a local Variable, like a single-name local `val`
        - new grammar limit found (tree-sitter-scala 0.24.0, no syntax error): a grouped declaration without an initializer followed on the next line by an unmodified untyped `val`/`var` parses as one definition with infix type `Int var z`; recorded in the Grammar limits section of `docs/symbol-kinds-and-visibility.md` (`GRAMMAR_LIMITS` only accepts snippets with a syntax error)
        - tests added (L1, inline sources, no fixture or golden change):
                - `symbols_kinds`: 24 `DEFERRED_KIND_ROWS` rows (class/trait/object/enum, `val`/`var`, initialized/declared; these also feed the narrow-query and cache tests), tuple/local negatives, `scala_grouped_locals_are_variables_like_single_ones`
                - `symbols_field_lists::scala_grouped_body_fields_are_listed_with_their_shared_type`
                - `symbols_static::scala_grouped_fields_take_their_owners_static`
                - `symbols_header::scala_grouped_names_share_their_declarations_header`
                - CLI `classes::scala_grouped_field_names_keep_their_shared_type`
        - docs and skill updated: `docs/symbol-kinds-and-visibility.md`, `.claude/skills/tree-hugger/SKILL.md`, `query-system.md`
        - `just test`: 1,009 passed, 0 skipped; `just check-tier-coverage tree-hugger`: passed
        - `just lint` failed only on `clippy::type_complexity` in `symbols_flags.rs` (`TRAILING_DOC_CASES`), which is the concurrent session's uncommitted edit; that session has since added a `DocFacts` alias, and lint is re-run at the end of this cycle
- work completed for 'Grouped Scala fields disappear or lose their type annotations' at 15:59:12
- starting the work on 'Declarator attributes turn C++ constructors into methods' at 15:59:12
        - defect class: a C++ declaration reader assumes the declared name sits directly under the function declarator, where a valid declarator can wrap the same name in an attribute or in parentheses
        - class sweep: declarator wrappers hidden from a declaration reader; sites checked: `classify.rs` (constructor detection, captured-function-to-Method rule, destructor/operator exclusions), `declaration.rs` (`declaration_for_context`, `implied_c_function_context`), header `keyword.rs` and the signature cut, `visibility/c.rs` (`declarator_layers`, linkage), `static_flags/cpp.rs`, `type_metadata.rs` (`c_field_name`), `signatures.rs` (`find_c_param_name`), `scope.rs`, `qualified_name.rs`, C++ `locals.scm`, `attributes.rs` (`declaration_attributes`), `flags.rs` (`is_deprecated`, `has_test_attribute`), header `is_annotation`, doc comments, the `attributes` facet, and the Python/PHP/Java/C#/Swift/JS/TS constructor detectors; fixed here: the `classify.rs` C++ constructor rule (attributed and parenthesized names, via a new `transparent_declarator` in `c_declarators.rs` over `declared_function_named`), `declaration_attributes` (attributes on the declarator-id of the captured name, via a new `declarator_id_attributes`; reaches `is_deprecated` and `has_test_attribute`); clean: every other site listed
        - the constructor rule asks the shared declarator code which function the declaration declares and accepts it only when that function's name is the captured `identifier` with only attributes or parentheses between; return types, non-members, `~C`, `operator T`, `C::C` and pointer/reference/array wrappers are still excluded, and the name is never compared with the class name
        - sibling found beyond the review: the parenthesized form `(C)(long y);` (valid C++, clang accepts it) was also a Method; now a Constructor
        - sibling found beyond the review: `C [[deprecated]] (int x)`, `int m [[deprecated]] (int x)` and `int a [[deprecated]], b;` reported `is_deprecated: false` (spec: constructors retain attributes); now true for exactly the named entity (`b` stays false); type-position attributes (after a parameter list, an array bound or a returned pointer's parentheses) stay false per the C++ standard; `declaration_attributes`, `is_deprecated` and `has_test_attribute` now take the captured name from `records_from`
        - C is unchanged: tree-sitter-c 0.24.1 parses C23 `[[deprecated]]`, but C has no deprecation marker in the spec or docs; three rows pin C as false
        - full `--json-full` records of each plain form and its attributed/parenthesized spelling match on kind, scope, container, access, `static`, parameters, docs and arity
        - out-of-class `C::C [[deprecated]] () {}` stays a container-less Function, matching plain `C::C() {}`
        - tests added (L1, inline sources, no fixture or golden change):
                - `symbols_kinds`: `CPP_WRAPPED_CONSTRUCTORS` (the six review forms plus `(C)(int x);`), 7 Constructor rows and 1 attributed Method row in `DEFERRED_KIND_ROWS` (also feed query and cache comparisons), 5 `DEFERRED_KIND_NEGATIVES` rows (out-of-class, attributed destructor, attributed conversion operator, attributed method, parenthesized method)
                - `symbols_deferred_metadata`: 7 `DEFERRED_METADATA_ROWS` (container, `C::C`, Public, not static, header, parameters)
                - `symbols_declarator_wrappers::cpp_attributes_after_the_name_deprecate_the_declared_entity` (24 rows)
                - CLI `filters::wrapped_cpp_constructor_names_select_as_constructors` (`--kind constructor` selects all three spellings; `--kind method` selects only `m`)
        - the kind-table and CLI tests fail with the old `classify.rs` restored and pass with the fix
        - docs and skill updated: `docs/symbol-kinds-and-visibility.md` (C++ kind, constructor and deprecation rows), `.claude/skills/tree-hugger/SKILL.md`
        - two runs each had one 30 s timeout in an unrelated CLI test while the host load average was about 170 (other sessions); each passed alone and the full rerun passed
        - `just test`: 1,014 passed, 0 skipped; `just lint`: passed; `just check-tier-coverage tree-hugger`: passed
- work completed for 'Declarator attributes turn C++ constructors into methods' at 16:18:14
- starting the work on 'Scala enum cases lose representable payload types' at 16:18:14
        - defect class: enum-list completion synthesizes unit variants for parameterized declarations without reading payload data the schema can retain
        - class sweep: enum-list completion synthesizes unit variants without reading payload; sites checked: the Rust, TypeScript, C, C++, C#, PHP, Java, Swift and Scala (brace and indentation syntax) enum-variant readers, `align_variant_metadata`/`reconcile_variants`; fixed here: Scala `extract_scala_enum_cases`; clean: Rust, TypeScript, C, C++, C#, PHP, Java, Swift (owner is Type, no list), reconciliation (it never replaces a reader's payload entry with a unit one; it fills only a missing discriminant or doc)
        - root cause: the Scala reader looked for `simple_enum_case`/`identifier` directly under the enum body, but the grammar always nests cases in `enum_case_definitions`, so it listed nothing and alignment synthesized a unit entry for every case
        - the reader now walks `enum_case_definitions`: a simple case is a unit variant; a full case lists its class parameters as named payload fields (`struct_fields`) with their types as written (generic `T` and function types included); defaults such as `x: Int = 1` are never discriminants
        - decision: multiple parameter clauses are flattened in order, `using` clauses included (`case V(x: Int)(using o: Ord)` lists `x`, then `o`), matching how `parameters` treats curried Scala lists; documented
        - grammar ambiguity: `case U(using Int)` parses as a parameter named `Int` with no type; listed that way and documented
        - unchanged: case parameters keep their Parameter records (container is the enum); standalone EnumVariant records keep the `Unknown` payload
        - `EXTRACTOR_REVISION` bumped for this cycle's record changes; a concurrent session bumped it once more for its Go doc fix, so it is now 17, with `StaleRevision` rows for 15 (this cycle) and 16 (concurrent) in `cache_upgrade.rs`; skill and `docs/cache-design.md` name 17
        - goldens regenerated with `HUG_UPDATE_GOLDEN=1`; the only diff is `extractor_version` (the multi fixture has no parameterized Scala cases)
        - tests added:
                - `symbols_field_lists::enum_variant_lists_keep_declared_payload_types` (payload contents on `symbols()`, on `symbol_index_v2()` records, after the adapter round trip, and on an enum-only narrowed query equal to the full one; brace and indentation enums; Rust controls)
                - a Scala row in `symbols_field_lists::enum_variant_lists_keep_discriminant_text`
                - `symbols_cache::warm_enum_case_payloads_equal_uncached_results` (cold miss, then `CacheLookup::Hit` with no parse, equal to uncached for the complete index and the enum-only query)
                - CLI `filters::json_enum_variant_lists_keep_scala_case_payloads`
                - CLI `cache_upgrade::entry_from_extractor_revision_fifteen_is_not_served_after_the_repairs`
        - docs and skill updated: `docs/symbol-kinds-and-visibility.md`, `docs/symbol-schema-design.md`, `docs/cache-design.md`, `.claude/skills/tree-hugger/SKILL.md`
        - host load average was 176–301 from other sessions; several unrelated tests hit the 30 s nextest timeout and each passed alone
- work completed for 'Scala enum cases lose representable payload types' at 16:39:09
- final verification by the orchestrator, from `tree-hugger/`: `just lint` passed for `tree-hugger` and `tree-hugger-cli`; `just test` 1,021 run, 1,021 passed, 0 skipped; `just check-tier-coverage tree-hugger` 0 stranded

### Successful Completion

The implementation of review cycle 1 has completed successfully in 49 minutes (15:49:34 → 16:39). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no review finding was deferred
- two observations outside the three defect classes were not changed; neither is a requirement of the specification:
        - Scala enum-case doc comments are not extracted onto the EnumVariant records; this predates the feature, the spec names no variant doc requirement, and the reader (`doc_comments.rs`) was being edited by a concurrent session
        - `extract_scala_class_parameters` matches parameter types by node kind and misses function and tuple types; the spec requires class parameters to retain their existing field-list behavior
- C23 `[[deprecated]]` is not read in C: C has no deprecation marker in the spec or docs; adding one is an author decision

The files changed in this cycle are `tree-hugger/lib/queries/vendor/scala/locals.scm`, `tree-hugger/lib/src/queries/provenance.rs`, `tree-hugger/lib/src/file/extract/{type_metadata,classify,c_declarators,attributes,flags}.rs`, `tree-hugger/lib/src/file/tree_file.rs`, `tree-hugger/lib/src/file/mod.rs`, the L1 tests listed above, `tree-hugger/cli/tests/symbols/{classes,filters,cache_upgrade}.rs`, the two goldens, `tree-hugger/docs/{symbol-kinds-and-visibility,symbol-schema-design,cache-design}.md`, and `.claude/skills/tree-hugger/{SKILL,query-system}.md`.
