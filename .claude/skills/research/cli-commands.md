# CLI Commands Reference

Complete reference for the `research` CLI commands.

## Global Options

```bash
research [OPTIONS] <COMMAND>

Options:
  -v, -vv, -vvv    Increase verbosity (info, debug, trace)
  --json           Output logs as JSON
  -h, --help       Show help
```

### Verbosity Levels

| Flag | Level | What's Shown |
|------|-------|--------------|
| (none) | WARN | Failures and recoverable issues |
| `-v` | INFO | Tool calls, phase transitions, research progress |
| `-vv` | DEBUG | Tool arguments, API requests, intermediate results |
| `-vvv` | TRACE | Request/response bodies, verbose internals |

## research library

Research a software library.

```bash
research library <TOPIC> [QUESTIONS...] [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TOPIC` | Library name or `-` to read from stdin |
| `QUESTIONS` | Additional research questions (optional) |

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <DIR>` | Output directory (default: `$RESEARCH_DIR/.research/library/<TOPIC>`) |
| `--skill` | Regenerate skill files from existing research only |
| `--force` | Force recreation of all documents (bypass incremental mode) |

### Examples

```bash
# Basic research
research library clap

# Read topic from stdin
echo "tokio" | research library -

# With additional questions
research library clap "How does it compare to structopt?" "What are the derive macros?"

# Custom output filenames (arrow syntax)
research library clap "comparison -> How does it compare to structopt?"

# Multiple custom filenames
research library tokio \
  "runtime -> How does the runtime work?" \
  "channels -> What channel types are available?"

# Force full re-research
research library clap --force

# Regenerate skill from existing research
research library clap --skill
```

### Custom Filename Syntax

Format: `"filename -> prompt text"`

- Filename will be sanitized (lowercase, hyphens for spaces, `.md` appended)
- Cannot use reserved names: `overview`, `similar_libraries`, `integration_partners`, `use_cases`, `changelog`, `deep_dive`, `brief`
- Can mix custom and default naming in same command

## research list

List all research topics.

```bash
research list [FILTERS...] [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILTERS` | Glob patterns to filter topics (e.g., `"clap*"`, `"*parser*"`) |

### Options

| Option | Description |
|--------|-------------|
| `-t, --type <TYPE>` | Filter by research type (repeatable) |
| `--verbose` | Show detailed metadata with issues |
| `--json` | Output as JSON |
| `--migrate` | Migrate all v0 metadata files to v1 schema |

### Examples

```bash
# List all topics
research list

# Filter by pattern
research list "serde*"
research list "*parser*"

# Filter by type
research list -t library
research list -t library -t framework

# Combine filters
research list -t library "clap*"

# Verbose with issues
research list --verbose

# JSON output
research list --json

# Migrate old metadata
research list --migrate
```

### Output Formats

**Terminal (default)**:
```
- clap [library] : A full-featured command-line argument parser
- incomplete-topic [framework] : An incomplete framework
    🐞 missing *final* output deliverables: Brief, Skill
    🐞 missing *underlying* research docs: similar_libraries.md
```

Color coding:
- **RED + BOLD**: Missing output files or metadata.json
- **ORANGE + BOLD**: Missing underlying documents only
- **BOLD**: All files present

**JSON** (`--json`):
```json
[
  {
    "name": "clap",
    "type": "library",
    "description": "A full-featured CLI parser",
    "missing_underlying": [],
    "missing_output": [],
    "missing_metadata": false,
    "location": "/home/user/.research/library/clap"
  }
]
```

## research link

Create symbolic links from research skills to framework directories.

```bash
research link [FILTERS...] [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILTERS` | Glob patterns to filter topics |

### Options

| Option | Description |
|--------|-------------|
| `-t, --type <TYPE>` | Filter by research type (repeatable) |
| `--json` | Output as JSON |

### Link Targets

Creates symlinks in:
- `~/.claude/skills/<topic>/` (Claude Code)
- `~/.opencode/skill/<topic>/` (OpenCode, if detected)

### Examples

```bash
# Link all research skills
research link

# Link only libraries
research link -t library

# Link specific topics
research link "clap*" "serde*"
```

## research show

Open a research topic's deep dive document.

```bash
research show <TOPIC>
```

Opens `$RESEARCH_DIR/.research/library/<TOPIC>/deep_dive.md` in the system's default application.

### Examples

```bash
research show clap
research show tokio
```

## research api

Research a public API (currently placeholder).

```bash
research api <API_NAME> [QUESTIONS...] [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `API_NAME` | The API name (e.g., `stripe`, `github`, `openai`) |
| `QUESTIONS` | Additional research questions |

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <DIR>` | Output directory (default: `$RESEARCH_DIR/.research/api/<API_NAME>`) |
| `-f, --force` | Force recreation even if research exists |

**Note**: API research prompts are not yet implemented. Currently creates directory structure only.

## research pull

Pull a research skill from user scope to current repository.

```bash
research pull <TOPIC> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TOPIC` | Topic to pull from `~/.research/library/<topic>/` |

### Options

| Option | Description |
|--------|-------------|
| `--local` | Also copy underlying research documents |

### What It Does

1. **Git detection**: Fails if not in a git repository
2. **Skill copying**: Copies `~/.research/library/<topic>/skill/` to `.claude/skills/<topic>/`
3. **Framework symlinks**: Creates relative symlinks if detected:
   - `.roo/skills/<topic>` if `.roo/` exists (Roo framework)
   - `.opencode/skill/<topic>` if `AGENTS.md` exists (OpenCode framework)
4. **Local research** (with `--local`): Copies underlying docs to `.claude/research/<topic>/`

### Output Locations

```
.claude/
├── skills/<topic>/           # Skill files (always)
│   ├── SKILL.md
│   └── *.md
└── research/<topic>/         # Research docs (with --local)
    ├── overview.md
    ├── similar_libraries.md
    └── ...

.roo/skills/<topic>/          # Symlink (if .roo/ exists)
.opencode/skill/<topic>/      # Symlink (if AGENTS.md exists)
```

### Examples

```bash
# Pull skill only
research pull clap

# Pull with underlying research
research pull tokio --local
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (check error message) |
| 130 | Cancelled (Ctrl+C) |
