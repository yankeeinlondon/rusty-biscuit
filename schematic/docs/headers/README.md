# Header Auth Precedence

Schematic has two credential paths, one per kind of generated client:

- **REST clients** send through `schematic-runtime`, which resolves a
  `Credential` per send. They do not use `schematic_define::Headers`.
- **WebSocket clients** carry a `schematic_define::Headers` builder in their
  public `headers` field and send its headers with the connection request.

## REST clients

The runtime resolves one credential per send, and the first source that
supplies one wins:

1. `.anonymous()` on the call, which sends no credential from any source
2. a per-call credential, `.credential(..)` on the call
3. the client credential, set with `api_key(..)`, `basic_auth(..)`, or
   `credentials(..)`
4. the first set, non-empty variable of the API's environment fallback
   (`env_order([..])` replaces the list)
5. otherwise the request is sent anonymously

There is no missing-credential error: a server that needs a credential
answers (typically with a 401) inside the response envelope, and the
metadata records `AuthMethod::Anonymous`.

```rust
use schematic_schema::prelude::*;

let client = GitHub::new().api_key(token);
```

[Credentials](../runtime/credentials.md) covers each source, the refusal
cases, redirects, and why the runtime never retries.

### Where the variables come from

The generated `definition()` translates the API's environment mapping into
the runtime's fallback. `EnvMapping` is the authoritative source:

- `bearer_token` for bearer-token fallback
- `basic_user` + `basic_pass` for Basic fallback (both must resolve)
- `api_key` for API-key header fallback

Legacy `RestApi::env_auth` and `RestApi::env_username` are still supported
as authoring inputs and are normalized into `EnvMapping` first. An API key
sent as a query parameter or cookie reads the variables `env_auth` names.
Each generated module's docs list the variables its client reads.

## WebSocket clients and `Headers`

`Headers` is a builder for a list of header pairs with credential support.
Generated WebSocket clients create it with the API's `EnvMapping` attached,
and send whatever `headers.build()` returns when connecting:

```rust
use schematic_schema::elevenlabs_ws::ElevenLabsTTSWs;

let mut api = ElevenLabsTTSWs::new();

// Explicit key:
api.headers = api.headers.clone().use_api_key("my-key", "xi-api-key");

// Or from the environment, using the attached mapping:
api.headers = api.headers.clone().from_env();
```

Attaching a mapping does not read the environment by itself; call
`from_env()` (skip variables that are not set) or `try_from_env()` (fail with
`HeaderError::MissingCredential` naming the variables) to resolve it.

### Precedence inside `Headers`

1. Explicit auth already set on the builder (`use_bearer_token`,
   `use_basic_auth`, `use_api_key`)
2. The environment mapping, in the order bearer token, Basic, API key

Once any explicit auth is present, environment resolution is skipped. This
includes explicit API keys, not just `Authorization` headers.

### Explicit auth detection

Use `Headers::has_explicit_auth()` when you need to know whether auth was
supplied programmatically.

`Headers::has_authorization()` is narrower: it only reports whether the
`Authorization` header is populated.

## OAuth

OAuth token acquisition is handled outside generated clients by
`schematic-oauth`. A REST client takes the token as
`Credential::access_token(token)`; a WebSocket client takes it through
`Headers::use_bearer_token(token)`.
