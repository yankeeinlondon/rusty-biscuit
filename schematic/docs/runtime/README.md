# The Schematic Runtime

Every REST client in `schematic-schema` is a
[`schematic-runtime`](../../runtime/README.md) client. Generated code only
describes an API: a marker type such as `GitHub`, a `GitHubClient` alias, and
one request struct per endpoint. The hand-written runtime does everything
else: it builds the client, resolves credentials, sends the request, follows
redirects, enforces timeouts, decodes the reply, and reports the result.

These pages describe that runtime from the caller's side. You can:

- send a typed request and get back one envelope that holds the decoded
  value or the failure, **plus** the status, headers, and request id of the
  reply, whatever the status was
- tell "the server answered with an error" apart from "the server never
  answered" with one `match`
- supply credentials per call, per client, or from environment variables,
  and see in the metadata which one was sent
- bound a whole multi-request operation by one shared deadline
- control which redirects and next-page links may leave the API's origin,
  and whether the credential goes with them
- reshape a JSON body before it is decoded, or adjust the decoded value,
  with response hooks
- read a reply's raw bytes without any decoding
- log metadata and `Display` output without leaking a credential

## A first send

```rust
use schematic_schema::github::GetRepositoryRequest;
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads GITHUB_TOKEN or GH_TOKEN at send time; sends anonymously if neither is set.
    let client = GitHub::new()?;

    let response = client
        .send(GetRepositoryRequest::new("rust-lang", "rust"), &CallOptions::new())
        .await?; // Err only when no HTTP response arrived

    println!("{}", response.meta); // status and redacted request line
    match response.data() {
        Some(repo) => println!("{}", repo.full_name),
        None => println!("failed: {}", response.error().unwrap()),
    }
    Ok(())
}
```

## Pages

| Page | Read it to learn |
| --- | --- |
| [Sending and responses](./sending-and-responses.md) | `send` and `send_raw`, the `Response` envelope, every `ErrorKind`, `TransportError`, codecs, and what is safe to log |
| [Building clients](./building-clients.md) | `{Api}::builder()`, timeouts, proxies and TLS, `SharedTransport`, and every `BuildError` |
| [Credentials](./credentials.md) | precedence (per call, client, environment, anonymous), `CallOptions::anonymous()`, and why the runtime never retries |
| [Origins and redirects](./origins-and-redirects.md) | the cross-origin policies, how a redirect is followed hop by hop, and `check_link` |
| [Deadlines and timeouts](./deadlines-and-timeouts.md) | the per-request timeout, a `Deadline` shared across sends, and stream bounds |
| [Response hooks](./hooks.md) | `pre_response_json` and `mutate_response` |
| [Metadata and redaction](./metadata-and-redaction.md) | `RequestMeta`, `ResponseMeta`, request ids, and which values are redacted |

How a request struct's fields become a URL, headers, and body is covered by
[Request-Side Inputs](../generation/request-side.md).

## Three rules that shape everything else

1. **`Ok` means a response arrived.** A send returns `Ok` whenever an HTTP
   response head (status line and headers) arrived, whatever the status and
   whether or not the body could be read or decoded. It returns
   `Err(TransportError)` only when no head arrived. See
   [Sending and responses](./sending-and-responses.md#where-a-result-lands).
2. **The runtime never retries.** It sends each request once (following
   redirects within that one send). A retry, such as a second call without a
   credential after a 401, is an explicit second call by the consumer.
3. **Generated code is thin on purpose.** You cannot inject your own HTTP
   client, and no client follows redirects on its own, so credential
   resolution, the origin policy, timeouts, and redaction apply to every
   request.

## Planned

- **Pagination.** Pagers that follow next-page links through
  `Client::check_link` and count pages against a shared `Deadline` are
  **planned**. `RequestMeta::page` exists for them and is always `None`
  today.
- **More codecs.** Server-sent events, NDJSON, and GraphQL codecs, built on
  the same `Decode` trait as the four built-in codecs, are **planned**. Until
  then a streaming endpoint is modeled as binary and framed by the caller.
- **Declared error bodies in definitions.** The runtime can decode an
  endpoint's declared error body on arrival, but API definitions cannot
  declare one yet. Every generated endpoint uses `NoDeclaredError`, so its
  error replies arrive as `ErrorKind::Undeclared` with the raw body.
