---
sequence: 
    - name: "start"
    - name: "api-detection"
    - name: "auth-scraping"
    - name: "change-detection"
    - name: "library-detection"
    - name: "page-categories"
    - name: "main-content"
    - name: "metadata"
    - name: "page-categories"
    - name: "variants"
    - name: "snapshots"
    - name: "tantivy"
    - name: "variants"
    - name: "closure"
prompt: |-
    The body of this Markdown document is meant to be a draft model specification for the following entities:

    1. WebPage - a Rust enumeration with both `Simple` and `Complete` variants which both describe all the metadata characteristics of a web page in varying levels of detail.
    2. WebSite - a more macro set of metadata that describes macro metadata about the site as well as reference pages which we have information about. For example, this struct should provide a place to store information about the website's ownership, provide glob patterns to identify different page types on the site, etc.
    3. Company - information about a company
    4. Person - information about a person
    5. Product - information about a product
    6. Place - information about a place/location
    
    ::block when="state.name == 'start'"
    ## Task

    No prior research has been done but the Markdown document has been seeded with the **structure** of the document we are trying to create (it's possible that if this prompt is being re-run ... that there WILL be content and in that case just review the content and make any adjustments you feel are needed). Your task is to do broad based research on each of the entities described above and come up with an initial set of Rust structs and enums that shape these entities.

    - each entity has a H2 heading designated for it
    - inside each H2 heading are two sections:
        - `### Schema`
        - `### Notes`
    - you are to keep the structure as is but fill in each H3 level section:
        - the schema section is primarily a Rust code block which offers structs and enums that help to define the section's entity
        - the notes section is for adding a unordered list of bullet points which provide context to the schema, elucidate rational for design decisions, bring up open questions which need more research, etc.
    - DO NOT read the various research documents in @reaper/docs/research as they'll be incorporated in another task

    Because this is the first (of several) passes, these sections are probably empty but you must add a first draft to each H2/H3 section.

    > **NOTE:** the structs/enums should all have at least a minimal doc comment; individual properties and variants should also have a short description in most cases.
    
    ::end-block
    
    ::block when="!contains('start, closure', state.name)"
    ## Task

    A draft of the various entities described above should already exist in this document. Your task is to review the document "./{{state.name}}.md" (which contains research information about a related topic to web page scraping and the Reaper project). Based on the knowledge in the research document, review the existing content in the body of this Markdown document and update the schema where you see fit as well as add additional context to the notes sections.

    Once you've reviewed and updated each section in the document, and SAVED your updates, the task is complete.
    ::end-block
    
    ::block when="state.name == 'closure'"
    We have gone through several iterations of defining schemas for the entities above. We have now reached the final step and your task is to read through each section:

    - look for errors, redundancies, or overlapping ideas and fix inline in the body of this document
    - make sure the doc comments appropriately describe the symbols and the fields in the structs, variants in the enums
    - when you have completed all sections of the document add a new H2 heading at the end: `## Next Steps for Closure
        - in this section describe how complete you believe these different entities are
        - mention any other entities you feel are missing from the current design
            - missing for the **Reaper** project which is a comprehensive screen scraping project
        - describe any further steps you believe would be helpful in finalizing the schema's of the core Reaper entities.
    ::end-block
last_updated: 2026-06-03
---
Let me first find the existing Markdown document and understand its structure.Now let me look at the existing repo code for any existing types or conventions:Now let me look at some key research docs to inform the schema design, plus the README for context:Now I have a thorough understanding of the project context. Let me do research on the key schema standards to inform the design:I now have enough context to produce the first draft. Let me compose the full Markdown body:

## WebPage

### Schema

```rust
use url::Url;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Broad category describing the kind of content a page contains.
enum PageType {
    /// A news article, blog post, or editorial piece.
    Article,
    /// A product listing or product detail page.
    Product,
    /// A landing page or marketing page.
    Landing,
    /// Documentation or reference material.
    Documentation,
    /// A forum thread, discussion, or Q&A page.
    Forum,
    /// A user or organization profile page.
    Profile,
    /// A recipe with ingredients and instructions.
    Recipe,
    /// An event listing or detail page.
    Event,
    /// A job posting.
    JobPosting,
    /// A search results or listing page.
    SearchResults,
    /// A category, tag, or taxonomy archive page.
    Category,
    /// A media-focused page (video, audio, gallery).
    Media,
    /// The site's homepage.
    Homepage,
    /// Any page type not covered above.
    Other(String),
}

/// How the page content was retrieved.
enum FetchMethod {
    /// Raw HTTP GET without JavaScript execution.
    HttpFetch,
    /// Full browser rendering with JavaScript execution.
    BrowserRendered,
}

/// Whether the page requires authentication to access.
enum AccessLevel {
    /// Publicly accessible without credentials.
    Public,
    /// Requires a free account or registration.
    RegistrationRequired,
    /// Requires a paid subscription or specific tier.
    Paywalled,
    /// Access level could not be determined.
    Unknown,
}

/// Open Graph protocol metadata extracted from `og:*` meta tags.
struct OpenGraphMeta {
    /// The title of the object as it should appear in the graph (og:title).
    title: Option<String>,
    /// The type of the object, e.g. "article", "website" (og:type).
    og_type: Option<String>,
    /// The canonical URL of the object (og:url).
    url: Option<Url>,
    /// The URL of the representative image (og:image).
    image: Option<Url>,
    /// A brief description of the object (og:description).
    description: Option<String>,
    /// The name of the website hosting the page (og:site_name).
    site_name: Option<String>,
    /// The locale of the content, e.g. "en_US" (og:locale).
    locale: Option<String>,
    /// Additional OG properties not mapped to typed fields.
    extra: HashMap<String, String>,
}

/// Twitter/X Card metadata extracted from `twitter:*` meta tags.
struct TwitterCardMeta {
    /// The card type: summary, summary_large_image, app, player.
    card_type: Option<String>,
    /// The Twitter handle of the site (@handle).
    site: Option<String>,
    /// Title for the card.
    title: Option<String>,
    /// Description for the card.
    description: Option<String>,
    /// Image URL for the card.
    image: Option<Url>,
}

/// Structured data extracted from Schema.org JSON-LD, Microdata, or RDFa.
struct SchemaOrgData {
    /// The JSON-LD or Microdata content as a loosely typed value.
    raw: serde_json::Value,
    /// The primary @type(s) declared, e.g. "Article", "Product".
    types: Vec<String>,
}

/// Dublin Core metadata extracted from DC.* or dcterms.* meta tags.
struct DublinCoreMeta {
    /// DC.title — the resource title.
    title: Option<String>,
    /// DC.creator — the author or creator.
    creator: Option<String>,
    /// DC.subject — subject keywords or phrases.
    subject: Option<String>,
    /// DC.description — a textual description.
    description: Option<String>,
    /// DC.publisher — the entity responsible for publishing.
    publisher: Option<String>,
    /// DC.date — date associated with the resource.
    date: Option<String>,
    /// DC.type — the nature or genre of the resource.
    resource_type: Option<String>,
    /// DC.format — the MIME type or physical medium.
    format: Option<String>,
    /// DC.identifier — an unambiguous reference (DOI, ISBN, URL).
    identifier: Option<String>,
    /// DC.language — the language of the resource.
    language: Option<String>,
    /// DC.rights — rights information or license.
    rights: Option<String>,
    /// Additional DC properties not mapped to typed fields.
    extra: HashMap<String, String>,
}

/// Standard HTML `<meta>` tag metadata.
struct HtmlMeta {
    /// The page title from `<title>`.
    title: Option<String>,
    /// The meta description.
    description: Option<String>,
    /// Meta keywords (comma-separated).
    keywords: Option<String>,
    /// The declared author.
    author: Option<String>,
    /// The robots directive (e.g. "index, follow").
    robots: Option<String>,
    /// The canonical URL from `<link rel="canonical">`.
    canonical_url: Option<Url>,
    /// The generator tag (e.g. "WordPress 6.4").
    generator: Option<String>,
    /// The viewport meta content.
    viewport: Option<String>,
    /// The charset declaration.
    charset: Option<String>,
    /// The theme-color meta value.
    theme_color: Option<String>,
    /// All remaining `<meta name="..." content="...">` pairs.
    extra: HashMap<String, String>,
}

/// A breadcrumb item representing one level of the page's navigation hierarchy.
struct Breadcrumb {
    /// The display label for this breadcrumb.
    label: String,
    /// The URL this breadcrumb links to.
    url: Option<Url>,
    /// Position in the breadcrumb trail (1-based).
    position: u32,
}

/// Links discovered in the page's `<head>` section.
struct HeadLinks {
    /// RSS or Atom feed URLs.
    feed_urls: Vec<Url>,
    /// The Web App Manifest URL.
    manifest_url: Option<Url>,
    /// oEmbed discovery endpoint URL.
    oembed_url: Option<Url>,
    /// Alternate language/region versions of this page.
    alternates: Vec<(String, Url)>,
    /// Favicon or touch-icon URLs.
    icon_urls: Vec<Url>,
    /// Stylesheet URLs.
    stylesheets: Vec<Url>,
    /// Preload, prefetch, or preconnect hints.
    preload_hints: Vec<(String, Url)>,
}

/// A simplified representation of an image found on the page.
struct PageImage {
    /// The source URL of the image.
    src: Url,
    /// Alt text if available.
    alt: Option<String>,
    /// Declared width in pixels.
    width: Option<u32>,
    /// Declared height in pixels.
    height: Option<u32>,
}

/// Metadata about the HTTP response itself.
struct ResponseMeta {
    /// The final URL after any redirects.
    final_url: Url,
    /// The HTTP status code received.
    status_code: u16,
    /// The content-type header value.
    content_type: Option<String>,
    /// The content-length in bytes, if known.
    content_length: Option<u64>,
    /// Response headers that may be relevant for scraping.
    headers: HashMap<String, String>,
    /// How the page was fetched.
    fetch_method: FetchMethod,
    /// Timestamp when the page was scraped.
    scraped_at: DateTime<Utc>,
    /// Wall-clock time for the fetch/render in milliseconds.
    response_time_ms: Option<u64>,
}

/// Lightweight page metadata obtainable without full rendering.
///
/// Contains all information that can be extracted from a raw HTTP response
/// (HTML meta tags, structured data, response headers) without executing
/// JavaScript or performing expensive content analysis.
struct SimpleWebPage {
    /// Response-level metadata including the final URL and fetch details.
    response: ResponseMeta,
    /// Standard HTML `<meta>` tags.
    html_meta: HtmlMeta,
    /// Open Graph protocol metadata, if present.
    open_graph: Option<OpenGraphMeta>,
    /// Twitter Card metadata, if present.
    twitter_card: Option<TwitterCardMeta>,
    /// Schema.org structured data (JSON-LD / Microdata / RDFa).
    schema_org: Vec<SchemaOrgData>,
    /// Dublin Core metadata, typically on academic/institutional pages.
    dublin_core: Option<DublinCoreMeta>,
    /// Breadcrumb navigation trail, if detectable from structured data.
    breadcrumbs: Vec<Breadcrumb>,
    /// Links discovered in the `<head>` element.
    head_links: HeadLinks,
    /// The declared or inferred primary language (BCP 47 tag).
    language: Option<String>,
    /// The page type inferred from metadata signals.
    page_type: Option<PageType>,
}

/// A fully analyzed web page with extracted content and deeper signals.
///
/// Extends `SimpleWebPage` with content analysis that requires rendering
/// or more expensive processing (main content extraction, link graphs,
/// image inventories, category classification, and network/API detection).
struct CompleteWebPage {
    /// All fields from the simple analysis.
    simple: SimpleWebPage,
    /// The main textual content extracted from the page body.
    main_content: Option<String>,
    /// Word count of the extracted main content.
    word_count: Option<u32>,
    /// All outbound links found on the page.
    outbound_links: Vec<Url>,
    /// All internal links (same host) found on the page.
    internal_links: Vec<Url>,
    /// Notable images found in the page body.
    images: Vec<PageImage>,
    /// The primary image representing the page (OG image, hero, etc.).
    primary_image: Option<PageImage>,
    /// Category or topic tags assigned by the publisher or inferred.
    categories: Vec<String>,
    /// Whether the page appears to require authentication.
    access_level: AccessLevel,
    /// The publishing date, if detectable.
    date_published: Option<DateTime<Utc>>,
    /// The last-modified date, if detectable.
    date_modified: Option<DateTime<Utc>>,
    /// The author name(s), if detectable from metadata or content.
    authors: Vec<String>,
    /// Estimated reading time in minutes based on word count.
    reading_time_minutes: Option<u32>,
    /// Network requests observed during browser rendering.
    network_requests: Vec<PageNetworkRequest>,
    /// API endpoints detected during page rendering, classified by category.
    detected_apis: Vec<DetectedApi>,
}

/// A network request observed during browser-rendered page analysis.
///
/// Captured via CDP `Network` domain events. Not all fields are populated
/// for every request; the depth of capture depends on whether the request
/// was classified as a probable API candidate.
struct PageNetworkRequest {
    /// The CDP request ID used to join request/response/lifecycle events.
    request_id: String,
    /// The requested URL.
    url: Url,
    /// HTTP method (GET, POST, PUT, PATCH, DELETE, etc.).
    method: String,
    /// CDP-classified resource type (XHR, Fetch, Document, Script, etc.).
    resource_type: NetworkResourceType,
    /// The initiator that caused this request (parser, script, redirect, etc.).
    initiator: Option<RequestInitiator>,
    /// Whether this request was classified as a probable API call.
    is_probable_api: bool,
    /// Confidence score (0.0–1.0) for API classification.
    api_confidence: Option<f32>,
    /// HTTP status code from the response, if observed.
    status_code: Option<u16>,
    /// Response MIME type, if observed.
    response_mime_type: Option<String>,
    /// Wall-clock time in milliseconds from request start to loading finished.
    duration_ms: Option<f64>,
    /// Transfer size in bytes, if reported by CDP.
    transfer_size_bytes: Option<u64>,
}

/// CDP resource types relevant to API detection.
enum NetworkResourceType {
    /// An XMLHttpRequest.
    Xhr,
    /// A Fetch API request.
    Fetch,
    /// An EventSource (server-sent events) connection.
    EventSource,
    /// A WebSocket connection.
    WebSocket,
    /// A top-level document navigation.
    Document,
    /// A script resource.
    Script,
    /// A stylesheet.
    Stylesheet,
    /// An image resource.
    Image,
    /// A media resource (audio/video).
    Media,
    /// A font resource.
    Font,
    /// Any other resource type.
    Other(String),
}

/// What initiated a network request, derived from CDP `Network.requestWillBeSent`.
enum RequestInitiator {
    /// Initiated by the HTML parser.
    Parser,
    /// Initiated by JavaScript execution.
    Script,
    /// Initiated by a redirect from another request.
    Redirect,
    /// Initiated by a preload or prefetch hint.
    Preload,
    /// Initiated by a signed exchange.
    SignedExchange,
    /// Unknown or unclassifiable initiator.
    Other(String),
}

/// A detected API endpoint with classification and loose schema information.
///
/// Represents a higher-level summary derived from one or more observed
/// network requests that hit the same endpoint template. Multiple requests
/// with varying query parameters or request bodies may be grouped into a
/// single `DetectedApi`.
struct DetectedApi {
    /// A parameterized endpoint template, e.g. "/api/search?q={q}&page={page}".
    endpoint_template: String,
    /// The HTTP method(s) observed for this endpoint.
    methods: Vec<String>,
    /// The detected API category (REST, GraphQL, RPC, etc.).
    category: ApiCategory,
    /// Confidence score (0.0–1.0) for this classification.
    confidence: f32,
    /// Human-readable evidence supporting this classification.
    evidence: Vec<String>,
    /// The number of times this endpoint was hit during page observation.
    observation_count: u32,
    /// Inferred query parameter shapes from observed requests.
    query_params: Vec<ApiFieldShape>,
    /// Inferred request body shape, if captured.
    request_body_shape: Option<JsonShape>,
    /// Inferred response body shape, if captured.
    response_body_shape: Option<JsonShape>,
    /// Response content type(s) observed.
    response_content_types: Vec<String>,
    /// Whether pagination markers were detected in responses.
    has_pagination: Option<bool>,
}

/// Categories of detected API interactions.
enum ApiCategory {
    /// A JSON-based REST endpoint.
    JsonRest,
    /// A GraphQL endpoint (query, mutation, or subscription).
    GraphQl,
    /// A remote procedure call endpoint.
    Rpc,
    /// A search or query endpoint.
    Search,
    /// A paginated data endpoint.
    Pagination,
    /// An autocomplete or typeahead endpoint.
    Autocomplete,
    /// An authentication or session endpoint.
    Authentication,
    /// An analytics or telemetry beacon.
    Analytics,
    /// An ad or tracking pixel/endpoint.
    AdsOrTracking,
    /// A first-party structured data endpoint.
    FirstPartyData,
    /// A third-party data endpoint.
    ThirdPartyData,
    /// Category could not be determined.
    Unknown,
}

/// A loose schema for an API field (query parameter, JSON key, etc.).
struct ApiFieldShape {
    /// The field name.
    name: String,
    /// Observed value types (string, integer, boolean, array, object, null).
    observed_types: Vec<String>,
    /// Whether this field was absent in some observations.
    optional: bool,
    /// Example values observed for this field.
    examples: Vec<String>,
}

/// A loose schema for a JSON body shape.
///
/// This is inferred from observed examples, not a formal contract.
/// It captures the top-level type, field names, value types, and
/// nested structure to the depth observed.
struct JsonShape {
    /// The top-level JSON type.
    json_type: JsonType,
    /// For objects: field name → shape.
    fields: Option<HashMap<String, JsonShape>>,
    /// For arrays: the inferred item shape.
    items: Option<Box<JsonShape>>,
    /// Example values at this level.
    examples: Vec<String>,
}

/// The top-level type of a JSON value.
enum JsonType {
    Object,
    Array,
    String,
    Integer,
    Float,
    Boolean,
    Null,
}

/// A web page at varying levels of analysis depth.
///
/// The `Simple` variant is cheap to produce (no rendering, no content extraction)
/// and captures everything available from the raw HTML and HTTP response.
/// The `Complete` variant adds expensive content analysis including main-content
/// extraction, link inventories, image discovery, and category classification.
enum WebPage {
    /// Lightweight analysis from raw HTML meta tags and response headers.
    Simple(SimpleWebPage),
    /// Full analysis including rendered content, links, images, and classification.
    Complete(CompleteWebPage),
}
```

### Notes

- **Simple vs Complete split**: The two-level design reflects Reaper's CLI options (`reaper <url>` vs `reaper <url> --deep`). Simple is fast and cheap; Complete requires rendering and content analysis.
- **Schema.org as `serde_json::Value`**: Schema.org JSON-LD is highly variable across sites. Using a loosely typed value with extracted `@type` strings avoids an enormous enum while still enabling downstream consumers to parse specific types.
- **OpenGraph / TwitterCard / DublinCore as separate structs**: These are distinct vocabularies with different field sets. Keeping them separate avoids naming collisions (e.g. `title` means different things in OG vs DC) and makes it clear which standard each field came from.
- **PageType enum**: Initial set is based on the most common Schema.org verticals and URL patterns. The `Other(String)` variant handles anything not in the closed set. This may expand as we encounter more categories.
- **ResponseMeta separation**: Keeping response-level data (status code, timing, fetch method) in its own struct makes it easy to compare results across different fetch variants (raw HTTP vs browser render, different user agents).
- **Breadcrumb position**: 1-based to match Schema.org's `BreadcrumbList` position convention.
- **Network request capture**: `CompleteWebPage` includes `network_requests` and `detected_apis` because API detection requires a browser-rendered page with JavaScript execution (via `chromiumoxide` and CDP's `Network` domain). These fields are empty when the page was fetched via raw HTTP (`FetchMethod::HttpFetch`). The `PageNetworkRequest` struct is a simplified view of the full CDP event stream, joining `requestWillBeSent`, `responseReceived`, `loadingFinished`, and `loadingFailed` by CDP `requestId`.
- **DetectedApi groups requests by endpoint template**: Multiple observed requests to `/api/search?q=...&page=...` are collapsed into one `DetectedApi` with an `endpoint_template`, accumulated `query_params`, and `observation_count`. This avoids storing every individual network request when only the API shape matters.
- **ApiCategory enum**: Derived from the detection strategies in the api-detection research. Categories distinguish first-party data endpoints (valuable for scraping) from analytics, ads, and tracking (less valuable). The `FirstPartyData` variant covers structured data APIs that the page's own JavaScript consumes to render content.
- **JsonShape is intentionally loose**: Browser observation only sees examples, not the API contract. `JsonShape` captures inferred types and field names without claiming completeness. This is useful for understanding what data the page fetches dynamically, even when the static HTML lacks that content.
- **Pagination detection**: `DetectedApi.has_pagination` signals whether the API supports pagination (detected via markers like `next`, `cursor`, `offset`, `limit`, `page`, `total`, `has_more`). This is relevant for knowing whether a single page scrape captured all available data or only a partial view.
- **API confidence scoring model**: The `api_confidence` field uses a weighted scoring model derived from the api-detection research: CDP resource type (XHR/Fetch/EventSource/WebSocket = +0.35), JSON response content type (+0.25), API-like URL path (+0.15), structured response body (+0.15), post-load timing (+0.10), repeated endpoint with changing params (+0.10), minus penalties for known analytics/ad domains (-0.25) or static resource types (-0.20).
- **Open questions**:

    - Should `CompleteWebPage.authors` be `Vec<AuthorRef>` (with URL / Person / Organization discrimination) instead of plain strings?
    - Should we store microformats2 data as a first-class struct alongside the other metadata standards?
    - Should `PageType` detection confidence be stored alongside the detected type?
    - How should we represent AMP variants, paginated pages, or translated alternates?
    - Should `PageNetworkRequest` include request/response headers, or is that too verbose for the page-level struct? Headers might belong in a separate detailed network log.
    - Should we use CDP's `Fetch` domain for active interception alongside passive `Network` observation, or keep interception as a separate scrape mode?
    - Should captured response bodies be stored inline in `PageNetworkRequest` or kept in a separate body store keyed by request ID? Bounded storage is important to avoid unbounded memory use.

## WebSite

### Schema

```rust
use url::Url;
use std::collections::HashMap;

/// The type of CMS or technology platform powering a website.
enum CmsType {
    /// WordPress.
    WordPress,
    /// Shopify.
    Shopify,
    /// Squarespace.
    Squarespace,
    /// Wix.
    Wix,
    /// Ghost.
    Ghost,
    /// Joomla.
    Joomla,
    /// Drupal.
    Drupal,
    /// A custom or unrecognizable CMS.
    Other(String),
    /// CMS could not be determined.
    Unknown,
}

/// A glob pattern rule that classifies URL paths into page-type buckets.
struct PageTypePattern {
    /// A glob-style pattern matching URL paths, e.g. "/blog/**", "/products/*/reviews".
    pattern: String,
    /// The page type assigned to matching URLs.
    page_type: crate::WebPagePageType,
    /// A human-readable label for this pattern group, e.g. "Blog Posts", "Product Pages".
    label: String,
}

/// A known API endpoint pattern discovered at the site level.
///
/// Aggregated from `DetectedApi` observations across multiple pages on the
/// same site. Captures the common API surface the site exposes, enabling
/// direct API scraping as an alternative or supplement to page rendering.
struct SiteApiPattern {
    /// A parameterized endpoint template, e.g. "/api/v1/products/{id}".
    endpoint_template: String,
    /// The HTTP method(s) observed.
    methods: Vec<String>,
    /// The detected API category.
    category: crate::ApiCategory,
    /// URL or glob pattern for pages that are known to call this API.
    source_page_patterns: Vec<String>,
    /// Whether this API appears to return data that is also present in
    /// server-rendered HTML (redundant) or only available via the API.
    is_data_source: bool,
    /// Whether pagination was detected on this endpoint.
    has_pagination: Option<bool>,
    /// The number of distinct pages where this API was observed.
    observation_count: u32,
}

/// Contact information associated with a website or organization.
struct ContactInfo {
    /// Email addresses found on the site.
    emails: Vec<String>,
    /// Phone numbers found on the site.
    phones: Vec<String>,
    /// Social media profile URLs.
    social_profiles: HashMap<String, Url>,
}

/// Branding and visual identity elements extracted from the site.
struct SiteBranding {
    /// The site's name (from og:site_name, manifest, or HTML title).
    name: Option<String>,
    /// The tagline or subtitle if detectable.
    tagline: Option<String>,
    /// Primary logo URL.
    logo_url: Option<Url>,
    /// Favicon URL.
    favicon_url: Option<Url>,
    /// Primary brand color (from theme-color meta tag or manifest).
    primary_color: Option<String>,
    /// Theme color for the browser chrome (manifest or meta).
    theme_color: Option<String>,
}

/// Technical characteristics of a website's infrastructure.
struct SiteTechnology {
    /// The detected CMS, if any.
    cms: CmsType,
    /// The detected web framework or server technology, e.g. "Next.js", "Nginx".
    frameworks: Vec<String>,
    /// The analytics providers detected (Google Analytics, Plausible, etc.).
    analytics_providers: Vec<String>,
    /// The advertising networks detected.
    ad_networks: Vec<String>,
    /// Whether the site has a Web App Manifest.
    has_manifest: bool,
    /// Whether the site declares a service worker.
    has_service_worker: Option<bool>,
    /// Whether the site uses HTTPS.
    uses_https: bool,
    /// Other detected technologies (CDN, hosting provider, etc.).
    extra: Vec<String>,
}

/// A URL that redirects to or mirrors the site's primary URL.
struct SiteAlias {
    /// The alias URL.
    url: Url,
    /// The kind of alias (redirect, canonical mirror, subdomain, etc.).
    alias_type: SiteAliasType,
}

/// The relationship between an alias URL and the primary URL.
enum SiteAliasType {
    /// The alias issues an HTTP redirect to the primary URL.
    Redirect,
    /// The alias serves identical content without redirecting.
    Mirror,
    /// A subdomain of the primary domain, e.g. "blog.example.com".
    Subdomain,
    /// An alternate TLD, e.g. ".net" instead of ".com".
    AlternateTld,
    /// An AMP cache URL.
    AmpCache,
    /// A mobile-specific subdomain, e.g. "m.example.com".
    MobileSubdomain,
}

/// Ownership and legal entity information about the website.
struct SiteOwnership {
    /// The company or individual that owns/publishes the site.
    publisher: Option<EntityRef>,
    /// Copyright holder text as declared on the site.
    copyright_holder: Option<String>,
    /// Privacy policy URL.
    privacy_policy_url: Option<Url>,
    /// Terms of service URL.
    terms_url: Option<Url>,
    /// The domain registrar (if WHOIS data is available).
    registrar: Option<String>,
    /// Domain registration and expiry dates.
    domain_expires: Option<chrono::NaiveDate>,
}

/// A reference to an entity (Company or Person) stored elsewhere.
///
/// Uses a lightweight reference rather than embedding the full entity,
/// since the same company or person may be referenced by many pages/sites.
enum EntityRef {
    /// A reference by URL to an external identifier (LinkedIn, Wikidata, etc.).
    ByUrl(Url),
    /// A reference by a well-known identifier scheme and value.
    ById { scheme: String, id: String },
    /// An inline name when no structured reference is available.
    ByName(String),
}

/// Metadata about a site's content feeds.
struct FeedInfo {
    /// The URL of the RSS or Atom feed.
    url: Url,
    /// The feed format (RSS 2.0, Atom, JSON Feed).
    format: String,
    /// The feed title.
    title: Option<String>,
}

/// Macro-level metadata about an entire website.
///
/// Aggregates information that spans the whole site rather than individual
/// pages: identity, ownership, technology stack, URL structure patterns,
/// discovered API surface, and references to pages that have been scraped.
struct WebSite {
    /// The primary base URL of the website (e.g. "https://example.com").
    base_url: Url,
    /// Alternative URLs that redirect to or mirror the primary URL.
    aliases: Vec<SiteAlias>,
    /// A glob pattern scoping which URLs under base_url belong to this site definition.
    url_glob: Option<String>,
    /// Visual branding and identity elements.
    branding: SiteBranding,
    /// Detected technology stack and infrastructure.
    technology: SiteTechnology,
    /// Ownership and legal entity information.
    ownership: SiteOwnership,
    /// Contact information found across the site.
    contact: ContactInfo,
    /// Known RSS, Atom, or JSON feeds provided by the site.
    feeds: Vec<FeedInfo>,
    /// URL patterns that classify pages into types within this site.
    page_type_patterns: Vec<PageTypePattern>,
    /// API endpoint patterns discovered across scraped pages on this site.
    api_patterns: Vec<SiteApiPattern>,
    /// The primary language of the site (BCP 47 tag).
    language: Option<String>,
    /// Geographic regions the site targets.
    target_regions: Vec<String>,
    /// URL patterns that should be excluded from scraping (e.g. admin, login).
    exclude_patterns: Vec<String>,
    /// The number of pages discovered on this site (may be estimated).
    estimated_page_count: Option<u64>,
    /// Timestamp of the last site-level scan or crawl.
    last_crawled_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

### Notes

- **EntityRef for cross-referencing**: `EntityRef` is a lightweight reference that avoids embedding full `Company` or `Person` structs. This keeps `WebSite` independent and allows entities to be stored/looked up separately. The `ByUrl` variant works well for Schema.org `sameAs` links; `ById` handles DOIs, Wikidata IDs, etc.
- **PageTypePattern for URL classification**: Glob patterns let users teach Reaper how to interpret the site's URL structure (e.g. `/blog/*` → Blog Posts, `/products/*` → Product Pages). This avoids classifying every page from scratch on each scrape.
- **url_glob scoping**: A single glob on `base_url` lets users scope what "the site" means — e.g. only `/en/**` for the English subset, or everything under `docs.example.com/**`.
- **CmsType enum**: Initial set covers the most common CMS platforms. The `Other(String)` and `Unknown` variants handle everything else. This will grow as detection improves.
- **SiteAlias**: Websites often have multiple entry points (www vs non-www, HTTP vs HTTPS, mobile subdomains, alternate TLDs). Explicit alias tracking avoids re-scraping the same content under different URLs.
- **SiteTechnology**: Detection is based on HTTP headers, meta generators, script/src patterns, and manifest contents. The `extra` Vec catches everything not covered by named fields.
- **SiteApiPattern for discovered APIs**: Aggregated from per-page `DetectedApi` observations across multiple scraped pages. When Reaper detects that many product pages on a Shopify site call `/api/v1/products/{id}`, this becomes a `SiteApiPattern` with `source_page_patterns: ["/products/*"]` and `is_data_source: true`. This enables two scraping strategies for the same site: render pages, or call the discovered API directly. The `is_data_source` flag indicates whether the API returns data that supplements or replaces what is in the rendered HTML.
- **API detection vs page rendering tradeoff**: Some sites (SPAs, headless CMS frontends) render almost no meaningful content in the initial HTML — all data comes from API calls. In those cases, `SiteApiPattern` with `is_data_source = true` is the more efficient scraping path. Sites with server-rendered content may still benefit from API detection for structured data extraction (e.g. getting product pricing as JSON instead of parsing HTML).
- **Open questions**:

    - Should `page_type_patterns` support regex in addition to globs?
    - Should `WebSite` store a set of sampled/reference `WebPage` structs directly, or just keep a count and let consumers query separately?
    - How should we represent multi-region / multi-language sites (e.g. `example.com/en/`, `example.com/de/`)?
    - Should `ownership` link to a full `Company` entity or remain inline?
    - Should crawl rate / politeness settings live here or in a separate crawl configuration struct?
    - Should `api_patterns` include authentication requirements observed during detection (e.g. "requires session cookie" vs "requires API key header")?
    - Should `SiteApiPattern` track rate-limiting signals (HTTP 429 responses, `Retry-After` headers) observed during scraping?

## Company

### Schema

```rust
use url::Url;
use std::collections::HashMap;

/// The legal structure of a company.
enum CompanyType {
    /// A publicly traded company with stock ticker.
    PublicCompany,
    /// A privately held company.
    PrivateCompany,
    /// A limited liability company.
    Llc,
    /// A nonprofit organization.
    Nonprofit,
    /// A government entity or agency.
    Government,
    /// A sole proprietorship.
    SoleProprietorship,
    /// A partnership.
    Partnership,
    /// A cooperative.
    Cooperative,
    /// The legal structure is unknown or not specified.
    Other(String),
}

/// A company's stock market listing details.
struct StockListing {
    /// The exchange name, e.g. "NASDAQ", "NYSE", "LSE".
    exchange: String,
    /// The ticker symbol, e.g. "AAPL", "MSFT".
    ticker: String,
}

/// An industry or sector classification.
struct IndustryClassification {
    /// The classification scheme used (e.g. "NAICS", "SIC", "GICS", "free-form").
    scheme: String,
    /// The category code or label within that scheme.
    category: String,
}

/// A social media profile belonging to the company.
struct SocialProfile {
    /// The platform name, e.g. "twitter", "linkedin", "github".
    platform: String,
    /// The profile URL.
    url: Url,
    /// The handle or username on that platform.
    handle: Option<String>,
}

/// Identifiers from external databases and registries.
struct CompanyIdentifiers {
    /// LinkedIn company page URL.
    linkedin_url: Option<Url>,
    /// Wikidata entity ID (Q-number).
    wikidata_id: Option<String>,
    /// Wikipedia article URL.
    wikipedia_url: Option<Url>,
    /// Crunchbase organization URL.
    crunchbase_url: Option<Url>,
    /// A SEC CIK (Central Index Key) for US public companies.
    sec_cik: Option<String>,
    /// The company registration number in its home jurisdiction.
    registration_number: Option<String>,
    /// LEI (Legal Entity Identifier).
    lei: Option<String>,
    /// External IDs in other registries.
    extra: HashMap<String, String>,
}

/// Information about a company or organization.
///
/// Represents a legal entity that may own websites, publish content,
/// manufacture products, or employ people. Fields are optional because
/// the depth of available information varies widely by source.
struct Company {
    /// The official legal name of the company.
    legal_name: String,
    /// The common or trading name, if different from the legal name.
    trading_name: Option<String>,
    /// A brief description of what the company does.
    description: Option<String>,
    /// The company's primary website.
    website: Option<Url>,
    /// The legal structure (public, private, LLC, etc.).
    company_type: Option<CompanyType>,
    /// Stock listings, if the company is publicly traded.
    stock_listings: Vec<StockListing>,
    /// Industry or sector classifications.
    industries: Vec<IndustryClassification>,
    /// The year the company was founded.
    founded_year: Option<u16>,
    /// The date the company was dissolved or ceased operations, if applicable.
    dissolved_date: Option<chrono::NaiveDate>,
    /// The headquarters location (free-text or structured address).
    headquarters: Option<String>,
    /// Total number of employees (may be a range or estimate).
    employee_count: Option<String>,
    /// Most recent annual revenue figure, if public.
    annual_revenue: Option<String>,
    /// The CEO or top executive's name.
    ceo: Option<String>,
    /// The parent company, if this is a subsidiary.
    parent_company: Option<Box<Company>>,
    /// Key brands or product lines owned by the company.
    brands: Vec<String>,
    /// Physical addresses associated with the company.
    addresses: Vec<String>,
    /// Social media profiles.
    social_profiles: Vec<SocialProfile>,
    /// External database identifiers.
    identifiers: CompanyIdentifiers,
    /// A logo image URL.
    logo_url: Option<Url>,
    /// Tags or keywords describing the company.
    tags: Vec<String>,
}
```

### Notes

- **parent_company as `Box<Company>`**: Recursive ownership chains are common (subsidiary of a subsidiary). Boxing avoids infinite-size issues. For deep ownership trees, consider switching to an `EntityRef` or a separate ownership graph.
- **CompanyType covers major legal forms**: The enum handles the most common structures globally. `Other(String)` handles jurisdiction-specific types (e.g. GmbH, Ltd, B.V.).
- **IndustryClassification is scheme-agnostic**: Different data sources use different classification systems (NAICS, SIC, GICS). Storing the scheme alongside the category makes merging data from multiple sources possible.
- **employee_count and annual_revenue as strings**: These are often ranges ("1,000-5,000"), estimates, or currency-denominated. Using strings preserves the source representation. A more structured approach could use `Range<u64>` and a `Currency` enum in a future pass.
- **CompanyIdentifiers**: External IDs are the primary way to de-duplicate and cross-reference companies across data sources. The `extra` HashMap handles any registry not covered by the named fields.
- **API detection relevance**: When Reaper detects API calls on a company's website, the `DetectedApi` and `SiteApiPattern` types may reveal the company's technology choices (e.g. GraphQL API suggests a modern frontend stack, Shopify Storefront API suggests the company uses Shopify). The `SiteTechnology` struct on `WebSite` already captures detected frameworks; API patterns provide a complementary signal. Company data may also be enriched by following `sameAs` links in Schema.org JSON-LD (e.g. Wikidata Q-numbers, LinkedIn company pages, Crunchbase URLs) which are exposed as network requests during page rendering.
- **Open questions**:

    - Should `headquarters` be a `Place` struct reference instead of a free-text string?
    - Should we store subsidiary relationships as a separate graph rather than only `parent_company`?
    - How should we handle company mergers, acquisitions, and name changes over time?
    - Should `addresses` be `Vec<Place>` references?

## Person

### Schema

```rust
use url::Url;
use std::collections::HashMap;

/// A person's role or relationship to an organization.
struct Affiliation {
    /// The organization name.
    organization: String,
    /// The person's job title or role.
    role: Option<String>,
    /// The department or division.
    department: Option<String>,
    /// When the affiliation started.
    start_date: Option<chrono::NaiveDate>,
    /// When the affiliation ended (None if current).
    end_date: Option<chrono::NaiveDate>,
}

/// A person's presence on a social or professional platform.
struct PersonSocialProfile {
    /// The platform name, e.g. "twitter", "linkedin", "github", "mastodon".
    platform: String,
    /// The profile URL.
    url: Url,
    /// The handle or username.
    handle: Option<String>,
    /// The display name on the platform, if different from the person's real name.
    display_name: Option<String>,
    /// Whether this profile has been verified as belonging to the person.
    is_verified: Option<bool>,
}

/// External identifiers for a person across databases.
struct PersonIdentifiers {
    /// Wikidata entity ID (Q-number).
    wikidata_id: Option<String>,
    /// Wikipedia article URL.
    wikipedia_url: Option<Url>,
    /// ORCID iD for academic authors.
    orcid: Option<String>,
    /// IMDb name identifier.
    imdb_id: Option<String>,
    /// GitHub username.
    github_username: Option<String>,
    /// External IDs in other registries.
    extra: HashMap<String, String>,
}

/// Information about a person.
///
/// Represents an individual human being. Used to capture authorship,
/// organizational leadership, contact details, and public presence.
/// All fields are optional except the name because the available
/// information varies enormously by source and context.
struct Person {
    /// The person's full display name.
    full_name: String,
    /// Given / first name.
    given_name: Option<String>,
    /// Family / last name.
    family_name: Option<String>,
    /// Middle name(s) or initial(s).
    middle_name: Option<String>,
    /// A honorific prefix, e.g. "Dr.", "Prof.", "Sir".
    honorific_prefix: Option<String>,
    /// A honorific suffix, e.g. "Jr.", "PhD", "III".
    honorific_suffix: Option<String>,
    /// A brief biographical summary.
    bio: Option<String>,
    /// The person's primary job title.
    job_title: Option<String>,
    /// Organizational affiliations (current and past).
    affiliations: Vec<Affiliation>,
    /// The person's primary email address.
    email: Option<String>,
    /// A personal or professional website.
    website: Option<Url>,
    /// A profile photo or avatar URL.
    image_url: Option<Url>,
    /// The person's birth date, if public.
    birth_date: Option<chrono::NaiveDate>,
    /// The person's death date, if applicable.
    death_date: Option<chrono::NaiveDate>,
    /// The person's geographic location (city, region, country).
    location: Option<String>,
    /// Nationality or citizenship.
    nationality: Option<String>,
    /// Languages the person works in (BCP 47 tags).
    languages: Vec<String>,
    /// Social and professional profiles.
    social_profiles: Vec<PersonSocialProfile>,
    /// External database identifiers.
    identifiers: PersonIdentifiers,
    /// Keywords or areas of expertise.
    expertise: Vec<String>,
}
```

### Notes

- **Name decomposition**: Storing `given_name`, `family_name`, and `middle_name` separately alongside `full_name` supports both display and matching. Name ordering conventions differ by culture, so keeping the parsed parts avoids ambiguity.
- **Affiliation with date range**: The `start_date`/`end_date` pattern lets us track both current and past affiliations. A `None` end_date means the affiliation is current.
- **PersonIdentifiers**: External IDs are critical for deduplication. Two Person records with different names but the same ORCID or Wikidata ID are the same individual.
- **location as free-text string**: Person locations are often imprecise ("San Francisco Bay Area", "London, UK"). Using a string avoids over-structuring data that is inherently fuzzy. A `Place` reference could be used when more precision is available.
- **API detection relevance**: Person profile pages on platforms like LinkedIn, GitHub, or Twitter/X load profile data via API calls. The `DetectedApi` types from browser network monitoring can capture structured person data (name, title, company, location) that may be more complete than what is visible in the rendered HTML. Schema.org `Person` JSON-LD on personal homepages and author pages is another rich source for populating `Person` fields.
- **Open questions**:

    - Should `location` be an `Option<EntityRef>` pointing to a `Place` entity?
    - Should we track name changes (e.g. maiden name, legal name changes)?
    - Should `Affiliation.organization` be an `EntityRef` to a `Company` instead of a string?
    - How should we handle pseudonyms or pen names?

## Product

### Schema

```rust
use url::Url;
use std::collections::HashMap;

/// The availability state of a product.
enum ProductAvailability {
    /// In stock and ready to ship.
    InStock,
    /// Available for pre-order.
    PreOrder,
    /// Out of stock, may restock.
    OutOfStock,
    /// Discontinued, no longer available.
    Discontinued,
    /// Limited availability or low stock.
    LimitedAvailability,
    /// Available only in physical stores.
    InStoreOnly,
    /// Available for purchase online.
    OnlineOnly,
    /// The product has been recalled.
    Recalled,
    /// Availability status could not be determined.
    Unknown,
}

/// A price with currency information.
struct Price {
    /// The numeric price value.
    amount: f64,
    /// The ISO 4217 currency code, e.g. "USD", "EUR", "GBP".
    currency: String,
    /// The date this price was observed.
    observed_at: chrono::DateTime<chrono::Utc>,
    /// Whether this is the list price, sale price, or another type.
    price_type: PriceType,
}

/// The type of price being represented.
enum PriceType {
    /// The manufacturer's suggested retail price.
    Msrp,
    /// A sale or promotional price.
    Sale,
    /// The regular non-sale price.
    List,
    /// The minimum advertised price.
    Map,
    /// A subscription or recurring price.
    Subscription,
    /// A used / refurbished price.
    Used,
    /// A rental or lease price.
    Rental,
    /// Some other price designation.
    Other(String),
}

/// A product rating, either aggregate or individual.
struct Rating {
    /// The rating value (on the scale defined by best/worst).
    value: f64,
    /// The best possible rating value (usually 5).
    best: Option<f64>,
    /// The worst possible rating value (usually 1).
    worst: Option<f64>,
    /// Number of ratings contributing to this aggregate.
    count: Option<u64>,
}

/// A structured review of a product.
struct ProductReview {
    /// The review headline or title.
    title: Option<String>,
    /// The full review body text.
    body: Option<String>,
    /// The rating given in this review.
    rating: Option<Rating>,
    /// The author's display name.
    author: Option<String>,
    /// The date the review was published.
    date: Option<chrono::NaiveDate>,
    /// Whether this review was verified as a real purchase.
    is_verified: Option<bool>,
}

/// A product identifier in a specific naming system.
struct ProductIdentifier {
    /// The identifier scheme.
    scheme: ProductIdentifierScheme,
    /// The identifier value within that scheme.
    value: String,
}

/// Known product identifier schemes.
enum ProductIdentifierScheme {
    /// Universal Product Code (12-digit barcode).
    Upc,
    /// International Article Number (13-digit barcode).
    Ean,
    /// International Standard Book Number.
    Isbn,
    /// Amazon Standard Identification Number (ASIN).
    Asin,
    /// Global Trade Item Number.
    Gtin,
    /// Manufacturer Part Number.
    Mpn,
    /// SKU (Stock Keeping Unit).
    Sku,
    /// A custom or proprietary identifier scheme.
    Other(String),
}

/// Product dimension or weight measurement.
struct Measurement {
    /// The measurement type (length, width, height, weight).
    dimension: MeasurementType,
    /// The numeric value.
    value: f64,
    /// The unit of measurement, e.g. "cm", "in", "kg", "lb".
    unit: String,
}

/// The kind of physical dimension being measured.
enum MeasurementType {
    Length,
    Width,
    Height,
    Weight,
    Depth,
    Volume,
}

/// Information about a product or service offering.
///
/// Captures the core attributes of a product as found on e-commerce pages,
/// product review sites, and structured data (Schema.org Product, OG product).
struct Product {
    /// The product name as displayed to consumers.
    name: String,
    /// A detailed description of the product.
    description: Option<String>,
    /// A brief summary or tagline.
    summary: Option<String>,
    /// The brand name or manufacturer.
    brand: Option<String>,
    /// The product category or taxonomy path (e.g. "Electronics > Phones > Smartphones").
    category: Option<String>,
    /// The URL of the product page.
    url: Option<Url>,
    /// URLs of product images.
    image_urls: Vec<Url>,
    /// Product identifiers (UPC, EAN, ISBN, ASIN, etc.).
    identifiers: Vec<ProductIdentifier>,
    /// Current and historical prices.
    prices: Vec<Price>,
    /// Current availability status.
    availability: ProductAvailability,
    /// The aggregate rating across all reviews.
    aggregate_rating: Option<Rating>,
    /// Individual product reviews, if available.
    reviews: Vec<ProductReview>,
    /// Physical dimensions and weight.
    measurements: Vec<Measurement>,
    /// The product's color options, if applicable.
    colors: Vec<String>,
    /// The product's size options, if applicable.
    sizes: Vec<String>,
    /// The material composition, if applicable.
    materials: Vec<String>,
    /// The manufacturer's name.
    manufacturer: Option<String>,
    /// The country of manufacture.
    country_of_origin: Option<String>,
    /// The date the product was first available.
    release_date: Option<chrono::NaiveDate>,
    /// Additional product attributes not covered by typed fields.
    extra_attributes: HashMap<String, String>,
}
```

### Notes

- **Price with observed_at**: Prices change over time. Storing the observation timestamp allows price tracking and historical analysis. Multiple `Price` entries capture price history.
- **PriceType discrimination**: A single product page may show multiple prices (MSRP, sale, subscription). The enum distinguishes them.
- **ProductIdentifierScheme**: Different retailers and regions use different ID systems. Storing the scheme alongside the value enables cross-referencing the same product across stores.
- **Rating with best/worst**: Not all rating scales are 1-5. Some are 1-10, some are 0-100. Explicit best/worst makes ratings comparable across sources.
- **Measurements as a list**: Products may have multiple dimensions. A flat list of `Measurement` structs with a dimension type tag is more flexible than named fields.
- **extra_attributes HashMap**: Product pages contain a huge variety of attributes (wattage, thread count, battery capacity, etc.) that can't be anticipated in typed fields. The HashMap captures them without schema explosion.
- **API-driven product data**: Many e-commerce sites (Shopify, Magento, WooCommerce) load product data via XHR/Fetch API calls rather than embedding it in the initial HTML. When `CompleteWebPage` captures these API calls via CDP network monitoring, the `DetectedApi` entries may contain product pricing, availability, variants, and reviews as structured JSON. The `Product` struct should be populatable from either HTML parsing or API response analysis — the api-detection research shows that response body shapes for product APIs often include fields like `id`, `title`, `variants[].price`, `available`, and `images[]` that map directly to `Product` fields. Using the API path is more reliable than HTML parsing for SPA-based stores.
- **Product data from Schema.org + API fusion**: Some product pages include Schema.org JSON-LD with basic product info in the static HTML, then load detailed pricing and availability via API. The `Product` struct should support merging data from both sources, with API-sourced data taking precedence for volatile fields (price, availability, inventory count).
- **Open questions**:

    - Should `brand` and `manufacturer` be `EntityRef` to `Company`?
    - Should variants (color, size, etc.) be modeled as a separate `ProductVariant` struct?
    - Should `reviews` be stored inline or referenced by URL?
    - How should we model product bundles or kits?
    - Should `Product` include a `data_source` field indicating whether fields were populated from HTML, Schema.org, detected API, or a combination?
    - Should product APIs discovered via `DetectedApi` (e.g. `/api/products/{id}`) be tracked on the `Product` itself so that future scrapes can hit the API directly?

## Place

### Schema

```rust
use url::Url;

/// The kind of place being described.
enum PlaceType {
    /// A country.
    Country,
    /// A state, province, or region.
    Region,
    /// A city, town, or municipality.
    City,
    /// A specific street address.
    Address,
    /// A neighborhood or district within a city.
    Neighborhood,
    /// A point of interest (landmark, attraction).
    Landmark,
    /// A business location (store, restaurant, office).
    BusinessLocation,
    /// A geographic area (park, lake, mountain).
    GeographicArea,
    /// Some other type of place.
    Other(String),
}

/// A geographic coordinate pair.
struct GeoCoordinates {
    /// Latitude in decimal degrees (positive = north).
    latitude: f64,
    /// Longitude in decimal degrees (positive = east).
    longitude: f64,
    /// The coordinate reference system, defaults to WGS 84 (EPSG:4326).
    crs: Option<String>,
    /// The altitude/elevation in meters above sea level.
    elevation_meters: Option<f64>,
}

/// A structured postal address.
struct PostalAddress {
    /// The street address including house number and street name.
    street_address: Option<String>,
    /// The apartment, suite, or unit number.
    unit: Option<String>,
    /// The city or locality.
    city: Option<String>,
    /// The state, province, or region.
    region: Option<String>,
    /// The postal code or ZIP code.
    postal_code: Option<String>,
    /// The country name or ISO 3166-1 code.
    country: Option<String>,
    /// The full formatted address as a single string.
    formatted: Option<String>,
}

/// Opening hours for a place, expressed as a schedule.
struct OpeningHours {
    /// The day of the week.
    day: DayOfWeek,
    /// The opening time (HH:MM format, 24-hour).
    opens: String,
    /// The closing time (HH:MM format, 24-hour).
    closes: String,
    /// Whether this is a seasonal or temporary schedule.
    is_seasonal: bool,
}

/// Days of the week for opening hours scheduling.
enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
    /// A public holiday with variable schedule.
    Holiday,
}

/// A geographic containment hierarchy (e.g. city contains neighborhood).
struct PlaceContainment {
    /// The broader containing place.
    contained_by: Option<Box<Place>>,
}

/// Information about a geographic place or location.
///
/// Represents a physical location at varying levels of specificity — from
/// a country down to a street address. Used to geotag content, describe
/// business locations, and represent addresses found in metadata.
struct Place {
    /// The common name of the place.
    name: String,
    /// The kind of place (city, country, address, etc.).
    place_type: PlaceType,
    /// A description of the place.
    description: Option<String>,
    /// The geographic coordinates, if known.
    coordinates: Option<GeoCoordinates>,
    /// The structured postal address, if applicable.
    address: Option<PostalAddress>,
    /// The telephone number, if this is a business location.
    telephone: Option<String>,
    /// The URL of the place's website or listing page.
    url: Option<Url>,
    /// Opening hours, if this is a business or facility.
    opening_hours: Vec<OpeningHours>,
    /// The place that contains this place (e.g. country contains city).
    contained_by: Option<Box<Place>>,
    /// URLs of images representing the place.
    image_urls: Vec<Url>,
    /// External identifiers (Wikidata ID, GeoNames ID, etc.).
    identifiers: std::collections::HashMap<String, String>,
    /// The IANA timezone identifier, e.g. "America/Los_Angeles".
    timezone: Option<String>,
}
```

### Notes

- **Recursive containment via `Box<Place>`**: Places form a natural hierarchy (country → region → city → neighborhood → address). The `contained_by` field allows traversal up the hierarchy. For deep chains, consider a flattened path or a separate hierarchy graph.
- **PlaceType discrimination**: The enum covers the most common granularity levels. It aligns with Schema.org's Place subtypes (Country, State, City, LocalBusiness).
- **GeoCoordinates with CRS**: WGS 84 is the universal default, but storing the CRS explicitly handles edge cases (projected coordinate systems, local grids).
- **PostalAddress.formatted**: Even when we can parse into components, preserving the original formatted address is valuable — parsing may lose information (building names, care-of lines).
- **OpeningHours**: Modeled per-day with open/close times. This covers most cases but doesn't handle split shifts (e.g. closed 12-1 for lunch) — that could be modeled as two `OpeningHours` entries for the same day.
- **API detection relevance**: Many local business and store locator pages (e.g. restaurant chains, retail stores) load location data from API endpoints (often `/api/stores`, `/api/locations`, or similar). The `DetectedApi` types can capture these API calls which return structured `Place` data (coordinates, addresses, hours) as JSON. This is particularly valuable for business directory sites where the API returns cleaner data than the rendered HTML. The api-detection research's body shape detection can infer the `GeoCoordinates` and `PostalAddress` structures directly from the JSON response shapes.
- **Open questions**:

    - Should `contained_by` use an `EntityRef` instead of `Box<Place>` to avoid deep nesting and duplication?
    - Should we support GeoJSON geometries (polygons, multi-polygons) for geographic areas, not just point coordinates?
    - Should `OpeningHours` support date ranges for seasonal schedules?
    - Should the `timezone` field use `chrono_tz::Tz` instead of a string?
    - How should we represent a place that has moved or changed boundaries over time?
    - Should `Place` include a reference to the API endpoint that provided the location data (when sourced from a detected API)?
