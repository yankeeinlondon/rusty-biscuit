# Authentication & Tokens

Complete guide to authenticating with Hugging Face APIs, managing tokens, and setting up clients.

## API Token Management

### Creating Tokens

1. Visit [huggingface.co/settings/tokens](https://huggingface.co/settings/tokens)
2. Click "New token"
3. Choose token type:
   - **Fine-grained**: Recommended for production (specific permissions)
   - **Read**: Download models, access metadata
   - **Write**: Upload models, create repositories

### Token Best Practices

```python
from huggingface_hub import get_token, login

# Method 1: Interactive login (stores token)
login()  # Prompts for credentials

# Method 2: Programmatic (CI/CD)
import os
os.environ["HF_TOKEN"] = "hf_..."

# Method 3: Get stored token
token = get_token()
print(f"Current token: {token[:10]}...")  # Show only prefix
```

> 💡 **Security**: Use fine-grained tokens with minimal required permissions. Never commit tokens to version control.

## Client Initialization

### Python Clients

```python
from huggingface_hub import HfApi, InferenceClient

# Hub API client (model/dataset operations)
api = HfApi(token=token)

# Inference client (run models)
inference_client = InferenceClient(token=token)

# Provider-specific client
provider_client = InferenceClient(
    provider="hf-inference",  # or "together", "groq", etc.
    api_key=token
)
```

### Environment Variables

```bash
# Token for authentication
export HF_TOKEN="hf_..."

# Custom cache directory
export HF_HOME="/path/to/cache"

# Disable progress bars
export HF_HUB_DISABLE_PROGRESS_BARS=1

# Offline mode (cache only)
export HF_HUB_OFFLINE=1

# Disable telemetry
export HF_HUB_DISABLE_TELEMETRY=1
```

## Advanced Authentication

### Using Tokens in Requests

```python
import requests

# Direct API calls
API_URL = "https://api-inference.huggingface.co/models/gpt2"
headers = {"Authorization": f"Bearer {token}"}

response = requests.post(API_URL, headers=headers, json=payload)
```

### Organization Access

```python
# List organization repositories
api = HfApi(token=token)
org_repos = api.list_models(author="my-organization")

# Create repo in organization
api.create_repo(
    repo_id="my-org/my-model",
    repo_type="model",
    private=True,
    organization="my-org"
)
```

## Token Scopes & Permissions

### Fine-grained Token Example

```python
# Token with specific repo access
# Created via web UI with:
# - Read access to: username/specific-model
# - Inference access: enabled

from huggingface_hub import InferenceClient

# This token can only:
# 1. Download the specific model
# 2. Run inference on it
# 3. Cannot modify or access other repos
client = InferenceClient(token="hf_fineGrained...")
```

### Checking Token Permissions

```python
from huggingface_hub import whoami

# Get token info
info = whoami(token)
print(f"Username: {info['name']}")
print(f"Organizations: {info.get('orgs', [])}")
print(f"Token type: {info.get('type', 'unknown')}")
```

## Troubleshooting

### Common Authentication Issues

**403 Forbidden**
```python
# Verify token has required permissions
try:
    api.model_info("private-model")
except Exception as e:
    if "403" in str(e):
        print("Token lacks access to this resource")
```

**Token Validation**
```python
from huggingface_hub import HfApi

def validate_token(token):
    try:
        api = HfApi(token=token)
        api.whoami()
        return True
    except Exception:
        return False

# Check if token is valid
if validate_token(token):
    print("Token is valid!")
```

**Proxy Configuration**
```python
# For corporate environments
import os

os.environ["HTTP_PROXY"] = "http://proxy.company.com:8080"
os.environ["HTTPS_PROXY"] = "http://proxy.company.com:8080"

# Or configure in client
from huggingface_hub import configure_http_backend
import requests

session = requests.Session()
session.proxies = {
    "http": "http://proxy.company.com:8080",
    "https": "http://proxy.company.com:8080"
}
configure_http_backend(session)
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/hf-api.yml
name: HF API Workflow

on: [push]

jobs:
  use-hf-api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          pip install huggingface_hub

      - name: Use HF API
        env:
          HF_TOKEN: ${{ secrets.HF_TOKEN }}
        run: |
          python -c "
          from huggingface_hub import HfApi
          api = HfApi()
          models = api.list_models(limit=5)
          print(f'Found {len(models)} models')
          "
```

### Docker

```dockerfile
FROM python:3.11-slim

# Install HF Hub
RUN pip install huggingface_hub

# Pass token as build arg (not recommended)
# ARG HF_TOKEN
# ENV HF_TOKEN=$HF_TOKEN

# Better: Pass at runtime
# docker run -e HF_TOKEN=$HF_TOKEN myimage
```

## Best Practices Summary

1. **Development**: Use interactive login for convenience
2. **Production**: Use environment variables with fine-grained tokens
3. **CI/CD**: Store tokens as secrets, never in code
4. **Security**: Rotate tokens regularly, use minimal permissions
5. **Debugging**: Enable logging to diagnose authentication issues