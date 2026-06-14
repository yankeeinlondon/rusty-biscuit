---
prompt: |-
    The Reaper library and CLI are website scrapers and one of the things we want to scrape is all of the microdata that is found on a webpage's `<meta>` tags.
   
    Your task is to identify and report on:
      
    - all major microdata standards
        - describe the standard
        - what kind of websites are most likely to include this
        - what types of data is this standard used to define or is it broad based?
    - if different standards have overlap with one another mention what that overlap is
        - describe when this overlap is a desired outcome, when it is not
    - are there any follow-on network lookups that this metadata points to or references opaquely which could be used to further enhance our metadata understanding?
last_updated: 2026-06-03
---
Now I have enough research material. Let me compose the report.

## Microdata Standards in `<meta>` Tags

### 1. HTML Standard Meta Tags

**Description:** The native HTML `<meta>` element provides key-value pairs in the `<head>` via `name`/`content` or `http-equiv`/`content` attributes. Core tags include `description`, `keywords`, `author`, `robots`, `viewport`, `charset`, `referrer`, `theme-color`, `color-scheme`, and `generator`.

**Common on:** Virtually every website. `description` and `viewport` are near-universal. `robots` and `author` are very common on content sites.

**Data types:** Broad. Covers identity (author, generator), SEO directives (robots, description), presentation (viewport, theme-color, color-scheme), and security/privacy (referrer, content-security-policy).

---

### 2. Open Graph Protocol (OGP)

**Description:** Created by Facebook, OGP uses `<meta property="og:..." content="...">` to turn any webpage into a rich social-graph object. Four required properties: `og:title`, `og:type`, `og:image`, `og:url`. Supports structured properties (e.g. `og:image:width`, `og:image:height`, `og:image:alt`) and typed verticals: `music.song`, `music.album`, `video.movie`, `video.episode`, `article`, `book`, `profile`, `website`.

**Common on:** Nearly every website that cares about social sharing -- news, e-commerce, blogs, streaming services, SaaS landing pages. CMS platforms (WordPress, Shopify, etc.) emit OGP by default.

**Data types:** Broad but social-sharing oriented. Covers identity, media (images/video/audio), article metadata, book metadata, person profiles, and locale information.

---

### 3. Twitter / X Cards

**Description:** Uses `<meta name="twitter:..." content="...">` to control how content appears when shared on X/Twitter. Defines four card types: `summary`, `summary_large_image`, `app`, and `player`. Key properties include `twitter:card`, `twitter:site`, `twitter:title`, `twitter:description`, `twitter:image`, `twitter:player`, `twitter:app:id:iphone`, etc.

**Common on:** Most websites that share content socially -- same audience as OGP. Any site wanting rich previews on X/Twitter.

**Data types:** Narrower than OGP. Focused on card presentation: title, description, image, and optionally embedded media players or app download links.

---

### 4. Schema.org / Structured Data

**Description:** A collaborative vocabulary (founded by Google, Microsoft, Yahoo, Yandex) for structured data. As of 2024, over 45 million web domains use it with 450+ billion objects. Can appear in three encoding formats:

- **JSON-LD** (`<script type="application/ld+json">`) -- Google's recommended format, not in `<meta>` tags but in `<head>` scripts
- **Microdata** (`itemscope`, `itemprop`, `itemtype` attributes on HTML elements) -- inline in the DOM
- **RDFa** (`vocab`, `typeof`, `property` attributes) -- inline in the DOM

Covers hundreds of types: `Article`, `Product`, `Recipe`, `Event`, `LocalBusiness`, `Person`, `Organization`, `FAQPage`, `HowTo`, `VideoObject`, `BreadcrumbList`, `AggregateRating`, `JobPosting`, and many more.

**Common on:** E-commerce (Product, Offer, AggregateRating), news/media (Article, NewsArticle), local businesses (LocalBusiness, Restaurant), events, recipe sites, job boards, FAQ pages. Any site targeting Google rich results.

**Data types:** Extremely broad. The most comprehensive structured data vocabulary on the web. Covers commercial, editorial, geographic, temporal, social, and domain-specific data.

---

### 5. Dublin Core Metadata Initiative (DCMI)

**Description:** An ISO-standardized vocabulary (ISO 15836) for describing digital resources. Uses `<meta name="DC.title" content="...">` or `<meta name="dcterms.title" content="...">` (or the `property` attribute variant). Defines 15 core elements: `title`, `creator`, `subject`, `description`, `publisher`, `contributor`, `date`, `type`, `format`, `identifier`, `source`, `language`, `relation`, `coverage`, `rights`. Extended terms add dozens more (e.g. `abstract`, `bibliographicCitation`, `license`, `provenance`).

**Common on:** Academic and institutional repositories, digital libraries, museum collections, government portals, archival systems. Less common on commercial websites.

**Data types:** Broad, bibliographic. Optimized for cataloging and resource discovery rather than social sharing or commerce.

---

### 6. oEmbed (Discovery via `<link>` tags)

**Description:** While not `<meta>` tags per se, oEmbed uses `<link rel="alternate" type="application/json+oembed" href="...">` in the `<head>` to advertise an endpoint that returns structured embed data (title, author, thumbnail, dimensions, HTML embed code) in JSON or XML. The discovery mechanism lives alongside `<meta>` tags and should be scraped similarly.

**Common on:** Media-rich sites -- YouTube, Flickr, Spotify, SoundCloud, Vimeo, SlideShare, deviantART, and 378+ registered providers. Any site that wants its content embeddable in third-party contexts.

**Data types:** Focused on embeddable content. Returns `photo`, `video`, `link`, or `rich` types with associated metadata (title, author, provider, thumbnail, dimensions, HTML embed snippet).

---

### 7. Microformats (HTML class-based)

**Description:** Not `<meta>` tags, but HTML class-based markup (`h-card`, `h-entry`, `h-event`, `h-product`, `h-review`, etc.) embedded in the page body. microformats2 uses `class="h-*"` for root elements and `p-*` / `u-*` / `dt-*` / `e-*` prefixes for properties. Worth noting because parsers that scrape `<meta>` tags often also consume microformats from the same page.

**Common on:** Blogs (especially IndieWeb sites), event pages, review sites, personal homepages. Lower adoption than OGP/Schema.org but strong in the IndieWeb community.

**Data types:** Moderate. Covers contacts (h-card), posts (h-entry), events (h-event), products (h-product), reviews (h-review), recipes (h-recipe), resumes (h-resume), and addresses (h-adr).

---

### 8. App Links and Mobile Deep Linking

**Description:** Uses `<meta property="al:..." content="...">` (App Links protocol) to define platform-specific deep links into native mobile apps. Properties include `al:ios:url`, `al:ios:app_store_id`, `al:ios:app_name`, `al:android:url`, `al:android:package`, `al:android:app_name`, `al:web:url`, `al:web:should_fallback`. Facebook uses these for its in-app browser deep-linking.

**Common on:** Websites with companion mobile apps -- social media platforms, news apps, e-commerce, streaming services.

**Data types:** Narrow. Focused on cross-platform app routing and fallback behavior.

---

### 9. Apple Smart App Banners

**Description:** Uses `<meta name="apple-itunes-app" content="app-id=..., affiliate-data=..., app-argument=...">` to display a banner at the top of Safari promoting an iOS app. The `app-argument` parameter passes context (e.g. a deep link URL) to the app on launch.

**Common on:** Websites with iOS companion apps -- news outlets, e-commerce, social media, SaaS products.

**Data types:** Narrow. Just app identification and an optional deep-link argument.

---

### 10. Microsoft `msapplication-*` and Pinned Sites

**Description:** A set of `<meta>` tags (`msapplication-TileColor`, `msapplication-TileImage`, `msapplication-notification`, `msapplication-starturl`, `msapplication-task`, etc.) that configure how a site behaves when pinned to the Windows Start menu or taskbar. Predecessor to the more modern Web App Manifest.

**Common on:** Older enterprise sites, sites that invested in Windows integration. Declining in relevance as modern browsers converge on Web App Manifest (`<link rel="manifest">`).

**Data types:** Narrow. Windows tile appearance, pin behavior, and notification frequency.

---

### 11. Pinterest Rich Pins

**Description:** Pinterest uses a mix of OGP, Schema.org, and its own `<meta>` tags to create rich pins. For article pins it consumes `og:title`, `og:description`, `og:image`. For product pins it uses `og:price:amount`, `og:price:currency`, and Schema.org `Product` data. Pinterest-specific tags include `pinterest` meta tags for opting out of pinning (`<meta name="pinterest" content="nopin">`).

**Common on:** E-commerce, recipe sites, article publishers -- any site that benefits from Pinterest referral traffic.

**Data types:** Narrow. Product pricing/availability and article metadata, riding on top of OGP/Schema.org.

---

## Overlap Between Standards

### OGP and Twitter Cards

**Overlap:** Both define `title`, `description`, and `image` for the same page. When `twitter:title` is absent, Twitter's crawler falls back to `og:title` (and similarly for `description` and `image`). Many sites only implement OGP and let Twitter inherit from it.

**Desired outcome:** This overlap is by design. Twitter intentionally falls back to OGP, so maintaining both ensures each platform gets optimized content. When they diverge (e.g. a different image crop for Twitter's timeline vs. Facebook's feed), the site author explicitly sets both.

**Undesired outcome:** When OGP and Twitter tags drift out of sync unintentionally (e.g. one is updated and the other is not), different platforms show stale or contradictory metadata.

### OGP and Schema.org

**Overlap:** Both represent the page's title, description, image, date published, author, and content type. `og:title` overlaps with Schema.org `name`, `og:description` with `description`, `og:image` with `image`, `article:published_time` with `datePublished`.

**Desired outcome:** OGP serves social sharing consumers (Facebook, LinkedIn, Discord, Slack) while Schema.org serves search engines (Google rich results, Bing, Pinterest). Maintaining both is a deliberate practice to serve different consumers with different vocabulary conventions.

**Undesired outcome:** When values diverge, search engines and social platforms present contradictory information. Some scraper consumers may merge both sources and get conflicting values.

### Dublin Core and OGP

**Overlap:** OGP was explicitly inspired by Dublin Core. `dc:title` / `dcterms:title` maps to `og:title`, `dc:description` to `og:description`, `dc:creator` to `article:author`, `dc:date` to `article:published_time`, `dc:identifier` to `og:url`.

**Desired outcome:** Academic or institutional sites that already have Dublin Core metadata can add OGP on top for social sharing without changing their cataloging infrastructure.

**Undesired outcome:** If both are maintained independently, they can drift. Since Dublin Core and OGP serve different communities (catalogers vs. social platforms), drift is often undetected until a stakeholder from one community notices stale data.

### Dublin Core and Schema.org

**Overlap:** Both define bibliographic-style metadata: creator/author, date, subject/keywords, description, language, format, rights. Schema.org is much larger but its core overlaps heavily with Dublin Core's 15 elements.

**Desired outcome:** Academic sites may use Dublin Core for repository cataloging and Schema.org for search engine visibility -- complementary goals.

**Undesired outcome:** Redundant maintenance burden. Some sites choose one or the other to avoid drift.

### OGP / Twitter Cards and App Links

**Overlap:** Both OGP and App Links can point to mobile app content. `og:type` with custom namespaces and `al:ios:url` / `al:android:url` both serve mobile deep-linking. Twitter Cards' `app` card type also overlaps with App Links functionality.

**Desired outcome:** App Links provides richer, platform-specific deep linking while OGP provides the social graph representation. Used together, they cover both social sharing and native app routing.

**Undesired outcome:** Confusion about which system handles deep linking. A page might have contradictory deep link URLs in different metadata systems.

### Microformats and Schema.org

**Overlap:** Both can describe people (h-card vs. Person), events (h-event vs. Event), products (h-product vs. Product), reviews (h-review vs. Review), and recipes (h-recipe vs. Recipe). They encode the same real-world entities using different syntaxes.

**Desired outcome:** Microformats serve the IndieWeb / Fediverse community while Schema.org serves search engines. Some sites publish both for maximum interoperability.

**Undesired outcome:** HTML bloat and maintenance burden. Most modern sites choose Schema.org (JSON-LD) for its broader consumer support.

---

## Follow-On Network Lookups

Several metadata values contain URLs or identifiers that point to additional resources. Fetching these can significantly enrich the scraped dataset:

### oEmbed Endpoint Discovery

**Tag:** `<link rel="alternate" type="application/json+oembed" href="...">`

**Follow-on action:** The `href` attribute contains a full API endpoint URL with query parameters. Making a GET request to that URL returns a JSON (or XML) payload with structured data: title, author name, author URL, provider name, thumbnail URL, embed dimensions, and HTML embed code. This is one of the highest-value follow-on lookups because the response is fully structured and often contains data not present in `<meta>` tags.

### `og:image`, `twitter:image`, and Image Structured Properties

**Tags:** `<meta property="og:image" content="...">`, `<meta property="og:image:width" content="...">`, `<meta name="twitter:image" content="...">`

**Follow-on action:** The image URLs themselves can be fetched to extract dimensions (if not provided in structured properties), perform content analysis, or verify the image still exists. Some sites use dynamic image generators (e.g. `https://example.com/og-image?id=123`) that generate images on the fly -- fetching reveals the actual content.

### `og:url` and `<link rel="canonical">`

**Tags:** `<meta property="og:url" content="...">`, `<link rel="canonical" href="...">`

**Follow-on action:** These point to the authoritative URL for the page. If the scraped URL differs from these, it indicates the current page is an alias, paginated variant, or AMP version. Following the canonical URL may yield a page with richer metadata or different content.

### Schema.org `@id` and Same-As Links

**Tag:** `<script type="application/ld+json">` with `"@id"`, `"sameAs"`, `"url"`, `"identifier"` properties

**Follow-on action:** The `sameAs` array typically contains Wikidata, Wikipedia, LinkedIn, Twitter, and Facebook profile URLs for people/organizations. These can be fetched to cross-reference identity. The `@id` URI can resolve to linked-data endpoints. ISBNs, DOIs, and other identifiers in Schema.org can be resolved through their respective authority databases.

### Dublin Core `identifier`, `relation`, `source`, `references`

**Tags:** `<meta name="DC.identifier" content="...">`, `<meta name="DC.relation" content="...">`, `<meta name="DC.source" content="...">`

**Follow-on action:** These often contain DOIs (`https://doi.org/...`), URNs, or URLs pointing to related scholarly works, datasets, or source materials. Resolving DOIs through doi.org redirects to the publisher's landing page with full citation metadata. `DC.relation` may point to other resources in the same collection.

### Web App Manifest

**Tag:** `<link rel="manifest" href="...">`

**Follow-on action:** The manifest JSON file contains the app's `name`, `short_name`, `icons` (with URLs to app icons at multiple resolutions), `theme_color`, `background_color`, `display` mode, `start_url`, and `scope`. Fetching this provides a richer understanding of the site's identity and brand assets.

### `apple-touch-icon` and Favicon Icons

**Tags:** `<link rel="apple-touch-icon" href="...">`, `<link rel="icon" href="...">`

**Follow-on action:** While these are image URLs rather than metadata APIs, the icon images themselves can be analyzed for brand identification, color extraction, and visual fingerprinting of the site.

### RSS / Atom Feed Discovery

**Tags:** `<link rel="alternate" type="application/rss+xml" href="...">`, `<link rel="alternate" type="application/atom+xml" href="...">`

**Follow-on action:** Fetching the feed URL returns structured XML with recent articles, publish dates, authors, and summaries. This provides a broader view of the site's content beyond the single scraped page.

### `pingback` and `webmention` Endpoints

**Tags:** `<link rel="pingback" href="...">`, `<link rel="webmention" href="...">`

**Follow-on action:** These advertise endpoints that accept notifications when other sites link to this page. While primarily used for sending (not receiving) data, the presence of these endpoints indicates the site participates in the IndieWeb ecosystem, which correlates with richer microformats content.

### App Store / Deep Link Resolution

**Tags:** `<meta name="apple-itunes-app" content="app-id=..., app-argument=...">`, `<meta property="al:ios:url" content="...">`, `<meta property="al:android:package" content="...">`

**Follow-on action:** The iTunes `app-id` can be used to query the iTunes Lookup API (`https://itunes.apple.com/lookup?id={app-id}`) to get the app's name, developer, category, icon URL, and current version. Android package names can be resolved through alternative app store APIs or the Google Play Store page to obtain similar metadata.

### Authorization Endpoints (IndieAuth)

**Tag:** `<link rel="authorization_endpoint" href="...">`, `<link rel="token_endpoint" href="...">`

**Follow-on action:** These IndieAuth endpoints reveal the site owner's identity provider. The authorization endpoint URL can be fetched to discover the authentication mechanisms supported, which sometimes reveals additional profile information about the site author.
