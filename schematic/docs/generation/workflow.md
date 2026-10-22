# Generate and verify API artifacts

Use `schematic-gen` to validate a definition or generate Rust clients, OpenAPI
3.0.3 documents, and Postman v2.1 collections from the same module group.
Run the examples below from the repository root.

## Shipped REST APIs

| CLI API name | Generated module | Endpoints |
| --- | --- | --- |
| `anthropic` | `anthropic` | 4 |
| `artificial-analysis-data` | `artificial_analysis` | 6 |
| `artificial-analysis-critpt` | `artificial_analysis` | 1 |
| `bitbucket` | `bitbucket` | 15 |
| `openai` | `openai` | 258 |
| `elevenlabs` | `elevenlabs` | 42 |
| `gitea` | `gitea` | 15 |
| `github` | `github` | 16 |
| `gitlab` | `gitlab` | 19 |
| `huggingface` | `huggingface` | 26 |
| `lmstudio` | `lmstudio` | 6 |
| `ollama-native` | `ollama` | 11 |
| `ollama-openai` | `ollama` | 4 |
| `emqx-basic` | `emqx` | 36 |
| `emqx-bearer` | `emqx` | 38 |
| `eversolo` | `eversolo` | 24 |
| `samsung-smart-tv` | `samsung_smart_tv` | 7 |
| `unfolded-circle-core-rest` | `unfolded_circle_core_rest` | 11 |

Gitea's `GetGitTree` accepts an optional `recursive` query value. GitLab's
`GetRepositoryFile` exposes the `git_ref` setter and sends it as one `ref` query
value: `client.get_repository_file(id, path).git_ref("HEAD".into())`. OpenAI's optional `beta` controls select ordinary query
values on the surviving endpoint IDs. Query values do not distinguish routes.

WebSocket clients also cover ElevenLabs TTS, Samsung Smart TV remote control,
and Unfolded Circle Core, Dock, and Integration APIs. Their definition types
remain separate from REST builders.

## Generate artifacts

```sh
just -f schematic/justfile generate
just -f schematic/justfile generate-one gitea
just -f schematic/justfile generate-openapi
just -f schematic/justfile generate-postman
just -f schematic/justfile check-drift
```

The standard output roots are `schematic/schema/src`, `schematic/openapi`, and
`schematic/postman`. When the CLI output ends in `schema/src`, it selects the
sibling export directories automatically; `--no-openapi` and `--no-postman`
suppress those outputs. A Rust module can be a single file or a directory
containing `mod.rs`, `requests.rs`, and `responses.rs` when it exceeds the split
threshold. See [partial generation](partial-generation.md) for publication,
ownership, and cleanup rules.

```sh
schematic-gen validate --api openai
schematic-gen generate --api openai --output generated/src
schematic-gen generate --api openai --output generated/src --openapi-out specs/ --openapi-format yaml
schematic-gen import --input petstore.yaml --output generated/src --strict
schematic-gen bundle --api github --out github.bundle.json
schematic-gen export-bundle --input github.bundle.json --openapi-out specs/ --postman-out collections/
```

[OpenAPI import](../io/import-openapi.md), [OpenAPI export](../io/export-openapi.md),
and [Postman export](../io/export-postman.md) describe the complete options,
version selection, and format behavior. [Definition bundles](../io/definition-bundles.md)
describes the `bundle` file and the checks `export-bundle` applies before it
exports without the Rust registry.

## Share a generated module

APIs with one types module assemble their definitions and ordered schemas through
one `DefinitionContext`. Give sibling API builders the same `.module_path(...)`
and distinct `.request_suffix(...)` values; for example Ollama uses `ollama`,
`NativeRequest`, and `OaiRequest`. EMQX similarly shares `emqx`. Grouped exports
contain all sibling endpoints; their APIs still have distinct client structs.
See [typed registration](../typed-registration.md) for the assembly contract.

## Verify a change

```sh
just -f schematic/justfile test
just -f schematic/justfile lint
just -f schematic/justfile test-schema
```

The ordinary test recipe includes the separate fresh-output compiler guards and
runtime misuse checks. Fresh temporary client crates compile and execute offline;
the dependency versions in `schematic/schema/Cargo.lock` must be cached. If Cargo
reports a missing offline dependency, populate the cache with
`cargo fetch --manifest-path schematic/schema/Cargo.toml` and rerun.

The schema recipe checks the committed generated crate, including paging and
compiler diagnostics. Current artifact tests compare complete Rust, OpenAPI, and Postman file sets
and bytes. Loopback HTTP fixtures assert generated
wire requests, credentials, codecs, hooks, and paging behavior; they make no live
provider requests. This evidence establishes generated behavior against recorded
contracts, not the current behavior of every remote service.

When changing a response, check the endpoint's success type and codec:
`.returns_json::<T>(ctx)`, `.returns_text()`, `.returns_binary()`, and
`.returns_empty()` produce `Json<T>`, `Text`, `Binary`, and `Empty`, respectively.
Choosing JSON for binary audio would produce the wrong success type and a
malformed-success error at runtime. See [definition builders](../definition-builders.md).

On Windows, the generator reserves an 8 MiB main-thread stack through its build
script for catalog assembly. Keep that build script when packaging the CLI;
Windows' default 1 MiB stack can overflow during debug generation, including
`schematic-gen generate --api gitea`. This applies to MSVC and GNU targets.
