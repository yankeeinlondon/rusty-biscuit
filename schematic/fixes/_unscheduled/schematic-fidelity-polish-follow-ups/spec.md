---
area: schematic
status: unscheduled
created: 2026-10-23
owner: Ken Snyder <ken@ken.net>
origin: review 2 of 2026-10-18-schematic-fidelity-polish
related:
    - 2026-10-18-schematic-fidelity-polish
packages:
    - schematic-define
    - schematic-gen
    - schematic-runtime
---

# Schematic Fidelity Polish Follow-ups

These items are still open after the final review of
`2026-10-18-schematic-fidelity-polish`. That review found the feature
production ready with no findings. It recorded the three observations below
(R2-O1 to R2-O3). The repair cycle for review 1 also found two nonblocking
defects that only its log recorded (RL-1 and RL-2). See
[review 2](../../../features/2026-10-18-schematic-fidelity-polish/review-2.md)
for the full context.

## R2-O1: Inline JSON request schemas remain outside the supported typed import

**Defect.** When an OpenAPI request body's JSON schema is inline, such as an
array of a component or an inline object, the import produces untyped JSON
instead of a typed request. Only a direct `$ref` to
`#/components/schemas/…` imports as a named type.

**Where.** `schematic-define`, `reference_from_schema` in
`define/src/openapi/import/mappings/responses.rs`. The
`ReferenceOr::Item(_)` arm returns `JSON_VALUE`.

**Why not fixed now.** This behavior predates the feature, and review 1
excluded it from the repair inventory. Recovering types from inline schemas
needs a design decision: either map inline containers to `TypeRef` wrappers,
or synthesize named models for inline objects.

**What fixing it looks like.**

- Map `array` items, `additionalProperties` maps, and nullable wrappers
  recursively to `TypeRef::Array`, `TypeRef::Map`, and `TypeRef::Option`
  around a named reference.
- Decide whether inline objects become synthesized components and how they are
  named, and record the decision in `docs/io/import-openapi.md`.
- Add public import tests for each wrapper, mirroring the renamed-component
  tests in `define/tests/openapi_multipart_import.rs`.

## R2-O2: Existing runtime TLS tests exceed the Level-1 time budget

**Defect.** Four certificate tests take 5.8 to 7.5 seconds each. The Level-1
budget is 5 seconds.

**Where.** `schematic-runtime`, `runtime/tests/l1/builder.rs`:

- `an_added_pem_root_lets_the_client_reach_a_server_it_signed`
- `an_added_der_root_lets_the_client_reach_a_server_it_signed`
- `without_the_added_root_the_handshake_fails_as_a_tls_error`
- `accepting_invalid_certificates_reaches_a_self_signed_server_that_the_default_rejects`

**Why not fixed now.** This feature did not change these tests. A review
observation is not repaired in the follow-up pass.

**What fixing it looks like.** Profile one test to find where the time goes.
Likely candidates are `rcgen` key generation, which runs for every test in
`ca_signed_pki` and `self_signed_leaf`, and a connect or handshake timeout
that the failure case waits out. Generate the PKI once per binary (for
example with a `OnceLock`), and pass a short timeout through the builder for
the failing-handshake case. Keep that case's assertion that the error is a
TLS error. Do not use a `slow_` prefix or a timeout override as the fix.

## R2-O3: Whole-work closure remains separate from this feature

**Defect.** This feature is one part of a larger body of Schematic work. That
work cannot close until the following items close, and closing this feature
does not close them:

- the sniff handwritten-client migration, which removes
  `FocusedProviderClient` from sniff
- the verifying review of the predecessor `2026-10-15-schematic-definition-vocabulary`
- the earlier base-URL composition check

**Where.** `sniff/fixes/2026-10-09-consistent-use-schematic`, and the "Remaining
open items for the whole work" paragraph in this feature's implementation
log.

**Why not fixed now.** The changes are outside this spec's code and lifecycle.
Two items are separate reviews or another package area's spec.

**What fixing it looks like.** Schedule and land sniff's
`2026-10-09-consistent-use-schematic`. Run the verifying review of
`2026-10-15-schematic-definition-vocabulary`, and re-run the base-URL
composition check. Then confirm that
`rg FocusedProviderClient sniff` returns nothing.

## RL-1: AsyncAPI v2 import merges schemas whose names collide after PascalCase conversion

**Defect.** In an AsyncAPI v2 document, two schemas whose names map to the
same PascalCase identifier, such as `user_event` and `UserEvent`, import as
one model. One definition silently replaces the other.

**Where.** `schematic-gen`, `gen/src/asyncapi_import.rs`. The payload and
catalog naming paths share a version-specific transform, and that transform
has no collision handling.

**Why not fixed now.** The repair for review 1's rename finding (R1-F2) found
this defect. It predates that finding's defect class, and fixing it needs a
naming decision.

**What fixing it looks like.** Detect collisions after the transform. Either
reject them with a located diagnostic, or apply the OpenAPI importer's
collision rename and rewrite references the same way. Add a JSON and a YAML
fixture with a colliding pair.

## RL-2: A JSON multipart part that is an array of a component imports as untyped in plain documents

**Defect.** Take a plain OpenAPI document with no `x-schematic` metadata. If a
multipart part has JSON encoding and its schema is an array of a component,
that part imports as untyped JSON. This is the multipart counterpart of
R2-O1.

**Where.** `schematic-define`, OpenAPI multipart import under
`define/src/openapi/import/mappings/`.

**Why not fixed now.** It belongs to the same inline-schema class as R2-O1 and
should be fixed with it.

**What fixing it looks like.** Fix R2-O1 and route multipart part schemas
through the same recursive mapping. Add a plain-document test next to
`renamed_component_in_plain_multipart_part_is_followed`.
