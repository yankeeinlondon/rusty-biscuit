# Metadata and Redaction

Every send records what was requested and what came back. The records are
redacted **when they are captured**, so a stored metadata field never holds
a secret and can be logged, attached to an error report, or shown to a user.

## What is recorded

`ResponseMeta` (in `response.meta` and `raw.meta`):

| Field | Holds |
| --- | --- |
| `request` | the `RequestMeta` of the request that received this reply |
| `status` | the response status |
| `headers` | response headers, secret values redacted; repeated headers stay repeated |
| `request_id` | the provider's request id, when the API names a request-id header and the reply carries it |
| `elapsed` | time from the start of the send until its outcome was produced: every redirect hop, the body read, decoding, and response hooks; for a streaming output, until the codec returned it |

`RequestMeta` (also in `TransportError::request`):

| Field | Holds |
| --- | --- |
| `endpoint_id` | the endpoint's id, unique within its API |
| `method` | the HTTP method actually sent |
| `url` | the full URL, as a `RedactedUrl` |
| `query` | decoded query pairs in URL order, secret values redacted |
| `headers` | request headers, secret values redacted |
| `auth` | the credential's source: `PerCall`, `ClientLevel`, `EnvVar(name)`, or `Anonymous` |
| `page` | the page's position in its traversal, from 1, on a pager's page request (and on that page's transport error); `None` on every other send |

After a redirect, `request` describes the last request sent, the hop that
received the reply.

```rust
let response = client.get_repository("o", "r").full().await?; // the whole envelope
let meta = &response.meta;
eprintln!(
    "{} {} via {} ({}), request id {:?}",
    meta.request.endpoint_id,
    meta.request.url,  // secret query values and user information redacted
    meta.request.auth, // names the source, never the value
    meta.status,
    meta.request_id,
);
```

## Request ids

An API definition can name the response header that carries the provider's
request id (`request_id_header` on `RestApi`). The runtime matches that
header case-insensitively and puts its first value in
`ResponseMeta::request_id`, so you can quote it in a support ticket without
digging through headers. GitHub (`x-github-request-id`) and OpenAI
(`x-request-id`) name one today; for other APIs it is `None`.

## What is redacted

A value is replaced by `redact::REDACTED` when its name is secret. The
secret names are the union of:

- **built-in header names**: `Authorization`, `Proxy-Authorization`,
  `Cookie`, `Set-Cookie`, `X-Api-Key`, `X-Auth-Token`, `Api-Key`;
- **built-in query names**: `api_key`, `apikey`, `access_token`, `token`,
  `key`, `signature`;
- the header, query, or cookie names the API's **authentication** uses (for
  example `PRIVATE-TOKEN` for GitLab); and
- the API definition's own **`secret_names`**.

Header names match case-insensitively; query names match exactly, after
percent-decoding. Every other value, including a repeated non-secret query
value, is kept as it is.

Two more rules apply regardless of names:

- **URL user information** (`https://user:pass@host`) is always redacted.
- **`Location` and `Content-Location`** headers keep their destination
  visible but have secret query and fragment values redacted, so a redirect
  target can be logged safely.

`RedactedUrl` can only be produced by this redaction, so holding one means
it has been applied.

## What is not redacted

Body bytes and decoded values are evidence and are never altered:
`ApiError::body`, `RawResponse::body`, a decoded declared error, and every
success value are exactly what the provider sent. `Display` never prints a
body, but `Debug` does, so a provider that echoes a credential in an error
body exposes it through `Debug` logging. Prefer the `Display` form (a
`CallError`'s included, which names the endpoint, status, and redacted
request) and the metadata for shared logs.

A codec reading a live body sees the unredacted response headers; they are
for decoding only and are never stored.

## Client and credential output

A client's `Debug` output shows the API name, the base URL (user information
redacted), the timeout, and the kind of its client credential. A
`Credential` prints its kind only. Neither
ever shows a token, key, or password, however it was supplied.
