---
$schema: feature-review.yaml
ready: false
findings: []
observations: []
human_in_the_loop:
    - |-
        Decide whether serialized schema references should carry the new descriptor evidence.
    
        What it is: In schematic-define, the stored request/response schema reference now has an optional `descriptor` field. The implementation log says this preserves the component name and container shape after lowering a typed registration to a Rust path. For example, a response containing a list of models needs both its Rust type spelling and the identity of the schema describing each model. See [Schema](../../define/src/schema.rs) and the implementation log's “Builder implementation and lowering” section.
    
        Why this needs an author: This meets test 1 (hard to reverse). The specification requires destination validation, but does not dictate adding descriptor evidence to the public serialized Schema representation. The implementer selected that representation (implementation-log.md, line 1094). Unlike a private builder detail, it becomes data that readers and the next specification's self-contained bundles must understand. The next specification currently describes serialized definitions as naming their request/response types by string; this choice gives those references additional structured evidence. This is a decision about that new contract, not a claim that the implementation is defective.
    
        Options:
    
        - Keep the optional serialized evidence. This preserves the implementer's destination-validation approach and avoids rebuilding the migration. The cost is supporting another representation of type identity alongside the Rust path and registered schemas, including checking that they agree when loading future bundles.
        - Keep schema references as names and move the validation evidence into context-owned data or a separate transfer representation. This keeps the stored reference smaller and closer to the next specification's current description. The cost is redesigning how transferred definitions retain enough evidence for validation and revisiting the builder, exporter, and migration tests.
    
        Recommendation: Keep the evidence. The log identifies a concrete need: retaining type identity and wrapper information through lowering so destination assembly can reject mismatched references. Approving it lets the next serialization stage build on an explicit contract rather than accidentally inheriting one.
    
        If you answer “go with your recommendation”: Retain the optional field and its validation approach; after the normal implementation review passes, refresh the next specification and plan to describe the evidence and its relationship to bundle schema references. This ruling does not approve the code or its tests; those still need review.
    
        If undecided: This review remains paused. Starting the next serialization stage risks designing a bundle against a reference representation that may be removed or relocated.
human_resolved: true
human_review: true
human_review_items:
    - Decide whether to retain the new optional descriptor evidence in serialized schema references; see human_in_the_loop item 1 for options and recommendation.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-14T22:56:32-07:00"
spec: "2026-10-14-schematic-definition-api/spec.md"
implemented: true
description: "A **feature** review of `2026-10-14-schematic-definition-api/spec.md`"
feature: "2026-10-14-schematic-definition-api/review-1.md"
next_spec: "schematic/features/2026-10-15-schematic-definition-vocabulary/spec.md"
next_plan: "schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md"
log: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-14-schematic-definition-api/implementation-log.md"
implemented_by: claude/opus
next: "2026-10-14-schematic-definition-api/review-2.md"
---

# Review 1: author decision before implementation review

This review pauses for one public data-format decision. Two agents read the implementation log and specification in parallel, including the human review agenda, proposed choices, supplied defaults, and implementation handoff. Their reports and a targeted read of the next serialization requirements informed the classification below. Implementation code was not compared with the specification, and tests were not rerun. Production readiness has **not been assessed**: `ready: false` and the empty findings list mean that implementation review is pending.

## Human decision

### Item 1: Keep descriptor evidence in serialized schema references

Decide whether serialized schema references should carry the new descriptor evidence.

What it is: In schematic-define, the stored request/response schema reference now has an optional `descriptor` field. The implementation log says this preserves the component name and container shape after lowering a typed registration to a Rust path. For example, a response containing a list of models needs both its Rust type spelling and the identity of the schema describing each model. See [Schema](../../define/src/schema.rs) and the implementation log's “Builder implementation and lowering” section.

Why this needs an author: This meets test 1 (hard to reverse). The specification requires destination validation, but does not dictate adding descriptor evidence to the public serialized Schema representation. The implementer selected that representation (implementation-log.md, line 1094). Unlike a private builder detail, it becomes data that readers and the next specification's self-contained bundles must understand. The next specification currently describes serialized definitions as naming their request/response types by string; this choice gives those references additional structured evidence. This is a decision about that new contract, not a claim that the implementation is defective.

Options:

- Keep the optional serialized evidence. This preserves the implementer's destination-validation approach and avoids rebuilding the migration. The cost is supporting another representation of type identity alongside the Rust path and registered schemas, including checking that they agree when loading future bundles.
- Keep schema references as names and move the validation evidence into context-owned data or a separate transfer representation. This keeps the stored reference smaller and closer to the next specification's current description. The cost is redesigning how transferred definitions retain enough evidence for validation and revisiting the builder, exporter, and migration tests.

Recommendation: Keep the evidence. The log identifies a concrete need: retaining type identity and wrapper information through lowering so destination assembly can reject mismatched references. Approving it lets the next serialization stage build on an explicit contract rather than accidentally inheriting one.

If you answer “go with your recommendation”: Retain the optional field and its validation approach; after the normal implementation review passes, refresh the next specification and plan to describe the evidence and its relationship to bundle schema references. This ruling does not approve the code or its tests; those still need review.

If undecided: This review remains paused. Starting the next serialization stage risks designing a bundle against a reference representation that may be removed or relocated.

## Implementation decisions (informational)

These are recorded for context and do not independently require a human decision:

- **Registration and naming:** The implementer selected `Registration: JsonSchema + 'static`, declaration-local registrations, and separate native/container implementations. The specification expressly allows an additional Rust-path derivation bound and supplies the nonoverlapping-dispatch default. Foreign adapters live in the trait-owning crate. Typed paths check identity and accept concrete generic arguments; first registration fixes canonical spelling, so aliases must be registered first. These implement the specified typed-path mechanism; there is no separate proposal here to replace it.
- **Descriptors:** Private named, inline, list, optional, and map shapes accompany the reserved `Option<TypeRef>` structural carrier. Unknown fields are rejected; absent/null reserved structure means no structural declaration. Non-null structure is reserved for later authoring work. These fit the approved carrier constraint. The additional placement of evidence in stored schema references is the separate decision above.
- **Builders and defaults:** The implementation uses required-fact typestate and the proposed context, builder, success, error-set, and error names. It keeps all 20 audited existing defaults with stated reasons, including incremental WebSocket construction. Applying `#[non_exhaustive]` follows the specification's explicit recommended default; no reaffirmation is requested. Single-success lowering and future method reservations follow the stated response contract.
- **Authentication and runtime metadata:** API authentication policy is retained; unused endpoint OAuth scopes are removed. Explicit empty environment mappings retain their existing meaning. Endpoint origin policies replace API policy and callers may only tighten. Request-id headers are normalized. The missing parameter secret marker was added over existing secret-name metadata, following the explicit parameter-marker requirement; the log also records that the prerequisite lacked this field despite the conditional planning instruction. Its name, Boolean form, and requested behavior were already specified, so this is not treated as an independent unchosen contract. Existing HEAD-body behavior remains unchanged because only GET rejection is required.
- **Migration and output:** The emitter/OpenAI-first and smallest-first batches follow supplied defaults. The 8,449,975-byte baseline exceeded the supplied threshold, so canonical JSON uses biscuit-hash BLAKE3 fingerprints with ordered arrays and registries. Nine query-bearing paths were migrated, including seven compatible OpenAI beta merges and the Gitea merge. The log reports 539 final endpoints, 258 OpenAI endpoints, and 93 pagers. Container components were removed; seven nominal leaf entries were made explicit at their former container positions. These output changes are logged for the later migration-equivalence review, not approved by this decision-only review.
- **Compatibility and implementation mechanics:** Fallible module-group assembly underlies retained compatibility factories. REST identity uses qualified paths and descriptors while WebSocket module-path readers remain. Qualified root-list inference preserves pagination. Serialized pagination category names remain despite table removal. Existing compiler misuse infrastructure replaces a new trybuild suite; generated helper batches and an 8 MiB Windows generator stack address compilation/execution constraints. Test layout, helper structure, and README-reference relocation are implementation choices.

## Deferred work and verification limits

The specification already assigns general query-required fidelity, structural compositions, self-contained bundles, richer authentication vocabulary, response sets, streaming, and GraphQL to later stages. Optional authentication, broader authentication dual-source cleanup, HuggingFace stub shapes, and orphan registrations remain expressly identified candidates for the later fidelity stage. None adds a new scope decision to this review.

The log reports execution on macOS, Linux, native Windows/MSVC, and WSL2, while identifying existing resource-test exclusions, native Windows TLS ignores, untested Windows-GNU configuration, and the absence of qualifying committed-tree receipts. It does not identify a newly required verification that only a human can perform. These are reported implementation claims, not independently verified evidence in this review.

No follow-up specification or plan was refreshed: that work is conditional on a completed implementation review with `ready: true`. No implementation changes, lifecycle moves, or commits were made.

## Findings

Not assessed. The implementation comparison will begin after the author records the ruling and sets `human_resolved: true`.

## Observations

None recorded in this decision-only review.

## Human rulings

### Item 1

Ruling: **Do not keep the embedded descriptor evidence. Normalize schema references (option C), remediated within this spec.**

Neither presented option is taken as written. Embedding evidence in every reference (option A) duplicates `type_name`, types a REST-required field as `Option`, repeats inline schemas per reference, and creates agreement/forgery checks that every future loader must repeat. It also keeps two type-shape vocabularies in `schematic-define`: the private registration `Shape` and the existing `TypeRef`.

Required direction:

- A REST request/response reference stores only canonical identity and wrapper shape, expressed with `TypeRef` (`Named`, `Array`, `Optional`, `Map`, ...) rather than a parallel `Shape` enum. It carries no Rust path and no inline copy of a registered schema.
- `RestApi` carries the registry its context produced: canonical schema name → Rust path binding and wire schema. Each fact is stored once.
- API assembly validation reduces to: every reference resolves in the destination registry (dangling names are errors naming API, endpoint and name), and merging module-group registries rejects conflicting definitions of the same name. Descriptor/path agreement and forgery checks are removed because the reference no longer holds facts that can disagree.
- `TypeRef` may be extended, never replaced, where the reference needs it: Rust map key types, and free-form inline JSON that has no registry entry. Record the extensions in the implementation log.
- `Schema::descriptor` is removed. WebSocket `Schema` (`type_name` + `module_path`) is unchanged by this ruling.
- Generator, OpenAPI/Postman export, typed list inference, the JSON snapshots, the canonical baseline, and the builder/transfer tests are updated to the normalized form. The R7 "Builder implementation and lowering" log entry is amended to record the departure.

Rationale: the normalized form matches OpenAPI's `$ref` model and is the self-contained bundle the next specification (R7) needs, so that stage builds on this contract instead of replacing it. Doing it here avoids churning the 4a JSON snapshots and transfer tests twice.

This ruling does not approve the code or tests; implementation review follows the remediation.
