# Header Auth Precedence

Generated clients resolve credentials through `schematic-runtime`, with
the same sources and the same placement rules:

- **REST clients** resolve one `Credential` per send.
- **WebSocket clients** resolve one `Credential` per explicit connect and
  reuse it for every reconnect of that connection.

Neither uses the `schematic_define::Headers` builder.

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

For an endpoint that inherits the API policy there is no missing-credential
error: a server that needs a credential answers (typically with a 401) inside
the response envelope, and the metadata records `AuthMethod::Anonymous`. An
endpoint can override this: a public endpoint never sends a credential, and an
endpoint with its own policy refuses a send that resolves none, before any
request. See [endpoint authentication](../runtime/credentials.md#endpoint-authentication).

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

Legacy `RestApi::env_auth` (bearer and API-key fallback chains) and the
username and password lists carried by `AuthStrategy::Basic` are still
supported as authoring inputs and are normalized into `EnvMapping` first;
an explicit `env_mapping` replaces that derived mapping wholesale. An API key
sent as a query parameter or cookie reads the variables `env_auth` names.
Each generated module's docs list the variables its client reads.

## WebSocket clients

Each `connect_*` call selects one handshake credential, and the first source
that supplies one wins:

1. a per-connect credential, `WsClientOptions::builder().credential(..)`
2. the client credential, set with `api_key(..)` or `credentials(..)` on
   the API struct
3. the first set, non-empty variable of the API's environment mapping
4. otherwise the upgrade is sent with no credential, and the endpoint
   client's `auth_method()` reports `AuthMethod::Anonymous`

```rust
use schematic_schema::elevenlabs_ws::ElevenLabsTTSWs;

// Explicit key:
let api = ElevenLabsTTSWs::new().api_key("my-key");

// Or nothing: connecting reads ELEVEN_LABS_API_KEY, then ELEVENLABS_API_KEY.
let api = ElevenLabsTTSWs::new();
```

The variables come from the definition's environment mapping, exactly as for
a REST client (see above). A rejected upgrade (401 or 403) is never retried
with another credential, and a redirect is never followed. `header(name,
value)` adds a handshake header but cannot carry the credential: a header
named like the API's credential header is refused. See
[the handshake credential](../websocket-definitions.md#the-handshake-credential).

## OAuth

OAuth token acquisition is handled outside generated clients by
`schematic-oauth`. A REST client takes the token as
`Credential::access_token(token)`, and so does a WebSocket client
(`credentials(Credential::access_token(token))`).
