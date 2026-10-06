---
feature: 2026-10-06-symbol-type-relations
deferred_perf_measurement: false
implementation_1: "2026-10-05T19:22:41-07:00"
---

# Symbol Type Relations — Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-05T19:22:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-06-symbol-type-relations/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Relation IDs are assigned without proving path meaning' at 19:22:54
        - defect class: matching a complete qualified-name string is treated as proof of a path's meaning, without enforcing its root, intermediate bindings, or all visible generic parameters
        - class sweep: written path to same-file declaration resolution; sites checked: `Declarations::resolve` (shared by every base reader: Rust supertraits, TS/JS, Python, Java, Go, C++, PHP), `base_path` anchors (Rust `::`, C++ `::`, PHP `\`), `scope_chain` (PHP namespace rule), `resolve_impl` generic evidence and its consumers (type-level `implements`, member `parent`/`container`, the type's `members` through `analysis::link_containers`), cache widening to Containment (Rust reparses through `link_relations`), `visible_type_parameters` for base lists, import bindings of the first segment, class-scope visibility (JS/TS, Python methods), inherited member types (Java, C++), `analysis::find_container_by_identity` (Go receivers, Lua tables, Swift extensions), `definitions.rs::resolve_container_kind`, `qualified_names::rust_impl_prefix`, reference binding by `NameIndex`; fixed here: `Declarations::resolve` (first segment decided by the nearest binding with no outward fallback, each further segment only through a namespace/module/type, never a function, variable or parameter), `base_path` (Rust `::` is `PathAnchor::Extern` and never resolves, C++ `::` is `Root`), `scope_chain` (PHP relative names stay in their namespace), `resolve_impl` and base lists (`type_parameters_around`: enclosing records' and enclosing Rust impls' generics), import bindings (`import_binding` for Rust/C++/PHP/Python/JS/TS, wildcard imports and C++ using-directives make a scope opaque), JS/TS class scopes skipped and Python class scopes skipped once a function was passed, Java/C++ types with bases end the lookup, `resolve_container_kind` (a Rust impl member's kind now comes from the resolver's self-type decision, none when unresolved); clean: cache widening (no separate resolver; it reparses through the fixed path), `find_container_by_identity` (Go receivers name package-level types only, Lua tables by table identity, Swift extensions are file-scope and `agrees_with_owner_path` already rejects a qualification naming another owner), `rust_impl_prefix` (qualified names and IDs deliberately unchanged), reference binding (a separate by-name contract of the bind pass, not relation targets)
        - discovered: the review's Python, TS and JS rows are all decided by records the index already held (parameters are `Parameter` records `f.Outer`/`f.ns`; a function's local class is `outer.Local` under a `Function` record), so first-segment binding plus "no path through a callable" fixes them without new node inspection
        - discovered: the same defect also arose through imports (a C++ `using other::B;`, a PHP `use Lib\ns;` in the namespace of a local `ns\B`, a Python function-local import, a TS `import Base = …` in a namespace, a Rust block `use`/glob), through C++ namespace aliases, through inherited member types (Java/C++ class with bases), and through class-scope visibility (Python method bodies do not see class names; JS/TS class bodies are not lexical); each is now a row of `PATH_MEANING_MATRIX` and fails on the old code
        - discovered: a glob import in a Rust block makes the impl's trait unresolved too (`Base` may come from the glob); recorded as the row's expectation
        - discovered: PHP `qualified_name` children carry no `name:` field for the last segment, and a statement-form `namespace N;` is a sibling of what follows, so imports are keyed by the namespace name as written
        - completed: `PathAnchor::Extern`, `starts_at_global_scope`, `import_binding`/`ImportBinding` and `impl_type_parameters` in `lib/src/file/extract/`; `binding_scope`, `is_traversable`, lazy `implied` prefixes and one lazy `FileWalk` (imports + Rust impls, replacing the separate impl walk) in `lib/src/file/tree_file/relations.rs`
        - completed: `EXTRACTOR_REVISION` 23 → 24; `cache_upgrade::entry_from_extractor_revision_twenty_three_is_not_served_after_path_meaning` (Python parameter-shadowing payload with the old wrong ID); goldens `symbols.json`/`symbols.jsonl` regenerated, only `extractor_version` changed (95 lines each)
        - completed: tests — `symbols_relations::PATH_MEANING_MATRIX` (30 labeled rows: every failing and clean row of the review table plus the sibling rows), `path_meaning_matrix_holds_on_complete_and_declarations_only_paths`, `path_meaning_impl_rows_hold_on_complete_and_method_only_queries` (complete, method-only, method-only + Containment), `symbols_cache::path_meaning_impl_rows_widen_to_containment_with_the_same_facts` (widened parse once, then a real warm `Hit` with no work); the rows also feed `declaration_fact_sources`, so the query, cache projection/widening and adapter equivalence tests run on them
        - verified: with the old `lib/src` restored, 19 rows of the base matrix and all 5 failing impl rows fail and the cache test fails; the 6 clean controls pass on both
        - completed: docs `docs/type-relations.md` (first-segment flowchart, rules table, Rust impl rows, limitations), `docs/cache-design.md` revision example, `.claude/skills/tree-hugger/SKILL.md`
        - not done: import shadowing is read only for Rust, C++, PHP, Python, JS and TS; Java and Go imports are skipped because a same-file declaration of an imported name does not compile. Python/TS lookups ignore statement order (documented limitation)
        - timing: `just test` once timed out `projected_results_equal_uncached_queries` and once `a_generic_list_beyond_u16_positions_is_an_analysis_error` at host load averages of 150–245; their CPU time old vs new was equal within noise (7.5/8.3s vs 8.1/7.1s; 7.38s vs 7.49s)
        - test results: `just test` 1,078 passed, 0 skipped; `just lint` passed for `tree-hugger` and `tree-hugger-cli`
        - follow-up fix (coordinator): `resolve_container_kind` labeled an unresolved Rust impl member's `identity.container_kind` from a same-named local declaration (`impl ::ext::B` beside `mod ext { struct B; }` gave `Type`), the same defect class; the resolver index is now built from light `Declared` entries (records, or definitions before records exist), `relations::impl_self_types` gives each impl's resolved self type, and `definitions.rs` takes an impl member's container kind from it (the resolved declaration's kind; none, the existing no-match outcome, when unresolved), so `symbols()` and records agree and link_relations still makes the same decision through the shared `resolve_self_type`
                - tests: `ImplMember.container_kind` asserted on complete, method-only and method-only + Containment queries, the older `symbols()` model, and the widened and warm cache (`::ext::B` rows: none for `probe`, `Type` for `crate::ext::B`; enclosing-generic `T` rows: none); without the `definitions.rs` change the `::ext::B` rows fail in the query and cache tests (the enclosing-generic `T` row already gave none, because the owner path `outer::T` never agreed with `T`)
                - discovered: `impl Other for &Local` members lose their borrowed `Type` kind, so the outline heading becomes `[in &Local]` instead of `[in type &Local]`; `cli/tests/symbols/outline.rs` expectations updated (its own comment already called it the textual `[in &Local]` heading); goldens unchanged; `EXTRACTOR_REVISION` stays 24 (same uncommitted change)
                - docs: `docs/type-relations.md`, `docs/symbol-schema-design.md` (container kind and the shared-context table), `.claude/skills/tree-hugger/SKILL.md` and `api-reference.md`
                - test results: `just test` 1,078 passed, 0 skipped; `just lint` passed for `tree-hugger` and `tree-hugger-cli`
- work completed for 'Relation IDs are assigned without proving path meaning' at 20:11:03
- starting the work on 'Template-template parameter defaults are dropped' at 20:11:03
        - defect class: a field stored on a nested parameter declaration is read only from the outer wrapper node (or a name nested inside a wrapper is looked for only on the wrapper), so the record silently omits it
        - class sweep: a generic-parameter field read from a wrapper instead of the nested node that holds it; sites checked: Rust `type_parameter`/`lifetime_parameter`/`const_parameter` and `where_predicate`, TypeScript `type_parameter` (`constraint`/`default_type` wrappers), Go `type_parameter_declaration`, Python `type` → `constrained_type`/`splat_type`, Java `type_parameter`/`type_bound`, C# `type_parameter` (variance tokens) and `type_parameter_constraints_clause`, C++ `type_parameter_declaration`, `optional_type_parameter_declaration`, `variadic_type_parameter_declaration`, `template_template_parameter_declaration` (plain, defaulted, variadic `class... TTs`), `parameter_declaration`, `optional_parameter_declaration`, `variadic_parameter_declaration`, concept-constrained parameters, Swift `type_parameter` (plain, bounded, `type_parameter_pack`) and `type_constraints` (after the list and inside it), Scala flat invariant and wrapped `covariant_type_parameter`/`contravariant_type_parameter` (bounds, higher-kinded `F[_]`); fixed here: C++ template-template default (read from the nested `class TT = Box` declaration that owns the name), Swift pack name (`each T` was dropped entirely; the name is now read from inside `type_parameter_pack`), Swift `where` clause written inside `<…>` (nested in the list, read only from the declaration, so its bounds and requirements were lost); clean: Rust, TypeScript, Go, Python, Java, C#, Scala, C++ ordinary/optional/variadic type and non-type parameters, variadic template-template (no default form), C++ concept-constrained parameters (the documented `Const` grammar limit; `template <std::integral T = int>` does not parse)
        - discovered: node shapes confirmed by parsing with the pinned grammars (tree-sitter-cpp 0.23.4, tree-sitter-swift 0.7.1, tree-sitter-rust 0.24.0) in a scratch crate outside the repository: `template_template_parameter_declaration` has only a `parameters` field, with the name and `default_type` on its nested `optional_type_parameter_declaration`; a Swift pack is `type_parameter(type_parameter_pack(user_type(type_identifier)))`; `<T where T: P>` puts `type_constraints` inside `type_parameters`
        - discovered (different class, fixed): a Rust attribute in a generic list (`<#[cfg(x)] T, U>`) is an `attribute_item` sibling entry and took a position, so `T` was 1 and `U` 2; positions now count parameter entries only. No other reader enumerates non-parameter siblings (Scala annotations already use `params.len()`; Java/C#/Swift attributes sit inside the entry)
        - not changed: a Swift `where repeat each T: P` requirement stays a declaration-level constraint (its subject is a pack expansion, not a bare declared parameter)
        - completed: `lib/src/file/extract/generics.rs` (`cpp_param` reads the name and default from one `declared` node; `swift_generics` reads pack names and both `type_constraints` locations; `rust_generics` filters non-parameter entries before numbering)
        - completed: tests — `symbols_generics::GENERICS_MATRIX` C++ row: `template <template <class> class TT = Box, class T = int, int N = 3>` on a struct (`Child`), a function (`build`), an alias (`Wrapped = TT<int>`), a member function (`Host::probe`) and a constructor (`Host(TT<int>)`), each asserting `TT` default `"Box"` position 0 Type with the `"int"` and `"3"` controls; Swift row: `zip<each T: Equatable, U>`, `Packed<each T>`, `older<T: Sequence where T: Hashable, T.Element == Int>`; Rust row: `Tagged<#[cfg(all())] T, U>`; new `generics_matrix_holds_on_warm_cache_hits_and_projections` asserts every matrix declaration on a warm `Hit` and on each declarations-only projection served from the cache (the existing fact-source cache tests compare cached with uncached only, which would pass if both lost a default); the extended rows also flow through `declaration_fact_sources` into the query, cache widening and adapter tests
        - verified: against the committed `generics.rs` with each fix reverted alone, the matrix and cache tests fail on `Child` (`TT` default `None`), `zip` (`T` missing), `older` (`Hashable` missing) and `Tagged` (positions 1 and 2)
        - completed: `EXTRACTOR_REVISION` left at 24 (never shipped; the 23 → 24 bump already invalidates caches); goldens unchanged (no fixture uses these forms)
        - completed: docs `docs/symbol-schema-design.md` (packs and template-template defaults, Rust attribute positions, C++ defaults and Swift bounds columns), `.claude/skills/tree-hugger/SKILL.md` generics bullet
        - test results: `just test` 1,079 passed, 0 skipped; `just lint` passed for `tree-hugger` and `tree-hugger-cli`
- work completed for 'Template-template parameter defaults are dropped' at 20:19:02
- starting the work on 'Callable requirements disappear without a direct template wrapper' at 20:19:02
        - defect class: extraction of an independently written declaration fact is gated on the presence of a wrapper node (template wrapper, parameter list) the declaration need not own, so the fact is silently dropped
        - discovered: parse dumps with the pinned grammars (tree-sitter-cpp 0.23.4, tree-sitter-c-sharp 0.23.1, scratch crate outside the repository) put a C++ trailing `requires_clause` inside the `function_declarator` in every callable shape (member `field_declaration`, in-class `function_definition`, constructor `function_definition` with no `type`, `friend_declaration` → `function_definition`, out-of-class `Child<T>::probe` definition); the template-level `requires_clause` is a child of `template_declaration` and never reaches a member whose parent is a `field_declaration_list`. C# parses `class D where T : class {}` and a `where` on a method of a list-less class without errors (the compiler rejects both, CS0080)
        - class sweep: a declaration-owned fact read only when a wrapper or list is present; sites checked: C++ `cpp_generics` template parameters, template-level `requires`, trailing `requires` (member declaration, member definition, constructor, friend, out-of-class member definition, directly wrapped function), Rust `where_clause` on a method without its own generics, Swift `type_constraints` on a member without its own generics, C# `type_parameter_constraints_clause` reader, TypeScript/Java/Go/Python/Scala generics readers, Python decorator attributes and `staticmethod` flag (`decorated_definition`), TS/JS decorator hosts, Java/C# `modifier_host`, C++ `override`/`final` modifiers, `declared_bases` (name's parent), C/C++ earlier-`static` `TemplateSignature::of` trailing constraint, doc-comment wrapper climbing; fixed here: C++ trailing `requires` (now read for every callable, wrapped or not; the template's parameters and its own `requires` still only from the direct `template_declaration` parent), C# `where` clauses (no longer skipped when the declaration has no type-parameter list); clean: Rust and Swift `where` (read independently of the list), TypeScript/Java/Go/Python/Scala (no declaration-level requirement form), template-level `requires` (never copied onto members), out-of-class member definitions and directly wrapped callables (own wrapper, already read), Python/TS/JS decorators (written on the wrapper, which is where they are read), modifiers and `declared_bases` (no wrapper gate), `TemplateSignature::of` (a trailing `requires` on a non-templated namespace-scope function is ill-formed, so no valid `static` linkage pair can differ only there)
        - completed: `lib/src/file/extract/generics.rs` (`cpp_generics` reads the wrapper's parameters and `requires` only under a direct `template_declaration` and the function's trailing `requires` always; `csharp_generics` reads `where` clauses with or without a list; module doc updated)
        - completed: tests — `symbols_generics::GENERICS_MATRIX` C++ row: `template <class T> struct Gate` with `declared()` (member declaration), `defined()` (member definition), `Gate()` (constructor) and friend `pal(Gate)`, each `["Small<T>"]` with empty `generic_params`; positive controls: `Gate` itself (`T`, no constraints), the out-of-class `Gate<T>::declared` definition (`T` and `["Small<T>"]`), `template <class T> requires Small<T> struct Bound` (`T` and `["Small<T>"]`) with its non-template member `unconstrained` (neither), plus the existing `trailing` (directly wrapped trailing `requires`) and `fits` (template-level) rows; C# row: `class Loose where T : class` (`["T : class"]`) and its method `Tight() where U : struct` (`["U : struct"]`), both with no parameters. The rows flow through the complete vs declarations-only test, the warm cache `Hit`/projection test (which asserts the empty parameter list and the requirement together) and `declaration_fact_sources`
        - verified: with the C++ fix reverted, `declared`, `defined`, `Gate` (constructor) and `pal` fail on both the complete and declarations-only paths and the warm cache test fails; with the C# fix reverted, `Loose` and `Tight` fail the same way; all controls pass with and without the fixes
        - completed: `EXTRACTOR_REVISION` left at 24 (same uncommitted change); goldens unchanged (no fixture uses these forms)
        - completed: docs `docs/symbol-schema-design.md` (new rule with a C++ example; C++ `constraints` column), `.claude/skills/tree-hugger/SKILL.md` generics bullet
        - test results: `just test` 1,079 passed, 0 skipped; `just lint` passed for `tree-hugger` and `tree-hugger-cli`
- work completed for 'Callable requirements disappear without a direct template wrapper' at 20:24:44
- starting the work on 'Three languages still omit required base relations' at 20:24:44
        - defect class: supported source forms remain unimplemented because their classification decision is unresolved, while tests assert the empty output
        - blocked: the review marks this finding as blocked on the author's decision (review frontmatter `human_review_items`; spec `## Open Questions` and `human_review_items`). No ruling on how to classify C#, Swift, and Scala mixed base lists, or how to attach Swift extension conformances, has been recorded. The spec says its recommendation "does not become required design until the author accepts" it.
        - class sweep: unresolved classification leaving supported base-list forms unimplemented; sites checked: `declared_bases` dispatcher readers for all 16 languages, `RELATIONS_MATRIX` Pending rows; fixed here: none (blocked on the author's decision); clean: Rust, TypeScript, JavaScript, Go, Python, Java, C++, PHP (readers present); C, Perl, Bash, Zsh, Lua (no required relation forms); deferred: C#, Swift, Scala
        - no code changes; the Pending matrix rows and "planned" docs still describe current behavior accurately
- work deferred for 'Three languages still omit required base relations' at 20:24:44
- final verification after all fixes:
        - `just test` (last run by the finding 3 subagent, after all code changes): 1,079 passed, 0 skipped
        - `just lint`: passed for `tree-hugger` and `tree-hugger-cli`
        - `just test-l2`: 6 passed (tmux and WezTerm executed; the two Kitty entry points returned early because that backend is unavailable, so they are not Kitty evidence)
        - `just check-tier-coverage tree-hugger`: 0 stranded tests
        - cross-OS: no OS-specific code was touched (pure syntax-tree and string logic), so no cross-check run was needed; CI covers the other environments

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1h 03m. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred (see reasons below):

- **deferred — Three languages still omit required base relations (high):** the review blocks this finding on the author's decision in its `human_review_items`. That decision is how to classify C#, Swift, and Scala base lists that mix inheritance with conformance, and how to attach Swift extension conformances. The spec says its recommended option "does not become required design until the author accepts" it, and no acceptance has been recorded. Implementing the readers now would mean guessing at that decision. Once the author rules, the work is the spec's tasks 6.5-6.7 (readers in `declared_bases`, replacing the `Pending` rows in `RELATIONS_MATRIX`, and documentation updates).

The files changed in this cycle are:

- `tree-hugger/lib/src/file/extract/relations.rs`, `tree-hugger/lib/src/file/extract/rust_impl.rs`, `tree-hugger/lib/src/file/tree_file/relations.rs`, `tree-hugger/lib/src/file/tree_file/definitions.rs`, `tree-hugger/lib/src/file/extract/generics.rs`, `tree-hugger/lib/src/file/mod.rs` (`EXTRACTOR_REVISION` 23 → 24)
- `tree-hugger/lib/tests/l1/symbols_relations.rs`, `tree-hugger/lib/tests/l1/symbols_cache.rs`, `tree-hugger/lib/tests/l1/symbols_generics.rs`, `tree-hugger/cli/tests/symbols/cache_upgrade.rs`, `tree-hugger/cli/tests/symbols/outline.rs`, `tree-hugger/cli/tests/fixtures/golden/symbols.json{,l}` (only `extractor_version` changed)
- `tree-hugger/docs/type-relations.md`, `tree-hugger/docs/cache-design.md`, `tree-hugger/docs/symbol-schema-design.md`, `.claude/skills/tree-hugger/SKILL.md`, `.claude/skills/tree-hugger/api-reference.md`

## Implementation of Review Findings #2

> **started at:** 2026-10-05T20:32:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-06-symbol-type-relations/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Python nested class scopes still assign incorrect base identities' at 20:32:14


## Author decision and Option B follow-up

The author accepted **Option B**: `extends` retains inheritance semantics,
`implements` retains conformance semantics, and `unclassified_bases` retains
written bases whose category cannot be established. Source syntax can prove
conformance without proving a same-file target ID. Swift extensions attach
only to uniquely resolved same-file owners; conformances on external or
ambiguous owners cannot be represented on a local record and are documented.

The implementation adds the third list under schema 4.2 and bumps the
extractor revision from 25 to 26. Swift relation targets carry optional
`source_span` evidence so full-signature rendering can distinguish the type's
own header from conformance added by an extension without searching text.
The original spec body remains a snapshot of the proposal; this decision
supersedes its recommendation of Option A.

### Review findings and implementation outcome

- The review-2 Python scope repair was present when this follow-up began.
  Nested classes and methods skip nonlexical enclosing class scopes, while
  a header written directly in a class body can use that body's names.
  Added a permanent revision-24 upgrade regression proving a cache entry
  with the old incorrect identity is rejected.
- Implemented tasks 6.5–6.7. C#, Swift and Scala retain written bases in the
  accepted three lists. Syntax establishes categories where possible;
  otherwise an unambiguous local declaration can classify the target.
  Imports, aliases, generic parameters, inherited member names and namespace
  roots participate in the shared conservative resolver. Constructor
  arguments remain in C# primary-constructor and Scala base text. Scala
  objects retain their parents on their Module records.
- Swift local-owner extensions add conformances without creating a duplicate
  type record. Header entries precede extension entries, and equal written
  expressions are deduplicated. Full-signature outlines use persisted source
  spans to show extension additions without repeating header conformances.
- The permanent cache robustness matrix now covers the relation collections,
  generic-parameter collections and nested fields, and nested relation
  target fields against every existing malformed-value shape. A revision-25
  upgrade regression verifies that pre-reader results are not served for the
  new language behavior. Controls use an independent sentinel record.
- Replaced pending language rows with positive and negative extraction cases;
  the shared fact corpus exercises complete and selective indexes, projection,
  widening and real warm-cache hits. Added plain/full-signature Swift outline
  cases and real-terminal relation-style checks.
- Current docs, READMEs and the tree-hugger skill describe Option B. The plan
  records the three tasks as done. The original spec body is unchanged;
  frontmatter records the accepted decision and implementation status.

### Validation

Local macOS evidence:

- `just test`: **1,092 passed, 0 skipped**.
- `just lint`: passed for both packages.
- `just test-l2`: **6 test entry points passed**. tmux and WezTerm exercised
  the new relation rendering; Kitty used the unavailable-backend skip path.
  Harness execution preserves foreground focus. The first run exposed an
  incorrect test row matcher, which was corrected before the passing run.
- Golden regeneration: **14 passed**; intended relation output and Swift
  target spans are retained alongside the current schema and provenance.
- After adding a Scala object case to the shared matrix, the relation matrix,
  declaration fact consistency, query work-counter, cache projection and
  older-model adapter checks passed again (**5 tests**).
- `just check-tier-coverage tree-hugger`: no stranded tiers.
- `git diff --check`: passed.

A concurrent cyclomatic-metrics session is using the same working tree and
advanced schema 4.2/revision 26 to **schema 4.3/revision 27**. Relation fields
are preserved in that shape, and emitters use `SchemaVersion::CURRENT`.
Validation above ran against the integrated shape. This follow-up made no
commits and did not run `cargo fmt`.

### Remaining limits and review state

An external, aliased or ambiguous Swift extension owner has no local record
on which to attach its conformances. Conditional extension requirements are
not represented as relation edge conditions. External first entries remain
unclassified when neither syntax nor an unambiguous local target establishes
their category. These are documented boundaries of the accepted behavior.

**Implementation complete, ready for review.** The feature remains active;
the author closes the review cycle and moves it when appropriate.

### Review cycle 2 complete

All three review-2 findings are implemented: the Python scope repair,
permanent cache robustness coverage, and the C#/Swift/Scala readers under
the author's accepted Option B. `review-2.md` is marked `implemented: true`.
The review's original findings and readiness judgment remain the historical
assessment; a new review will assess the repaired implementation. The feature
remains active and ready for review iteration 3.

## Implementation of Review Findings #3

> **started at:** 2026-10-05T22:17:23-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-06-symbol-type-relations/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains 5 findings (2 high, 3 medium); it is marked `recurrence: true`, so its `## Recurrence` site list is the minimum scope
- the worktree holds uncommitted, concurrent `2026-10-08-scoped-references` work (references queries, `analysis/scope/`, `schema_v2/occurrences.rs`, `tree_file.rs`, cache files); every subagent was told to preserve it and to build on top of it
- starting the work on 'Relation lookup still drops import and generic-shadowing evidence' at 22:18:08
        - defect class: failed or incomplete collection of a nearer name binding is treated as absence of that binding
        - discovered: `type_parameters_around` called the positioned reader `declared_generics` and dropped every enclosing name on its `u16` overflow `Err`; the one other caller (`records_from`) is the selected-owner error path and stays
        - discovered: the Scala `import_binding` arm read only the last plain path or the first selector group, treated every `as_renamed_identifier` (braced or not) as opaque, and returned `None` (no import at all) through `selector.named_child(1)?`; the grammar keeps a clause's comma-separated expressions as flat `path:` siblings split by `,` tokens
        - discovered: Swift selective imports, Rust `use` and C++ using-declarations also left the reader through `?` (absent) when the bound name could not be read; these now report `Opaque`
        - discovered: Rust impl consumers (`impl_header`/`impl_type_parameters`, `resolve_impl`, `impl_self_types`) read impl lists without positions and reach enclosing function generics only through `type_parameters_around`; cache widening re-runs the same analysis, so no separate site
        - completed: `generics::declared_generic_names` (names only, never numbered) beside `declared_generics`, both over one `written_params` dispatcher; `type_parameters_around` uses it and its doc states the contract
        - completed: `scala_import_names` reads every expression by grammar nodes (`path` field, `,` tokens, `namespace_selectors`, both rename nodes, `wildcard` alias = exclusion); only `namespace_wildcard`, `given` selectors or unknown nodes are opaque
        - completed: `EXTRACTOR_REVISION` 29 → 30 (no in-flight bump found); `cache_upgrade::entry_from_extractor_revision_twenty_nine_is_not_served_after_the_binding_evidence_repairs`; `docs/cache-design.md` revision example; goldens `symbols.json`/`symbols.jsonl` changed only in `extractor_version` (95 lines each); skill revision list
        - completed: docs `docs/type-relations.md` (oversized-owner shadowing row, two Scala import rows, unreadable-import limitation), `docs/symbol-schema-design.md` (unselected oversized owner), `.claude/skills/tree-hugger/SKILL.md`
        - completed: tests: 17 Scala import rows in `PATH_MEANING_MATRIX` (so complete, declarations-only, warm-hit and fact-source equivalence tests consume them); `GENERIC_BOUNDARY_ROWS` (nine readers) with `generic_boundary::<reader>::{fits_complete,fits_selective,overflows_complete,overflows_selective}`; `an_oversized_enclosing_generic_list_still_owns_its_impls`; `symbols_cache::oversized_owner_warm_hits::<reader>` and `an_oversized_owner_impl_row_widens_and_hits_with_the_same_facts`
        - test results: against the pre-repair sources, 17 new tests failed (8 `overflows_selective`, 8 warm-hit readers, the path-meaning matrix on its Scala rows); C++ and the Rust impl row passed there, matching the review's clean controls
        - discovered: each C++/C# boundary analysis takes ~7s alone in a debug build (65,536 parameter records; tree-sitter child lookups at `-O0`) and 15-20s with the other boundary tests on a loaded host, so each reader/check is its own test and `.config/nextest.toml` gives these tests the repo's earned 90s leash (CI's profile default already is 90s)
        - test results: `just test` first stopped on a compile error in concurrent scoped-references work (`Capability::Occurrences` not matched in `analysis/coverage.rs`), which that work fixed minutes later; the rerun and a no-fail-fast `cargo nextest run -p tree-hugger -p tree-hugger-cli`: **1,178 run, 1,175 passed, 3 failed**, all from the concurrent references/occurrences work (`symbols_query::distinct_unresolved_references_are_all_kept`, `symbols_query::is_recursive_is_unknown_when_semantics_run_and_absent_otherwise`, `cache_robustness::the_cache_entry_records_the_coverage_it_holds` expecting no `occurrences` capability); every new boundary, Scala, warm-hit, upgrade and golden test passed
        - test results: `just lint` fails only on `clippy::type_complexity` in concurrent `analysis/occurrences.rs:49`; `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets` reports no other warning, so this change is clippy-clean
- class sweep: failed or incomplete collection of a nearer name binding treated as its absence; sites checked: `import_binding` for Rust, C++ (using-declaration, namespace alias, using-directive), PHP, Python, JS, TS (`import`, `import x =`), C#, Swift, Scala; Java/Go (no reader by design); `type_parameters_around` (all nine generic readers); `impl_header`/`impl_type_parameters`; `resolve_impl`/`resolve_self_type`/`impl_self_types`; `written_bases` and `swift_extensions` own-list reads; `records_from` `declared_generics` error path; cache widening (re-runs the same analysis); `FileWalk` Scala block-package naming; fixed here: Scala import reader (every expression, both rename nodes, exclusions not opaque), Swift/Rust/C++ `?` exits now `Opaque`, `type_parameters_around` via `declared_generic_names`; clean: Rust `rust_use_names`, C++ namespace alias/directive, PHP, Python, JS/TS, C#, impl readers (no positions), `written_bases`/`swift_extensions` (own list only on selected records, whose overflow already errors), `records_from` (explicit `too_many_generic_parameters` kept)
- work completed for 'Relation lookup still drops import and generic-shadowing evidence' at 22:43:54
- starting the work on 'Built-in name exclusions erase valid Go and Swift bases' at 22:44:52
        - defect class: a spelling-based non-relation exclusion deletes a written base before any binding evidence proves the spelling means the built-in (shadowing declarations, aliases, imports and failed resolution are ignored)
