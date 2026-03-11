# Header Auth Precedence

This note documents the runtime credential precedence used by generated REST clients and by `schematic_define::Headers`.

## Precedence

Generated clients resolve REST auth in this order:

1. Explicit auth already attached to `Headers`
2. Explicit auth injected through generated client helpers such as `.api_key(...)`, `.bearer_token(...)`, `.basic_auth(...)`, or `.oauth_token(...)`
3. Environment-variable fallback from `EnvMapping`
4. `SchematicError::AuthenticationRequired`

Once any explicit auth is present, environment fallback is skipped. This includes explicit API keys, not just `Authorization` headers.

## Explicit Auth Detection

Use `Headers::has_explicit_auth()` when you need to know whether auth was supplied programmatically.

`Headers::has_authorization()` is narrower: it only reports whether the `Authorization` header is populated.

## Environment Fallback

`EnvMapping` is the authoritative runtime source for environment-based auth fallback:

- `bearer_token` for bearer-token fallback
- `basic_user` + `basic_pass` for basic auth fallback
- `api_key` for API-key header fallback

Legacy `RestApi::env_auth` and `RestApi::env_username` are still supported as authoring inputs, but generated clients normalize them into `EnvMapping` before runtime resolution.

## OAuth

OAuth token acquisition is handled outside generated clients by `schematic-oauth`. Generated clients only accept an already-obtained token, typically through `.oauth_token(...)` or an explicit `Headers` builder.
