# Downloading Models

Programmatic methods for downloading models from the Hugging Face Hub using the API.

## Basic Downloads

### Download Entire Model

```python
from huggingface_hub import snapshot_download

# Download complete model repository
model_path = snapshot_download(
    repo_id="bert-base-uncased",
    cache_dir="/custom/cache/path",  # Optional
    local_dir="./local_model",       # Optional
    local_dir_use_symlinks=False     # Copy files instead of symlinks
)
print(f"Model downloaded to: {model_path}")
```

### Download Specific Files

```python
from huggingface_hub import hf_hub_download

# Download single file
config_path = hf_hub_download(
    repo_id="bert-base-uncased",
    filename="config.json",
    cache_dir="/custom/cache/path"
)

# Download file from subfolder
model_path = hf_hub_download(
    repo_id="facebook/opt-350m",
    filename="pytorch_model.bin",
    subfolder="",  # Root folder
    cache_dir=None  # Use default cache
)
```

## Advanced Download Options

### Download with Patterns

```python
# Download only specific file types
model_path = snapshot_download(
    repo_id="microsoft/phi-2",
    allow_patterns=["*.json", "*.model", "*.txt"],  # Include patterns
    ignore_patterns=["*.bin", "*.h5"]  # Exclude patterns
)

# Download specific model shards
snapshot_download(
    repo_id="meta-llama/Llama-2-7b-hf",
    allow_patterns=["*00001-of-*", "*.json", "tokenizer*"]
)
```

### Version Control

```python
# Download specific revision
model_path = snapshot_download(
    repo_id="gpt2",
    revision="main",  # Branch name
    # revision="v1.0",  # Tag
    # revision="d50d5e9"  # Commit hash
)

# Download from pull request
pr_model = snapshot_download(
    repo_id="bigcode/starcoder",
    revision="refs/pr/42"
)
```

## Cache Management

### Understanding Cache Structure

```python
# Default cache location: ~/.cache/huggingface/hub/
# Structure:
# hub/
# ├── models--{org}--{model}/
# │   ├── blobs/           # Actual files (deduplicated)
# │   ├── refs/            # Branch/tag references
# │   └── snapshots/       # Commits with symlinks to blobs

from huggingface_hub import scan_cache_dir

# Analyze cache
cache_info = scan_cache_dir()
print(f"Total cache size: {cache_info.size_on_disk_str}")
print(f"Number of repos: {len(cache_info.repos)}")

# List cached repos
for repo in cache_info.repos:
    print(f"- {repo.repo_id}: {repo.size_on_disk_str}")
```

### Cache Control

```python
# Force re-download (ignore cache)
fresh_download = snapshot_download(
    repo_id="gpt2",
    force_download=True
)

# Use cache only (offline mode)
import os
os.environ["HF_HUB_OFFLINE"] = "1"

# Now downloads will fail if not cached
try:
    model = snapshot_download("gpt2")
except Exception as e:
    print("Model not in cache!")
```

## Download Strategies

### Partial Model Downloads

```python
# For large models, download in stages
def download_large_model(repo_id):
    # Step 1: Download metadata and tokenizer
    print("Downloading metadata...")
    snapshot_download(
        repo_id=repo_id,
        allow_patterns=["*.json", "tokenizer*", "*.txt"]
    )

    # Step 2: Download model shards progressively
    for i in range(1, 5):  # Assuming 4 shards
        print(f"Downloading shard {i}...")
        snapshot_download(
            repo_id=repo_id,
            allow_patterns=[f"*{i:05d}-of-*"]
        )

# Usage
download_large_model("meta-llama/Llama-2-13b-hf")
```

### Resume Interrupted Downloads

```python
# Downloads automatically resume from where they left off
# The hub client handles this internally

import signal
import sys

def signal_handler(sig, frame):
    print('\nDownload interrupted! Run again to resume.')
    sys.exit(0)

signal.signal(signal.SIGINT, signal_handler)

# Large download that can be resumed
model_path = snapshot_download(
    repo_id="meta-llama/Llama-2-70b-hf",
    local_dir="./llama-70b"
)
```

## Integration with ML Libraries

### Transformers Integration

```python
from transformers import AutoModel, AutoTokenizer

# Download and load model
model_id = "bert-base-uncased"

# Method 1: Let transformers handle download
model = AutoModel.from_pretrained(model_id)
tokenizer = AutoTokenizer.from_pretrained(model_id)

# Method 2: Pre-download then load
local_path = snapshot_download(model_id)
model = AutoModel.from_pretrained(local_path)
tokenizer = AutoTokenizer.from_pretrained(local_path)
```

### Custom Download Function

```python
def download_with_progress(repo_id, local_dir=None):
    """Download model with custom progress tracking"""
    from tqdm import tqdm
    import time

    # Get repo info first
    from huggingface_hub import HfApi
    api = HfApi()
    info = api.model_info(repo_id)

    print(f"Downloading {repo_id}")
    print(f"Total files: {len(info.siblings)}")

    # Download with progress
    with tqdm(total=len(info.siblings)) as pbar:
        def progress_callback(filename):
            pbar.update(1)
            pbar.set_description(f"Downloading {filename}")

        path = snapshot_download(
            repo_id=repo_id,
            local_dir=local_dir,
            tqdm_class=None,  # Disable default progress
        )

    return path
```

## Batch Downloads

### Download Multiple Models

```python
from concurrent.futures import ThreadPoolExecutor
import json

def download_model_list(model_list, max_workers=3):
    """Download multiple models in parallel"""
    results = {}

    def download_single(model_id):
        try:
            path = snapshot_download(model_id)
            return model_id, {"status": "success", "path": path}
        except Exception as e:
            return model_id, {"status": "error", "error": str(e)}

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(download_single, model) for model in model_list]
        for future in futures:
            model_id, result = future.result()
            results[model_id] = result

    return results

# Usage
models_to_download = [
    "bert-base-uncased",
    "gpt2",
    "distilbert-base-uncased"
]

results = download_model_list(models_to_download)
print(json.dumps(results, indent=2))
```

## Docker Integration

### Dockerfile with Model Downloads

```dockerfile
FROM python:3.11-slim

# Install dependencies
RUN pip install huggingface_hub transformers

# Download models during build
RUN python -c "from huggingface_hub import snapshot_download; \
    snapshot_download('bert-base-uncased', cache_dir='/models')"

# Set cache directory
ENV HF_HOME=/models

# Your application
COPY app.py /app/
CMD ["python", "/app/app.py"]
```

### Docker Compose with Volume

```yaml
version: '3.8'

services:
  ml-service:
    build: .
    volumes:
      # Share model cache between containers
      - hf-models:/root/.cache/huggingface

  downloader:
    image: python:3.11-slim
    command: |
      bash -c "
      pip install huggingface_hub &&
      python -c 'from huggingface_hub import snapshot_download;
      snapshot_download(\"gpt2\")'
      "
    volumes:
      - hf-models:/root/.cache/huggingface

volumes:
  hf-models:
```

## Best Practices

### 1. Handle Download Errors

```python
from huggingface_hub import snapshot_download
from huggingface_hub.utils import RepositoryNotFoundError, RevisionNotFoundError

def safe_download(repo_id, **kwargs):
    try:
        return snapshot_download(repo_id, **kwargs)
    except RepositoryNotFoundError:
        print(f"Repository {repo_id} not found!")
    except RevisionNotFoundError:
        print(f"Revision not found for {repo_id}")
    except Exception as e:
        print(f"Download failed: {e}")
    return None
```

### 2. Optimize for Production

```python
# Production download script
def production_download(repo_id, verify=True):
    """Download with production considerations"""
    import hashlib

    # Download to staging first
    staging_dir = f"/tmp/staging_{repo_id.replace('/', '_')}"

    path = snapshot_download(
        repo_id=repo_id,
        local_dir=staging_dir,
        local_dir_use_symlinks=False,  # Real files for verification
    )

    if verify:
        # Verify critical files exist
        import os
        required_files = ["config.json", "pytorch_model.bin"]
        for file in required_files:
            if not os.path.exists(os.path.join(path, file)):
                raise ValueError(f"Missing required file: {file}")

    # Move to production location
    import shutil
    prod_dir = f"/models/{repo_id}"
    shutil.move(staging_dir, prod_dir)

    return prod_dir
```

### 3. Monitor Download Progress

```python
import time
from datetime import datetime

def download_with_monitoring(repo_id):
    """Download with detailed monitoring"""
    start_time = time.time()
    print(f"[{datetime.now()}] Starting download of {repo_id}")

    try:
        path = snapshot_download(repo_id)
        duration = time.time() - start_time
        print(f"[{datetime.now()}] Completed in {duration:.2f}s")
        print(f"Downloaded to: {path}")
        return path
    except Exception as e:
        duration = time.time() - start_time
        print(f"[{datetime.now()}] Failed after {duration:.2f}s: {e}")
        raise
```