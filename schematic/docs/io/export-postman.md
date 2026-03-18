# Exporting Postman Collections from Schematic

Schematic generates Postman Collection v2.1.0 JSON files from API definitions. Collections are generated alongside Rust clients and OpenAPI specs during the standard generation workflow.

## Quick Start

```bash
# Generate all artifacts (Rust + OpenAPI + Postman)
just -f schematic/justfile generate

# Generate only Postman collections
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
- **Variables** — `baseUrl` variable for the API's base URL
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
| `Basic` | `basic` | `{{username}}`, `{{password}}` |
| `None` | `noauth` | — |

## Body Mapping

| Schematic | Postman Mode | Content-Type |
|-----------|-------------|--------------|
| `Json(Schema)` | `raw` | `application/json` |
| `FormData` | `formdata` | — |
| `UrlEncoded` | `urlencoded` | — |
| `Text` | `raw` | declared content type |
| `Binary` | `file` | — |

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
