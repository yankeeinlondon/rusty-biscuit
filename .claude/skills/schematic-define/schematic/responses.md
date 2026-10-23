# Responses, errors, streams, and GraphQL

Response model v2: an endpoint stores one or more success shapes and an
effective error table, and a REST API may carry GraphQL operations. Canonical
behavior: `schematic/docs/responses-and-errors.md`, `schematic/docs/graphql.md`,
`schematic/docs/runtime/codecs.md`, `schematic/docs/pagination.md`.

## Success shapes

`Endpoint::response: Vec<SuccessShape>` (read it with
`Endpoint::success_shapes()`); each `SuccessShape` holds one `ResponseShape`
(`Json(TypeRef)`, `Text`, `Binary`, `Empty`, `Stream { framing, end, keep }`)
plus selection facts (`content_type`, `statuses`, `json_tag`, `fallback`,
`note`, `variant`). Author them through the builder, never as literals:

```rust
Endpoint::post("Chat", "/api/chat")
    .describes("Chat completion (NDJSON by default)")
    .request_json::<ChatBody>(ctx)
    .returns(
        Success::ndjson(ctx.register::<ChatStreamItem>()?)
            .note("returned when `stream` is true or omitted")
            .ends_when_field("done", true)
            .keep_end(),
    )
    .returns(Success::json(ctx.register::<ChatCompletion>()?).note("returned when `stream` is false"))
    .build()?
```

- `Success::{json(desc), text(), binary(), empty(), sse(item), ndjson(item),
  bytes()}`; shortcuts `returns_json::<T>(ctx)`, `returns_text()`,
  `returns_binary()`, `returns_empty()`. `.returns(..)` may repeat.
- Modifiers: `content_type`, `status` (exact 2xx, repeatable), `tagged(field,
  value)`, `json_array`/`json_object`, `fallback`, `note` (docs only),
  `variant` (enum variant name), and for streams `ends_at_data_line`
  (`[DONE]`), `ends_at_event` (`message_stop`), `ends_when_field(path,
  value)`, `keep_end`. A stream without `ends_*` ends at connection close;
  `ends_*`/`keep_end` on a non-stream fails at build.
- Endpoint build runs `response::validate_success_shapes`: concrete media
  types, 204 only on the one empty shape, statuses in 200–299, shapes sharing
  a status and media type must be JSON told apart by a tag (a sequence/map
  type is tagged array/object automatically), at most one fallback, and
  stream end conditions that suit the framing. `json_references()` includes
  SSE/NDJSON item types, so a dangling item fails API build and bundle load.
- Selection at runtime (mirrors validation): 204 → empty shape; candidates are
  the shapes declaring the status, else the status-less ones; normalized
  `Content-Type` match; JSON tag; else fallback. Nothing left is
  `ErrorKind::UnexpectedContentType`. A single shape never checks the reply.
- Model a streaming endpoint as a stream shape (Ollama, LM Studio, Anthropic,
  ElevenLabs, OpenAI `GetResponse`), never as `Binary` framed by the caller.
- Inferred pagination reads only a sole JSON shape; a multi-shape list needs
  an explicit `Collection`.
- Hand-built fixtures: `SuccessShape::{new, json, json_type, json_vec_type,
  text, binary, empty}`. `SuccessShape::plain()` is `Some` for a lone
  fact-free non-stream shape, whose generated and exported output matches
  what pre-v2 endpoints produced.

### Generated success types

| Shapes | `type Success` | `type Decoder` |
|---|---|---|
| one `Json(T)` | `T` | `schematic_runtime::Json<T>` |
| one `Text` / `Binary` / `Empty` | `String` / `bytes::Bytes` / `()` | `Text` / `Binary` / `Empty` |
| one SSE / NDJSON stream | `ItemStream<T>` | `Sse<T, End>` / `Ndjson<T, End>` |
| one byte stream | `ByteStream` | `ByteStream` |
| several | `{Id}Response` enum | `Shapes<{Id}Response>` |

`End` is a generated `{Id}End` (or `{Id}{Variant}End`) marker, or
`UntilClose`. A stream is consumed with `stream.next().await` →
`Option<Result<T, StreamError>>` (it also implements `futures_core::Stream`); `StreamError`
variants other than `Malformed` are terminal. `Client::max_frame_bytes(n)`
bounds one frame (default `DEFAULT_MAX_FRAME_BYTES`, 16 MiB). Response
mutators (`mutate_response`) compile only for JSON endpoints and all-JSON
alternatives; streams get none.

## Declared errors

```rust
// API defaults every endpoint and operation inherits
RestApi::builder(&ctx, "Api", BASE)
    .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::json(ctx.register::<Problem>()?)))
// Endpoint overlay: adds entries, an identical match replaces the API's
Endpoint::get("Get", "/x").describes("..").returns_empty()
    .errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::text()))
// Endpoint opt-out of every declared error, API defaults included;
// combining it with entries fails at build
Endpoint::get("Ping", "/ping").describes("..").returns_empty().errors(ErrorSet::none())
```

- `StatusMatch::{exact(n), range(lo, hi), class(4|5)}` within 400–599;
  `ErrorShape::{json(desc), text(), binary(), empty()}.content_type(..)`; no
  stream error shapes.
- Stored in `RestApi::errors`, `Endpoint::errors`, `Endpoint::no_declared_errors`
  (and the same pair on `GraphQlOperation`). Resolve with
  `RestApi::effective_errors(&endpoint)` / `effective_operation_errors(&op)` →
  `ErrorTable` (`entries`, `lookup`): an identical endpoint match replaces the
  API entry in place; lookup is exact, then narrowest range. Equal-width
  overlapping ranges are rejected.
- Generated `type Failure`: `NoDeclaredError` for an empty table; the one
  distinct shape's type (`String`, `Bytes`, `()`, `serde_json::Value`, or the
  registered JSON type with a generated `impl ErrorBody<{Api}Api>` per API
  module); several distinct shapes give an `{Id}Error` enum. `const ERRORS`
  routes each `StatusMatch` to its shape index.
- Runtime: a matched status decodes as `ErrorKind::Declared(E)`; unmatched is
  `Undeclared`; undecodable is `MalformedError`. `Declared` also carries a
  failure a 2xx body reports (GraphQL errors).
- Name trap (D-15/D-23): a registered type named `{Id}Response` or `{Id}Error`
  collides with the generated enum (`generated-name-collision`); Ollama's
  buffered replies were renamed for this.

## GraphQL operations

```rust
let issues = GraphQlOperation::query("Issues", ISSUES_DOCUMENT)
    .describes("Open issues")
    .variables::<IssuesVariables>(&mut ctx) // or .no_variables()
    .data::<IssuesData>(&mut ctx)
    .operation_name("Issues")               // optional
    .errors(ErrorSet::new().on(StatusMatch::exact(403), ErrorShape::json(problem)))
    .pagination(Relay::forward("repository.issues").max_page_size(100).collection("Issue"))
    .build()?;
let api = RestApi::builder(&ctx, "GitHub", BASE).describes("..").bearer_env(["GITHUB_TOKEN"])
    .graphql(AddressRule::github())         // ordered rules; required before .operations
    .operations([issues])
    .build()?;
```

- `GraphQlOperation::{query, mutation}` (typestate: `describes`, one of
  `variables`/`no_variables`, `data`). Stored in `RestApi::graphql:
  Option<GraphQlSurface { address, operations }>`. The document is opaque,
  sent verbatim. Typed variables must be a named type or map.
- `AddressRule::{replace_suffix(suffix, with), append(path)}`; `github()` is
  `[replace_suffix("/api/v3", "/api/graphql"), append("/graphql")]`. Segment
  matching, trailing slash ignored. Callers override with
  `Client::graphql_url(..)`.
- Operation ids share the generated naming space with endpoint ids. Operation
  rule failures are `DefinitionError::InvalidOperation` (`{api} operation
  {id}: ...`); address and base-URL failures are `InvalidDefinition`.
- Generated: a request struct `{id}{suffix}` (typed variables are its one
  field, taken by `new`), `ADDRESS = Address::GraphQl`, codec `GraphQl<D>`,
  failure `GraphQlOperationError<D, H>` (`GraphQl(GraphQlFailure { errors,
  partial_data })` or `Http(H)`). No endpoint method: send with
  `client.send(IssuesRequest::new(vars))`.
- OpenAPI skips operations with a `graphql-operation` warning; Postman
  exports each as a runnable `POST` to `{{graphqlUrl}}` with `query`,
  `operationName`, and example `variables`. No shipped definition declares
  operations yet.

### Relay pagination

`schematic_define::pagination::Relay` (not in the prelude) expands to an
ordinary cursor `Strategy` plus `Collection`:

| | `Relay::forward(root)` | `Relay::backward(root)` |
|---|---|---|
| cursor / size variables | `variables.after` / `variables.first` | `variables.before` / `variables.last` |
| cursor read | `{root}.pageInfo.endCursor` | `{root}.pageInfo.startCursor` |
| more while | `{root}.pageInfo.hasNextPage` | `{root}.pageInfo.hasPreviousPage` |

`.with_edges()` reads `{root}.edges[*].node` instead of `{root}.nodes`;
`.cursor_variable(..)`, `.size_variable(..)`, `.max_page_size(n)`,
`.sent_page_size(n)`, `.resumable()` (caller resumes with `.after(..)`), then
`.collection("Item")`. One direction per pager; there is no bidirectional
preset. Paths read under `data`; every pager input must be a body path under
`variables`; the strategy must be explicit, not next-link, with no follow-up,
session, or alternates; a pager-managed variable listed in the variables
schema's `required` fails API build (make it `Option`).

The generated request implements `Paginated` with `page_root =
schematic_runtime::graphql::page_data`: a page with GraphQL errors stops
paging as `ErrorKind::Declared(GraphQlOperationError::GraphQl(..))`, and
partial data never joins collected items. Page it with
`client.send(req).pager().page_size(50).collect_all().await`. REST endpoints
inherit the default `Paginated::page_root` (the whole body); a custom root
returns `PageRootError::{Declared, Malformed}`. Runtime `ABI` is 7.

## Provider notes

- OpenAI is importer output: `definitions/src/openai_pagination.rs` narrows
  every imported endpoint to the one shape the pre-v2 importer chose, with no
  errors; migrate an endpoint there (only `GetResponse` so far), never in
  `openai/mod.rs`.
- Bitbucket `GetSource` declares `Directory` (JSON) and `File` shapes;
  `sniff` reads files with `send_raw`, since a committed `.json` file would
  select `Directory`.
