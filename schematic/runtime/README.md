# schematic-runtime

The hand-written request runtime shared by every REST client that
`schematic-gen` generates into `schematic-schema`.

## Overview

Generated code stays thin: per API it declares a marker type and an
`ApiDefinition`, and per endpoint a request struct implementing
`EndpointSpec`. Everything that executes a request lives here, written once
and tested once instead of being regenerated into every API module:

- a managed `Client<A>`, built through `ClientBuilder`, that sends only the
  requests of its own API
- a typed `send` that returns one `Response` envelope for every reply that
  arrived, and a `send_raw` that returns the bytes undecoded
- credential resolution: per call, per client, from environment variables,
  or anonymous
- redirect following under a per-API cross-origin policy, with the
  credential attached only where the policy allows
- a per-request timeout and a `Deadline` shared across many sends
- the built-in codecs `Json`, `Text`, `Binary`, and `Empty`
- JSON response hooks
- metadata that is redacted when it is captured, so it is always safe to log

The runtime never retries, and it cannot be handed an HTTP client: it builds
every HTTP client itself, with the library's own redirect following turned
off, so every request goes through the policy above.

## Quick Start

Most code uses the runtime through a generated client:

```rust
use schematic_schema::github::GetRepositoryRequest;
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GitHub::builder()
        .credentials(Credential::access_token(std::env::var("MY_TOKEN")?))
        .build()?; // BuildError

    let response = client
        .send(GetRepositoryRequest::new("rust-lang", "rust"), &CallOptions::new())
        .await?; // TransportError: no response head arrived

    println!("{} (request id {:?})", response.meta.status, response.meta.request_id);
    let repo = response.into_result()?; // ApiError, with the raw body
    println!("{}", repo.full_name);
    Ok(())
}
```

`schematic-schema` re-exports this crate as `schematic_schema::schematic_runtime`,
and its prelude re-exports `CallOptions`, `Credential`, `Deadline`,
`OriginPolicy`, `Response`, `RawResponse`, and `TransportError`.

## The Governing Rule

A send returns `Ok` if and only if an HTTP response head arrived. Every
status, including an undecodable or cut-off body, is reported inside the
envelope with its raw bytes. `Err(TransportError)` means no head arrived: the
request was refused before sending (invalid argument or URL, cross-origin
target, expired deadline) or the connection, TLS handshake, or wait for the
first head failed.

```rust
use schematic_schema::schematic_runtime::{ErrorKind, Outcome};

match client.send(request, &CallOptions::new()).await {
    Err(transport) => eprintln!("no response: {transport}"),
    Ok(response) => match response.outcome {
        Outcome::Success(value) => { /* typed value */ }
        Outcome::Failure(error) => match error.kind {
            ErrorKind::Undeclared => eprintln!("{}: {} bytes", response.meta.status, error.body.len()),
            ErrorKind::MalformedSuccess => { /* 2xx that did not decode */ }
            _ => eprintln!("{error}"),
        },
    },
}
```

## Documentation

The topic pages in [`schematic/docs/runtime/`](../docs/runtime/README.md)
explain each behavior with examples:

| Topic | Page |
| --- | --- |
| `send`, `send_raw`, the envelope, error kinds, codecs | [Sending and responses](../docs/runtime/sending-and-responses.md) |
| builder options, `SharedTransport`, `BuildError` | [Building clients](../docs/runtime/building-clients.md) |
| credential precedence and anonymous calls | [Credentials](../docs/runtime/credentials.md) |
| cross-origin policies, redirects, `check_link` | [Origins and redirects](../docs/runtime/origins-and-redirects.md) |
| timeouts and shared deadlines | [Deadlines and timeouts](../docs/runtime/deadlines-and-timeouts.md) |
| `pre_response_json` and `mutate_response` | [Response hooks](../docs/runtime/hooks.md) |
| `RequestMeta`, `ResponseMeta`, request ids, redaction | [Metadata and redaction](../docs/runtime/metadata-and-redaction.md) |

The crate-level rustdoc (`cargo doc -p schematic-runtime --open`) documents
every type.

## Module Layout

| Module | Contents |
| --- | --- |
| `client` | `Client`, `ClientBuilder`, `ApiDefinition`, `send`, `send_raw`, `check_link`, `DEFAULT_TIMEOUT`, `MAX_REDIRECTS` |
| `response` | `Response`, `Outcome`, `ApiError`, `ErrorKind`, `RedirectRefusal`, `FailureCategory`, `NoDeclaredError` |
| `error` | `TransportError`, `TransportErrorKind`, `InvalidArgument`, `BuildError` |
| `raw` | `RawResponse`, `RawCompletion`, `RawJsonError` |
| `codec` | the `Decode` trait, `LiveBody`, and the `Json`, `Text`, `Binary`, `Empty` codecs |
| `endpoint` | the `Api` and `EndpointSpec` contracts, `RequestParts`, `RequestBody`, `FormPart`, `ErrorBody` |
| `credentials` | `Credential`, `AuthConfig`, `CredentialPlacement`, `EnvFallback` |
| `options` | `CallOptions` |
| `deadline` | `Deadline` |
| `origin` | `OriginPolicy` |
| `hooks` | `HookError`, `SupportsHooks`, `HookStage` |
| `meta` | `RequestMeta`, `ResponseMeta`, `AuthMethod` |
| `redact` | `SecretNames`, `RedactedUrl`, `REDACTED` |
| `transport` | `SharedTransport`, `SharedTransportBuilder` |

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
  each with its expected diagnostic: a request of another API, a mutator on
  a non-JSON endpoint or typed by another value, an endpoint missing its
  associated types, a `Declared` error for an endpoint that declares no
  error body, and an any-origin policy spelled without its `danger_` name.

## License

AGPL-3.0-only
