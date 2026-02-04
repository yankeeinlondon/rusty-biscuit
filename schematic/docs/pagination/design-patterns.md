# Dominant Design Patterns for Pagination


## 1) Offset + limit

**How it works:** client supplies `offset` (start index) and `limit` (page size).
**Pros:** trivial to understand; supports random access (“jump to offset 10,000”).
**Cons:** can get slow on large datasets; **unstable under writes** (inserts/deletes can shift results → duplicates/misses).

**Example**

Request:

```http
GET /v1/widgets?offset=40&limit=20

Response:

{
  "data": [/* 20 widgets */],
  "offset": 40,
  "limit": 20,
  "total": 1234
}

Common variants: skip/take, start/count.

⸻

2) Page number + page size

How it works: client supplies page and page_size (or per_page). Internally, many implementations convert to offset.

Pros: “classic UI” semantics; easy for humans and docs.
Cons: same write-instability as offset; deep pages often slow.

Example

Request:

GET /v1/widgets?page=3&page_size=25

Response:

{
  "data": [/* 25 widgets */],
  "page": 3,
  "page_size": 25,
  "total_pages": 50
}


⸻

3) Cursor (opaque token) — “cursor-based pagination”

How it works: server returns an opaque cursor (or next_cursor) you pass back to get the next slice. Cursor encodes position (often plus filters/sort) but is not interpreted by clients.

Pros: performant; stable under many write patterns (especially append-only feeds); avoids large offsets.
Cons: no easy random access; cursors are typically tied to sort/filter.

This is common in Stripe-style list endpoints and many modern APIs.

Example

Request:

GET /v1/widgets?limit=50

Response:

{
  "data": [/* 50 widgets */],
  "has_more": true,
  "next_cursor": "eyJwb3MiOjUwLCJzb3J0IjoiaWQ6YXNjIn0="
}

Next request:

GET /v1/widgets?limit=50&cursor=eyJwb3MiOjUwLCJzb3J0IjoiaWQ6YXNjIn0=


⸻

4) Keyset pagination (a.k.a. “seek method”) using a stable sort key

How it works: client paginates by providing the last seen key(s), e.g. created_at, id, or compound keys like (created_at, id).

Pros: very fast; stable under inserts before the current window (when done right).
Cons: requires a deterministic ordering; less flexible for arbitrary sorting; can’t jump to page N without scanning.

Example (by time then ID)

First page:

GET /v1/widgets?limit=50&sort=created_at:asc,id:asc

Next page uses the last item’s keys:

GET /v1/widgets?limit=50&sort=created_at:asc,id:asc&after_created_at=1707000000&after_id=9821


⸻

5) “Since / until” or “after / before” (ID-based feed pagination)

How it works: pass an anchor like since_id, max_id, after, before. Common in event streams and timelines.

Pros: maps well to feeds; easy incremental sync (“give me anything after X”).
Cons: semantics vary; needs clear sorting guarantees (usually by ID or timestamp).

Example flavor used in GitHub REST-style list endpoints (often alongside per_page):

GET /repos/acme/widgets/issues?per_page=50&since=2026-02-01T00:00:00Z

Response typically includes items newer than since.

⸻

6) Hypermedia pagination links (next/prev URLs)

How it works: server returns full URLs for next, prev, sometimes first/last. Clients follow links; params can be opaque.

Pros: client logic stays simple; server can change strategy without breaking clients.
Cons: harder to construct URLs manually; must treat links as authoritative.

Example

Response:

{
  "data": [/*...*/],
  "links": {
    "next": "/v1/widgets?limit=50&cursor=abc",
    "prev": "/v1/widgets?limit=50&cursor=xyz"
  }
}


⸻

7) RFC 5988 / Web Linking via HTTP Link header

How it works: pagination URLs delivered in the Link response header using rel="next", rel="prev", etc.

Pros: standard HTTP mechanism; doesn’t clutter body; widely used.
Cons: some client stacks ignore headers by default; still need to parse.

Example

Response headers:

Link: </v1/widgets?per_page=50&page=2>; rel="next",
      </v1/widgets?per_page=50&page=10>; rel="last"

Body just contains the page data.

⸻

8) Continuation tokens (cloud APIs)

How it works: response includes a token like continuationToken, nextPageToken, marker. You pass it back to continue. Functionally similar to cursors but often explicitly named and sometimes not tied to sorting choices.

Common in AWS-style list operations.

Example

Request:

GET /v1/objects?max_keys=1000

Response:

{
  "objects": [/*...*/],
  "is_truncated": true,
  "next_continuation_token": "1w2x3y..."
}

Next request includes continuation_token=1w2x3y....

⸻

9) GraphQL “Connection” pattern (edges/nodes + pageInfo)

How it works: standardized shape: edges { node cursor } + pageInfo { hasNextPage endCursor }, with first/after (and last/before).

Pros: consistent across schemas; cursor-based and explicit about boundaries.
Cons: verbose; still requires stable ordering; can be misused if backend doesn’t honor cursor semantics.

Commonly associated with Relay conventions.

Example (GraphQL query shape)

Request conceptually:

widgets(first: 50, after: "cursor123") {
  edges { cursor node { id name } }
  pageInfo { hasNextPage endCursor }
}


⸻

Practical notes (things you should watch for)
 • Sorting guarantee: Good pagination docs state an explicit order (e.g., created_at asc, id asc). Without it, duplicates/misses are likely.
 • Filtering + cursors: Cursors usually encode filters/sort; mixing a cursor with different filters should error.
 • Total counts: total is common in offset/page-number APIs but expensive at scale; cursor-based APIs often omit it or provide it separately.
 • Write behavior: If the dataset changes frequently, prefer cursor/keyset strategies with a stable order key.

