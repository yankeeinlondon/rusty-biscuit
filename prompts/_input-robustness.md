## Input Robustness Matrix

This section applies when the work adds or changes code that reads a file format or configuration: a parser, a manifest or lockfile reader, a config loader, a deserializer. Skip it otherwise, including for text grammars (Markdown, prose markup, templates) and for writers, serializers, and renderers: their fields are not configuration, and this matrix does not describe their inputs.

A **load-bearing field** is any field whose value changes what the code reports or decides. For every load-bearing field, each shape below has a defined outcome. The specification or plan names the outcome in a table, and a test asserts it through the public result, not the parser's return value.

| Shape                       | Example                               | Must never                                                              |
|-----------------------------|---------------------------------------|-------------------------------------------------------------------------|
| absent                      | key omitted                           | read as an empty collection, unless the format defines absence that way |
| explicit null               | `"k": null`, YAML `k: null` or `k:`   | be conflated with absent; serde's `Option<T>` does exactly that         |
| wrong type, whole field     | `"k": 123` where an array is required | read as empty or as absent                                              |
| wrong type, one element     | `["a", 123]`                          | be filtered out silently                                                |
| wrong type, every element   | `[123]`                               | read as empty                                                           |
| empty                       | `[]`, `{}`                            | be treated as absent, or absent as empty                                |
| duplicate key               | the same key twice                    | be last-wins where the format forbids it                                |
| trailing or invalid content | valid document plus garbage           | be accepted                                                             |

Rules:

- the matrix table has one column per load-bearing field, on each side the code reads (manifest, lockfile, configuration), for each supported format
- one test per format walks the whole matrix from a real-tool fixture with one edit per cell; a hand-written case per cell found in review is the sign the table was skipped
- a control row proves the unedited fixture gives the positive result, so each edit is known to be load-bearing
- code smells to grep for before declaring the matrix done: `#[serde(default)]` on a load-bearing field; `Option<T>` where absent and null must differ (use a `present` deserializer or `Option<Option<T>>`); `filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on a load-bearing parse
- a review finding about one cell must list the state of every other cell in the same row and column; one cell per review is the failure mode this section exists to prevent
