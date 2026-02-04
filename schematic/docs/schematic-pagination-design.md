# Schematic Pagination Design

## Summary
This document proposes first-class pagination for Schematic by adding a pagination
contract to `schematic/define` and generating paginated client APIs in
`schematic/gen`. The design covers the strategies documented in
`schematic/docs/pagination/design-patterns.md` and aligns with the
codegen-focused contract sketched in
`schematic/docs/pagination/schema-design.md`.

## Goals
- Define pagination strategies at the API level with endpoint overrides.
- Generate ergonomic, strongly typed pagination APIs in `schematic-schema`.
- Support the dominant strategies (offset/page/cursor/keyset/anchor/link/token).
- Make pagination self-documenting, discoverable, and safe by default.

## Non-goals
- Runtime auto-detection of pagination strategies.
- OpenAPI inference or HTTP traffic observation.
- Full GraphQL support outside of REST-style JSON responses (tracked separately).

## Strategy taxonomy (from `docs/pagination/design-patterns.md`)
- Offset + limit
- Page number + page size
- Cursor token (opaque)
- Keyset (seek)
- Anchor (since/until/after/before)
- Link header pagination (RFC 5988)
- Body links (hypermedia)
- Continuation tokens
- GraphQL connection (optional / future)

## `schematic/define` design

### New module and exports
Add `pagination.rs` and re-export from `schematic/define/src/lib.rs` and
`schematic/define/src/prelude.rs`.

### API-level defaults and endpoint overrides
Add pagination fields to `RestApi` and `Endpoint`:

```rust
pub struct RestApi {
    // ... existing fields ...
    pub pagination: Option<PaginationStrategy>,
}

pub struct Endpoint {
    // ... existing fields ...
    pub pagination: Pagination,
}

pub enum Pagination {
    /// Endpoint does not support pagination.
    None,
    /// Use `RestApi.pagination` as-is.
    Default,
    /// Use API default with local overrides.
    Refine(PaginationRefinement),
    /// Endpoint-specific, fully specified strategy.
    Strategy(PaginationStrategy),
}
```

Notes:
- `Pagination::Default` requires `RestApi.pagination` to be `Some`, otherwise
  `schematic-gen` should fail validation.
- `Pagination::Refine` merges into the API default (see below).
- `Pagination::None` is the explicit opt-out for endpoints on an API that
  otherwise has a default pagination strategy.

### Core pagination model
Use a strategy enum plus shared request/response metadata to keep ergonomics
and codegen predictable.

```rust
pub struct PaginationStrategy {
    pub kind: PaginationKind,
    pub request: PaginationRequest,
    pub response: PaginationResponse,
    pub items: PaginationItems,
    pub limits: PaginationLimits,
    pub direction: PaginationDirection,
    pub ordering: Option<PaginationOrdering>,
}

pub enum PaginationKind {
    OffsetLimit,
    PageNumber,
    CursorToken,
    Keyset,
    Anchor,
    LinkHeader,
    BodyLinks,
    ContinuationToken,
    GraphQLConnection,
}
```

#### Request controls
Pagination controls must specify **where** they live and their names so
codegen can apply them safely.

```rust
pub struct PaginationRequest {
    pub offset: Option<PaginationControl>,
    pub limit: Option<PaginationControl>,
    pub page: Option<PaginationControl>,
    pub page_size: Option<PaginationControl>,
    pub cursor: Option<PaginationControl>,
    pub after: Option<PaginationControl>,
    pub before: Option<PaginationControl>,
    pub token: Option<PaginationControl>,
}

pub struct PaginationControl {
    pub location: PaginationLocation,
    pub name: String,
    pub required: bool,
    pub value_type: PaginationValueType,
    /// Optional JSON pointer (body-only) for nested fields, e.g. "/variables/after".
    pub path: Option<String>,
}

pub enum PaginationLocation {
    Query,
    Header,
    Body,
}

pub enum PaginationValueType {
    Integer,
    String,
    Boolean,
}
```

Notes:
- Most REST pagination uses `Query` controls.
- Body controls are necessary for GraphQL or REST endpoints that embed
  pagination inside a JSON body.
- Header controls are rare but supported for completeness.

#### Response extraction
Codegen needs explicit extraction paths for continuation and termination.

```rust
pub struct PaginationResponse {
    pub has_more_path: Option<String>,
    pub next_cursor_path: Option<String>,
    pub total_items_path: Option<String>,
    pub total_pages_path: Option<String>,
    pub link_header_rels: Option<PaginationLinkRels>,
    pub body_next_link_path: Option<String>,
}

pub struct PaginationLinkRels {
    pub next: String,
    pub prev: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
}
```

Notes:
- Link-header pagination requires header parsing and optional absolute URLs.
- Body links should use JSON-pointer paths ("/links/next").

#### Item extraction
Pagination should yield items, not the raw page wrapper. Items must be explicit
because codegen cannot infer struct fields.

```rust
pub struct PaginationItems {
    pub item_type: Schema,
    /// JSON pointer to the array of items. Use None if the response is a bare array.
    pub items_path: Option<String>,
}
```

#### Limits, direction, ordering
```rust
pub struct PaginationLimits {
    pub default: Option<u32>,
    pub max: Option<u32>,
    pub min: Option<u32>,
}

pub enum PaginationDirection {
    ForwardOnly,
    ForwardBackward,
}

pub struct PaginationOrdering {
    pub fields: Vec<PaginationOrderField>,
}

pub struct PaginationOrderField {
    pub name: String,
    pub direction: PaginationSortDirection,
}

pub enum PaginationSortDirection {
    Asc,
    Desc,
}
```

### Refinement semantics
`Pagination::Refine` merges with `RestApi.pagination`:
- Any `Some(...)` in the refinement replaces the default field.
- `None` means keep the API-level default.
- `PaginationKind` cannot change unless the refinement specifies a full
  `PaginationStrategy` (use `Pagination::Strategy` in that case).

### Examples

API default with per-endpoint override:

```rust
use schematic_define::prelude::*;

let pagination = PaginationStrategy {
    kind: PaginationKind::CursorToken,
    request: PaginationRequest {
        limit: Some(PaginationControl::query("limit")),
        cursor: Some(PaginationControl::query("cursor")),
        ..PaginationRequest::default()
    },
    response: PaginationResponse {
        has_more_path: Some("/has_more".to_string()),
        next_cursor_path: Some("/next_cursor".to_string()),
        ..PaginationResponse::default()
    },
    items: PaginationItems {
        item_type: Schema::new("Widget"),
        items_path: Some("/data".to_string()),
    },
    limits: PaginationLimits { default: Some(50), max: Some(100), min: None },
    direction: PaginationDirection::ForwardOnly,
    ordering: None,
};

let api = RestApi {
    // ...
    pagination: Some(pagination),
    endpoints: vec![
        Endpoint {
            id: "ListWidgets".to_string(),
            // ...
            pagination: Pagination::Default,
            ..Default::default()
        },
        Endpoint {
            id: "ListEvents".to_string(),
            // ...
            pagination: Pagination::Refine(PaginationRefinement {
                items: Some(PaginationItems {
                    item_type: Schema::new("Event"),
                    items_path: Some("/events".to_string()),
                }),
                ..PaginationRefinement::default()
            }),
            ..Default::default()
        },
    ],
};
```

## `schematic/gen` design

### Request parts and query support
Pagination requires query parameters, which are not currently represented. Update
the generated request parts to include query params and apply them in the client:

- `RequestParts` becomes a struct or tuple that includes `query: Vec<(String, String)>`.
- `build_and_send_request` should call `req_builder.query(&query)`.
- Endpoint headers continue to merge with API headers as today.

This change is also useful for non-pagination query params in the future and
should be designed as a general capability.

### Pagination runtime helpers (generated)
Add a small pagination runtime module to the generated schema:

```rust
pub struct Page<Item, State> {
    pub items: Vec<Item>,
    pub next: Option<State>,
}

pub trait PaginatedEndpoint {
    type Request;
    type Response;
    type Item;
    type State;

    fn apply_pagination(req: &mut Self::Request, state: &Self::State);
    fn extract_page(resp: &Self::Response) -> Result<Page<Self::Item, Self::State>, SchematicError>;
}
```

`schematic-gen` should emit an endpoint-specific `State` type and implement
`PaginatedEndpoint` for each paginated endpoint.

### Generated client API
For each paginated endpoint `ListWidgets`, generate:

- `list_widgets_page(request) -> Result<Page<Widget, ListWidgetsState>, SchematicError>`
- `list_widgets_paginator(request) -> Paginator<ListWidgetsEndpoint>`

Example usage:

```rust
let pager = client.list_widgets_paginator(ListWidgetsRequest::default().with_limit(50));
let mut stream = pager.stream();
while let Some(item) = stream.next().await {
    let item = item?;
    // ...
}
```

### Strategy-specific behavior
- **Offset/Limit**: state holds `offset` and `limit`; next offset is
  `offset + items.len()`; termination when `items.len() < limit`.
- **Page/PageSize**: state holds `page` and `page_size`; next page is `page + 1`;
  optionally stop using `total_pages` or empty page.
- **Cursor/Continuation**: state holds `cursor` or `token` from response fields;
  terminate when `has_more` is false or cursor/token is missing.
- **Keyset**: state holds last-seen key fields; `ordering` must be specified; next
  state extracted from the final item in the page.
- **Anchor**: state holds `after`/`before`/`since`/`until`; typically forward-only.
- **Link header**: store `next` URL and allow an absolute URL override in the
  request builder (requires `build_and_send_request` to accept absolute URLs).
- **Body links**: `next` URL extracted via JSON pointer; treat as absolute or
  path relative to the API base.

### Request struct generation changes
When an endpoint is paginated, generate pagination fields and helpers:

- Add optional fields (e.g., `limit: Option<u32>`, `cursor: Option<String>`).
- Add `with_limit`, `with_cursor`, `with_page`, etc., helpers for ergonomics.
- `into_parts()` should translate those fields into query params (or body/header
  controls when `PaginationLocation` indicates otherwise).

### Validation rules
`schematic-gen` should validate and fail generation for:
- `Pagination::Default` with no API-level pagination.
- Pagination on a non-JSON response where item extraction is impossible.
- Missing `items_path` when the response type is not a bare array.
- Body-based controls without a JSON request body.
- Keyset strategies without ordering metadata or item key extraction fields.

## Migration and compatibility
- Default behavior remains unchanged when pagination is omitted.
- Existing APIs can opt in by setting `RestApi.pagination` and
  `Endpoint.pagination`.
- No breaking changes to request/response schemas for non-paginated endpoints.

## Test plan (minimum)
- `schematic-define`: serialization and builder tests for pagination types.
- `schematic-gen`: generated code validation for:
  - RequestParts includes query params
  - pagination fields and helper methods in request structs
  - `PaginatedEndpoint` implementations compile
  - link-header and cursor extraction paths
- `schematic-schema`: compile checks for generated pagination APIs.

## Phased delivery (recommended)
- Phase 1: Offset/Page/Cursor/Continuation/Anchor (query-based + JSON response).
- Phase 2: Link header + body links (requires raw response handling and URL override).
- Phase 3: Keyset + GraphQL connection (requires item key extraction and body controls).
