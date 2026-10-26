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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-21
review_iterations: 6
completed: true
clarified: false
implemented: true
implemented_by: "claude/opus"
human_review: false
depends_on:
    - 2026-10-14-schematic-definition-api
    - 2026-10-12-schematic-endpoint-methods
related:
    - 2026-10-12-schematic-runtime-envelope
    - 2026-10-13-schematic-pagination
    - 2026-10-18-schematic-fidelity-polish
message_to_agent: |-
  The three Open Questions were ruled on 2026-10-21 with their recommended options: JSON bodies are checked on the serialized representation that is sent, formats are annotation-only, and numeric rules use exact arithmetic. Earlier author-delegated choices favor generated checks, shared runtime helpers, and no response validation. The inline review updates those choices where current request, schema, and pagination contracts require it; reader notes explain the changes. Keep optional call build checks and awaited sends on the same validation path. On 2026-10-24 the author resolved review-5's blocked findings (R1-F3, R3-F1) with an explicit, API-level unchecked-composition opt-in; see "Unchecked-composition opt-in" under Supported schema traversal.
---

# Type constraints

Schematic definitions describe request and response shapes. This feature preserves value rules, such as “a string of at most 100 characters” or “an integer between 1 and 100,” and checks supported rules before sending a request. It does not add response validation.

> **Reader note — review basis.** This review checked the code on 2026-10-21. The original draft predated typed registration and the current pager. Stored schemas already preserve some rules; the missing work is reliable preservation across every generation route and enforcement on outgoing values. Earlier decisions remain the starting point, with explicit corrections below. The author ruled the Open Questions on 2026-10-21; each ruling is recorded beneath its question.

## Problem and current state

A caller can supply an invalid string or number through an ordinary request setter and learn of the problem only from the server. However, page-size bounds are already enforced locally: the GitHub pager's `.page_size(500).fetch_page().await` fails before sending because its declared maximum is 100. This feature must extend that protection without removing it.

The current contracts are:

- In `schematic-define`, [FieldDef](../../define/src/models/structs.rs) describes imported Rust model fields without value constraints. [ParamDef](../../define/src/params/param_def.rs) describes query, path, header, and cookie inputs and has no general constraint representation.
- REST bodies and responses use component references. In `schematic-define`, [SchemaComponent](../../define/src/components.rs) stores JSON Schema in the API's component list, including rules produced by Schemars. The separate [Schema](../../define/src/schema.rs) type names WebSocket models and is outside this feature.
- The `schematic-define` importer [retains source component schemas](../../define/src/openapi/import/mappings/schema.rs), but its Rust model representation and rebuilt typed assemblies do not reliably preserve all those rules. Its [numeric normalization](../../define/src/openapi/import/normalize.rs) can clamp large bounds. The [OpenAPI converter](../../define/src/openapi/export/conversion.rs) also needs constraint-aware conversion. “The importer discards every constraint” is therefore inaccurate.
- In `schematic-runtime`, [EndpointSpec::into_parts](../../runtime/src/endpoint.rs) prepares a request before credentials and network work. [Call::build](../../runtime/src/call.rs) already performs the send's argument checks and returns `InvalidArgument`; it rechecks on await. A client's `.build()` instead validates client configuration and is not a request check.
- In `schematic-runtime`, the [pager engine](../../runtime/src/pager/engine.rs) checks page-size bounds after choosing the effective size. The pager can change values after assembling the original endpoint request.

## Goals

1. Carry value constraints for handwritten and imported definitions without inventing a second source of rules for registered models.
2. Refuse requests that violate supported rules before network work, with useful errors that contain no submitted values.
3. Preserve constraint meaning through OpenAPI import, definition serialization, emitted Rust definitions, registration, and OpenAPI export.
4. Preserve the existing fluent setters, response decoding, and pagination controls.

## Non-goals

- Reply-side value validation, including an opt-in mode. Existing type, required-field, and enum decoding remains unchanged.
- Full JSON Schema conformance, including conditional schemas, dynamic references, and arbitrary schema composition.
- Checks needing server knowledge, such as whether a repository exists.
- WebSocket message validation, new body kinds, file-content inspection, or adding OpenAPI 3.1 import support as an incidental part of this change.
- Replacing existing Rust model types with validated wrapper types.

## Constraints and their meaning

| Applies to | Keywords |
| --- | --- |
| Strings | `minLength`, `maxLength`, `pattern`, `format` (assertion policy remains an Open Question) |
| Numbers and integers | `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` |
| Arrays | `minItems`, `maxItems`, `uniqueItems` |
| Any JSON value | `enum`, `const` |

These are the supported validation keywords, not a claim that every schema containing them can be evaluated. Enforcement also needs a supported way to reach the constrained value, as described below.

Use the [JSON Schema validation definitions](https://json-schema.org/draft/2020-12/json-schema-validation) for value semantics. String length counts Unicode code points; patterns search unless explicitly anchored. Numeric equality is mathematical: `1` equals `1.0`; `multipleOf` requires a positive divisor and an integral quotient. `uniqueItems` uses deep JSON equality, including object members independent of order. `enum` and `const` apply to whole JSON values. Formats are annotations unless an assertion policy is chosen.

Do not equate Rust `PartialEq`, UTF-8 byte length, a serialized query string, or an arbitrary floating-point epsilon with these definitions. In particular, array length is checked before joining or exploding parameter values. Absent optional inputs are skipped; a present empty string or array is checked. Explicit JSON null follows the schema's nullability and applicable rules. Do not inject schema `default` values as part of validation.

### Supported schema traversal

Validation must reach root values, object properties, array items, map values, local component references, and recursive models. Type-specific rules apply only to matching JSON kinds. Nullable wrappers and the equivalent nullable alternatives emitted by Schemars must remain supported. Inspect the supported Schemars output in a fixture rather than assuming every `anyOf` represents a nullable wrapper.

Generic `oneOf`, `anyOf`, `allOf`, and `not` evaluation is outside scope. A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error; it must not produce a client that claims to enforce the rule. This holds at every request entry point (JSON body, multipart JSON field, parameter), including a composition at a component root. The only relaxation is the author opt-in below. Recursive schema compilation must terminate by reusing named plans rather than expanding references indefinitely. Runtime traversal follows the finite request value.

#### Unchecked-composition opt-in

**Ruled by the author on 2026-10-24** (resolving review-5's conflict between strict refusal and Goal 4): an API author may explicitly accept unchecked composition rules for one API, instead of losing generation for it.

- **Stored and explicit.** The opt-in is API-level definition metadata, absent by default, set through the fluent `RestApi` builder (final name in the plan), serialized with the definition, and kept by bundles and emitted definitions. It is a general author choice, not a special case keyed on a catalog name.
- **Narrow.** It covers only constrained request rules beneath `anyOf`, `oneOf`, multi-member `allOf`, and `not`. Unresolved references, positional items, and the other unsupported keywords still fail generation. It never relaxes definition errors such as malformed or contradictory constraints.
- **Disclosed.** With the opt-in, each covered rule is a warning naming the API, endpoint, schema path, keyword, and composition. Generation continues and emits no check for that rule. The generated rustdoc for the affected request input states which rules are not enforced. Without the opt-in, the same rules are generation errors.
- **Preserved through rebuilding.** Imported definitions must keep the composition structure and the constraints beneath it, for all four generic shapes at every rebuilding position, so the rebuilt API produces the same errors (without the opt-in) or the same warnings and disclosures (with it) as the direct import. Keeping them is preservation, not evaluation; no runtime composition evaluator is added.
- **OpenAI.** The OpenAI catalog sets the opt-in in its hand-written overlay (`definitions/src/openai_pagination.rs`, which the import never overwrites), so `just import-openai` and regeneration keep the existing client methods and pagination controls (Goal 4). Its catalog expectations record the resulting warning counts.

## Definition and preservation requirements

### One source for model rules

In `schematic-define`, registered models' stored component schemas remain authoritative. Schemars attributes on handwritten fields record the rules once, beside the model. Do not add a competing editable constraint list to registered fields or change component names, Rust paths, collision handling, or registration order. An endpoint-specific rule must not mutate a shared model's schema for every endpoint.

Add serializable constraint metadata where no model component carries the rule: parameters, inline body roots, form fields, and imported model declarations needed to rebuild schemas. Carry array item rules as well as rules on the array itself. Provide fluent authoring through the existing parameter and body builders; never require public endpoint or API struct literals. Specify the final method names in the implementation plan and verify compiling examples.

Imported Rust definitions must rebuild equivalent rules through Schemars attributes where expressible and explicit schema metadata where attributes cannot express them. This includes aliases, inline schemas, nested-only components, renamed fields, and a constrained reference used in multiple endpoints. A direct library import succeeding is insufficient if `import --definitions-out` followed by assembly loses the rule.

New optional stored metadata is omitted when absent and reads as absent from existing snapshots. Validate it at API build and generator entry, including definitions read from serialized data. Continue using the existing reference validation and diagnostic conventions.

### Definition errors versus unsupported rules

Reject malformed keyword values, negative lengths or counts, nonpositive `multipleOf`, incompatible authored types, empty enums, contradictory intervals, and lower bounds above upper bounds. Equal exclusive endpoints also define an empty interval. This is a check for direct contradictions, not a general proof that an arbitrary schema is satisfiable.

Preserve unsupported rules when the source representation permits it, with API, endpoint when applicable, schema path, keyword, and reason in the diagnostic. Annotation-only keywords such as `title` and `description` do not trigger warnings. Generation must fail for a supported request rule it cannot enforce, except for explicitly documented pattern and format exceptions below and the unchecked-composition opt-in above. Response-only constraints remain exportable without generating response checks.

### OpenAPI conversion

The existing importer and exporter target OpenAPI 3.0. Normalize its Boolean exclusive-bound flags with their associated bounds into the internal numeric-bound meaning, then reverse that mapping for export. Translate `const` to a singleton `enum` for OpenAPI 3.0 output; reimport need only preserve meaning. Nullable schemas must retain their value rules. These translations follow the [OpenAPI 3.0 Schema Object contract](https://spec.openapis.org/oas/v3.0.3.html#schema-object).

Never clamp, round, or silently narrow a constraint to make a parser accept it. Replace numeric normalization for constraint values with lossless capture before a restrictive parser can change them; if a source number cannot be represented, fail with its path. The same rule applies to export through the current OpenAPI library: use a lossless conversion route or report an error. Preserve source precision, even when a Rust request type can express a smaller range. A whole OpenAPI parser replacement is not assumed by this specification.

Round-trip acceptance compares meaning, not byte spelling: renamed component references, `const` lowering, and exclusive-bound normalization may change syntax. Tests must separately cover direct import/export and emitted typed definitions rebuilt through registration.

## Request enforcement and errors

### Generated entry point and send integration

Each generated request with supported constraints exposes `validate(&self)`, returning all constraint violations in deterministic order. The runtime owns the violation type and checking helpers. Each violation records a safe field path, keyword, and reason; the concrete result type is fixed in the implementation plan. This method performs no network or credential lookup and does not mutate the request.

In `schematic-gen`, generated [request preparation](../../gen/src/codegen/request_structs/into_parts.rs) must invoke the same checking logic before returning request parts. This preserves the existing `schematic-runtime` [EndpointSpec](../../runtime/src/endpoint.rs) contract and covers endpoint calls, `client.send(request)`, `.full()`, and `send_raw`. No caller has to remember to call `validate()`.

The call's optional `.build()?` returns the existing `schematic-runtime` [InvalidArgument](../../runtime/src/error.rs), with an aggregate reason containing the violations. Awaited ordinary calls wrap the corresponding `TransportErrorKind::InvalidArgument` in `CallError::Transport`; `.full().await` and `send_raw` return the transport error directly with the existing redacted request metadata. Do not change those public error shapes merely to add structured violations; callers needing individual violations use `validate()`.

Build checks and sends must agree for the same arguments and settings, and await always rechecks. Setters or public-field mutation after validation cannot create a permanent “already checked” flag. The send path validates the actual representation it sends; standalone `validate()` is an early report, not proof about a later serialization. How this works for JSON bodies requires the author choice in Open Questions.

> **Reader note — checking location.** The earlier draft required checking typed values before encoding. That works for parameters, but does not settle bodies with Serde renames, skipped fields, flattened objects, or custom serialization. The Open Question on body validation makes that tradeoff explicit. Credential placement still follows request preparation, and no new response validation is implied.

### Safe, useful errors

Use Rust parameter names when available, with a location prefix to distinguish `query.limit` from `header.limit`. JSON body paths follow serialized property names, for example `body.items[2].displayName`, so flattened or renamed fields still identify the submitted document. Array indices are safe; arbitrary map keys can contain secrets and must use a redacted placeholder in displayed paths.

Never include submitted values in `Display`, `Debug`, error sources, or aggregate messages, whether or not a field was marked secret. Report a keyword and numeric limit when safe; do not quote an entire allowed-value set, constant, or pattern in request errors. A request can contain a token in a body or ordinary parameter before authentication is applied, so checking before credentials is not itself a secrecy guarantee. Preserve the runtime's redaction of request metadata.

Order violations by parameter declaration, then body traversal with deterministic object-key ordering, then array index and keyword. Return all violations for the supported traversal; a serialization failure is an existing preparation error, not a fabricated list of field violations.

### Bodies and forms

Cover JSON request roots and nested data, text-body string rules, and textual or JSON form fields in the existing body variants. Form values are checked before URL encoding or multipart assembly. Binary bodies and file contents have no JSON string-length or format interpretation here. Reject attempts to attach those rules to them rather than silently skipping checks.

## Pagination integration

Keep `schematic-define` page-size minimum/maximum declarations as the authoring source for strategy-managed inputs. Lower their bounds into the shared runtime constraint helpers and expose their effective constraints in OpenAPI and generated documentation. A declared parameter and the strategy that adopts it must agree; conflicting bounds fail definition validation rather than one silently winning.

The `schematic-runtime` [pager engine](../../runtime/src/pager/engine.rs) must keep calling these helpers after selecting each effective size. This includes initial knobs, sent defaults, and changes during traversal. Adopted parameters no longer exist on the generated request, so a request's `validate()` cannot validate a future pager setting. Do not remove the page-planning validation site merely because request checks now exist. Preserve the current handling of provider next links that carry a size unless the caller overrides it.

> **Reader note — intended migration.** Reuse the checking implementation, not a single checking location. The original proposal to replace pagination's maximum with an ordinary request-field check would miss pager changes made after the original request was prepared. Existing paging methods and safety checks remain intact.

Use GitHub's existing `.page_size(500).fetch_page().await` refusal as a regression example; use an ordinary constrained fixture parameter to demonstrate early `.build()` checks. Do not teach `.per_page(...)` on an endpoint where the pager has adopted that parameter.

## Shared helpers, patterns, and formats

Generated code calls a `validate` module in `schematic-runtime`; do not add validation dependencies to each generated crate. Keep the existing runtime compatibility marker synchronized when generated code starts requiring new helpers. No validation trait implementation is required of user-owned body models.

Patterns retain the earlier decision to use Rust `regex`, compiled during generation and once per pattern at runtime through lazy initialization. This is a supported subset, not an ECMAScript-compatible engine. Only patterns with equivalent supported meaning may be enforced; both syntax that Rust rejects (such as look-around) and accepted syntax with incompatible meaning need a diagnostic and preservation for export. Authored patterns outside the subset fail definition validation; imported valid patterns outside it remain unenforced with a warning identifying the schema path. Invalid source patterns must not be described as merely unsupported. Generated rustdoc labels unenforced rules. Pattern errors must not panic at runtime or compile per request.

The earlier proposed format checks were:

| Format | Proposed checker |
| --- | --- |
| `date-time` | RFC 3339 date and time parser |
| `date` | Full calendar date parser, distinct from the date-time parser |
| `uri` | URI syntax checker; `url` is available but must not impose extra scheme restrictions |
| `uuid` | Structural hexadecimal groups of lengths 8-4-4-4-12 |
| `ipv4`, `ipv6` | Standard-library address parsing with syntax compatibility tests |
| `email` | One `@`, nonempty parts, and a dot in the domain |

**These are not adopted: formats are annotation-only (Open Questions, ruled 2026-10-21).** In particular, the proposed email rule can reject valid addresses and must not be described as having no false rejections. Unknown formats are preserved, exported, and identified as annotation-only in generation diagnostics and rustdoc. Do not treat a library accepting a string as proof that it implements the format standard.

Add `regex` to the runtime when its helpers land. Add `chrono` only if the chosen format policy needs it, without clock or timezone features for pure parsing. Record added dependencies in the area's dependency documentation and maintain compilation on every supported OS.

## Open Questions

### How should JSON bodies be checked when serialization changes their shape?

Registered bodies can be user-owned Rust types, with no generated accessors or validation trait. Serde can rename, flatten, omit, or transform fields. A check of Rust fields alone may reject omitted data or approve a different value from the one transmitted.

1. **Validate the serialized JSON representation and reuse it for sending — recommended.** Serialize once during actual request preparation, evaluate a generated schema plan, and encode that same representation. Pros: follows what is sent, handles custom serialization, and requires no new trait on user models. Cons: may require a temporary JSON tree and changes the earlier typed-before-encoding choice. Standalone `validate()` serializes separately; `.build()` and await remain separate preparations as today.
2. **Generate typed field traversal.** Pros: avoids a temporary tree for ordinary models. Cons: cannot generally inspect custom serializers or external private fields; needs restrictions or an additional model contract and can diverge from the wire representation.
3. **Require model-owned validation implementations.** Pros: lets authors optimize checks. Cons: spreads rule implementation across models, duplicates Schemars rules, and burdens imported and user-owned types.

Recommend the serialized representation because correctness about submitted values is the feature's purpose. Before implementation, record how the representation is reused so there is no serialize-check-reserialize gap. No new performance spike is required by this review; if the author wants one for the temporary-tree cost, keep it to one host and a quick sample.

**Ruled by the author on 2026-10-21:** option 1, validate the serialized representation and send that same representation. This replaces D2's typed-before-encoding check for JSON bodies; parameters are still checked on typed values.

### Which formats should reject requests?

Format assertions can prevent legitimate calls when a convenient parser implements a narrower grammar. The earlier format decision needs review because its email heuristic has exactly that problem.

1. **Preserve all formats as annotations for this feature — recommended.** Pros: predictable, standards-compatible default and no accidental rejection from approximate checks. Cons: catches fewer local mistakes and revises the earlier decision to assert several formats.
2. **Assert a documented, verified subset.** Pros: catches more errors without knowingly using heuristics. Cons: requires format-by-format compatibility evidence; any format not fully covered stays annotation-only, including email unless a suitable checker is chosen.
3. **Assert the original structural heuristics as an explicitly named Schematic policy.** Pros: small implementation. Cons: rejects valid provider inputs and cannot claim JSON Schema format compatibility.

Recommend annotation-only formats initially: length, range, and other explicit value assertions deliver the core protection without introducing unreliable local rejection. Resolve this question before adding format dependencies or promising format enforcement in docs.

**Ruled by the author on 2026-10-21:** option 1, all formats are annotations. This replaces D4's format assertions; no format-checker dependency is added.

### How should exact numeric rules be implemented?

Large integers and decimal divisors cannot safely be converted to `f64`; for example, a divisor of `0.1` needs decimal arithmetic semantics. This also affects deep numeric equality in `enum`, `const`, and `uniqueItems`.

1. **Use one exact decimal/rational helper for preserved JSON numbers — recommended.** Pros: consistent range, divisibility, and equality semantics. Cons: needs a carefully selected arithmetic dependency or a bounded exact-number implementation, and must integrate with parser/export precision.
2. **Support only exactly representable numeric constraints and reject the rest at generation.** Pros: smaller initial implementation and no silent approximation. Cons: common decimal constraints may prevent importing otherwise usable APIs; narrows the stated scope.
3. **Use floating-point checks with a tolerance.** Pros: little code. Cons: changes equality and divisibility meaning and can approve an invalid request or reject a valid one.

Recommend exact arithmetic because faithful constraint preservation is a stated goal. Choose the representation and dependency in the plan; do not quietly adopt a tolerance or clamp source bounds.

**Ruled by the author on 2026-10-21:** option 1, one exact decimal/rational helper.

## Validation and delivery

This is implementation acceptance coverage, not a request to run code tests for this document-only review. Use the area's `just test` and `just lint` recipes, `just doctest` for authoring examples, and `just test-schema` when committed generated clients change. Use the monorepo test toolkit and nextest conventions.

- **Definition checks:** valid and contradictory constraints, serialized definitions missing new optional metadata, malformed deserialized rules, unsupported-pattern handling, and Schemars capture on root and nested types.
- **Preservation:** one compact fixture covering every supported keyword through direct import/export and emitted definitions rebuilt through registration. Include inline roots, aliases, renamed properties, arrays and items, local recursive references, OpenAPI exclusive bounds, and `const` lowering. Test a large integer bound and a decimal `multipleOf` without narrowing.
- **Runtime semantics:** Unicode lengths, unanchored patterns, absent/empty/null distinctions, inclusive and exclusive endpoints, decimal divisibility, and deep array uniqueness including equal objects with different member order. Cover multiple violations and deterministic safe paths.
- **Send integration:** fresh generated code compiles and uses a mock transport. A violating request produces the established error shape and zero transport sends through build, ordinary await, `.full()`, direct `send`, and `send_raw`; a valid request sends unchanged data. Include direct request mutation after an early check, custom serialization once the body choice is resolved, and secret data in body values and map keys. Reuse the existing generation guard rather than building a new test harness.
- **Pager regression:** initial invalid size, invalid size changed during traversal, adopted sent defaults, and conflicting parameter/strategy bounds. GitHub's maximum stays 100 and its current controls remain available.
- **No response enforcement:** a structurally decodable reply outside a declared value bound still decodes as before.

Regenerate affected imported definitions and client/OpenAPI artifacts through the existing recipes; never hand-edit generated outputs. Update the current documentation for typed registration, definition builders, request generation, pagination, and OpenAPI conversion, along with the Schematic skills and dependency documentation where applicable. Generated field docs must distinguish enforced rules from preserved annotations or unsupported patterns. Feature references use their dated directory names alone; current topic documentation describes behavior without linking back to this specification.
