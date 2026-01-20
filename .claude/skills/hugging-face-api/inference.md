# Running Inference

Complete guide to running model inference using Hugging Face's various API offerings.

## Serverless Inference API

### Basic Text Generation

```python
import requests
from huggingface_hub import InferenceClient

# Method 1: Using requests directly
API_URL = "https://api-inference.huggingface.co/models/gpt2"
headers = {"Authorization": f"Bearer {token}"}

def query(payload):
    response = requests.post(API_URL, headers=headers, json=payload)
    return response.json()

# Generate text
result = query({
    "inputs": "The future of AI is",
    "parameters": {
        "max_new_tokens": 50,
        "temperature": 0.7,
        "top_p": 0.95,
        "do_sample": True
    }
})
print(result[0]['generated_text'])
```

### Using InferenceClient

```python
# Method 2: Using the official client (recommended)
from huggingface_hub import InferenceClient

client = InferenceClient(token=token)

# Text generation
response = client.text_generation(
    "The meaning of life is",
    model="gpt2",
    max_new_tokens=100,
    temperature=0.7,
    return_full_text=True
)
print(response)
```

## Supported Tasks

### Text Generation

```python
# Advanced parameters
response = client.text_generation(
    "Write a poem about Python programming",
    model="meta-llama/Llama-2-7b-chat-hf",
    max_new_tokens=200,
    temperature=0.8,
    top_p=0.9,
    top_k=50,
    repetition_penalty=1.2,
    do_sample=True,
    seed=42  # For reproducibility
)
```

### Image Classification

```python
from PIL import Image
import requests

# Load image
image_url = "https://huggingface.co/datasets/huggingface/documentation-images/resolve/main/cats.jpg"
image = Image.open(requests.get(image_url, stream=True).raw)

# Classify
results = client.image_classification(
    image,
    model="google/vit-base-patch16-224"
)

for result in results[:5]:
    print(f"{result['label']}: {result['score']:.2%}")
```

### Text Classification

```python
# Sentiment analysis
result = client.text_classification(
    "I love using Hugging Face models!",
    model="distilbert-base-uncased-finetuned-sst-2-english"
)
print(f"Sentiment: {result[0]['label']} ({result[0]['score']:.2%})")
```

### Token Classification (NER)

```python
# Named Entity Recognition
entities = client.token_classification(
    "My name is John and I work at Microsoft in Seattle.",
    model="dslim/bert-base-NER"
)

for entity in entities:
    print(f"{entity['word']}: {entity['entity_group']} ({entity['score']:.2f})")
```

### Zero-Shot Classification

```python
# Classify without training
result = client.zero_shot_classification(
    "This is a great movie with excellent acting and cinematography",
    candidate_labels=["positive", "negative", "neutral"],
    model="facebook/bart-large-mnli"
)

for label, score in zip(result['labels'], result['scores']):
    print(f"{label}: {score:.2%}")
```

## Streaming Responses

### Stream Text Generation

```python
# For long generations, use streaming
stream = client.text_generation(
    "Write a comprehensive guide to machine learning",
    model="gpt2",
    max_new_tokens=500,
    stream=True
)

print("Streaming response:")
for chunk in stream:
    print(chunk, end="", flush=True)
```

### Async Streaming

```python
import asyncio
from huggingface_hub import AsyncInferenceClient

async def stream_generation():
    async_client = AsyncInferenceClient(token=token)

    stream = await async_client.text_generation(
        "Explain quantum computing",
        model="gpt2",
        max_new_tokens=200,
        stream=True
    )

    async for chunk in stream:
        print(chunk, end="", flush=True)

# Run async
asyncio.run(stream_generation())
```

## Batch Processing

### Batch Text Generation

```python
# Process multiple inputs efficiently
inputs = [
    "The capital of France is",
    "The speed of light is",
    "The largest planet is"
]

# Using direct API
payload = {
    "inputs": inputs,
    "parameters": {
        "max_new_tokens": 20
    }
}

response = requests.post(API_URL, headers=headers, json=payload)
results = response.json()

for i, result in enumerate(results):
    print(f"Input {i+1}: {result['generated_text']}")
```

### Parallel Processing

```python
from concurrent.futures import ThreadPoolExecutor
import time

def process_single(text):
    return client.text_generation(
        text,
        model="gpt2",
        max_new_tokens=50
    )

# Process in parallel
texts = ["Tell me about " + topic for topic in ["AI", "space", "oceans", "history"]]

start = time.time()
with ThreadPoolExecutor(max_workers=4) as executor:
    results = list(executor.map(process_single, texts))

print(f"Processed {len(texts)} requests in {time.time()-start:.2f}s")
```

## Custom Endpoints

### Dedicated Inference Endpoints

```python
# For production workloads with dedicated infrastructure
from openai import OpenAI

# Initialize with custom endpoint
client = OpenAI(
    base_url="https://your-endpoint.us-east-1.aws.endpoints.huggingface.cloud/v1/",
    api_key=token
)

# Use OpenAI-compatible interface
response = client.chat.completions.create(
    model="tgi",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Explain machine learning in simple terms"}
    ],
    temperature=0.7,
    max_tokens=200
)

print(response.choices[0].message.content)
```

### Streaming with Custom Endpoints

```python
# Stream from dedicated endpoint
stream = client.chat.completions.create(
    model="tgi",
    messages=[{"role": "user", "content": "Write a story"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

## Error Handling

### Rate Limiting

```python
import time
from requests.exceptions import HTTPError

def robust_inference(payload, max_retries=3):
    """Handle rate limits with exponential backoff"""
    for attempt in range(max_retries):
        try:
            response = requests.post(API_URL, headers=headers, json=payload)
            response.raise_for_status()
            return response.json()

        except HTTPError as e:
            if e.response.status_code == 429:  # Rate limited
                retry_after = int(e.response.headers.get('Retry-After', 2 ** attempt))
                print(f"Rate limited. Waiting {retry_after}s...")
                time.sleep(retry_after)
            else:
                raise

    raise Exception("Max retries exceeded")
```

### Model Loading

```python
# Handle model loading delays
def query_with_wait(payload, wait_for_model=True):
    """Query with automatic model loading wait"""
    payload_with_options = {
        **payload,
        "options": {
            "wait_for_model": wait_for_model,
            "use_cache": True
        }
    }

    response = requests.post(API_URL, headers=headers, json=payload_with_options)

    if response.status_code == 503:  # Model loading
        estimated_time = response.json().get('estimated_time', 20)
        print(f"Model loading, estimated time: {estimated_time}s")
        time.sleep(estimated_time)
        # Retry
        return query_with_wait(payload, wait_for_model)

    return response.json()
```

## Performance Optimization

### Caching Responses

```python
from functools import lru_cache
import hashlib
import json

@lru_cache(maxsize=1000)
def cached_inference(prompt, model, max_tokens):
    """Cache inference results for identical inputs"""
    # Create cache key
    cache_key = hashlib.md5(
        f"{prompt}{model}{max_tokens}".encode()
    ).hexdigest()

    result = client.text_generation(
        prompt,
        model=model,
        max_new_tokens=max_tokens
    )
    return result

# Repeated calls use cache
result1 = cached_inference("Hello", "gpt2", 50)
result2 = cached_inference("Hello", "gpt2", 50)  # From cache
```

### Connection Pooling

```python
import requests
from requests.adapters import HTTPAdapter
from requests.packages.urllib3.util.retry import Retry

# Create session with connection pooling
session = requests.Session()

# Configure retry strategy
retry_strategy = Retry(
    total=3,
    backoff_factor=1,
    status_forcelist=[429, 500, 502, 503, 504]
)

adapter = HTTPAdapter(
    max_retries=retry_strategy,
    pool_connections=10,
    pool_maxsize=10
)

session.mount("https://", adapter)

# Use session for all requests
def optimized_query(payload):
    return session.post(API_URL, headers=headers, json=payload).json()
```

## Monitoring & Logging

```python
import logging
from datetime import datetime

# Setup logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def monitored_inference(prompt, model="gpt2"):
    """Inference with monitoring"""
    start_time = datetime.now()

    try:
        result = client.text_generation(
            prompt,
            model=model,
            max_new_tokens=50
        )

        # Log success
        duration = (datetime.now() - start_time).total_seconds()
        logger.info(f"Inference successful: model={model}, duration={duration:.2f}s")

        return result

    except Exception as e:
        # Log error
        logger.error(f"Inference failed: model={model}, error={str(e)}")
        raise
```

## Best Practices Summary

1. **Use InferenceClient** over raw requests for better error handling
2. **Implement retry logic** for production applications
3. **Cache responses** when appropriate to reduce API calls
4. **Use streaming** for long generations to improve UX
5. **Monitor usage** to stay within rate limits
6. **Use dedicated endpoints** for production workloads
7. **Handle model loading** delays gracefully
8. **Batch requests** when processing multiple inputs