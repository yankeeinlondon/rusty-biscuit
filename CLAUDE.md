# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

**Dockhand** is a Rust monorepo for AI-powered research and automation tools. It uses a workspace-based architecture with multiple areas, each containing focused modules.

## Architecture

### Monorepo Structure

The repository is organized into the following packages:

> Note: each package should have a `justfile` to provide common devops operations like build, lint, install, etc.

```txt
dockhand/
├── ai-pipeline
│   ├── cli/          # Binary: `research` (FUTURE)
│   └── lib/          # Core research library
│   └── service/      # Server to abstract AI pipelining functionality (FUTURE)
├── biscuit/          # Common utilities (providers, tools, codegen, hashing)
├── darkmatter-cli/   # Binary: `md` (markdown terminal renderer)
├── darkmatter-lib/   # Markdown parsing, mermaid diagrams, syntax highlighting
├── queue/            # TUI command scheduler
│   ├── cli/          # Binary: `queue` (TUI application)
│   └── lib/          # Core library (types, persistence, execution, terminal detection)
├── research/         # AI-powered library research tools
│   ├── cli/          # Binary: `research`
│   └── lib/          # Core research library
├── schematic/        # Schema generation for REST API clients
│   ├── define/       # API definition primitives (RestApi, Endpoint, AuthStrategy)
│   ├── definitions/  # Pre-built API definitions (OpenAI, Ollama, ElevenLabs, HuggingFace)
│   ├── gen/          # Code generator CLI with validate/generate subcommands
│   └── schema/       # Generated API clients (auto-generated, do not edit)
├── sniff/
│   ├── cli/          # Binary: `sniff`
│   └── lib/          # Hardware, Network, OS, and package manager discovery
├── so-you-say/       # Binary: `speak` (TTS CLI)
├── tree-hugger/      # Tree-sitter symbol extraction
│   ├── cli/          # Binary: `hug` (symbol/import/export CLI)
│   └── lib/          # Symbol extraction library (16 languages)
└── tui/              # Future: ratatui-based interactive chat
```


### Key Architectural Patterns

#### 1. Two-Phase LLM Pipeline (Research)

The research system uses a parallel two-phase approach:

**Phase 1: Underlying Research** (parallel execution)

- `overview.md` - Library features/API (ZAI GLM-4-7)
- `similar_libraries.md` - Alternatives (Gemini Flash)
- `integration_partners.md` - Ecosystem (Gemini Flash)
- `use_cases.md` - Patterns (Gemini Flash)
- `changelog.md` - Version history (OpenAI GPT-5.2)
- `question_N.md` - Additional prompts (Gemini Flash)

**Phase 2: Synthesis** (parallel, after Phase 1)

- `skill/SKILL.md` - Claude Code skill format (OpenAI GPT-5.2)
- `deep_dive.md` - Comprehensive reference (OpenAI GPT-5.2)
- `brief.md` - Quick summary (Gemini Flash)

**Incremental Research (DRY Approach)**:

- Checks for `metadata.json` to detect existing research
- New prompts are compared semantically using Gemini Flash for overlap detection
- Interactive selection for conflicts (conflicting prompts unselected by default)
- Re-runs Phase 2 synthesis with expanded corpus after adding new documents

**Provider Strategy**:

- **Fast models** (Gemini Flash): Phase 1 parallel research where speed matters
- **Stronger models** (GPT-5.2): Phase 2 synthesis requiring cross-document reasoning, changelog analysis
- All tasks in Phase 1 run concurrently via `tokio::join!`
- Ctrl+C exits immediately (exit code 130), preserving completed results

**Cancellation & Notifications**:

- Graceful degradation: Phase 2 proceeds with available Phase 1 content
- TTS completion announcement via system text-to-speech (skipped on cancellation)
- Markdown normalization ensures consistent formatting (pulldown-cmark + extensions)

#### 2. Provider Discovery System (Shared Library)

The `shared` library's provider system has three layers:

- **Base layer** (`providers/base.rs`): Environment-based API key management, OpenAI-compatible `/v1/models` endpoint discovery
- **Curated registry** (`providers/curated.rs`): Hardcoded model list (last updated timestamp)
- **Discovery layer** (`providers/discovery.rs`): API fetching with 24-hour cache, rate limiting with exponential backoff

Provider support includes: Anthropic, Deepseek, Gemini, MoonshotAI, Ollama (local), OpenAI, OpenRouter, ZAI, ZenMux.

#### 3. Agent Tools (rig-core integration)

The `biscuit/tools` module provides rig-core compatible agent tools:

- **BraveSearchTool**: Web search with plan-based rate limiting (free: 1/sec, base: 20/sec, pro: 50/sec)
- **ScreenScrapeTool**: Web scraping with multiple output formats (Markdown, HTML, PlainText, JSON, Links)

Both tools include comprehensive tracing instrumentation with OpenTelemetry semantic conventions.

#### 4. Safe Code Injection (Codegen)

The `biscuit/codegen` module provides AST-based code manipulation:

- Uses `syn` for parsing and `prettyplease` for formatting
- Validates injection points before modification
- Prevents duplicate injections via semantic analysis

#### 5. REST API Client Generation (Schematic)

The `schematic` package provides type-safe REST API client generation:

**Definition → Generation → Client:**
- `schematic-define`: Primitives for describing APIs (`RestApi`, `Endpoint`, `AuthStrategy`)
- `schematic-definitions`: Pre-built API definitions (OpenAI, HuggingFace, Ollama, ElevenLabs)
- `schematic-gen`: Code generator CLI with `validate` and `generate` subcommands
- `schematic-schema`: Generated API clients ready for consumption

**Key CLI Commands:**
```bash
# Validate an API definition
schematic-gen validate --api openai

# Generate client code
schematic-gen generate --api openai --output schematic/schema/src

# Regenerate all APIs
just -f schematic/justfile generate
```

**Configuration Options:**
- `module_path`: Override generated module name (for multi-API modules)
- `request_suffix`: Customize wrapper struct suffix (default: "Request")

**⚠️ CRITICAL - Response Type Selection:**

When defining endpoints, choose the correct `ApiResponse`:

| Response Type | Use For | Generated Method |
|---------------|---------|------------------|
| `ApiResponse::Json(schema)` | JSON responses | `request<T>()` |
| `ApiResponse::Binary` | Audio, images, archives | `request_bytes()` |
| `ApiResponse::Text` | Plain text | `request_text()` |
| `ApiResponse::Empty` | 204 No Content | `request_empty()` |

**⚠️ CRITICAL - Module Path for Multi-API Modules:**

When multiple APIs share one definitions module (e.g., Ollama), you MUST set `module_path`:

```rust
// Both APIs in definitions/src/ollama/mod.rs
RestApi { name: "OllamaNative".to_string(), module_path: Some("ollama".to_string()), ... }
RestApi { name: "OllamaOpenAI".to_string(), module_path: Some("ollama".to_string()), ... }
```

**⚠️ CRITICAL - Testing Gap:**

Schematic tests verify **syntax only**, NOT runtime behavior! After modifying:
1. Run `cargo test -p schematic-gen`
2. Run `just -f schematic/justfile generate`
3. Run `cargo check -p schematic-schema`
4. **Manually verify** correct methods generated: `grep "request_bytes" schematic/schema/src/*.rs`

#### 6. Tree-sitter Symbol Extraction (Tree Hugger)

The `tree-hugger` package provides multi-language symbol extraction using Tree-sitter:

- **16 supported languages**: Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua
- **Symbol kind distinction**: Differentiates struct vs enum, class vs interface, trait vs module
- **Rich metadata**: Extracts doc comments, function signatures, type parameters, struct fields, enum variants
- **Query vendoring**: Uses `nvim-treesitter` query files in `lib/queries/vendor/<lang>/locals.scm`

**IMPORTANT - Cross-Language Test Coverage**: When modifying tree-sitter queries or symbol extraction:
1. Every language with type constructs must have type distinction tests
2. All typed languages need `types.*` fixture files exercising their type system
3. Bug fixes require regression tests that would fail without the fix
4. Run `cargo test -p tree-hugger-lib` to verify all language tests pass

#### 7. Queue TUI Command Scheduler

The `queue` package provides a terminal-based task scheduler with async execution:

**Architecture:**
- **queue-lib**: Core library with data types, persistence, execution engine, terminal detection
- **queue-cli**: ratatui-based TUI with modal forms and event handling

**Key Components:**
- **Terminal Detection**: Auto-detects 8 terminal types (Wezterm, iTerm2, Terminal.app, GNOME, etc.)
- **Execution Targets**: NewPane (Wezterm), NewWindow (native terminal), Background (detached)
- **Persistence**: JSONL file storage with cross-platform file locking (`~/.queue-history.jsonl`)
- **Async Execution**: tokio-based task scheduling with mpsc event channels

**Wezterm Split Workflow:**
When running in Wezterm, Queue creates a split layout:
- Top 80%: Task execution area (commands run in new splits here)
- Bottom 20%: TUI control pane (schedule and monitor tasks)

**Documentation Navigation:**
| Document | Purpose |
|----------|---------|
| `queue/README.md` | High-level overview, quick start, key features |
| `queue/lib/README.md` | Data types, persistence API, executor, terminal detection |
| `queue/cli/README.md` | TUI architecture, keyboard shortcuts, modal system |

### Local Skills

This repository has local Claude Code skills in `.claude/skills/`:

- `clap` - Command-line argument parsing
- `color-eyre` - Error reporting
- `ratatui` - Terminal UI framework
- `resvg` - SVG rendering
- `rig` - LLM agent framework
- `syntect` - Syntax highlighting
- `thiserror` - Error derive macros

**Prefer using these local skills** as they contain project-specific research and are optimized for this codebase.

## Mandatory Workflows

When working in this repository, you **must** follow these workflows:

1. **Skill Usage**: Always use the `rig` skill when working with LLM interactions. Evaluate which links to follow within the skill's `SKILL.md` entry point. Always use the `rust` skill.

2. **Module-Specific Skills**:
   - Working in `tui/`? Use the `ratatui` skill
   - Working in CLI modules (`research/cli`, etc.)? Use the `clap` skill

3. **Dependency Management**: Before introducing new dependencies:
   - Check `docs/dependencies.md` first (primary source)
   - If missing, check `Cargo.toml` files
   - Prefer existing dependencies over adding new ones with overlapping functionality

4. **Report Skills Used**: At the start of work, explicitly state which skills you'll use to answer the request

## Common Commands

### Building

```bash
# Build all areas
just build

# Build specific area
just -f research/justfile build
just -f biscuit/justfile build
just -f so-you-say/justfile build

# Build specific package
cargo build -p research-cli
cargo build -p research-lib
cargo build -p shared
cargo build -p so-you-say
```

### Testing

```sh
# Test all areas
just test

# Test specific area
just -f research/justfile test
just -f biscuit/justfile test
just -f so-you-say/justfile test

# Test specific package with additional args
cargo test -p shared -- --nocapture
cargo test -p research-lib --lib

# Run a single test
cargo test -p shared --lib providers::base::tests::test_has_provider_api_key_with_set_env_var

# Tree Hugger tests (16 languages - critical for cross-language coverage)
cargo test -p tree-hugger-lib
cargo test -p tree-hugger-cli

# Queue tests
cargo test -p queue-lib
cargo test -p queue-cli
```

### Installing Binaries

```bash
# Install all binaries
just install

# Install specific binary
just -f research/justfile install    # Installs `research`
just -f so-you-say/justfile install  # Installs `speak`
cargo install --path queue/cli       # Installs `queue`
```

### Running in Development

```bash
# Research CLI (debug mode)
just research library clap "How does it compare to structopt?"
# Or directly:
just -f research/justfile cli library clap

# Pull skill to repository
research pull clap
research pull tokio --local  # Also copy underlying research docs

# Speak CLI (debug mode)
just -f so-you-say/justfile cli "Hello world"
```

### Linting

```bash
# Lint specific area
just -f so-you-say/justfile lint

# Or use cargo clippy directly
cargo clippy -p shared
cargo clippy -p queue-lib -p queue-cli
cargo clippy --workspace
```

## Environment Variables

| Variable | Description | Required For |
|----------|-------------|--------------|
| `RESEARCH_DIR` | Base directory for research output (default: `$HOME`) | Research CLI |
| `OPENAI_API_KEY` | OpenAI API key (GPT-5.2 for synthesis) | Research CLI |
| `GEMINI_API_KEY` | Google Gemini API key (Flash for underlying research) | Research CLI |
| `ZAI_API_KEY` | ZAI API key (GLM-4-7 for overview) | Research CLI |
| `BRAVE_API_KEY` | Brave Search API key | Agent tools (optional) |
| `BRAVE_PLAN` | Brave plan tier: `free`, `base`, `pro` (default: `free`) | Agent tools (optional) |

### Provider API Keys (Shared Library)

The shared library's provider system checks for these environment variables:

- Anthropic: `ANTHROPIC_API_KEY`
- Deepseek: `DEEPSEEK_API_KEY`
- Gemini: `GEMINI_API_KEY` or `GOOGLE_API_KEY`
- MoonshotAI: `MOONSHOT_API_KEY` or `MOONSHOT_AI_API_KEY`
- OpenAI: `OPENAI_API_KEY`
- OpenRouter: `OPEN_ROUTER_API_KEY` or `OPENROUTER_API_KEY`
- ZAI: `ZAI_API_KEY` or `Z_AI_API_KEY`
- ZenMux: `ZENMUX_API_KEY` or `ZEN_MUX_API_KEY`

## Output Locations

### Research Output

Research is stored at: `${RESEARCH_DIR:-$HOME}/.research/library/<package-name>/`

Example structure:

```
~/.research/library/clap/
├── metadata.json
├── overview.md
├── similar_libraries.md
├── integration_partners.md
├── use_cases.md
├── changelog.md
├── question_1.md
├── deep_dive.md
├── brief.md
└── skill/
    └── SKILL.md
```

## Package Manager Detection

The research system auto-detects package managers:

| Manager | Language | Detection Method |
|---------|----------|------------------|
| crates.io | Rust | API query |
| npm | JavaScript/TypeScript | Registry API |
| PyPI | Python | JSON API |
| Packagist | PHP | Search API |
| LuaRocks | Lua | HEAD request |
| pkg.go.dev | Go | HEAD request |

## Key Dependencies

### AI & LLM

- **rig-core** (v0.27.0): LLM agent framework with completion models, embeddings, and RAG abstractions - powers the research system
- **tokio** (v1.48.0): Async runtime for concurrent LLM operations

### CLI & User Interaction

- **clap** (v4.5.53): Command-line argument parser with derive API (see local skill `.claude/skills/clap/`)
- **inquire** (v0.9): Interactive CLI prompts for overlap detection and user confirmation
- **tts** (v0.26.3): Cross-platform text-to-speech for completion announcements

### HTTP & Web

- **reqwest** (v0.12): HTTP client for provider APIs and package manager queries
- **scraper** (v0.20): HTML parsing with CSS selectors for web scraping tool

### Serialization & Parsing

- **serde/serde_json** (v1.0): JSON serialization for metadata and API responses
- **pulldown-cmark** (v0.13.0): Markdown parsing for research document manipulation
- **syn**: AST parsing for safe code injection (codegen module)

### Rust Documentation Best Practices

- Avoid explicit `# Heading` (H1) inside a `///` docblock unless intentionally titling the item
    - Rustdoc already supplies the item name as a top-level title.
    - Adding an H1 duplicates visual hierarchy and is usually redundant.
- Use `## Heading` (H2) for primary sections
    - Example Sections:
        - `## Returns`
        - `## Errors`
        - `## Panics`
        - `## Safety`
        - `## Examples`
        - `## Notes`
- This aligns with:
    - Rust Standard Library documentation
    - rustc and clippy codebases
    - IDE hover and symbol views
- Use ### Heading (H3) only for subsections
    - Example:
        - `## Environment Variables`
        - `### Priority Order`
        - `### Fallback Behavior`
- Recommended section order
  1. Brief summary paragraph (no heading)
  2. `## Examples`
  3. `## Returns` (functions)
  4. `## Errors` (if applicable)
  5. `## Panics` (if applicable)
  6. `## Safety` (for unsafe APIs)
  7. `## Notes` or `## Implementation Notes`

### Testing

- **wiremock** (v0.6): HTTP mocking for provider API tests
- **tempfile** (v3.15): Temporary directories for research output tests
- **serial_test**: Test isolation for environment variable manipulation

For complete dependency information, see `docs/dependencies.md`.

## Development Notes

### Test Isolation

- Environment variable tests use `#[serial_test::serial]` to prevent race conditions
- The `ScopedEnv` test helper (in `biscuit/src/providers/base.rs`) provides RAII-based cleanup

### Tracing

**Core Principles**:

- **Libraries emit, applications configure**: Libraries (`research-lib`, `shared`) only emit events/spans, never install subscribers
- **Structured fields over messages**: Use machine-readable fields for filtering (e.g., `tool.name`, `tool.duration_ms`)
- **Spans for context**: Group related events and measure durations with `#[instrument]`

**Semantic Conventions** (OpenTelemetry):

| Field | Description | Example |
|-------|-------------| :---------: |
| `tool.name` | Tool being called | `"brave_search"` |
| `tool.query` | Search query/URL | `"rust async"` |
| `tool.duration_ms` | Execution time | `1234` |
| `tool.results_count` | Results returned | `10` |
| `http.status_code` | HTTP response | `200` |
| `otel.kind` | Span kind | `"client"` |

**Levels** (CLI flags):

- ERROR/WARN (default): Failures and recoverable issues
- INFO (`-v`): Tool calls, phase transitions, research progress
- DEBUG (`-vv`): Tool arguments, API requests, intermediate results
- TRACE (`-vvv`): Request/response bodies, verbose internals

**Security**: Always skip sensitive data: `#[tracing::instrument(skip(api_key))]`

**Testing**: Use `tracing-test` crate with `#[traced_test]` attribute for assertions

For complete tracing architecture, see `docs/tracing.md`.

### Error Handling

- Library code uses `thiserror` for error types
- No `unwrap()` or `expect()` in production code paths (only in tests)
- All public functions return `Result` types

### Code Injection Safety

- The `codegen` module uses AST manipulation (never regex on code)
- Always validate injection points before modification
- Use `syn` for parsing, `prettyplease` for formatting

## Additional Documentation

For deeper architectural details, see:

- **`docs/dependencies.md`**: Complete dependency list with descriptions and links
- **`docs/tracing.md`**: Comprehensive tracing architecture (665 lines) - libraries emit/apps configure, PromptHook implementation, OpenTelemetry integration
- **`research/docs/architecture.md`**: Research pipeline internals - prompt templates, metadata schema, package manager detection, LLM provider rationale
- **Code review from 2025-12-30**: `.ai/code-reviews/20251230.provider-base-implementation.md` - identifies code duplication issues in provider module
