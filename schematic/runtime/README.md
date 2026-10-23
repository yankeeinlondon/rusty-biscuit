# schematic-runtime

The hand-written request runtime shared by every REST client that
`schematic-gen` generates into `schematic-schema`.

## Overview

Generated code stays thin: per API it declares a client struct, a tag type,
and an `ApiDefinition`, and per endpoint a request struct implementing
`EndpointSpec`, plus an endpoint method and a call type that wrap it. Everything that executes a request lives here, written once
and tested once instead of being regenerated into every API module:

- a managed `Client<A>`, created infallibly and configured by chained
  settings that are validated on its first call or by an optional
  `build()`, that sends only the requests of its own API
- the `api_client!` macro, which gives each generated API struct those
  settings and sends as inherent methods, written once here
- a `Call` that nothing sends until it is awaited, which resolves to the
  decoded value or a `CallError`, accepts per-call overrides (`anonymous`,
  `credential`, `deadline`), and with `.full()` becomes a `FullCall` that
  returns one `Response` envelope for every reply that arrived
- a generic `send(request)` that returns the same `Call` for any request of
  the API, and a `send_raw` that returns the bytes undecoded
- credential resolution: per call, per client, from environment variables,
  or anonymous
- redirect following under a per-API cross-origin policy, with the
  credential attached only where the policy allows
- a per-request timeout and a `Deadline` shared across many sends
- the built-in codecs: `Json`, `Text`, `Binary`, and `Empty` for buffered
  bodies; `Sse`, `Ndjson`, and `ByteStream` for bodies read item by item;
  `Shapes` for an endpoint with several success shapes; and `GraphQl` for a
  GraphQL reply
- declared error bodies, chosen by status from each endpoint's error table
- JSON response hooks
- metadata that is redacted when it is captured, so it is always safe to log

The runtime never retries, and it cannot be handed an HTTP client: it builds
every HTTP client itself, with the library's own redirect following turned
off, so every request goes through the policy above.

## Quick Start

Most code uses the runtime through a generated client:

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GitHub::new()
        .api_key(std::env::var("MY_TOKEN")?)
        .build()?; // optional: BuildError now instead of on the first send

    // CallError: no response head arrived, or a failed reply with its raw body
    let repo = client.get_repository("rust-lang", "rust").await?;
    println!("{}", repo.full_name);

    // `.full()` keeps the metadata: Ok whenever a response head arrived
    let response = client.get_repository("rust-lang", "rust").full().await?;
    println!("{} (request id {:?})", response.meta.status, response.meta.request_id);
    Ok(())
}
```

`schematic-schema` re-exports this crate as `schematic_schema::schematic_runtime`,
and its prelude re-exports `Call`, `CallError`, `CallOptions`, `Credential`,
`Deadline`, `FullCall`, `OriginPolicy`, `RawResponse`, `Response`, and
`TransportError`. [Calling an API](../docs/runtime/calling-an-api.md) walks
through endpoint methods, setters, per-call overrides, and `CallError`.

## The Governing Rule

A `.full()` call returns `Ok` if and only if an HTTP response head arrived.
Every status, including an undecodable or cut-off body, is reported inside the
envelope with its raw bytes. `Err(TransportError)` means no head arrived: the
request was refused before sending (invalid argument or URL, cross-origin
target, expired deadline) or the connection, TLS handshake, or wait for the
first head failed.

Awaiting a call without `.full()` folds the same split into one error:
`CallError::Transport` when no head arrived, and `CallError::Api` carrying the
`ApiError` (with its raw body) and the `ResponseMeta` when a reply arrived but
was not a decoded success. `CallError` is `#[non_exhaustive]`, so a `match`
outside the runtime needs a wildcard arm:

```rust
use schematic_schema::schematic_runtime::ErrorKind;

match client.list_releases("rust-lang", "rust").await {
    Ok(releases) => { /* typed value */ }
    Err(CallError::Transport(transport)) => eprintln!("no response: {transport}"),
    Err(CallError::Api { error, meta }) => match error.kind {
        ErrorKind::Undeclared => eprintln!("{}: {} bytes", meta.status, error.body.len()),
        ErrorKind::MalformedSuccess => { /* 2xx that did not decode */ }
        _ => eprintln!("{error}"),
    },
    _ => eprintln!("a failure added after this code was written"),
}
```

## Documentation

The topic pages in [`schematic/docs/runtime/`](../docs/runtime/README.md)
explain each behavior with examples:

| Topic | Page |
| --- | --- |
| endpoint methods, setters, per-call overrides, `Call`, `FullCall`, `CallError`, `.build()` | [Calling an API](../docs/runtime/calling-an-api.md) |
| `send`, `send_raw`, the envelope, error kinds, codecs | [Sending and responses](../docs/runtime/sending-and-responses.md) |
| client settings and when they are validated, `SharedTransport`, `BuildError` | [Building clients](../docs/runtime/building-clients.md) |
| credential precedence and anonymous calls | [Credentials](../docs/runtime/credentials.md) |
| cross-origin policies, redirects, `check_link` | [Origins and redirects](../docs/runtime/origins-and-redirects.md) |
| timeouts and shared deadlines | [Deadlines and timeouts](../docs/runtime/deadlines-and-timeouts.md) |
| `pre_response_json` and `mutate_response` | [Response hooks](../docs/runtime/hooks.md) |
| the `Decode` and `ErrorBody` traits, `LiveBody`, streams, several shapes, writing a codec | [Codecs and decoding](../docs/runtime/codecs.md) |
| GraphQL addresses, reply classification, and paging an operation | [GraphQL operations](../docs/graphql.md) |
| `RequestMeta`, `ResponseMeta`, request ids, redaction | [Metadata and redaction](../docs/runtime/metadata-and-redaction.md) |
| pagers, explicit retry, `collect_all`, `Incomplete`, `PagerError`, sessions, partitions | [Pagination](../docs/pagination.md) |

The crate-level rustdoc (`cargo doc -p schematic-runtime --open`) documents
every type.

## Module Layout

| Module | Contents |
| --- | --- |
| `client` | `Client` and its settings, `ApiDefinition`, `build`, `send` (returns a `Call`), `send_raw`, `check_link`, `DEFAULT_TIMEOUT`, `MAX_REDIRECTS` |
| `call` | `Call`, `FullCall`, `CallError`, `CallFuture` |
| `response` | `Response`, `Outcome`, `ApiError`, `ErrorKind`, `RedirectRefusal`, `FailureCategory`, `NoDeclaredError` |
| `error` | `TransportError`, `TransportErrorKind`, `InvalidArgument`, `BuildError` |
| `raw` | `RawResponse`, `RawCompletion`, `RawJsonError` |
| `codec` | the `Decode` trait, `LiveBody`, `BodyReadError`, `DecodeFailure`, `DEFAULT_MAX_FRAME_BYTES`, and the `Json`, `Text`, `Binary`, `Empty` codecs |
| `stream` | the `Sse`, `Ndjson`, and `ByteStream` codecs, `ItemStream`, `StreamError`, `StreamEnd`, `EndCondition`, `UntilClose` |
| `shapes` | the `Shapes` codec, `Alternatives`, `JsonAlternatives`, `Route`, `ShapeKind`, `JsonTag` |
| `graphql` | the `GraphQl` codec, `GraphQlError`, `GraphQlFailure`, `GraphQlOperationError`, `AddressRule`, `derive_address`, `request_body`, `page_data` |
| `endpoint` | the `Api` and `EndpointSpec` contracts, `Address`, `RequestParts`, `RequestBody`, `FormPart`, `ErrorBody`, `ErrorRoute`, `StatusMatch` |
| `credentials` | `Credential`, `AuthConfig`, `CredentialPlacement`, `EnvFallback` |
| `options` | `CallOptions`, the per-call settings for `send_raw` |
| `deadline` | `Deadline` |
| `origin` | `OriginPolicy` |
| `hooks` | `HookError`, `SupportsHooks`, `HookStage` |
| `meta` | `RequestMeta`, `ResponseMeta`, `AuthMethod` |
| `redact` | `SecretNames`, `RedactedUrl`, `REDACTED` |
| `transport` | `SharedTransport`, `SharedTransportBuilder` |
| `pager` | `Paginated` and its capability traits, `PageRootError`, `Pager`, `Pages`, `Items`, `Page`, `PagerError`, `Incomplete`, `IncompleteReason`, `SessionScope`, `SessionError`, `DEFAULT_PAGE_LIMIT`, `DEFAULT_CLOSE_GRACE` |

The runtime does not depend on `schematic-define`; generated code translates
definition data into runtime configuration.

## Compatibility With Generated Code

`ABI` is a compatibility marker between this crate and generated code. The
generated `lib.rs` asserts at compile time that it equals the value its
generator was built against, so a `schematic-schema` built against a runtime
of another ABI fails to compile with a message that says to regenerate.
Increment `ABI` with every change that generated code can observe. See
[Partial Generation](../docs/generation/partial-generation.md#shared-runtime-compatibility).

## Testing

```bash
just -f schematic/justfile test               # includes this crate's L1 suite
cargo nextest run -p schematic-runtime        # this crate only
just -f schematic/justfile test-compile-fail  # the trybuild cases
```

- The L1 tests in `tests/l1/` run every send against loopback mocks:
  wiremock for wire observation, raw TCP listeners for cut-off bodies,
  stalled heads, and connection counting, and a local TLS server.
- `tests/fixtures/compile_fail/` holds the misuses that must not compile,
  each with its expected diagnostic: a request of another API (directly or
  through a generated API struct), a mutator on a non-JSON endpoint or typed
  by another value, an endpoint missing its associated types, a `Declared`
  error for an endpoint that declares no error body, an any-origin policy
  spelled without its `danger_` name, a request change after `.full()`, and a
  `CallError` match without a wildcard arm.

## License

AGPL-3.0-only
