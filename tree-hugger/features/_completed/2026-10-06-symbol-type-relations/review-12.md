---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-06T15:35:46-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: false
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-12.md
previous: 2026-10-06-symbol-type-relations/review-11.md
---

# Review of Symbol Type Relations — Iteration 12

The feature is **ready for production within its accepted scope**. Review 11's C# generic-arity repair reaches candidate collection, bounded summaries, outward lookup, final targets and intermediate members. Its required query, cache, classification and work-counter consumers pass. No remaining required repair, regression or new blocking finding was reproduced. No additional human decision is required.

The accepted behavior keeps inheritance in `extends`, conformance in `implements`, and uncertain written bases in `unclassified_bases`. Swift extensions attach only to a uniquely identified same-file owner. These boundaries were accepted before the mixed-list readers were implemented; they are not unfinished work.

## Previous Finding Verification

The earlier reviews do not assign stable finding or child IDs. Their identifiers below therefore retain **review filename plus original title**. The mixed-reader finding retains its review-1 identity when carried through review 2. This table includes the entire historical inventory, including review 1's single Findings section, rather than treating later completion flags as proof. “Verified fixed” applies to the enumerated instances and consumers, not a claim that every conceivable source program is resolved.

Evidence names refer to the tree-hugger library's [relation matrices and shared corpus](../../lib/tests/l1/symbols_relations.rs), [generic matrix](../../lib/tests/l1/symbols_generics.rs), [owner-identity matrices](../../lib/tests/l1/symbols_owner_identity.rs), [query consumers](../../lib/tests/l1/symbols_query.rs), [cache consumers](../../lib/tests/l1/symbols_cache.rs), [relation work matrix](../../lib/tests/l1/symbols_relation_work.rs), and the CLI's [public cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs) and [outline checks](../../cli/tests/symbols/outline.rs). All cited test consumers executed in this review's passing Level-1 run.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| review-11.md — C# relation lookup discards generic arity before selecting the target | Six type forms, partial and ordinary declarations, arities 0/1/2; bare, namespace-qualified, nested, rooted, anchored final/intermediate targets; outward lookup from unique/anchored types and namespaces; named intermediate and unique controls; same-arity ambiguity, wrong counts, aliases and generic barriers; complete/selective output, projection, widening, actual hits, classification, bounded work, docs and invalidation | **verified fixed** | All 117 `CSHARP_WRITTEN_ARITY_ROWS`, their coverage guard and shared consumers pass. Five new work shapes assert exact IDs or required unresolved results at both sizes. Independent 144-source CLI sweep verifies 720 bases, including deeper paths. Revision-39 stale-entry test passes; current docs describe the repaired lookup. Detailed decision sweep below. | None |
| review-10.md — C# partial-owner merging ignores type identity | Category, arity and enclosing-owner identity; all six forms, 30 directed category pairs, compatible fragments, own/inherited members, outside paths, namespace collision, generic and non-partial controls; parent/container/membership and query/cache consumers | **verified fixed** | All 70 `CSHARP_PARTIAL_IDENTITY_ROWS` and the source-contained-membership test pass. Owner groups include category, arity and enclosing group; compatible fragments share members while distinct owners remain separate. Four existing partial-identity work shapes and shared consumers pass. Review 11's formerly missing arity-selection siblings are independently closed above. | None |
| review-10.md — Aggregate declaration-fact tests exceed the normal execution budget | Query equivalence; warm hits/projection; Containment/Metrics widening; counter and adapter consumers; older-schema control; full row registration and normal-concurrency execution | **verified fixed** | Five full-corpus consumers each execute seven registered groups; two path consumers execute six. The partition guard checks exact registration and every source once. Normal `just test` passes without retries or timeout changes. The largest query-equivalence, projection and widening group takes 5.824, 5.389 and 4.949 seconds respectively, below the ordinary 30-second limit. | None |
| review-9.md — Binding and scope checks still combine distinct source owners | Owner-local imports, wildcards, unrecorded bindings, inherited-name barriers and members across Rust/Python/Scala/Java/Swift/C#/C++; namespace/partial controls; four Rust impl consumers; all 30 Swift raw-type spellings; actual-owner negatives, generic and lexical controls; query/cache/containment | **verified fixed** | `SOURCE_OWNER_MATRIX`, `SWIFT_SIBLING_SHADOW_ROWS`, source-owner coverage guard and all shared consumers pass. Source anchoring precedes scope-fact lookup; owner facts and members use owner-group identity. The C++ inherited-sibling case that the repair ledger corrected from a claimed clean control is registered and passes. | None |
| review-8.md — Relation and membership decisions discard unresolved owner evidence | Canonical C++ inline/direct/reopened namespace identities; repeated Rust modules and Scala objects; anchored members and Rust self/trait/empty/inherent impls; Swift ambiguous/alias/import/external owners, generic uncertainty, 30 raw spellings, lexical controls; conformance, container, parent, membership and cache widening | **verified fixed** | `OWNER_AMBIGUITY_MATRIX`, `SWIFT_UNPROVED_OWNER_ROWS` and `UNPROVED_OWNER_MATRIX` pass through exact-target, method-only, widening and genuine-hit consumers. An unproved Swift extension owner supplies an unknown context and textual ownership; containment does not select a nearby same-named type. | None |
| review-8.md — Inline namespace paths still repeat broad candidate scans | Qualified/rooted/bare/deeper distinct suffixes below ambiguous owners; explicit and ordinary paths; repeated final names, struct-owner negatives; all summary, traversal and binding consumers | **verified fixed** | Registered shared-intermediate work shapes pass exact unresolved/resolved assertions and counted growth bounds. Inline lookup records ambiguity before suffix traversal instead of carrying every competing owner into each lookup. | None |
| review-7.md — Swift nested-type extensions lose enclosing generic bindings | Outer/middle/own generic layers, qualified bases, methods and closures, extension conformance, all 30 raw spellings, ordinary/nearer-lexical/header controls; complete/selective/cache/containment/outline | **verified fixed** | `SWIFT_NESTED_GENERIC_ROWS`, oversized-owner cache checks and dedicated Swift widening checks pass. `owner_generic_context` follows the resolved owner's actual ancestors, is shared by owner identity, and does not copy those parameters into child metadata or expose ordinary owner members in extension headers. Outline regressions pass. | None |
| review-7.md — Inline namespace and generic processing still repeat input-sized scans | Every inline spelling, ambiguity and explicit-path control; both Swift extension context consumers; Rust/C#/Swift bound attachment including both Swift clause locations; candidate, parameter and set work accounting | **verified fixed** | Relation/shared-context work matrices and generic-bound growth tests pass. Indexed inline names, memoized owner layers and subject-to-parameter indexes remove the enumerated repeated scans; work counters include their visits. Exact results remain asserted alongside growth. | None |
| review-6.md — Relation lookup still loses binding and owner evidence after selecting the first name | Python named/local-owner imports, deletion, assignment/pattern/member writes, redirected declarations and two-/three-segment paths; Swift closure-shadowed nested extension ownership; conformance, member-only, containment, widening, warm hits and outline | **verified fixed** | Generated `PYTHON_MEMBER_ROWS`, `NESTED_SHADOW_MATRIX`, member-binding widening checks and outline tests pass. Every path continuation checks the chosen owner's bindings; redirected declarations do not become members; Swift descendants are selected by source ownership. | None |
| review-6.md — Relation processing exceeds the bounded per-file work contract | Record-less Rust/TS owners, repeated syntax-owner members in C++/Rust/Java/Swift/Scala/Python, reopened namespaces, base deduplication and nested inline spellings; exact results and one-query/one-parse guarantees | **verified fixed** | Registered repeated-name, deduplication, inline-depth and query-counter tests pass. Owner/member indexes and bounded summaries are built per file; neither source nor linker deduplication scans a growing result list for each entry. | None |
| review-5.md — Relation lookup loses binding evidence between syntax scopes and display-name indexes | Local-owner two-/three-segment paths; Rust local modules and impl self/trait/member consumers; record-less parameter scope, value/type categories; Python global/nonlocal write families; PHP function/closure/method/conditional class binding; complete/selective/widening/hit/outline controls | **verified fixed** | Registered path rows assert exact IDs by declaration position, absent IDs and category. Method-only and Containment-widening checks pass. Syntax-owner continuation, scoped/category-preserving unrecorded bindings, directive destinations and PHP binding namespaces retain the evidence needed by later lookup. | None |
| review-4.md — Relation lookup still substitutes record names for source binding evidence | All source lexical boundaries and declaration positions; type/value groups; JS/TS/Python/Scala patterns; anonymous generic and inherited owners; Swift extension context; Rust impl and PHP lookup-site consumers; cache and outline | **verified fixed** | Permanent block, pattern, name-group, anonymous-owner and import rows in `PATH_MEANING_MATRIX` pass, including the shared JS/TS parameter-environment controls. Complete/declarations-only assertions provide expected identities, not just cache equivalence. | None |
| review-4.md — Attribute and modifier members lack permanent cache robustness columns | Attribute name/value; Custom tag/payload and fixed Static tag; nine shapes per member, extraction controls, public service/recomputation | **verified fixed** | All 45 member-matrix cells pass, including raw nonempty arguments, Custom payload and Static extraction controls. Optional argument absence/null is explicitly a None contract; required names/tags/payloads reject malformed shapes. | None |
| review-3.md — Relation lookup still drops import and generic-shadowing evidence | Every Scala import expression/selector/rename/exclusion/wildcard; ordinary import-reader controls; all generic readers' selected and unselected oversized-owner boundaries; complete/selective/hit output | **verified fixed** | Registered import rows and `GENERIC_BOUNDARY_ROWS` pass, including 65,536/65,537-position boundaries and oversized-owner hits. Scala collects every expression; unnumbered generic names remain binding evidence even when an owner is unselected. | None |
| review-3.md — Built-in name exclusions erase valid Go and Swift bases | All 19 Go names; local interface, alias and function scope; Swift simple/qualified raw spellings, protocol/alias/import shadows, standard raw controls; classification and caches | **verified fixed** | Per-spelling path rows, raw-type controls and shared consumers pass. Exclusion follows visible binding evidence through `may_bind`, rather than spelling or failure to obtain an ID. | None |
| review-3.md — C# record superclasses remain unclassified | Implicit/explicit/positional/generic/qualified record classes; ordinary class/interface, struct/record-struct, alias/duplicate and owner-syntax controls; warm output and outline | **verified fixed** | Record-class path and mixed-list rows pass. `declares_class` reads target declaration syntax without changing broad Type kinds; the new all-form arity cases and independent CLI sweep retain the same classification. Outline checks pass. | None |
| review-3.md — Current schema examples advertise obsolete versions | CLI README envelope; report envelope/record and field table; cache entry; historical-number controls, source constants and skill | **verified fixed** | Current examples advertise schema 5.0, matching `SchemaVersion::CURRENT` and CLI output. Earlier 4.x numbers are explicitly version history. Schema rejection and current fingerprint/record tests pass. | None |
| review-3.md — New relation source-span members lack permanent robustness columns | Start/end line and column, start/end byte; whole span and all relation-list consumers; full-signature outline suppression | **verified fixed** | Six required member columns and whole-object cells pass in the public CLI matrix. Full-signature Swift extension outline checks retain added conformance while suppressing header duplicates using the byte evidence. | None |
| review-2.md — Python nested class scopes still assign incorrect base identities | Bare/qualified nested bases, outer type/value/import and absent-module cases, enclosing function, own-body/direct-header/method controls; query and warm output | **verified fixed** | All nine Python scope rows and other-language controls are permanent in the path corpus and pass. Lookup skips nonlexical enclosing class bodies but preserves the immediate body and enclosing function/module. | None |
| review-2.md — Generic and base fields lack a permanent cache robustness matrix | Generic collection and six members, declaration constraints, extends/implements/unclassified collections, relation target members; controls and every malformed shape through public output | **verified fixed** | The format-wide CLI matrix has all required columns and passes. A separate sentinel distinguishes service from recomputation; accepted edits are asserted as public values. See the robustness tables below. | None |
| review-1.md — Relation IDs are assigned without proving path meaning | Rust roots/enclosing generics and impl consumers; JS/TS/Python lexical/parameter paths; C++ root/intermediate names; PHP relative/root/import paths; Java/Go and absent-language controls; method-only/widening | **verified fixed** | Original path rows, Rust method-only and widening checks pass. Root anchors, first-segment bindings and syntax-owner continuation preserve exact target identity or required unresolved text. Later discovered siblings are accounted for individually above. | None |
| review-1.md — Template-template parameter defaults are dropped | C++ struct/function/alias/method/constructor owners; ordinary type/non-type defaults; other-language controls; complete/declarations-only/projection/hit | **verified fixed** | `GENERICS_MATRIX` contains all five template-template owners and mixed default controls. The nested parameter declaration supplies both name and default; generic, query and cache assertions pass. | None |
| review-1.md — Callable requirements disappear without a direct template wrapper | C++ declarations/definitions/constructors/friends/out-of-class definitions and wrapped controls; Rust/Swift/C# list-less requirements; owned-list absence, narrowing and hits | **verified fixed** | Generic matrix requirements remain independent of an owned parameter wrapper. Public results assert written constraints and empty owned generic lists; shared query/cache consumers pass. | None |
| review-1.md — Three languages still omit required base relations; carried in review-2.md | C#/Swift/Scala mixed readers, raw/uncertain/local/external categories, Scala objects and constructor text, Swift extension aggregation and source-span-aware rendering; selective/cache consumers | **verified fixed** | **Unblocked before repair** by the author's accepted choice in [the repair log, “Author decision and Option B follow-up”](./log.md:102), also recorded in the spec frontmatter. All three language rows are implemented rather than Pending and pass. External/unproved Swift extension owners remain the accepted boundary. The original implementation log's earlier “Open” state is superseded by this acceptance and follow-up, not treated as a current blocker. | None |

## Repair Decision and Consumer Sweep

The C# argument count now remains an input to each relevant decision in the tree-hugger library. The following checks cover both missing required targets and incorrect extra targets.

| Site | Shape tested or precise invariant | Observed and expected result | Verification evidence |
|---|---|---|---|
| Written path and declaration collection | Generic counts on every segment; six forms, namespace zero, partial identity category/enclosing owner | Counts stay aligned when `global::` is removed; declarations distinguish arities before lookup | [Path reader](../../lib/src/file/extract/relations.rs:125), `csharp_arity`, partial/written-arity matrix results |
| Candidate summaries and truncation | Three arities coexist; multiple candidates of the same arity | `ByArity` indexes counts before `Few` limits candidates; another arity cannot remove a unique matching target; same-arity competition stays unresolved | [ByArity](../../lib/src/file/tree_file/relations.rs:460), named summaries, direct-member index; new work shapes |
| Outward scope selection | Nearer wrong-arity declaration in unique type, anchored type or namespace; also a wrong-arity first qualifier | Continue to the matching outer declaration; nearer matching arity still wins; no outer match stays unresolved | [binding_scope](../../lib/src/file/tree_file/relations.rs:1633), `is_bound`; registered rows and supplemental outward-qualified cases |
| Final named target | Bare, namespace-qualified, nested and rooted families | The one matching-arity declaration supplies the ID and in-file name; class/record class gives extends, interface gives implements | [named_written](../../lib/src/file/tree_file/relations.rs:1279), final `resolve_path` branch; complete/selective/warm assertions |
| Anchored final and intermediate members | Competing outer owners, each with an overloaded member family; deeper paths selecting three generic owner/target levels | Read the actual owner group's matching-arity member before checking uniqueness; no sibling borrowing or missing provable member | [members](../../lib/src/file/tree_file/relations.rs:1311), both member branches of `resolve_path`; exact source-line target assertions |
| Named intermediate owners | Arity-specific owner groups and matching partial fragments; absent/wrong count and duplicate-owner controls | Enter the group proved by the written count; an unproved group stops continuation | `written_group` and `is_traversable`; named-intermediate controls and independent deep-path cases |
| Lexical, alias and generic barriers | C# has no function/block-local type declarations; imports/type aliases/type parameters remain barriers under the accepted conservative rules | No arity-based alias expansion or parameter-to-type inference is introduced; non-C# `declared_arity` is None, preserving their candidate selection | Final `arity_fits` guard, registered alias/parameter negatives; all prior language matrices |
| Category and rendering | Six target forms and written counts; resolved class/interface, uncertain struct targets; full/none/short signatures | Correct category and complete written text survive reports; outline consumes those lists without presenting method conformance as inheritance | `declares_class`, [Renderer::declared_bases](../../cli/src/render.rs:418), [header_writes](../../cli/src/render.rs:1010); CLI outline and real-terminal capture |
| Query, cache and invalidation | Complete/declarations-only/selective kinds; projection and Containment/Metrics widening; actual complete/type-only hits | Same independently expected facts on every path; genuine hits perform zero parses; revision-39 entries recompute | Shared corpus consumers, stale-entry CLI test and independent CLI sweep |
| Work and ownership consumers | Several arities under repeated owners, root/outward/reopened-namespace paths; Rust impl and Swift extension consumers | Candidate work remains bounded; Rust/Swift ownership and ordinary C# source containment remain correct | Five new arity work shapes, older relation-work rows, method-only/owner matrices and one-parse/one-capture counters |

The repaired C# finding is closed for its enumerated instances. No unchanged failing reproduction, repair regression or newly discovered sibling remains to carry forward.

## Unblocked Findings

None.

## Blocked Findings

None. The accepted mixed-list decision is not reopened, and no cross-OS receipt or human sign-off is being treated as an implementation defect.

## Recurrence

No current finding repeats an earlier defect class, so `recurrence: false`. Earlier reviews repeatedly found losses of source-binding or owner-identity evidence. Their listed stages and downstream consumers are reconciled above; this review does not infer class closure solely from passing an earlier minimal example.

## Input Robustness Matrix

The affected serialized input is the **persistent JSON symbol cache**. This feature adds no manifest, lockfile, YAML, TOML or configuration reader. Source-language syntax is checked separately by the extraction matrices.

The following contracts are exercised by the CLI's [format-wide matrix](../../cli/tests/symbols/cache_robustness.rs:865) and [attribute/modifier member matrix](../../cli/tests/symbols/cache_robustness.rs:981). Each starts from a CLI-written fixture, restores a populated control, changes one field per cell and checks the public result. A separate renamed sentinel identifies served entries; rejected entries must produce the uncached result. Some optional fields are populated in the stored control because that fixture does not write them. These cells verify decoding contracts, not the source validity of arbitrary edited IDs or spans.

**S(value)** means served with that public value; **R** means rejected and recomputed. “Control” is the populated original value for that column. Absent collections explicitly encode empty collections in this format; null collections are rejected. Optional scalar absence and null intentionally mean None. Required metrics keys must exist even when their value is null.

### Feature collections

| Shape | generic_params | declaration constraints | extends | implements | unclassified_bases | attributes | modifiers |
|---|---|---|---|---|---|---|---|
| Unedited/control | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) | S(control) |
| Absent | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Null | R | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R | R |
| One wrong element | R | R | R | R | R | R | R |
| Every element wrong | R | R | R | R | R | R | R |
| Empty | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) | S([]) |
| Duplicate key | R | R | R | R | R | R | R |
| Invalid content | R | R | R | R | R | R | R |

### Generic-parameter members

| Shape | name | kind | constraints | default | variance | position |
|---|---|---|---|---|---|---|
| Control | S(T) | S(Type) | S([MatrixBase]) | S(MatrixBase) | S(out) | S(0) |
| Absent | R | R | S([]) | S(None) | S(None) | R |
| Null | R | R | R | S(None) | S(None) | R |
| Wrong whole type | R | R | R | R | R | R |
| One wrong element | R | R | R | R | R | R |
| Every element wrong | R | R | R | R | R | R |
| Empty | S("") | R | S([]) | S("") | S("") | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Invalid content | R | R | R | R | R | R |

### Relation-target members

| Shape | name | id | qualified_name | source_span |
|---|---|---|---|---|
| Control | S(MatrixBase) | S(matrix-id) | S(MatrixBase) | S(control span) |
| Absent | R | S(None) | S(None) | S(None) |
| Null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| One wrong element | R | R | R | R |
| Every element wrong | R | R | R | R |
| Empty | S("") | S("") | S("") | R |
| Duplicate key | R | R | R | R |
| Invalid content | R | R | R | R |

### Relation-span members

| Shape | start.line | start.column | end.line | end.column | start_byte | end_byte |
|---|---|---|---|---|---|---|
| Control | S(2) | S(3) | S(4) | S(5) | S(6) | S(7) |
| Absent | R | R | R | R | R | R |
| Null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| One wrong element | R | R | R | R | R | R |
| Every element wrong | R | R | R | R | R | R |
| Empty/zero | S(0) | S(0) | S(0) | S(0) | S(0) | S(0) |
| Duplicate key | R | R | R | R | R | R |
| Invalid content | R | R | R | R | R | R |

### Attribute/modifier members

| Shape | Attribute.name | Attribute.value | Custom.type | Custom.value | Static.type |
|---|---|---|---|---|---|
| Control | S(ReviewAttr) | S(raw argument) | S(Custom) | S(final) | S(Static) |
| Absent | R | S(None) | R | R | R |
| Null | R | S(None) | R | R | R |
| Wrong whole type | R | R | R | R | R |
| One wrong element | R | R | R | R | R |
| Every element wrong | R | R | R | R | R |
| Empty string | S("") | S("") | R | S("") | R |
| Duplicate key | R | R | R | R | R |
| Invalid content | R | R | R | R | R |

### Requested metrics

| Shape | whole facet | parameters | has_docs | lines |
|---|---|---|---|---|
| Control | S({parameters: 1, has_docs: false, lines: 2, cyclomatic: null}) | S(1) | S(false) | S(3) |
| Absent | R | R | R | R |
| Null | R | S(None) | S(None) | S(None) |
| Wrong whole type | R | R | R | R |
| One wrong element | R | R | R | R |
| Every element wrong | R | R | R | R |
| Empty | R | R | R | S(0) |
| Duplicate key | R | R | R | R |
| Invalid content | R | R | R | R |

The whole-facet control contains its extracted body count of 2; the individual `lines` column deliberately sets a populated control of 3. An unrequested facet is absent, which the separate projection/coverage tests verify. Fractions, negative numbers, overflow, strings, arrays and objects cannot become a line count. The concurrently added `cyclomatic` column is required too; its source counting rules are outside this feature.

The matrix's invalid-content cell injects garbage immediately after the selected field value. Rejection of **a complete valid document followed by garbage** also follows the production reader's `serde_json::from_slice::<StoredEntry>` contract, which consumes the entire document; it does not use a streaming prefix decoder. Duplicate nested keys reach typed deserializers without passing through a last-wins JSON object first. These reader invariants were inspected alongside the public-output cells. No permissive-default or silently filtered-element defect was found in the feature fields.

## Requirement-to-Verification Assessment

| User-facing requirement | Appropriate level | Evidence and result |
|---|---|---|
| Explicit generics, source order, declaration ownership, bounds/defaults/variance/requirements and overflow errors | Level 1 public API | Per-language generic matrix, five C++ template-template owners, list-less requirements and boundary rows pass. Grammar limits remain demonstrated and documented. |
| Complete written bases, conservative local identity, source order/deduplication and category | Level 1 public API/CLI | Relation/path/owner matrices and the independent C# sweep assert exact required IDs and missing IDs, all three categories and written text. Pass. |
| Rust positive/negative/empty/inherent impls, method-only facts and type/member ownership | Level 1 public API/CLI | Dedicated impl tests, path rows, member-only queries, parent/container/membership, cache widening and outline checks pass. |
| Swift local extensions, unproved-owner boundaries and raw-type exclusion; Go interface exclusions | Level 1 public API/CLI | Known/unproved/nested-generic/source-owner matrices, all raw spellings and Go predeclared-name controls pass. |
| Declaration-owned modifiers and attributes, order/repetitions/raw/empty arguments and no body leakage | Level 1 public API/CLI | Modifier/attribute language and wrapper matrices plus member robustness cells pass. |
| Physical callable body lines, no-body/expression/nested/wrapped forms, LF/CRLF and boundary semantics | Level 1 public API | Body-line/span matrices and matching-capability cache projection/widening pass. God-files' independent declaration-size behavior remains covered. |
| One parse/definitions query, optional passes skipped, bounded lookup and generic work | Level 1 counters | Shared corpus work consumers and every registered relation/shared-context work shape pass alongside exact-result assertions. |
| Cache service/projection/widening and exact current schema/revision | Level 1 public API/CLI | Actual Hit assertions, strict-field matrices, metric coverage and stale revision/schema tests pass. Current schema is 5.0 and extractor revision is 41. |
| Outline relation labels, source-span/signature suppression, filtering, order, wrapping and styling | Level 1 CLI plus Level 2 real-terminal capture | CLI outline/golden checks and tmux/WezTerm captured rows and effective styles pass. Rendering uses biscuit-terminal Prose and UnorderedList components. Kitty entry points supply no rendering evidence on this host. |
| JSON/JSONL/full indexes, count and statistics behavior | Level 1 CLI | Registered report/golden tests pass; supplemental complete/selective C# outputs agree with independently expected facts. |
| Keyboard, paste, IME or mouse behavior | Level 3 only if promised | This feature introduces no such interaction requirement; no Level-3 coverage is needed. |
| Current behavior docs, examples, adapter losses and accepted limits | Inspection plus Level 1 | READMEs, topic docs, current schema examples and skill agree with the accepted implementation. Adapter-loss assertions pass. |

The library sets `autotests = false`, but its [declared L1 target](../../lib/Cargo.toml:55) includes all relevant modules in [main.rs](../../lib/tests/l1/main.rs). The new arity rows enter the shared corpus and ordinary L1 test groups; coverage guards require their shape and group registrations. CLI cache/report modules enter the declared symbols target. Both real-terminal targets require `terminal-tests`, enabled by the live `test-l2` recipe and CLI CI metadata. `just check-tier-coverage tree-hugger` reports zero stranded tests. No test was added or renamed by this review.

## Verification

- **`just test`: 1,360 passed, zero skipped**, 82.095 seconds, at normal recipe concurrency. No retry, timeout adjustment or restricted-worker rerun was needed.
- **`just lint`: passed** for tree-hugger and tree-hugger-cli.
- **`just check-tier-coverage tree-hugger`: zero stranded tests.**
- **`just test-l2`: six entry points passed**, 16.219 seconds. tmux and WezTerm performed real-terminal declaration/report capture. Kitty's two unavailable-backend gates returned early and do not prove Kitty rendering. The recipe uses background resources; this review made no keyboard injection or foreground-focus request.
- **Supplemental shipped-CLI sweep: 144 corrected sources, 720 written bases.** Each source copies the C# fixture, removes its file-scoped namespace for block-namespace compatibility, then appends an isolated case. Twelve sites cover all six forms and both partial states: bare, namespace-qualified, nested, anchored final/intermediate, outward from unique/anchored types and namespaces, rooted, three-level generic paths, outward qualified lookup and namespace/wrong-arity competition. Uncached complete output, cache population, genuine warm complete output and selective `types --json` all pass independently stated exact-ID/category/text assertions; every warm run reports one hit and zero parses. Saved warm indexes also have empty diagnostics.
- The first supplemental sweep retained the fixture's file-scoped namespace while appending block namespaces. Its outward-namespace result was excluded: C# forbids combining those forms in one file ([Microsoft's namespace diagnostic reference](https://learn.microsoft.com/en-us/dotnet/csharp/misc/cs0101)). Correcting that fixture prerequisite removed the failure. This was not an unchanged prior reproduction, repair regression or new finding.

The [original implementation log](./implementation-log.md) was read as a snapshot; [the repair log](./log.md) records the subsequent accepted choice and review-1 through review-11 repairs. Implementation flags were preserved as claims and checked against current source, required instances and live consumers. Concurrent scoped-reference changes are outside this feature's new behavior, except for shared lookup/cache effects exercised by the area suite. Cross-OS evidence remains CI/CD work and does not affect this readiness decision.

Used the tree-hugger, rust-testing, biscuit-test-harness and biscuit-terminal skills. No production code, permanent tests, golden files or lifecycle location were changed. Only this review and the requested previous-review/spec metadata were edited. The feature remains active for the author to close after the review cycle.
