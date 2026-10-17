---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Recorded loopback-test flakes still need separate diagnosis
  - ElevenLabs continuation parameter remains unverified
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-12T21:18:57-07:00"
spec: "2026-10-12-schematic-runtime-envelope/spec.md"
implemented: false
description: "A **feature** review of `2026-10-12-schematic-runtime-envelope/spec.md`"
feature: "2026-10-12-schematic-runtime-envelope/review-3.md"
previous: "2026-10-12-schematic-runtime-envelope/review-2.md"
refresh: refreshed
refresh_changes:
  - Corrected the next spec's current-client example to use infallible GitHub::new().
  - Corrected the next spec's GitHub endpoint count to 16; OpenAI remains 265.
  - Recorded the implemented client surface, error-metadata split, elapsed-time behavior, runtime ABI 2, and manifest-based schema commands in the next spec's Refresh notes.
  - Updated the next spec's implementer note and refresh flag, and appended a dated refresh log; no plan exists to refresh.
---

# Runtime envelope review 3

**Production ready for this specification, with the author's accepted limitations.** Review 2's elapsed-time finding is repaired. The seven earlier author rulings remain carried out, and this review found no new high, medium, or low finding. No additional human design decision is required for this feature.

## Previous findings and author rulings

Review 2 lists one high finding in both its frontmatter and body, and has no new Human rulings section. Its observations were informational and did not require repairs. The governing author rulings remain in [review-1.md, Human rulings](./review-1.md#human-rulings). Where those rulings differ from the original specification, the rulings win: particularly the one-name client shape, endpoint-typed link checks, and retaining both partial-generation checks.

| Previous obligation | Verification against the current implementation | Result |
| --- | --- | --- |
| Review 2: buffered elapsed time covers the whole logical send | In schematic-runtime, [Client::send_unboxed and send_raw_unboxed](../../runtime/src/client.rs) finalize `meta.elapsed` after producing their outcome. The [whole-send elapsed tests](../../runtime/tests/l1/whole_send_elapsed.rs) exercise public typed and raw results against delayed socket bodies; the live-stream control also passes. [ResponseMeta](../../runtime/src/meta.rs) rustdoc and the runtime metadata, codec, and deadline topic pages describe the corrected behavior. | Implemented |
| Review 1, item 1: one API name, chained client settings, optional same-type validation, key and Basic convenience | In schematic-gen, [generate_api_struct](../../gen/src/codegen/api_struct/mod.rs) emits the API struct and internal tag. In schematic-runtime, [api_client!](../../runtime/src/api_client.rs) supplies the shared inherent surface, and [Client](../../runtime/src/client.rs) caches validation. Client-settings tests exercise invalid settings through `build`, typed/raw sends, and link checking with zero requests. Fresh generated fixtures compile and run; foreign requests fail compilation. Consumers and current documentation use this surface. | Implemented; endpoint calls remain assigned to the next feature |
| Review 1, item 2: settled decoder interfaces and hook-capable codec test obligation | In schematic-runtime, [Decode](../../runtime/src/codec.rs) accepts an owned live body through type-level functions; [ErrorBody](../../runtime/src/endpoint.rs) is separate. [Codec documentation](../../docs/runtime/codecs.md) records the interfaces and override-plus-test obligation. The later response-model specification carries that obligation. JSON hook and stream-ownership tests pass. | Implemented |
| Review 1, item 3: shared transport owns connection settings | In schematic-runtime, [ConnectionSettings](../../runtime/src/transport.rs) detects proxy, roots, invalid-certificate acceptance, and connect timeout on an attached client. Builder and client-settings tests exercise refusal. [Building clients](../../docs/runtime/building-clients.md) documents the ownership split and separate-transport guidance. | Implemented |
| Review 1, item 4: endpoint-typed link checking and checked absolute URLs | In schematic-runtime, [Client::check_link](../../runtime/src/client.rs) requires the request type; typed and raw sends check absolute request URLs through the same origin rules. Origin/redirect and generated-client tests pass. Current docs and the pagination handoff use the endpoint type. | Implemented |
| Review 1, item 5: narrow ElevenLabs query-parameter scope exception | `ListVoices` retains optional integer `page_size` and string `page_token`; biscuit-speaks uses the generated request. The implementation log records precisely that exception and preserved wire traffic, and the later definition feature includes it in its baseline. | Implemented |
| Review 1, item 6: retain runtime and generated-helper compatibility checks | In schematic-gen, the ABI check and shared-file comparisons remain. [Partial-generation documentation](../../docs/generation/partial-generation.md) explains the two questions and removal condition. Endpoint-method and WebSocket planning record helper-move candidates. Fresh-output ABI mismatch and partial-generation tests pass. | Implemented; this ruling supersedes immediate replacement of the byte comparison |
| Review 1, item 7: accepted connect-timeout effect gap and bounded Windows TLS investigation | The delivery test in schematic-runtime's [transport module](../../runtime/src/transport.rs) records the loopback limitation; ownership tests remain. The repair log records both the preconfigured-rustls attempt and debugger investigation. The named Windows aws-lc fix exists; all four TLS ignores name it; the OS skill records the diagnosis and evidence. | Implemented through the ruling's permitted fallback; the separate Windows fix remains open |

## Elapsed-time repair sweep

**Authority:** the specification's **Redirect construction and retained evidence** contract says, “Elapsed time covers the whole logical send.” Metadata requirements R4 and R27 apply to typed and raw replies. The stream default separately allows approximately time-to-head metadata for an output that keeps reading after the send returns.

**Defect class checked:** finalizing response timing before awaited body reading or classification omits part of the caller's wait.

The repair places one final assignment in each public send implementation, after both `Exchange::Final` and `Exchange::Stopped` branches. That covers every buffered outcome without a codec-specific switch. A live decoder returns its owned stream before later reads, so measuring when it returns preserves the stream contract. Hooks see the earlier head-time snapshot because they execute before the outcome exists; this is explicit in `ResponseMeta.elapsed` rustdoc.

The new socket fixture sends headers immediately, delays its body by 80 ms, and asserts at least 70 ms through the public envelope. Truncated cases advertise extra bytes and close. These delays are the stimulus being measured, rather than sleeps used to guess when a server is ready. The tests assert the outcome as well as elapsed time, so taking an unintended error branch cannot establish the success case.

| Sibling site in schematic-runtime | Shape checked | Result and contract |
| --- | --- | --- |
| Typed buffered success: `Json`, `Text`, `Binary`, `Empty` | Valid delayed body, including a body to drain | All four public envelopes include the body wait |
| Typed declared and undeclared failures | Delayed declared JSON and delayed undeclared 500 | Correct failure kinds; elapsed includes body reading |
| Typed decode failures | Invalid successful JSON and invalid declared-error JSON | `MalformedSuccess` / `MalformedError`; elapsed includes body reading and classification |
| Typed hook failure | Global hook refuses a valid delayed JSON body | `Hook`; elapsed includes the body wait; final assignment follows hook execution |
| Typed interrupted final body | Advertised length exceeds delayed bytes sent | `BodyInterrupted`; elapsed includes waiting until interruption |
| Execution core: refused redirects | Cross-origin, missing location, unsafe scheme | Retained redirect envelopes include delayed redirect-body reading |
| Execution core: interrupted redirect | Redirect body closes before advertised length | Retained 302 with `BodyInterrupted`; elapsed includes the wait |
| Raw complete/error/interrupted/refused outcomes | Delayed 200, 500, truncated body, refused 302 | Correct raw completion values; all include body wait |
| Followed final response, typed and raw | Immediate same-origin redirect followed by delayed JSON body | Final-hop metadata and whole-send elapsed are preserved |
| Failed next hop, typed and raw | Existing connection-failure, timeout, and deadline cases | Existing redirect/raw suites pass; both stopped branches now use their caller's common final assignment |
| Before-head and pre-send errors | Refused connection, stalled head, exhausted deadline, invalid settings/arguments | Existing transport/timing tests pass; no body work follows these results |
| Returned live stream | Head immediately, first line delayed | Send returns before the body delay; later reading still waits for the line. Existing idle-timeout and whole-stream deadline tests pass |
| Generated API forwarding and result projections | Macro forwards into the same typed/raw methods; response/raw accessors retain the record | No independent timing calculation exists to repair |

All seven new tests are declared by `tests/l1/main.rs`, which is the test target in schematic-runtime's `Cargo.toml` (`autotests = false`). Their names select ordinary L1 execution. In the full area run they took approximately 0.09–0.43 seconds each; none approaches the five-second ordinary-test budget. The shared helper move keeps the earlier timing tests in that same declared target.

## Findings

None. The previous high finding is closed, with no remaining sibling timing defect found in the specified paths.

## Observations

### Recorded loopback-test flakes still need separate diagnosis

The earlier repair log preserves native-Windows stress and WSL failures followed by passing runs. This review's local suites passed. A passing rerun does not establish the cause of those earlier failures; the released-port helper remains a plausible explanation for one symptom, rather than a confirmed general diagnosis. Keep the recorded transcripts for a separate investigation. The new elapsed fixture uses bound listeners and a policy-refused foreign origin rather than reserving and releasing a would-be closed port.

### ElevenLabs continuation parameter remains unverified

The existing biscuit-speaks pager and generated definition send `page_token`, while provider research names `next_page_token`. Review 1, item 5 explicitly accepts preserving that existing wire traffic, so this is not a regression or a missing runtime requirement. The pagination specification already records the question for provider verification before adopting a managed continuation parameter. No live-provider claim is made here.

## Verification and boundary fidelity

I compared the specification and previous reviews with the runtime send, codec, metadata, envelope, transport, deadline, origin, credential, hook, and redaction paths; generator/client and compatibility paths; generated-client tests; current documentation and the earlier repair handoffs. The implementation logs provide the earlier dependent-package and OS evidence; those were not rerun as part of this timing review.

Fresh local verification on macOS:

- `just test schematic-runtime`: 174 tests passed; the Cargo-dependent compile-fail test is excluded from this root recipe and was run below through the area recipe.
- `just test` in schematic: 1,882 ordinary tests passed, followed by six fresh-output tests and one runtime compile-fail test, all passing.
- `just test-schema` in schematic: all 37 generated-client and WebSocket tests passed.
- `just check-tier-coverage schematic`: passed; no stranded tests.

No production source was changed, no application windows were opened, and no formatting, commit, or push command was run. Cross-OS results are outside this review's readiness gate. The native-Windows TLS gap is accepted under the specific author ruling above, with its separate fix still required; connect-timeout enforcement relies on the HTTP library after verified ownership and delivery.

F1 means inputs and expectations authored here; F2 means a real compiler, HTTP stack, socket, clock, or other consumer observes the result; F3 means a real counterparty originates the input. Local mock payloads remain F1 evidence for provider wire formats even when sockets or generated Rust reach F2. The spec expressly requires local mocks and prohibits provider calls, so provider-originated F3 traffic is not an unmet requirement for this feature.

| User-observable requirement | Boundary and strongest relevant evidence | Assessment |
| --- | --- | --- |
| Shared runtime ownership and thin generation (R1, R31) | Generated Rust consumed by rustc: F2 fresh-output compilation and execution; F1 ownership/output assertions | Satisfied |
| Endpoint response types, API identity, removed generic response selection and HTTP-client injection (R2, R17) | Rust compiler: F2 positive and negative fresh generated-client compilation, plus runtime trybuild | Satisfied |
| Envelope accessors, failure predicates, uninhabited error placeholder (R3, R10–R13) | Internal semantics: F1 accessor/classification tests; rustc F2 rejects placeholder construction | Satisfied |
| Status, headers, method, auth, request id, page slot, and elapsed (R4) | F1 scripted field assertions; F2 compiled generated-client calls and actual delayed socket reads | Satisfied, including repaired whole-send timing; page remains the explicitly reserved slot |
| Stored-secret masking, safe diagnostic text, retained provider bodies (R5–R7) | Internal redaction at F1 across names/outcomes; actual socket failures at F2 test transport diagnostics | Satisfied within the specified body-Debug limitation |
| Before-head versus after-head failure placement (R8–R13) | Real HTTP parser and truncated socket bodies: F2; declared-status/body shapes authored at F1 | Satisfied |
| Credential precedence, ordered environment scan, one resolution and attempt (R14–R16) | Controlled environment at F1; compiled generated clients and recorded local requests at F2 | Satisfied |
| Managed settings, shared connections, ownership refusal (R18–R19; review 1, items 1 and 3) | Proxy/TLS peers, headers, connection counts and generated code: F2; connect-timeout delivery/ownership F1 | Satisfied under review 1, item 7's explicit effect-gap and Windows fallback ruling |
| Origin comparison, redirect policy, tightening, checked links/absolute requests (R20–R25; review 1, item 4) | F1 parsed-origin tables; real local origins and generated-client execution F2, including zero target requests on refusal | Satisfied |
| Redirect replay, method changes, refusal and failed-hop evidence (R24) | Real local HTTP requests at F2 with scripted redirect statuses at F1; corrected timing covered by delayed-body cases | Satisfied |
| Shared budgets, cancellation, completed count, idle and stream deadlines (R26, R30) | Real monotonic clock, socket shutdown, stream reads, runtime shutdown: F2; arithmetic at F1 | Satisfied |
| Raw results, completeness, JSON convenience and shared policy path (R27) | Real sockets and compiled generated clients: F2; disposition and convenience semantics F1 | Satisfied, including repaired timing |
| Removed response-kind guard, hook eligibility/order/evidence (R28–R29) | F1 hook and body shapes; compiled generated clients and compile-fail cases F2 | Satisfied |
| Future streaming and multi-shape decoder capability (R30; review 1, item 2) | Owned live socket stream and early first-item delivery F2; two-shape decoder F1; extension interface compiled | Satisfied; production stream protocols are deferred |
| Runtime ABI mismatch and partial-generation compatibility (R32; review 1, item 6) | Rustc against a doctored runtime and public generation on temporary output: F2 | Satisfied; both checks deliberately remain |
| Consumer migration, safe logging, WebSocket compilation (R33–R35) | Fresh generated-code/WebSocket compilation and local schema execution F2; consumer mapping/redaction tests F1 with earlier passing consumer gates recorded in the logs | Satisfied; consumer provider-body behavior is outside the metadata promise |
| Runnable area recipes and test placement (R37) | Actual just/nextest execution, declared targets, layout guards, tier audit | Satisfied |

No newly unmet requirement has evidence below the rung its boundary needs. R36 is withdrawn and needs no separate test.

The input robustness matrix does not apply: the feature adds Rust API-definition values and runtime request/response handling, not a configuration-file loader. The ABI reader inspects generated Rust, a text grammar excluded by the matrix. Its positive, absent, malformed, wrong-type, duplicate/conflicting, and differing-marker cases are exercised through the public partial-generation tests.

## Next-spec refresh

The [endpoint-method specification](../2026-10-12-schematic-endpoint-methods/spec.md) was refreshed after checking the implementation and repair logs, both previous reviews and author rulings, this specification's deferred scope, and relevant parent macro-review decisions. It had no `parent` property or existing Refresh notes section. Review 1, item 1 supplies the later authority that replaces the parent's earlier no-endpoint-wrapper decision; this review does not reopen it.

The factual refresh fixes the infallible-client example and GitHub count, records runtime ABI 2 and the implemented macro/client paths, and gives the next implementer the current error-metadata and timing contracts. In particular, schematic-runtime's `ApiError` carries kind and body while metadata remains on `Response`; the next feature must account for that existing split when implementing its already-decided combined-error accessor. Manifest-based schema commands are recorded without changing the decided measurement thresholds or fallback order.

The next spec's Decided requirements and author rulings were left unchanged. Its refresh flag and implementer note were updated, and a dated Refresh log was appended. Its plan does not exist, so only the spec was refreshed. No contested scope or design change requires a human decision.
