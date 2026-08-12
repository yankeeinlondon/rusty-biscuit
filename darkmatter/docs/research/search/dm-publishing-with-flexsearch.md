---
hash: ef46db3751d8e999-00edb4b3121712cf
---
---
prompt: |-
    ## Context
    
    Darkmatter will soon be introducing a subcommand called "publish" which will take an
    "entry point" Markdown document and then _publish_ the tree of documents (through both links and transclusion) that this root document connects to.

    > Note: 
    >
    > - the primary output for _publish_ will be a single HTML file that is "self-contained" in terms of content, Javascript, and CSS.
    > - the Darkmatter CLI will expose publishing via `md publish <file>` and the Darkmatter CLI will be responsible for building the index for FlexSearch too. Whether that index is embedded into the HTML payload or provided as a "sidecar" file has not been decided yet.

    ## FlexSearch Research

    Below you will find research on the FlexSearch library:

    ::file @darkmatter/docs/research/search/flexsearch.md

    ## Task

    Your task is to imagine ways of using the FlexSearch library to provide a high speed, in browser, search UX for the published content.

    - brainstorm 3-4 different approaches that you think might be employed to achieve this functionality
        - each approach should be given a descriptive name and written to the document as it's own H2 heading
    - For each approach:
        - describe how the approach treats the problem
        - what is it optimized for? what potential downsides does this approach have?
        - use Typescript code examples where appropriate to ground how this approach might be implemented
    - based on the different approaches laid out and your understanding of Darkmatter:
        - what are some of the CLI switches you might imagine being exposed in the `md publish <file>` subcommand?
        - what Frontmatter properties could you imagine being leveraged (in the base document) to configure the publish process?
last_updated: 2026-07-10
---# Browser Search for Published Content

Darkmatter can treat search as another build artifact derived from the resolved publication graph. After following links and transclusions, composing documents, and assigning final URLs and anchors, `md publish` can project the resulting content into search records and build a FlexSearch index.

The most useful search unit is usually a section rather than an entire Markdown file. Section-level records produce precise result links, smaller snippets, and better matches in long documents.

```ts
interface SearchRecord {
  /** Compact, stable identifier assigned by md publish. */
  id: number;

  /** Canonical source document identity. */
  documentId: number;

  title: string;
  section?: string;
  body: string;

  /** Final location inside the published HTML document. */
  href: string;

  /** Optional result metadata. */
  tags?: string[];
}
```

Search records should be generated after composition so users can find the content they actually see. Darkmatter should nevertheless retain source-document identity so the same transcluded section can be indexed once and linked to a canonical location rather than appearing as several indistinguishable results.

## Embedded Corpus with Runtime Indexing

This approach embeds normalized search records in the generated HTML and constructs the FlexSearch index when the page loads.

```html
<script id="darkmatter-search-records" type="application/json">
[
  {
    "id": 1,
    "documentId": 1,
    "title": "Publishing",
    "section": "Search",
    "body": "Darkmatter builds a browser search index...",
    "href": "#publishing-search"
  }
]
</script>
```

The browser reads those records and creates a field-specific document index:

```ts
import { Document } from "flexsearch";

interface SearchRecord {
  id: number;
  documentId: number;
  title: string;
  section?: string;
  body: string;
  href: string;
  tags?: string[];
}

const records = JSON.parse(
  document.querySelector("#darkmatter-search-records")!.textContent!,
) as SearchRecord[];

const index = new Document<SearchRecord>({
  document: {
    id: "id",
    index: [
      {
        field: "title",
        tokenize: "forward",
        encoder: "Default",
        resolution: 9,
      },
      {
        field: "section",
        tokenize: "forward",
        encoder: "Default",
        resolution: 8,
      },
      {
        field: "body",
        tokenize: "strict",
        encoder: "Default",
        resolution: 7,
        context: {
          depth: 2,
          resolution: 3,
          bidirectional: true,
        },
      },
    ],
    tag: ["tags"],
    store: ["documentId", "title", "section", "href"],
  },
});

for (const record of records) {
  index.add(record);
}
```

This is optimized for implementation simplicity, transparent artifacts, and small publications. The HTML contains ordinary records that can be inspected, tested, or consumed by a different search engine later. It also avoids coupling the published artifact to FlexSearch’s serialized index format.

The main downside is startup work. The browser must parse the corpus, normalize every field, and build the complete index before full search is available. For a large publication this increases time-to-interactive, creates a temporary memory spike, and can block the main thread. It also duplicates some content because the rendered HTML and the embedded search corpus both contain the body text.

This mode is a strong development option and a reasonable production option for small publications.

## Embedded Prebuilt Index

This approach makes `md publish` build the FlexSearch index ahead of time. The generated HTML embeds the exported index fragments together with a small result metadata table. The browser constructs an identically configured index and imports the prepared data instead of re-indexing prose.

Conceptually, the generated payload contains:

```ts
interface EmbeddedSearchArtifact {
  schemaVersion: number;
  flexsearchVersion: string;
  configurationFingerprint: string;
  indexParts: Record<string, string>;
  records: Record<
    number,
    {
      title: string;
      section?: string;
      href: string;
    }
  >;
}
```

The runtime configuration must match the configuration used by `md publish`:

```ts
import { Document } from "flexsearch";
import artifact from "virtual:darkmatter-search-artifact";

function createIndex(): Document {
  return new Document({
    document: {
      id: "id",
      index: [
        { field: "title", tokenize: "forward", encoder: "Default" },
        { field: "section", tokenize: "forward", encoder: "Default" },
        {
          field: "body",
          tokenize: "strict",
          encoder: "Default",
          context: {
            depth: 2,
            resolution: 3,
            bidirectional: true,
          },
        },
      ],
    },
  });
}

const index = createIndex();

for (const [key, data] of Object.entries(artifact.indexParts)) {
  index.import(key, data);
}
```

This mode is optimized for fast startup and a self-contained deployment. Expensive tokenization and index construction happen once in the CLI, where they do not affect browser responsiveness. The browser only parses and imports the prepared structures.

It is a natural default for Darkmatter because the publisher already has the complete resolved document graph and final rendered anchors. It can build records, validate every result target, generate the index, and place all required JavaScript, CSS, metadata, and index data into one HTML artifact.

The principal downside is format coupling. FlexSearch exports must be imported by a compatible FlexSearch runtime using the same index configuration. Darkmatter should therefore embed:

- A Darkmatter search-artifact schema version.
- The FlexSearch version used by the publisher.
- A fingerprint of the effective index configuration.
- A checksum for every exported fragment.
- A small fixture query that can be used for build-time validation.

The CLI should fail publication if an exported index cannot be re-imported and queried successfully. This makes incompatibility a publishing error instead of a broken search box discovered by the reader.

Embedding also increases the HTML file size. Compression from the web server helps transfer size, but the browser must still hold the decoded index in memory.

## Worker-Isolated Embedded Search

This approach keeps the single-file artifact while moving index import and query execution into a Web Worker. It can use either an embedded corpus or, preferably, an embedded prebuilt index.

The page communicates with the worker through a small request protocol:

```ts
interface SearchRequest {
  type: "search";
  sequence: number;
  query: string;
  limit: number;
}

interface SearchResponse {
  type: "results";
  sequence: number;
  results: Array<{
    id: number;
    title: string;
    section?: string;
    href: string;
    score?: number;
  }>;
}

let sequence = 0;

export function search(query: string): void {
  const request: SearchRequest = {
    type: "search",
    sequence: ++sequence,
    query,
    limit: 12,
  };

  searchWorker.postMessage(request);
}

searchWorker.addEventListener(
  "message",
  (event: MessageEvent<SearchResponse>) => {
    if (event.data.sequence !== sequence) {
      return;
    }

    renderSearchResults(event.data.results);
  },
);
```

To remain self-contained, `md publish` can bundle the worker and FlexSearch runtime into an inline string, create a `Blob`, and start the worker from its object URL:

```ts
const workerSource =
  document.querySelector("#darkmatter-search-worker")!.textContent!;

const workerUrl = URL.createObjectURL(
  new Blob([workerSource], { type: "text/javascript" }),
);

const searchWorker = new Worker(workerUrl);
```

This mode is optimized for UI responsiveness. Importing a large index, decoding records, and running expensive queries no longer competes directly with rendering, scrolling, or keyboard input on the main thread. It is particularly useful when a publication contains long prose, code blocks, or thousands of sections.

Its costs are additional runtime machinery and duplicated memory during worker startup or message transfer. Search is necessarily asynchronous, so the UI must debounce input and reject stale responses. A self-contained blob worker also affects Content Security Policy: a host may need to permit `worker-src blob:`. Darkmatter should document the required policy and optionally support a separate worker asset for deployments with stricter CSP rules.

A worker should be a publishing strategy rather than a requirement. Small publications gain little from its complexity.

## Progressive Sidecar Search

This approach separates the index into one or more sidecar assets. The HTML embeds a small navigation index for titles and headings, while body-search data is loaded only when the reader opens search or enters a sufficiently specific query.

```text
manual.html
manual.search-manifest.json
manual.search-navigation.bin
manual.search-body-00.bin
manual.search-body-01.bin
```

The manifest describes the artifact as a versioned collection:

```ts
interface SearchManifest {
  schemaVersion: number;
  contentHash: string;
  navigation: string;
  bodyShards: Array<{
    url: string;
    firstId: number;
    lastId: number;
    integrity: string;
  }>;
}
```

The browser can provide immediate title and heading results while loading the body index in the background:

```ts
const manifest = await fetch(
  new URL("manual.search-manifest.json", document.baseURI),
).then((response) => response.json() as Promise<SearchManifest>);

await importNavigationIndex(manifest.navigation);
enableNavigationSearch();

requestIdleCallback(async () => {
  for (const shard of manifest.bodyShards) {
    await importBodyShard(shard.url, shard.integrity);
  }

  enableFullTextSearch();
});
```

This strategy is optimized for large publications, repeat visits, and conventional web hosting. The browser downloads only the search material it needs, sidecars can be cached independently from the HTML, and an updated publication can reuse unchanged shards if their content hashes remain stable.

It also creates the clearest path toward future range requests, service-worker caching, or an IndexedDB-backed local cache without changing the content model.

The downsides are deployment complexity and reduced portability. Moving or emailing the HTML without its sidecars breaks full-text search. Local `file:` URLs may also encounter browser restrictions that do not affect an entirely embedded artifact. Sharding must be based on deterministic document or section identities; arbitrary byte-sized shards would produce unnecessary cache invalidation when one early document changes.

This should be an explicit scalability mode rather than the default. Darkmatter’s primary output can remain self-contained while allowing users with large corpora and managed hosting to choose sidecars.

## Recommended Default

The embedded prebuilt index is the strongest default for `md publish`. It preserves the single-file experience, moves indexing cost into the CLI, and lets Darkmatter validate the search artifact before writing the final HTML.

A balanced default configuration would:

- Index one record per heading section, with a document-level record for content before the first heading.
- Use numeric record IDs assigned deterministically during publication.
- Use prefix tokenization for titles and headings.
- Use strict contextual indexing for prose.
- Exclude generated navigation, repeated transclusion wrappers, and presentation-only text.
- Index transcluded content once under a canonical section identity.
- Store only result-card metadata in FlexSearch.
- Keep snippets in a compact record table or derive them from source ranges.
- Switch to a worker only when requested or when an explicit automatic threshold is enabled.
- Reserve sidecars for large, hosted publications.

The search-record projection should remain a Darkmatter-owned intermediate representation. FlexSearch is then a backend used to encode that representation, rather than the data model to which publishing becomes permanently coupled.

## Possible CLI Switches

The CLI should expose a small set of policy-oriented switches and avoid requiring users to understand every FlexSearch implementation detail.

```text
md publish <file> [OPTIONS]
```

Core search controls could include:

```text
--search
--no-search
--search-strategy <embedded-corpus|embedded-index|embedded-worker|sidecar>
--search-output <path>
--search-profile <exact|balanced|prefix|tolerant>
--search-unit <document|section>
--search-fields <title,headings,body,code>
--search-language <language>
--search-limit <number>
--search-min-query-length <number>
```

Publication graph controls could include:

```text
--follow-links
--no-follow-links
--follow-transclusions
--no-follow-transclusions
--max-depth <number>
--include <glob>
--exclude <glob>
--include-drafts
```

Index-content controls could include:

```text
--search-include-code
--search-exclude-code
--search-include-tags
--search-context-depth <number>
--search-deduplicate-transclusions
--search-index-each-transclusion
```

Artifact and performance controls could include:

```text
--search-compression <none|gzip|brotli>
--search-shard-size <bytes>
--search-worker-threshold <records>
--search-cache
--no-search-cache
--search-debug-artifact <path>
```

The distinction between `--search-output` and the ordinary publish output matters only for sidecar mode. Embedded modes should reject a separate search output unless it is being requested as a debug artifact.

`--search-profile` should expand to a versioned Darkmatter-owned configuration. For example, `balanced` might use forward title indexing and strict contextual body indexing. This provides stable user intent even if a later FlexSearch release renames an encoder or changes a low-level option.

As with existing Darkmatter style handling, CLI claims should override frontmatter field by field. An explicit `--no-search`, for example, should win over `publish.search.enabled: true` without discarding unrelated frontmatter such as the result limit.

## Possible Frontmatter Properties

A namespaced `publish` mapping keeps publication policy separate from document style and composition:

```yaml
publish:
  title: Darkmatter Guide

  traversal:
    links: true
    transclusions: true
    max-depth: 8
    include:
      - "docs/**/*.md"

    exclude:

      - "docs/drafts/**"

  search:
    enabled: true
    strategy: embedded-index
    profile: balanced
    unit: section
    fields:

      - title
      - headings
      - body

    language: en
    limit: 12
    min-query-length: 2
    include-code: false
    deduplicate-transclusions: true
    context-depth: 2

  search-ui:
    placeholder: Search this publication
    keyboard-shortcut: "/"
    show-snippets: true
    highlight-matches: true
    empty-query: recent-sections
    no-results-message: No matching sections found
```

A sidecar publication could add transport-specific settings without changing the search semantics:

```yaml
publish:
  search:
    strategy: sidecar
    profile: balanced
    unit: section

    sidecar:
      filename: guide.search-manifest.json
      shard-size: 262144
      preload: navigation
      cache: true
```

A worker configuration could be similarly scoped:

```yaml
publish:
  search:
    strategy: embedded-worker

    worker:
      startup: on-focus
      debounce-ms: 80
```

Traversal properties belong under `publish.traversal` because they determine the publication graph itself. Search properties should only affect the projection and indexing of documents already admitted to that graph.

Darkmatter could eventually support limited per-document publication metadata, such as `draft`, `publish.search: false`, or an explicit search title. Those properties should not silently alter traversal from arbitrary descendant documents. The entry point should remain the authority for graph-wide policy, while descendant metadata controls only the treatment of that descendant.

The effective configuration should be serialized into the generated artifact in a compact diagnostic form. That makes a published file explainable: developers can determine which strategy, profile, fields, graph rules, schema version, and FlexSearch version produced its search experience.
