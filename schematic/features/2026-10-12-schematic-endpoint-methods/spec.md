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
depends_on:
    - 2026-10-12-schematic-runtime-envelope
related:
    - 2026-10-20-type-constraints
next_spec: 2026-10-13-schematic-pagination
refresh_required: false
human_review: true
human_review_items:
  - |-
    **Please confirm: no per-API on/off switches (feature flags) for the generated API crate. Branch A has been applied, pending your answer.**

    This work gives every generated API client one method per endpoint, which made the generated crate (`schematic-schema`) slower to compile. The spec's rule (D3) says: if a clean debug build gets more than 25% slower, add one feature flag per API, so that a program using only ElevenLabs does not compile the other 17 APIs. The two ways of reading "the build" give different answers. You were asked before Phase 5, but no answer was recorded, so Phase 5 applied the plan's own rule (option A below). That changed no code, so it is easy to reverse. Phase 6 has since moved the consumers (biscuit-speaks, sniff, and the homelab crates) onto the new methods. Their call code works the same under A or B, so choosing B now would add only a feature line to each consumer's `Cargo.toml`, plus the generator change. It is best decided before the final phase closes the work.

    Measured on the development Mac (debug builds, old and new code built back to back):
    - **The whole build command**, including dependencies every user compiles anyway: 45.5 s before, 47.5 s after, **+4.5%**.
    - **The `schematic-schema` crate alone**, the only part this work changed: 6.4 s before, 8.5 s after, **+34%**.

    Options:
    - **A. Keep it as applied: no feature flags.** This uses the whole-command number, as the plan's ruling 14 and D3's literal wording do. *Pros:* nothing more to build, and consumers keep their current setup. *Cons:* about 2 s more on every clean build of every consumer, for every API whether it is used or not.
    - **B. Switch to per-API feature flags.** This uses the crate-only number. *Pros:* a consumer compiles only the APIs it uses, which also helps as more APIs are added. *Cons:* a generator change, plus a `Cargo.toml` change in six consumer crates. In a whole-workspace build the flags merge, so the saving shows mainly when one package is built alone. If you choose B, it should be done before the final phase (Phase 8).
    - **C. Keep A now, and record per-API feature flags as a separate future item** to revisit if the API count or build times grow. *Pros:* this change stays small. *Cons:* the 2 s per clean build remains until then.

    **Recommendation: A (or C if you want the idea tracked).** The extra wait a user actually sees is about 2 s on a build of about 46 s, well under 25%. Branch B's generator change and six consumer edits are out of proportion to that saving.
message_to_agent: |-
  Phase 7 is complete: docs, READMEs, and skills now describe endpoint methods and CallError. Read "## Phase 7" in implementation-log.md.
  - The author has still NOT confirmed D3 branch A; the human_review item stays open. If the spec now records B, run plan Tasks 5.B.1-5.B.3 before Phase 8's validation, and add the per-API features to schematic/docs/dependencies.md, schematic/schema/README.md, and docs/runtime/calling-an-api.md.
  - New page: schematic/docs/runtime/calling-an-api.md. Its Rust examples are not compiled; they were hand-checked against generated signatures. If Task 8.3's audit wants executable evidence for R10, the existing schema tests (github_client.rs) cover every documented behavior; see the Phase 7 requirement-to-test table.
  - The only source change was a /// comment on api_client! (schematic/runtime/src/api_client.rs). schematic `just test` and `just lint` pass; consumers were not re-run in Phase 7 because their source did not change.
  - The schematic-define SKILL.md entry points exceed the 200-line target (308 and 287 lines). They were already over before this work; trimming them is out of scope, but worth listing as an open item in Task 8.4.
  - Phase 8 Task 8.2 (`just ci-local --plan`) will show the doc and skill changes; per repo CI rules, non-source Markdown schedules no package unless a test reads it.
---

# Schematic endpoint methods

Every generated API client gets one method per endpoint, so a caller finds what an API can do by typing `client.` in an editor, passes required values as arguments, chains optional ones, and sends with `.await`.

## Problem

After the runtime-envelope work, calling an endpoint means knowing the name of a request type, importing it from the right module, building it, and passing it to a generic `send` along with per-call options:

```rust
use schematic_schema::github::{GitHub, ListReleasesRequest};
use schematic_schema::schematic_runtime::CallOptions;

let client = GitHub::new();
let releases = client
    .send(ListReleasesRequest::new("yankeeinlondon", "rusty-biscuit").with_per_page(10), &CallOptions::new())
    .await?
    .into_result()?;
```

- **Nothing is discoverable.** Autocomplete on `client.` shows `send`, `send_raw`, and `check_link`, not the 16 GitHub endpoints or OpenAI's 265.
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

The numbers are recorded in the implementation log, and the decision rule in D3 is applied to them. No pause for the author is needed.

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

## Decisions

The author delegated these rulings on 2026-10-12, so the pipeline can run without pausing for them. Each records the reason it was chosen.

**D1. Combined error (R3): Decided.** The runtime defines `CallError<E>`:

- **Shape:** a `#[non_exhaustive]` enum with `Transport(TransportError)` and `Api(ApiError<E>)`.
- **Traits:** it implements `std::error::Error` and `Display`.
- **Accessor:** `request_meta()` returns the request metadata both variants share.

Why:

- **The name.** `CallError` names exactly what failed, a call. It doesn't shadow `std::error::Error` or crate-level `Error` aliases in consumers.
- **`#[non_exhaustive]`.** Later specs can add a variant (for example, pagination's own failures) without breaking callers' `match`.

**D2. Name collisions: Decided.** Collisions are detected by the generator and resolved with a diagnostic, never silently.

| Collision | Resolution | Why |
|---|---|---|
| An optional value's setter collides with a shared per-call method (`full`, `build`, `anonymous`, `credential`, `deadline`, plus any method later specs add to call types, such as pagination's `pages`) | The setter gets a `_param` suffix (`.full_param(..)`) | The per-call methods have to be stable across every endpoint. Renaming the rarer, endpoint-specific name keeps `.full()` meaning the same thing everywhere. |
| An endpoint method collides with an inherent method on the API struct (`new`, `builder`, client settings) | The endpoint method gets an `_endpoint` suffix | Same reason: client methods must be stable across APIs |
| A name is a Rust keyword | The existing request-field name sanitizer is reused, so setters match today's field names | Keeps one rule instead of two |

The generator's diagnostic lists every rename. A test asserts the rename list for the shipped definitions, so a new collision shows up in review.

**D3. Compile cost (R8, formerly PD3): Decided, as a measured rule.** Chained setters are the default. Measure a clean debug `cargo build -p schematic-schema` on the development Mac, before and after:

1. **The increase is 25% or less:** keep chained setters. Done.
2. **The increase is more than 25%:** add per-API Cargo features to `schematic-schema`, default all on, so that a consumer can compile only the APIs it uses. Switch each in-repo consumer to the features it needs. Then measure the dependent build of the heaviest single-API consumer (biscuit-speaks with ElevenLabs) against its own baseline.
   - If that increase is 25% or less, keep chained setters.
   - Otherwise switch to the options-struct fallback: `gh.list_releases(owner, repo, ListReleasesOptions { per_page: Some(10), ..Default::default() })`. Endpoint methods remain either way.

Why this order:

- Today every consumer compiles all 18 APIs (`schematic-schema` has no feature flags), so most of any increase would be paid for APIs a consumer never calls.
- Per-API features remove that waste without giving up the approved call style. The options struct is the last resort because it reads worse.
- Record every measurement and which branch was taken in the implementation log.

**D4. Sharing per-call methods (PD4): Decided as drafted.** The generated per-endpoint call type wraps the runtime's generic call type. The generator emits the shared per-call methods on each call type as one-line forwards.

Why: forwards keep them inherent, so no trait import is needed, which is the point of this feature. They're generated, so they cost output size, not hand maintenance. The output size is counted in D3's measurement.

**Note: shared helpers into the runtime (candidate).** Partial generation keeps two compatibility checks: the `schematic-runtime` ABI marker, and a byte comparison of the generated `shared.rs` and `ws_shared.rs`, which the marker does not cover. The direction set by `2026-10-12-schematic-runtime-envelope` review 1, item 6 is to move shared helpers into `schematic-runtime` as features touch them, until no generated shared helper file remains and the byte comparison can be deleted. Where this feature touches the `shared.rs` request helpers (`encode_cookie_value`, `combine_cookie_headers`, `FormFile`), consider moving them into the runtime rather than extending the generated file, and log the choice.

## Validation

- `schematic-gen`, L1: generated-output assertions for a fixture API covering required arguments, optional setters, collisions (D2), and rustdoc content.
- `schematic-runtime`, L1:
  - `.await` versus `.full()` results;
  - `.build()` refusing an invalid argument with no network work;
  - a built call sent twice.
- A trybuild compile test that a fixture API's endpoint methods resolve with only the API struct imported (no trait import).
- R8 measurements recorded in the implementation log.
- Consumer packages' `just test` and `just lint` after migration (R9).

## Human review agenda

None open. D1–D4 were ruled under the author's delegation. A reviewer should still flag any measurement or collision result that the D2 and D3 rules don't cover.

## Refresh notes

Verified after runtime-envelope review 3 on 2026-10-12:

- In schematic-gen, [generate_api_struct](../../gen/src/codegen/api_struct/mod.rs) emits one API struct and an internal API tag. In schematic-runtime, [api_client!](../../runtime/src/api_client.rs) supplies its inherent settings and sends. `GitHub::new()` is infallible; `.build()` validates and returns the same `GitHub` type. The current call is still `send(request, &CallOptions)`; endpoint methods, call types, and `CallError` remain this feature's work.
- In schematic-runtime, [Response::into_parts and ApiError](../../runtime/src/response.rs) define the existing evidence split: metadata belongs to `Response`, while `ApiError` carries only `kind` and `body`. `Response::into_result()` discards metadata. The decided combined-error `request_meta()` accessor therefore needs to retain metadata before that conversion; the existing API failure alone cannot supply it. This records the implementation baseline without changing the decided error shape.
- The runtime and generator ABI constants are both `2`, in [schematic-runtime](../../runtime/src/lib.rs) and [schematic-gen](../../gen/src/lib.rs). Keep the runtime check and generated-shared-file comparison until no generated helper file remains, as already decided.
- `schematic-schema` is excluded from the workspace. From the repository root, address it with `cargo build --manifest-path schematic/schema/Cargo.toml` (or the corresponding `cargo check` / nextest invocation), rather than a workspace-only `-p schematic-schema`. The decided measurement thresholds and fallback order are unchanged.
- Buffered response elapsed time includes body reading, decoding, hooks, and redirects. A returned live stream's elapsed time stops when the decoder returns; subsequent reads retain their own timeout and deadline bounds. New call adapters must preserve these runtime results.

## Refresh log

### 2026-10-12 — Runtime-envelope review 3

Source: `2026-10-12-schematic-runtime-envelope` review 3 and its implementation and repair logs.

- Removed `?` from the current-client example because generated `new()` is now infallible.
- Corrected the GitHub endpoint count from “30-odd” to the verified 16; OpenAI remains 265.
- Added verified client, error-evidence, timing, ABI, and schema-command handoffs so implementation starts from the runtime that exists.
- Set `refresh_required: false` and replaced the implementer note with the verified baseline. The decided requirements, error shape, collision rules, measurement policy, and pipeline order are unchanged.
- Refreshed only this spec: its plan does not exist yet. There was no existing Refresh notes section or `parent` property; authority was checked against the runtime-envelope reviews and the parent macro review named by that feature.
