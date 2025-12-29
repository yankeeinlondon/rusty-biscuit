# Research CLI

Command-line interface for the research automation tool.

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/research`.

## Commands

### `research library <package> [prompts...]`

Perform automated research on a software library.

```bash
# Basic research
research library clap

# With additional questions
research library clap "How does it compare to structopt?" "What are the derive macros?"
```

See the [main README](../README.md) for detailed documentation on library research.

### `research list [OPTIONS] [FILTER]...`

List all research topics from the research library.

#### Usage

```bash
research list [OPTIONS] [FILTER]...
```

#### Arguments

- `[FILTER]...` - Glob patterns to filter topics (e.g., "foo", "foo*", "bar")
  - Patterns are case-sensitive
  - Multiple patterns use OR logic (matches any pattern)
  - Example: `research list "clap*" "serde*"` matches topics starting with "clap" or "serde"

#### Options

- `-t, --type <TYPE>` - Filter by research type (repeatable)
  - Types are case-insensitive
  - Multiple types use OR logic (matches any type)
  - Example: `research list -t library -t framework`

- `--json` - Output as JSON instead of terminal format
  - Useful for scripting and integration with other tools

- `-v, --verbose...` - Increase verbosity (-v, -vv, -vvv)
  - Controls logging output level

- `-h, --help` - Print help

#### Examples

**List all topics:**
```bash
research list
```

**Filter by pattern:**
```bash
# List all topics starting with "serde"
research list "serde*"

# List all topics containing "parser"
research list "*parser*"
```

**Filter by type:**
```bash
# List only libraries
research list -t library

# List libraries and frameworks
research list -t library -t framework
```

**Combine filters:**
```bash
# List all libraries starting with "clap"
research list -t library "clap*"
```

**JSON output:**
```bash
# Get JSON output for scripting
research list --json

# Filter and output as JSON
research list -t library "serde*" --json
```

#### Output Format

**Terminal format** (default):

```
- complete-topic [library] : A complete test library with all files present

- incomplete-topic [framework] : An incomplete test framework with missing files
    🐞 missing *final* output deliverables: Brief, Skill
    🐞 missing *underlying* research docs: similar_libraries.md, integration_partners.md, use_cases.md, changelog.md

- corrupt-metadata [library]
    🐞 **metadata.json** missing or has invalid properties
```

Topics are color-coded based on status:
- **RED + BOLD**: Missing output files or metadata.json (critical issues)
- **ORANGE + BOLD**: Missing underlying documents only (minor issues)
- **BOLD**: All files present (no issues)

**JSON format** (`--json`):

```json
[
  {
    "name": "complete-topic",
    "type": "library",
    "description": "A complete test library with all files present",
    "additional_files": [],
    "missing_underlying": [],
    "missing_output": [],
    "missing_metadata": false,
    "location": "/Users/user/.research/library/complete-topic"
  },
  {
    "name": "incomplete-topic",
    "type": "framework",
    "description": "An incomplete test framework with missing files",
    "additional_files": [],
    "missing_underlying": [
      "similar_libraries.md",
      "integration_partners.md",
      "use_cases.md",
      "changelog.md"
    ],
    "missing_output": [
      "brief",
      "skill"
    ],
    "missing_metadata": false,
    "location": "/Users/user/.research/library/incomplete-topic"
  }
]
```

#### Research Directory

The list command searches for topics in:

```
${RESEARCH_DIR:-$HOME}/.research/library/
```

Set the `RESEARCH_DIR` environment variable to use a different location:

```bash
export RESEARCH_DIR=/path/to/research
research list
```

## Global Options

- `-v, --verbose...` - Increase verbosity (-v, -vv, -vvv)
- `--json` - Output logs as JSON
- `-h, --help` - Print help

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RESEARCH_DIR` | Base directory for research output | `$HOME` |
| `OPENAI_API_KEY` | OpenAI API key for GPT-5.2 | (required for research) |
| `GEMINI_API_KEY` | Google Gemini API key | (required for research) |
| `ZAI_API_KEY` | ZAI API key for GLM-4-7 | (required for research) |

## Exit Codes

- `0` - Success
- `1` - Error (check error message and logs)
