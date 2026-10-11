# HTTP API pagination strategies: catalog

Spike date: 2026-10-09. Purpose: inform one enumeration of "pagination strategies" in a Rust API-definition library.

Verification legend: **[V]** = fetched from the cited official page this session. **[M]** = from memory/general knowledge, not re-verified (flagged; check before relying on it). Several pages returned 403/404/truncated (Google Drive guide, OpenAI reference, Jira intro, Twilio, Salesforce); those APIs are marked [M].

## 1. Strategies

### 1.1 Page number + page size ("page/per_page")

- Client asks: `page=N` (1-based, usually) plus `per_page` / `page_size` / `count` / `PageSize`; query string.
- "More" signal: varies per API. `Link` header with `rel=next` (GitHub), `X-Next-Page` / `X-Total-Pages` headers (GitLab), `next_page` body field null at end (Zendesk offset), `total` count, or empty/short page.
- Items: root array (GitHub) or nested (`data`, `items`).
- Backward: yes, trivially (`page-1`); GitHub and GitLab also emit `rel=prev`.
- Page size: client-selectable. GitHub max 100 for most endpoints, over-large values silently capped, default per endpoint [V]. GitLab default 20, max 100 [V].
- Caveats: drift under concurrent inserts/deletes (items skipped or duplicated); deep pages slow; hard caps (Zendesk offset: first 100 pages / 10,000 records, then 400 [V]; GitLab: above 10,000 records `X-Total`, `X-Total-Pages`, `rel="last"` omitted, plus an instance-configurable max offset [V]).
- Examples:
  - GitHub REST: `page`, `per_page`, `Link` header. https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api [V]
  - GitLab offset mode: `page`, `per_page`; headers `X-Next-Page`, `X-Page`, `X-Per-Page`, `X-Prev-Page`, `X-Total`, `X-Total-Pages`; Link rels prev/next/first/last. https://docs.gitlab.com/api/rest/#pagination [V]
  - Zendesk offset: `page`, `per_page`, stop when `next_page` is null. https://developer.zendesk.com/api-reference/introduction/pagination/ [V]

### 1.2 Offset + limit

- Client asks: `offset` (or `startAt`, `$skip`, `from`, `start`) plus `limit` (or `maxResults`, `$top`, `size`); query string, or body for search endpoints.
- "More" signal: `total` count and arithmetic, an `isLast` boolean (Jira legacy: `startAt`, `maxResults`, `total`, `isLast`, items in `values`) [M], or a short/empty page.
- Items: usually nested (`values`, `items`, `hits`).
- Backward: yes (subtract).
- Page size: client-selectable, server max (Jira silently lowers `maxResults` [M]). Elasticsearch `from` + `size` cannot go past 10,000 hits (`index.max_result_window`) [V]. https://www.elastic.co/docs/reference/elasticsearch/rest-apis/paginate-search-results
- Caveats: same drift as page numbers; O(offset) cost server-side; `total` can be expensive or capped.
- Examples: Jira Cloud legacy `GET /rest/api/3/search` (`startAt`, `maxResults`, `total`) [M], https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/#pagination (page returned truncated, not verified); Elasticsearch `from`/`size` in the body [V]; Microsoft Graph `$top` + `$skip` where supported (`/users` supports `$top` but not `$skip`) [V].

### 1.3 Cursor token in the body, "next token" style (opaque)

- Client asks: pass the token from the previous response as a request parameter. Names: `cursor` (Slack), `pageToken` (Google), `start_cursor` (Notion), `NextToken` / `nextToken` / `Marker` / `ContinuationToken` (AWS), `nextPageToken` (Jira new search), `page_info` (Shopify), `page[after]` (Zendesk). Location: query for GET, JSON body for POST endpoints.
- "More" signal: next-token field present and non-empty. Absent / null / empty string at end. Sometimes paired with a boolean (`has_more` + `next_cursor` in Notion).
- Items: nested (`files`, `results`, `members`, `Contents`), occasionally root.
- Backward: usually none (forward-only); a few supply prev tokens.
- Page size: client-selectable via separate parameter (`pageSize`, `limit`, `page_size`, `MaxKeys`/`MaxResults`).
- Caveats: opaque; must not be parsed or constructed. Can expire (Slack: "use them within a reasonable time", `invalid_cursor` on expired/malformed [V]; Google Drive tokens "typically valid for several hours", discard a rejected token and restart [V]). A page can contain fewer than `limit` items, even zero, while more pages exist (Slack explicit [V]; Microsoft Graph "A page of results might contain zero or more results" [V]), so a short page is NOT an end signal. Token may need URL-encoding (Slack `=` to `%3D` [V]). Some tokens only valid for the same filters. Graph: retry tokens are not valid; use the token from the last successful non-retry response [V].
- Examples:
  - Slack: request `cursor`, `limit` (recommended 100-200, max 1000, varies per method); response `response_metadata.next_cursor`; end = empty string/null/absent. https://docs.slack.dev/apis/web-api/pagination [V]
  - Google Drive `files.list`: request `pageSize` (max 1000), `pageToken`; response `nextPageToken` (absent at end), items in `files`. https://developers.google.com/drive/api/reference/rest/v3/files/list [V]
  - Notion: request `start_cursor` + `page_size` (default and max 100; the intro prose says 10, docs conflict); response `has_more`, `next_cursor` (only present when `has_more`), `results`. GET puts params in the query, POST endpoints in the JSON body. https://developers.notion.com/reference/intro#pagination [V]
  - AWS: request `--starting-token`/`NextToken`/`Marker` (service-specific), response `NextToken` or `IsTruncated` + `NextMarker`/`NextContinuationToken`. https://docs.aws.amazon.com/cli/latest/userguide/cli-usage-pagination.html [V for the CLI-level `NextToken`; S3 default page size 1000 [V]; per-service field names such as `IsTruncated`/`NextContinuationToken` [M]].
  - Jira Cloud `GET /rest/api/3/search/jql`: `nextPageToken`, `maxResults`, `isLast`, `issues` [M, page not verified].
  - Airtable: `offset` token in response, passed back as `offset` [M].

### 1.4 ID cursor ("starting_after / ending_before", "after / before" object IDs)

- Client asks: pass an existing object's ID as the position: `starting_after`, `ending_before` (Stripe), `after`, `before` (OpenAI, [M]), `after_id`, `before_id` (Anthropic), `since_id`, `max_id` (Twitter v1, Shopify legacy [M]). Plus `limit`.
- "More" signal: `has_more` boolean in response; client derives the next cursor from the last item's ID, or the response supplies `first_id` / `last_id`.
- Items: nested under `data`.
- Backward: yes (`ending_before` / `before_id`). Stripe makes the two mutually exclusive.
- Page size: Stripe `limit` default 10, 1-100 [V]; Anthropic Models `limit` default 20, 1-1000 [V].
- Caveats: stable under inserts (no drift) when ordering is by an immutable key; cursor object deleted, behavior varies; ordering fixed by the server (Stripe reverse chronological [V]) so there is no free sort; `has_more` is the end signal, not page length.
- Examples:
  - Stripe v1: `limit`, `starting_after`, `ending_before`; response `object:"list"`, `data`, `has_more`, `url`. https://docs.stripe.com/api/pagination [V]. Stripe's auto-pagination helpers exist; v2 uses a different interface [V that it differs; details not fetched].
  - Anthropic `GET /v1/models`: request `after_id`, `before_id`, `limit`; response `data`, `has_more`, `first_id` (use as `before_id`), `last_id` (use as `after_id`). https://platform.claude.com/docs/en/api/models-list [V]
  - OpenAI list endpoints: `after`, `limit`, `order`; response `data`, `has_more`, `first_id`, `last_id` [M; reference page returned 403].
  - Stripe search endpoints use a separate `page` token + `next_page` field [M].

### 1.5 Link-following (server-provided next URL)

The client never builds the request; it follows a URL the server provided.

Variants by where the URL lives:

1. **HTTP `Link` header** (RFC 8288): `rel="next"`, `prev`, `first`, `last` (RFC 8288 defines the Link header and registered relations; RFC 5988 is the predecessor [M]). GitHub [V], GitLab keyset (only `rel="next"`, absent when exhausted) [V], Shopify (`rel=next` and `rel=previous`; no Link header when no further page) [V].
2. **URL in body**: Microsoft Graph `@odata.nextLink` (absolute URL; iterate until the property is no longer returned) [V]; Zendesk `links.next` with `meta.has_more` [V]; JSON:API top-level `links.first/last/prev/next`, each omitted or `null` when unavailable [V]; Twilio `meta.next_page_url` / `next_page_uri` [M; page 404]; Salesforce `nextRecordsUrl` + `done` + `totalSize` + `records` [M; page 403]; HAL `_links.next.href` [M]; RFC 5005 Atom paging feeds `<link rel="next">` [M].
3. **Custom headers**: GitLab `X-Next-Page`, `X-NEXT-CURSOR` / `X-PREV-CURSOR` [V]; others use `X-Total-Count` [M].

- Backward: usually `prev` link when supported.
- Page size: normally inherited from the first request; Shopify says a `page_info` request may include only `limit` and `fields`, other filters must be in the first request, `page` returns an error [V]; Shopify default `limit` 50, max 250 [V]; Graph's nextLink embeds all original query parameters [V].
- Caveats: the client must not parse or rebuild the URL (Graph: "Don't try to extract the `$skiptoken`..." [V]; GitLab: "don't build URLs yourself" [V]); links can be temporary (Shopify: do not store [V]); Graph does not carry custom (non-Authorization/Content-Type) request headers like `ConsistencyLevel` onto subsequent requests, so the client must re-send them [V]; `@odata.count` returned only on the first page for directory resources [V].
- **Security**: the next URL is server-controlled data. GitHub's page shows absolute URLs in Link and says to request them directly, with no validation guidance [V]. A malicious or compromised response could point to another host and cause credential leakage (bearer token sent to attacker) or SSRF. A client should treat the link as untrusted: require same origin (scheme + host + port) as the configured base URL (or an explicit allow-list), reject downgrade to `http`, and never forward auth headers cross-origin. This is an engineering recommendation, not something the cited docs prescribe.

### 1.6 Keyset / seek pagination (sort-value cursor)

- Client asks: pass the sort-key values of the last row seen. Explicit forms: Elasticsearch `search_after: [sortvalues...]` in the request body [V]; GitLab keyset `pagination=keyset&order_by=...&sort=...` then follow Link, whose next URL carries `id_after` or `cursor` [V]; SQL-style `?created_before=...&id_before=...` [M].
- "More" signal: Elasticsearch has no explicit flag: empty `hits` ends iteration [V for "until hits is empty" for scroll; for search_after treated by convention, [M]]; GitLab: `rel="next"` absent [V].
- Backward: reverse the sort (Elasticsearch) or `X-PREV-CURSOR` (GitLab) [V].
- Page size: `size` (ES), `per_page` (GitLab keyset default 20, max 100 [V]).
- Caveats: requires a deterministic total order with a unique tiebreaker; ES: without a tiebreaker pages can skip or repeat hits [V]. Sort-order dependent (cursor only valid for same `query` and `sort` [V]). Only selected endpoints and `order_by`/`sort` combinations support it on GitLab [V]. Needs a snapshot to be consistent: ES Point-in-Time (`pit.id` + `keep_alive`; `from` must be 0 or -1 with PIT; delete PIT when done) [V].
- Performance: runtime independent of collection size (GitLab) [V].

### 1.7 Server-side snapshot cursors (scroll / PIT / queryMore)

- Elasticsearch scroll: first request includes `scroll=1m`, returns `_scroll_id`; subsequent requests pass the latest `_scroll_id` to the scroll API; ends when `hits` is empty; results reflect a snapshot; open contexts cost resources (default cap 500); explicit clear-scroll call needed; docs now recommend `search_after` + PIT for pagination [V]. Sliced scroll/PIT (`slice.id`, `slice.max`) parallelizes [V].
- Salesforce `queryMore` via `nextRecordsUrl` is similar (server cursor, `done:true` at end) [M].
- Key point: there is a cleanup/lifetime obligation (keep-alive renewal, delete cursor), and a changing ID per page (always use the latest).

### 1.8 GraphQL Relay connections

- Client asks (arguments in the query/body, usually POST): forward `first: N, after: "<cursor>"`; backward `last: N, before: "<cursor>"`.
- Response: `edges[] { cursor, node }`, `pageInfo { hasNextPage, hasPreviousPage, startCursor, endCursor }`, optional `totalCount`. Cursors opaque (base64 recommended). https://graphql.org/learn/pagination/ [V for first/after/edges/node/cursor/startCursor/endCursor/hasNextPage/totalCount; `last`/`before`/`hasPreviousPage` stated in the Relay spec, not on that page].
- End signal: `hasNextPage == false`. Items are two levels deep (`edges[].node`), or `nodes` shortcut on some servers [M].
- Single endpoint, so "pagination" is part of the document, not the URL. For an HTTP-level definition library this is a body-level cursor with a nested response path.

### 1.9 Time-window / date-range paging

- Client asks: `since`, `until`, `oldest`, `latest`, `created[gte]`, `start_time`, `from`/`to`, `before` / `after` as timestamps; advance the window using the last timestamp seen.
- Examples: Slack deprecated timeline paging `oldest`, `latest`, `inclusive` on `*.history` [V]; GitHub `since` (user/issue IDs or timestamps; "Some endpoints use `since` for non-pagination purposes") [V]; Zendesk incremental exports (`start_time`, `end_of_stream`) [M].
- Caveats: ties at the boundary (identical timestamps need an inclusive/exclusive rule or a secondary key), clock skew, items with updated timestamps reappearing. Sort-order dependent.

### 1.10 Single-shot / no paging

Some "list" endpoints return everything (Google Drive non-shared drive default returns the full list [V]; Graph says not all resources support paging, e.g. directoryRole [V]). Model as "None".

## 2. Cross-cutting features

| Feature | Notes |
| --- | --- |
| Client-set page size | Param names: `per_page`, `limit`, `page_size`, `pageSize`, `PageSize`, `$top`, `maxResults`, `size`, `page[size]`, `first`/`last`, `count`. Servers differ on over-max: silently cap (GitHub [V], Drive [V]), ignore, or error (Graph: "might be ignored, default to max, or return an error" [V]). Page size can be absent/fixed (Notion has a cap of 100 [V]). Some APIs let you change it mid-iteration via the next link (Shopify `limit` in link [V]); others bind it to the token (Graph nextLink keeps `$top` [V]). |
| Default / max | Per endpoint; examples: Stripe 10/100 [V], GitLab 20/100 [V], Shopify 50/250 [V], Anthropic Models 20/1000 [V], Slack rec. 100-200, max 1000 [V], Drive 100 (shared) or full list, max 1000 [V], Notion 100/100 [V], Graph `/users` default 100 [V], S3 1000 [V]. |
| Total count | Sources: header (`X-Total`, `X-Total-Pages`, `X-Total-Count` [M]), body (`total`, `totalSize`, `totalCount`, `@odata.count`, Stripe `total_count` on search only [M]). Often optional/expensive: Graph needs `$count=true` and returns it only on the first page [V]; GitLab omits it above 10,000 records [V]; ES `track_total_hits` [V]. Never guaranteed. |
| Sort-order dependence | Keyset needs explicit `order_by`+`sort` (GitLab [V]) or sort array with tiebreaker (ES [V]); Stripe fixed reverse-chronological [V]; Shopify: range searches time out if search field differs from sort order [V]; Anthropic Models: newest first [V]. |
| POST-body paginated search | Cursor and size live in the JSON body: Notion `POST` query endpoints take `start_cursor`/`page_size` in body [V]; Elasticsearch `search_after`, `pit`, `size`, `from` in body [V]; GraphQL args in the body. A model must allow the request-side paging inputs to target query, header, or body. Also POST with a path variable token (AWS JSON-protocol services place `NextToken` in the body [M]). |
| Untrusted absolute next links | See 1.5. Also Link-header values are attacker-influenced if the API proxies user content. |
| Opaque vs derivable cursors | Opaque (Slack, Notion [V says treat as opaque], Graph). Derivable from last item (Stripe ID, ES sort values, Anthropic `last_id`). A client library should know whether it can compute the next cursor from the last item. |
| Cursor lifetime | Expiring (Slack, Drive hours, ES PIT keep-alive, scroll keep-alive) [V]; needs restart-on-invalid behavior. |
| Retry safety | Graph: reuse the last good token, not a retry's token [V]. |
| Header carry-over | Graph does not forward custom request headers on page 2+ [V]. |
| Parallelism | Sliced scroll/PIT (ES) [V]; Drive/others: none. Most cursor schemes are inherently serial. |
| Backward navigation | Stripe `ending_before` [V], Anthropic `before_id` [V], GitHub/GitLab/Shopify `prev` [V], Relay `last`/`before`, JSON:API `prev`; many token styles (Slack, Drive, Notion, Graph) are forward-only [V by absence of mention; not explicitly stated forward-only]. |

## 3. Guidelines and standards consulted

- RFC 8288 (Web Linking, Link header): https://www.rfc-editor.org/rfc/rfc8288 [M, not fetched].
- RFC 5005 (Atom feed paging, `first`/`last`/`previous`/`next` link relations): [M, not fetched].
- JSON:API: strategy-agnostic; defines `links.first/last/prev/next` (each omitted or `null` if unavailable) and reserves `page[...]` query family, examples `page[number]`/`page[size]`/`page[cursor]`. https://jsonapi.org/format/#fetching-pagination [V]
- HAL `_links.next`: [M].
- OpenAPI 3.1: has no first-class pagination construct; tools use `x-` extensions (e.g. `x-ms-pageable` with `nextLinkName`, Redocly `x-pagination`/Speakeasy `x-speakeasy-pagination` with `type: offsetLimit | cursor | url`) [M, not verified; check before citing].
- Zalando RESTful guidelines (prefer cursor-based, opaque cursor, `cursor`/`limit` params, `items` array, `self`/`first`/`prev`/`next`/`last` links), Google AIP-158 (`page_size`, `page_token`, `next_page_token`; empty `next_page_token` = end; optional `total_size`), Microsoft REST guidelines (`$top`, `$skip`, `@nextLink`): [M, none fetched this session; links to verify: https://opensource.zalando.com/restful-api-guidelines/#pagination, https://google.aip.dev/158, https://github.com/microsoft/api-guidelines].
- Offset vs keyset vs cursor discourse (Use The Index, Luke "We need tool support for keyset pagination"; Slack engineering "Evolving API pagination at Slack"): [M, not fetched].

## 4. Could not verify

OpenAI list reference (403), Google Drive pagination guide (404; reference page worked), Jira intro and `search/jql` pages (truncated), Twilio pagination page (404), Salesforce query page (403), Hugging Face, ElevenLabs, Ollama (not attempted: Ollama list endpoints are, to my knowledge, un-paginated [M]), HubSpot (`paging.next.after` + `after`/`limit` [M]), Bitbucket (`next` URL in body plus `pagelen`, `page`, `size` [M]), Gitea (`page`/`limit` with `Link` and `X-Total-Count` [M]), Elasticsearch `search_after` response end-signal (inferred), AWS per-service field names beyond the CLI page.

---

## 5. Synthesis

### 5.1 Minimal set of distinct strategy KINDS

1. **None** (single response).
2. **Page number** (`page` + size).
3. **Offset** (`offset`/`startAt`/`from` + size).
4. **Token cursor** (opaque token returned by server, echoed back; forward-only).
5. **Item-ID cursor** (`after`/`before` object ID; bidirectional; client may derive from last item or from `first_id`/`last_id`).
6. **Sort-key / keyset cursor** (array of sort values from last hit, e.g. ES `search_after`; client derives, needs sort+tiebreaker).
7. **Link-following** (server gives next request as URL; Link header, body field, or custom header). Can be treated as a *continuation signal* orthogonal to kinds 2-6 (GitHub is page-number with link following; GitLab keyset is keyset with link following).
8. **Server-side snapshot / stateful cursor** (scroll, PIT, queryMore), which adds handle lifetime and cleanup.
9. **Time window** (`since`/`until` advance by last timestamp).
10. **Relay connection** (cursor kind with nested `edges[].node` and `pageInfo`); reducible to token cursor plus item path plus `hasNextPage` signal.

Kinds 2-6 and 9 differ in the *request* inputs; 7 and 10 differ mostly in the *response* shape. A good model separates these axes rather than making each combination a variant.

### 5.2 Orthogonal dimensions

- **Request-side paging inputs**: a list of (name, role, location) triples. Roles: `page`, `offset`, `limit/page_size`, `cursor/token` (forward), `before-cursor` (backward), `since/until`, `sort key`, `snapshot handle`. Locations: query, header, body (JSON pointer path), path template.
- **Continuation signal (response side)**: one or more of: next-token field (JSON path; absent/null/empty = end); next-URL field (JSON path; absolute or relative); `Link` header rel (`next`); custom header (`X-Next-Page`, `X-NEXT-CURSOR`); `has_more` boolean (JSON path, paired with a cursor source); `isLast`/`done`/`hasNextPage` boolean (inverted polarity variants); total count with arithmetic (`offset + len < total`, `page < total_pages`); item-derived cursor (`last_id`, last item's field, last item's sort values).
- **End-of-data detection**: explicit boolean (either polarity), cursor field absent/null/empty, link absent, empty page, short page (fewer than limit; NOT reliable for Slack/Graph/Drive), total reached, status 204/404 at end (not verified for any real API in this spike), or `end_of_stream` (Zendesk incremental [M]).
- **Item location**: root array, or JSON path to array (`data`, `items`, `results`, `values`, `files`, `records`, `hits.hits`, `edges[*].node`), plus how to find optional metadata (total, cursors) relative to the root.
- **Page-size rules**: parameter name+location, default, min, max, behavior on over-max (cap/ignore/error), and whether it can change mid-iteration (frozen in token vs allowed).
- **Direction**: forward-only vs bidirectional; the backward cursor source (`first_id`, `startCursor`, `rel=prev`).
- **Cursor semantics**: opaque vs derivable; lifetime/expiry; requires same filters/sort; needs cleanup call.
- **Ordering requirement**: fixed by server, client-chosen with tiebreaker, or irrelevant.
- **Total count**: absent, always present, opt-in (`$count=true`, `track_total_hits`), first page only, capped.
- **Link trust policy**: same-origin required, allow-list, auth header forwarding policy, header carry-over for subsequent pages.

## 6. Patterns awkward for a simple page+per_page / offset+limit / cursor trio

Current model: three request styles and three response signals (`BodyField{next_field}`, `LinkHeader`, `TotalCount{total_field,page_field}`).

1. **Next URL in the body** (`@odata.nextLink`, Zendesk `links.next`, JSON:API `links.next`, Twilio `next_page_uri`, Salesforce `nextRecordsUrl`): `BodyField{next_field}` conflates "next cursor value to echo as a parameter" with "next URL to request wholesale". These need different client behavior (no request construction; same-origin check; param/headers carry-over).
2. **Cursor in the request body** (Notion POST, ES `search_after`, GraphQL variables, AWS JSON protocols): the request side must support body location with a JSON path, not only query.
3. **Cursor and `has_more` in separate fields** (Notion `has_more` + `next_cursor`, Stripe `has_more` + last item ID, Anthropic `has_more` + `last_id`): the end signal is a boolean while the next cursor is elsewhere or must be *derived from the last item* (`starting_after = data[-1].id`). `BodyField{next_field}` cannot express "derive from last item" or "stop on `has_more == false`".
4. **Boolean with inverted polarity** (`isLast`, `done`, `hasNextPage`, `has_more`, `IsTruncated`): needs polarity.
5. **Nested cursor path** (Slack `response_metadata.next_cursor`; Relay `data.x.pageInfo.endCursor`): needs a JSON path, not a top-level field name.
6. **Bidirectional ID cursors** (`after_id`/`before_id`, `starting_after`/`ending_before`, Relay `last`/`before`): trio has a single cursor parameter.
7. **Sort-value array cursors** (ES `search_after`, composite keyset): the cursor is a structured array copied from the last item's `sort`, not a scalar.
8. **Empty-page / short-page end detection** (ES scroll `hits` empty; legacy APIs): none of the three response signals covers "stop when the items array is empty". Also, short page must NOT be used as the end signal for Slack/Graph/Drive.
9. **Stateful snapshots** (ES PIT + keep_alive, scroll): a handle must be opened first, updated each page (ES returns a new PIT id), and closed. No place for open/close steps.
10. **Link-header with only `next`** (GitLab keyset, Shopify): fine for `LinkHeader`, but the Link `rel` name varies (`next` vs `previous`), and the model must also handle "no header at all = end" and the rule that follow-up requests may carry only some parameters (Shopify).
11. **Total count variants**: `TotalCount{total_field, page_field}` assumes page-number arithmetic. Offset+total (Jira `startAt` + `total`), total in a header (`X-Total`, `X-Total-Pages`), `total_pages` rather than `total`, optional/first-page-only totals, and counts that disappear above a threshold (GitLab 10,000) don't fit.
12. **Time-window paging**: no page or cursor at all; the next request's input is derived from the last item's timestamp (and needs a tiebreaker).
13. **Mixed strategies on one API** (Zendesk offset vs cursor chosen by presence of `page[size]`; GitLab offset vs keyset chosen by `pagination=keyset`; Stripe v1 vs v2; Slack's mix of cursor, page, and timeline): strategy is per endpoint, not per API; needs endpoint-level override and a mode-selecting constant parameter.
14. **Constant parameters required to enable a strategy** (`pagination=keyset`, `order_by`, `sort`, ES `pit`): fixed query params that are part of the paging contract.
15. **Parameters forbidden on follow-up requests** (Shopify: only `limit` and `fields` allowed with `page_info`, `page` errors): "first request vs subsequent request" parameter sets differ.
16. **Page size not client-selectable, or capped silently** (Notion max 100, GitHub cap): model should record default/max and the over-max behavior.
17. **Items at a path with per-item wrapping** (`edges[].node`, ES `hits.hits[]._source`): needs item path with projection.
18. **Auth and header carry-over across pages** (Graph `ConsistencyLevel`; cross-origin next URLs): a security/behavior policy attached to the strategy.
