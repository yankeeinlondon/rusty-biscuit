# Building Clients

Each generated API has two ways to get a client:

- **`{Api}::new()`** builds one with every default.
- **`{Api}::builder()`** returns a runtime `ClientBuilder` already seeded
  with the API's definition (base URL, accepted credentials, environment
  fallback, cross-origin policy, secret names, request-id header) and its
  static default headers. Adjust it, then call `build()`.

Both return `Result<{Api}Client, BuildError>`; `{Api}Client` is an alias for
`schematic_runtime::Client<{Api}>`.

```rust
use std::time::Duration;

use schematic_schema::prelude::*;

let client: GitHubClient = GitHub::builder()
    .base_url("http://127.0.0.1:8080")             // e.g. a local mock
    .connect_timeout(Duration::from_secs(5))
    .timeout(Duration::from_secs(10))
    .credentials(Credential::access_token(token))
    .build()?;
```

## Builder options

| Method | Effect |
| --- | --- |
| `base_url(url)` | send to `url` instead of the definition's base URL; it must be an absolute HTTP(S) URL with a host and no user information |
| `timeout(duration)` | the per-request timeout (default `DEFAULT_TIMEOUT`, 30 s); see [Deadlines and timeouts](./deadlines-and-timeouts.md) |
| `connect_timeout(duration)` | bound on establishing a connection |
| `proxy(url)` | send every request through this proxy |
| `add_root_certificate(pem_or_der)` | trust an extra root certificate, in addition to the platform's |
| `danger_accept_invalid_certificates()` | accept any server certificate, including self-signed and expired ones; only for devices that cannot present a valid one |
| `user_agent(value)` | the `User-Agent` of every request |
| `default_header(name, value)` | a header sent with every request unless the request sets the same header |
| `credentials(credential)` | the client [credential](./credentials.md) |
| `env_order([..])` | replace the order of the environment variables the fallback reads |
| `cross_origin(policy)` | tighten the API's [cross-origin policy](./origins-and-redirects.md) |
| `transport(&shared)` | take connections from a `SharedTransport` |
| `pre_response_json(hook)`, `mutate_response::<R, _>(hook)` | register [response hooks](./hooks.md) |

There is no option to inject your own HTTP client, and no client follows
redirects on its own: the runtime builds every HTTP client itself with
redirect following turned off, and follows redirects one hop at a time so
the origin policy is checked at each hop.

## Sharing connections

A `SharedTransport` is one connection pool that several clients use. Two
clients attached to the same transport reuse each other's idle connections
to the same origin:

```rust
use std::time::Duration;

use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::SharedTransport;

let transport = SharedTransport::builder()
    .proxy("http://proxy.internal:3128")
    .connect_timeout(Duration::from_secs(5))
    .build()?;

let github = GitHub::builder().transport(&transport).build()?;
let gitlab = GitLab::builder().transport(&transport).build()?;
```

The settings that belong to a pool, `proxy`, `add_root_certificate`,
`danger_accept_invalid_certificates`, and `connect_timeout`, are set on the
transport. A client attached to a transport that also sets one of them fails
to build with `BuildError::TransportOptionConflict`, naming the option:

```rust
let refused = GitHub::builder()
    .transport(&transport)
    .proxy("http://other:3128") // owned by the transport
    .build();
assert!(refused.is_err());
```

`timeout`, credentials, headers, the origin policy, and hooks stay per
client.

## Build errors

`build()` validates everything up front, so a misconfigured client never
reaches the network. `BuildError` is `#[non_exhaustive]`:

| `BuildError` | Cause |
| --- | --- |
| `InvalidBaseUrl { reason }` | the base URL is not absolute, not HTTP(S), has no host, or carries user information |
| `InvalidAllowedOrigin { reason }` | an `allow_origins` entry is not a bare HTTP(S) origin (it has a path, query, fragment, or user information) |
| `OriginPolicyNotPermitted { definition, requested }` | `cross_origin(..)` would loosen the API's policy |
| `TransportOptionConflict { option }` | a pool option was set on a client attached to a `SharedTransport` |
| `AuthBearingDefaultHeader { name }` | a default header would carry a credential (for example `Authorization`, `Cookie`, or the API's key header); use `credentials(..)` instead |
| `InvalidHeader { name }` | a default header name or value is not valid HTTP |
| `IncompatibleCredential { supplied }` | the client credential is of a kind the API does not accept |
| `InvalidProxy` | the proxy URL is not valid |
| `InvalidCertificate` | a root certificate could not be parsed |
| `Transport` | the HTTP client could not be initialized |

For example, an API defined as `deny` cannot be loosened to `same_origin`:

```rust
let client = SomeDenyApi::builder()
    .cross_origin(OriginPolicy::same_origin())
    .build(); // Err(OriginPolicyNotPermitted { definition: "deny", requested: "same_origin" })
```

## Building a client by hand

Generated code calls `Client::builder(definition)` with an `ApiDefinition`
built from the API definition. Tests and hand-written integrations can do
the same with any type that implements the `Api` marker trait, and with
request types that implement `EndpointSpec`:

```rust
use schematic_runtime::{ApiDefinition, AuthConfig, Client, CredentialPlacement, EnvFallback};

let definition = ApiDefinition::new("https://api.example.com")
    .auth(
        AuthConfig::none()
            .accept(CredentialPlacement::Bearer { header: None })
            .env_fallback(EnvFallback::new(
                CredentialPlacement::Bearer { header: None },
                ["EXAMPLE_TOKEN"],
            )),
    )
    .request_id_header("x-request-id")
    .secret_names(["X-Session"]);

let client = Client::<Example>::builder(definition).build()?;
```

`ApiDefinition::cross_origin(policy)` sets the API's own policy (default
`same_origin`); callers can only tighten it afterward.
