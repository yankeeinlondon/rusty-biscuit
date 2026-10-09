# Upgrading from Schema 4.x

Part of [Scope-Aware References](../references.md).

This note is for a consumer that read the relationship fields of records
written before schema 5. The field layout did not change; their meaning did.

In schema 4.x, `relations.references`, `dependencies`, `referenced_by` and
`semantics.is_recursive` pointed at "the first declaration with this name".
From schema 5.0 they mean "proven by a `Scoped` occurrence" (see
[Relationship summaries](../references.md#relationship-summaries)). A
consumer now gets edges to the declaration the name really binds, and
`is_recursive` is decided instead of always `Unknown`:

```rust
fn parse() {}
fn run() { let parse = || 1; parse(); }
fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }
fn walk(w: &W) { w.walk(); }
```

| Field | 4.x | 5.x |
|---|---|---|
| `run.references` | the function `parse` | the local `let parse` (its own record ID) |
| `fn parse`'s `referenced_by` | `[run]` | `[]` |
| `countdown.semantics.is_recursive` | `Unknown` (never trusted) | `Yes` |
| `parse.semantics.is_recursive` | `Unknown` | `No` |
| `walk.semantics.is_recursive` | `Unknown` | `Unknown` (`w.walk()` is a receiver call) |

Schema 5.0 also lets a file index carry `occurrences` and
`reference_coverage` (only under `Capability::Occurrences`). Schema 5.1, the
current version, adds an occurrence's optional `import_span` (see
[Selected import provenance](identity-and-written-names.md#selected-import-provenance)).
The `--json` envelopes report `schema_version` `{"major":5,"minor":1}`.

Nothing has to be migrated by hand. The persistent cache serves an entry
only when it and every record in it are exactly the current version, 5.1, so
an entry written by an earlier version, 5.0 included, is analyzed again on
first use and rewritten (see
[Cache Design](../cache-design/robustness.md#schema-versions)).
