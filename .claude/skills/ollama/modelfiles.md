# Modelfiles

Modelfiles are Ollama's declarative configuration format for creating and customizing models without modifying weights.

## Basic Syntax

Modelfiles use a Dockerfile-like syntax:

```dockerfile
FROM llama3
PARAMETER temperature 0.7
PARAMETER num_ctx 4096
SYSTEM "You are a helpful assistant specialized in Python programming."
```

## Directives

### FROM (Required)
Specifies the base model or source:

```dockerfile
# From existing Ollama model
FROM llama3

# From local GGUF file
FROM ./models/my-model.gguf

# From local Safetensors directory
FROM ./safetensors_model/

# With specific tag
FROM llama3:70b
```

### PARAMETER
Sets runtime parameters:

```dockerfile
# Temperature (0.0-2.0) - randomness
PARAMETER temperature 0.7

# Context window size
PARAMETER num_ctx 8192

# Max tokens to generate
PARAMETER num_predict 512

# Sampling parameters
PARAMETER top_k 40
PARAMETER top_p 0.9
PARAMETER repeat_penalty 1.1

# Mirostat sampling
PARAMETER mirostat 2
PARAMETER mirostat_eta 0.1
PARAMETER mirostat_tau 5.0

# GPU/CPU settings
PARAMETER num_gpu 35        # GPU layers (-1 for all)
PARAMETER num_thread 8      # CPU threads
PARAMETER f16_kv true       # 16-bit KV cache

# Stop sequences
PARAMETER stop "<|end|>"
PARAMETER stop "Human:"
```

### TEMPLATE
Defines the prompt format using Go template syntax:

```dockerfile
TEMPLATE """
{{- if .System }}
<|im_start|>system
{{ .System }}<|im_end|>
{{- end }}
{{- if .Prompt }}
<|im_start|>user
{{ .Prompt }}<|im_end|>
{{- end }}
<|im_start|>assistant
{{ .Response }}<|im_end|>
"""
```

Advanced template with chat history:
```dockerfile
TEMPLATE """
{{- range .Messages }}
{{- if eq .Role "system" }}
<|im_start|>system
{{ .Content }}<|im_end|>
{{- else if eq .Role "user" }}
<|im_start|>user
{{ .Content }}<|im_end|>
{{- else if eq .Role "assistant" }}
<|im_start|>assistant
{{ .Content }}<|im_end|>
{{- end }}
{{- end }}
<|im_start|>assistant
"""
```

### SYSTEM
Sets the system prompt:

```dockerfile
SYSTEM "You are a helpful coding assistant specialized in Rust. Always provide safe, idiomatic Rust code with proper error handling."
```

Multi-line system prompt:
```dockerfile
SYSTEM """
You are an expert Python developer with deep knowledge of:
- Django and FastAPI frameworks
- Async programming
- Database optimization
- Testing best practices

Always suggest type hints and follow PEP 8 conventions.
"""
```

### MESSAGE
Pre-seeds conversation history:

```dockerfile
# Single exchange
MESSAGE user "What's your specialty?"
MESSAGE assistant "I specialize in Python web development with Django and FastAPI."

# Multiple exchanges
MESSAGE user "Can you help with async code?"
MESSAGE assistant "Absolutely! I'm well-versed in asyncio and async/await patterns."
MESSAGE user "Great, let's start with a FastAPI example."
```

### ADAPTER
Applies LoRA adapters:

```dockerfile
# Single adapter
ADAPTER ./lora-adapter.safetensors

# Multiple adapters (applied in order)
ADAPTER ./coding-adapter.safetensors
ADAPTER ./python-specialty.safetensors
```

### LICENSE
Specifies the license:

```dockerfile
LICENSE """
MIT License

Copyright (c) 2024

Permission is hereby granted...
"""
```

## Creating Custom Models

### Step 1: Create Modelfile
```bash
# Create a specialized coding assistant
cat > CodeAssistant.Modelfile << 'EOF'
FROM llama3

PARAMETER temperature 0.3
PARAMETER num_ctx 8192
PARAMETER repeat_penalty 1.2

SYSTEM """
You are an expert programmer. Follow these principles:
1. Write clean, maintainable code
2. Include error handling
3. Add helpful comments
4. Suggest tests when appropriate
"""

TEMPLATE """
{{- if .System }}System: {{ .System }}
{{- end }}
{{- if .Prompt }}
Human: {{ .Prompt }}
{{- end }}
Assistant: {{ .Response }}
"""

MESSAGE user "What languages do you know best?"
MESSAGE assistant "I'm proficient in Python, JavaScript, Rust, Go, and many others. I can help with any programming language or technology stack."
EOF
```

### Step 2: Create Model
```bash
ollama create code-assistant -f CodeAssistant.Modelfile
```

### Step 3: Test Model
```bash
ollama run code-assistant "Write a Python function to validate email addresses"
```

## Advanced Examples

### Multi-Modal Configuration
```dockerfile
FROM llava
PARAMETER temperature 0.5
PARAMETER num_ctx 4096

SYSTEM "You are a helpful assistant that can analyze images and answer questions about them."

TEMPLATE """
{{- if .System }}{{ .System }}{{ end }}
{{- if .Prompt }}User: {{ .Prompt }}{{ end }}
{{- if .Images }}
Images provided: {{ len .Images }}
{{- end }}
Assistant: {{ .Response }}
"""
```

### Chain-of-Thought Reasoning
```dockerfile
FROM llama3:70b
PARAMETER temperature 0.7
PARAMETER num_ctx 16384

SYSTEM """
You solve problems step-by-step. Always:
1. Restate the problem
2. Break it into steps
3. Solve each step
4. Verify the solution
"""

TEMPLATE """
{{- if .Prompt }}
Problem: {{ .Prompt }}

Let me solve this step-by-step.

{{ .Response }}
{{- end }}
"""
```

### Language-Specific Assistant
```dockerfile
FROM codellama
PARAMETER temperature 0.2
PARAMETER num_predict 2048

SYSTEM "You are a Rust expert. Always use Rust 2021 edition features and follow official style guidelines."

# Rust-specific template
TEMPLATE """
{{- if .Prompt }}
Question: {{ .Prompt }}

```rust
{{ .Response }}
```
{{- end }}
"""

MESSAGE user "Show me an example of error handling"
MESSAGE assistant "Here's idiomatic Rust error handling using Result:

```rust
use std::fs::File;
use std::io::{self, Read};

fn read_file_contents(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

// Usage with pattern matching
match read_file_contents("config.toml") {
    Ok(contents) => println!("File contents: {}", contents),
    Err(e) => eprintln!("Failed to read file: {}", e),
}
```"
```

## Parameter Reference

### Generation Parameters
| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `temperature` | 0.8 | 0.0-2.0 | Randomness (0=deterministic) |
| `num_predict` | 128 | -2 to ∞ | Max tokens (-1=infinite, -2=fill context) |
| `top_k` | 40 | 1-∞ | Limit token selection pool |
| `top_p` | 0.9 | 0.0-1.0 | Nucleus sampling threshold |
| `repeat_penalty` | 1.1 | 0.0-2.0 | Penalize repetition |
| `presence_penalty` | 0.0 | -2.0-2.0 | Penalize already-present tokens |
| `frequency_penalty` | 0.0 | -2.0-2.0 | Penalize frequent tokens |

### System Parameters
| Parameter | Default | Description |
|-----------|---------|-------------|
| `num_ctx` | 2048 | Context window size |
| `num_batch` | 512 | Batch size for prompt eval |
| `num_gpu` | -1 | GPU layers (-1=all, 0=none) |
| `main_gpu` | 0 | GPU to use for compute |
| `num_thread` | auto | CPU threads |

### Advanced Sampling
| Parameter | Default | Description |
|-----------|---------|-------------|
| `mirostat` | 0 | Mirostat version (0, 1, 2) |
| `mirostat_eta` | 0.1 | Mirostat learning rate |
| `mirostat_tau` | 5.0 | Mirostat target entropy |
| `typical_p` | 1.0 | Typical sampling threshold |
| `seed` | 0 | Random seed (0=random) |

## Best Practices

1. **Start from proven base models** - Use well-tested foundations
2. **Tune temperature carefully** - Lower for factual, higher for creative
3. **Set appropriate context** - Balance memory usage and capability
4. **Test incrementally** - Verify each directive's effect
5. **Version your Modelfiles** - Track changes in git
6. **Document parameters** - Explain why specific values were chosen

## Debugging Modelfiles

### View Existing Model Configuration
```bash
ollama show llama3 --modelfile
```

### Test Parameters
```bash
# Create test variants
ollama create test-cold -f - <<< "FROM llama3
PARAMETER temperature 0.1"

ollama create test-hot -f - <<< "FROM llama3
PARAMETER temperature 1.5"

# Compare outputs
echo "Explain quantum physics" | ollama run test-cold
echo "Explain quantum physics" | ollama run test-hot
```

### Common Issues

**Template not working:**
- Check Go template syntax
- Ensure variable names match (.System, .Prompt, .Response)
- Test with simple templates first

**Parameters ignored:**
- Verify parameter names are correct
- Check value ranges
- Some parameters only work with specific models

**Model creation fails:**
- Ensure base model exists (`ollama list`)
- Check file paths are correct
- Verify GGUF files are valid