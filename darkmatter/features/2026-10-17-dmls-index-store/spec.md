---
created: 2026-10-17
status: draft-spec
reviewed: false
implemented: false
review_iterations: 0
depends-on:
    - 2026-10-17-logging-improvements
related:
    - 2026-10-11-dmls-request-timeouts
human_review: true
message_to_agent: |-
    Stub. It records what the author decided on 2026-10-17 and the questions
    still open. Do not plan or implement from it until
    2026-10-17-logging-improvements has landed and the open questions are
    answered.
---

# DMLS Index Store

`2026-10-17-logging-improvements` makes DMLS parse only what it needs, and
builds a full in-memory index the first time someone asks for references,
rename, or workspace symbols. That build costs about 4 s and 680 MB on this
repository, and every server restart pays it again.

This feature moves the full index into a persistent SQLite store, so a restart
re-parses only the files that changed, and backlink and symbol lookups read
from disk pages instead of holding the whole graph in memory.

## Decided

- **The store is a cache, never the source of truth.** Deleting it is always
  safe; DMLS rebuilds it. A format version stamp that does not match the
  running server's makes the store rebuild itself.
- **One fingerprint per file:** the `biscuit-hash` xxHash of the file's raw
  bytes, the same value DMLS already computes as `DocumentIndex::content_hash`.
  Darkmatter's Markdown-aware hash is not used: it ignores whitespace and
  managed keys, which move the cached positions.
- **SQLite in WAL mode**, through the workspace's existing `rusqlite` 0.31 with
  the `bundled` feature (as `biscuit-icon` and `claudine` use it). WAL lets
  several `dmls` processes on the same workspace read concurrently with one
  writer; redb's exclusive file lock would not.
- **What it stores, per Markdown file:** path, xxHash, headings (text, slug,
  level, range), and outgoing links (target, fragment, kind, range).
- **Backlinks** are an indexed lookup from target (path, and path plus
  fragment) to source ranges.
- **Workspace symbols** scan every stored heading. The search is fuzzy, so no
  database index serves it; a scan over about 100k short strings takes
  milliseconds.
- **The same three call sites as the in-memory build** (references, rename,
  workspace symbols) read from it. Rename still revalidates every referrer it
  edits before producing edits.

## Open Questions

1. Where the store lives: which user cache directory, and how it is keyed (per
   workspace root, so each worktree has its own).
2. When the store is filled: on the first of the three features, as the
   in-memory build is, or in the background after startup once a store exists.
3. Startup revalidation: reading and hashing every file costs about 360 ms here.
   Is that acceptable in the background, or should a size and modification
   time check skip unchanged files?
4. Removing stores for workspaces that no longer exist.
5. Writer contention: what a second process does while another holds the write
   lock.
