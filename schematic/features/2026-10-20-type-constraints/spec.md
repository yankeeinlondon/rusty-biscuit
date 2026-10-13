---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
review_iterations: 0
clarified: false
implemented: false
human_review: true
depends_on:
    - 2026-10-14-schematic-definition-api
related:
    - 2026-10-12-schematic-runtime-envelope
    - 2026-10-13-schematic-pagination
    - 2026-10-18-schematic-fidelity-polish
message_to_agent: |-
  Draft split out of 2026-10-12-schematic-runtime-envelope review 1, Item 2. The author agreed that constraints matter primarily on what is sent and that this is separate work. The proposed decisions (PD) below are drafts; none is ruled. The "endpoint method" call surface referenced here is the one ruled in that review's Item 1 and is itself scheduled as a separate feature.
---

# Type constraints

Schematic definitions describe the **shape** of request and response data (types, required fields, enum values) but not the **rules on values** that JSON Schema can express, such as "a string of at most 100 characters" or "an integer between 1 and 100." This feature adds those rules to definitions and enforces them on what a client sends, before anything goes over the network.

## Problem

Today a caller can build a request the API will refuse, and only learns that from the server:

```rust
let releases = gh.list_releases("yankeeinlondon", "rusty-biscuit")
    .per_page(500)        // GitHub's maximum is 100
    .await?;              // GitHub rejects or silently caps this; nothing local catches it
```

### Current state (verified 2026-10-12)

- A definition field records only `name`, `serde_rename`, `description`, `required`, and `field_type` (`schematic/define/src/models/structs.rs`). There is nowhere to put a constraint.
- A response `Schema` is a Rust type name (`schematic/define/src/schema.rs`).
- The OpenAPI importer **discards** constraints. Its only use of `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, and `multipleOf` is clamping out-of-range numbers so the document parses (`schematic/define/src/openapi/import/normalize.rs`). An imported definition therefore silently loses every value rule its source document carried.
- `enum` is the one constraint enforced today: a closed set of strings becomes a Rust enum, and serde rejects unknown values while parsing.
- Pagination (`2026-10-13-schematic-pagination`, D24) plans its own pre-send check for "page size above the maximum." That is one special case of this feature.

## Goals

1. Definitions can carry JSON Schema value constraints, whether written by hand or imported from OpenAPI.
2. Generated clients check request values against those constraints **before sending**, and report a violation as a local error that names the field and the rule.
3. Constraints survive the round trip: imported from OpenAPI, carried in definitions, and exported back to OpenAPI.

## Non-goals

- Changing the response decoder contract. It was ruled as built in `2026-10-12-schematic-runtime-envelope` review 1, Item 2.
- Full JSON Schema conformance (`if`/`then`/`else`, `dependentSchemas`, `$dynamicRef`, …).
- Semantic validation that needs the server (for example, "this repository exists").

## Constraints in scope

| Applies to | Keywords |
|---|---|
| strings | `minLength`, `maxLength`, `pattern`, `format` (see PD4) |
| numbers and integers | `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` |
| arrays | `minItems`, `maxItems`, `uniqueItems` |
| any | `enum` (already supported), `const` |

## Requirements

**R1. Definitions carry constraints.** A field, query parameter, path parameter, or header definition can state any constraint in the table above. Constraint values are validated when the definition is built: for example, `minLength` cannot exceed `maxLength`, a `pattern` must compile, and a constraint must match its type (no `maxLength` on an integer).

**R2. The OpenAPI importer keeps constraints.** Every in-scope keyword in an imported document reaches the definition. A keyword the importer cannot represent produces a diagnostic naming the schema path and keyword; it is never dropped silently.

**R3. Hand-written definitions can declare constraints on their Rust types.** `2026-10-14-schematic-definition-api` moves type registration to `schemars`, whose attributes (`#[schemars(length(max = 100))]`, `range(min = 1, max = 100)`, `regex(...)`) already record constraints in the generated schema. Those recorded constraints are the source for hand-written types, so a constraint is written once, next to the field it governs.

**R4. Requests are checked before sending.** A generated request checks every constrained value it is about to send. The check runs:

- at `.build()?`, when the caller asks for early checking (the optional build step ruled in the runtime-envelope review, Item 1); and
- otherwise immediately before the request is sent, so no request that breaks a declared constraint reaches the network.

A violation is a pre-send error: the existing invalid-argument kind of `TransportError`, which carries request metadata. It names the field path, the rule, and the limit (for example `per_page: must be at most 100`). It never echoes a value from a field marked secret.

```rust
let call = gh.list_releases("yankeeinlondon", "rusty-biscuit")
    .per_page(500)
    .build();
// Err(... InvalidArgument: "per_page: must be at most 100" ...)
```

**R5. Pagination's page-size bound uses this mechanism.** Once this feature lands, the page-size maximum in `2026-10-13-schematic-pagination` (D24) is expressed as an ordinary `maximum` constraint, not a pagination-specific check. If pagination ships first, this feature migrates it.

**R6. Constraints are exported.** The OpenAPI exporter emits every constraint a definition carries. Generated Rust documentation states each constrained field's rules, so they are visible in the editor.

**R7. Replies are not rejected for violating constraints by default.** Constraint checking does not run on replies unless the caller opts in (PD3). Structural checking of replies (types, required fields, enum values) is unchanged.

## Proposed decisions (unruled)

**PD1. How generated code enforces constraints.** *Default:* generate a plain `validate()` method on each request type that checks its constrained fields and returns every violation, not only the first. This adds no dependency beyond `regex` (PD5). Alternatives considered:

- *Newtypes* (`PerPage(u8)` validated on construction): enforcement is guaranteed by the type, but every setter would take a wrapper (`.per_page(PerPage::new(10)?)`). That breaks the call ergonomics ruled in Item 1.
- *The `validator` crate*: derive-based and familiar, but it is an extra dependency, and its attribute dialect differs from JSON Schema in small ways that would need mapping.

**PD2. Where the check runs relative to credentials and encoding.** *Default:* before credentials are applied and before the request is encoded, on the request's own typed values, so error messages use field names rather than wire names.

**PD3. Reply-side checking.** *Default:* off. The author agreed during the runtime-envelope review that constraints matter primarily on what is sent. Open question for the author: when a reply-side check is turned on, should it log the violation and still return the data, or reject the reply? See "Human review agenda."

**PD4. Which `format` values are checked.** *Default:* `date-time`, `date`, `email`, `uri`, `uuid`, `ipv4`, `ipv6`. Any other `format` is carried and exported but not enforced, and the generator emits an informational diagnostic so the gap is visible.

**PD5. `pattern` dialect.** JSON Schema patterns use the ECMA-262 regular-expression dialect. The Rust `regex` crate does not support look-around or back-references. *Default:* compile each pattern with `regex` at generation time. A pattern that does not compile is carried and exported but not enforced, with a generator diagnostic naming the API, schema path, and pattern. Adding `regex` to generated crates is recorded in `docs/dependencies.md`.

## Validation

- `schematic-define`, L1: constraint construction and rejection of contradictory constraints (R1); import of a fixture document containing every in-scope keyword, plus a diagnostic for an unsupported keyword (R2); `schemars` attribute capture (R3).
- `schematic-gen`, L1: generated `validate()` for a fixture endpoint covering each keyword; a mock-server test proving a violating request is refused before any connection is made and a conforming one is sent unchanged (R4); OpenAPI export round trip (R6).
- Shipped definitions: GitHub `per_page` (1..=100) as the adopting example, with its constraint visible in the generated docs.

## Human review agenda

1. **Reply-side behavior when opted in (PD3):** log and return the data, or reject the reply?
2. **Enforcement mechanism (PD1):** generated `validate()` (default), newtypes, or the `validator` crate.
3. **Scheduling relative to the endpoint-method feature ruled in review 1, Item 1.** Both touch the generated request types. Running this one after it avoids reworking the setters twice.
