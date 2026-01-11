# Research
<img src="../assets/research.png" style="position: fixed; max-width: 30%; height: 150px; right: 0; top: 0; opacity: 0.75"></img>

> a library and CLI application to aid and abet you in doing research for your own edification or for your favorite AI agent

## Modules for Research

This area of the `dockhand` monorepo is focused on **research** and is broken up into two discrete modules:

- **Research Library** (`/research/lib`)

    Exposes functions which allow research into various topics and items.

    > **Note:** _leverages the `biscuit` module in this monorepo for highly generalizable operations_

- **CLI** (`/research/cli`)

    Exposes the `research` CLI command and leverages the research library to achieve its goals.

## Types of Research

1. **Libraries** - Research code libraries found on various package managers like `crates.io`, `npm`, `PyPI`, etc.

2. **APIs** - Research public APIs (REST, GraphQL, etc.).

3. **Other Types** - The metadata schema supports many additional types (CLI tools, Apps, Standards, etc.) but the CLI currently only implements `library` and `api` commands.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RESEARCH_DIR` | Base directory for research output | `$HOME` |
| `OPENAI_API_KEY` | OpenAI API key for `gpt-5.2` (synthesis, changelog) | (required) |
| `GEMINI_API_KEY` | Google Gemini API key for `gemini-3-flash-preview` | (required) |
| `ZAI_API_KEY` | ZAI API key for `glm-4.7` (overview) | (optional, falls back to Gemini) |
| `BRAVE_API_KEY` | Brave Search API key for web search tool | (optional) |
| `BRAVE_PLAN` | Brave plan tier: `free`, `base`, `pro` | `free` |

### Output Location

Research output is stored at:

```
${RESEARCH_DIR:-$HOME}/.research/library/<package-name>/
```

## Using the CLI

### Global Options

| Option | Description |
|--------|-------------|
| `-v`, `-vv`, `-vvv` | Increase verbosity (info, debug, trace) |
| `--json` | Output logs as JSON |
| `--help` | Show help |

### Commands

#### Library Research (`research library`)

Research a software library.

```bash
research library <TOPIC> [QUESTIONS...] [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-o`, `--output <DIR>` | Output directory (default: `$RESEARCH_DIR/.research/library/<TOPIC>`) |
| `--skill` | Regenerate skill files from existing research |
| `--force` | Force recreation of all research output documents |

**Examples:**

```bash
# Basic research
research library clap

# Read topic from stdin
echo "tokio" | research library -

# With additional questions
research library clap "How does it compare to structopt?" "What are the derive macros?"

# With custom filenames using arrow syntax
research library clap "comparison -> How does it compare to structopt?"
research library tokio \
  "runtime -> How does the runtime work?" \
  "channels -> What channel types are available?"

# Regenerate skill from existing research
research library clap --skill

# Force full re-research
research library clap --force
```

**Custom Prompt Naming Syntax:**

By default, additional prompts are saved as `question_1.md`, `question_2.md`, etc. You can specify custom filenames using the arrow syntax:

- Syntax: `"filename -> prompt text"`
- Filename will be sanitized (lowercase, hyphens for spaces, `.md` appended)
- Cannot use reserved names: `overview`, `similar_libraries`, `integration_partners`, `use_cases`, `changelog`, `deep_dive`, `brief`
- Can mix custom and default naming in the same command

#### List Topics (`research list`)

List all research topics.

```bash
research list [FILTERS...] [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-t`, `--type <TYPE>` | Filter by research type (repeatable) |
| `--verbose` | Show detailed metadata with issues |
| `--json` | Output as JSON |
| `--migrate` | Migrate all v0 metadata files to v1 schema |

**Examples:**

```bash
# List all topics
research list

# Filter by glob pattern
research list "clap*" "tokio"

# Filter by type
research list -t library

# Show detailed metadata
research list --verbose

# Migrate old metadata format
research list --migrate
```

#### Link Skills (`research link`)

Create symbolic links from research skills to Claude Code and OpenCode directories.

```bash
research link [FILTERS...] [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-t`, `--type <TYPE>` | Filter by research type (repeatable) |
| `--json` | Output as JSON |

#### Show Topic (`research show`)

Open a research topic's deep dive document in the system's default application.

```bash
research show <TOPIC>
```

#### API Research (`research api`)

Research a public API.

```bash
research api <API_NAME> [QUESTIONS...] [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-o`, `--output <DIR>` | Output directory (default: `$RESEARCH_DIR/.research/api/<API_NAME>`) |
| `-f`, `--force` | Force recreation even if research exists |

## Library Research Output

### Underlying Research (Phase 1)

All library research starts with a set of _underlying_ research documents executed in parallel:

| File | Description | Model |
|------|-------------|-------|
| `overview.md` | Comprehensive library overview covering features, API surface, and usage patterns | `glm-4.7` (ZAI) or `gemini-3-flash-preview` fallback |
| `similar_libraries.md` | Alternative libraries with comparisons, pros/cons, and when to use each | `gemini-3-flash-preview` |
| `integration_partners.md` | Libraries commonly used alongside this one (ecosystem partners) | `gemini-3-flash-preview` |
| `use_cases.md` | Common use cases, patterns, and real-world examples | `gemini-3-flash-preview` |
| `changelog.md` | Major-version change history with breaking changes and migration notes | `gpt-5.2` |
| `question_N.md` | Answers to user-provided additional prompts | `gemini-3-flash-preview` |

> **Note:** If `ZAI_API_KEY` is not set, the overview generation falls back to `gemini-3-flash-preview`.

### Synthesis (Phase 2)

Once Phase 1 completes, synthesis outputs are generated:

#### Phase 2a: Parallel Generation

These are generated in parallel using `gpt-5.2`:

**Deep Dive Document (`deep_dive.md`)**

- A single comprehensive document covering everything
- Starts with a table of contents for navigation
- Combines rich prose with code examples
- Intended for both humans and LLMs without skill-based knowledge

**Skill (`skill/SKILL.md`)**

- A tree-shaped linked structure of documents
- Modeled after Claude Code's **skill** structure:
  - Entry point: `SKILL.md` (concise, <200 lines)
  - Links to sub-areas with greater detail
- Enables LLMs to selectively use relevant parts
- Optimizes context window usage

#### Phase 2b: Brief Generation (Sequential)

Generated after deep_dive.md completes using `gemini-3-flash-preview`:

**Brief (`brief.md`)**

- A compact summary for quick reference
- Frontmatter includes:
  - `summary`: Single-sentence description
  - `repo`: Link to source repository (when available)
- Body contains a paragraph-length overview
- Derived from the deep dive document

## Metadata

Research metadata is stored in `metadata.json` using schema version 1:

```json
{
  "schema_version": 1,
  "kind": "Library",
  "details": {
    "type": "Library",
    "package_manager": "crates.io",
    "language": "Rust",
    "url": "https://crates.io/crates/clap",
    "repository": "https://github.com/clap-rs/clap"
  },
  "additional_files": {
    "question_1.md": "How does it compare to structopt?"
  },
  "created_at": "2025-12-28T10:00:00Z",
  "updated_at": "2025-12-28T10:00:00Z",
  "brief": "A full-featured command-line argument parser for Rust applications.",
  "summary": "clap is a fast, ergonomic command-line argument parser...",
  "when_to_use": "Use when building CLI applications in Rust..."
}
```

### Schema Version 1 Fields

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | `u32` | Always `1` for current format |
| `kind` | `enum` | Research type: `Library`, `Api`, `Cli`, `App`, `Standard`, etc. |
| `details` | `object` | Type-specific details (tagged with `"type"` field) |
| `additional_files` | `object` | Map of filename to prompt text |
| `created_at` | `datetime` | ISO 8601 creation timestamp |
| `updated_at` | `datetime` | ISO 8601 last update timestamp |
| `brief` | `string?` | Single-sentence summary |
| `summary` | `string?` | Paragraph-length summary |
| `when_to_use` | `string?` | Guidance extracted from SKILL.md frontmatter |

### Supported Research Types

The `details` field is a tagged enum supporting:

- **Library** - Package/library research with `package_manager`, `language`, `url`, `repository`
- **Api** - Public API research
- **Cli** - Command-line tool research
- **App** - Application research
- **CloudProvider** - Cloud service research
- **Standard** - Technical standard/specification research
- **SolutionSpace** - Problem space comparison research
- **Person**, **People**, **Place**, **Product**, **Company**, **CompanyCategory**, **News**, **SkillSet** - Other research types

### Schema Migration

Old v0 metadata (with `library_info` field) is automatically migrated to v1 format on load. The original file is backed up as `metadata.v0.json.backup`. Use `research list --migrate` to batch-migrate all topics.

## Incremental Research (DRY)

The research system avoids repeating work:

1. **Existence check**: If `metadata.json` exists, runs in incremental mode
2. **Overlap detection**: Compares new prompts against existing documents
3. **Interactive selection**: For conflicting prompts, user chooses which to include
4. **Re-synthesis**: Regenerates Phase 2 deliverables with expanded corpus

### Overlap Handling

- **New prompts**: Added by default
- **Conflicting prompts**: Unselected by default, user confirmation required

## Architecture

For detailed technical documentation, see [`/research/docs/architecture.md`](./docs/architecture.md).

### Two-Phase Pipeline

```
Phase 1: Underlying Research (parallel via tokio::join_all)
├── overview.md             [glm-4.7 or gemini-3-flash-preview fallback]
├── similar_libraries.md    [gemini-3-flash-preview]
├── integration_partners.md [gemini-3-flash-preview]
├── use_cases.md            [gemini-3-flash-preview]
├── changelog.md            [gpt-5.2]
└── question_N.md           [gemini-3-flash-preview]

Phase 2a: Synthesis (parallel via tokio::join!, after Phase 1)
├── skill/SKILL.md          [gpt-5.2]
└── deep_dive.md            [gpt-5.2]

Phase 2b: Brief (sequential, after deep_dive)
└── brief.md                [gemini-3-flash-preview]
```

**Execution Notes:**

- Phase 1 requires 50% success OR all 5 core prompts to proceed to Phase 2
- Ctrl+C gracefully exits with code 130, preserving completed results
- TTS completion announcement plays only on successful completion

### Package Manager Support

| Manager | Language | Detection |
|---------|----------|-----------|
| crates.io | Rust | API query |
| npm | JavaScript/TypeScript | Registry API |
| PyPI | Python | JSON API |
| Packagist | PHP | Search API |
| LuaRocks | Lua | HEAD request |
| pkg.go.dev | Go | HEAD request |
