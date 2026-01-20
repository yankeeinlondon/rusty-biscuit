---
name: ollama
description: Expert knowledge for working with Ollama - running LLMs locally, using native and OpenAI-compatible APIs, managing model storage, and creating custom models with Modelfiles
---

# Ollama

Comprehensive expertise for deploying and managing large language models locally with Ollama, including API integration, storage optimization, and model customization.

## Core Principles

- **Local-first execution** - Models run entirely on your hardware, no cloud dependencies
- **Content-addressable storage** - Efficient deduplication via SHA-256 hashing
- **Two API interfaces** - Native API for full control, OpenAI-compatible for easy migration
- **Declarative configuration** - Modelfiles define behavior without modifying weights
- **Cross-platform consistency** - Works identically on Windows, macOS, and Linux

## Quick Reference

### Starting Ollama
```bash
# Run a model (pulls if needed)
ollama run llama3

# Pull model explicitly
ollama pull mistral

# List models
ollama list
```

### API Usage (Native)
```bash
# Chat completion
curl -X POST http://localhost:11434/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3",
    "messages": [{"role": "user", "content": "Hello!"}],
    "options": {"temperature": 0.7, "num_ctx": 4096}
  }'
```

### API Usage (OpenAI Compatible)
```bash
# Point OpenAI clients to localhost
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_API_KEY=ollama  # Can be anything
```

### Basic Modelfile
```dockerfile
FROM llama3
PARAMETER temperature 0.7
PARAMETER num_ctx 4096
SYSTEM "You are a helpful assistant."
```

## Storage Locations

| OS | Default Path | Environment Variable |
|----|--------------|---------------------|
| **macOS** | `~/.ollama/models/` | `OLLAMA_MODELS` |
| **Linux** | `/usr/share/ollama/.ollama/models/` | `OLLAMA_MODELS` |
| **Windows** | `%USERPROFILE%\.ollama\models\` | `OLLAMA_MODELS` |

## Detailed Topics

### API Integration
- [Native API](./api-native.md) - Full control with `/api/*` endpoints, model lifecycle management
- [OpenAI Compatibility](./api-openai.md) - Drop-in replacement using `/v1/*` endpoints

### Model Management
- [Storage Architecture](./storage.md) - Blob storage, manifests, and optimization strategies
- [Modelfiles](./modelfiles.md) - Creating custom models with parameters, templates, and adapters

### Advanced Usage
- [Offline Deployment](./storage.md#offline-deployment) - Running models in air-gapped environments
- [Performance Tuning](./api-native.md#performance-parameters) - GPU layers, context windows, thread counts

## Key Differences from OpenAI

| Feature | OpenAI | Ollama |
|---------|--------|--------|
| **Execution** | Cloud | Local |
| **Models** | Pre-selected | Pull any model |
| **Context Window** | Model-dependent | User configurable |
| **API Key** | Required | Optional/Ignored |
| **Streaming Format** | SSE with `choices` | SSE with `message` |

## Resources

- [Official Documentation](https://github.com/ollama/ollama)
- [Model Library](https://ollama.ai/library)
- [API Reference](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [Modelfile Reference](https://github.com/ollama/ollama/blob/main/docs/modelfile.md)