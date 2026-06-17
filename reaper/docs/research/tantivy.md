---
prompt: |-
    The `tantivy` crate can support keyword, BM25, and evidence retrieval against taxonomy labels or training corpora.

    Your task is to research both BM25 and the `tantivy` crate and then report on:

    - what is BM25? Where is it used? Give an example or two of it's usage
    - investigate the `tantivy` crate and:
        - report on it's functional footprint
        - what feature flags does it provide and when should you use them?
        - give 2-3 use cases of where you might use `tantivy` and provide a code example for each.
last_updated: 2026-06-03
---
## BM25

BM25, or Okapi BM25, is a classic keyword relevance-ranking function used in information retrieval. It ranks documents by combining:

- **Term frequency**: query terms appearing more often in a document help, but with saturation so repetition does not grow score linearly.
- **Inverse document frequency**: rare query terms across the corpus count more than common terms.
- **Document length normalization**: long documents are normalized so they do not win only because they contain more words.

A simplified shape of the score is:

```text
score(document, query) = sum_over_query_terms(IDF(term) * saturated_tf(term, document, document_length))
```

BM25 is a bag-of-words scorer: it cares that query terms occur in a document, not necessarily that they occur near each other. Phrase search, proximity, field boosts, filters, and rerankers are usually layered around it.

BM25 is widely used as the default or baseline sparse retriever in search systems such as Lucene-style engines, Elasticsearch/OpenSearch-style systems, and hybrid retrieval pipelines. It is especially strong for exact-match, identifier-like, long-tail, and vocabulary-sensitive queries where dense embeddings can blur important lexical distinctions.

Examples:

- **Documentation search**: a query like `schema field tokenizer` should rank pages containing those exact technical terms highly.
- **Evidence retrieval for RAG**: before asking an LLM to answer, retrieve top passages containing terms like `BM25 fieldnorm Tantivy scorer` and pass those passages as grounded evidence.
- **Taxonomy label matching**: map a free-text phrase like `wireless router setup` against labels such as `Networking > Routers > Configuration` by indexing labels, aliases, and descriptions.

Sources: [Okapi BM25 overview](https://en.wikipedia.org/wiki/Okapi_BM25), [Tantivy architecture notes](https://docs.rs/crate/tantivy/latest/source/ARCHITECTURE.md), [Tantivy crate page](https://docs.rs/crate/tantivy/latest).

## Tantivy

`tantivy` is a Rust search engine library inspired by Apache Lucene. It is not a hosted search server; it is an embeddable crate for building one. The current docs describe Tantivy as focused on full-text search over a prebuilt index, returning the top matching documents efficiently, with BM25 as its built-in relevance score.

Its functional footprint includes:

- Full-text indexing and search
- BM25 scoring
- Schema-defined fields
- Text, numeric, date, IP, bool, JSON, bytes, and facet fields
- Configurable tokenization, stemming, stop words, and custom tokenizers
- Natural query parsing with boolean operators, field targeting, boosts, ranges, phrase queries, fuzzy fields, set terms, and match-all queries
- Incremental and multithreaded indexing
- Stored fields for result reconstruction
- Fast fields, similar to Lucene DocValues, for sorting, filtering, scoring features, and aggregation
- Range queries
- Faceted search
- Aggregation collectors, including histograms, range buckets, averages, and stats
- Segment-based indexing and background merging
- Mmap-backed directory support
- Compressed document store using LZ4, Zstd, or no compression
- Custom queries, scorers, collectors, and tokenizers

One important design point: stored documents are for fetching the final hits, not scanning every match. Tantivy’s architecture notes warn that repeatedly hitting the document store for large result sets is usually misuse; use fast fields and collectors for per-match computation.

## Feature Flags

As of `tantivy` 0.26.1, the crate declares these feature flags in `Cargo.toml`:

| Feature                     | Default | Use When                                                                                                                                                                               |
|-----------------------------|--------:|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `mmap`                      | yes     | You want fast local-disk indexes backed by memory-mapped files. This is the normal production choice on Linux, macOS, and Windows.                                                     |
| `stopwords`                 | yes     | You want built-in stop-word support for text analysis. Keep it on unless minimizing dependencies or implementing all analysis yourself.                                                |
| `lz4-compression`           | yes     | You want fast document-store compression. This is the usual default for search result reconstruction.                                                                                  |
| `columnar-zstd-compression` | yes     | You want Zstd compression support in Tantivy’s columnar storage layer. Keep it on for general-purpose indexes unless dependency size matters more.                                     |
| `stemmer`                   | yes     | You want stemming support through `rust-stemmers`, useful for natural-language search over supported languages.                                                                        |
| `zstd-compression`          | no      | You want Zstd for the document store, typically when smaller stored-field size matters more than LZ4’s speed.                                                                          |
| `failpoints`                | no      | You are testing failure paths in Tantivy itself or in low-level integrations. Do not enable for normal application builds.                                                             |
| `unstable`                  | no      | You need APIs or internals marked unstable, usually for benchmarks or experiments. Avoid in library public APIs.                                                                       |
| `quickwit`                  | no      | You are integrating with Quickwit-related storage/components. Most Tantivy users should leave this off.                                                                                |
| `compare_hash_only`         | no      | You want faster indexing by comparing only term hashes. This trades correctness for speed because rare hash collisions can cause missing terms; only use when that risk is acceptable. |

Default dependency form:

```toml
[dependencies]
tantivy = "0.26"
```

Lean build example:

```toml
[dependencies]
tantivy = { version = "0.26", default-features = false, features = ["mmap", "lz4-compression"] }
```

Source: [Tantivy `Cargo.toml` feature section](https://docs.rs/crate/tantivy/latest/source/Cargo.toml).

## Use Case: Taxonomy Label Retrieval

Use Tantivy when you need lexical matching over labels, aliases, descriptions, and paths. This works well for category assignment because taxonomy labels are often short, exact, and vocabulary-sensitive.

```rust
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};

fn taxonomy_lookup() -> tantivy::Result<()> {
    let mut schema_builder = Schema::builder();
    let label = schema_builder.add_text_field("label", TEXT | STORED);
    let aliases = schema_builder.add_text_field("aliases", TEXT);
    let path = schema_builder.add_text_field("path", STRING | STORED);
    let schema = schema_builder.build();

    let index = Index::create_in_ram(schema.clone());
    let mut writer = index.writer(50_000_000)?;

    writer.add_document(doc!(
        label => "Wireless router configuration",
        aliases => "wifi setup network router access point",
        path => "Networking/Routers/Configuration",
    ))?;
    writer.add_document(doc!(
        label => "Password reset",
        aliases => "login account credentials forgot password",
        path => "Accounts/Authentication/PasswordReset",
    ))?;
    writer.commit()?;

    let reader = index.reader()?;
    let searcher = reader.searcher();

    let mut parser = QueryParser::for_index(&index, vec![label, aliases]);
    parser.set_field_boost(label, 2.0);

    let query = parser.parse_query("wifi router setup")?;
    let hits = searcher.search(&query, &TopDocs::with_limit(5).order_by_score())?;

    for (score, address) in hits {
        let doc: TantivyDocument = searcher.doc(address)?;
        println!("{score:.3} {}", doc.to_json(&schema));
    }

    Ok(())
}
```

## Use Case: Evidence Retrieval for a Training Corpus

Use Tantivy to retrieve passages from documentation, tickets, transcripts, or training examples before handing evidence to a classifier, reranker, or LLM. BM25 gives a strong sparse baseline and pairs well with dense/vector retrieval in hybrid systems.

```rust
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};

fn retrieve_evidence() -> tantivy::Result<Vec<String>> {
    let mut schema_builder = Schema::builder();
    let source_id = schema_builder.add_text_field("source_id", STRING | STORED);
    let passage = schema_builder.add_text_field("passage", TEXT | STORED);
    let schema = schema_builder.build();

    let index = Index::create_in_ram(schema.clone());
    let mut writer = index.writer(50_000_000)?;

    writer.add_document(doc!(
        source_id => "doc-17",
        passage => "Tantivy stores fieldnorm data used by BM25 scoring.",
    ))?;
    writer.add_document(doc!(
        source_id => "doc-42",
        passage => "Fast fields are column-oriented values used for sorting and aggregation.",
    ))?;
    writer.commit()?;

    let reader = index.reader()?;
    let searcher = reader.searcher();
    let parser = QueryParser::for_index(&index, vec![passage]);
    let query = parser.parse_query("BM25 scoring fieldnorm")?;

    let hits = searcher.search(&query, &TopDocs::with_limit(3).order_by_score())?;

    let mut evidence = Vec::new();
    for (_score, address) in hits {
        let doc: TantivyDocument = searcher.doc(address)?;
        evidence.push(doc.to_json(&schema));
    }

    Ok(evidence)
}
```

## Use Case: Search With Numeric Filters and Ranking Fields

Use Tantivy for embedded application search where documents need lexical relevance plus structured fields for filtering, sorting, or aggregation. Examples include package search, email archives, product catalogs, internal knowledge bases, and local CLI indexes.

```rust
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, RangeQuery};
use tantivy::schema::*;
use tantivy::{doc, Index};

fn search_recent_articles() -> tantivy::Result<()> {
    let mut schema_builder = Schema::builder();
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let published_year = schema_builder.add_u64_field("published_year", INDEXED | FAST | STORED);
    let schema = schema_builder.build();

    let index = Index::create_in_ram(schema.clone());
    let mut writer = index.writer(50_000_000)?;

    writer.add_document(doc!(
        title => "Hybrid retrieval with BM25",
        body => "BM25 remains useful for sparse retrieval and exact terminology.",
        published_year => 2026u64,
    ))?;
    writer.add_document(doc!(
        title => "Legacy search notes",
        body => "Older keyword systems used term statistics for document ranking.",
        published_year => 2018u64,
    ))?;
    writer.commit()?;

    let reader = index.reader()?;
    let searcher = reader.searcher();

    let parser = QueryParser::for_index(&index, vec![title, body]);
    let text_query = parser.parse_query("BM25 sparse retrieval")?;

    let year_filter = RangeQuery::new_u64_bounds(
        "published_year".to_string(),
        2024u64..=2026u64,
    );

    let query = BooleanQuery::new(vec![
        (Occur::Must, text_query),
        (Occur::Filter, Box::new(year_filter)),
    ]);

    let hits = searcher.search(&query, &TopDocs::with_limit(10).order_by_score())?;

    for (score, address) in hits {
        let doc: TantivyDocument = searcher.doc(address)?;
        println!("{score:.3} {}", doc.to_json(&schema));
    }

    Ok(())
}
```

## Practical Fit

Tantivy is a good fit when you want a Rust-native, embeddable search index with Lucene-like concepts but without running Elasticsearch, OpenSearch, or Solr. It is especially useful for local indexes, CLIs, desktop apps, developer tools, knowledge-base search, taxonomy matching, and evidence retrieval pipelines.

It is not the right abstraction if you need a complete distributed search service out of the box. For distributed search, the Tantivy project points users toward Quickwit, which is built on top of Tantivy.
