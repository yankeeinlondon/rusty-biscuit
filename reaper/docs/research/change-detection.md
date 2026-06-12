---
prompt: |-
    When evaluating a web page, it would be useful to be able to hash a number of characteristics about a page so that later we can compare these hashes and have a reasonable "explanation" for what has changed on the page.

    Your task is to:

    - identify a set of hashable things on webpages which could be useful in explaining change
        - content hash, DOM slices whose content is hashed, link URLs, image URLs, CSS used, meta tags, a subset of meta tags, etc.
    - come up with as many things to hash and for each specify what this hash would "mean" and how it could be used in an "explanation" of a page's change

    Once we have a full inventory of hashable things, then:

    - come up with 2-3 strategies for leveraging some/all of these hashes and describe each approaches pros/cons.
last_updated: 2026-06-03
---
## Page Change Hash Inventory

The goal is to hash page characteristics at multiple levels so later comparisons can explain *what kind* of change occurred, not only that a page changed.

A single page fingerprint should not be treated as one opaque hash. It should be a collection of named hashes with stable meanings.

## Core Page Hashes

| Hash                       | What Is Hashed                                                                                              | What It Means                                                     | Explanation Use                                                  |
|----------------------------|-------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------|------------------------------------------------------------------|
| `document_html_hash`       | Raw fetched HTML bytes after transport decoding                                                             | The delivered HTML document changed byte-for-byte                 | "The server returned different HTML."                            |
| `normalized_html_hash`     | HTML after parser normalization, whitespace normalization, and removal of volatile attributes if configured | The structural HTML changed, ignoring irrelevant formatting noise | "The page markup changed."                                       |
| `rendered_text_hash`       | Visible text extracted from the rendered DOM                                                                | User-visible textual content changed                              | "The visible page text changed."                                 |
| `main_content_text_hash`   | Visible text from the detected main content region                                                          | Primary content changed                                           | "The article/product/body content changed."                      |
| `text_blocks_hash`         | Ordered list of normalized visible text blocks                                                              | Text changed at the block/paragraph level                         | "One or more visible text sections changed."                     |
| `text_word_set_hash`       | Sorted unique normalized words/tokens                                                                       | Vocabulary changed, independent of order                          | "The set of words used on the page changed."                     |
| `text_numeric_values_hash` | Extracted numbers, prices, dates, percentages, counts                                                       | Numeric facts changed                                             | "A price, date, count, or numeric value changed."                |
| `text_entities_hash`       | Extracted names, organizations, locations, products, dates                                                  | Named entities changed                                            | "The people, places, dates, or organizations mentioned changed." |
| `page_title_hash`          | `<title>` text                                                                                              | Browser/search title changed                                      | "The page title changed."                                        |
| `h1_hash`                  | All visible `h1` text                                                                                       | Primary heading changed                                           | "The main heading changed."                                      |
| `headings_hash`            | Ordered `h1`-`h6` text and levels                                                                           | Page outline changed                                              | "The page's section structure changed."                          |

## DOM Structure Hashes

| Hash                            | What Is Hashed                                               | What It Means                              | Explanation Use                                  |
|---------------------------------|--------------------------------------------------------------|--------------------------------------------|--------------------------------------------------|
| `dom_tree_shape_hash`           | DOM node tree with tag names only                            | Structural layout changed                  | "The page's DOM structure changed."              |
| `dom_tree_with_attributes_hash` | DOM tree with selected attributes                            | Structure or meaningful attributes changed | "The page markup or element attributes changed." |
| `semantic_dom_hash`             | Tags, roles, ARIA labels, headings, landmarks, forms         | Semantic/accessibility structure changed   | "The semantic page structure changed."           |
| `body_structure_hash`           | DOM shape under `<body>`                                     | Visible document structure changed         | "The body layout changed."                       |
| `main_region_dom_hash`          | DOM shape under `<main>` or detected main container          | Primary content structure changed          | "The main content area was reorganized."         |
| `nav_dom_hash`                  | DOM under navigation regions                                 | Navigation changed                         | "The site's navigation changed."                 |
| `footer_dom_hash`               | DOM under footer regions                                     | Footer/legal/sitewide content changed      | "The footer changed."                            |
| `header_dom_hash`               | DOM under header/banner regions                              | Header or masthead changed                 | "The page header changed."                       |
| `form_dom_hash`                 | Forms, fields, labels, buttons, methods, actions             | User input surface changed                 | "A form changed."                                |
| `table_structure_hash`          | Table captions, headers, row/column counts                   | Tabular structure changed                  | "A table changed shape."                         |
| `list_structure_hash`           | Ordered/unordered list lengths and item text hashes          | Lists changed                              | "A list of items changed."                       |
| `button_text_hash`              | Visible button text and accessible names                     | Calls to action changed                    | "Buttons or actions changed."                    |
| `interactive_elements_hash`     | Links, buttons, inputs, selects, textareas, details, dialogs | Interactive surface changed                | "The set of interactive controls changed."       |
| `aria_roles_hash`               | ARIA roles and landmark roles                                | Accessibility semantics changed            | "ARIA roles or landmarks changed."               |
| `aria_labels_hash`              | `aria-label`, `aria-labelledby`, accessible names            | Accessibility labels changed               | "Accessible names changed."                      |
| `data_attributes_hash`          | Selected `data-*` attributes, optionally allowlisted         | App-specific state markers changed         | "Application state markers changed."             |
| `element_id_set_hash`           | Sorted element IDs                                           | Addressable anchors/components changed     | "Element IDs changed."                           |
| `class_set_hash`                | Sorted CSS class names                                       | Styling/component identifiers changed      | "CSS classes changed."                           |
| `class_usage_hash`              | Tag/class combinations and counts                            | Component usage changed                    | "The page appears to use different components."  |

## DOM Slice Hashes

DOM slice hashes are useful because they identify *where* change happened.

| Hash                               | What Is Hashed                                            | What It Means                      | Explanation Use                         |
|------------------------------------|-----------------------------------------------------------|------------------------------------|-----------------------------------------|
| `slice_by_selector_hashes`         | Hash per configured CSS selector                          | Specific monitored regions changed | "The `.price` region changed."          |
| `slice_by_heading_hashes`          | Content grouped under each heading                        | Section-level content changed      | "The 'Specifications' section changed." |
| `slice_by_landmark_hashes`         | Content grouped by `main`, `nav`, `aside`, `footer`, etc. | Landmark-level change              | "The sidebar changed."                  |
| `slice_by_xpath_hashes`            | Stable XPath-selected regions                             | Known page regions changed         | "The monitored DOM path changed."       |
| `slice_by_text_density_hashes`     | High text-density blocks                                  | Article-like regions changed       | "A dense content block changed."        |
| `slice_by_component_hashes`        | Repeated card/product/result/listing components           | Repeated item sets changed         | "One or more product cards changed."    |
| `slice_by_viewport_hashes`         | Elements visible above/below fold after render            | Viewport-level change              | "Above-the-fold content changed."       |
| `slice_by_dom_depth_hashes`        | Nodes grouped by depth or subtree                         | Coarse structural areas changed    | "A deep nested region changed."         |
| `slice_by_repeated_pattern_hashes` | Detected repeated DOM patterns                            | Search results/listings changed    | "The result list changed."              |
| `slice_by_stable_anchor_hashes`    | Regions anchored by IDs, headings, or labels              | Stable named sections changed      | "The anchored section changed."         |

## Link Hashes

| Hash                                    | What Is Hashed                                  | What It Means                                      | Explanation Use                                          |
|-----------------------------------------|-------------------------------------------------|----------------------------------------------------|----------------------------------------------------------|
| `link_url_set_hash`                     | Sorted normalized link URLs                     | Link destination set changed                       | "The page links to different URLs."                      |
| `link_url_ordered_hash`                 | Ordered normalized link URLs                    | Link order changed                                 | "The order of links changed."                            |
| `internal_link_hash`                    | Same-origin links                               | Internal navigation changed                        | "Internal links changed."                                |
| `external_link_hash`                    | Cross-origin links                              | External references changed                        | "External links changed."                                |
| `link_text_hash`                        | Link accessible text                            | Link labels changed                                | "Link text changed."                                     |
| `link_text_url_pair_hash`               | Normalized `(text, href)` pairs                 | Link meaning changed                               | "A link label now points somewhere else, or vice versa." |
| `canonical_link_hash`                   | `<link rel="canonical">`                        | Canonical URL changed                              | "The canonical page URL changed."                        |
| `alternate_link_hash`                   | `<link rel="alternate">`, hreflang, feeds       | Alternate versions changed                         | "Alternate language/feed URLs changed."                  |
| `pagination_link_hash`                  | `rel=prev`, `rel=next`, pagination controls     | Pagination changed                                 | "Pagination links changed."                              |
| `download_link_hash`                    | Links to files/assets by extension or MIME hint | Downloadable resources changed                     | "Download links changed."                                |
| `mailto_tel_link_hash`                  | `mailto:` and `tel:` URLs                       | Contact links changed                              | "Contact information links changed."                     |
| `tracking_parameter_stripped_link_hash` | URLs after removing known tracking params       | Meaningful links changed, ignoring analytics noise | "The effective link destinations changed."               |

## Image and Media Hashes

| Hash                         | What Is Hashed                                                     | What It Means                                   | Explanation Use                          |
|------------------------------|--------------------------------------------------------------------|-------------------------------------------------|------------------------------------------|
| `image_url_set_hash`         | Sorted normalized image URLs from `src`, `srcset`, CSS backgrounds | Referenced images changed                       | "Images changed."                        |
| `image_url_ordered_hash`     | Ordered image URLs                                                 | Image placement/order changed                   | "Image order changed."                   |
| `image_alt_text_hash`        | Image alt text                                                     | Image accessibility descriptions changed        | "Image descriptions changed."            |
| `image_src_alt_pair_hash`    | `(src, alt)` pairs                                                 | Image meaning changed                           | "An image or its description changed."   |
| `hero_image_hash`            | Detected prominent/above-fold image URL or bytes                   | Primary visual changed                          | "The hero image changed."                |
| `og_image_hash`              | Open Graph image URL                                               | Social preview image changed                    | "The social sharing image changed."      |
| `favicon_hash`               | Favicon URLs or fetched icon bytes                                 | Site identity icon changed                      | "The favicon changed."                   |
| `video_url_hash`             | Video source/poster URLs                                           | Video assets changed                            | "Video content changed."                 |
| `audio_url_hash`             | Audio source URLs                                                  | Audio assets changed                            | "Audio content changed."                 |
| `media_caption_hash`         | Captions near media elements                                       | Media descriptions changed                      | "Media captions changed."                |
| `image_dimensions_hash`      | Declared or natural dimensions                                     | Image sizing changed                            | "Image dimensions changed."              |
| `fetched_image_bytes_hashes` | Optional per-image content hashes after fetching assets            | Actual image files changed even if URLs did not | "An image file changed at the same URL." |

## CSS and Visual Presentation Hashes

| Hash                               | What Is Hashed                                           | What It Means                        | Explanation Use                             |
|------------------------------------|----------------------------------------------------------|--------------------------------------|---------------------------------------------|
| `stylesheet_url_hash`              | Linked stylesheet URLs                                   | CSS dependencies changed             | "Stylesheet references changed."            |
| `inline_style_hash`                | Inline `<style>` contents                                | Embedded CSS changed                 | "Inline CSS changed."                       |
| `style_attribute_hash`             | `style=""` attributes                                    | Element-level inline styling changed | "Inline element styles changed."            |
| `css_rule_hash`                    | Parsed CSS rules after normalization                     | CSS behavior changed                 | "CSS rules changed."                        |
| `css_selector_hash`                | CSS selectors only                                       | Styled target set changed            | "CSS now targets different elements."       |
| `css_custom_property_hash`         | CSS variables                                            | Design tokens changed                | "CSS variables changed."                    |
| `css_color_palette_hash`           | Extracted colors                                         | Visual color palette changed         | "The color palette changed."                |
| `css_font_hash`                    | Font families, font-face URLs, typography properties     | Typography changed                   | "Fonts or typography changed."              |
| `css_layout_property_hash`         | Display/grid/flex/position/spacing-related properties    | Layout behavior changed              | "Layout CSS changed."                       |
| `computed_style_above_fold_hash`   | Selected computed styles for visible elements above fold | Rendered visual styling changed      | "Above-the-fold styling changed."           |
| `computed_style_key_elements_hash` | Computed styles for monitored selectors                  | Important component styling changed  | "A watched component's styling changed."    |
| `css_media_query_hash`             | Media queries                                            | Responsive behavior changed          | "Responsive breakpoints changed."           |
| `css_animation_hash`               | Keyframes, transitions, animation declarations           | Motion behavior changed              | "Animation or transition behavior changed." |
| `critical_css_hash`                | CSS affecting above-fold nodes                           | Initial visual rendering changed     | "Critical rendering CSS changed."           |
| `fetched_stylesheet_bytes_hashes`  | Per-stylesheet content hashes                            | External CSS file content changed    | "A stylesheet file changed."                |

## Metadata Hashes

| Hash                     | What Is Hashed                                           | What It Means                        | Explanation Use                       |
|--------------------------|----------------------------------------------------------|--------------------------------------|---------------------------------------|
| `meta_all_hash`          | All `<meta>` name/property/http-equiv/content pairs      | Metadata changed                     | "Page metadata changed."              |
| `meta_description_hash`  | Description meta tag                                     | Search snippet changed               | "The meta description changed."       |
| `meta_robots_hash`       | Robots directives                                        | Indexing policy changed              | "Search indexing directives changed." |
| `meta_viewport_hash`     | Viewport meta tag                                        | Mobile viewport behavior changed     | "Viewport settings changed."          |
| `meta_charset_hash`      | Charset declaration                                      | Encoding declaration changed         | "Document charset changed."           |
| `meta_refresh_hash`      | Refresh/redirect metadata                                | Client-side refresh behavior changed | "Meta refresh behavior changed."      |
| `open_graph_hash`        | `og:*` tags                                              | Social preview metadata changed      | "Open Graph metadata changed."        |
| `twitter_card_hash`      | `twitter:*` tags                                         | Twitter/X preview metadata changed   | "Twitter card metadata changed."      |
| `schema_org_jsonld_hash` | Parsed JSON-LD structured data                           | Structured data changed              | "Schema.org structured data changed." |
| `microdata_hash`         | HTML microdata attributes/items                          | Embedded structured data changed     | "Microdata changed."                  |
| `rdfa_hash`              | RDFa attributes                                          | RDFa structured data changed         | "RDFa metadata changed."              |
| `seo_metadata_hash`      | Title, description, canonical, robots, hreflang          | SEO-relevant metadata changed        | "SEO metadata changed."               |
| `social_metadata_hash`   | Open Graph, Twitter card, social image/title/description | Social sharing metadata changed      | "Social preview metadata changed."    |

## Script and Behavior Hashes

| Hash                           | What Is Hashed                                                    | What It Means                           | Explanation Use                          |
|--------------------------------|-------------------------------------------------------------------|-----------------------------------------|------------------------------------------|
| `script_url_hash`              | External script URLs                                              | JavaScript dependencies changed         | "Script references changed."             |
| `inline_script_hash`           | Inline script bodies after normalization                          | Embedded JavaScript changed             | "Inline scripts changed."                |
| `script_type_hash`             | Script types, modules, async/defer flags                          | Script loading behavior changed         | "Script loading changed."                |
| `event_handler_attribute_hash` | `onclick`, `onchange`, etc.                                       | Inline event behavior changed           | "Inline event handlers changed."         |
| `json_data_script_hash`        | Non-executable JSON script tags                                   | Embedded page data changed              | "Embedded JSON data changed."            |
| `hydration_data_hash`          | Framework data blobs such as `__NEXT_DATA__`, Remix, Nuxt, Apollo | App/server-rendered state changed       | "Hydration data changed."                |
| `fetched_script_bytes_hashes`  | Per-script content hashes after fetching external JS              | Actual JavaScript file changed          | "A script file changed."                 |
| `script_global_config_hash`    | Detected global config assignments                                | Runtime configuration changed           | "Client-side configuration changed."     |
| `analytics_tag_hash`           | Analytics/ad/tracking script references and IDs                   | Tracking setup changed                  | "Analytics or advertising tags changed." |
| `modulepreload_hash`           | Module preload URLs                                               | JavaScript module loading graph changed | "Preloaded modules changed."             |

## Network and Resource Hashes

| Hash                     | What Is Hashed                                            | What It Means                            | Explanation Use                                               |
|--------------------------|-----------------------------------------------------------|------------------------------------------|---------------------------------------------------------------|
| `resource_url_set_hash`  | All fetched resource URLs                                 | Resource dependency set changed          | "The page loads different resources."                         |
| `resource_domain_hash`   | Domains contacted by the page                             | Third-party dependency footprint changed | "The page contacts different domains."                        |
| `resource_type_hash`     | Counts and URLs by resource type                          | Resource mix changed                     | "The page loads a different mix of scripts/images/fonts/etc." |
| `font_url_hash`          | Font asset URLs                                           | Font resources changed                   | "Font resources changed."                                     |
| `preload_prefetch_hash`  | `preload`, `prefetch`, `preconnect`, `dns-prefetch` links | Loading hints changed                    | "Resource loading hints changed."                             |
| `iframe_url_hash`        | Iframe source URLs                                        | Embedded third-party content changed     | "Embedded frames changed."                                    |
| `worker_url_hash`        | Service worker or web worker URLs                         | Worker behavior changed                  | "Worker scripts changed."                                     |
| `manifest_url_hash`      | Web app manifest URL and optionally fetched manifest      | PWA manifest changed                     | "Web app manifest changed."                                   |
| `sitemap_feed_link_hash` | RSS, Atom, sitemap references                             | Discovery/feed links changed             | "Feed or sitemap references changed."                         |

## HTTP and Fetch Context Hashes

| Hash                      | What Is Hashed                                              | What It Means                             | Explanation Use                               |
|---------------------------|-------------------------------------------------------------|-------------------------------------------|-----------------------------------------------|
| `final_url_hash`          | Final URL after redirects                                   | Redirect target changed                   | "The page now resolves to a different URL."   |
| `redirect_chain_hash`     | Ordered redirect URLs and status codes                      | Redirect behavior changed                 | "The redirect chain changed."                 |
| `status_code_hash`        | HTTP status code                                            | Availability/status changed               | "The HTTP status changed."                    |
| `content_type_hash`       | `Content-Type` header                                       | Returned media type changed               | "The response content type changed."          |
| `cache_header_hash`       | Cache-related headers                                       | Caching policy changed                    | "Cache policy changed."                       |
| `security_header_hash`    | CSP, HSTS, X-Frame-Options, Referrer-Policy, etc.           | Security policy changed                   | "Security headers changed."                   |
| `etag_hash`               | ETag header                                                 | Server-provided content version changed   | "The server ETag changed."                    |
| `last_modified_hash`      | Last-Modified header                                        | Server-provided modification time changed | "The server reports a new modification time." |
| `content_length_hash`     | Content-Length header                                       | Response size changed                     | "The response size changed."                  |
| `http_header_subset_hash` | Configured allowlist of headers                             | Important headers changed                 | "Selected HTTP headers changed."              |
| `cookie_set_hash`         | Set-Cookie names and selected attributes, not secret values | Cookie behavior changed                   | "Cookie names or policies changed."           |

## Layout and Rendering Hashes

| Hash                           | What Is Hashed                                                 | What It Means                               | Explanation Use                                       |
|--------------------------------|----------------------------------------------------------------|---------------------------------------------|-------------------------------------------------------|
| `screenshot_hash`              | Full-page screenshot bytes or perceptual hash                  | Visual rendering changed                    | "The rendered page changed visually."                 |
| `viewport_screenshot_hash`     | Screenshot for a specific viewport                             | Viewport-specific visual change             | "The desktop/mobile rendering changed."               |
| `above_fold_screenshot_hash`   | Screenshot of first viewport                                   | Initial view changed                        | "Above-the-fold content changed visually."            |
| `perceptual_screenshot_hash`   | Perceptual image hash                                          | Visual change tolerant of small pixel noise | "The page looks meaningfully different."              |
| `layout_box_hash`              | Bounding boxes for important elements                          | Layout geometry changed                     | "Element positions or sizes changed."                 |
| `layout_shift_risk_hash`       | Late-loading elements, dimensions, injected content indicators | Potential layout stability changed          | "Layout stability characteristics changed."           |
| `viewport_visible_text_hash`   | Text visible in a specific viewport                            | Initial visible text changed                | "The text visible on load changed."                   |
| `z_index_stack_hash`           | Z-index and positioned elements                                | Overlay behavior changed                    | "Layering or overlay behavior changed."               |
| `scroll_height_hash`           | Document dimensions                                            | Page length changed                         | "The page became longer or shorter."                  |
| `responsive_breakpoint_hashes` | Rendering hashes across multiple viewport sizes                | Responsive behavior changed                 | "The page changed differently across viewport sizes." |

## Accessibility Hashes

| Hash                         | What Is Hashed                               | What It Means                           | Explanation Use                         |
|------------------------------|----------------------------------------------|-----------------------------------------|-----------------------------------------|
| `accessibility_tree_hash`    | Browser accessibility tree                   | Assistive technology experience changed | "The accessibility tree changed."       |
| `accessible_name_hash`       | Accessible names of controls/links/images    | Control labeling changed                | "Accessible labels changed."            |
| `landmark_hash`              | Accessibility landmarks                      | Page navigation regions changed         | "Accessibility landmarks changed."      |
| `heading_accessibility_hash` | Heading hierarchy from accessibility tree    | Assistive heading navigation changed    | "Accessible heading structure changed." |
| `form_label_hash`            | Form labels, names, required state           | Form accessibility changed              | "Form labels or requirements changed."  |
| `alt_missing_count_hash`     | Count/list of images missing alt             | Image accessibility changed             | "Image alt coverage changed."           |
| `aria_state_hash`            | ARIA expanded/selected/checked/hidden states | Initial component states changed        | "ARIA state changed."                   |
| `tab_order_hash`             | Focusable elements in tab order              | Keyboard navigation changed             | "Tab order changed."                    |

## Data and Embedded State Hashes

| Hash                       | What Is Hashed                                         | What It Means                          | Explanation Use                      |
|----------------------------|--------------------------------------------------------|----------------------------------------|--------------------------------------|
| `jsonld_graph_hash`        | Parsed JSON-LD graph normalized by keys                | Structured entity data changed         | "Structured entity data changed."    |
| `embedded_json_hash`       | All parseable JSON blobs in scripts/attributes         | Embedded machine-readable data changed | "Embedded JSON changed."             |
| `state_blob_hash`          | Framework state payloads                               | Client app state changed               | "Application state payload changed." |
| `product_data_hash`        | Detected product name, price, availability, SKU        | Product facts changed                  | "Product data changed."              |
| `article_data_hash`        | Detected author, publish date, modified date, headline | Article facts changed                  | "Article metadata changed."          |
| `event_data_hash`          | Detected event date, location, price, availability     | Event facts changed                    | "Event data changed."                |
| `breadcrumb_hash`          | Breadcrumb text and URLs                               | Page hierarchy changed                 | "Breadcrumbs changed."               |
| `search_result_items_hash` | Detected result cards/items                            | Search/listing results changed         | "The result set changed."            |
| `commerce_offer_hash`      | Offers, prices, currency, availability                 | Commercial offer changed               | "Price or availability changed."     |

## Counts and Summary Hashes

| Hash                    | What Is Hashed                     | What It Means                  | Explanation Use                                   |
|-------------------------|------------------------------------|--------------------------------|---------------------------------------------------|
| `element_count_hash`    | Counts by tag name                 | Structural composition changed | "The number of page elements changed."            |
| `word_count_hash`       | Total visible word count           | Text volume changed            | "The amount of text changed."                     |
| `link_count_hash`       | Number of links by type            | Link volume changed            | "The number of links changed."                    |
| `image_count_hash`      | Number of images                   | Image volume changed           | "The number of images changed."                   |
| `form_count_hash`       | Number of forms and controls       | Input surface size changed     | "The number of forms or fields changed."          |
| `script_count_hash`     | Number of scripts by type          | Script footprint changed       | "The number of scripts changed."                  |
| `stylesheet_count_hash` | Number of stylesheets/style blocks | CSS footprint changed          | "The amount of CSS changed."                      |
| `resource_count_hash`   | Counts by resource type            | Resource footprint changed     | "The page loads a different number of resources." |
| `dom_depth_hash`        | Maximum/average DOM depth          | Structural complexity changed  | "DOM nesting changed."                            |
| `dom_size_hash`         | Number of DOM nodes                | Page complexity changed        | "DOM size changed."                               |

## Security and Policy Hashes

| Hash                      | What Is Hashed                                       | What It Means                               | Explanation Use                        |
|---------------------------|------------------------------------------------------|---------------------------------------------|----------------------------------------|
| `csp_hash`                | Content Security Policy header/meta                  | Executable/resource security policy changed | "Content Security Policy changed."     |
| `permissions_policy_hash` | Permissions-Policy header                            | Browser feature permissions changed         | "Permissions policy changed."          |
| `referrer_policy_hash`    | Referrer policy header/meta                          | Referrer behavior changed                   | "Referrer policy changed."             |
| `robots_policy_hash`      | Robots meta and robots-related headers               | Crawling/indexing policy changed            | "Robots policy changed."               |
| `mixed_content_risk_hash` | HTTP resources on HTTPS pages                        | Mixed content exposure changed              | "Mixed-content risk changed."          |
| `third_party_script_hash` | Third-party script origins                           | Third-party executable dependencies changed | "Third-party scripts changed."         |
| `privacy_surface_hash`    | Trackers, pixels, analytics domains, consent scripts | Privacy/tracking surface changed            | "Privacy or tracking surface changed." |

## Normalization Choices

Each hash should define its normalization rules. Otherwise the explanation becomes unreliable.

Useful normalization options:

| Normalization                                  | Purpose                                                      |
|------------------------------------------------|--------------------------------------------------------------|
| Trim and collapse whitespace                   | Ignore formatting-only differences                           |
| Lowercase tag and attribute names              | Avoid parser/source casing differences                       |
| Sort unordered sets                            | Make set hashes independent of document order                |
| Preserve order for ordered hashes              | Detect reordering when order matters                         |
| Strip volatile query params                    | Ignore analytics/cache-busting noise                         |
| Keep volatile query params                     | Detect asset version changes                                 |
| Remove timestamps/nonces/random IDs            | Reduce false positives                                       |
| Preserve timestamps/nonces/random IDs          | Detect deployment/runtime variation                          |
| Resolve relative URLs                          | Compare URLs consistently                                    |
| Canonicalize URLs                              | Normalize scheme, host casing, default ports, fragments      |
| Parse CSS/JSON before hashing                  | Avoid formatting-only differences                            |
| Hash fetched assets separately                 | Distinguish URL reference changes from asset content changes |
| Use selector allowlists                        | Focus on meaningful regions                                  |
| Use selector denylists                         | Exclude ads, personalization, cookie banners, timestamps     |
| Include viewport and user-agent in hash key    | Keep render hashes comparable                                |
| Include locale/auth/device context in hash key | Avoid comparing incompatible fetch contexts                  |

## Recommended Hash Record Shape

A useful record should include more than the digest:

```text
name
digest
algorithm
normalization_version
source
scope
dependencies
observed_at
fetch_context
explanation_template
```

Example:

```text
name: main_content_text_hash
digest: xxh3:...
algorithm: xxh3-128
normalization_version: visible-text-v2
source: rendered_dom
scope: main
dependencies: [url, viewport, user_agent, locale]
explanation_template: "The main visible content changed."
```

This makes hashes explainable, comparable, and upgradable over time.

## Strategy 1: Layered Fingerprint

Hash the page at several layers:

```text
transport -> document -> DOM -> text -> metadata -> resources -> rendering
```

A comparison first checks broad hashes, then drills into narrower hashes when broad hashes changed.

Example explanation flow:

```text
The page changed.
The visible text did not change.
The DOM structure changed.
The stylesheet URLs changed.
Likely explanation: presentation or layout changed, but user-visible text stayed the same.
```

Pros:

| Pro                  | Details                                                      |
|----------------------|--------------------------------------------------------------|
| Easy to reason about | Changes are grouped by natural page layers                   |
| Good explanations    | Can distinguish text, layout, metadata, and resource changes |
| Efficient            | Expensive render/resource hashes can be optional             |
| Flexible             | Works for static and dynamic pages                           |

Cons:

| Con                         | Details                                                 |
|-----------------------------|---------------------------------------------------------|
| Less precise by default     | Broad layer hashes may not identify exact changed nodes |
| Requires good normalization | Poor normalization creates noisy explanations           |
| Some layers are expensive   | Rendering and fetched asset hashes cost more            |

Best for:

```text
General-purpose page monitoring where explanations should be concise and reliable.
```

## Strategy 2: Region-Oriented Fingerprint

Hash meaningful page regions independently:

```text
header
navigation
main content
article body
product card
price block
reviews
sidebar
footer
metadata
resources
```

Regions can be selected by semantic landmarks, configured selectors, headings, or detected repeated components.

Example explanation flow:

```text
The page changed.
The main content hash changed.
The header, navigation, and footer hashes did not change.
The changed region contains different numeric values.
Likely explanation: the page's primary content changed, possibly price/date/count-related.
```

Pros:

| Pro                                 | Details                                         |
|-------------------------------------|-------------------------------------------------|
| Strong localized explanations       | Identifies where change happened                |
| Useful for noisy pages              | Ads, nav, and footer can be isolated or ignored |
| Good for products/articles/listings | Important sections can be monitored directly    |
| Supports targeted alerts            | Alert only when watched regions change          |

Cons:

| Con                      | Details                                           |
|--------------------------|---------------------------------------------------|
| Needs page understanding | Region detection can be brittle                   |
| Selector maintenance     | Site redesigns can break configured selectors     |
| Harder to generalize     | Different page types need different region models |

Best for:

```text
Known websites, product pages, articles, dashboards, search result pages, and monitored business-critical regions.
```

## Strategy 3: Explanation-First Diff Classifier

Compute many hashes, then classify changes into explanation categories.

Example categories:

```text
content_changed
main_content_changed
metadata_changed
seo_changed
social_preview_changed
links_changed
images_changed
style_changed
layout_changed
script_behavior_changed
security_policy_changed
tracking_changed
availability_changed
```

Each category is backed by one or more hashes.

Example explanation flow:

```text
The page changed in three ways:
1. SEO metadata changed because the title and meta description hashes changed.
2. Social preview changed because Open Graph tags changed.
3. Visible content did not change because rendered text and main content hashes are unchanged.
```

Pros:

| Pro                           | Details                                             |
|-------------------------------|-----------------------------------------------------|
| Best user-facing explanations | Outputs are framed as meaningful change categories  |
| Handles many signals          | Can combine weak signals into stronger conclusions  |
| Prioritizes importance        | Content changes can outrank tracking or cache noise |
| Extensible                    | New hash types can map into existing categories     |

Cons:

| Con                      | Details                                                              |
|--------------------------|----------------------------------------------------------------------|
| More implementation work | Requires a classifier/rules layer                                    |
| Needs tuning             | Category weights and precedence must be calibrated                   |
| Can overstate certainty  | Explanations should say "likely" when inferred from indirect signals |

Best for:

```text
Monitoring systems that need human-readable change summaries rather than raw hash comparisons.
```

## Practical Recommendation

Use a hybrid approach:

1. Store a layered fingerprint for every page.
2. Add region hashes for known or high-value page types.
3. Run an explanation classifier over changed hashes.

A minimal useful baseline would include:

```text
final_url_hash
status_code_hash
normalized_html_hash
rendered_text_hash
main_content_text_hash
headings_hash
link_url_set_hash
image_url_set_hash
meta_all_hash
seo_metadata_hash
stylesheet_url_hash
script_url_hash
resource_domain_hash
screenshot_hash or perceptual_screenshot_hash
```

A richer explainable fingerprint would add:

```text
dom_tree_shape_hash
semantic_dom_hash
slice_by_landmark_hashes
slice_by_heading_hashes
text_numeric_values_hash
text_entities_hash
open_graph_hash
twitter_card_hash
schema_org_jsonld_hash
css_rule_hash
computed_style_above_fold_hash
accessibility_tree_hash
security_header_hash
third_party_script_hash
fetched_asset_hashes
```

The most important design principle is to avoid a single page hash. A page should have many named hashes, each with a clear scope and explanation meaning.
