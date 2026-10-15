---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Call build skips pre-send argument validation outside request encoding"
observations:
  - "Calling guide overstates required query and header arguments"
  - "Generated-schema behavior tests run locally but not in CI"
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-13T12:52:29-07:00"
spec: "2026-10-12-schematic-endpoint-methods/spec.md"
implemented: true
next: "2026-10-12-schematic-endpoint-methods/review-3.md"
implemented_by: claude/opus
log: schematic/features/2026-10-12-schematic-endpoint-methods/log.md
description: "A **feature** review of `2026-10-12-schematic-endpoint-methods/spec.md`"
feature: "2026-10-12-schematic-endpoint-methods/review-2.md"
previous: "2026-10-12-schematic-endpoint-methods/review-1.md"
---

# Review 2

**The feature is not production ready.** One high finding remains: early call validation accepts arguments that the send path subsequently rejects before making a request. The previous review's three findings are implemented. No further author decision is needed to repair the remaining contract gap.

## Previous findings and author decisions

The previous review's `## Human rulings` entries still say “pending.” However, its frontmatter explicitly records `human_resolved: true` and an author `DECISION` for each item. This review takes those explicit decisions as authority, as the repair log did; it does not reopen them.

| Previous finding | Implementation checked | Result |
| --- | --- | --- |
| Item 1: keep the combined error's named fields and record the departure | schematic-runtime's [CallError](../../runtime/src/call.rs) retains `Api { error, meta: Box<ResponseMeta> }`; `request_meta()` reads either variant. The implementation log's `Review 1 rulings` section records acceptance. Runtime tests exercise declared errors, malformed success bodies, interrupted bodies, transport failures, and error sources. Consumer matches retain wildcard arms. | Implemented. The author decision overrides the specification's decided tuple variant. |
| Item 2: keep generic `FullCall` and document setter order | schematic-runtime's [FullCall](../../runtime/src/call.rs), generated call rustdoc, the [calling guide](../../docs/runtime/calling-an-api.md), READMEs, and generator skill describe placing endpoint setters before `.full()`. The generator documentation test and fresh-output compile-fail case check the restriction. | Implemented. Shared settings remain available after `.full()`; endpoint-specific setters do not. |
| Item 3: keep homelab's fallback and review conversions and documentation | homelab's [RestError](../../../homelab/lib/src/rest.rs) keeps `Call`, now with transparent error chaining. Transport, fully read error-status, and other response failures retain their intended mappings. Its Rust tests pass, including the new cause-preservation test; the device-design error table matches the mappings. | Implemented. No currently produced error requires the future-variant fallback. |

The specification's explicit approval of the measured whole-command build-cost branch also stands: chained setters remain without per-API feature flags. Historical log entries and the spec's implementer note that call this approval outstanding are stale, not a reason to reopen it.

## Findings

### High: Call build skips pre-send argument validation outside request encoding

**Authority: specification requirement R5:** “`.build()` runs everything that can fail before network work: argument encoding (today's `InvalidArgument`)”. Requirement R6 also requires the endpoint-method and generic-request paths to behave identically. The new schematic-runtime [Call::build](../../runtime/src/call.rs) only calls `request.clone().into_parts()`. Other argument checks happen later in schematic-runtime's [request execution](../../runtime/src/client.rs): assembling headers and the body content type, resolving an accepted credential kind, and placing credentials without overwriting request values. These checks can fail as `InvalidArgument` before any network work, yet `.build()` reports success.

**Defect class:** early validation checks only one stage of request preparation, leaving other deterministic argument failures until the call is awaited.

A realistic example is a generic caller supplying Basic credentials to the generated GitHub client, which accepts a bearer token. Another is a token copied with a trailing newline. Both successfully build a call, then fail before sending. A caller using `.build()?` to validate work before storing or cloning calls receives the wrong validation result. The requests are still refused safely when awaited; the defect is the promised timing of validation.

This public-API reproduction uses the shipped GitHub definition and no live provider:

```rust
use schematic_schema::github::GitHub;
use schematic_schema::schematic_runtime::{CallError, Credential, TransportErrorKind};

let gh = GitHub::new().base_url(mock_server.uri())
    .env_order(Vec::<String>::new()).build().unwrap();
let call = gh.get_repository("octo", "widgets")
    .credential(Credential::basic("user", "password"));
assert!(call.clone().build().is_ok()); // Incorrect: this argument is already invalid.
let Err(CallError::Transport(error)) = call.await else { panic!("expected rejection") };
assert!(matches!(error.kind, TransportErrorKind::InvalidArgument(_)));
assert!(mock_server.received_requests().await.unwrap().is_empty());
```

The sweep covered every new call-validation entry and each argument-validation stage it reaches. A temporary, compiled integration test exercised the public API against a recording loopback server. All negative cases below were refused at await with zero requests recorded; the positive control built and successfully sent one request. The temporary test was removed from the repository after review; its source is retained locally at `/tmp/schematic-review-2-build-probe.rs`.

| Site or stage | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Generated GitHub endpoint call's `build` | Per-call Basic credentials on `get_repository` | `Ok(call)`; await returns `InvalidArgument` | `build` returns `InvalidArgument` |
| Generated GitHub call converted to `FullCall` | Same unsupported credential | `Ok(full_call)` | Same early rejection as the endpoint call |
| Generic GitHub `send(request)` call | Access token ending in newline | `Ok(call)`; await returns `InvalidArgument` | Early rejection of invalid credential header value |
| Runtime `Call` and `FullCall` through public request objects | Each header, body, query, and cookie case below | Both build paths accept all cases | Both build paths perform the same applicable argument checks |
| Request header assembly | Invalid header name; separately, header value ending in newline | Both build paths accept; await rejects each | Early rejection |
| Raw body content-type assembly | `text/plain` ending in newline | Both build paths accept; await rejects | Early rejection |
| Credential header placement | Request already sets `Authorization`, plus per-call bearer token | Both build paths accept; await rejects | Early rejection of conflicting values |
| Credential query placement | Request already sets `key`, plus an API key assigned to that query slot | Both build paths accept; await rejects | Early rejection of conflicting values |
| Credential cookie placement | Request already sets cookie `key`, plus an API key assigned to that cookie slot | Both build paths accept; await rejects | Early rejection of conflicting values |
| Request's own `into_parts` validation | Request returns an encoding `InvalidArgument` | `Call::build` and `FullCall::build` both reject | Clean: retain this behavior |
| Client configuration validation | Authentication-bearing default header on GitHub | Client `.build()` rejects with `AuthBearingDefaultHeader` | Clean: retain the separate client-setting check |
| Valid request control | Unedited request parts, no conflicting credential | Builds; await decodes a local 204 reply; one request recorded | Clean |

The header, raw-body, and credential-placement stages existed before this feature; they are listed because the new `.build()` contract is required to check their argument failures. This is not a request to change their send behavior. Unsupported credential kinds and copied token values demonstrate realistic failures independently of the synthetic generic-request cases.

**Repair:** share the existing request-argument preparation checks between early validation and sending, including the effective per-call credential where applicable. Keep send-time checks, since setters, environment values, and time can change after a successful build. Add assertions through the public call results for both response modes and the generated/generic entry points. This does not require a new escaping layer or another generated wrapper type; the validation logic already exists. Update the affected rustdoc and calling guide to describe the resulting validation scope accurately. Future value constraints remain outside this feature.

## Requirement and boundary evidence

The feature changes Rust call construction and generated code, not the providers' existing protocol definitions. Compiling generated output and exercising it through a local HTTP server is the appropriate F2 boundary for its new behavior. Live provider responses are not needed to establish that endpoint methods preserve the existing runtime behavior. None of the reviewed changes adds a configuration or file-format reader, so the Input Robustness Matrix does not apply.

| Requirement | Boundary and strongest evidence | Assessment |
| --- | --- | --- |
| One inherent method per REST endpoint, required arguments in existing constructor order, no immediate send | Rust compiler and HTTP send boundary, F2: fresh generated fixture compile/run and no-trait-import control. Source inventory finds 547 request structs and their 547 corresponding call structs; fresh shipped output compiles. | Met |
| Local call types, optional setters, shared settings, and collision handling | Rust compiler plus local HTTP server, F2: `an_endpoint_method_sends_its_values_and_awaits_to_the_data` and `renamed_endpoint_methods_and_setters_send_under_their_wire_names`; F1 resolver tests sweep reserved names, duplicate names, keywords, module suffixes, and the pinned shipped rename list. | Met |
| Await returns data or combined failure, retaining error evidence | Compiled runtime/generated clients and HTTP replies, F2: runtime `call` tests and the fresh-output endpoint test cover success, declared failure, malformed and interrupted bodies, and transport failure. | Met |
| Full response preserves status, headers, and request id | Compiled client and local HTTP server, F2: runtime `full_returns_the_envelope_for_a_success_and_for_an_error_reply` and schema `success_metadata_carries_request_id_status_and_headers`. Compile checks enforce setter ordering. | Met under accepted ordering |
| Early validation, cloneable built calls, repeated sends | Compiled client and recording server, F2: existing tests prove `into_parts` rejection and two sends from copies; the review probe proves additional pre-send argument checks are omitted. | Not met: high finding above |
| Public request objects and equivalent generic sends | Rust compiler and local HTTP server, F2: schema `endpoint_method_and_generic_send_of_its_request_agree`, runtime call tests, and fresh-crate wrong-API compile controls. | Met, with the common early-validation defect |
| Endpoint description, route, arguments, and setter rustdoc | Generated documentation text, F1: generator doc tests assert content, empty-section handling, and parameter descriptions. This is a content requirement, not an editor-rendering requirement. | Met |
| Before/after build measurements and generated size | Actual compiler measurements recorded in implementation-log.md, Tasks 1.4 and 4.5: paired clean builds, incremental builds, isolated OpenAI probe, source and rlib size. Whole-command clean build increased 4.5%; crate-only time increased 34%. | Met under the author's approved whole-command interpretation |
| Consumer migration | Compiled consumers and local HTTP fixtures, F2: homelab mappings verified this review; biscuit-speaks and sniff migration tests and lint passes recorded in Phase 6. Source audit distinguishes remaining unrelated reqwest, hand-written provider, and WebSocket sends from generated REST calls. | Met; prior consumer evidence reused |
| Calling guide with client settings, endpoint arguments, overrides, full replies, early checking, and request-object use | Written examples checked against generated signatures; behavior supported by the F2 rows above. | Required topics present; one nonblocking overstatement below |

Fresh-crate compilation is **F2**, not F3 as some implementation-log entries call it: no independent vendor schema or SDK sample supplies that extra boundary. The generated-client behavior is actually exercised against local servers; this review does not claim native provider-wire evidence.

## Verification

- `schematic/just test`: 1,925 ordinary L1 tests passed, 12 tier tests skipped; all six fresh-output checks and the one compile-fail harness passed.
- `schematic/just test-schema`: 39 tests passed, none skipped.
- `schematic/just lint`: passed.
- `just check-tier-coverage schematic`: passed; no stranded tests.
- Temporary public-API validation sweep via `just test-schema --test review_build_probe`: passed its assertions of the defective behavior and the valid control. No production code was changed.
- `homelab/just test`: all 244 Rust tests passed, with eight existing tier skips. The later frontend step failed because the local pnpm launcher references a missing `pnpm.cjs`. This environment failure is outside the changed Rust behavior; the whole recipe is not reported as passing. A first attempt to pass a narrow nextest filter through this recipe also failed at shell parsing, so the full recipe was used.

Consumer lint and biscuit-speaks/sniff suites were not rerun: their passing migration evidence is recorded in the implementation log, and this review changes only review metadata. No cross-OS results are claimed; those are left to CI as requested. No terminal or browser windows were opened.

## Observations

### Calling guide overstates required query and header arguments

The new [calling guide](../../docs/runtime/calling-an-api.md) says required values include “required query or header values.” In schematic-gen, [required_args](../../gen/src/codegen/endpoint_methods/mod.rs) preserves the existing request constructor's path-then-body argument order, while query/header/cookie values remain setters; the pre-existing [named parameter generator](../../gen/src/codegen/request_structs/named_params.rs) explicitly does not enforce `required`. The spec requires constructor parity, not stronger required-parameter enforcement. Narrow the documentation when convenient rather than expanding this feature's constructor contract.

### Generated-schema behavior tests run locally but not in CI

The schema crate is excluded from the workspace, and its own 39 tests run through `just test-schema`, rather than the normal CI package suites. This arrangement predates the feature. Fresh-output CI checks already compile and exercise the new generated methods against local servers, and runtime tests cover early validation and repeat sends, so the new behavior is not entirely dependent on the locally run schema suite. The author can separately decide whether to schedule that suite; this review does not add a CI gate.

The next pagination spec and plan were not refreshed because this review is not ready. Their refresh belongs after the remaining finding is repaired and readiness is established.
