# biscuit-hash CLI Reference (`bh`)

## Installation

```bash
# From repository root
cargo install --path biscuit-hash/cli

# Or using just
just -f biscuit-hash/justfile install
```

## Synopsis

```
bh [OPTIONS] [CONTENT]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `CONTENT` | Content to hash. Use `-` with `--password` to read from stdin |

## Options

| Flag | Short | Description |
|------|-------|-------------|
| `--file <PATH>` | `-f` | Hash contents of a file instead of direct content |
| `--crypto` | `-c` | Use BLAKE3 cryptographic hash (64-char hex output) |
| `--password` | `-p` | Password hashing mode (Argon2id, PHC format output) |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

## Mutual Exclusivity

- `--crypto` and `--password` cannot be used together
- `--file` and positional `CONTENT` cannot be used together

## Output Formats

| Mode | Output Format | Example |
|------|---------------|---------|
| Default (xxHash) | Decimal u64 | `5020219685658847592` |
| `--crypto` (BLAKE3) | 64-char hex | `d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24` |
| `--password` (Argon2id) | PHC format | `$argon2id$v=19$m=19456,t=2,p=1$...` |

## Usage Examples

### Basic Hashing (xxHash)

```bash
# Hash a string
bh "hello world"
# Output: 5020219685658847592

# Hash file contents
bh --file Cargo.toml
bh -f Cargo.toml

# Hash from piped input
echo "content" | bh
cat file.txt | bh
```

### Cryptographic Hashing (BLAKE3)

```bash
# Hash a string with BLAKE3
bh --crypto "hello world"
bh -c "hello world"
# Output: d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24

# Hash file with BLAKE3
bh --file document.pdf --crypto
bh -f document.pdf -c
```

### Password Hashing (Argon2id)

```bash
# Hash a password (visible in shell history - NOT recommended for real passwords)
bh --password "mysecret"
bh -p "mysecret"
# Output: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>

# Password from stdin (RECOMMENDED - not in shell history)
echo "mysecret" | bh --password -
read -s pw && echo "$pw" | bh -p -

# Interactive password entry
bh -p -
<type password, press Enter, then Ctrl+D>
```

## Shell Completions

Enable tab completions by adding one line to your shell config:

### Bash (`~/.bashrc`)

```bash
source <(COMPLETE=bash bh)
```

### Zsh (`~/.zshrc`)

```bash
source <(COMPLETE=zsh bh)
```

### Fish (`~/.config/fish/config.fish`)

```fish
COMPLETE=fish bh | source
```

### PowerShell (`$PROFILE`)

```powershell
Invoke-Expression (& bh _complete powershell)
```

### Elvish

```bash
COMPLETE=elvish bh
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (file not found, invalid input, hashing failure) |

## Error Messages

| Error | Cause |
|-------|-------|
| `Failed to read file '<path>'` | File doesn't exist or isn't readable |
| `Empty input from stdin` | Piped input was empty |
| `No content provided` | No argument, file, or piped input |
| `cannot be used with` | Mutually exclusive flags used together |
| `Error hashing password` | Argon2id failure (rare) |

## Common Patterns

### File Integrity Checking

```bash
# Generate checksums for files
for f in *.rs; do
    echo "$(bh -c -f "$f")  $f"
done > checksums.txt

# Verify later
while read hash file; do
    if [ "$(bh -c -f "$file")" != "$hash" ]; then
        echo "CHANGED: $file"
    fi
done < checksums.txt
```

### Change Detection in Scripts

```bash
# Track content changes
OLD_HASH=$(bh -f config.json)
# ... some operation ...
NEW_HASH=$(bh -f config.json)
if [ "$OLD_HASH" != "$NEW_HASH" ]; then
    echo "Config was modified"
fi
```

### Password Hashing in Scripts

```bash
# Securely hash password without shell history
read -sp "Password: " pw
HASH=$(echo "$pw" | bh -p -)
echo "Hash: $HASH"
unset pw
```
