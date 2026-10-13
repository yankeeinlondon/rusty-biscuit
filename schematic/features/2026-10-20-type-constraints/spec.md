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
human_review: false
depends_on:
    - 2026-10-14-schematic-definition-api
    - 2026-10-12-schematic-endpoint-methods
related:
    - 2026-10-12-schematic-runtime-envelope
    - 2026-10-13-schematic-pagination
    - 2026-10-18-schematic-fidelity-polish
message_to_agent: |-
  Split out of 2026-10-12-schematic-runtime-envelope review 1, Item 2. The author agreed that constraints matter primarily on what is sent and that this is separate work. Decisions D1–D5 were ruled on 2026-10-12 under the author's delegation, and each states its reason; apply them without pausing. Checks run at the `.build()?` / `.await` points defined by 2026-10-12-schematic-endpoint-methods (its R5).
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
| strings | `minLength`, `maxLength`, `pattern`, `format` (see D4) |
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

**R7. Replies are not checked against value constraints.** No reply-side constraint checking is built, opt-in or otherwise (D3). Structural checking of replies (types, required fields, enum values) is unchanged.

## Decisions

The author delegated these rulings on 2026-10-12, so the pipeline can run without pausing for them. Each records the reason it was chosen.

**D1. Enforcement mechanism: Decided.** Each request type with constrained values gets a generated `validate()` method. It checks every constrained field and returns all violations, not just the first.

The two alternatives were considered and rejected:

- **Newtypes**, e.g. `PerPage(u8)`, validated on construction. Every setter would then take a wrapper (`.per_page(PerPage::new(10)?)`), which breaks the call style approved in `2026-10-12-schematic-endpoint-methods`.
- **The `validator` crate.** It adds a dependency, and its attribute dialect differs from JSON Schema in small ways that would need mapping. Generated checks can follow JSON Schema's definitions exactly.

**D2. Where the check runs: Decided as drafted.** The check runs on the request's typed values, before credentials are applied and before encoding. It is called from the `.build()?` and `.await` points defined by `2026-10-12-schematic-endpoint-methods` (its R5).

Why:

- **Readable errors.** Messages name fields the way the caller wrote them (`per_page`), not their wire spelling.
- **No secrets in errors.** A violation can never involve a credential, because credentials haven't been added yet.

**D3. Reply-side checking: Decided. None is built.** Replies are not checked against value constraints, and no opt-in mode is added.

Why:

- **Nobody needs it.** The author's stated priority is what is sent, and no consumer has asked for reply checking.
- **It would mean two extra decisions with no user to inform them:** a logging-versus-rejecting policy, and a configuration surface.
- **It can be added later without breaking anything.** That would be a new opt-in, not a change to existing behavior.

**D4. Which `format` values are checked: Decided.**

| `format` | Checked with |
|---|---|
| `date-time`, `date` | `chrono` (RFC 3339) |
| `uri` | `url`, already a runtime dependency |
| `uuid` | A hand-written structural check: 8-4-4-4-12 hex digits. No `uuid` dependency. |
| `ipv4`, `ipv6` | The standard library |
| `email` | A structural check only: one `@`, non-empty local part and domain, domain contains a `.` |

Any other `format` is kept in the definition and exported, but not enforced. The generator emits an informational diagnostic so the gap is visible.

Why:

- Full email validation per RFC 5322 is notoriously unreliable and rejects real addresses. A structural check catches typos without false rejections.
- Avoiding a `uuid` dependency for a fixed-shape string keeps the dependency list short.

**D5. Where the checks and `pattern` support live: Decided.**

- **The checking helpers** (each `format`, length, range, `multipleOf`, uniqueness, and pattern matching) live in `schematic-runtime`, in a `validate` module. Generated `validate()` methods call them.
- **Patterns** are compiled with the `regex` crate at generation time to prove they compile, and again once at run time through a lazily initialized static.
- **Unsupported patterns.** A pattern that `regex` cannot compile (look-around, back-references) is kept in the definition and exported, but not enforced. A generator diagnostic names the API, the schema path, and the pattern.
- **Dependencies.** `regex` and `chrono` are added to `schematic-runtime` and recorded in `schematic/docs/dependencies.md`.

Why the runtime, not the generated crate:

- The generated crate gains no new dependencies.
- The runtime's ABI marker covers the helpers. That follows the direction set in `2026-10-12-schematic-runtime-envelope` review 1, Item 6: move shared helpers into the runtime instead of adding new generated shared code.

## Validation

- `schematic-define`, L1: constraint construction and rejection of contradictory constraints (R1); import of a fixture document containing every in-scope keyword, plus a diagnostic for an unsupported keyword (R2); `schemars` attribute capture (R3).
- `schematic-gen`, L1: generated `validate()` for a fixture endpoint covering each keyword; a mock-server test proving a violating request is refused before any connection is made and a conforming one is sent unchanged (R4); OpenAPI export round trip (R6).
- Shipped definitions: GitHub `per_page` (1..=100) as the adopting example, with its constraint visible in the generated docs.

## Human review agenda

None open. D1–D5 were ruled under the author's delegation. A reviewer should still flag any constraint or `format` found in the shipped definitions that these rules do not cover.
