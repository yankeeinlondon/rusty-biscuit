---
phases: 3
starting_phase: 3
---

# Implementation Plan: 2026-04-29 Performance Review

## Phase 1: Quick Wins — Localized Regex and I/O Fixes

**Goal:** Apply low-risk, high-confidence optimizations with minimal API surface changes.

### 1.1 Precompile `extract_status_code` regexes
- **File:** `claudine/lib/src/stream/logs/opencode.rs`
- **Change:** Replace loop-time `Regex::new` with `std::sync::LazyLock<[Regex; 2]>`
- **Test:** Unit test `extract_status_code` with both pattern variants; verify no regression in stream log parsing.

### 1.2 Precompile `parse_github_url` regex
- **File:** `research/lib/src/changelog/discovery.rs`
- **Change:** Extract `Regex::new(r"https?://github\.com/([^/]+)/([^/]+)")` into a `LazyLock` static
- **Test:** Unit test `parse_github_url` with valid/invalid URLs.

### 1.3 Eliminate double file reads in composition
- **Files:** `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/composition/sequence.rs`
- **Change:** Read file once into `String`, then construct `Markdown` from the string (add `Markdown::try_from_str` if needed). Remove the second `fs::read_to_string`.
- **Test:** Composition integration tests; assert identical output with half the I/O calls.

---

## Phase 2: Async I/O Modernization

**Goal:** Remove blocking I/O from async contexts and replace eager indexing with lazy lookups.

### 2.1 Replace `std::fs` with `tokio::fs` in permission providers
- **Files:** `claudine/lib/src/permissions/providers/claude.rs`, `claudine/lib/src/permissions/providers/gemini.rs`, and siblings
- **Change:** Swap `std::fs::read_to_string` / `std::fs::read_dir` for `tokio::fs::read_to_string` / `tokio::fs::read_dir`, adding `.await` where needed.
- **Test:** Permission provider integration tests under async runtime; verify no blocking behavior via `tokio::task::yield_now` or timeout guards.

### 2.2 Convert `sniff` eager PATH indexing to lazy lookup
- **File:** `sniff/lib/src/programs/find_program.rs`
- **Change:** Replace `build_with_bundles` full-PATH `HashMap` with on-demand PATH iteration (e.g., `which`-style lookup per binary name). Remove or deprecate the eager `HashMap<String, PathBuf>` cache.
- **Test:** Criterion benchmark comparing `build()` wall-clock time before and after; unit tests for specific binary lookups (e.g., `git`, `uv`).

---

## Phase 3: Stream Processing Structural Refactor

**Goal:** Eliminate `serde_json::Value` DOM allocations on hot LLM stream paths.

### 3.1 Design typed structs for semantic stream payloads
- **Files:** `claudine/lib/src/stream/gemini_semantic.rs`, `claudine/lib/src/stream/claude_semantic.rs`, and other `*_semantic.rs` modules
- **Change:** Define provider-specific structs matching the JSON Lines schema. Use `#[serde(flatten)]` for dynamic fallback fields if necessary. Replace `serde_json::from_str::<Value>` with `serde_json::from_str::<ProviderEvent>`.
- **Optional:** Use `serde_json::from_slice` with zero-copy `&str` fields where lifetimes permit.
- **Test:** 
  - Property-based tests ensuring typed deserializers accept all known provider event shapes.
  - Flamegraph/heaptrack validation confirming reduced allocation churn.
  - End-to-end stream tests against recorded provider responses.

---

## Phase Dependency Graph

```
Phase 1 (Quick Wins) ─┐
                      ├──► All phases are independent; no cross-phase dependencies.
Phase 2 (Async I/O) ──┤
                      │
Phase 3 (Typed Streams) ┘
```

Each phase can be developed, reviewed, and merged independently. Recommended execution order: 1 → 2 → 3.
