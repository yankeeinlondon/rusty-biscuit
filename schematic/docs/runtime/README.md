# The Schematic Runtime

Every REST client in `schematic-schema` is a
[`schematic-runtime`](../../runtime/README.md) client. Generated code only
describes an API: a client struct such as `GitHub`, the only name a caller
writes, with one method per endpoint, and one request and call type per
endpoint behind it. The hand-written runtime does
everything else: it validates the client's settings, resolves credentials, sends the request, follows
redirects, enforces timeouts, decodes the reply, and reports the result.

These pages describe that runtime from the caller's side. You can:

- call an endpoint and get its decoded value with one `.await?`, or ask for
  one envelope that holds the decoded value or the failure, **plus** the
  status, headers, and request id of the reply, whatever the status was
- tell "the server answered with an error" apart from "the server never
  answered" with one `match`
- supply credentials per call, per client, or from environment variables,
  and see in the metadata which one was sent; an endpoint the definition
  declares public never sends one
- page through a list endpoint, or collect all of it, and know whether the
  list is complete
- bound a whole multi-request operation by one shared deadline
- control which redirects and next-page links may leave the API's origin,
  and whether the credential goes with them
- reshape a JSON body before it is decoded, or adjust the decoded value,
  with response hooks
- read a reply's raw bytes without any decoding
- log metadata and `Display` output without leaking a credential

## A first send

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads GITHUB_TOKEN or GH_TOKEN at send time; sends anonymously if neither is set.
    let client = GitHub::new();

    // One method per endpoint; nothing is sent until `.await`.
    let repo = client.get_repository("rust-lang", "rust").await?; // CallError on failure
    println!("{}", repo.full_name);

    // `.full()` awaits to the whole envelope instead: status, headers, and
    // request id for a reply of any status.
    let response = client.get_repository("rust-lang", "rust").full().await?; // Err only when no HTTP response arrived
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
| [Calling an API](./calling-an-api.md) | creating a client, endpoint methods, optional values, per-call overrides, `.full()`, `.build()?`, `CallError`, and when to use request objects |
| [Sending and responses](./sending-and-responses.md) | awaiting a call, `.full()` and `send_raw`, the `Response` envelope, `CallError`, every `ErrorKind`, `TransportError`, codecs, and what is safe to log |
| [Building clients](./building-clients.md) | `{Api}::new()` and its chained settings, when settings are validated, timeouts, proxies and TLS, `SharedTransport`, and every `BuildError` |
| [Credentials](./credentials.md) | precedence (per call, client, environment, anonymous), public endpoints and endpoint replacement policies, `.anonymous()` on a call, and why the runtime never retries |
| [Origins and redirects](./origins-and-redirects.md) | the cross-origin policies, how a redirect is followed hop by hop, and `check_link` |
| [Deadlines and timeouts](./deadlines-and-timeouts.md) | the per-request timeout, a `Deadline` shared across sends, and stream bounds |
| [Response hooks](./hooks.md) | `pre_response_json` and `mutate_response` |
| [Codecs and decoding](./codecs.md) | how an endpoint fixes its result type, the `Decode` and `ErrorBody` traits, `LiveBody`, and what a codec that runs hooks must do |
| [Metadata and redaction](./metadata-and-redaction.md) | `RequestMeta`, `ResponseMeta`, request ids, and which values are redacted |
| [Pagination](../pagination.md) | `collect_all`, `pages()`, `fetch_page()`, the pager's controls, `Incomplete` and `PagerError`, sessions, and partitions |

How a request struct's fields become a URL, headers, and body is covered by
[Request-Side Inputs](../generation/request-side.md).

## Three rules that shape everything else

1. **`Ok` means a response arrived.** A `.full()` call or a `send_raw`
   returns `Ok` whenever an HTTP response head (status line and headers)
   arrived, whatever the status and whether or not the body could be read or
   decoded. It returns `Err(TransportError)` only when no head arrived.
   Awaiting a call directly folds both into one `CallError`, whose variants
   keep the same distinction. See
   [Sending and responses](./sending-and-responses.md#where-a-result-lands).
2. **The runtime never retries.** It sends each request once (following
   redirects within that one send). A retry, such as a second call without a
   credential after a 401, is an explicit second call by the consumer.
3. **Generated code is thin on purpose.** You cannot inject your own HTTP
   client, and no client follows redirects on its own, so credential
   resolution, the origin policy, timeouts, and redaction apply to every
   request.

## Planned

- **Streams and several shapes in shipped definitions.** The generator emits
  the stream, multi-shape, and GraphQL codecs (see
  [Codecs and decoding](./codecs.md#what-generated-code-emits)), but a
  definition cannot store several success shapes or a stream yet, so a
  streaming endpoint is still declared binary and framed by the caller.
  Generated error tables work today; no shipped definition declares errors
  yet, so their error replies still arrive as `ErrorKind::Undeclared`.
