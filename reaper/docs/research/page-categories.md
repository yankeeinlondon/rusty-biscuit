---
prompt: |-
    When evaluating a web page it is often useful to categorize it into one or more categories.

    Your task starts with enumerating the various metadata standards which are used to categorize web pages and report:

    - the standard's name
    - where is it most typically used?
    - what does it describe? 
    - is there any direct mapping of microdata on a page to this standard's schema structure?

    Having enumerated the standards, you will then turn your attention to the actual categorization:

    - what deterministic heuristics are available in categorization?
    - which categorizations can be done relatively accurately with a fast/cheap LLM?
        - which ones are more complicated with an LLM and why

    Finally, investigate whether there are any Rust crates which might help in the area of categorization?
last_updated: 2026-06-03
---
## Web Page Categorization Metadata Standards

| Standard                                                                                               | Typical use                                                                                          | What it describes                                                                                                                                        | Direct microdata mapping?                                                                                                                                                                                                        |
|--------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [Schema.org](https://schema.org/)                                                                      | Search engines, rich results, commerce, recipes, events, organizations, articles, products           | Entity types and properties embedded in a page: `Article`, `Product`, `BreadcrumbList`, `FAQPage`, `Organization`, etc.                                  | Yes. Schema.org explicitly supports Microdata, RDFa, and JSON-LD. Microdata `itemscope`, `itemtype`, and `itemprop` map directly into Schema.org typed objects and properties.                                                   |
| [Open Graph Protocol](https://ogp.me/)                                                                 | Social sharing previews, Facebook-derived crawlers, Slack, Discord, iMessage, LinkedIn-like previews | Share-card identity: title, type, canonical URL, image, description, site name, locale, article tags, media metadata                                     | Not directly. OG uses `<meta property="og:*">` tags, based on RDFa-style properties. It can be converted into a graph-like structure, but it is not Microdata.                                                                   |
| [X / Twitter Cards](https://developer.x.com/cards/getting-started)                                     | X/Twitter link previews and crawlers that copy Twitter Card conventions                              | Card type and preview fields: `summary`, `summary_large_image`, app cards, player cards, title, image, description                                       | No. Twitter Cards use `<meta name="twitter:*">` tags. They describe presentation more than page taxonomy.                                                                                                                        |
| [Dublin Core / DCMI Metadata Terms](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/) | Libraries, archives, institutional repositories, academic publishing, document catalogs              | General resource metadata: title, creator, subject, type, format, coverage, language, rights, relation                                                   | Indirect only. DCMI terms are RDF vocabularies and can be expressed in RDFa, XML, JSON, or HTML meta tags. Microdata can technically use URL-valued properties, but there is no common direct page-to-DCMI Microdata convention. |
| [IAB Tech Lab Content Taxonomy](https://dev.iabtechlab.com/standards/content-taxonomy/)                | Adtech, contextual targeting, brand safety, OpenRTB, SSP/DSP/ad verification workflows               | Topic “aboutness” categories plus orthogonal vectors such as content purpose, format, source, media type, environment, language, and suitability signals | No. IAB categories are usually transmitted as taxonomy IDs in ad protocols such as OpenRTB `cat`/`cattax`, not as Microdata. A crawler may infer IAB IDs from page text or publisher metadata.                                   |
| [IPTC Media Topics / NewsCodes](https://iptc.org/standards/newscodes/)                                 | Newsrooms, wire services, media asset management, editorial syndication                              | Controlled news subject taxonomy for journalism and media content                                                                                        | No direct Microdata mapping. IPTC vocabularies are machine-readable controlled vocabularies, often used in NewsML-G2, ninjs, CMS metadata, or editorial tooling.                                                                 |
| [Microformats2](https://developer.mozilla.org/en-US/docs/Web/HTML/Guides/Microformats)                 | IndieWeb, blogs, feeds, events, people, lightweight semantic HTML                                    | HTML class-based objects such as `h-entry`, `h-card`, `h-event`; categories via properties like `p-category` and `rel=tag`                               | No. Microformats are their own HTML convention using classes and `rel`, not Microdata. They can still provide useful category/tag signals.                                                                                       |
| RSS / Atom categories                                                                                  | Feeds, blogs, podcasts, news syndication                                                             | Entry-level or feed-level categories/tags. Atom defines `atom:category`; RSS 2.0 has `<category>`                                                        | No. These are feed XML elements, not page Microdata. They may be more reliable than the page itself when available.                                                                                                              |
| HTML meta keywords / news keywords                                                                     | Legacy SEO, publisher CMS exports, some news pages                                                   | Free-form keyword strings supplied by the publisher                                                                                                      | No. This is plain HTML metadata. It is cheap to parse but low-trust because it is often stale, spammy, or ignored by major search engines.                                                                                       |

## Practical Interpretation

Schema.org is the strongest page-embedded structure if present. It can describe both the page type and the entities on the page, and Microdata maps directly to its schema structure.

Open Graph and Twitter Cards are useful but mostly presentation-oriented. `og:type=article`, `article:section`, and `article:tag` are categorization signals, but OG rarely provides a full semantic taxonomy.

IAB and IPTC are true categorization taxonomies, but they are usually not embedded directly in public page markup. They are more often applied by publishers, ad systems, editorial systems, or classifiers after page analysis.

## Deterministic Heuristics

Deterministic categorization should be treated as a high-precision signal layer, not a complete classifier.

Useful heuristics include:

- Parse structured data: Schema.org JSON-LD, Microdata, RDFa, OG, Twitter Cards, Dublin Core, microformats.
- Extract declared page type: `schema.org/@type`, `og:type`, article markup, RSS/Atom entry metadata.
- Extract publisher categories: breadcrumbs, nav hierarchy, URL path segments, CMS category links, tags, `article:section`, `article:tag`, `keywords`, `news_keywords`.
- Detect content format: article, product page, listing page, recipe, event, video, podcast, forum thread, documentation page, app store page, profile page.
- Use URL and host patterns: `/news/`, `/blog/`, `/docs/`, `/products/`, `/recipes/`, `/category/sports/`, subdomains, known CMS routes.
- Use DOM structure: `<article>`, product offer blocks, review aggregates, comments, publish dates, author bylines, prices, ratings, recipe instructions, event times.
- Use feed metadata when available: RSS/Atom categories often preserve CMS tags more cleanly than rendered HTML.
- Use canonical and alternate links: AMP pages, syndicated pages, language alternates, feed links.
- Use entity dictionaries: known sports teams, stock tickers, product brands, locations, people, medical terms, legal terms.
- Use taxonomy keyword maps: controlled phrase lists for broad IAB/IPTC candidates.
- Use boilerplate removal before text analysis so nav/footer terms do not dominate classification.
- Score agreement between independent signals. For example, `schema.org/Recipe`, `/recipes/`, recipe instructions, and nutrition fields together should outrank a weak keyword match.

Deterministic systems work best for page type, publisher-declared categories, obvious verticals, language, media format, and strong entity presence. They are weaker for nuanced topic, intent, risk, stance, suitability, and pages with thin or misleading metadata.

## LLM-Assisted Categorization

A fast, cheap LLM can usually do well on:

- Broad topic classification: sports, politics, technology, finance, health, entertainment, travel.
- Page type classification: article, product, landing page, documentation, forum, profile, recipe, event, job posting.
- Intent classification: informational, transactional, navigational, support, opinion, review.
- Short tag generation from cleaned main text.
- Mapping a page to top-level or mid-level taxonomy nodes when labels are descriptive.
- Summarizing why a deterministic classifier chose a category.
- Resolving conflicts between metadata and visible text when the evidence is simple.
- Multi-label classification where a page is clearly about several topics.

LLMs get more complicated when:

- The taxonomy is large and fine-grained. IAB/IPTC-style trees can have many close siblings, and label-only prompting often confuses adjacent categories.
- The decision depends on publisher policy. Brand safety and suitability need calibrated thresholds, not just semantic similarity.
- The page mixes contexts. A news article about regulation of gambling, medical pricing, or extremist activity may mention sensitive entities without endorsing or primarily being about them.
- The classifier must distinguish “about X” from “mentions X.” This is central for contextual advertising and editorial tagging.
- The category requires domain expertise. Legal, medical, financial, scientific, and political content often needs careful interpretation.
- The page is sparse or heavily templated. Product grids, login walls, app pages, and JavaScript-rendered pages may not expose enough text.
- The output must be stable. Cheap LLM calls can vary unless constrained with schemas, few-shot examples, deterministic decoding, and post-validation.
- The taxonomy has privacy implications. Sensitive categories require conservative handling, auditability, and possibly human review.
- The result must be explainable. A black-box label is weaker than an evidence-backed label with source spans.

A practical pipeline is hybrid:

1. Extract deterministic metadata and main text.
2. Normalize signals into candidate labels.
3. Use rules for high-confidence cases.
4. Use a cheap LLM only for ambiguous or semantic mapping cases.
5. Validate the LLM output against an allowed taxonomy.
6. Store evidence: source fields, text spans, confidence, and conflicts.

## Rust Crates That May Help

Useful metadata and extraction crates:

- [`webpage-info`](https://docs.rs/webpage-info/latest/webpage_info/) extracts title, description, Open Graph, Schema.org JSON-LD, links, and common page metadata.
- [`scraper`](https://docs.rs/scraper/) is a practical HTML selector library for custom metadata, breadcrumbs, tags, and DOM heuristics.
- [`html5ever`](https://docs.rs/html5ever/) and [`kuchiki`](https://docs.rs/kuchiki/) help when lower-level or DOM-like HTML parsing is needed.
- [`microformats`](https://docs.rs/microformats/latest/microformats/) parses Microformats2 from HTML into structured documents.
- [`json-ld`](https://docs.rs/json-ld/latest/json_ld/) and [`oxjsonld`](https://docs.rs/oxjsonld/) help process JSON-LD into linked-data structures.
- [`oxigraph`](https://docs.rs/oxigraph/) and [`sophia`](https://docs.rs/sophia/) are useful if categorization needs RDF graph storage/querying.
- [`open-graph`](https://docs.rs/open-graph/) can help parse Open Graph metadata specifically.
- [`readability-js`](https://docs.rs/readability-js/), [`readable-rs`](https://docs.rs/readable-rs/), [`legible`](https://docs.rs/legible/), and [`readability`](https://docs.rs/readability-rs/latest/readability/) can extract the main readable content before classification.

Useful classification and NLP crates:

- [`rust-bert`](https://docs.rs/rust-bert/latest/rust_bert/pipelines/zero_shot_classification/) includes a zero-shot classification pipeline suitable for candidate-label classification, though it brings heavier model dependencies.
- [`fasttext`](https://docs.rs/fasttext/) / [`fast_text`](https://docs.rs/fast_text/) can support fast supervised text classification if trained on labeled examples.
- [`candle`](https://docs.rs/candle-core/) can run local transformer models when you want Rust-native inference building blocks.
- [`ort`](https://docs.rs/ort/) can run ONNX models for embeddings or classifiers.
- [`tokenizers`](https://docs.rs/tokenizers/) is useful for preparing text for transformer-based classifiers.
- [`whatlang`](https://docs.rs/whatlang/) or CLD-style crates can classify language before routing to language-specific taxonomies.
- [`tantivy`](https://docs.rs/tantivy/) can support keyword, BM25, and evidence retrieval against taxonomy labels or training corpora.

There does not appear to be a single mature Rust crate that takes an arbitrary web page and returns IAB/IPTC/Schema.org-style categories end to end. The likely Rust architecture is a composition of HTML extraction, structured metadata parsing, readability extraction, deterministic scoring, and either a local classifier or external LLM call for semantic mapping.
