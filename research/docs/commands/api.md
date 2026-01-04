# `api` command

The **research** CLI has an `api` command which generates research documentation for public APIs. This command creates a research directory structure for API documentation, including endpoints, authentication, and integration patterns.

## Syntax

> **research api** \<api-name\> [QUESTIONS...] [FLAGS]

### Parameters

- `api-name` **(required)**: The name of the API to research
  - Examples: `stripe`, `github`, `openai`, `twilio`

- `questions` (optional): Additional research questions to answer

### Switches

- `--output`, `-o` <path>: Specify output directory for research artifacts
  - Defaults to `$RESEARCH_DIR/.research/api/<api-name>`

- `--force`, `-f`: Force recreation even if research exists

## Output

The command generates artifacts in the output directory under `api/<api-name>/`:

### Directory Layout

```
$RESEARCH_DIR/.research/api/<api-name>/
├── metadata.json           # Research metadata with kind: "api"
├── overview.md             # API overview, endpoints, authentication
├── similar_apis.md         # Alternative APIs in the space
├── use_cases.md            # Common integration patterns
└── skill/
    └── SKILL.md            # Claude Code skill format
```

### Metadata Schema

```json
{
  "schema_version": 1,
  "kind": "api",
  "details": {
    "type": "Api"
  },
  "additional_files": {},
  "created_at": "2026-01-04T...",
  "updated_at": "2026-01-04T..."
}
```

## Examples

### Basic Usage

Research the Stripe API:

```bash
research api stripe
```

Research with additional questions:

```bash
research api github "How does rate limiting work?" "What are the webhook events?"
```

### Custom Output Location

Generate research in a project-specific directory:

```bash
research api openai --output ./docs/research
```

### Force Regeneration

Regenerate all research from scratch:

```bash
research api stripe --force
```

## Current Status

The `api` command is currently a placeholder that:

1. Creates the research directory structure
2. Initializes `metadata.json` with `ResearchKind::Api`
3. Reports success

Full LLM-powered API research (overview, endpoints, authentication docs) will be implemented in a future update.

## Integration with Claude Code

After generating research, use the `research link` command to create symbolic links:

```bash
research api stripe
research link stripe
```

## Notes

- The command uses the same output structure as `research library`
- API research outputs are stored under `.research/api/` (not `.research/library/`)
- The `ApiDetails` struct in metadata will be expanded as API research features are added
