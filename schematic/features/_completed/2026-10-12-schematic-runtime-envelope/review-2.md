---
$schema: feature-review.yaml
ready: false
findings:
  - title: Buffered responses report header time instead of whole-send elapsed time
    priority: high
observations:
  - Recorded loopback-test flakes lack a confirmed cause
  - ElevenLabs continuation parameter remains unverified
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-12T21:04:52-07:00"
spec: "2026-10-12-schematic-runtime-envelope/spec.md"
log: "schematic/features/2026-10-12-schematic-runtime-envelope/log.md"
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-12-schematic-runtime-envelope/spec.md`"
feature: "2026-10-12-schematic-runtime-envelope/review-2.md"
previous: "2026-10-12-schematic-runtime-envelope/review-1.md"
next: "2026-10-12-schematic-runtime-envelope/review-3.md"
---

# Runtime envelope review 2

**Not production ready.** One high finding remains: buffered replies report time to headers rather than time to completion. The seven first-review repair obligations are implemented. No further author decision is needed to repair this finding.

## Previous findings and author rulings

The authority here is the **Human rulings** section of review-1.md. Where it differs from the original specification, the ruling wins. In particular, the one-name client replaces both the marker/client-alias implementation and the original forwarding-wrapper default; the endpoint-method feature owns the later call surface.

| Ruling in review-1.md | Verification | Result |
| --- | --- | --- |
| Item 1: one API name, chained settings, optional validation, API-key and Basic convenience | In schematic-gen, [generate_api_struct](../../gen/src/codegen/api_struct/mod.rs) emits the API struct and internal tag. In schematic-runtime, [api_client!](../../runtime/src/api_client.rs) supplies the shared surface and [Client](../../runtime/src/client.rs) caches validation, reports invalid settings before network work, and returns the same type from `build`. Basic convenience is emitted for APIs accepting Basic. Runtime client-settings tests and compiled generated-fixture tests exercise this behavior; foreign requests fail compilation. Consumers, current docs, and skills use the new shape. | Implemented; endpoint call methods remain assigned to the next feature. |
| Item 2: decoder interfaces and tests required for future hook-capable codecs | [Decode](../../runtime/src/codec.rs) in schematic-runtime remains type-level, consumes owned `LiveBody`, and provides `decode_hooked`; error decoding remains separate. [Codec documentation](../../docs/runtime/codecs.md) explains these interfaces, structural checking, and the override-plus-hook-test obligation. The response-model spec carries that obligation for each codec implementing `SupportsHooks`. Current JSON hook tests pass. | Implemented. NDJSON being the expected first extension is a forecast, not a requirement to ship it here. |
| Item 3: shared transport owns every connection setting | [ConnectionSettings](../../runtime/src/transport.rs) in schematic-runtime checks proxy, roots, invalid-certificate acceptance, and connect timeout. Attached-client conflicts surface at explicit validation or first use. Runtime tests cover the conflict class. [Building clients](../../docs/runtime/building-clients.md) records the ownership table and separate-transport guidance. | Implemented. |
| Item 4: endpoint-typed link checks and checked absolute URLs | [Client::check_link](../../runtime/src/client.rs) in schematic-runtime takes `R`; typed and raw sends share origin checking for absolute request paths. Redirect tests cover refused and permitted links. Current origin docs and the pagination spec and plan name the request type and checked send path. | Implemented. |
| Item 5: only the two ElevenLabs query parameters are an allowed scope exception | `ListVoices` declares optional integer `page_size` and string `page_token`; biscuit-speaks uses the generated request. The implementation log's **Review 1 repair** section records the precise departure and unchanged wire traffic. The later definition spec includes both in its baseline. | Implemented; no additional endpoint change is authorized. |
| Item 6: retain two compatibility checks until generated shared helpers move | The ABI check and shared-file byte checks both remain in schematic-gen. [Partial-generation documentation](../../docs/generation/partial-generation.md) separates their purposes and records the planned removal condition. Endpoint-method and WebSocket planning identify helper-move candidates. | Implemented; the ruling supersedes the spec's instruction to replace the byte comparison immediately. |
| Item 7: accepted connect-timeout effect gap and bounded Windows TLS repair | The delivery test in schematic-runtime's [transport module](../../runtime/src/transport.rs) records the loopback limitation beside its assertion; ownership tests remain. The repair log records a failed preconfigured-rustls attempt, debugger captures identifying aws-lc entropy initialization, and a control build with no abort in 1,000 TLS runs. The named fix spec exists; all four Windows ignores name it; the OS skill records the diagnosis. | Implemented through the ruling's fallback. Native-Windows TLS tests remain excluded pending that separate fix, as the author explicitly permitted after both attempts. |

## Findings

### High: Buffered responses report header time instead of whole-send elapsed time

**Authority:** the specification's **Redirect construction and retained evidence** contract says, “Elapsed time covers the whole logical send.” Requirements R4 and R27 expose that metadata on typed and raw replies. The separate stream default in **Planning-time details**, “elapsed for streams,” permits time to the response head for a returned live stream; it does not exempt buffered replies.

**Defect class:** response timing is finalized before an awaited body read or classification finishes, so buffered outcomes omit time the caller actually spent waiting.

In schematic-runtime, [Client::send_unboxed and Client::send_raw_unboxed](../../runtime/src/client.rs) return the metadata captured by the execution core at header arrival. Neither updates `elapsed` after its body work. The execution core also keeps the header-time record when reading a redirect body ends in refusal or interruption. Its failed-next-hop branch does refresh elapsed, which explains why the existing redirect-timeout tests pass.

This is ordinary server behavior: headers arrive quickly while a download, JSON body, or stalled response arrives later. A caller can wait almost the entire request timeout and receive a `BodyInterrupted` envelope reporting less than a millisecond. Timing used for diagnostics then hides the slow part of the operation.

**Reproduction:** I temporarily declared an ordinary test module in schematic-runtime's existing `l1` binary and ran it with `just test schematic-runtime`. It used the existing request and declared-error fixtures through public `Client::send` and `send_raw`. A loopback TCP server sent a response head immediately, waited 80 ms, then sent the body. For interrupted cases it advertised 100 additional bytes and closed after sending the partial body. For followed-success controls, a wiremock origin redirected to that same delayed-body server. Both request paths were tested against a permitted failed next hop as clean controls. The temporary test and its module declaration were removed after reproduction.

The handler's distinguishing behavior was:

```rust
read_head(&mut stream).await;
write_all(&mut stream, response_head.as_bytes()).await;
tokio::time::sleep(Duration::from_millis(80)).await;
write_all(&mut stream, body).await;
```

The sweep covers both public send paths, every built-in decoder, both error-body decisions, hook rejection, interrupted final and redirect bodies, refused redirects, successful followed replies, and failed next hops. The generated API structs forward into these same paths and introduce no separate timing projection.

| Site in schematic-runtime | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Typed final reply: `Json<Item>` | Valid 200 JSON, delayed body | Metadata 0.64 ms; send 82.32 ms | Include body wait |
| Typed final reply: `Text` | 200 text, delayed body | Metadata 0.51 ms; send 81.80 ms | Include body wait |
| Typed final reply: `Binary` | 200 bytes, delayed body | Metadata 0.57 ms; send 82.62 ms | Include body wait |
| Typed final reply: `Empty` | 200 with a body to drain | Metadata 0.65 ms; send 82.98 ms | Include drain wait |
| Typed declared-error decoding | 400 valid error JSON | Metadata 0.62 ms; send 82.98 ms | Include body/decode work |
| Typed undeclared-error decision | 500 error body | Metadata 0.66 ms; send 82.91 ms | Include body wait |
| Typed malformed-success decoding | 200 invalid JSON | Metadata 0.61 ms; send 81.90 ms | Include body/decode work |
| Typed malformed-error decoding | 400 invalid declared JSON | Metadata 0.67 ms; send 82.53 ms | Include body/decode work |
| Typed hook rejection | Valid 200 JSON; global hook refuses | Metadata 0.68 ms; send 82.62 ms | Include body/hook work |
| Typed interrupted final body | 200, advertised body longer than bytes | Metadata 0.56 ms; send 81.05 ms | Include wait until interruption |
| Execution core: refused redirect | 302 to another origin | Metadata 0.55 ms; send 83.58 ms | Include redirect-body wait |
| Execution core: missing redirect location | 302 without `Location` | Metadata 0.66 ms; send 83.03 ms | Include redirect-body wait |
| Execution core: unsafe redirect location | 302 with `ftp://` target | Metadata 0.72 ms; send 82.68 ms | Include redirect-body wait |
| Execution core: interrupted redirect body | 302, advertised body longer than bytes | Metadata 0.65 ms; send 82.19 ms | Include wait until interruption |
| Raw final reply | Complete 200 body | Metadata 0.62 ms; send 81.82 ms | Include body wait |
| Raw final reply | Complete 500 body | Metadata 0.62 ms; send 82.50 ms | Include body wait |
| Raw interrupted body | 200, advertised body longer than bytes | Metadata 0.64 ms; send 82.82 ms | Include wait until interruption |
| Raw refused redirect | 302 to another origin | Metadata 0.60 ms; send 82.92 ms | Include redirect-body wait |
| Typed successful followed reply | 302 then delayed 200 body | Metadata 1.15 ms | Include final-body wait of at least 80 ms |
| Raw successful followed reply | 302 then delayed 200 body | Metadata 1.45 ms | Include final-body wait of at least 80 ms |
| Typed failed next hop — clean control | 302 body delayed, then connection refusal | Metadata 83.00 ms | Includes completed body wait: passed |
| Raw failed next hop — clean control | Same failed-hop shape | Metadata 83.22 ms | Includes completed body wait: passed |
| Pre-head transport failures — existing clean controls | Stalled head and shared-deadline expiration | Passing `timing` tests assert error elapsed includes wait | Preserve current behavior |
| Returned live stream — separate contract | Decoder returns owned body before it ends | Existing stream tests prove early return; metadata captured at head | Preserve time-to-head semantics |

**Repair:** finalize timing for buffered outcomes after body reading and classification, including stopped redirects and raw responses. Preserve the documented time-to-head behavior for live streaming outputs; the codec boundary needs to distinguish that case. Update the affected rustdoc and metadata topic page in the same change. Add delayed-body assertions through the public typed and raw results, with a live-stream control. Existing tests delay response headers or check how long the future waited; neither detects the omitted body duration.

This requires a focused timing correction at the runtime/codec boundary, rather than another transport or escaping layer.

## Observations

### Recorded loopback-test flakes lack a confirmed cause

The repair log records non-TLS failures during native-Windows stress and an ordinary WSL run, followed by a passing WSL rerun. Symptoms include unexpected connection failures, extra requests, and a failed-next-hop test receiving `Undeclared`. My local suites passed. The runtime test helper `closed_port_url` releases its port reservation before use, and one redirect test obtains that URL before starting another server; port reuse is a plausible explanation for that specific symptom, not an established diagnosis for all recorded failures. Preserve the failure transcripts for separate diagnosis rather than treating a rerun as proof of stability.

### ElevenLabs continuation parameter remains unverified

The inherited biscuit-speaks pager and generated definition send `page_token`, while the repository's provider research and later pagination table name `next_page_token`. Review 1, item 5 expressly accepts preserving the existing wire traffic, so this is neither a regression nor an unmet runtime requirement. The pagination spec now records the discrepancy for confirmation before assigning the pager-managed parameter; this review does not establish which name the live provider accepts.

## Verification and boundaries

I read the specification, both implementation logs, the previous review and its rulings, the runtime implementation and suites, generation/compatibility paths, generated clients and tests, migrated call sites, current runtime docs, affected skills, and the downstream handoffs relevant to the rulings.

Local verification on macOS:

- `just test` in schematic: 1,875 ordinary tests passed; six fresh-output tests and one runtime compile-fail test passed.
- `just test-schema` in schematic: all 37 generated-client and WebSocket tests passed.
- Temporary public-API elapsed sweep: reproduced the single finding across 20 failing cases and two passing failed-hop controls; the other 167 runtime tests passed.
- After removing the probe, `just test schematic-runtime` passed again.

No production source was changed, no application windows were opened, and no formatting, commit, or push command was run. Cross-OS receipts are outside this review's readiness gate. I inspected the prior recorded OS investigation rather than repeating its stress campaign.

Fidelity below describes what the assertions depend on, not the test name. Scripted HTTP payloads are F1 evidence for provider wire formats; compiling generated clients and exercising them against a real local server is F2 evidence for generated-code behavior. This spec explicitly prohibits contacting real providers and requires local mocks, so provider-originated F3 payloads are not a requirement here.

| User-observable requirement | Boundary and strongest relevant evidence | Assessment |
| --- | --- | --- |
| Runtime ownership and thin generation (R1, R31) | Generated Rust consumed by rustc: F2 fresh-output compilation; F1 output assertions establish ownership | Present |
| Correct response types, API identity, removed generic request/client injection (R2, R17) | Rust compiler: F2 generated-code positive and negative compilations, runtime trybuild | Present |
| Envelope accessors, failure predicates and placeholder error (R3, R10–R13) | Internal contract: F1 accessor/classification cases; compiler F2 rejects constructing the placeholder | Present |
| Status, headers, request id, method, auth and elapsed metadata (R4) | F1 scripted HTTP fields; F2 compiled generated-client execution and actual socket/timing waits | Present except whole-send elapsed: finding above |
| Secret masking and safe text; retained provider body (R5–R7, R34) | Internal redaction: F1 name tables and sentinel assertions across outcomes and consumers; real sockets exercise transport errors | Present; documented provider-body `Debug` limitation retained |
| Before-head versus after-head failure placement (R8–R13) | Actual HTTP parser, sockets and truncated bodies: F2, with scripted status/body semantics at F1 | Present |
| Credential precedence, ordered environment fallback, one resolution and attempt (R14–R16) | Controlled process environment at F1; compiled generated clients exercised against a local server at F2 | Present |
| Client settings and shared connections (R18) | Local proxy, TLS peer, request headers, keep-alive socket count: F2; connect-timeout delivery/ownership F1 | Present within review-1 item 7's accepted effect gap and tracked Windows exception |
| One-name client, deferred validation, same-type build, key/Basic convenience (review-1 item 1) | Rustc plus freshly generated clients executing against a local server: F2 | Present |
| Origin policy and tightening (R19–R23, R25) | Internal comparisons: F1 tables; real local origins and compiled generated-client execution: F2 | Present |
| Redirect method/body replay, refusal and failed-hop evidence (R24) | Actual HTTP requests to local servers: F2; statuses and redirect targets scripted at F1 | Present except elapsed on stopped/read-complete outcomes |
| Shared deadlines and cancellation (R26) | Monotonic clock, sockets, connection close and runtime task shutdown: F2; arithmetic F1 | Present |
| Raw result, completeness and shared policy path (R27) | Real sockets and compiled generated clients: F2; JSON convenience/internal disposition F1 | Present except whole-send elapsed |
| Guard removal and hook ordering/retained evidence (R28–R29) | F1 scripted bodies/hooks; F2 compiled generated-client calls and compile-fail misuse checks | Present |
| Live decoder ownership, early item delivery and later bounds (R30) | Real socket streaming/cancellation/timing: F2; multi-shape codec F1 | Present; no production stream protocol is promised here |
| ABI mismatch and partial-generation protection (R32) | Rust compiler consuming doctored runtime: F2; public generation over actual temporary output: F2 | Present; both compatibility mechanisms remain |
| Dependent migration and WebSocket compilation (R33, R35) | Compiled consumers/generated modules: F2; consumer status mapping F1 | Present; prior consumer gates recorded in the repair log |
| Runnable recipes and test placement (R37) | Declared `l1` target/modules, layout guards, and actual just/nextest execution | Present; explicit fresh-output and compile-fail recipes run the Cargo-dependent tests |

The input robustness matrix does not apply: this feature's new API fields are Rust definition values without file deserialization, and its ABI reader parses generated Rust source, a text grammar. No new configuration-file loader was introduced. The ABI reader's absent, malformed, duplicate, differing, and positive-control cases were nevertheless checked through the existing public partial-generation tests.

The next endpoint-method spec was not refreshed because this review is not ready. Its plan does not yet exist.
