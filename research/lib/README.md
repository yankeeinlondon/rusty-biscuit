# Research Library

Core library for AI-powered research on software libraries and APIs, using a two-phase LLM pipeline with parallel execution, incremental updates, and multi-provider orchestration.

## Modules

| Module | Description |
|--------|-------------|
| `changelog` | Tiered changelog generation: GitHub Releases, registry versions, changelog files, LLM synthesis |
| `link` | Symlink creation from research skills to Claude Code, OpenCode, and Roo Code user directories |
| `list` | Topic discovery, glob/type filtering, and terminal/JSON output formatting |
| `metadata` | Schema-versioned research metadata, SQLite persistence, v0-to-v1 migration, content policy, topic types |
| `pull` | Copy skills from user-level research library into git repositories with framework symlinks |
| `validation` | SKILL.md frontmatter parsing/repair and comprehensive topic health checking |
| `utils` | Filename sanitization for custom prompt naming |

## Key Types

### Root (`lib.rs`)

- **`ResearchMetadata`** -- Schema-versioned metadata with `load()`/`save()`, auto-migration, and `when_to_use` extraction from SKILL.md frontmatter
- **`ResearchKind`** -- Enum: `Library`, `Api`
- **`LibraryInfo`** -- Package manager detection result (manager, language, URL, repository)
- **`TracingPromptHook`** -- `PromptHook` implementation that emits structured tracing events for agent tool calls
- **`ResearchResult`** -- Pipeline execution result with per-prompt metrics

### Metadata Module

- **`ResearchDetails`** -- Tagged enum with 16 research type variants (Library, Api, Cli, App, Standard, etc.)
- **`Topic`** -- Rich topic type with `Document`, `Flow`, `ContentType`, `License` support
- **`ResearchInventory`** -- Filesystem-based topic inventory with filtering
- **`ResearchInventoryDb`** -- SQLite-backed inventory with `sqlx` migrations
- **`ContentPolicy`** / **`ContentExpiry`** -- Staleness and refresh policy for research documents

### Changelog Module

- **`VersionHistory`** / **`VersionInfo`** -- Structured version data with confidence levels
- Submodules: `aggregator` (source dedup), `discovery` (changelog file parsing), `github` (Releases API), `registry` (crates.io/npm/PyPI)

### Validation Module

- **`SkillFrontmatter`** / **`ChangelogFrontmatter`** -- Typed frontmatter with parse + repair
- **`ResearchHealth`** -- Comprehensive health check (missing files, frontmatter issues, completeness)

## Architecture

### Two-Phase LLM Pipeline

Phase 1 prompts have access to web search (`BraveSearchTool`) and page scraping (`ScreenScrapeTool`) tools via `unchained-ai`. Phase 2 prompts run tool-free, synthesizing from the Phase 1 corpus.

All Phase 1 tasks execute concurrently via `join_all`. Phase 2a runs in parallel (`join!`), then Phase 2b (brief) runs sequentially after deep_dive completes.

Prompt templates are embedded at compile time via `include_str!` from the `prompts/` directory.

### Provider Strategy

| Phase | Model | Provider | Rationale |
|-------|-------|----------|-----------|
| Phase 1 (overview) | glm-4.7 | ZAI | Balanced quality for core overview |
| Phase 1 (other) | gemini-3-flash-preview | Google | Speed for parallel research |
| Phase 1 (changelog) | gpt-5.2 | OpenAI | Cross-document reasoning for version history |
| Phase 2 (skill, deep_dive) | gpt-5.2 | OpenAI | Quality for synthesis |
| Phase 2b (brief) | gemini-3-flash-preview | Google | Fast summarization |

### Incremental Research

When `metadata.json` exists, the pipeline detects existing work and avoids redundant LLM calls:

1. Compares new prompts against existing documents for overlap
2. Uses Gemini Flash for semantic overlap detection
3. Presents interactive selection via `inquire` for conflicts
4. Re-runs Phase 2 synthesis with the expanded corpus

### Package Manager Detection

`find_library()` queries 6 registries concurrently: crates.io, npm, PyPI, Packagist, LuaRocks, pkg.go.dev. Results are presented via `select_library()` using `inquire::Select`.

## Workspace Dependencies

| Crate | Purpose |
|-------|---------|
| `unchained-ai` | LLM provider clients (OpenAI, Gemini, ZAI), tool definitions (BraveSearch, ScreenScrape) |
| `darkmatter` | Markdown normalization via pulldown-cmark round-tripping |
| `biscuit-speaks` | TTS completion announcements |
| `sniff` | System detection (used for environment context) |

## Testing

```bash
cargo test -p research --lib          # Unit tests
cargo test -p research --lib -- pull  # Pull module tests
```

Key test infrastructure:

- **`tempfile`** -- Isolated filesystem for pull/copy tests
- **`wiremock`** -- HTTP mocking for package manager API tests
- **`serial_test`** -- Isolation for `RESEARCH_DIR` / `HOME` env var manipulation
- **`tracing-test`** -- Assertions on tracing span/event output
- **`proptest`** -- Property-based tests for filename sanitization

## Further Reading

- [`../README.md`](../README.md) -- CLI usage, environment variables, output structure
- [`../docs/architecture.md`](../docs/architecture.md) -- Prompt templates, metadata schema, provider rationale
- [`../docs/research-filesystem.md`](../docs/research-filesystem.md) -- Output directory layout

## Lessons Learned

- **Frontmatter repair is essential**: LLM-generated SKILL.md files frequently have malformed YAML (markdown headers in YAML, escaped brackets). The `repair_skill_frontmatter()` function handles common patterns before parsing.
- **Graceful cancellation matters**: Ctrl+C preserves completed Phase 1 results. Phase 2 proceeds with whatever content is available rather than requiring 100% Phase 1 success (50% threshold or all 5 core prompts).
- **Compile-time prompt embedding**: Using `include_str!` for prompt templates ensures they are bundled in the binary and eliminates runtime file I/O or path resolution issues.
- **Schema migration on load**: Auto-migrating v0 metadata during `ResearchMetadata::load()` with backup creation avoids a separate migration step while preserving data safety.
