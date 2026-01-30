# Pipeline Architecture

The research system uses a two-phase LLM pipeline optimized for parallel execution and comprehensive output.

## Phase 1: Underlying Research

All Phase 1 tasks run concurrently via `tokio::join_all`:

| Task | Output File | Model | Rationale |
|------|-------------|-------|-----------|
| Overview | `overview.md` | ZAI GLM-4.7 | Comprehensive analysis needs strong reasoning |
| Similar Libraries | `similar_libraries.md` | Gemini Flash | Comparison is straightforward, speed matters |
| Integration Partners | `integration_partners.md` | Gemini Flash | Ecosystem mapping is parallelizable |
| Use Cases | `use_cases.md` | Gemini Flash | Pattern extraction is well-defined |
| Changelog | `changelog.md` | OpenAI GPT-5.2 | Version analysis requires deep understanding |
| Additional Questions | `question_N.md` | Gemini Flash | User questions benefit from speed |

### Tools Available in Phase 1

Phase 1 agents have access to:
- **BraveSearchTool**: Web search via Brave Search API (requires `BRAVE_API_KEY`)
- **ScreenScrapeTool**: Web page content extraction

Phase 2 agents run without tools (consolidation only).

### Execution Characteristics

- All tasks are independent and can fail without affecting others
- Progress is reported as tasks complete
- Ctrl+C exits immediately (code 130), preserving completed results
- Requires 50% success OR all 5 core prompts to proceed to Phase 2

## Phase 2: Synthesis

Runs after Phase 1 completes, using combined context from all Phase 1 documents.

### Phase 2a (Parallel)

| Task | Output | Model |
|------|--------|-------|
| Skill Generation | `skill/SKILL.md` + supporting files | GPT-5.2 |
| Deep Dive | `deep_dive.md` | GPT-5.2 |

Both use GPT-5.2 because synthesis requires:
- Cross-document reasoning
- Complex summarization
- Structured output generation

### Phase 2b (Sequential)

| Task | Output | Model |
|------|--------|-------|
| Brief | `brief.md` | Gemini Flash |

Runs after `deep_dive.md` completes because it derives from that document.

## Model Selection Strategy

### Fast Models (Gemini Flash)
- Phase 1 parallel research where speed matters
- Tasks are straightforward information gathering
- Error tolerance is high (can proceed with partial results)

### Strong Models (GPT-5.2)
- Phase 2 synthesis requiring multi-document reasoning
- Changelog analysis (understanding repository history)
- Skill structure generation (complex formatting requirements)

### Overview Model (ZAI GLM-4.7)
- Comprehensive library analysis
- Falls back to Gemini Flash if `ZAI_API_KEY` not set

## Prompt Templates

Located at `/research/lib/prompts/*.md`:

| Template | Phase | Variables |
|----------|-------|-----------|
| `overview.md` | 1 | `{{topic}}` |
| `similar_libraries.md` | 1 | `{{topic}}` |
| `integration_partners.md` | 1 | `{{topic}}` |
| `use_cases.md` | 1 | `{{topic}}` |
| `changelog.md` | 1 | `{{topic}}` |
| `additional_question.md` | 1 | `{{topic}}`, `{{question}}` |
| `context.md` | 2 | Aggregation template |
| `skill.md` | 2 | `{{topic}}`, `{{context}}`, all Phase 1 outputs |
| `deep_dive.md` | 2 | `{{topic}}`, `{{context}}`, all Phase 1 outputs |
| `brief.md` | 2b | Derived from `deep_dive.md` |

## Package Manager Detection

Before research begins, all package managers are queried concurrently:

| Manager | Language | API |
|---------|----------|-----|
| crates.io | Rust | `https://crates.io/api/v1/crates/{name}` |
| npm | JavaScript/TypeScript | `https://registry.npmjs.org/{name}` |
| PyPI | Python | `https://pypi.org/pypi/{name}/json` |
| Packagist | PHP | `https://packagist.org/search.json?q={name}` |
| LuaRocks | Lua | HEAD request + fallback search |
| pkg.go.dev | Go | HEAD request with prefix attempts |

Detection flow:
1. All managers queried via `tokio::join!`
2. Results collected with metadata (description, URL, repository)
3. Multiple matches: interactive prompt for user selection
4. Single match: auto-selected with confirmation
5. No matches: continues as "general topic" mode

## Cancellation & Graceful Degradation

### SIGINT Handling
- Ctrl+C triggers `std::process::exit(130)` (Unix convention: 128 + signal)
- Completed Phase 1 documents are preserved
- TTS announcement is skipped on cancellation

### Graceful Degradation
- If some Phase 1 tasks fail, Phase 2 proceeds with available content
- Empty/missing documents handled gracefully in context aggregation
- Only if ALL Phase 1 tasks fail does the operation error

## Markdown Normalization

All outputs are normalized via `pulldown-cmark`:
- Consistent formatting regardless of LLM output variations
- Filters empty anchor tags
- Enabled extensions: Tables, Footnotes, Strikethrough, Task lists

## TTS Notification

On successful completion (not cancelled):
```rust
speak_when_able(&format!("Research for the {} library has completed", topic), &TtsConfig::default()).await;
```

Uses `biscuit-speaks` for cross-platform text-to-speech.
