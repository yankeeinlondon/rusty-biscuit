---
blast_radius:
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/lib/src/composition/sequence.rs
  - claudine/lib/src/composition/types.rs
---
# Claudine's `sequence` command

The `sequence` command allows you to run a serial sequence of composition steps defined in a single Markdown document. It is ideal for complex workflows that require multiple agent interactions, such as researching a topic across multiple providers, performing a multi-stage code refactor, or generating a series of related documents.

## Usage

```bash
claudine sequence <file> [flags]
```

## Frontmatter Configuration

A document is recognized as a sequence if it contains a `sequence` key in its frontmatter.

### Inline Sequence

You can define the steps directly in the Markdown file's frontmatter as a list of strings or objects.

```markdown
---
sequence:
  - "Step 1: Research"
  - "Step 2: Implement"
  - name: "Step 3: Test"
    framework: "jest"
---

Prompt for all steps: {{state.name || state}}
```

- **String steps**: The string is used as both the step name and the `state` variable.
- **Object steps**: Must contain a `name` property. The entire object is available via the `state` variable.

### External Sequence

For reusable sequences, you can point to an external YAML file.

```markdown
---
sequence: "@fixtures/steps.yaml"
---
```

External YAML files support two formats:

#### 1. Plain List Format
```yaml
sequence:
  - "Step A"
  - name: "Step B"
    option: "value"
```

#### 2. Templated Format
This format allows you to define a common template for step properties, reducing duplication.

```yaml
kind: sequence
template:
  description: "Executing {{name}} for {{target}}"
list:
  - name: "Lint"
    target: "src/"
  - name: "Test"
    target: "tests/"
```

Templates support `{{key}}` and `{{key || 'default'}}` fallback syntax. Template keys cannot collide with [Reserved Overlay Keys](#reserved-overlay-keys).

### `fail_fast`

Controls whether the sequence should stop immediately if a step fails.

- **Default**: `true`
- **Frontmatter**: `fail_fast: false`
- **CLI Override**: `--fail-fast false` (accepts `true`/`false`, `1`/`0`, `yes`/`no`)

A step is considered failed if the composition fails (e.g., template error) or if the provider CLI exits with a non-zero code.

## Template Variables (Overlay)

Each step in the sequence has access to a set of automatically injected variables. These "overlay" variables take precedence over any values provided via `--set`.

| Variable | Description |
|----------|-------------|
| `state` | The current step configuration (string or object). |
| `previous_state` | The configuration of the previous step (`null` for the first step). |
| `next_state` | The configuration of the next step (`null` for the last step). |
| `is_first` | `true` if this is the first step in the sequence. |
| `is_last` | `true` if this is the last step in the sequence. |
| `step` | The 1-based index of the current step. |
| `total_steps` | The total number of steps in the sequence. |

### Reserved Overlay Keys
The following keys are reserved and will always be overwritten by the sequence orchestrator: `state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, `total_steps`.

## Execution Behavior

### Serial Execution
Steps are executed one after another in the order they are defined. If `fail_fast` is enabled, execution stops as soon as a step fails.

### Shared Approval Cache
Claudine maintains a shared shell-approval cache for the duration of the sequence run. If you approve a shell command with "Allow once" in an early step, that approval carries over to subsequent steps in the same sequence, preventing redundant prompts.

### Environment Variables
The `FAIL_FAST` environment variable is injected into each step's session, reflecting the effective fail-fast setting for the run.

## CLI Flags

The `sequence` command inherits all shared composition flags:

- **Provider Selection**: `--claude`, `--gemini`, `--provider <NAME>`, etc.
- **Session Control**: `--interactive` (`-i`), `--timeout <SECONDS>`, `--yolo` (`-y`).
- **Resource Management**: `--mcp`, `--use <SERVERS>`, `--repo`.
- **System Prompt**: `--append-system-prompt` (`--asp`), `--replace-system-prompt` (`--rsp`).
- **Output Control**: `--output <FORMAT>`, `--quiet` (`-q`), `--silent`.
- **Overrides**: `--set <JSON>`, `--model <MODEL>`.

## Example: Multi-Provider Research

```markdown
---
sequence:
  - name: "Claude"
    provider: "claude"
  - name: "Gemini"
    provider: "gemini"
fail_fast: false
---

# Research Task
Research the following topic using {{state.name}}:

{{topic || 'Rust 2024 Edition changes'}}
```

Run with: `claudine sequence research.md --set '{"topic": "Async traits in Rust"}'`
