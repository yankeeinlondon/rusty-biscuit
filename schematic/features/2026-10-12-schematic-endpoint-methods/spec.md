---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
review_iterations: 0
clarified: false
implemented: false
human_review: true
depends_on:
    - 2026-10-12-schematic-runtime-envelope
related:
    - 2026-10-20-type-constraints
next_spec: 2026-10-13-schematic-pagination
message_to_agent: |-
  Draft created from 2026-10-12-schematic-runtime-envelope review 1, Item 1 ruling. The author approved the call shape in the Examples section during that review; treat it as Decided. Everything under "Proposed decisions" is a draft default, not ruled. This spec runs after the runtime-envelope repair (which delivers the one-name client struct, chained client settings, optional `.build()?`, and `.api_key()`) and before pagination, whose entry points build on the call types defined here.
---

# Schematic endpoint methods

Every generated API client gets one method per endpoint, so a caller finds what an API can do by typing `client.` in an editor, passes required values as arguments, chains optional ones, and sends with `.await`.

## Problem

After the runtime-envelope work, calling an endpoint means knowing the name of a request type, importing it from the right module, building it, and passing it to a generic `send` along with per-call options:

```rust
use schematic_schema::github::{GitHub, ListReleasesRequest};
use schematic_schema::schematic_runtime::CallOptions;

let client = GitHub::new()?;
let releases = client
    .send(ListReleasesRequest::new("yankeeinlondon", "rusty-biscuit").with_per_page(10), &CallOptions::new())
    .await?
    .into_result()?;
```

- **Nothing is discoverable.** Autocomplete on `client.` shows `send`, `send_raw`, and `check_link`, not the 30-odd GitHub endpoints or OpenAI's 265.
- **The basic path is noisy.** Every call passes `&CallOptions`, and getting the data takes two `?`.

## Examples (Decided)

The author approved this call shape during the runtime-envelope review (Item 1):

```rust
let gh = GitHub::new();   // client shape from the runtime-envelope repair

// required values are arguments; optional ones are chained; `.await` sends
let releases = gh.list_releases("yankeeinlondon", "rusty-biscuit")
    .per_page(10)
    .await?;                              // Vec<Release>, one `?`

// per-call overrides chain the same way
let repo = gh.get_repository("yankeeinlondon", "rusty-biscuit")
    .anonymous()
    .await?;

// the full reply (status, headers, request id) when wanted
let reply = gh.list_releases("yankeeinlondon", "rusty-biscuit").full().await?;

// optional early checking, mirroring the client's `.build()?`
let call = gh.create_issue("yankeeinlondon", "rusty-biscuit", "Title")
    .body(text)
    .build()?;
let issue = call.await?;
```

## Requirements

**R1. One method per endpoint on the generated API struct.** For every REST endpoint, the API's generated struct (`GitHub`, `OpenAI`, …) has an inherent method. Being inherent, it shows in autocomplete with no trait import. The method name is the endpoint id in snake case (`ListReleases` becomes `list_releases`). Its arguments are the endpoint's required values, in the same order today's `{Endpoint}Request::new` takes them. It returns that endpoint's call type (R2) and sends nothing by itself.

**R2. A call type per endpoint.** Each method returns a generated call type that holds the client reference, the request, and the per-call options.

- It has one chainable setter per optional request value, named after the value without a `with_` prefix (`.per_page(10)`).
- It has the shared per-call methods: `.anonymous()`, `.credential(..)`, `.deadline(..)`, `.full()`, and `.build()`.
- It is a generated local type, so later specs (pagination first) can add inherent methods to it, such as `.pages()`.

**R3. `.await` sends and returns the data.** A call implements `IntoFuture`. Awaiting it returns `Result<Success, E>`, where `E` is one error type covering both "no reply arrived" (`TransportError`) and "the reply was an error or could not be read" (`ApiError`). The basic path therefore needs one `?`. The error keeps everything the two current error types carry.

**R4. `.full()` returns the envelope.** Awaiting `.full()` returns today's `Result<Response<Success, Failure>, TransportError>`, with metadata, status, headers, and request id, for callers who need more than the data.

**R5. `.build()?` checks a call before sending.** `.build()` runs everything that can fail before network work: argument encoding (today's `InvalidArgument`), and value constraints once `2026-10-20-type-constraints` lands. It returns the same call type, ready to `.await`. A built call is `Clone`, so it can be sent again. Without `.build()`, the same checks run at `.await`.

**R6. Request objects remain available.** `{Endpoint}Request` types and a generic send stay public for generic code and for requests built elsewhere. The generic entry returns a runtime call type with the same shared per-call methods and the same `.await` and `.full()` behavior, so both paths behave identically.

**R7. Documentation follows the method.** Each endpoint method's rustdoc carries:

- the endpoint's description;
- its HTTP method and path;
- its required arguments;
- the optional setters available on its call type.

A caller reading the hover text in an editor needs nothing else.

**R8. Compile cost is measured, not assumed.** `2026-10-13-schematic-pagination` rejected per-endpoint methods for compile cost without measuring it. This spec records, before and after, on one host:

- clean and incremental `cargo build` time of `schematic-schema`, with OpenAI's 265 endpoints reported separately;
- the size of the generated crate.

The numbers are recorded in the implementation log and presented to the author. If the author judges the cost too high, the fallback in PD3 applies.

**R9. In-repo consumers adopt the methods.** Every consumer of generated REST clients (biscuit-speaks, homelab, sniff, model-citizen if still a consumer) moves from request objects plus `send` to endpoint methods, except where generic code needs R6. This is the evidence that the surface is usable, and it removes the old pattern from the codebase.

**R10. A docs page explains calling an API.** `schematic/docs/` gains a page for a developer new to the repository covering:

- creating a client;
- client-wide settings (base URL override, `api_key`, timeout, proxy);
- calling an endpoint with required and optional values;
- per-call overrides, `.full()`, and `.build()?`;
- when to use request objects instead.

It uses the GitHub and OpenAI examples above.

## Non-goals

- WebSocket APIs. They belong to `2026-10-17-schematic-websocket`.
- Pagination. `2026-10-13-schematic-pagination` adds `.pages()` and related entry points onto the call types defined here.
- Value constraints. Those are `2026-10-20-type-constraints`; this spec only gives them a place to run (R5).
- Changing endpoint definitions.

## Proposed decisions (unruled)

**PD1. Shape of the combined error (R3).** *Default:* a runtime `CallError<E>` enum with `Transport(TransportError)` and `Api(ApiError<E>)`, implementing `std::error::Error`, with accessors for the shared request metadata.

**PD2. Name collisions.** An optional value could share a name with a shared per-call method (`full`, `build`, `anonymous`, `credential`, `deadline`), and a method name could collide with an existing inherent method on the API struct (`new`, `builder`, client settings). *Default:* the generator detects every collision and appends `_param` to the optional value's setter, or `_endpoint` to the endpoint method, with a generator diagnostic listing each rename. No silent renaming without a diagnostic.

**PD3. Fallback if compile cost is unacceptable (R8).** Optional values are passed as one options struct argument instead of chained setters: `gh.list_releases(owner, repo, ListReleasesOptions { per_page: Some(10), ..Default::default() })`. The method-per-endpoint discoverability is kept either way.

**PD4. How the per-endpoint call type shares the per-call methods.** *Default:* the generated call type wraps the runtime's generic call type, and the generator emits the handful of shared per-call methods on each call type as one-line forwards. This keeps them inherent, so no trait import is needed. The forwarding is generated, so it costs generator output size, not hand maintenance.

## Validation

- `schematic-gen`, L1: generated-output assertions for a fixture API covering required arguments, optional setters, collisions (PD2), and rustdoc content.
- `schematic-runtime`, L1:
  - `.await` versus `.full()` results;
  - `.build()` refusing an invalid argument with no network work;
  - a built call sent twice.
- A trybuild compile test that a fixture API's endpoint methods resolve with only the API struct imported (no trait import).
- R8 measurements recorded in the implementation log.
- Consumer packages' `just test` and `just lint` after migration (R9).

## Human review agenda

1. Compile-cost numbers (R8) and whether PD3's fallback is needed.
2. The combined error's name and shape (PD1).
3. The collision-renaming rule (PD2).
