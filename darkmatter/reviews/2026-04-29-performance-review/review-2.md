---
agent: "Codex"
model: "GPT-5"
repo: "rusty-biscuit"
created: "2026-04-30"
---

# Performance Review Follow-up

## Summary

I re-reviewed the recommendations from `review.md` against the current workspace. The highest-impact quick wins are implemented: ANSI regex recompilation is gone from the block constraint hot path, filesystem sorting now precomputes case-insensitive keys, UID/GID lookups are cached per tree build, and `CodeHighlighter` now borrows the static `SyntaxSet`.

Two recommendations are not fully fixed:

1. `tree-hugger` owner lookup is indexed, but still has worst-case linear scanning per reference.
2. `biscuit-file` PDF text normalization is still the same two-pass implementation.

The markdown renderer allocation work is partially addressed: the main prose wrapper path now appends into an existing buffer and `LineWrapper` flushes at block-safe boundaries, but several helper paths still allocate intermediate strings.

## Findings

### Medium: `OwnerSymbolIndex::find` still has worst-case O(S x R) behavior

Location: `tree-hugger/lib/src/analysis/mod.rs:327`

The original linear `find_owner_symbol` was replaced with `OwnerSymbolIndex`, sorted by `start_byte`, and lookup now begins with `binary_search_by_key`. That is progress, but `find` then scans every earlier entry:

```rust
for entry in self.entries[..=idx].iter().rev() {
```

Because `end_byte` is not monotonic, the implementation cannot break when a candidate fails containment. The comments acknowledge this at lines 348-351. In the worst case, a reference near the end of a file with many preceding symbols still scans O(S), so the bind pass remains O(S x R) for adversarial or large flat files.

There is also a smaller correctness risk in the comment at line 328: `binary_search_by_key` is documented to return any matching equal key, not necessarily the rightmost one. If multiple symbols have the same `start_byte`, entries after the returned match may be skipped. Current tests cover two same-start entries and pass, but they do not make the rightmost-match assumption guaranteed.

Suggested fix: sort by `(start_byte, end_byte)` and use `partition_point(|e| e.start_byte <= start_byte)` to get the true upper bound, then add an interval index or an auxiliary prefix-max-end structure so lookup can stop early. If keeping the current approach temporarily, update the comment and add tests with three or more identical `start_byte` spans.

### Low: PDF normalization recommendation was not implemented

Location: `biscuit-file/lib/src/pdf/backends.rs:38`

`normalize_text` still performs a dehyphenation pass into `dehyphenated`, then a second whitespace-collapse pass into `result`:

```rust
let dehyphenated = { ... };
let mut result = String::with_capacity(dehyphenated.len());
for ch in dehyphenated.chars() { ... }
```

The behavior tests pass, but the performance recommendation from the first review was specifically to combine this into one pass to avoid the extra full-size allocation and traversal. This is still outstanding.

Suggested fix: use one state machine over `text.chars().peekable()` that skips `-\n` / `-\r\n` soft breaks, collapses whitespace as it emits, and avoids the final `trim().to_string()` by tracking whether any non-whitespace has been emitted.

### Low: Markdown renderer allocation work is partial

Location: `darkmatter/lib/src/markdown/output/terminal.rs:1711`, `:1837`, `:1873`, `:2454`

The hot prose path now uses `push_prose_text(&mut String, ...)`, and `LineWrapper::flush_into` clears buffered output at semantic boundaries. That addresses much of the original concern.

Residual allocations remain in helper paths:

- `emit_prose_text` still returns a new `String`.
- `render_table_link` formats around `emit_prose_text`.
- marked table-cell text still returns via `emit_prose_text`.
- `emit_inline_code` still returns a formatted `String`, then the wrapper appends it.

These are lower risk than the original per-word wrapper path, but the recommendation is not fully complete. I would not block on this without profiling; convert the remaining helpers to push-style APIs only if allocation profiles still point here.

## Confirmed Fixed

- `biscuit-terminal/lib/src/utils/block_constraint.rs:50`: `content_length` now calls `visible_width` instead of compiling regexes per line.
- `biscuit-terminal/lib/src/components/filesystem.rs:1582`: directory entries now cache `is_dir` and lowercase `sort_name` before sorting.
- `biscuit-terminal/lib/src/components/filesystem.rs:500` and `:1795`: UID/GID resolution is cached in `MetricContext` during a tree build.
- `darkmatter/lib/src/markdown/highlighting/grammars.rs:33` and `darkmatter/lib/src/markdown/highlighting/mod.rs:29`: syntax grammars are now borrowed as `&'static SyntaxSet`; the large `SyntaxSet` clone is gone.

## Validation Run

- `cargo test -p tree-hugger owner_index --lib`
- `cargo test -p biscuit-file normalize_text --lib`
- `cargo test -p biscuit-terminal content_length --lib`

All three targeted test runs passed.
