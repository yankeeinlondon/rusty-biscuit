# Entry Format and Fingerprint

Part of [Tree Hugger's Persistent Symbol Cache](../cache-design.md).

This page describes what an entry's key is made of, how a file's identity is
decided, which inputs invalidate an entry, and every field an entry stores.
Read the [entry page](../cache-design.md) first for how the cache is used.

## The key

The entry's **key** is a digest of a fixed `symbols` tag and four things:

1. the file's [identity](#file-identity), with `\` written as `/`;
2. the xxHash of the file's text, computed from the same bytes that are
   parsed (the file is read once);
3. the language, when it was forced with `--language`; otherwise the word
   `detected`;
4. the [records fingerprint](#the-fingerprint).

Editing the file changes its hash, so the next run uses a new key and the old
entry is never read again.

Neither the scan root nor a library caller's `NamingContext` is a key input.
Neither changes a stored record (the scan root only changes how paths and
qualified names are displayed, which happens after the lookup), so keying on
them would only split one file's analysis into several identical entries.
Likewise a query's predicates (scope, visibility, names, flags, signature
shape) are applied after the lookup: the entry always holds every record of
its covered kinds under its in-file name, and a predicate only adds the kinds
and capabilities it reads to what the lookup must cover.

Every digest is BLAKE3 (from `biscuit-hash`) over a canonical encoding:
`canonical_digest` writes each field as its byte length, `:` and its bytes,
so `["a|b", "c"]` and `["a", "b|c"]` can never collide.

## File identity

A file can be named many ways: `src/lib.rs`, `./src/../src/lib.rs`, its
absolute path, a symlink to it, or (on macOS) `/var/…` and `/private/var/…`
for the same temporary file. All of them are one file, so they must share
one entry, and `--prelude` must find the analysis it already made under
whatever name the scan lists the file. `SourceFile` therefore reads the file
through the name it is given and then analyzes it under its **identity**,
which `tree_hugger::file_identity` computes:

- the path made absolute, with `.`, `..` and every symlink resolved
  (`std::fs::canonicalize`), and on Windows written in its ordinary
  `C:\…` form rather than `\\?\C:\…`, with long names in place of 8.3
  short names;
- if the path cannot be resolved (the file vanished after it was read), the
  path made absolute against the working directory, unresolved.

```text
src/lib.rs                  ┐
./src/../src/lib.rs         │
/repo/src/lib.rs            ├──> identity /repo/src/lib.rs ──> one key, one entry
lib_link.rs -> src/lib.rs   ┘
```

**Symlinks share their target's identity.** A symlink and the file it points
to are analyzed, cached and reported as the target: the same entry, the same
records, the same `file_path` and the same IDs. Language detection reads the
identity too, so a symlink named `script` that points at `script.rs` is Rust.

**Errors still use the name you gave.** A missing or unreadable file is
`TreeHuggerError::Io` for that spelling, and an undetectable language names
it as well. The file's bytes are read once, through that spelling; resolving
the identity reads no file contents.

**Library callers pick the constructor by what they hold.** Every path the
scanner (`collect_files`) returns is already an identity, so `hug` reads
scanned files with `SourceFile::read_identified`, which keeps the path it is
given instead of resolving it a second time. That constructor is only for
identities: any other spelling would become the file's path, key and IDs.
Library callers with arbitrary paths use `SourceFile::read` or
`read_with_language`.

The same identity is used everywhere `hug` compares files:

- `scanner::collect_files` and `collect_selected_files` (which adds the
  files `hug --file` names) return identities, so a file named twice under
  different spellings, or named and also matched by a glob, is analyzed and
  listed once;
- `hug` resolves its working directory to its identity before deriving the
  scan, Git and package roots, so they are spelled like the files beneath
  them and paths display relative to them;
- `--changed` compares scanned files with its changed set by identity;
- the `--prelude` reuse map is keyed by identity;
- `god-files` reports paths relative to its root's identity.

Reports never show the identity directly: JSON paths are projected relative
to the scan root and terminal paths relative to the Git root
([Symbol Reports](../symbol-reports.md)).

### What decides a declaration's ID

A record's stable key, from which its ID is derived, is
`{language}::{file identity}::{kind}::{qualified name}::{ordinal}` (the full
recipe is in [Field Reference](../symbol-records/fields.md#ids)). Two
consequences follow.

**IDs follow the identity.** One file in one checkout has the same IDs
whichever spelling named it and whichever scan root reached it. They are not
equal across checkouts in different directories, since the identity is
absolute.

**IDs follow the kind and the qualified name.** An extractor change that
reclassifies a declaration gives its record a new ID, and so does one that
changes a name segment above it. When an extractor revision began reporting
Python `__init__` as a Constructor instead of a Method, its ID changed; a
constructor parameter kept its own, because its kind and qualified name
(`C.__init__.value`) did not change. Records that did not change keep their
IDs, including the members of a namespace that became a record of its own
(`App\inside` stays `App\inside`). Old IDs are never kept as aliases:
anything that stored an ID across an extractor upgrade must look it up
again.

## The fingerprint

The **records fingerprint** is a `PassFingerprint` computed from everything
besides the file's text that decides its records, imports, exports and every
analysis pass but diagnostics:

| Input | Where it comes from | Changes when |
|---|---|---|
| schema version | `SchemaVersion::CURRENT` (5.1) | the record shape or meaning changes |
| implementation version | crate version and `EXTRACTOR_REVISION`, written `<version>+rev.<revision>` (for example `0.1.0+rev.50`) | extraction code changes |
| grammar identity | the compiled grammar: name, declared version (if any), ABI, node-kind, field and parse-state counts | a grammar crate is upgraded |
| grammar ID | the grammar variant (`typescript` or `tsx`, …) | the file's grammar variant differs |
| query text | the effective `locals`, `imports` and `references` query text, with inherited vendored queries and overlays resolved | a `.scm` file the analysis compiles changes |

A change to Rust extraction code cannot be seen in query text, so
`EXTRACTOR_REVISION` must be bumped with any change to extracted records,
including a repair on an unreleased branch and a change to a
[cyclomatic counting rule](../cyclomatic-complexity.md), which changes the
numbers reported for unchanged source. After a bump, every entry written by
the earlier revision is a miss and is recomputed on first use; nothing is
migrated.

### The diagnostics fingerprint

Diagnostics depend on more inputs, which only diagnostics use. They have
their own fingerprint, stored **inside** the entry rather than in the key, so
a rule change does not throw away a file's symbols:

| Input | Changes when |
|---|---|
| schema, implementation and grammar inputs | as above |
| `lint` and `comments` query text | a lint pattern or the ignore-comment query changes |
| built-in rule metadata (ID, version, category, default severity, confidence, languages, default enablement, experimental flag of every rule) | a rule is added or its metadata changes |
| `experimental_semantics` | `hug lint --experimental-semantics` is toggled |

Stored diagnostics with a different diagnostics fingerprint are treated as
not computed: a symbol listing is still served from the entry, and a request
that needs diagnostics recomputes them and rewrites the entry. Lint policy
(`--deny`, `--warn`, `--allow`, `--strict`) and external adapters such as
Oxlint run after the cache and are not inputs.

The query text is an injectable input (`SymbolCache::with_query_text`,
`FingerprintInputs::with_query_text`), so a test can change one query's text
and watch the entry stop matching without rebuilding the crate.

## What an entry holds

An entry for `shapes.rs` after `hug functions`, trimmed:

```json
{
  "schema_version": { "major": 5, "minor": 1 },
  "fingerprint": "9a3ba83e2881d4a7e866a9250509933c5f550fe846be9f30bafbc38bc7770065",
  "file": "/repo/src/shapes.rs",
  "language": "Rust",
  "source_hash": "f4507a7ed1c6c9b2",
  "covered_kinds": ["Function", "Method", "Constructor"],
  "completed_capabilities": ["declarations"],
  "diagnostics_fingerprint": null,
  "records": [ ... ],
  "imports": [],
  "import_inventory": "d53d18c23212ea7b6300594bb89bce60218f6eff2b9d628b8cc42d3e79bbd5ab",
  "exports": [ ... ],
  "diagnostics": [],
  "occurrences": [],
  "reference_coverage": [],
  "reexports": null
}
```

- `fingerprint` is the [records fingerprint](#the-fingerprint) digest, and
  `source_hash` the xxHash of the text the entry was written from.
- `file` is the path exactly as analyzed (the file's
  [identity](#file-identity)), since the records repeat it.
- `covered_kinds` is the string `"all"` or a non-empty list of symbol kinds
  in ascending order, such as `["Function", "Method"]`. An empty list is
  never written and never read as "all".
- `completed_capabilities` lists the
  [capabilities](../symbol-records/file-index.md#capabilities) (analysis results) the entry
  holds, in ascending order and closed under prerequisites, so it always
  contains `declarations`.
- `diagnostics_fingerprint` is `null` exactly when `diagnostics` is not
  among the capabilities, and then `diagnostics` is empty.
- `records` holds every record of the covered kinds, after the entry's
  passes ran, under their in-file qualified names.
- `imports` holds the file's imports, and `import_inventory` a digest of the
  complete import list the grammar produced, which the reader uses to detect
  a lost or invented import (see
  [Stored import evidence](./robustness.md#stored-import-evidence)).
- `occurrences` and `reference_coverage` hold the whole file's
  [reference evidence](../references.md) when `occurrences` is among the
  capabilities, and are empty otherwise; under that capability
  `reference_coverage` always starts with the file-level region. Both keys
  are required, and so is every key inside them: unlike the public index,
  the entry writes an absent owner or ID as `null` and an empty candidate or
  reason list as `[]` (see
  [Reference evidence](./robustness.md#reference-evidence)).
- `reexports` is required and nullable: the file's re-export report when
  `reexports` is among the capabilities, and `null` exactly when it is not.
  Each import also carries a required `reexported` boolean, which may be
  `true` only under that capability. An entry that has a report without the
  capability, the capability without a report, or a marker without the
  capability is refused and recomputed. So is a report whose well-typed
  content no analysis could produce: a state that disagrees with the
  language or with its limits, spans outside the file, entries out of
  source order, names that contradict the form, an `import_span` that is not
  an import, or a marker that disagrees with the entries.

An entry is internally consistent by construction, and the reader checks it:
an entry whose contents contradict its coverage is refused. The
[Damaged Entries](./robustness.md) page lists every check.

## Related pages

- [Tree Hugger's Persistent Symbol Cache](../cache-design.md): the entry page.
- [How a Lookup Is Decided](./lookup.md): how an entry is used, widened and
  narrowed.
- [Damaged Entries](./robustness.md): which damaged entries are served.
