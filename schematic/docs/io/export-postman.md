# Exporting Postman Collections from Schematic

Schematic generates Postman Collection v2.1.0 JSON files from API definitions. Collections are generated alongside Rust clients and OpenAPI specs during the standard generation workflow.

## Quick Start

```bash
# Generate all artifacts (Rust + OpenAPI + Postman)
just -f schematic/justfile generate

# Generate Rust and Postman collections
just -f schematic/justfile generate-postman

# Single API
schematic-gen generate --api openai --output schema/src --postman-out postman
```

## Output Layout

```
schematic/postman/
├── anthropic.postman_collection.json
├── bitbucket.postman_collection.json
├── elevenlabs.postman_collection.json
├── emqx.postman_collection.json          # Grouped: EmqxBasic + EmqxBearer
├── eversolo.postman_collection.json
├── gitea.postman_collection.json
├── github.postman_collection.json
├── gitlab.postman_collection.json
├── huggingfacehub.postman_collection.json
├── lmstudio.postman_collection.json
├── ollama.postman_collection.json        # Grouped: OllamaNative + OllamaOpenAI
├── openai.postman_collection.json
├── samsung_smart_tv.postman_collection.json
└── unfolded_circle_core_rest.postman_collection.json
```

## Collection Structure

Each collection includes:

- **Info** — Collection name, description, and schema version
- **Auth** — Collection-level authentication (bearer, apikey, basic, or noauth)
- **Variables** — `baseUrl` variable for the API's base URL, plus every
  auth variable the collection and its requests reference
- **Items** — Endpoints organized into folders by URL path segment

## Folder Grouping

Endpoints are automatically grouped into folders based on the first stable path segment:

| Path | Folder |
|------|--------|
| `/models` | Models |
| `/models/{model}` | Models |
| `/v1/audio/speech` | Audio |
| `/repos/{owner}/{repo}/issues` | Repos |
| `/{id}` | (root — no folder) |

The algorithm strips leading `/`, ignores path variables `{...}`, and skips version prefixes (`v1`, `v2`).

## Auth Mapping

| Schematic | Postman Type | Variable |
|-----------|-------------|----------|
| `BearerToken { header: None }` | `bearer` | `{{bearerToken}}` |
| `BearerToken { header: Some(h) }` | `apikey` | `{{apiKey}}` |
| `ApiKey { header }` | `apikey` | `{{apiKey}}` |
| `ApiKeyParam { location: Query, name }` | `apikey`, added to the query as `name` | `{{apiKey}}` |
| `ApiKeyParam { location: Cookie, name }` | `apikey`, added as a `Cookie` header valued `name={{apiKey}}` | `{{apiKey}}` |
| `Basic` | `basic` | `{{username}}`, `{{password}}` |
| `None` | `noauth` | — |

Postman's API-key auth can add a key only to a header or the query string,
so a cookie key travels in a `Cookie` header that holds the cookie pair.

### Endpoint authentication

A request whose endpoint declares its own authentication always carries a
request-level `auth`, which takes precedence over the collection's:

| Endpoint declaration | Request `auth` |
| --- | --- |
| none (inherits) | none of its own; the collection's applies |
| `.public()` | `{"type": "noauth"}` |
| `.auth(policy)` | the policy's first accepted method, mapped as in the table above (an OAuth2 method is a `bearer` token) |
| `.auth_with_env_mapping(policy, mapping)` | the same, with variables suffixed by the endpoint id |

A replacement that declares its own environment mapping names a credential
distinct from the API's, so its variables are suffixed with the endpoint id
(`{{bearerToken_Admin}}` for an endpoint `Admin`, or `{{username_Upload}}`
and `{{password_Upload}}`) instead of sharing `{{bearerToken}}`,
`{{apiKey}}`, `{{username}}`, or `{{password}}` with the API. Every variable a
request references is declared on the collection.

A Postman request holds one auth. When a replacement policy accepts more
than one alternative, only the first is exported and the generator reports
an `endpoint-auth` warning; the collection is still written (see
[Generation Validation](../generation/validation.md#what-each-output-checks)).

## Parameter Mapping

Every declared parameter appears in the request it belongs to, so a user can
fill in any input the API accepts:

| Location | Postman entry |
|----------|---------------|
| path | URL variable valued `<name>`, described by its type when declared |
| query | URL query entry with no value |
| header | request header `name: <name>` |
| cookie | request header `Cookie: name=<name>` |

A required header or cookie parameter is an enabled header. An optional one
is written with `"disabled": true`, so Postman leaves it out until the user
switches it on. A declared description becomes the header's description:

```json
"header": [
  { "key": "x-required", "value": "<x-required>" },
  { "key": "openai-beta", "value": "<openai-beta>", "disabled": true,
    "description": "Optional beta features to enable for this request." },
  { "key": "Cookie", "value": "session=<session>" }
]
```

A path parameter declared with `path_param` gives its URL variable a
description, since a Postman variable has no type: the parameter's
description followed by its type in parentheses, or the type alone. An enum
is listed by value:

```json
"variable": [
  { "key": "number", "value": "<number>", "description": "Issue number (integer)" },
  { "key": "kind", "value": "<kind>", "description": "one of: bug, help-wanted" }
]
```

Each cookie parameter is its own `Cookie` entry so that each can be switched
on separately. Postman sends every enabled entry, so a request with several
enabled cookies carries several `Cookie` header lines, unlike the generated
Rust client, which folds them into one. Declared parameters follow the API's
and the endpoint's static headers and do not replace a same-named static
header.

## Body Mapping

| Schematic | Postman Mode | Content-Type |
|-----------|-------------|--------------|
| `Json(TypeRef)` | `raw` | `application/json` |
| `FormData` | `formdata` | — |
| `UrlEncoded` | `urlencoded` | — |
| `Text` | `raw` | declared content type |
| `Binary` | `file` | — |

## Request Descriptions

A request's description is its endpoint's description, followed, after a
blank line, by one line for each piece of definition vocabulary that Postman
has no field for. Each line appears only when it applies:

| Line | When |
| --- | --- |
| ``Response content type: `…`.`` | the endpoint's first success shape declares a content type or is a stream (`text/event-stream`, `application/x-ndjson`, or the declared type); a plain JSON, text, binary, or empty shape adds no line |
| `Deprecated.` | the endpoint is deprecated |
| `Deprecated parameters: a, b.` | any path, query, header, or cookie parameter is deprecated |
| ``Rate limit headers: limit `…`, remaining `…`, reset `…` (unit), retry after `…`.`` | the endpoint has an effective rate limit (its own, else the API's) |
| `Retry guidance: retry on …; honor Retry-After: …; max attempts … (including the first); backoff ….` | the endpoint has effective retry guidance |

Every GitHub request, for example, ends with:

```text
Rate limit headers: limit `x-ratelimit-limit`, remaining `x-ratelimit-remaining`, reset `x-ratelimit-reset` (epoch seconds), retry after `retry-after`.
```

An endpoint that declares none of these keeps its description alone.

## Schema Composition

A Postman collection cannot declare `oneOf`, `anyOf`, or `allOf`. An
endpoint whose JSON body or response names a composed component (see
[typed registration](../typed-registration.md)) reports a
`schema-composition` warning; the collection is still written, the request
keeps its existing example, and no example payload is invented.

## GraphQL Operations

A collection does not carry an API's GraphQL operations yet. Each one is
reported as the warning
`<API> operation <id>: GraphQL operation skipped in Postman output`, and the
collection is still written without it.

## Grouped Collections

APIs sharing a module (`ollama`, `emqx`) produce a single grouped collection that merges endpoints from all APIs. The grouped collection:

- Uses the module name as the collection title (title-cased)
- Sets collection-level auth from the first API
- Includes base URL variables for each unique base URL
- Combines descriptions from all APIs
- Merges all endpoints into shared folder groups

## CLI Flags

| Flag | Description |
|------|-------------|
| `--postman-out <DIR>` | Output directory for Postman collections |
| `--no-postman` | Skip Postman collection generation |

When `--output` ends with `schema/src` and no explicit `--postman-out` is provided, Postman output defaults to `postman/` as a sibling directory.

## Library API

```rust
use schematic_gen::postman_output::{build_postman_collection, write_postman};

let api = define_openai_api();
let collection = build_postman_collection(&api);
let path = write_postman(&api, Path::new("postman"), false)?;
```

For grouped collections:

```rust
use schematic_gen::postman_output::{build_postman_collection_grouped, write_postman_grouped};

let apis = vec![&api1, &api2];
let collection = build_postman_collection_grouped(&apis, "ollama");
let path = write_postman_grouped(&apis, "ollama", Path::new("postman"), false)?;
```

## Authoring inputs

Postman export consumes the same checked module-group assembly as Rust and
OpenAPI generation. Use [definition builders](../definition-builders.md) and
[typed registration](../typed-registration.md); do not fill a separate registry.
Parameters keep their declared query, header, cookie, or body location. Paths
contain placeholders, with query inputs represented separately. Authentication
comes from the effective API policy and each endpoint's own declaration; runtime origin/redaction settings are not
Postman transport enforcement.
