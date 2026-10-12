# Typed-send spike findings

**Date:** 2026-10-11
**Spike code:** this directory (`Cargo.toml`, `src/`, `tests/`, `scripts/`). It is a throwaway prototype crate, not part of the workspace, and not shippable.
**Question:** does the design decided on 2026-10-09 (D3: typed request only; D7: an envelope `Result<Response<T, E>, TransportError>` whose inner outcome is a success or a failure) work in Rust, and what does it cost?

**Reading guide.** Schematic generates a Rust client for each REST API. Today a caller can ask for any response type (`request::<WrongType>(...)` compiles). D3 removes that: each request type fixes its response type. D7 says every call returns one envelope carrying status, headers, and the body, whether the call succeeded or not. This spike builds both and tries them against sniff's real call sites.

**Status labels.** *Verified here*: checked against the code or tests in this directory on 2026-10-11 (counts and presence only). *Reported, not re-run*: from the spike run's report. *Unverified*: the report says it was not tested.

## Result

The decided design works. No blocking defect was found. Several semantic details need author rulings (see "Risks needing rulings").

## Tests

| Suite | Count | Status |
|---|---|---|
| Fixture tests (`tests/fixtures.rs`) | 28 | Count verified; pass reported |
| Ported sniff call-site tests (`tests/sniff_port.rs`) | 5 | Count verified; pass reported |
| Ergonomics test (`tests/ergonomics.rs`) | 1 | Count verified; pass reported |
| trybuild test (`tests/compile_fail.rs`), 8 cases in `tests/ui/` | 1 test, 8 cases | Case count verified; pass reported |
| Doctests (`src/lib.rs`) | 7 (6 `compile_fail`, 1 positive) | Count verified; pass reported |

## The shape that was tested

```rust
trait Endpoint {
    type Success;                 // no serde bound; a stream endpoint's Success is an item stream
    type Failure: ErrorBody;      // or the uninhabited NoDeclaredError
    type Decode: Decode<Self::Failure, Output = Self::Success>;
    const ENDPOINT_ID: &'static str;
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16>; // default 400..=599
}
// Decode is async trait-like: decode(resp, ctx) -> impl Future<Output = Result<Output, ApiError<E>>>.
// It receives the live response, which is why one generic send covers streaming too.

struct Response<T, E> { meta: ResponseMeta, outcome: Outcome<T, E> }
enum   Outcome<T, E>  { Success(T), Failure(ApiError<E>) }
struct ApiError<E>    { kind: ErrorKind<E>, body: Bytes }   // raw body always kept
enum   ErrorKind<E>   { Declared(E), Undeclared, MalformedError, MalformedSuccess,
                        UnexpectedContentType, BodyInterrupted, Hook }
// ErrorKind should be #[non_exhaustive]; the spike does NOT mark it (verified).
struct ResponseMeta   { request: RequestMeta, status, headers, elapsed, request_id }
struct RequestMeta    { endpoint_id, method, url, headers, query /* redacted */,
                        auth: AuthMethod, page }
struct TransportError { request: RequestMeta, kind, elapsed }
enum   AuthMethod     { PerCall, ClientLevel, EnvVar(name), Anonymous }
```

Codecs: `Json<T>`, `Text`, `Binary`, `Empty`, `Sse<T>`, `Ndjson<T>`, `GraphQl<D>`. A multi-shape success is a generated enum that implements `Decode` itself. GraphQL is a blanket impl over a per-operation trait (string document, typed `Variables`, typed `Data`). `NoDeclaredError` is an uninhabited enum (verified in `runtime.rs`).

## Verdicts

- **Q1 One generic send for everything?** Yes, including streaming and GraphQL. No `send_stream` split. A test shows incremental delivery.
- **Q2 Hooks?** `pre_response_json` and `mutate_response::<R>` survive. Hooks on text, binary, or stream endpoints become compile errors (today they silently never run). GraphQL trap: a mutator must run on typed `data`, not the `{data, errors}` wrapper.
- **Q3 Wrong types fail readably?** Yes (E0308, E0599, E0631, E0277, E0271, E0046). A cross-API request error is emitted 3-4 times and rustc truncates the list to "and 542 others" at 550 variants.
- **Q4 Compile cost?** Negligible for the trait (reported, not re-run; host load average reached 165). Synthetic 550 request structs + 550 response structs + 550-variant enum, rustc 1.98.1, M4 Max, 5 interleaved rounds, minimum seconds: none 0.70; today's `EndpointSpec` x550 11.54; new `Endpoint` x550 11.70; new x550 plus 550 `send` calls 18.54 (about 7 s extra, so do NOT generate a wrapper per endpoint). Incremental rebuild inconclusive.
- **Q5 Ported sniff call site?** More readable than today (reported). `focused.rs` `fetch_json` (~50 lines) + `forbidden()` (~35) and `gitlab.rs` `map_schematic_error` (~64) shrink. 404 maps to `Ok(None)`. One `From<HttpFailure<GhError>>` (~30 lines) holds status policy: 401 split on `auth == Anonymous`; 403/429 rate limit from `x-ratelimit-remaining` or the body; `Retry-After` parsed from failure headers. `list_pulls` reads `resp.header("link")` and returns Incomplete instead of a partial list. Today's generated path cannot read headers (`SchematicError::ApiError` has none, so `retry_after` is always `None`). Friction: a 2xx body that fails to decode arrives as a failure with status 200 (check `MalformedSuccess` first; add `is_decode_failure()`).
- **Q6 Ergonomics?** Fine: `send(r).await?.into_result()?`, `.into_data()?`, peek via `data()`/`error()`, exhaustive match. Downsides: `into_result` drops success metadata (use `into_parts`); `CallError<E>` is generic over the endpoint error type, so mixing endpoints needs an own error enum; mark `ErrorKind` `#[non_exhaustive]`.
- **Q7 GraphQL errors in a 200?** On the failure side as `Declared(GraphQlFailure { errors, partial_data })`, status 200, raw body kept. Least bad: `?` cannot silently ignore them; partial data kept. Streams: `elapsed` measures time to headers; mid-stream errors are distinct `StreamError` items.

## Risks needing rulings (none decided here)

1. D21 "sentinel never appears in debug output" cannot hold if a provider echoes the token in its error body: metadata and `Display` stay clean, the decoded `E`'s `Debug` shows it. Narrow D21 to stored metadata, or hide decoded bodies in `Debug`.
2. `reqwest::Error` `Display` includes the full URL (query secrets): never store it. Today's `SchematicError::Http` would leak a query-string API key.
3. The generator must split query parameters out of `RequestParts` / `into_parts` (today formatted into the path).
4. Pre-send failures (missing credential, auth required): the spike made them `TransportError`; this moves sniff's exhaustive matches. Needs a ruling.
5. Body interrupted after headers: the spike made it `Failure(BodyInterrupted)`; sniff treats it as transport. Needs a ruling.
6. A 2xx that fails to decode is a failure with status 200.
7. Declared error statuses live on the endpoint in the spike; D6 is open.
8. `Success` restates `Decode::Output`: keep (compile-checked, readable errors).
9. The erasing request enum is still needed for `into_parts` and per-client scoping.
10. The `[DONE]` marker is hard-coded (D6 open); stream responses are not `Clone` or `Sync`.
11. `pre_response_json` is client-global and also sees the GraphQL wire body.

## Not tested

Real providers and the generator; D12 beyond redirect denial; D13 deadlines; real pagers; multipart/form/binary request bodies; HTTP/2; non-UTF-8 headers; stream cancel on drop; custom decoding path (D3 open); same-URL streaming and non-streaming (D6 open); Linux/Windows/WSL2 (macOS only); real schematic-schema compile time; rust-analyzer latency; incremental compile time. The trybuild `.stderr` snapshots contain `kache`-remapped `/kache/workspace` paths; regenerate with `TRYBUILD=overwrite`.
