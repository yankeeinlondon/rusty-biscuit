# Rust schema for describing pagination strategies observed in external APIs

This assumes you’re building an **API observer** (e.g., OpenAPI/SDK inspection + traffic observation) that needs to **characterize an endpoint’s pagination** in a consistent, analyzable way.

Below are two solid approaches that map well to the common patterns: offset/limit, page/page_size, cursor tokens, keyset/seek, Link headers, hypermedia links, continuation tokens, and GraphQL connections.

---

## Approach 1: A normalized “strategy enum” (strongly typed, descriptive)

### Idea

Model pagination as a **single classified strategy** (`enum`) with **variant-specific fields** that capture the contract:

- where inputs are supplied (query params vs headers vs body)
- what the “position” looks like (offset, cursor token, last-seen keys, etc.)
- how next/prev is discovered (`has_more`, `Link` header rels, `links.next`, `pageInfo.endCursor`, etc.)

This is ideal when your observer can confidently decide *what strategy it is* and wants a stable schema for analytics and reporting.

### Example Rust schema

~~~rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointPagination {
    /// The observer's best classification.
    pub strategy: PaginationStrategy,

    /// Optional: how confident the observer is about classification (0.0..=1.0).
    pub confidence: Option<f32>,

    /// Raw signals that justify the classification (useful for audits and debugging).
    pub evidence: Evidence,

    /// Optional: additional notes (e.g. "requires stable sort by created_at desc").
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Evidence {
    /// Observed request controls (query params, headers, body fields).
    pub request_controls: Vec<ControlRef>,
    /// Observed response controls (fields/headers/links that enable navigation).
    pub response_controls: Vec<ControlRef>,
    /// Any ordering constraints discovered (e.g. "created_at desc, id desc").
    pub ordering: Option<Ordering>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ordering {
    pub fields: Vec<OrderField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderField {
    pub name: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRef {
    pub location: ControlLocation,
    pub name: String,
    /// Optional: JSON Pointer-ish path for body fields (e.g. "/links/next", "/pageInfo/endCursor").
    pub path: Option<String>,
    /// Optional: a short description discovered by the observer.
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlLocation {
    Query,
    Header,
    Body,
    Path,
}

/// The normalized classification.
/// Keep variants fairly stable: add fields rather than creating overly specific variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaginationStrategy {
    /// offset=N, limit=M (or skip/take).
    OffsetLimit {
        offset_param: String,
        limit_param: String,
        total_field: Option<String>,
    },

    /// page=N, page_size=M (or per_page).
    PageNumber {
        page_param: String,
        page_size_param: String,
        total_pages_field: Option<String>,
        total_items_field: Option<String>,
    },

    /// Opaque cursor token provided by server and replayed by client.
    CursorToken {
        cursor_param: String,
        limit_param: Option<String>,
        next_cursor_field: Option<String>,
        has_more_field: Option<String>,
        /// Optional: supports backwards pagination (e.g. ending_before).
        prev_cursor_param: Option<String>,
    },

    /// Keyset/seek pagination using last-seen field(s).
    Keyset {
        /// Field(s) used to seek (often 1-2 keys like created_at + id).
        seek_params: Vec<String>,
        limit_param: Option<String>,
        /// Optional: strict ordering required for correctness.
        ordering: Option<Ordering>,
    },

    /// Time/ID anchored ("since", "until", "after", "before").
    Anchor {
        after_param: Option<String>,
        before_param: Option<String>,
        since_param: Option<String>,
        until_param: Option<String>,
        limit_param: Option<String>,
    },

    /// RFC 5988/8288 style `Link` header relations.
    LinkHeader {
        /// Common rels: next, prev, first, last.
        rels: Vec<String>,
    },

    /// Hypermedia links returned in the body (e.g. {"links":{"next":"..."}}).
    BodyLinks {
        next_path: String,
        prev_path: Option<String>,
        first_path: Option<String>,
        last_path: Option<String>,
    },

    /// Cloud-style continuation tokens ("nextPageToken", "continuationToken").
    ContinuationToken {
        token_param: String,
        token_field: String,
        truncated_field: Option<String>,
        limit_param: Option<String>,
    },

    /// GraphQL Relay-style Connection (edges/nodes + pageInfo).
    GraphQLConnection {
        first_arg: Option<String>,   // usually "first"
        after_arg: Option<String>,   // usually "after"
        last_arg: Option<String>,    // usually "last"
        before_arg: Option<String>,  // usually "before"
        page_info_path: String,      // e.g. "/pageInfo"
        end_cursor_path: String,     // e.g. "/pageInfo/endCursor"
        has_next_path: String,       // e.g. "/pageInfo/hasNextPage"
    },

    /// Could not classify, or the API uses a bespoke strategy.
    Unknown {
        /// Preserve any raw hints so downstream can attempt reclassification.
        hints: BTreeMap<String, String>,
    },
}
~~~

### How your observer would use it

1. **Collect signals** (request params and response fields/headers/links).
2. **Classify** into a `PaginationStrategy` variant.
3. Store `EndpointPagination { strategy, confidence, evidence, notes }`.

### Tradeoffs (Approach 1)

**Pros**

- Great for analytics: “group endpoints by strategy,” “count usage of cursor vs offset,” etc.
- Simple consumer UX: one enum variant tells the story.
- Easy to render a human summary (“OffsetLimit: offset/limit, total=…”) in tools.

**Cons**

- You must choose *one* strategy even when reality is messy (hybrids, partial docs, inconsistent endpoints).
- Extensibility pressure: new/bespoke pagination styles may force schema changes (new variants) or overuse of `Unknown`.
- If you only have partial observation, you can misclassify (e.g., token-based vs keyset vs anchor can look similar).

---

## Approach 2: An “evidence-first” observation model + derived classification (flexible, future-proof)

### Idea

Instead of storing the strategy directly, store **what you observed** in a structured way:

- request controls (param names, where they appear)
- response affordances (fields, `Link` headers, body links)
- ordering constraints
- sample shapes (optional)

Then, run an **inference step** that produces a best-effort classification (possibly multiple candidates with scores).

This is ideal when you’re building a *general API inspector* that must handle:

- incomplete documentation
- endpoints with inconsistent implementations
- vendor-specific variations
- version drift over time

### Example Rust schema (observation-first)

~~~rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedPagination {
    pub endpoint: EndpointRef,
    pub request: ObservedRequestControls,
    pub response: ObservedResponseControls,
    pub ordering: Option<ObservedOrdering>,

    /// Optional: snapshots/examples used by the observer (redact sensitive data).
    pub examples: Vec<PaginationExample>,

    /// Free-form extra metadata (e.g. discovered via docs or OpenAPI extensions).
    pub meta: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRef {
    pub method: String,
    pub path_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservedRequestControls {
    pub query_params: Vec<ObservedParam>,
    pub headers: Vec<ObservedHeader>,
    pub body_fields: Vec<ObservedFieldPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservedResponseControls {
    pub headers: Vec<ObservedHeader>,
    pub body_fields: Vec<ObservedFieldPath>,
    pub body_links: Vec<ObservedLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedParam {
    pub name: String,
    pub value_type: Option<ValueType>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedHeader {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedFieldPath {
    /// JSON Pointer-ish (e.g. "/links/next", "/has_more", "/pageInfo/endCursor").
    pub path: String,
    pub value_type: Option<ValueType>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedLink {
    /// Where the link was found (header vs body). For headers, parse Link rels.
    pub location: LinkLocation,
    pub rel: Option<String>,        // next/prev/first/last (if known)
    pub target_path: Option<String> // for body links: where the URL is stored
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkLocation {
    Header,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedOrdering {
    pub fields: Vec<ObservedOrderField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedOrderField {
    pub name: String,
    pub direction: Option<SortDirection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueType {
    Integer,
    String,
    Boolean,
    Object,
    Array,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationExample {
    pub request: BTreeMap<String, String>,  // e.g. {"limit":"50","cursor":"abc"}
    pub response_snippet: BTreeMap<String, String>, // e.g. {"has_more":"true","next_cursor":"..."}
}

/// Output of classification step.
/// You can store this separately or compute on demand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationClassification {
    pub primary: Candidate,
    pub alternatives: Vec<Candidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub score: f32, // 0.0..=1.0
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandidateKind {
    OffsetLimit,
    PageNumber,
    CursorToken,
    Keyset,
    Anchor,
    LinkHeader,
    BodyLinks,
    ContinuationToken,
    GraphQLConnection,
    Unknown,
}
~~~

### What the inference looks like (conceptually)

You implement heuristics such as:

- if response has `Link` header with `rel="next"` → `LinkHeader`
- if request has `offset` + `limit` → `OffsetLimit`
- if request has `page` + `page_size` → `PageNumber`
- if request has `starting_after`/`ending_before` (or generic `cursor`) and response has `has_more`/`next_cursor` → `CursorToken`
- if request has `after_id` or `after_created_at` (especially paired) and ordering hints → `Keyset`
- if response has `/pageInfo/endCursor` and `/pageInfo/hasNextPage` → `GraphQLConnection`

Store the *signals* and the *inference output* (including alternatives) to keep the system explainable.

### Tradeoffs (Approach 2)

**Pros**

- Most robust for real-world API drift and weirdness.
- Handles partial observation: you can say “likely cursor-based, but not sure” with ranked candidates.
- Future-proof: new strategies can be added to the classifier without breaking stored data.

**Cons**

- More moving parts: you need a classifier and scoring/rationale logic.
- Consumers must either run classification or read a separate computed artifact.
- Harder to produce a single authoritative label when signals conflict.

---

## Choosing between them

- Choose **Approach 1 (strategy enum)** if your product needs a *clean contract* for downstream tooling and you can reliably classify (e.g., from OpenAPI + consistent traffic samples).
- Choose **Approach 2 (observation-first + derived classification)** if you’re building a *general inspector* or “API archaeology” tool that must survive ambiguity and evolution.

A common hybrid:

- Store **Approach 2** as the source-of-truth (“what we saw”),
- Store **Approach 1** as a cached/derived summary (“what we believe it is”) for fast querying and UX.


# Refining the pagination schema for “codify → generate strongly-typed client” workflows

Given your updated use case:

1. You **codify** third-party APIs “in the wild” into a Rust API-definition library.
2. A **generation** step emits a strongly typed client where each endpoint is represented as a typed variant (often an enum or a set of endpoint structs).
3. Pagination must be modeled not just for description/analysis, but for **codegen correctness**: the generated client should be able to *page forward/back*, expose *iterators/streams*, and enforce *valid combinations* of parameters.

This shifts the schema design from “observer-friendly classification” to “codegen-friendly execution contract.”

Below are two refined schema approaches (you can also hybridize them).

---

## Design goals for the refined schema

### What changes vs an “analysis-only” schema

An analysis-only schema can stop at: “this endpoint uses cursor pagination.”
A codegen schema must additionally encode:

- **Where pagination controls live** (query/header/body) and their exact names (e.g., `starting_after`, `page[after]`, `offset`, `pageSize`).
- **How to detect termination** (e.g., `has_more=false`, missing `next`, missing `Link: rel="next"`, empty page, `is_truncated=false`).
- **How to extract the continuation** (cursor/token/offset computation) from the response payload or headers.
- **Directionality** (forward-only vs forward+back, e.g. `starting_after`/`ending_before` are mutually exclusive in Stripe).  [oai_citation:0‡Stripe Docs](https://docs.stripe.com/api/pagination?utm_source=chatgpt.com)
- **Ordering requirements** for correctness (keyset pagination requires a stable ordering; Relay connection model uses opaque cursors and explicit pageInfo fields).  [oai_citation:1‡jsonapi.org](https://jsonapi.org/profiles/ethanresnick/cursor-pagination/?utm_source=chatgpt.com)
- **Limits and server constraints** (max page size, default size), so codegen can clamp or expose safe defaults.
- **Item extraction** path (where the “items” live in the response), because paginated endpoints typically return a wrapper object rather than a raw array.

Also: the standard HTTP `Link` header semantics are defined by RFC 5988 (Web Linking).  [oai_citation:2‡RFC Editor](https://www.rfc-editor.org/rfc/rfc5988.html?utm_source=chatgpt.com)

---

## Approach A: Executable pagination contract (trait-based, strongly typed)

### Core idea

Model pagination as an **executable contract** attached to an endpoint spec.
Codegen then emits an endpoint type that implements a `PaginatedEndpoint` trait with associated types:

- `Item` (what the iterator yields)
- `PageToken` / `State` (cursor/offset/anchor)
- methods to **apply** pagination controls to a request builder and **extract** next state from a response

This approach optimizes for **generated client ergonomics and safety**.

### Sketch of a codegen-friendly contract

```rust
/// How the generated client pages through an endpoint.
pub trait PaginatedEndpoint {
    type Request;
    type Response;
    type Item;
    type State;      // cursor/offset/anchor; often Option<State> for "first page"
    type Error;

    /// Applies pagination controls (cursor/offset/page/limit) to the outgoing request.
    fn apply_pagination(req: &mut Self::Request, state: &Self::State);

    /// Extracts items and determines the next state (or termination) from the response.
    fn extract_page(resp: &Self::Response) -> Result<Page<Self::Item, Self::State>, Self::Error>;
}

pub struct Page<Item, State> {
    pub items: Vec<Item>,
    pub next: Option<State>,
}
