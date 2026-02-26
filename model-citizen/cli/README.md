# model-citizen-cli

Binary: `model` - Interactive CLI for managing local LLM models.

## Commands

### `model list [filter]`

Lists all models across all runners in a table or JSON. Optional positional `filter` does case-insensitive substring matching on model names.

```bash
model list                     # Default: alphabetical, table format
model list llama               # Filter by name substring
model list --runner ollama     # Filter by runner
model list --verbose           # Show additional columns (parameters, format)
model list --app               # Sort by source, then name
model list --size              # Sort by size (largest first)
model list --json              # JSON output
```

### `model info <model>`

Shows detailed metadata for a model. If multiple models match, presents an interactive selection prompt (errors in JSON mode for ambiguous matches).

Enriches data lazily (calls Ollama `/api/show` for detailed metadata). Displays:
- Core: name, parameters, context, architecture, quantization, size, source, format
- Extended: families, embedding, heads, layers, vision, function calling, license
- Inference: temperature, top-k, top-p, repeat penalty, capabilities, stop tokens
- Links: HuggingFace repo link (direct if known, search link otherwise)
- Sharing: symlink info, dependent symlinks, share registry entries

### `model search [query...]`

Searches HuggingFace for GGUF models. Query words are joined with spaces. Uses `biscuit-terminal::Table` for rich output with OSC8 hyperlinks.

```bash
model search "qwen2 7b"
model search phi --limit 10 --sort likes
model search --sort trending            # Browse by sort order (no query)
```

Table columns: Repository (hyperlinked), Downloads, Likes, G (GGUF indicator), ST (SafeTensors indicator), Tags. Created/Modified columns appear automatically when sorting by those fields or with `--verbose`.

Sort options: `downloads` (default), `likes`, `trending`, `created`, `modified`

### `model download [query...]`

Searches and downloads GGUF variants from HuggingFace with `indicatif` progress bars.

```bash
model download bartowski/Qwen2.5-7B-Instruct-GGUF  # Direct repo ID (contains /)
model download llama gguf                            # Search, then select repo
model download --sort trending                       # Browse top models (no query)
model download phi --limit 10 --sort likes           # Search with options
model download phi --verbose                         # Show created/modified dates
```

If query contains `/`, treated as direct repo ID (skips search). Otherwise searches HuggingFace and presents interactive repo selection. When no query is provided, shows "Browsing top models by [sort]..." and lists results by sort order. Always shows interactive multi-select for variant choice with size/RAM estimates.

Output directory priority: `--output` flag > `MODELS_DIR` env > shared models dir from config > current directory.

### `model remove <model>`

Removes models with confirmation. Shows symlink warnings for dependent links. Matches by substring on model name or ID.

```bash
model remove mistral
model remove mistral --runner ollama --force
```

### `model run [model]`

Runs a local GGUF model in `llama-server` and opens the browser GUI once the server is reachable.

```bash
model run llama3                              # Launch a specific model
model run                                     # Interactive picker (TTY only)
model run --runner llamacpp                   # Filter by source runner
model run llama3 --host 127.0.0.1 --port 8081
model run llama3 --ctx-size 8192 --threads 8 --n-gpu-layers 35
model run llama3 --api-key my-secret --no-browser
model run llama3 --llama-server-bin /opt/llama.cpp/llama-server
model run llama3 --dry-run                    # Print command only
```

Behavior notes:
- Only runnable GGUF file paths are considered (`.gguf` or valid GGUF header)
- Ambiguous names prompt in interactive terminals; non-interactive mode returns an error
- If `llama-server` is missing, the command explains how to set `--llama-server-bin`
- Port conflicts are detected before launch with a clear error
- Browser launch failures are warnings; the server stays running

### `model completions`

Prints shell completion setup instructions for Bash, Zsh, and Fish.

When dynamic completions are enabled (`COMPLETE=<shell> model`), `model run [model]`
completes discovered runnable GGUF models (unique names plus full model IDs).

## Global Options

- `--json` - Output JSON instead of terminal tables (applies to all commands)

## Key Dependencies

- `clap` (derive) + `clap_complete` (dynamic) - CLI parsing and completions
- `biscuit-terminal` - Rich table rendering with hyperlinks
- `serde_json` - JSON output for list, info, search commands
- `inquire` - Interactive prompts (model selection, download variants)
- `indicatif` - Progress bars for downloads
- `color-eyre` - Error reporting
