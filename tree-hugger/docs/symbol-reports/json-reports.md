# JSON Reports

Part of [Symbol Reports](../symbol-reports.md).

The declaration commands have three JSON forms, for three kinds of consumer:

| Flag | Shape | Use it for |
|---|---|---|
| `--json` | one report per run: an envelope with one entry per file holding its selected records | scripts that read the whole result |
| `--jsonl` | one complete JSON object per selected declaration, one per line | streaming into `jq`, `grep` or a pipeline |
| `--json-full` | the `--json` envelope with every scanned file's complete index | tools that need everything the analysis knows |

`--stats --json` has its own envelope, described in
[Package Stats](./package-stats.md#stats-json---stats---json). The record
fields themselves are defined in
[Symbol Records](../symbol-records.md).

## The `--json` report

`--json` prints one report per run:

```json
{
  "report_schema_version": 1,
  "schema_version": { "major": 5, "minor": 1 },
  "root_dir": "/private/tmp/sample",
  "files": [
    {
      "file": "src/shapes.rs",
      "language": "Rust",
      "completed_passes": ["Parse"],
      "completed_capabilities": ["declarations", "flags"],
      "symbols": [
        {
          "schema_version": { "major": 5, "minor": 1 },
          "id": "65e1e0ed…",
          "language": "Rust",
          "kind": "Method",
          "scope": "Member",
          "identity": {
            "name": "norm",
            "qualified_name": "src/shapes.rs#Point::norm",
            "is_namespace_rooted": false,
            "container_name": "Point",
            "container_kind": "Type",
            "keyword": "fn",
            "stable_key": "rust::src/shapes.rs::Method::Point::norm::0"
          },
          "source": {
            "file_path": "src/shapes.rs",
            "signature_text": "pub fn norm(&self) -> f64"
          },
          "visibility": { "level": "Public", "is_test": false, "is_deprecated": false },
          "docs": { "raw_doc": "Distance from the origin." }
        }
      ]
    }
  ],
  "errors": []
}
```

(Pretty-printed and abridged; `hug` writes the report on one line, and real
records also carry spans, modifiers (`Static`, `Async`, …), the rest of the
visibility facet, type information, kind-specific data and provenance.
`scope` and `qualified_name` are described in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility/scope-and-qualified-names.md#scope).
A `qualified_name` that does not start at a declared namespace or package
carries the file's path from the scan root plus `#`, so running from
another directory changes only that prefix and `file_path`; the `id` and the
cache entry stay the same.)

| Field | Meaning |
|---|---|
| `report_schema_version` | version of this envelope (`1`) |
| `schema_version` | version of the records inside (`5.1`); the record format is described in [Symbol Records](../symbol-records.md) |
| `root_dir` | the scan root; every path in the report is relative to it |
| `files` | one entry per file with at least one listed record, in path order |
| `files[].completed_passes`, `completed_capabilities` | the analysis that was actually done for this file |
| `files[].symbols` | the command's records only, ordered by `--sort` |
| `files[].members` | `classes` only: each container's displayed methods and constructors in source order, keyed by the container's `id` |
| `errors` | files that could not be analyzed; always present, always last |

## What a record carries

Records carry their declared facts on every JSON format: generic parameters
and constraints (`type_info`), `modifiers`, `attributes` and the written
bases (`relations.extends`, `relations.implements` and
`relations.unclassified_bases`). In a `classes` report, a container's fields
stay in its `kind_data`; `members` lists only methods and constructors.

What a default report leaves out:

- **Source text.** Records carry the declaration header
  (`source.signature_text`) but not `declaration_text`, the body or attached
  comments; the doc comment stays as `docs.raw_doc`.
- **Work the command did not ask for.** No references, semantics,
  diagnostics or `metrics`; `completed_passes` and `completed_capabilities`
  say so. The test and deprecation flags are the exception: `--json` always
  computes them (the `flags` capability), so `"is_test": false` means "not
  test-only code", never "not checked". Their rules are in
  [Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility/documentation-flags-and-metrics.md#deprecation-and-test-flags).
- **Timestamps.** Records have none, so the same files always produce the
  same bytes: cold or warm cache, one worker or many.

## Paths

Every path is `/`-separated and relative to `root_dir`, on every operating
system: `files[].file`, `indexes[].file`, `errors[].file`, each record's
`source.file_path`, and the file segment of `identity.stable_key`. Record IDs
are not rewritten: they are derived from the file's absolute path when it was
analyzed, so the same file has different IDs in two checkouts at different
locations.

Every spelling of one file is the same file: `src/lib.rs`,
`src/../src/lib.rs`, its absolute path, and a symlink to it are analyzed
once, listed once (also when a glob matches the same file), reported under
the same path, and carry the same IDs. A symlink is reported as its target.
[cache-design.md](../cache-design.md#file-identity) describes this file
identity.

A file named on the command line from outside the scan root climbs out of
it with `..` segments:

```sh
cd /work/repo
hug --json functions --language rust /work/vendor/external.rs
# "root_dir":"/work/repo", "file":"../vendor/external.rs",
# "stable_key":"rust::../vendor/external.rs::Function::parse::0"
```

- **Symlinked spellings agree.** When a path does not lie under `root_dir`
  as written, it is compared again with the directories of both resolved
  through symlinks, so `/var/folders/…/repo/src/lib.rs` is `src/lib.rs` under
  the root `/private/var/folders/…/repo` (macOS temp directories are
  symlinked this way).
- **No relative path exists.** A file on another Windows drive or network
  share than `root_dir` has no `..` route to it. Its path is then the file's
  absolute path, `/`-separated where it has a drive (`D:/vendor/external.rs`);
  a UNC path that cannot be written with forward slashes keeps its native
  `\\server\share\…` spelling. A consumer can tell this case apart because the
  path is absolute.
- **A record's file and its key's file are projected separately.** A
  `--prelude` re-export alias is located at its target's declaration but
  keyed by the prelude file, so `source.file_path` is `src/alpha.rs` while
  `stable_key` is `rust::src/prelude.rs::Enum::prelude-export::ElevatedEnum::0`.

## Empty selections and errors

An empty selection is still a complete report:
`{"report_schema_version":1,…,"files":[],"errors":[]}`.

Each file that could not be analyzed is one `errors` entry (`file`, `kind`
and `message`), is also written to stderr, and makes `hug` exit `1`. Because
`errors` is written last, a report cut short never parses as valid JSON; see
[Failed files](./streaming-and-failures.md#failed-files).

## JSON Lines (`--jsonl`)

One line per selected declaration, each a complete envelope, so a saved or
piped line is usable without the rest of the output:

```json
{"report_schema_version":1,"schema_version":{"major":5,"minor":1},"file":"src/shapes.rs","language":"Rust","completed_passes":["Parse"],"completed_capabilities":["declarations","flags"],"record":{…}}
```

- `report_schema_version` and `schema_version` mean what they mean in the
  `--json` envelope; `file`, `language`, `completed_passes` and
  `completed_capabilities` repeat that file's values.
- `record` is one record, trimmed like `--json` records (no source text).
  Like `--json`, every line carries computed `is_test` and `is_deprecated`.
- A `classes` line's `record` is a container, and the line adds `members`:
  a **list** of that container's displayed members. The `--json` class
  report keeps a `members` **map** keyed by container ID instead, because it
  holds many containers.
- Lines follow the JSON ordering rule (files in path order, `--sort` within
  each file), never the order workers finish in, and stream file by file.
- An empty selection writes nothing. A failed file is reported on stderr and
  makes the exit status `1`, even after earlier lines were written; the
  stream has no `errors` array.

## `--json-full`

`--json-full` has the same envelope with `indexes` in place of `files`: one
complete index per scanned file, whether or not anything matched, with every
kind, every pass (`Parse`, `Bind`, `Semantic`, `Docs`, `Diagnostics`),
every capability (the test and deprecation flags and the `metrics` facet
included), imports, exports, diagnostics, `declaration_text`, and the
file's reference `occurrences` and `reference_coverage` (see
[Scope-Aware References](../references.md); their names stay in-file, without
the file prefix records carry). A
callable's `metrics.lines` and `metrics.cyclomatic` therefore appear on every
`--json-full` record, but on `--json`/`--jsonl` records only when
`--undocumented` or `--min-params` asked for metrics (see
[Cyclomatic Complexity](../cyclomatic-complexity.md)); `--stats` computes
metrics for doc coverage but reports no complexity statistic.

Because it reports everything, `--json-full` rejects any flag that narrows
records (exit `2`): a symbol-name filter, `--name`, `--exclude-symbols`,
`--exported`, `--prelude`, `--kind`, `--scope`, `--all-scopes`,
`--visibility`, `--in`, `--exclude-tests`, `--tests-only`, `--undocumented`,
`--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params`, and
for `classes` `--static-only` and `--instance-only`. File selectors (paths,
globs, `--file`, `--exclude-files`, `--language`), `--exact`, `--sort` and
`--no-cache` are allowed. `lint` and `imports`
reject `--json-full`.

```bash
hug --json-full functions greet
# error: `--json-full` reports complete indexes and cannot be combined with
#        symbol selectors (a symbol-name FILTER)
```
