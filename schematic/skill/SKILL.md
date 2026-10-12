---
name: schematic
description: |-
    A detailed knowledge base of how to:

    1. Use the primitives provided in `schematic-define` package to define an API
    2. How to use the utilities in `schematic-gen` package to generate an API client which will be exposed in the `schematic-schema` package 
    
    Using this skill will give you deep knowledge on how to operate all of the following packages:

        - `schematic-definitions`
        - `schematic-define`
        - `schematic-gen`
        - and `schematic-schema`
---

# Schematic API Generation and Clients

The **Schematic** packages in the **rusty-biscuit** monorepo are designed to:

- help you design/describe an API surface
- provide code _generation_ utilities to convert a "definition" into an API Client

The four packages involved in this process are:

- `schematic-definitions` - this is where we define the API definitions

    For more details on **schematic-definitions** choose from any of the links below:

    ::toc-linking ../definitions/README.md level=h2

- `schematic-define` - this library provides the _primitives_ used to define an API surface

    For more details on the primitives defined in **schematic-define** choose from any of the links below:

    ::toc-linking ../define/README.md level=h2

- `schematic-gen` - once an API has been defined (in `schematic-definitions`) we will use the `schematic-gen` library to generate an API client and deploy it to `schematic-schema`

    For more details on the primitives defined in **schematic-gen** choose from any of the links below:
    
    ::toc-linking ../gen/README.md level=h2

- `schematic-schema` is the package that callers will look for API client definitions in

    For more details on the primitives defined in **schematic-schema** choose from any of the links below:
    
    ::toc-linking ../schema/README.md level=h2


## More Details on Packages

To understand the various package found in the **schematic** ecosystem in greater depth you should choose from the following documents:

- [schematic-definitions](./sections/definitions.md) has a full overview of the API definitions **schematic** has. It also gives code examples of how you might define different parts of an API.

## Tooling

We use the `just` runner to provide the most common operations you'll need when working with these packages:

- `just build {args}` - will build all schematic packages
- `just test {args}` - will run unit and integration tests on the schematic packages
- `just test-fresh-output` - the part of `just test` that generates client crates outside the checkout and compiles them offline (all 18 APIs plus fixture APIs); CI runs it on the build job
- `schematic-runtime`'s managed-send tests (`runtime/tests/l1/`: `send`, `credential_resolution`, `builder`, `timing`, `redirects`, `raw`, `hooks`) are L1 against loopback mocks (`redirects` uses wiremock servers on different loopback ports as origins A, B, and C): wiremock for wire observation, raw `TcpListener` mocks for cut-off bodies, stalled heads, connection counting, and proxy routing, and an rcgen + tokio-rustls TLS mock. The four TLS-mock tests are `ignore`d on native Windows only, because reqwest's built-in rustls client intermittently aborts the process there (see the `os` skill, `windows.md`)
- `just test-compile-fail` - the part of `just test` that runs `schematic-runtime`'s trybuild cases (`tests/fixtures/compile_fail/*.rs`), each compared with its expected compiler diagnostic in a `.stderr` file; regenerate those with `TRYBUILD=overwrite` only after checking the new diagnostic is the intended one. It is a `slow_` test, so CI's L1 run does not include it
- `just lint {args}` - will use the clippy linter to look for non-idiomatic code
- `just generate` - will generate the API definitions in `schematic-definitions` into API clients in `schematic-schema`
    - generation validates the whole batch separately for the Rust client, OpenAPI, and Postman (`schematic_gen::validation`); an output with an `error` diagnostic is not written, the other outputs still are, and the run exits 1. Warnings (only `query-in-path`) never fail; Postman carries header/cookie params as request headers, `disabled` when optional; a same-API `duplicate-method-path` (one method + normalized path declared twice) is an error in every output with no exception list, so merge such endpoints into one (optional parameter, one response type). Rules: `schematic/docs/generation/validation.md`
    - a module group (`ollama`, `emqx`) is one OpenAPI document, built for every module before any is written: two members' different operations on one method+path fail OpenAPI only; a shared `operationId` on different routes is renamed for the later member in registry order (`ListModels` → `OllamaOpenAIListModels`), so never hand-edit ids in `openapi/*.json`
    - `cargo run -p schematic-gen -- validate --api <name>` prints every diagnostic without writing
    - generated code only describes requests: per API an uninhabited marker (`pub enum OpenAI {}` with `BASE_URL`, `DOCS_URL`, `definition()`, `builder()`, `new() -> Result<OpenAIClient, BuildError>`) and `OpenAIClient = schematic_runtime::Client<OpenAI>`; per endpoint a request struct implementing `schematic_runtime::EndpointSpec` (`Api`, `Success`, `Failure = NoDeclaredError`, `Decoder` = `Json<T>`/`Text`/`Binary`/`Empty` from the `ApiResponse`). Callers send with `client.send(request, &CallOptions::new())` or `send_raw`. Its `into_parts()` returns a `RequestParts` struct (`method`, `path`, `query`, `headers`, `body`) or an `InvalidArgument`: declared query parameters are unencoded pairs in `query`, never in `path`; the runtime encodes each once (`%20` for a space) and appends them after any query already in the URL with `&` (otherwise `?`), so request lines stay byte-identical to the golden `gen/tests/fixtures/wire_baseline/query_request_lines.txt`, which `fresh_output_compile::slow_query_wire_matches_pre_split_baseline` enforces. A `?…` written in a path template stays in `path`. Generated `lib.rs` declares `SCHEMATIC_RUNTIME_ABI` and asserts it equals `schematic_runtime::ABI` at compile time. Rules: `schematic/docs/generation/request-side.md`
    - request-side inputs: `ApiRequest::Text`/`Binary` → `body: String`/`Vec<u8>` sent as `RequestBody::Raw`; header/cookie params → `with_*` fields in `headers`, cookies folded into one `Cookie` header (`shared::combine_cookie_headers`, RFC 6265 `%XX` encoding); credentials (any placement, including `ApiKeyParam` query/cookie keys) are resolved by the runtime per send: per call (`CallOptions::credential`) → client (`builder().credentials(..)`) → first non-empty env var of the definition's mapping (`builder().env_order([..])` replaces the order) → anonymous, with no missing-credential error; a credential never overrides a same-named request query/cookie/header value (`InvalidArgument`); client `Debug` never prints a credential. Runtime evidence: the fixture crate's wiremock tests in `fresh_output_compile::slow_fixture_apis_compile_and_send_offline`. Docs: `schematic/docs/generation/request-side.md`
- `just generate-one <api>` - regenerates that API's whole module group (e.g. `ollama-native` also regenerates `OllamaOpenAI`) in all three outputs and keeps every other module byte-for-byte; `lib.rs`/`prelude.rs` are rebuilt from the kept + regenerated groups
    - every output is prepared in memory (`output::prepare_rust_output` → `MutationPlan::publish`) and never overwrites or deletes a file without the generated header (`Cargo.toml`: the manifest marker line); symlinks are never followed
    - a partial run refuses (nothing changed) an output it cannot account for — missing/corrupt/hand-written `lib.rs` or `prelude.rs`, missing or unknown modules, prelude exports that no longer match the definitions — and a changed `shared.rs`/`ws_shared.rs`, or a `lib.rs` runtime ABI marker that differs, is missing, or is malformed, while other modules are kept requires `--api all` (`RequiresFullGeneration`)
    - a mid-write I/O failure reports completed and pending mutations; rules and examples: `schematic/docs/generation/partial-generation.md`

All packages in **Schematic** are written in Rust and use the 2024 edition.

## How to Design an API

### Key Design Primitives

### Best Practices

Read the following document to make sure you're building a high quality API by learning what are considered the "best practices":

- [Best Practices for Designing an API](./best-practices-in-designing-an-api.md)

Once you've made sure that your API definition meets the requirements described in the best practices guide, you then should use the completion checklist to fully verify that you are "done".

- [Completion Checklist](./completion-checklist.md)
