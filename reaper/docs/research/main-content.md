---
prompt: |-
    When evaluating a web page, there is a subset of the page's DOM that represents the "main content" versus things like footers, headers, asides, navigation elements, etc.

    Your task is to do a deep dive into deterministic approaches that could be used to identify the page's main content. While these approaches might be deterministic, they are likely going to be probabilistic in their assessments. You should be able to report on:

    - 5-10 algorithmic appoarches to parsing the DOM that might yield a reasonable statistical analysis of what the "main content" is:
        - give each algorithm a name
        - describe it's approach
        - call out pro's and con's as well as what types of content structures it's good at versus bad at
    - the top 2-3 Rust crates which provide functionality that overlap with this functionality
        - describe the approach each crate takes for their analysis
    - is there any corpus of knowledge that could be built up over time to improve these algorithms? 
last_updated: 2026-06-03
---
## Deterministic Main-Content Detection Approaches

"Main content" extraction is usually deterministic in implementation but probabilistic in result: the algorithm walks the same DOM and produces the same score each time, but the score is a heuristic estimate of authorial intent. A strong extractor usually combines several signals instead of trusting one.

### 1. Landmark-First Extraction

**Approach:** Prefer semantic containers such as `<main>`, `<article>`, `[role="main"]`, schema.org `Article`/`NewsArticle`, and OpenGraph/JSON-LD metadata. Penalize `<nav>`, `<aside>`, `<footer>`, `<header>`, forms, dialogs, cookie banners, and repeated site chrome.

**Pros:**

- Fast and explainable.
- Excellent on modern, accessible, well-authored pages.
- Preserves intended content boundaries when authors use semantic HTML correctly.

**Cons:**

- Fails when sites misuse `<article>` for cards, previews, comments, or product tiles.
- Many pages omit semantic landmarks or wrap the entire layout in `<main>`.
- Can miss multi-part content split across several sibling containers.

**Good at:** Documentation pages, blogs, news articles, government pages, accessibility-conscious sites.

**Bad at:** Legacy CMS output, ecommerce pages, infinite feeds, badly templated marketing pages.

### 2. Readability-Style Candidate Scoring

**Approach:** Score DOM nodes by tag type, paragraph count, text length, punctuation density, class/id hints, and link density. Promote parent and grandparent containers of strong paragraphs, then select the highest-scoring subtree and clean it. Mozilla Readability uses this family of heuristics and exposes knobs such as candidate count, character threshold, class preservation, and link-density modifiers.

**Pros:**

- Battle-tested for article-like pages.
- Handles common CMS clutter well.
- Produces a coherent content subtree, not just loose text blocks.

**Cons:**

- Tuned mostly for prose articles.
- Magic-number heavy.
- Can drop useful content like tables, code samples, comments, or short reference material.
- May over-select if an entire page has one large wrapper.

**Good at:** News, blogs, essays, recipes, long-form documentation pages.

**Bad at:** Search results, forums, product pages, dashboards, API references, sparse pages.

Sources: [Mozilla Readability](https://github.com/mozilla/readability), [Readability options and output](https://github.com/mozilla/readability), [Readability core scoring notes](https://deepwiki.com/mozilla/readability/2-core-content-extraction-system).

### 3. Text-Density / CETD

**Approach:** Compute density features for DOM nodes or text blocks: visible text length, word count, punctuation count, tag count, depth, and hyperlink text ratio. Main content is expected to have high text density and relatively low structural/link noise. DOM Content Extraction via Text Density variants use this idea directly.

**Pros:**

- Language-light if based mostly on character counts and tag structure.
- Simple to implement over a parsed DOM.
- Good fallback when semantic tags and classes are unreliable.

**Cons:**

- Can confuse dense navigation mega-menus, legal footers, and comment sections for content.
- Weak on image/video-first pages.
- Often loses structure unless paired with subtree reconstruction.

**Good at:** Article pages, text-heavy pages, old CMS templates, pages without useful semantic tags.

**Bad at:** Galleries, product listings, dense tables, code-heavy pages, short announcements.

Sources: [dom-content-extraction](https://docs.rs/crate/dom-content-extraction/latest), [CETR/CETD-related tag/density extraction literature](https://experts.illinois.edu/en/publications/cetr-content-extraction-via-tag-ratios).

### 4. Link-Density Rejection

**Approach:** Treat high anchor-text ratio as a boilerplate signal. Navigation, related-links, menus, tag clouds, and footers usually contain many links relative to ordinary text. A node with high text length but high link density is demoted unless it is a known content structure, such as citations or reference lists.

**Pros:**

- Very effective at removing nav, sidebars, related articles, and footers.
- Cheap to compute.
- Works well as a secondary score in many algorithms.

**Cons:**

- Penalizes legitimate content with many links: Wikipedia, documentation, link roundups, source-heavy articles.
- Can misclassify tables of contents or reference sections that should be preserved.
- Needs threshold tuning by page type.

**Good at:** News/blog pages with clear body prose.

**Bad at:** Wikis, docs, resource indexes, academic/reference content.

Sources: [Mozilla Readability link-density option](https://github.com/mozilla/readability), [Boilerpipe density classifier notes](https://www.ccs.neu.edu/home/vip/teach/IRcourse/6_ML/boilerpipe/boilerpipe-1.2.0/javadoc/1.1/de/l3s/boilerpipe/filters/english/DensityRulesClassifier.html).

### 5. Boilerpipe-Style Shallow Block Classification

**Approach:** Segment the page into text blocks, then classify each block using shallow features such as word count, text density, link density, and neighboring block context. Boilerpipe showed that relatively small feature sets can perform competitively for boilerplate removal.

**Pros:**

- Good precision on text extraction.
- Does not require perfect DOM semantics.
- Neighbor-aware classification helps recover short paragraphs around strong content blocks.

**Cons:**

- Produces block labels more naturally than a clean DOM subtree.
- Adjacent-block rules can be brittle across templates.
- Needs post-processing to restore headings, lists, images, and tables.

**Good at:** Search indexing, text-only extraction, boilerplate removal before NLP.

**Bad at:** Reader-mode rendering where preserving DOM structure matters.

Sources: [Boilerplate Detection using Shallow Text Features](https://research.uni-hannover.de/en/publications/boilerplate-detection-using-shallow-text-features/), [Boilerpipe classifier docs](https://www.ccs.neu.edu/home/vip/teach/IRcourse/6_ML/boilerpipe/boilerpipe-1.2.0/javadoc/1.1/de/l3s/boilerpipe/filters/english/DensityRulesClassifier.html).

### 6. jusText Paragraph Classification

**Approach:** Split page text into paragraphs, classify each paragraph independently using link density, stopword density, and length, then revise classifications using neighboring paragraphs. The Rust `justext` crate exposes this as `Good`, `Bad`, `NearGood`, and `Short` paragraph classes.

**Pros:**

- Strong for plain-text extraction.
- Stopword density helps distinguish real prose from boilerplate.
- Neighbor passes recover short headings or short paragraphs near content.

**Cons:**

- Language-dependent unless configured with weaker language-independent thresholds.
- Paragraph-first output does not naturally preserve rich DOM structure.
- Poor fit for non-prose pages.

**Good at:** Multilingual article text extraction, NLP preprocessing, corpus building.

**Bad at:** Tables, code docs, product specs, UI-like pages, pages with little prose.

Source: [Rust justext docs](https://docs.rs/crate/justext/latest).

### 7. CETR Tag-Ratio Clustering

**Approach:** Convert HTML into a sequence of lines or blocks and compute tag ratios: amount of text versus amount of markup. Cluster the resulting sequence into content and non-content regions. CETR extends simple one-dimensional ratios into a two-dimensional model to improve clustering.

**Pros:**

- Template-agnostic.
- Works across varied domains and languages better than class-name heuristics.
- Useful when DOM nesting is noisy but source order still reflects visible reading order.

**Cons:**

- Source-line sensitivity can be awkward after minification or generated markup.
- Clustering adds complexity and threshold choices.
- Reconstructing the original DOM subtree is harder than identifying text spans.

**Good at:** Diverse scraped web corpora, text extraction pipelines.

**Bad at:** Minified single-line HTML, client-rendered pages, pages where source order differs from rendered order.

Source: [CETR: Content Extraction via Tag Ratios](https://experts.illinois.edu/en/publications/cetr-content-extraction-via-tag-ratios).

### 8. Visual-Geometry Scoring

**Approach:** Use browser-rendered layout information: bounding boxes, viewport position, element area, font size, visibility, center proximity, overlap, z-index, and distance from page centers. Main content is often large, central, visible, and near the top-middle of the reading path.

**Pros:**

- Handles visual layout patterns that static DOM analysis misses.
- Useful for pages with poor semantic HTML.
- Can reject hidden, offscreen, or visually tiny boilerplate.

**Cons:**

- Requires a rendering engine or browser automation.
- More expensive and less deterministic across viewport sizes.
- Layout shifts, sticky headers, modals, and responsive variants complicate scoring.

**Good at:** Modern responsive pages, visually structured articles, pages with weak markup.

**Bad at:** Server-side batch extraction at scale, pages with heavy JavaScript or anti-bot behavior.

Source: [Don't read, just look: Main content extraction using visual features](https://arxiv.org/abs/2110.14164).

### 9. Site-Template Consensus

**Approach:** Crawl multiple pages from the same site and identify repeated DOM paths, repeated text, repeated link blocks, and common layout fragments. Demote repeated template regions and promote page-unique regions with substantial text.

**Pros:**

- Very strong for repeated CMS templates.
- Learns site-specific boilerplate without manual rules.
- Can improve over time for a known domain.

**Cons:**

- Needs multiple pages per site.
- Weak on single-page extraction.
- Can misclassify repeated but important content, such as standard disclaimers or recurring specs.
- Requires cache invalidation when templates change.

**Good at:** Crawlers, search indexing, domain-specific ingestion, documentation sites.

**Bad at:** One-off URLs, highly personalized pages, pages behind login, A/B tested layouts.

### 10. Hybrid Voting / Confidence Model

**Approach:** Run several deterministic extractors and score agreement. For each candidate subtree or block, combine signals: semantic landmark score, readability score, density score, link penalty, visual score, metadata alignment, and template uniqueness. Return both the selected subtree and a confidence score.

**Pros:**

- More robust than any single heuristic.
- Allows explainable diagnostics: "selected because it was inside `<main>`, had high text density, and matched JSON-LD headline."
- Can degrade gracefully when one signal is unavailable.

**Cons:**

- More engineering complexity.
- Needs calibration and a policy for conflicting signals.
- Harder to keep stable across changes unless test corpora are strong.

**Good at:** Production extraction systems, RAG ingestion, search indexing, reader-mode tooling.

**Bad at:** Very small tools where simplicity matters more than edge-case coverage.

## Rust Crates With Overlapping Functionality

### `dom_smoothie`

`dom_smoothie` is one of the strongest current Rust options for reader-style extraction. It closely follows Mozilla Readability, but adds alternative candidate-selection behavior intended to recover meaningful content in cases where Mozilla’s original approach can discard too much. It also exposes parsing policy, readability checks, text modes, metadata, and configuration.

**Approach:** Readability-style DOM scoring and cleaning, with configurable candidate selection and some implementation differences from Mozilla Readability.

**Best fit:** Article extraction where preserving cleaned HTML and metadata matters.

**Caveat:** Still heuristic-heavy and article-oriented.

Source: [dom_smoothie docs](https://docs.rs/crate/dom_smoothie/0.17.0).

### `readabilityrs` / `readable-rs` / `legible`

These crates are Rust ports of Mozilla Readability-style extraction. `readabilityrs` describes itself as a port of Mozilla Readability.js and reports compatibility with Mozilla’s test suite. `readable-rs` exposes a compact `extract` entry point and a score store over parsed nodes. `legible` similarly documents a pipeline of document preparation, metadata extraction, content scoring, candidate selection, and content cleaning.

**Approach:** Mozilla Readability-style scoring: rank elements by tag type, text density, link density, and class/id patterns, select the winning subtree, clean it, and return article metadata plus content.

**Best fit:** Reader-mode behavior for articles and blog posts.

**Caveat:** These crates overlap heavily. Selection should be based on maintenance, API fit, test behavior, and whether you need Markdown output, metadata, or exact Mozilla parity.

Sources: [readabilityrs docs](https://docs.rs/crate/readabilityrs/latest), [readabilityrs GitHub](https://github.com/theiskaa/readabilityrs), [readable-rs docs](https://docs.rs/readable-rs/latest/readable_rs/), [legible docs](https://docs.rs/legible).

### `dom-content-extraction`

`dom-content-extraction` is narrower but valuable because it implements a text-density-oriented algorithm rather than another Readability port.

**Approach:** DOM/content extraction via text-density analysis, based on the Content Extraction via Text Density family of algorithms.

**Best fit:** A second opinion or fallback extractor when Readability-style class/id and paragraph scoring is too template-specific.

**Caveat:** Text-density extraction is less naturally suited to preserving a polished article DOM with images, captions, embeds, and metadata.

Source: [dom-content-extraction docs](https://docs.rs/crate/dom-content-extraction/latest).

### `justext`

`justext` is not primarily a DOM-subtree extractor, but it overlaps strongly with boilerplate removal and main text identification.

**Approach:** Paragraph classification using link density, stopword density, character length, and neighboring paragraph revision.

**Best fit:** Plain-text extraction, corpus cleanup, multilingual NLP preprocessing.

**Caveat:** Less appropriate when the required output is a main-content DOM fragment.

Source: [justext docs](https://docs.rs/crate/justext/latest).

## Corpus Knowledge That Can Improve These Algorithms

Yes. A main-content extractor can improve significantly if it accumulates structured evidence over time.

Useful corpus layers:

1. **Gold-labeled pages:** Store raw HTML, rendered DOM, screenshot, expected main-content DOM paths, expected text, and expected metadata. Include negative labels for nav, footer, aside, comments, ads, cookie banners, and related links.
2. **Domain template fingerprints:** For each domain, cache repeated DOM paths, repeated text shingles, repeated class/id patterns, common nav/sidebar selectors, and known article containers.
3. **Extractor disagreement records:** When Readability, density, jusText, and visual scoring disagree, save the candidates and scores. These examples are especially valuable for tuning thresholds.
4. **Rendered layout features:** Preserve viewport size, bounding boxes, visibility, font sizes, and element positions. Static DOM-only corpora miss many modern layout signals.
5. **Metadata alignment:** Track whether candidate headings match `<title>`, OpenGraph title, JSON-LD headline, canonical URL, author, date, and breadcrumbs.
6. **Language and script profiles:** Stopword density, punctuation expectations, word segmentation, and text-length thresholds should vary by language and writing system.
7. **Content-type taxonomy:** Label pages as article, docs, forum thread, product page, search result, recipe, landing page, changelog, API reference, gallery, or dashboard. The best scoring policy differs by type.
8. **User correction feedback:** If humans or downstream systems mark extracted content as missing, over-included, or wrong, store the failed DOM paths and the corrected target.
9. **Versioned site behavior:** Templates change. Keep timestamps and invalidate domain-specific rules when repeated paths or class patterns drift.

The most practical architecture is a hybrid extractor: deterministic base algorithms, per-candidate feature logging, a labeled regression corpus, and optional domain-specific learned weights. The learned layer does not need to be an opaque model; even periodically recomputed thresholds and selector priors can improve accuracy while preserving explainability.
