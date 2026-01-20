# Storage Architecture

Ollama uses a sophisticated content-addressable storage system for efficient model management.

## Directory Structure

```
.ollama/models/
├── blobs/                    # Content-addressed model components
│   └── sha256-<hash>        # Individual files named by SHA-256 hash
└── manifests/               # Model definitions and metadata
    └── registry.ollama.ai/
        └── <model-name>/
            └── <tag>/       # Version-specific manifests
```

## Storage Locations by OS

### Default Paths

| Operating System | Default Path | Set via |
|-----------------|--------------|---------|
| **macOS** | `~/.ollama/models/` | `OLLAMA_MODELS` |
| **Linux** | `/usr/share/ollama/.ollama/models/` | `OLLAMA_MODELS` |
| **Windows** | `C:\Users\<username>\.ollama\models\` | `OLLAMA_MODELS` |

### Customizing Storage Location

#### macOS/Linux
```bash
# Add to ~/.bashrc or ~/.zshrc
export OLLAMA_MODELS="/path/to/models"

# Verify
echo $OLLAMA_MODELS
ollama list  # Should use new path
```

#### Windows
```powershell
# System Environment Variables
[System.Environment]::SetEnvironmentVariable('OLLAMA_MODELS', 'D:\OllamaModels', 'User')

# Or via GUI: System Properties → Environment Variables
```

#### Linux Systemd Service
```ini
# /etc/systemd/system/ollama.service.d/override.conf
[Service]
Environment="OLLAMA_MODELS=/data/ollama/models"
```

Then reload:
```bash
sudo systemctl daemon-reload
sudo systemctl restart ollama
```

## Blob Storage System

### Content Addressing
Each file is stored with its SHA-256 hash as the filename:
- `sha256-365c0bd3c000a3d9f2e8c4f646e68000d6e23f0e2e5c4d00b3df7e6db8e3e6b2`

### Benefits
- **Deduplication**: Identical files stored once
- **Integrity**: Hash verifies file contents
- **Efficient updates**: Only changed components downloaded

### Example: Inspecting Blobs
```bash
# List all blobs
ls ~/.ollama/models/blobs/

# Check blob size
du -h ~/.ollama/models/blobs/sha256-*

# Verify integrity
sha256sum ~/.ollama/models/blobs/sha256-365c0bd3c000* | cut -d' ' -f1
# Should match the filename after 'sha256-'
```

## Manifest Files

Manifests are JSON files defining model composition:

### Structure
```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.ollama.model.v1+json",
  "config": {
    "model": "sha256-<config-hash>",
    "parameters": {
      "temperature": 0.7,
      "num_ctx": 4096
    }
  },
  "layers": [
    {
      "digest": "sha256-<layer1-hash>",
      "mediaType": "application/vnd.ollama.model.layer.v1+gguf",
      "size": 4150000000
    },
    {
      "digest": "sha256-<layer2-hash>",
      "mediaType": "application/vnd.ollama.model.layer.v1+adapter",
      "size": 1000000
    }
  ]
}
```

### Inspecting Manifests
```bash
# View manifest for a specific model
cat ~/.ollama/models/manifests/registry.ollama.ai/library/llama3/latest

# Pretty print
jq . ~/.ollama/models/manifests/registry.ollama.ai/library/llama3/latest
```

## Offline Deployment

### Exporting Models

1. **On Online Machine**:
```bash
# Pull required models
ollama pull llama3:8b
ollama pull mistral:7b

# Package entire models directory
cd ~/.ollama
tar czf ollama-models.tar.gz models/

# Or specific model only
tar czf llama3-model.tar.gz \
  models/manifests/registry.ollama.ai/library/llama3 \
  $(cat models/manifests/registry.ollama.ai/library/llama3/latest | jq -r '.layers[].digest' | sed 's|sha256:|models/blobs/sha256-|g')
```

2. **Transfer to Offline Machine**:
```bash
# Via USB, network share, etc.
scp ollama-models.tar.gz offline-server:/tmp/
```

3. **On Offline Machine**:
```bash
# Stop Ollama
sudo systemctl stop ollama

# Extract models
cd ~/.ollama
tar xzf /tmp/ollama-models.tar.gz

# Fix permissions if needed
sudo chown -R ollama:ollama models/
chmod -R 755 models/

# Start Ollama
sudo systemctl start ollama

# Verify
ollama list
```

## Storage Optimization

### Disk Usage Analysis
```bash
# Total size
du -sh ~/.ollama/models/

# Size by model
for manifest in ~/.ollama/models/manifests/registry.ollama.ai/library/*/latest; do
  model=$(basename $(dirname $manifest))
  size=$(cat $manifest | jq -r '.layers[].size' | awk '{sum+=$1} END {print sum/1073741824}')
  echo "$model: ${size} GB"
done
```

### Cleanup Strategies

#### Remove Unused Models
```bash
# Remove specific model
ollama rm llama3:7b

# Remove old versions, keep latest
ollama list | grep llama3 | grep -v latest | awk '{print $1}' | xargs -I{} ollama rm {}
```

#### Manual Blob Cleanup
```bash
# Find orphaned blobs (not referenced by any manifest)
# WARNING: Advanced usage, be careful

# List all referenced blobs
find ~/.ollama/models/manifests -name "*" -type f -exec cat {} \; | \
  jq -r '.layers[].digest' | sort | uniq > /tmp/referenced-blobs.txt

# List all actual blobs
ls ~/.ollama/models/blobs/ | sed 's/^/sha256:/' > /tmp/actual-blobs.txt

# Find orphaned blobs
comm -13 /tmp/referenced-blobs.txt /tmp/actual-blobs.txt
```

### Storage Tiering
```bash
# Move large, rarely used models to slower storage
mkdir -p /mnt/slow-storage/ollama-archive

# Move specific blob
mv ~/.ollama/models/blobs/sha256-abc123 /mnt/slow-storage/ollama-archive/

# Create symlink
ln -s /mnt/slow-storage/ollama-archive/sha256-abc123 ~/.ollama/models/blobs/
```

## Configuration File

Ollama supports a configuration file for advanced settings:

```json
// ~/.ollama/config.json (or %APPDATA%\ollama\config.json on Windows)
{
  "models_path": "/custom/path/to/models",
  "keep_alive": -1,              // Keep models loaded (-1 = forever)
  "max_loaded_models": 2,        // Max concurrent models in memory
  "gpu_layers": 35,              // Default GPU layers
  "cpu_threads": 8,              // CPU thread count
  "f16_kv": true,                // Use 16-bit key/value cache
  "low_vram": false,             // Low VRAM mode
  "verbose": true                // Verbose logging
}
```

## Troubleshooting

### Permission Issues
```bash
# Check ownership
ls -la ~/.ollama/models/

# Fix permissions
sudo chown -R $USER:$USER ~/.ollama
chmod -R 755 ~/.ollama
```

### Disk Space Issues
```bash
# Check available space
df -h $(dirname ~/.ollama)

# Find large models
du -sh ~/.ollama/models/blobs/* | sort -hr | head -10

# Emergency cleanup
ollama list  # List all models
ollama rm <model-name>  # Remove unneeded models
```

### Path Not Recognized
```bash
# Verify environment variable
echo $OLLAMA_MODELS

# Check if Ollama is using it
ollama serve 2>&1 | grep -i "model path"

# Force reload (Linux)
sudo systemctl restart ollama
```

### Corrupted Downloads
```bash
# Remove incomplete download
ollama rm <model-name>

# Clear specific blobs
rm ~/.ollama/models/blobs/sha256-<partial-hash>*

# Re-pull model
ollama pull <model-name>
```

## Best Practices

1. **Regular Cleanup**: Remove unused model versions
2. **Monitor Disk Usage**: Set up alerts for low disk space
3. **Backup Important Models**: Archive models for offline use
4. **Use Symlinks**: For models shared across users/systems
5. **Document Custom Paths**: Keep track of `OLLAMA_MODELS` settings
6. **Verify Integrity**: Check SHA-256 hashes after transfers