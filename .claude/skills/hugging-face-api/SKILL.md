---
name: hugging-face-api
description: Expert knowledge for working with Hugging Face REST APIs - including Hub API for model search and downloads, Inference API for serverless model execution, and programmatic access using Python/Rust
---

# Hugging Face REST API

Comprehensive expertise for interacting with Hugging Face's API ecosystem, including model discovery, programmatic downloads, inference operations, and GGUF model management.

## Core APIs

| API | Purpose | Use Case |
|-----|---------|----------|
| **Hub API** | Model/dataset metadata, search, repository management | Discovering and managing models |
| **Inference API** | Serverless model inference | Quick prototyping, light production |
| **Inference Endpoints** | Dedicated infrastructure | Production workloads |
| **Inference Providers** | Unified third-party access | Cross-provider compatibility |

## Quick Reference

### Authentication Setup
```python
from huggingface_hub import HfApi, InferenceClient

# Initialize clients
api = HfApi(token="hf_...")
client = InferenceClient(token="hf_...")
```

### Model Search
```python
from huggingface_hub import ModelFilter

# Search with filters
filter = ModelFilter(
    task="text-generation",
    library="pytorch",
    language="english"
)
models = api.list_models(filter=filter, limit=10)
```

### Run Inference
```python
# Text generation
result = client.text_generation(
    "The future of AI is",
    model="gpt2",
    max_new_tokens=50
)

# Image classification
results = client.image_classification(
    image,
    model="google/vit-base-patch16-224"
)
```

## Key Principles

- **Use client libraries** when available (Python/JavaScript) for simplified interactions
- **Implement proper authentication** with fine-grained tokens for production
- **Handle rate limits gracefully** with exponential backoff
- **Cache model downloads** to avoid redundant transfers
- **Choose the right inference tier** - serverless for prototypes, endpoints for production

## Detailed Topics

### Setup & Configuration
- [Authentication & Tokens](./authentication.md) - API tokens, client setup, environment configuration

### Model Operations
- [Searching Models](./search-models.md) - Advanced filtering, semantic search, result processing
- [Downloading Models](./download-models.md) - Programmatic downloads, cache management, version control

### Inference
- [Running Inference](./inference.md) - Serverless API, streaming, batch processing, custom endpoints

### Specialized Formats
- [GGUF Models](./gguf-models.md) - Finding, listing, and downloading quantized models (with Rust examples)

## API Limits & Pricing

| Tier | Rate Limits | Monthly Cost | Use Case |
|------|------------|--------------|----------|
| **Free** | ~Few hundred/hour | $0 | Development, testing |
| **Pro** | 20x higher | $9 | Light production |
| **Enterprise** | Custom | Custom | Scale production |

## Resources

- [Inference Providers Documentation](https://huggingface.co/docs/inference-providers/index)
- [Hub Python Library](https://github.com/huggingface/huggingface_hub)
- [API Reference](https://huggingface.co/docs/huggingface_hub/package_reference/hf_api)
- [Pricing Calculator](https://huggingface.co/pricing)