# Generation Validation

`schematic-gen` writes three outputs from the API definitions: the Rust
client crate (`schematic/schema/src`), OpenAPI documents
(`schematic/openapi`), and Postman collections (`schematic/postman`). Before
it writes an output, it validates **the whole batch of selected APIs for that
output**. An output with a blocking error is not written at all, and its
failure never stops the other outputs.

You can:

- see every problem with a definition before anything is written:
  `cargo run -p schematic-gen -- validate --api gitea`
- read each problem as one line naming the API, endpoint, feature, and output
- rely on warnings never failing a run, and on errors failing only their own
  output

## How a generate run proceeds

```mermaid
flowchart TD
    A[Select APIs: one module group, or all] --> R[Validate batch for Rust]
    A --> O[Validate batch for OpenAPI]
    A --> P[Validate batch for Postman]
    R --> RE{Any error?}
    O --> OE{Any error?}
    P --> PE{Any error?}
    RE -->|No| RW[Prepare and publish the Rust crate and Cargo.toml]
    RE -->|Yes| RS[Write nothing for Rust]
    OE -->|No| OB[Build every module document in memory: merge members, make operationIds unique, scan]
    OE -->|Yes| OS[Write nothing for OpenAPI]
    OB --> OBE{Any module failed?}
    OBE -->|No| OW[Publish OpenAPI documents]
    OBE -->|Yes| OS
    PE -->|No| PW[Publish Postman collections]
    PE -->|Yes| PS[Write nothing for Postman]
    RW --> S[Print one OK or FAILED line per output]
    RS --> S
    OW --> S
    OS --> S
    PW --> S
    PS --> S
    S --> X{Any output failed?}
    X -->|Yes| N[Exit 1]
    X -->|No| Z[Exit 0]
```

The outputs run in the order Rust, OpenAPI, Postman. Selecting one API
selects its whole module group, separately for each output. After validation,
each output is prepared in memory and its paths are checked before anything
is written; a refused or failed publication (for example, the OpenAPI
directory path is a file) is reported the same way as a validation failure:
that output fails, and the next output is still attempted. How preparation,
ownership checks, and partial runs work is described in
[partial-generation.md](partial-generation.md).

```text
$ schematic-gen generate --api gitea --openapi-out ./not-a-dir ...
[OK] rust output
[FAILED] openapi output
[OK] postman output
[ERROR] generation failed for output(s) openapi (succeeded: rust, postman)
```

## Reading a diagnostic

Each diagnostic is printed to stderr as one line, after a `[WARN]` or
`[ERROR]` label:

```text
severity=<severity> api=<api> endpoint=<endpoint> feature=<feature> output=<output>: <message>
```

`validate` runs all three validators for one API and ends with one line per
output:

```text
$ schematic-gen validate --api gitea
[WARN] severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path output=rust: path template `/repos/{owner}/{repo}/git/trees/{sha}?recursive=true` embeds a query string; declare it as query parameters so it can be separated from the path
[WARN] ... output=openapi: ...
[WARN] ... output=postman: ...
  [PASS] rust
  [PASS] openapi
  [PASS] postman
```

For example:

```text
severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path output=postman: path template `/repos/{owner}/{repo}/git/trees/{sha}?recursive=true` embeds a query string; declare it as query parameters so it can be separated from the path
severity=error api=Feed,FeedCompat endpoint=<none> feature=generated-name-collision output=rust: `SubscribeRequest` is generated for the request struct of Feed::Subscribe and the request struct of FeedCompat::Subscribe
```

- `severity` is `error` (the output is not written) or `warning` (reported
  only).
- `api` names every API involved; a module-group problem lists each member,
  separated by commas.
- `endpoint` is `<none>` when the problem belongs to the API or the group.
- `output` is `rust`, `openapi`, or `postman`.

The same fields are available to code as
`schematic_gen::diagnostics::ValidationDiagnostic`, which also implements
`serde::Serialize`:

```json
{
  "severity": "warning",
  "api": ["Gitea"],
  "endpoint": "GetGitTreeRecursive",
  "feature": "query-in-path",
  "output": "postman",
  "message": "path template ... embeds a query string; ..."
}
```

## What each output checks

| Feature id | Rust | OpenAPI | Postman | Severity |
| --- | :-: | :-: | :-: | --- |
| `duplicate-endpoint-id`: two endpoints of one API share an id | ✓ | ✓ | ✓ | error |
| `duplicate-method-path`: two endpoints of one API share a method and path | ✓ | ✓ | ✓ | error (see below) |
| `query-in-path`: a path template contains `?` | ✓ | ✓ | ✓ | warning (see below) |
| `invalid-output-filename`: the file name is not one safe path component, or a module would overwrite a shared generated file | ✓ | ✓ | ✓ | error |
| `invalid-identifier`: an API name, endpoint id, or parameter name cannot become the Rust identifier generated from it | ✓ | | | error |
| `invalid-request-suffix`: `request_suffix` is empty or not alphanumeric | ✓ | | | error |
| `invalid-module-path`: the module path is not one Rust identifier | ✓ | | | error |
| `invalid-type-syntax`: a body, response, or form JSON type is not a Rust type | ✓ | | | error |
| `generated-name-collision`: two generated items, fields, or client methods share a name | ✓ | | | error |
| `incomplete-schema-registry`: a JSON type has no schema in the module's registry | | ✓ | | error |
| `duplicate-method-path` across a module group: two member APIs declare different operations on one method and path | | ✓ | | error (found while building the document) |
| `duplicate-operation-id`: a finished document declares one `operationId` twice | | ✓ | | error (found while building the document) |
| `unresolved-link-operation`: a link in a finished document names an `operationId` no operation declares | | ✓ | | error (found while building the document) |
| `text-request-body`, `binary-request-body`: the declared content type is empty or not a valid header value | ✓ | | | error |
| `api-key-param`: a query API-key parameter name is empty, or a cookie API-key name is not a cookie name | ✓ | | | error |
| `header-parameter`, `cookie-parameter`: the name is not an HTTP token | ✓ | | | error |
| `query-parameter`, `header-parameter`, `cookie-parameter`: the type is an array of arrays | ✓ | | | error |

The Rust client sends text and binary bodies, header and cookie parameters,
and API keys in a query parameter or cookie, so its rules for those features
check only that the declared names and content types can go on the wire.
How each one is sent is described in
[request-side inputs](./request-side.md#text-and-binary-bodies).
OpenAPI declares header and cookie parameters in their own locations, and a
Postman collection carries each one as a request header, disabled when the
parameter is optional (see
[Postman parameter mapping](../io/export-postman.md#parameter-mapping)), so
neither export has a rule for them.

The Rust client sends each element of an array parameter as text: one
repeated pair per element in the query, one `,`-joined value in a header or
cookie. An element that is itself an array has no text form, so an array of
arrays is an error at every location rather than a shape the client would
drop or fail to compile. OpenAPI writes the nested shape as a parameter
schema, and Postman carries no parameter type, so both exports are still
written:

```rust
// Fails the Rust client only (`query-parameter`).
EndpointParams::default().with_query_param(
    "matrix",
    QueryParamType::Array(Box::new(QueryParamType::Array(Box::new(QueryParamType::Boolean)))),
    false,
    None,
)
```

Rust-only rules are never applied to the exports. An export carries type
names and endpoint ids as data, so a name that is not valid Rust syntax still
produces a valid OpenAPI document and Postman collection:

```rust
// Fails the Rust client only; OpenAPI and Postman are still written.
response: ApiResponse::json_type("Vec<Model"),
```

### Routes and the uniqueness key

Two endpoints are the same route when their method and path match after
every placeholder is replaced by `{}`. `GET /items/{id}` and
`GET /items/{name}` are the same route; `GET /items/{id}` and
`DELETE /items/{id}` are not. This comparison lives in one function,
`schematic_gen::validation::endpoint_uniqueness_key`.

An endpoint **is** its method and path. Two endpoints of one API on the
same route are a `duplicate-method-path` **error** in every output, with no
exceptions, so none of the three outputs is written for that batch. Nothing
else joins the key: a query parameter, a request body, or a different
response does not make a second endpoint on the same route. Declare one
endpoint instead, with the distinguishing input as an optional parameter:

```rust
// Rejected: both are GET /repos/{owner}/{repo}/git/trees/{tree_sha}.
//   GetGitTree           (no parameters)
//   GetGitTreeRecursive  (recursive=true)

// Accepted: one endpoint; `recursive` is an optional query parameter.
Endpoint {
    id: "GetGitTree".to_string(),
    method: RestMethod::Get,
    path: "/repos/{owner}/{repo}/git/trees/{tree_sha}".to_string(),
    params: Some(EndpointParams::default().with_query_param(
        "recursive",
        QueryParamType::Boolean,
        false,
        Some("Fetch the tree recursively instead of a single level"),
    )),
    // ...
}
```

When one route answers with different shapes (Bitbucket's `src` route
returns a JSON directory listing or raw file bytes, depending on what the
path names), the single endpoint declares a response that is true for every
shape: Bitbucket's `GetSource` returns `Binary`, and the caller decodes it.

The same route in two **different** APIs is not a duplicate for the Rust
client or Postman. For OpenAPI it matters only when both APIs share one
module document; see the next section.

Across the APIs of one module group (for example `OllamaNative` and
`OllamaOpenAI`, which both generate into `ollama`), the same endpoint id is
allowed: their request suffixes keep the generated Rust names apart. Only an
actual collision of generated names is an error.

### OpenAPI documents of a module group

The APIs of one module group become one OpenAPI document (`ollama.json`
holds `OllamaNative` and `OllamaOpenAI`). Some problems only exist in that
merged document, so `schematic-gen` builds **every** module's document in
memory before it writes **any** of them; a failure in one module leaves the
whole OpenAPI directory untouched.

- **Same route, same operation:** when two members declare an operation that
  is identical apart from its authentication (EMQX Basic and EMQX Bearer),
  the document keeps one operation and both security alternatives.
- **Same route, different operations:** an OpenAPI path holds one operation
  per method, so this is a `duplicate-method-path` error for the OpenAPI
  output only. The Rust client and Postman collection are still written.
- **Same `operationId`, different routes:** every `operationId` in a document
  must be unique, so a clash is renamed rather than rejected.

The rename rule walks the operations in a fixed order: member APIs in
registry order, each API's paths in declaration order, and methods in the
order GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE. The first
operation to declare an id keeps it. A later operation with a taken id gets
its own API's name prefixed (PascalCase). If that name is taken too, or is
any operation's original id, a number from 2 upward is appended:

```text
OllamaNative  POST /api/embeddings  Embeddings  ->  Embeddings
OllamaOpenAI  POST /v1/embeddings   Embeddings  ->  OllamaOpenAIEmbeddings
```

Original ids are reserved before any rename, so a rename never takes an id
that a later operation already declares: if `FooC` declares `FooBList`, the
renamed `List` of `FooB` becomes `FooBList2`. An OpenAPI link that names an
`operationId` is updated together with the operation it pointed to in its own
API's document; the same id in another API's link is left alone. Per-API
documents are not renamed. Both kinds of document then pass one final scan,
`export::openapi::operation_id_diagnostics`, which reports any id still
declared twice (`duplicate-operation-id`) and any link naming no operation
(`unresolved-link-operation`). The Rust client and Postman collections keep
the original endpoint ids.

### Repeated path segments

A repeated segment such as `/api/api/items` is never an error by itself: a
base URL ending in `/api` joined with a path starting `/api` can be correct.
`schematic-gen` does not judge base-plus-path composition.

### Embedded queries

A path template such as `/repos/{owner}/{repo}/git/trees/{sha}?recursive=true`
mixes a query into the path, where it cannot be separated, typed, or masked.
Nine shipped endpoints still do this (one Gitea, one GitLab, seven OpenAI
`Beta*` endpoints), so `query-in-path` is a warning reported once per output
for each of them. Its severity is the constant `QUERY_IN_PATH_SEVERITY`.
The generated client keeps such a query in the path exactly as written and
appends declared query parameters after it; see
[request-side inputs](./request-side.md#path-templates-that-already-contain-a-query).

## Calling validation from code

Each output has a public validator that takes the whole batch:

```rust
use schematic_gen::validation::{validate_openapi, validate_postman, validate_rust_client};

let apis = schematic_gen::pipeline::resolve_all_apis();
let batch: Vec<_> = apis.iter().collect();
for diagnostic in validate_rust_client(&batch) {
    eprintln!("{diagnostic}");
}
```

`validate_openapi_registry(module, apis, registry)` adds the schema-registry
check for one module. `pipeline::generate_batch` runs every validator,
publishes the outputs that pass, and returns a `GenerationReport` naming the
succeeded and failed outputs.

The public writers keep the same boundary when called directly:
`output::prepare_rust_output`, `output::generate_and_write_all`,
`export::openapi::write_openapi[_grouped]`,
and `export::postman::write_postman[_grouped]` return
`GeneratorError::Validation` and write nothing when their output's validator
reports an error. `export::openapi::build_openapi_grouped` returns a module's
finished document without writing it, after the group checks above;
`write_openapi_document` writes a built document.

**Planned:** WebSocket definitions are not validated yet. When they are,
they will report through the same `ValidationDiagnostic`, `Severity`,
`Feature`, and `Output` types rather than a separate format.
