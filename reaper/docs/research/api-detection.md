---
prompt: |-
    It is not uncommon for a webpage to have an API that the Javascript on the page calls to 
    dynamically get data. To detect this behavior we will need to leverage a crate like 
    [chromiumoxide](https://crates.io/crates/chromiumoxide) which creates a headless Chrome browser and exposes the full [DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/) to Reaper.

    Your task is to:

    1. identify how to use chromiumoxide to detect API calls from within the browser
    2. layout a reasonable schema for capturing network requests in the browser
    3. describe 2-3 detection strategies which might be used to:
        - detect API calls
        - categorize these requests
        - if possible, provide a "loose schema" for the API interaction
last_updated: 2026-06-03
---
## Browser Network API Detection

Use `chromiumoxide` to run Reaper’s page fetch inside headless Chromium and subscribe to Chrome DevTools Protocol network events. The main CDP domain for passive capture is [`Network`](https://chromedevtools.github.io/devtools-protocol/tot/Network/); use [`Fetch`](https://chromedevtools.github.io/devtools-protocol/tot/Fetch/) only when Reaper needs interception or response-stage body capture.

The basic flow is:

1. Launch Chromium with `chromiumoxide`.
2. Open a page target.
3. Enable CDP network tracking with `Network.enable`.
4. Register page event listeners for request, response, failure, and completion events.
5. Navigate to the page and wait until the page becomes quiet enough to classify observed traffic.
6. Join request/response events by CDP `requestId`.
7. Optionally call `Network.getResponseBody` after `loadingFinished` for likely API responses.

Illustrative Rust shape:

```rust
use chromiumoxide::Browser;
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams,
    EventLoadingFailed,
    EventLoadingFinished,
    EventRequestWillBeSent,
    EventResponseReceived,
    GetResponseBodyParams,
};

async fn capture_network(url: &str) -> anyhow::Result<()> {
    let (browser, mut handler) = Browser::launch(Default::default()).await?;

    tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    let page = browser.new_page(url).await?;

    page.execute(EnableParams::default()).await?;

    let mut requests = page.event_listener::<EventRequestWillBeSent>().await?;
    let mut responses = page.event_listener::<EventResponseReceived>().await?;
    let mut finished = page.event_listener::<EventLoadingFinished>().await?;
    let mut failed = page.event_listener::<EventLoadingFailed>().await?;

    page.goto(url).await?;

    // Reaper would drain these streams until a timeout or network-idle window.
    // Each event is normalized into BrowserNetworkRequest records.

    Ok(())
}
```

For passive detection, `Network.requestWillBeSent` records the URL, method, headers, initiator, frame, resource type, and optional redirect information. `Network.responseReceived` adds status, MIME type, protocol, timing, and response headers. `Network.loadingFinished` marks the request complete and is the safest point to request the response body with `Network.getResponseBody`.

For active interception, enable `Fetch.enable` with request patterns. `Fetch.requestPaused` can pause at request or response stage, and the event includes a `networkId` when it corresponds to a `Network.requestWillBeSent` event. This is useful when Reaper needs response bodies before the browser consumes them, but it is heavier because every paused request must be continued, failed, or fulfilled.

## Capture Schema

A reasonable Reaper-side schema should separate the raw browser observation from higher-level API classification. The raw schema should be stable even if classification improves later.

```rust
pub struct BrowserNetworkRequest {
    pub id: BrowserRequestId,
    pub page_url: String,
    pub frame_id: Option<String>,
    pub loader_id: Option<String>,

    pub url: String,
    pub method: String,
    pub resource_type: BrowserResourceType,
    pub initiator: BrowserRequestInitiator,

    pub request_headers: Vec<HttpHeader>,
    pub request_body: Option<CapturedBody>,

    pub response: Option<BrowserNetworkResponse>,
    pub redirect_from: Option<BrowserRequestId>,
    pub redirect_to: Option<BrowserRequestId>,

    pub started_at_ms: f64,
    pub finished_at_ms: Option<f64>,
    pub duration_ms: Option<f64>,

    pub transfer_size_bytes: Option<u64>,
    pub encoded_body_size_bytes: Option<u64>,
    pub decoded_body_size_bytes: Option<u64>,

    pub failure: Option<BrowserNetworkFailure>,
    pub cache_status: BrowserCacheStatus,

    pub classification: BrowserRequestClassification,
}

pub struct BrowserNetworkResponse {
    pub status: u16,
    pub status_text: Option<String>,
    pub mime_type: Option<String>,
    pub protocol: Option<String>,
    pub remote_ip_address: Option<String>,
    pub remote_port: Option<u16>,
    pub response_headers: Vec<HttpHeader>,
    pub body: Option<CapturedBody>,
}

pub struct CapturedBody {
    pub content_type: Option<String>,
    pub encoding: CapturedBodyEncoding,
    pub byte_len: usize,
    pub truncated: bool,
    pub text_preview: Option<String>,
    pub json_shape: Option<JsonShape>,
}

pub struct BrowserRequestClassification {
    pub is_probable_api: bool,
    pub confidence: f32,
    pub categories: Vec<ApiRequestCategory>,
    pub evidence: Vec<String>,
    pub loose_schema: Option<ApiInteractionSchema>,
}
```

Suggested enums:

```rust
pub enum BrowserResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    Xhr,
    Fetch,
    EventSource,
    WebSocket,
    Manifest,
    Other,
}

pub enum ApiRequestCategory {
    JsonRest,
    GraphQl,
    Rpc,
    Search,
    Pagination,
    Autocomplete,
    Authentication,
    Analytics,
    AdsOrTracking,
    StaticAsset,
    FirstPartyData,
    ThirdPartyData,
    Unknown,
}

pub struct ApiInteractionSchema {
    pub endpoint_template: String,
    pub method: String,
    pub request_content_type: Option<String>,
    pub response_content_type: Option<String>,
    pub path_params: Vec<String>,
    pub query_params: Vec<ApiFieldShape>,
    pub request_body_shape: Option<JsonShape>,
    pub response_body_shape: Option<JsonShape>,
}

pub struct ApiFieldShape {
    pub name: String,
    pub observed_types: Vec<String>,
    pub optional: bool,
    pub examples: Vec<String>,
}
```

The `CapturedBody` should be bounded. Reaper should avoid storing unlimited bodies from the browser. Store small text bodies directly, store previews for larger bodies, and keep only inferred shape metadata for very large or binary responses.

## Detection Strategies

### 1. CDP Resource-Type and MIME Detection

The simplest signal is CDP’s own classification. Requests with resource type `XHR`, `Fetch`, `EventSource`, or `WebSocket` are strong API candidates. Responses with MIME types such as `application/json`, `application/graphql-response+json`, `application/x-ndjson`, or `text/event-stream` should increase confidence.

Useful evidence:

- CDP resource type is `XHR` or `Fetch`.
- Request has `accept: application/json` or similar.
- Response has a JSON, NDJSON, GraphQL, or event-stream MIME type.
- URL path contains common API markers such as `/api/`, `/graphql`, `/rpc`, `/v1/`, `/v2/`, `/query`, or `/search`.
- Method is not only `GET`, especially `POST`, `PUT`, `PATCH`, or `DELETE`.

This strategy is low-risk and should be the default first pass.

### 2. Body Shape Detection

When Reaper captures response bodies, parse bounded text bodies as structured data. JSON arrays and objects are strong API signals, especially when the same endpoint template returns repeated records.

Loose schema inference can include:

- Top-level JSON type: object, array, scalar, or null.
- Object keys and observed value types.
- Repeated array item shape.
- Pagination markers such as `next`, `cursor`, `offset`, `limit`, `page`, `total`, or `has_more`.
- Error envelope markers such as `error`, `errors`, `message`, `code`, or `status`.
- GraphQL markers such as request body `query`, `operationName`, `variables`, and response body `data` or `errors`.

Example loose schema:

```json
{
  "endpoint_template": "https://example.com/api/search?q={q}&page={page}",
  "method": "GET",
  "response_content_type": "application/json",
  "query_params": [
    { "name": "q", "observed_types": ["string"], "optional": false },
    { "name": "page", "observed_types": ["integer"], "optional": true }
  ],
  "response_body_shape": {
    "type": "object",
    "fields": {
      "results": {
        "type": "array",
        "items": {
          "type": "object",
          "fields": {
            "id": { "type": "string" },
            "title": { "type": "string" },
            "url": { "type": "string" }
          }
        }
      },
      "next": { "type": ["string", "null"] }
    }
  }
}
```

This should be described as a loose schema because browser observation only sees examples, not the API contract.

### 3. Behavioral and Initiator Detection

Some API calls are not obvious from URL or MIME type alone. Reaper can classify requests by when and why they occur.

Signals:

- Request starts after initial document load rather than during static page bootstrap.
- Request initiator stack points to page JavaScript rather than the parser.
- Request is triggered by scroll, click, form input, route transition, or timer.
- Repeated requests hit the same path with changing query parameters or JSON variables.
- Response content later appears in the DOM.
- Request is first-party and returns structured data used by the page.

This strategy helps separate real data APIs from static assets, analytics beacons, ads, and tracking pixels. It also helps identify pagination and search APIs by observing parameter changes across multiple interactions.

A practical confidence model:

```text
+0.35 resource type is XHR, Fetch, EventSource, or WebSocket
+0.25 response content type is JSON, NDJSON, GraphQL JSON, or event-stream
+0.15 URL path looks API-like
+0.15 response body parses as structured data
+0.10 request happens after initial page load
+0.10 repeated endpoint template with changing parameters
-0.25 known analytics, ads, telemetry, or tag-manager domain
-0.20 image, font, stylesheet, script, document, or manifest resource type
```

Requests above a threshold can be marked `is_probable_api = true`; lower-confidence requests can still be retained as `Unknown` with evidence attached.

## Recommended Approach

Start with passive `Network.enable` capture because it avoids changing page behavior. Normalize `requestWillBeSent`, `responseReceived`, `loadingFinished`, and `loadingFailed` into `BrowserNetworkRequest`. Add bounded `Network.getResponseBody` capture only for likely API candidates.

Use `Fetch.enable` later for cases where passive body capture is insufficient, request bodies are required, or Reaper needs response-stage interception. Because `Fetch.requestPaused` blocks matching requests until Reaper continues them, it should be opt-in or limited to likely API resource types and URL patterns.
