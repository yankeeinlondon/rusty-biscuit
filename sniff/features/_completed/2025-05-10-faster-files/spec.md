# Faster `sniff docs` Specification

Optimize the `sniff docs` execution path for sub-100ms performance, particularly when filtered by `--package-area`, `--package`, or `--has-prompt`. This is motivated by Claudine shell completions, where every millisecond counts.

## Current Performance

Measured against the rusty-biscuit monorepo (~2000 markdown files, ~24 MB total):

| Command | Wall time |
|---|---|
| `sniff docs --json` | ~115ms |
| `sniff docs --package-area sniff --json` | ~115ms |
| `sniff docs --has-prompt --json` | ~120ms |

All three are identical because filtering happens **after** full detection and parsing.

## Current Hot Path

```
commands/mod.rs                          sniff/lib/src/filesystem/docs.rs
─────────────────                        ──────────────────────────────────
git2::Repository::discover(&base)  ──►  detect_docs(&repo_root)
  repo_root = repo.workdir()              ├── RepoDocuments::new()
                                            │   ├── git2::Repository::discover()   ← REDUNDANT
                                            │   └── detect_repo()                  ← FULL detection (language scanning)
                                            └── collect_markdown_files()
                                                ├── WalkBuilder walks ENTIRE repo   ← 2000+ files
                                                └── parse_markdown_meta_full()      ← reads + hashes every file
filter_docs(&all_docs, &filter)        ← post-hoc filtering
```

## Identified Bottlenecks

### B1: Redundant `git2::Repository::discover`

The CLI discovers the repo root at `commands/mod.rs:188`, then passes it to `detect_docs()` which internally calls `RepoDocuments::new()` which calls `git2::Repository::discover` **again**.

**Impact:** ~1ms wasted. Low, but unnecessary.

### B2: Full `detect_repo` for package names only

`RepoDocuments::new()` at `docs.rs:116` calls `detect_repo()` which performs full per-package language scanning. The only thing needed is the package list (names + relative paths) for assigning `doc.package`. `detect_repo_structure()` exists and is documented as 10-50x faster.

**Impact:** ~5-10ms wasted.

### B3: Walk-and-parse everything, then filter

`collect_markdown_files()` walks the entire repo and calls `parse_markdown_meta_full()` on every markdown file. This reads entire file contents, hashes bodies, extracts titles, and resolves mtimes — even when `--package-area sniff` means only ~127 of ~2000 files will survive filtering.

**Impact:** ~50-80ms wasted on irrelevant files. This is the dominant cost.

### B4: Full parse when only frontmatter is needed

For `--has-prompt` (without `--verbose`), the only fields consumed are `prompt` and `relative`. But `parse_markdown_meta_full()` still hashes the body, extracts titles, resolves mtime, and collects frontmatter keys — all thrown away by the post-hoc filter.

The library already has `read_frontmatter_only()` and `DocParseMode::BlastRadiusOnly` demonstrating the pattern.

**Impact:** ~30-60ms wasted.

### B5: Bare-path mode for completions (not yet exposed)

The library already has `collect_markdown_paths()` which does a bare walk without any file parsing (~5-10ms). This is ideal for shell completions but is not wired to any CLI flag.

**Impact:** ~100ms savings vs current path.

## Proposed Optimizations

### O1: Accept pre-resolved repo root

Add a `detect_docs_from_root()` function that accepts an already-resolved `repo_root: &Path` and optional package info, skipping the redundant `Repository::discover` and `detect_repo` calls inside `RepoDocuments::new`.

The CLI already has `repo_root` at `commands/mod.rs:190`. Pass it through.

### O2: Use `detect_repo_structure` instead of `detect_repo`

Change `RepoDocuments::new()` to call `detect_repo_structure()` instead of `detect_repo()`. The docs module only needs `(name, relative_path)` pairs — not language stats.

### O3: Path-based pre-filtering

When `--package-area` or `--package` filters are present, pre-filter file paths during the walk phase **before** calling the expensive `parse_markdown_meta_full()`. A file at `homelab/docs/api.md` can be rejected in O(1) when the filter is `--package-area sniff`.

The walk still starts from repo root (to honor `.gitignore` correctly), but `parse_markdown_meta` is only called on matching paths.

### O4: Frontmatter-only parse mode

Add `DocParseMode::FrontmatterOnly` that stops reading at the closing frontmatter delimiter, extracts only `prompt`, `relative`, and `package`, and skips body hashing, title extraction, and mtime resolution.

Use this mode automatically when:
- Only `--has-prompt` is specified (no `--verbose`)
- Or more generally: when none of the output fields that depend on full parsing are required

### O5: `--paths-only` flag for completions

Add a `--paths-only` (or `--bare`) flag that calls `collect_markdown_paths()` and outputs relative paths only, without any file parsing. This is the fastest possible path and is suitable for shell completion scripts.

## Target Performance

| Command | Current | Target | Primary optimization |
|---|---|---|---|
| `sniff docs --json` | ~115ms | ~80ms | O1 + O2 |
| `sniff docs --package-area sniff --json` | ~115ms | ~15-25ms | O1 + O2 + O3 |
| `sniff docs --has-prompt --json` | ~120ms | ~30-50ms | O1 + O2 + O4 |
| `sniff docs --has-prompt --package-area sniff --json` | ~120ms | ~5-10ms | O1 + O2 + O3 + O4 |
| `sniff docs --paths-only` | N/A | ~5-10ms | O5 |

## Constraints

- **Public API stability**: Existing `detect_docs()`, `RepoDocuments`, and `MarkdownMeta` must continue to work unchanged.
- **Correctness**: Filtered output must be identical to the unfiltered output filtered post-hoc (same docs, same fields).
- **Cross-platform**: All optimizations must work on macOS, Linux, and Windows.
- **No new dependencies**: Use existing `ignore`, `rayon`, `git2` crates.

## Out of Scope

- Caching between invocations (different feature)
- Parallelizing the walk itself (already parallel via rayon for parsing)
- Changing the JSON shape of `MarkdownMeta`
