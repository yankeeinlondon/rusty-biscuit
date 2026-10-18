---
created: 2026-10-17
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: finalized-spec
reviewed: true
reviewed_on: 2026-10-17
review_note: the clarification process served as a review
clarified: true
clarified_by: claude/default
needs_rulings: false
implemented: true
review_iterations: 0
packages:
    - darkmatter
    - dmls
    - biscuit-file
depends-on:
    - 2026-10-17-logging-improvements
related:
    - 2026-10-11-dmls-request-timeouts
references:
    spike-s1.md: >-
        Spike S1 macOS measurements against this repository: store size, memory
        split, candidate fan-out, small-graph cost, revalidation, heading scan.
    spike-s1-windows.md: >-
        Spike S1 file-sharing check on Windows and macOS: deleting, sweeping,
        or touching a WAL store another process holds open.
human_review: true
message_to_agent: |-
    Clarified and finalized on 2026-10-17; every decision below is
    author-confirmed and spike S1 has reported. Planning waits only on
    2026-10-17-logging-improvements landing: this spec amends that fix's R5
    fallback clause, removes its R7 full in-memory build, replaces its R8
    `full index built` log line, and replaces its `--bench-index` Tests clause.
implemented_by: "claude/opus"
---

# DMLS Index Store

DMLS (`darkmatter/dmls`, the Darkmatter Markdown language server) answers
four requests that need facts from every Markdown file in a workspace: find
references, heading rename, file rename, and workspace symbols. The dependency
fix `2026-10-17-logging-improvements` stops DMLS from parsing every file at
startup; its R7 instead builds a **full in-memory index** on the first such
request. On this repository that parses all 5,680 Markdown files (about 4 s
unloaded) and peaks at 550–680 MiB RSS, mostly parse high-water; the index
retained afterwards is about 97 MiB (25 MiB `DocumentIndex`, 72 MiB graph).
Every restart, in each of an editor's `dmls` processes, pays the parse again.

This feature replaces that build with a persistent SQLite store of per-file
**facts**. After a restart DMLS re-parses only files whose bytes changed, and
each request loads only the files that could matter into a small, temporary
graph. **What the user gets:** a warm restart (store present, files
unchanged) answers the first references request in ≤ 500 ms and adds ≤ 64
MiB resident. **What does not change:** a first fill, or a mass change, still
parses those files and reaches roughly today's peak (552 MiB cold fill, S1).

## Relationship to `2026-10-17-logging-improvements`

That fix (uncommitted, in progress) is a hard dependency. This feature keeps
its file list (R5), on-demand target parsing (R6, whose lazy target cache
stays separate from store rows), log style (R8), and the R7 state machine
(not-started, building, ready, failed). It **amends** that spec (which is not
edited):

- **R7:** the full in-memory build (every file into a `DocumentIndex`, a
  workspace-wide `WorkspaceGraph` kept resident) is deleted, not kept as a
  mode. "Building" now means "revalidating the store", with R7's work-done
  progress, shared build, cancellation, and reconciliation unchanged;
  "failed" means a listed file could not be read or parsed, **never** that
  the store is unavailable (R12).
- **R8:** the `full index built …` line is replaced by the R13 lines.
- **R5:** the fallback-mode "(or all entries once the full index exists)" is
  replaced: after ready, a save re-hashes only the saved file plus files with
  pending events (R6 below).
- **Tests:** "Keep `dmls --bench-index` as the full-index benchmark" is
  replaced by R14.

## Terms

| Term | Meaning |
| --- | --- |
| `DocumentIndex` | dmls `graph/substrate.rs`: one file's full parse product (headings, links, wiki links, transclusions, schema/file/variable uses, frontmatter keys) plus `content_hash` |
| `WorkspaceGraph` | dmls `graph/arena.rs`: an immutable graph of documents, nodes, and resolved edges; built by `WorkspaceGraph::build_with_roots(indices, generation, wiki_roots, contexts)`. The existing providers (references, rename, symbols) query it |
| xxHash | `biscuit_hash` (package `biscuit-hash`) xxHash of a file's raw bytes; the same value as `DocumentIndex::content_hash` |
| file list | logging-improvements R5: the in-memory list of Markdown and YAML paths, built without reading contents |
| facts | the subset of `DocumentIndex` this feature stores per file (R1) |
| candidate key | a string derived from one outgoing link that names the file it *probably* targets (R2) |
| bare path | a relative link or transclusion target with no `./` or `../` prefix and no special prefix (`a/b.md`, `spec.md`) |
| wiki universe | the list of wiki names (`WikiDoc`s) a wiki link is resolved and judged ambiguous against; today `wiki_docs` |
| `FactsSource` | new dmls trait: where facts come from, `StoreFacts` (SQLite) or `MemoryFacts` (fallback) (R12) |
| lock file | `<store>.lock`, beside each store; holders take an OS advisory lock on it (R10) |
| fallback watch mode | `WatchMode::ServerRescan` (`router.rs`): the client cannot watch files, so DMLS rescans on save |

## How it works

```mermaid
sequenceDiagram
    participant E as Editor
    participant D as DMLS
    participant S as FactsSource (one store per root)
    participant G as temporary WorkspaceGraph
    E->>D: references / rename / willRenameFiles / workspace symbols
    alt first need in this process
        D->>S: open every root's store (shared lock; sweep stale siblings)
        D->>D: read + xxHash every listed Markdown file
        D->>S: upsert changed rows, delete rows for missing paths
        Note over D,S: R7 state: building -> ready
    end
    alt workspace symbols
        D->>S: scan stored headings
        S-->>D: matching headings (deduplicated by canonical path)
    else references / rename
        D->>S: candidates_for(target), every root
        S-->>D: candidate referrer paths (union, deduplicated)
        D->>D: facts for candidates + target + open buffers
        D->>G: build_with_roots(small set, wiki universe from file list)
        G-->>D: real resolution drops false candidates
    end
    D-->>E: answer
```

The store **narrows**; the existing graph **decides**. SQL never resolves a
link: it only proposes files that might reference the target, and the same
resolver and providers DMLS uses today produce the answer.

## Requirements

### R1. Stored facts and the row invariant

Per Markdown file the store holds:

| Fact | Fields |
| --- | --- |
| file | path, xxHash |
| headings | level, title, slug, title_span, line |
| links | raw target, kind/fragment, span, line |
| wiki links | target, heading, alias, target_span, heading_span, line, unsupported |
| transclusions (`::file`, `::code`) | raw target, path, span, line |
| candidate keys | R2 |

Not stored: `schema_uses`, `file_uses`, `variable_uses`, `frontmatter_keys`.
Open-document diagnostics get those from logging-improvements' lazy R6 layer;
no provider queries those edge kinds today (S1). Rows carry line numbers
because DMLS locations are line-based (`providers/location.rs`).

**Invariant: a row's facts depend only on `(path, bytes, FACTS_VERSION)`.**
Nothing that depends on configuration, schema roots, `.dmls.toml`, or a
`FileResolutionContext` (package `biscuit-file`) is stored; resolution happens
at query time, in the temporary graph. Example: `../spec.md#goals` is stored
as its raw target and fragment, never as the path it resolves to.

The fingerprint is the raw-bytes xxHash, not Darkmatter's Markdown-aware hash,
which ignores whitespace and managed frontmatter keys that move spans and lines.

### R2. Candidate keys over-approximate; they never decide

Each outgoing link yields one or more candidate keys:

| Link form | Candidate keys |
| --- | --- |
| explicitly relative (`./a.md`, `../a/spec.md`) | the joined key: the source directory lexically joined with the target, case-folded |
| bare path (`a/b.md`, `spec.md`) | the joined key **and** the file-name key |
| wiki link (`[[spec]]`) | the file-name key |
| root-prefixed form (`~/docs/spec.md`) | the file-name key |

**Why bare paths get both:** a bare reference plans context-dependent
candidates (document directory, then repository root, …;
`FileReference::candidate_plan_with_order`). With the joined key only, S1
missed 18 of 3,017 true referrers, all bare root-relative paths (for example
`claudine/docs/research/…/codex.md` written in `claudine/docs/topics/`); with
both keys it missed none, at p95 139 candidates (file-name key alone: 332).

**File-name key (default; the implementer may refine):** the file name
case-folded, Unicode-normalized to NFC, with the Markdown extension stripped.
Wiki resolution is case-sensitive, NFC, and extension-elided
(`wiki/logical_path.rs:56`, `resolve.rs:314`), so this broader key cannot miss
a wiki referrer.

**Unusual forms (default):** percent-encoded targets, `<…>` destinations,
absolute paths, `file://` URLs, directory links, and magic roots are decoded
first; one that still yields no probable path falls back to the file-name key.

`candidates_for(target)` returns every row whose key matches the target's
path key or file-name key. False matches (two `spec.md` files; a symlinked
directory joined lexically) are removed by real resolution. A key must never
**miss** a true referrer: when in doubt, emit a broader key.

### R3. The temporary graph

For references (including DSL "transcluded by", `providers/dsl.rs`, and wiki
references, `providers/wiki.rs`), heading rename, and file rename
(`workspace/willRenameFiles`, `providers/rename.rs::will_rename_files`):

1. take the candidate paths from R2 (union over every root, R8) and the target,
   **deduplicated by canonical path**;
2. load their facts from the `FactsSource`, adding every open buffer (R5);
3. build a `WorkspaceGraph` with `build_with_roots` over that set only, with
   the wiki universe supplied externally (below);
4. run the existing provider; drop the graph after the request.

**Wiki universe API change.** Today `WorkspaceGraph::assemble`
(`graph/arena.rs:334`) builds `wiki_docs` from the parsed documents, and
`apply_file_rename` / `safe_new_spelling` (`providers/rename.rs` ~261-275)
index `graph.wiki_docs()` by `DocumentId`, so a small graph would judge
ambiguity and pick spellings against only a few files. Therefore
`build_with_roots` gains an externally supplied wiki-name universe, built from
the file list and the wiki roots; rename reads it from there; and ambiguity
diagnostics and rename spellings equal the full-graph oracle's (Tests).

Referrers and targets outside the workspace behave as today: they are not in
the file list and are not candidates.

**Workspace symbols** do not build a graph: they scan stored headings of every
root and emit document titles (first level-1 heading, else first heading, else
file name, as `providers/symbols.rs::document_title` does) plus every heading,
fuzzy-matched and deduplicated by canonical path. A full scan suffices; no
database index (S1: 75,401 headings, 28 ms load, 11–13 ms per query).

### R4. Fill and revalidation on first need; "ready"

Nothing happens at startup. The first of the four requests in a process:

1. opens the store of **every** workspace root (R7, R8); runs the sweep (R10);
2. reads and xxHashes every Markdown file in the file list (S1, warm cache:
   about 50 ms with 16 threads, 110–150 ms sequential; the file-list walk
   itself, 190–300 ms, is logging-improvements R5 and not part of this pass);
3. re-parses only files whose hash differs from their row, or that have none;
4. upserts those rows in batches (R11) and deletes rows whose paths are
   missing on disk;
5. answers.

**Ready** means: every listed Markdown file has a row whose hash matches
bytes read **in this process**, and no row exists for a path missing on disk.
An unreadable listed file means failed (R7). Symlinks and duplicates follow
logging-improvements R5's duplicate policy.

There is no size-and-mtime fast path: a false "unchanged" leaves a stale row
missing a new link, which is never a candidate, so rename silently misses it.
Mtimes are unreliable on FAT/exFAT (2 s), after `cp -p`, `rsync -t`, or `tar`,
under clock skew, and on WSL drvfs. The hash pass costs about 45 ms more (S1).

### R5. Open buffers override rows

The store holds disk state only. **Every** open buffer, saved or not, joins
the candidate set and the temporary graph, its parse replacing its row for that
request. A buffer never writes a row.

### R6. Live updates and rename freshness

- **Before ready**, events update only the file list (as in
  logging-improvements R5); the first need revalidates everything anyway.
- **After ready, client watching**: trusted events re-read, re-hash, and
  upsert or delete rows file by file.
- **After ready, fallback watch mode**: a save re-hashes only the saved file
  plus files with pending events; **before each heading or file rename**, a
  full re-hash pass (R4 steps 2-4) runs, so a file changed outside the editor
  that gained a link to the target is a candidate.
- **Every rename** re-reads and re-hashes each referrer it edits, re-parsing
  one whose bytes no longer match its row.

### R7. Location, override, kill switch, and permissions

- New public function `biscuit_file::user_cache_dir() -> Option<PathBuf>` in
  `biscuit-file` (default signature), built on `biscuit_file::home_dir()` and
  environment variables: Linux `$XDG_CACHE_HOME`, else `~/.cache` (a relative
  or empty `$XDG_CACHE_HOME` is treated as unset, per the XDG specification);
  macOS `~/Library/Caches`; Windows `%LOCALAPPDATA%`. DMLS must not use
  `dirs::cache_dir` directly. `None` means memory fallback (R12).
- Store directory: `<user_cache_dir>/darkmatter/dmls/`.
- Override: environment variable `DMLS_CACHE_DIR` only; no `.dmls.toml` key (a
  committed file should not choose a machine's cache location), no CLI flag. A
  missing override directory is created; if creation fails, memory (default).
- **Kill switch:** `DMLS_CACHE_DIR=off` uses `MemoryFacts` for the process
  lifetime (R12).
- Unix: directory `0700`, database and lock files `0600` (paths and headings
  from possibly private repositories). Windows: the default `%LOCALAPPDATA%`
  ACL suffices; no ACL change (default; documented).
- This deliberately differs from the remote transport cache, whose
  no-fallback rule (darkmatter skill `remote-cache.md`) applies to it only.

### R8. Keying, file name, and multiple roots

- One store per **effective workspace root** (`workspace.root` applied, as in
  R5 of the dependency). Each worktree therefore has its own store.
- **Multiple roots:** a process opens and revalidates every root's store on
  first need (R4) and unions their candidates (R3), so cross-root references
  and renames stay complete, as with today's single graph.
- **Nested roots** may store one file twice (space only); candidates and
  workspace-symbol results are deduplicated by canonical path (R3).
- Key: xxHash of `biscuit_file::canonicalize_simplified(root)`, as 16 hex
  digits. No manual lowercasing; an L1 test pins macOS's canonicalization case
  behavior (worst case: a duplicate store).
- File name: `<root-xxhash>-f<FACTS_VERSION>.sqlite` (plus `-wal`, `-shm`,
  and the R10 lock file), e.g. `3f9a0c1d2e4b5a69-f1.sqlite`, so builds with
  different `FACTS_VERSION`s coexist.
- A `meta` table records the store format, `FACTS_VERSION`, the root path, and
  `last_opened`. A root mismatch on open (a hash collision) deletes and
  refills the store (under R10's exclusive lock). A format stamp that does not
  match the file name's version is treated as corruption (R12).
- Configuration is not part of the key (R1 invariant).

### R9. Version stamp and golden test

Two constants in dmls: `STORE_FORMAT` (table layout) and `FACTS_VERSION` (what
extraction produces). Not the crate version, not a build fingerprint. A golden
test extracts facts from a fixture corpus and compares them with a checked-in
snapshot; it fails when the snapshot changes without a `FACTS_VERSION` bump,
so a parser change cannot silently serve stale facts.

### R10. Liveness lock and sweep

**Why a lock file:** "the delete fails" is a liveness test on Windows only
(os error 32 on every file of an open store, S1). On Unix the unlink
succeeds; the holder keeps writing to an orphaned inode, a new store at the
same path splits the brain, and the holder's close can delete its `-wal`.

**Lock rules (every OS):**

- every process holding a store open holds a **shared** OS advisory lock
  (`flock` / `LockFileEx`) on `<store>.lock` for the connection's lifetime;
- the sweep and corruption repair (R12, and R8's root-mismatch refill) first
  take an **exclusive non-blocking** lock; if it is held, the sweep skips that
  store and repair falls back to memory;
- on Windows, a delete failing with `raw_os_error() == Some(32)` also means
  "in use, skip" (extra safety; its `io::ErrorKind` is `Uncategorized`);
- the lock helper (`fs4`/`fs2` or std `File::try_lock`, Rust 1.89+) is the
  implementer's choice after checking the toolchain and existing dependencies.

**Sweep.** On the first store open in a process (never at startup), delete
sibling store files (`.sqlite`, `-wal`, `-shm`, `.lock`) whose newest mtime
is older than **30 days**, unless the lock rules say the store is in use.
Never delete a store because its root is missing (an unmounted `/Volumes`
disk would wipe valid stores).

A live store's `meta.last_opened` and mtime are touched on open and on every
revalidation and batch (default); the mtime touch opens the file **for
write** (on Windows a read-only handle fails with os error 5). Old
`FACTS_VERSION` files expire through the sweep. No prune subcommand; docs
state the directory is always safe to delete by hand.

### R11. Multiple processes write safely

- SQLite in WAL mode via `rusqlite` 0.31 (`bundled`, as `biscuit-icon` and
  `claudine` use it; chosen over redb, whose exclusive file lock prevents
  sharing); `busy_timeout = 5000` ms, `synchronous = NORMAL`.
- **WAL switch on open:** `PRAGMA journal_mode=WAL` on a new file can return
  `SQLITE_BUSY` at once, ignoring the busy timeout (S1: 2 of 3 concurrent
  creations). Open retries it, bounded, with short backoff (S1 suggests about
  2 s); persistent failure is "busy", never corruption: memory (R12).
- Every process may write, in batches of **200** files per `BEGIN IMMEDIATE`
  transaction; writes are idempotent (keyed by path, skipped when the stored
  hash matches).
- Rows are deleted only for paths missing on disk, never for "absent from my
  file list" (builds may differ in ignore rules).
- A process uses another's row only when its hash equals its own read of the
  bytes: shared rows save parsing, never decide correctness.

### R12. Failure fallback: recreate once, then memory

`FactsSource` backends: `StoreFacts` (SQLite) and `MemoryFacts`
(`HashMap<PathBuf, FileFacts>`; `FileFacts` is the R1 row; about 16 MiB for
this repository per S1).

| Failure | Behavior |
| --- | --- |
| corrupt / not a database / format mismatch | take the R10 exclusive lock, delete and recreate once; if the lock is held or the delete fails (Windows os error 32), use memory |
| read-only directory, disk full, cannot open, busy beyond the timeout, WAL switch still busy after its retry | use memory |
| write error mid-fill | switch to memory, keeping facts already computed |
| `DMLS_CACHE_DIR=off`, or no cache directory | use memory |

**A cache problem never fails an editor request.** A process in memory stays
there until exit and logs one `warn` (default); it holds facts, not parse
trees. A `DMLS_CACHE_DIR` on NFS, SMB, or WSL drvfs (`/mnt/c`) is documented
as unsupported; nothing detects it.

### R13. Log lines

Logging-improvements R8 style (`workspace` field on each line); defaults the
implementer may refine.

| When | Level | Line |
| --- | --- | --- |
| store opened | `info` | `index store opened path=… created=false facts_version=… rows=…` |
| store created | `info` | `index store opened path=… created=true facts_version=…` |
| revalidation finishes | `info` | `index store revalidated markdown=… unchanged=… reparsed=… removed=… trigger=references elapsed_ms=…` |
| falls back to memory | `warn` | `index store unavailable, using memory reason=… path=…` |
| store recreated after corruption | `warn` | `index store recreated path=… reason=…` |
| sweep removed stores | `info` | `index store sweep removed=N skipped_in_use=… elapsed_ms=…` |

`trigger` takes R8's values. The fallback `warn` is emitted once per process.
Never log file contents or heading text.

### R14. `dmls --bench-index`

`dmls --bench-index <dir>` (`dmls/src/bench.rs`) replaces the full-build
benchmark. It reports store fill and revalidation, each **cold** (no store)
and **warm** (store present, files unchanged), plus the warm-restart
first-references latency (with and without the file-list walk) and added
resident memory that acceptance 11 is checked against.

## Decisions and alternatives

| Decision | Rejected alternative | Reason |
| --- | --- | --- |
| store narrows, graph decides (C) | A: warm-start the full in-memory graph from the store | A needs a full graph build per process (0.5–1.9 s in S1) and keeps about 97 MiB resident; C needs about 5 MiB and 0.1 s per request (S1) |
| | B: answer directly in SQL | duplicates link and wiki resolution; rename results drift from the resolver |
| joined key plus file-name key for bare paths | joined key only; file-name key only; suffix key | joined-only missed 18 of 3,017 referrers; name-only reaches p95 332 candidates; suffix is barely narrower (p95 136 vs. 139) |
| external wiki universe from the file list | placeholder documents; full path-only graph | the small graph must judge wiki ambiguity against every listed file |
| revalidate on first need | background revalidation at startup; tiered freshness | nothing runs at startup; background revalidation can be added later without a store change |
| full hash pass | size + mtime fast path | false "unchanged" silently breaks rename (R4); the hash pass costs about 45 ms more |
| full heading scan | a database index or narrowing step | 28 ms load, 11–13 ms per query (S1) |
| user cache dir via `biscuit-file` | `dirs::cache_dir` in dmls; a `~/.darkmatter` dotdir; inside the workspace | home resolution has one authority, `biscuit_file::home_dir()` |
| per effective root, all roots queried | per process; one store per git repository | per process loses restarts; a per-repository shared store is a possible future feature |
| age-based sweep behind a lock file | delete when the root is missing; delete-fails-means-in-use alone; prune subcommand | unmounted volumes; Unix unlinks a live store; manual deletion already suffices |
| version in file name | one file, rebuilt on mismatch | two builds on one workspace would rebuild each other forever |

## Spikes

**S1: done** (2026-10-17). Reports: [spike-s1.md](./spike-s1.md) (macOS
measurements) and [spike-s1-windows.md](./spike-s1-windows.md) (Windows and
macOS file-sharing check). No further spikes.

| Measurement | Result | Applied in |
| --- | --- | --- |
| store size; write all 5,680 files | 12.4 MiB (+4–6 MiB WAL before checkpoint); 0.2–0.6 s | R11 |
| full-index memory | 680 MiB peak is parse high-water; retained 97 MiB (25 MiB `DocumentIndex`, of which 16 MiB R1 facts; 72 MiB graph); cold fill peaks 552 MiB | intro, decisions |
| candidate fan-out (3,017 true pairs) | joined only: 18 misses; joined + bare-name: 0 misses, p95 139, max 146; name only: p95 332 | R2 |
| small graph per request | 63–103 ms total, 22–29 MiB peak RSS, 2–6 MiB heap; answers equal the full graph | R3, acceptance |
| heading scan, 75,401 headings | 28 ms load, 11–13 ms per query | R3 |
| revalidation (warm) | read + hash 48–53 ms (16 threads); metadata only 4–5 ms; discovery 186–297 ms | R4 |
| two concurrent cold fills, batch 50 / 200 / 1000 | `BEGIN IMMEDIATE` never busy; worst wait 218 ms; batch size not a lever; WAL switch busy at once in 2 of 3 creations | R11 |
| Windows: delete or sweep an open store | os error 32 on every file; skip works; repair delete fails, so memory fallback is reachable; read-only `set_modified` fails with os error 5 | R10, R12 |
| macOS: delete an open store | unlink succeeds; holder writes to an orphaned inode (split-brain) | R10 |

**Caveats.** Warm cache only; cold-disk figures are left to the
implementation's `--bench-index`. The macOS host was heavily loaded (load
30–190; parse times about ten times inflated), so ratios beat absolutes. The
tree changed during the spike (other sessions edited `darkmatter/dmls` and
corpus files). Small-graph runs excluded open buffers, the external wiki
universe, and edit construction; concurrent fills were not row-compared.

## Acceptance criteria

Done when each statement holds and has a test or recorded measurement:

1. A restart with no file changes re-parses **zero** files before answering
   the first references request (parse-counting seam).
2. Changing one file between restarts re-parses exactly that file.
3. After a restart, heading rename and file rename include an edit in a
   referrer that was never open and never referenced by an open document.
4. Every request answers identically, on a fixture corpus, to a test-only
   full-graph oracle (references, DSL transcluded-by, wiki references, heading
   rename, file rename, workspace symbols), including wiki ambiguity
   diagnostics and file-rename wiki spellings.
5. A candidate set never omits a true referrer (R2), on the same corpus,
   including bare repository-root-relative paths.
6. In a two-root workspace, a file rename in one root edits a referrer in the
   other root, after a restart; with nested roots, no result appears twice.
7. In fallback watch mode, a heading or file rename after an unwatched
   on-disk change that added a referrer includes an edit in that referrer.
8. With the store directory read-only, corrupt, locked, or disabled by
   `DMLS_CACHE_DIR=off`, all four requests still answer correctly from memory,
   with one `warn`; `off` creates no store file.
9. Two processes filling one store concurrently both reach ready with
   identical rows, including when both create the store at once.
10. No process deletes or recreates a store another process holds open, on any
    OS: the sweep skips it and repair falls back to memory.
11. **Warm restart on this repository** (store present, files unchanged), as
    reported by `dmls --bench-index` (R14): the first references answer takes
    ≤ **500 ms** excluding the file-list walk and ≤ **1 s** including it, and
    serving references, rename, and symbols adds ≤ **64 MiB** resident memory.
    A cold fill has no target; it is measured and recorded only.
12. The production in-memory full-build code path from logging-improvements R7
    no longer exists; the full-graph oracle lives only in tests.

## Tests

Run through `just test` (nextest) in `darkmatter/`, with deterministic read
and parse counters and injected seams rather than timing or permission tricks.
"Oracle" is a test-only full-graph build over the whole fixture corpus.

| Requirement | Tier | Test |
| --- | --- | --- |
| R1 | L1 dmls | round trip: extract facts, write, read back equal; repeated write/read/write is stable |
| R1, R9 | L1 dmls | golden: fixture corpus facts equal the checked-in snapshot; changed snapshot without `FACTS_VERSION` bump fails |
| R2 | L1 dmls | over-approximation: for every target in the corpus, candidates ⊇ oracle referrers, including case, NFC, extension-elided wiki links and the unusual link forms |
| R2 | L1 dmls | bare-path pattern from S1's 18 misses: a bare repository-root-relative link and `::file` in a nested document (`docs/topics/a.md` linking `area/docs/b.md`) is a candidate for its target; an explicitly relative `../` link yields no file-name key |
| R3 | L1 dmls | each of the four entry points answers identically to the oracle (acceptance 4) |
| R3 | L1 dmls | wiki: an ambiguous `[[name]]` whose rival is not a candidate is still ambiguous; file-rename spellings equal the oracle's |
| R3, R8 | L1 dmls | nested roots storing one file twice: candidates, references, and workspace symbols contain it once |
| R4 | L1 dmls | restart with unchanged files parses zero; one change parses one; deleted file's row removed; unreadable file fails |
| R5 | L1 dmls | open buffers (unsaved, and saved but differing from the row) override rows; no buffer writes a row |
| R6 | L1 dmls | after ready, a watched-file event updates one row; before ready, it touches only the file list; a fallback-mode save re-hashes only the saved file plus files with events |
| R6 | L1 dmls | fallback mode: change a file on disk with no event to add a link, then rename; the new referrer is edited (acceptance 7) |
| R6 | L1 dmls | rename re-reads a referrer changed on disk after ready and edits the new bytes |
| R7 | L1 biscuit-file | `user_cache_dir()` per OS with injected environment and home; relative or empty `$XDG_CACHE_HOME` treated as unset |
| R7 | L1 dmls | `DMLS_CACHE_DIR` override; missing directory created; Unix modes `0700`/`0600` (`#[cfg(unix)]`) |
| R7, R12 | L1 dmls | `DMLS_CACHE_DIR=off`: memory backend, no store file created |
| R8 | L1 dmls | key is stable; root mismatch in `meta` refills; macOS case behavior of `canonicalize_simplified` pinned |
| R8 | L1 dmls | two roots, two stores: cross-root references and file rename match the oracle (acceptance 6) |
| R10 | L1 dmls | sweep deletes stale siblings including `.lock` (injected clock or set mtimes), keeps fresh ones, keeps stores with missing roots, keeps a store touched by a recent batch |
| R10, R12 | L1 dmls | lock contention: a second process (or connection with its own lock handle) holds the shared lock on an aged store; the sweep skips it and corruption repair falls back to memory |
| R10 | L1 dmls, `#[cfg(unix)]` | unlink-while-open: with a holder open and its mtime aged past 30 days, the sweep does not unlink any of its files, and the holder's later writes are visible to a fresh connection at the same path |
| R10, R12 | L1 dmls, Windows | an open store fails to delete with os error 32 and is skipped; the mtime touch through a write handle succeeds (native Windows leg) |
| R11 | L1 dmls | two connections writing interleaved batches converge to identical rows; a foreign row with a mismatching hash is re-parsed |
| R11, R12 | L1 dmls | WAL-switch retry: two openers create one new store concurrently (barrier); both open in WAL mode, or a seam that keeps the switch busy past the bound yields memory with one `warn`, never a recreate |
| R12 | L1 dmls | fault-injecting `FactsSource` seam: corrupt file recreated once; second failure uses memory; mid-fill write error keeps computed facts; one `warn`; memory persists for later requests |
| R13 | L1 dmls | captured `tracing` subscriber shows each line and field once |
| R14 | L1 dmls | `bench_index` on a small fixture reports cold and warm fill, revalidation, and warm-restart latency and memory sections |

**Numeric targets** (acceptance 11) are **not** an L1 gate; timing and RSS on
shared runners are too noisy. The implementer runs `dmls --bench-index` on
this repository on a quiet host and records the warm-restart figures and the
cold fill (cold disk where `sudo purge` is available) in the implementation
log; a miss returns to the author. L1 asserts only deterministic proxies:
zero re-parses on a warm restart (acceptance 1) and the report's shape (R14).

## Documentation

Canonical pages for a developer new to this repository (example per rule,
Mermaid where clearer); they never name this spec.

- `darkmatter/docs/lsp/indexing.md` (created by logging-improvements): the
  store (what it holds, why it is only a cache), the sequence diagram, ready
  and revalidation, open-buffer override, rename freshness per watch mode,
  multiple and nested roots, location, `DMLS_CACHE_DIR` and `off`, lock file
  and sweep, multiple processes, fallback, the network filesystem limit,
  "always safe to delete", sizing (about 12 MiB here), R13 lines, and
  `--bench-index`. Remove the in-memory full-build description.
- `darkmatter/docs/lsp/configuration.md` (created by logging-improvements):
  `DMLS_CACHE_DIR` and `off`, and why it is not a `.dmls.toml` key.
- `darkmatter/docs/lsp/architecture.md`: the full index, and the external
  wiki universe of `build_with_roots`.
- `biscuit-file/docs/topics/` and `.claude/skills/biscuit-file/`:
  `user_cache_dir()` beside `home_dir()`.
- `darkmatter/docs/dependencies.md`, `docs/dependencies.md`: `rusqlite`
  (bundled) in `dmls`, and any lock helper crate.
- `.claude/skills/darkmatter/dmls.md`: the store, `FactsSource`, the lock
  file, the removed full build; `remote-cache.md`: its no-fallback rule does
  not apply to the DMLS store.
- `.claude/skills/os/` (repository rule for OS facts): os error 32 is
  `Uncategorized`, `set_modified` needs a write handle, Unix unlink-while-open.
- **Outside the repository diff:** a separate step edits the user-global
  `~/.claude/skills/storage-strategy` skill (no in-repo copy) to name
  `<user_cache_dir>/darkmatter/dmls/` as always safe to delete.

## Proposed symbols

To be confirmed in planning: `biscuit_file::user_cache_dir()`; in dmls an
`index_store` module with `IndexStore` (`open`, `upsert_batch`,
`remove_missing`, `candidates_for`, heading scan), `store_path(root)`,
`STORE_FORMAT`, `FACTS_VERSION`, the `meta` table, the store lock,
`sweep_stale_stores(dir, max_age)`, `FactsSource` (`StoreFacts`,
`MemoryFacts`), `FileFacts`, `StoreFailure`; a wiki-universe parameter on
`WorkspaceGraph::build_with_roots`; `rusqlite` 0.31 (bundled) in dmls.

## Out of Scope

- Background revalidation at startup (addable later without a store change).
- A store shared per git repository across worktrees.
- A prune subcommand or cache-size limit.
- Storing schema uses, file uses, variable uses, or frontmatter keys.
- Detecting network filesystems.
- Making a cold fill cheaper than today's full parse.
- Folder renames: a directory `willRenameFiles` makes no edits today (early
  return in `providers/rename.rs`) and still makes none; possible follow-up.

## Open Questions

None.
