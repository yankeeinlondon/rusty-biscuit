# Research
<img src="../assets/research.png" style="position: fixed; max-width: 30%; height: 150px; right: 0; top: 0; opacity: 0.75"></img>

> a library and CLI application to aid and abet you in doing research for your own edification or for your favorite AI agent

## Modules for Research

This area of the `dockhand` monorepo is focused on **research** and is broken up into two discrete modules:

- **Research Library** (`/research/lib`)

    Exposes functions which allow research into various topics and items.

    > **Note:** _will leverage the `shared` module in this monorepo for highly generalizable operations_

- **CLI** (`/research/cli`)

    Exposes the `research` CLI command and leverages the research library to achieve its goals.

## Types of Research

1. **Libraries** - Research code libraries found on various package managers like `crates.io`, `npm`, `PyPI`, etc.

2. **Software** - (Future) Research software applications and tools.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RESEARCH_DIR` | Base directory for research output | `$HOME` |
| `OPENAI_API_KEY` | OpenAI API key for GPT-5.2 | (required) |
| `GEMINI_API_KEY` | Google Gemini API key | (required) |
| `ZAI_API_KEY` | ZAI API key for GLM-4-7 | (required) |

### Output Location

Research output is stored at:

```
${RESEARCH_DIR:-$HOME}/.research/library/<package-name>/
```

## Using the CLI

### Global Switches

- `verbose` / `v` - verbose output
- `help` - show the help system

### Commands

#### Library Research (`research library <pkg> [prompt] [prompt]`)

Perform research on a software library.

```bash
# Basic research
research library clap

# With additional questions
research library clap "How does it compare to structopt?" "What are the derive macros?"

# With custom filenames (new in 2025-12-30 refactoring)
research library clap "comparison -> How does it compare to structopt?"
research library clap "derive-macros -> What are the derive macros?"
```

**Custom Prompt Naming Syntax (New):**

By default, additional prompts are saved as `question_1.md`, `question_2.md`, etc. You can now specify custom filenames using the arrow syntax:

```bash
# Syntax: "filename -> prompt text"
research library rig "agent-architecture -> How do agents work in rig?"
# Generates: agent-architecture.md with the prompt "How do agents work in rig?"

# Multiple custom prompts
research library tokio \
  "runtime -> How does the runtime work?" \
  "channels -> What channel types are available?"
# Generates: runtime.md and channels.md
```

**Rules:**
- Filename must come before the arrow (`->`)
- Filename will be sanitized (lowercase, hyphens for spaces, `.md` appended)
- Prompts without `->` still use `question_N.md` naming
- Can mix custom and default naming in the same command

## Library Research Output

### Underlying Research

All library research starts with a set of _underlying_ research documents:

| File | Description | Model |
|------|-------------|-------|
| `overview.md` | Comprehensive library overview covering features, API surface, and usage patterns | ZAI GLM-4-7 |
| `similar_libraries.md` | Alternative libraries with comparisons, pros/cons, and when to use each | Gemini Flash |
| `integration_partners.md` | Libraries commonly used alongside this one (ecosystem partners) | Gemini Flash |
| `use_cases.md` | Common use cases, patterns, and real-world examples | Gemini Flash |
| `changelog.md` | Major-version change history with breaking changes and migration notes | OpenAI GPT-5.2 |
| `question_N.md` | Answers to user-provided additional prompts | Gemini Flash |

### Summary Deliverables

Once all _underlying research_ is complete, three deliverables are produced:

#### 1. Deep Dive Document (`deep_dive.md`)

- A single comprehensive document covering everything
- Starts with a table of contents for navigation
- Combines rich prose with code examples
- Intended for both humans and LLMs without skill-based knowledge

#### 2. Skill (`skill/SKILL.md`)

- A tree-shaped linked structure of documents
- Modeled after Claude Code's **skill** structure:
  - Entry point: `SKILL.md` (concise, <200 lines)
  - Links to sub-areas with greater detail
- Enables LLMs to selectively use relevant parts
- Optimizes context window usage

#### 3. Brief (`brief.md`)

- A compact summary for quick reference
- Frontmatter includes:
  - `summary`: Single-sentence description
  - `repo`: Link to source repository (when available)
- Body contains a paragraph-length overview
- Generated using Gemini Flash from the deep dive

## Metadata

Research metadata is stored in `metadata.json`:

```json
{
  "kind": "library",
  "library_info": {
    "package_manager": "crates.io",
    "language": "Rust",
    "url": "https://crates.io/crates/clap",
    "repository": "https://github.com/clap-rs/clap"
  },
  "brief": "A full-featured command-line argument parser for Rust applications.",
  "summary": "clap is a fast, ergonomic command-line argument parser for Rust that supports derive macros for declarative argument definitions, rich help generation, shell completions, and comprehensive validation. It's the most popular CLI library in the Rust ecosystem, used by cargo and thousands of other tools.",
  "additional_files": {
    "question_1.md": "How does it compare to structopt?"
  },
  "created_at": "2025-12-28T10:00:00Z",
  "updated_at": "2025-12-28T10:00:00Z"
}
```

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
Phase 1: Underlying Research (parallel)
├── overview.md          [GLM-4-7]
├── similar_libraries.md [Gemini Flash]
├── integration_partners.md [Gemini Flash]
├── use_cases.md         [Gemini Flash]
├── changelog.md         [GPT-5.2]
└── question_N.md        [Gemini Flash]

Phase 2: Synthesis (parallel, after Phase 1)
├── skill/SKILL.md       [GPT-5.2]
├── deep_dive.md         [GPT-5.2]
└── brief.md             [Gemini Flash] (after deep_dive)
```

### Package Manager Support

| Manager | Language | Detection |
|---------|----------|-----------|
| crates.io | Rust | API query |
| npm | JavaScript/TypeScript | Registry API |
| PyPI | Python | JSON API |
| Packagist | PHP | Search API |
| LuaRocks | Lua | HEAD request |
| pkg.go.dev | Go | HEAD request |
