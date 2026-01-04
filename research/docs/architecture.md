# Research Architecture

## Overview

The Research Library provides automated research capabilities for software libraries using a two-phase LLM pipeline. It gathers comprehensive information about libraries from multiple sources and produces structured documentation suitable for both human readers and AI assistants.

The design philosophy emphasizes:

- **Parallel execution** for speed (multiple LLM calls run concurrently)
- **Incremental research** (DRY - don't repeat existing research)
- **Structured output** (skill trees for context-efficient AI consumption)
- **Multi-provider flexibility** (different models for different tasks)

## Two-Phase Research Pipeline

### Phase 1: Underlying Research

Phase 1 runs multiple LLM calls in parallel to gather raw research about the target library:

| Task | Output File | Model | Purpose |
|------|-------------|-------|---------|
| Overview | `overview.md` | ZAI GLM-4-7 | Comprehensive library analysis |
| Similar Libraries | `similar_libraries.md` | Gemini Flash | Alternatives and comparisons |
| Integration Partners | `integration_partners.md` | Gemini Flash | Ecosystem libraries |
| Use Cases | `use_cases.md` | Gemini Flash | Common patterns and examples |
| Changelog | `changelog.md` | OpenAI GPT-5.2 | Major-version change history |
| Additional Questions | `question_N.md` | Gemini Flash | User-defined research prompts |

**Key characteristics:**

- All tasks run concurrently via `tokio::join!`
- Each task is independent and can fail without affecting others
- Progress is reported as tasks complete
- Ctrl+C exits immediately, preserving completed results

### Phase 2: Synthesis

Phase 2 aggregates all Phase 1 outputs and generates consolidated deliverables:

| Task | Output | Model | Purpose |
|------|--------|-------|---------|
| Skill Generation | `skill/SKILL.md` + supporting files | OpenAI GPT-5.2 | Claude Code skill structure |
| Deep Dive | `deep_dive.md` | OpenAI GPT-5.2 | Dense comprehensive reference |

**Key characteristics:**

- Runs after all Phase 1 tasks complete (or are cancelled)
- Uses combined context from all Phase 1 documents
- Skill output supports multi-file format via `--- FILE: name.md ---` markers
- Both tasks run in parallel

## LLM Provider Strategy

### Provider Configuration

The library uses three LLM providers:

| Provider | Models | Purpose |
|----------|--------|---------|
| OpenAI | GPT-5.2 | Phase 2 synthesis, changelog analysis |
| Google | Gemini 3 Flash Preview | Phase 1 parallel research (fast) |
| ZAI | GLM-4-7 | Overview generation |

### Model Selection Rationale

- **Fast models (Gemini Flash)** for Phase 1 research where speed matters and tasks are straightforward
- **Stronger models (GPT-5.2)** for:
  - Phase 2 synthesis (requires reasoning across multiple documents)
  - Changelog analysis (requires deep understanding of repository history)
  - Complex summarization and skill structure generation

### Environment Variables

All providers read API keys from environment variables (via `dotenvy`):

- `OPENAI_API_KEY`
- `GEMINI_API_KEY` / `GOOGLE_API_KEY`
- `ZAI_API_KEY`

## Package Manager Detection

### Supported Package Managers

| Package Manager | Language | API |
|-----------------|----------|-----|
| crates.io | Rust | `https://crates.io/api/v1/crates/{name}` |
| npm | JavaScript/TypeScript | `https://registry.npmjs.org/{name}` |
| PyPI | Python | `https://pypi.org/pypi/{name}/json` |
| Packagist | PHP | `https://packagist.org/search.json?q={name}` |
| LuaRocks | Lua | `https://luarocks.org/modules/{name}` |
| pkg.go.dev | Go | `https://pkg.go.dev/{module}` |

### Detection Flow

1. All package managers are queried concurrently
2. Results are collected with library metadata (description, URL)
3. If multiple matches found: interactive prompt for user selection
4. If single match: auto-selected with confirmation message
5. If no matches: continues as "general topic" mode

## Prompt Templates

### Location

`/research/lib/prompts/*.md`

### Template Variables

| Variable | Description |
|----------|-------------|
| `{{topic}}` | Library/topic name |
| `{{context}}` | Aggregated Phase 1 content (Phase 2 only) |
| `{{question}}` | User-provided additional question |
| `{{overview}}` | Overview document content |
| `{{similar_libraries}}` | Similar libraries document content |
| `{{integration_partners}}` | Integration partners document content |
| `{{use_cases}}` | Use cases document content |
| `{{additional_content}}` | Combined additional question outputs |

### Templates

| File | Purpose | Phase | Model |
|------|---------|-------|-------|
| `overview.md` | Comprehensive library analysis | 1 | GLM-4-7 |
| `similar_libraries.md` | Alternatives and comparisons | 1 | Gemini Flash |
| `integration_partners.md` | Ecosystem libraries | 1 | Gemini Flash |
| `use_cases.md` | Common patterns and examples | 1 | Gemini Flash |
| `changelog.md` | Major-version change history | 1 | GPT-5.2 |
| `additional_question.md` | User-defined research | 1 | Gemini Flash |
| `context.md` | Phase 1 aggregation template | 2 | (template only) |
| `skill.md` | Claude Code skill generation | 2 | GPT-5.2 |
| `deep_dive.md` | Dense reference document | 2 | GPT-5.2 |

## Output Structure

### Directory Layout

```
$RESEARCH_DIR/.research/library/<pkg>/
├── metadata.json           # Research metadata
├── overview.md             # Phase 1: Library overview
├── similar_libraries.md    # Phase 1: Alternatives
├── integration_partners.md # Phase 1: Ecosystem
├── use_cases.md            # Phase 1: Patterns
├── changelog.md            # Phase 1: Major-version history
├── question_1.md           # Phase 1: Additional (optional)
├── question_N.md           # Phase 1: Additional (optional)
├── deep_dive.md            # Phase 2: Dense reference
└── skill/
    ├── SKILL.md            # Phase 2: Skill entry point
    └── *.md                # Phase 2: Supporting docs
```

### Default Output Location

The output directory is determined by:

```rust
fn default_output_dir(topic: &str) -> PathBuf {
    let base = std::env::var("RESEARCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default());
    base.join(".research").join("library").join(topic)
}
```

**Environment variable:** `RESEARCH_DIR`

- If set: `$RESEARCH_DIR/.research/library/<pkg>`
- If not set: `$HOME/.research/library/<pkg>`

### Metadata Schema (v1)

```json
{
  "schema_version": 1,
  "kind": "library",
  "details": {
    "type": "Library",
    "package_manager": "crates.io",
    "language": "Rust",
    "url": "https://crates.io/crates/<pkg>",
    "repository": "https://github.com/..."
  },
  "additional_files": {
    "question_1.md": "How does it compare to X?",
    "question_2.md": "What are the performance characteristics?"
  },
  "created_at": "2025-12-28T10:00:00Z",
  "updated_at": "2025-12-28T10:00:00Z"
}
```

**Schema Evolution:**

- **v0** (legacy): `library_info` at top level, no `schema_version` or `details`
- **v1** (current): `schema_version: 1`, type-specific `details` field with `ResearchDetails` enum

Legacy v0 metadata files are automatically migrated to v1 on load. A backup is created at `metadata.v0.json.backup` before migration.

## Incremental Research (DRY)

### Existence Check

Before running research, the library checks for existing metadata:

```rust
let metadata_path = output_dir.join("metadata.json");
if metadata_path.exists() {
    // Incremental mode: check for overlaps
} else {
    // Full research pipeline
}
```

### Overlap Detection

When additional prompts are provided for existing research:

1. Use Gemini Flash for semantic comparison
2. Compare each new prompt against existing underlying documents
3. Return `PromptOverlap` structure for each prompt

### PromptOverlap Structure

```rust
struct PromptOverlap {
    prompt: String,
    filename: String,
    verdict: OverlapVerdict,
    conflict: Option<String>, // Conflicting file if overlap
}

enum OverlapVerdict {
    New,      // No overlap, proceed
    Conflict, // Potential overlap with existing doc
}
```

### Interactive Selection

When overlaps are detected:

- **Single prompt with conflict:** Confirmation dialog
- **Multiple prompts:** Multi-select with defaults
  - New prompts: selected by default
  - Conflicting prompts: unselected by default

### Regeneration

After adding new underlying documents:

1. Add new files to the research directory
2. Update `metadata.json` with new `additional_files` entries
3. Re-run Phase 2 synthesis with expanded corpus
4. Update `updated_at` timestamp

## Markdown Normalization

### Purpose

Ensures consistent formatting regardless of LLM output style variations.

### Implementation

```rust
fn normalize_markdown(input: &str) -> String {
    // 1. Parse with pulldown-cmark (CommonMark + extensions)
    // 2. Filter empty anchor tags (<a name="..."></a>)
    // 3. Re-serialize with pulldown-cmark-to-cmark
}
```

### Enabled Extensions

- Tables
- Footnotes
- Strikethrough
- Task lists

## Cancellation Handling

### SIGINT Behavior

- Ctrl+C triggers immediate exit via `std::process::exit(130)`
- Exit code 130 = 128 + SIGINT(2) (Unix convention)
- Completed Phase 1 documents are preserved on disk
- TTS announcement is skipped on cancellation

### Graceful Degradation

- If some Phase 1 tasks fail, Phase 2 proceeds with available content
- Empty or missing documents are handled gracefully in context aggregation
- Only if ALL Phase 1 tasks fail does the operation return an error

## Text-to-Speech Notification

### Completion Announcement

Uses the `tts` crate for system text-to-speech:

```rust
// Filter for English, non-compact voices
// Message: "Research for the {topic} library has completed"
```

This provides audio notification when long-running research completes.
