# Searching Models

Advanced techniques for discovering models on the Hugging Face Hub using the API.

## Basic Search

### Simple Search by Name

```python
from huggingface_hub import HfApi

api = HfApi()

# Search by model name
models = api.list_models(search="bert-base")
print(f"Found {len(models)} models matching 'bert-base'")

# Display results
for model in models[:5]:
    print(f"- {model.modelId}: {model.downloads:,} downloads")
```

### Search with Sorting

```python
# Sort by downloads (most popular)
popular_models = api.list_models(
    search="llama",
    sort="downloads",
    direction=-1,  # Descending
    limit=10
)

# Sort by recent updates
recent_models = api.list_models(
    sort="modified",
    direction=-1,
    limit=10
)
```

## Advanced Filtering

### Using ModelFilter

```python
from huggingface_hub import ModelFilter

# Create comprehensive filter
filter = ModelFilter(
    task="text-generation",
    library="pytorch",
    language=["english", "french"],
    tags=["gpt", "llama"],
    trained_dataset="common_crawl"
)

# Apply filter
results = api.list_models(filter=filter)
```

### Available Filter Parameters

| Parameter | Description | Example Values |
|-----------|-------------|----------------|
| `task` | Model's intended task | `text-generation`, `image-classification`, `automatic-speech-recognition` |
| `library` | Framework used | `pytorch`, `tensorflow`, `jax`, `transformers` |
| `language` | Supported languages | `english`, `french`, `multilingual` |
| `tags` | Custom tags | `gpt`, `llama`, `finetuned`, `instruction-tuned` |
| `author` | Model creator | `facebook`, `google`, `microsoft`, `TheBloke` |
| `dataset` | Training dataset | `common_crawl`, `wikitext`, `imagenet` |

### Complex Search Example

```python
from huggingface_hub import ModelSearchArguments, ModelFilter

# Find instruction-tuned models for specific use case
args = ModelSearchArguments()  # For tab-completion

# Multi-criteria search
filter = ModelFilter(
    task="text-generation",
    library=["pytorch", "safetensors"],
    language="english",
    tags=["instruction-tuned", "chat"],
)

# Search with pagination
all_results = []
limit = 50

results = api.list_models(
    filter=filter,
    limit=limit,
    sort="likes",  # Sort by popularity
    direction=-1
)
all_results.extend(results)

# Continue fetching if more results exist
# Note: HF API handles pagination internally
```

## Semantic Search

### Finding Similar Models

```python
# Semantic search (when available)
# Note: This is a conceptual example - exact API may vary
query = "A large language model optimized for coding tasks"

# Some approaches:
# 1. Search by description keywords
coding_models = api.list_models(
    search="code generation programming",
    task="text-generation"
)

# 2. Use specific tags
filter = ModelFilter(
    tags=["code", "programming", "code-generation"],
    task="text-generation"
)
code_models = api.list_models(filter=filter)
```

## Processing Search Results

### Model Information

```python
# Get detailed model info
for model in results[:3]:
    # Basic info
    print(f"\nModel: {model.modelId}")
    print(f"Author: {model.author}")
    print(f"Downloads: {model.downloads:,}")
    print(f"Likes: {model.likes}")
    print(f"Last modified: {model.lastModified}")

    # Tags and metadata
    print(f"Tags: {', '.join(model.tags[:5])}")

    # Get full model card if needed
    full_info = api.model_info(model.modelId)
    print(f"Model size: {full_info.safetensors.get('total', 'Unknown')}")
```

### Filtering Results Locally

```python
# Additional local filtering
def filter_by_size(models, max_params_b=7):
    """Filter models by parameter count"""
    filtered = []
    for model in models:
        try:
            # This is approximate - actual implementation depends on model card
            if "7b" in model.modelId.lower():
                size = 7
            elif "13b" in model.modelId.lower():
                size = 13
            else:
                size = 0

            if size <= max_params_b:
                filtered.append(model)
        except:
            pass
    return filtered

# Apply local filter
small_models = filter_by_size(results, max_params_b=7)
```

## Search Patterns

### Finding Quantized Models

```python
# Search for quantized versions
base_model = "meta-llama/Llama-2-7b"
quantized_filter = ModelFilter(
    search=f"{base_model} GGUF GPTQ AWQ",
    task="text-generation"
)

quantized_versions = api.list_models(filter=quantized_filter)
```

### Finding Fine-tuned Variants

```python
# Find fine-tuned versions of a base model
base = "bert-base-uncased"
finetuned = api.list_models(
    search=base,
    filter=ModelFilter(tags=["finetuned"])
)

print(f"Found {len(finetuned)} fine-tuned variants of {base}")
```

### Organization Models

```python
# List all models from an organization
org_models = api.list_models(
    author="facebook",
    sort="downloads",
    direction=-1
)

# Filter org models by task
org_filter = ModelFilter(
    author="openai",
    task="text-generation"
)
openai_llms = api.list_models(filter=org_filter)
```

## Best Practices

### Efficient Searching

```python
# Cache search results
from functools import lru_cache

@lru_cache(maxsize=100)
def cached_search(query, task=None):
    filter = ModelFilter(task=task) if task else None
    return api.list_models(search=query, filter=filter, limit=20)

# Reuse cached results
results1 = cached_search("bert", "fill-mask")
results2 = cached_search("bert", "fill-mask")  # From cache
```

### Batch Model Info Retrieval

```python
# Get detailed info for multiple models efficiently
model_ids = [m.modelId for m in results[:5]]

detailed_info = {}
for model_id in model_ids:
    try:
        info = api.model_info(model_id)
        detailed_info[model_id] = {
            "pipeline_tag": info.pipeline_tag,
            "library_name": info.library_name,
            "model_size": info.safetensors.get("total", 0) if hasattr(info, "safetensors") else 0
        }
    except Exception as e:
        print(f"Error fetching {model_id}: {e}")
```

## Search CLI Tool Example

```python
#!/usr/bin/env python3
# hf_search.py - CLI tool for searching models

import argparse
from huggingface_hub import HfApi, ModelFilter

def main():
    parser = argparse.ArgumentParser(description="Search Hugging Face models")
    parser.add_argument("query", help="Search query")
    parser.add_argument("--task", help="Filter by task")
    parser.add_argument("--limit", type=int, default=10, help="Number of results")
    parser.add_argument("--sort", default="downloads", help="Sort by field")

    args = parser.parse_args()

    api = HfApi()
    filter = ModelFilter(task=args.task) if args.task else None

    models = api.list_models(
        search=args.query,
        filter=filter,
        sort=args.sort,
        direction=-1,
        limit=args.limit
    )

    for i, model in enumerate(models, 1):
        print(f"{i}. {model.modelId} ({model.downloads:,} downloads)")

if __name__ == "__main__":
    main()
```