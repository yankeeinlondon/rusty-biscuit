# gen-models

Generator for provider model enum files.

## Usage

```bash
# Generate all providers
cargo run -p ai-pipeline-gen

# Generate specific providers
cargo run -p ai-pipeline-gen -- --providers openai,anthropic

# Skip specific providers
cargo run -p ai-pipeline-gen -- --skip zenmux,ollama

# Custom output directory
cargo run -p ai-pipeline-gen -- --output ./models

# Dry run (preview without writing files)
cargo run -p ai-pipeline-gen -- --dry-run

# Verbose output
cargo run -p ai-pipeline-gen -- -v    # INFO
cargo run -p ai-pipeline-gen -- -vv   # DEBUG
cargo run -p ai-pipeline-gen -- -vvv  # TRACE
```

## Environment Variables

Set API keys for providers you want to generate:

| Provider | Variable |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Deepseek | `DEEPSEEK_API_KEY` |
| Gemini | `GEMINI_API_KEY` or `GOOGLE_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` or `OPEN_ROUTER_API_KEY` |
| XAI | `XAI_API_KEY` or `X_AI_API_KEY` |
| ZAI | `ZAI_API_KEY` or `Z_AI_API_KEY` |
| MoonshotAI | `MOONSHOT_API_KEY` or `MOONSHOT_AI_API_KEY` |
| HuggingFace | `HF_TOKEN`, `HUGGINGFACE_TOKEN`, or `HUGGING_FACE_TOKEN` |
| Mira | `MIRA_API_KEY` |

## Output

Generated files are placed in `ai-pipeline/lib/src/rigging/providers/models/` by default.

Each file contains:
- Auto-generated header with timestamp and version
- Enum with `ModelId` derive macro
- One variant per model ID
- `Bespoke(String)` variant for custom model IDs

## Notes

- Providers without API keys configured will be skipped
- Local providers (Ollama) are skipped by default
- Failed providers are logged but don't stop the generation
- Files are written atomically (temp file + rename) to prevent corruption
