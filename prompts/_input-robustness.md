## Input Robustness Matrix

This section applies when the work adds or changes code that reads a file format or configuration: a parser, a manifest or lockfile reader, a config loader, a deserializer. Skip it otherwise, including for text grammars (Markdown, prose markup, templates) and for writers, serializers, and renderers: their fields are not configuration, and this matrix does not describe their inputs.

A **load-bearing field** is any field whose value changes what the code reports or decides. Each one must have a defined outcome for every shape it can take, and a test must show that outcome through the crate's public result. The aim is to test each piece of code that decides an outcome **once**, not to test every field in every format.

### Two kinds of shape, tested at different places

**Field shapes** are decided by the code that validates the field. Test them once per field validator, whatever format the field arrived in:

| Shape                     | Example                               | Must never                                                              |
|---------------------------|---------------------------------------|-------------------------------------------------------------------------|
| absent                    | key omitted                           | read as an empty collection, unless the format defines absence that way |
| explicit null             | `"k": null`, YAML `k: null` or `k:`   | be conflated with absent; serde's `Option<T>` does exactly that         |
| wrong type, whole field   | `"k": 123` where an array is required | read as empty or as absent                                              |
| wrong type, one element   | `["a", 123]`                          | be filtered out silently                                                |
| wrong type, every element | `[123]`                               | read as empty                                                           |
| empty                     | `""`, `[]`, `{}`                      | be treated as absent, or absent as empty                                |

Only the shapes the field's type allows apply: the element rows apply to collections, not scalars. Do not write "not applicable" cells.

**Document shapes** are decided by the loader that turns the file into a value, before any field is read. Test them once per distinct loader, not once per field:

| Shape                       | Example                              | Must never                                    |
|-----------------------------|--------------------------------------|-----------------------------------------------|
| duplicate key               | the same key twice                   | be last-wins where the format forbids it      |
| trailing or invalid content | valid document plus garbage          | be accepted                                   |
| format-specific syntax      | YAML `k:` null, JSON5 comments and trailing commas, a second YAML document, a blank JSONL line | be misread, or reject what the format allows |

Formats that share a loader are one loader. Read the loading code to find out; for example, two file extensions mapped to the same parsing branch are tested once, and two formats parsed by one parser are tested once plus each format's own syntax.

### Rules

- **State a default outcome, then list only exceptions.** The specification or plan states once that every invalid shape produces a located error, runs nothing, and publishes nothing. It then lists only the fields and shapes that behave differently (for example "absent → default 100", "`[]` is an explicit no-op", "null is preserved as data"). Do not write a full table of cells that all say "error"
- **Fields that share a validator share its matrix.** Walk the field shapes once through the shared validator. Each field that uses it then needs only a control case and one invalid shape, which proves it is wired to that validator
- **Test in-process at the crate's public API.** "Public result" means the result the crate exposes (a preflight or execution result, a returned error with its location), not a private parser's return value, and not necessarily a spawned binary. Spawning the CLI for each cell multiplies the cost without adding coverage
- **Spawn the CLI only to prove wiring:** per loader, the control case plus one or two invalid shapes, asserting the error's location in the rendered output and that nothing launched
- **Keep a control row** that proves the unedited fixture gives the positive result, so each edit is known to be load-bearing. Build cells from a real-tool fixture with one edit per cell
- **Grep for these code smells before declaring the matrix done**: `#[serde(default)]` on a load-bearing field; `Option<T>` where absent and null must differ (use a `present` deserializer or `Option<Option<T>>`); `filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on a load-bearing parse. This is the cheapest check here and catches the most defects
- **A review finding about one cell must list the state of every other cell in the same row and column** of the validator or loader it hit. One cell per review is the failure mode this section exists to prevent
