# `biscuit-hash` Library & CLI

<table>
<tr>
<td><img src="../assets/biscuit-hash.png" style="max-width='25%'" width=200px /></td>
<td>
<p>This library provides a useful trifecta of of hash algorithms:</p>

1. `blake3` for cryptographic hashes
2. `argon2id` for password hashes
3. `xx_hash` for blazingly fast non-cryptographic hashes

<p>
Each of these algorithms is "best in class" for their purpose in 2026+.
</p>
</td>
</tr>
</table>

## CLI Installation

```sh
# From the repository root
cargo install --path biscuit-hash/cli

# Or using just
just -f biscuit-hash/justfile install
```

## CLI Usage

```sh
# Hash content with xxHash (default, fast non-cryptographic)
hash "hello world"
# => 5020219685658847592

# Hash file contents
hash --file path/to/file.txt

# Use BLAKE3 cryptographic hash
hash --crypto "hello world"
# => d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24

# Hash a password with Argon2id (produces PHC format)
hash --password "mysecret"
# => $argon2id$v=19$m=19456,t=2,p=1$...

# Password from stdin (more secure - not in shell history)
echo "mysecret" | hash --password -
```

### CLI Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--file <PATH>` | `-f` | Hash contents of a file |
| `--crypto` | `-c` | Use BLAKE3 cryptographic hash |
| `--password` | `-p` | Password hashing mode (Argon2id) |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

### Shell Completions

Enable tab completions by adding one line to your shell config:

**Bash** (`~/.bashrc`):
```sh
source <(COMPLETE=bash hash)
```

**Zsh** (`~/.zshrc`):
```sh
source <(COMPLETE=zsh hash)
```

**Fish** (`~/.config/fish/config.fish`):
```sh
COMPLETE=fish hash | source
```

## Library Usage

### Feature Flags

By default only the `xx_hash` algorithm is included but `blake3` and `argon2id` can be enabled where needed:

```sh
cargo add biscuit-hash -F blake3,argon2id
```

### Basic Hashing

```rust
use biscuit_hash::xx_hash;

let hash = xx_hash("hello world");
```

### Cryptographic Hashing (BLAKE3)

```rust
use biscuit_hash::blake3_hash;

let hash = blake3_hash("hello world");
// Returns 64-character hex string
```

### Password Hashing (Argon2id)

```rust
use biscuit_hash::{hash_password, verify_password};

let hash = hash_password("secret").unwrap();
assert!(verify_password("secret", &hash).unwrap());
```

## Semantic Hash Variants

We provide a basic `xx_hash()` function to hash any content but it is often more useful to "prepare" text content a little before hashing to avoid "false positive" in change detection. This is particularly true in whitespace insensitive grammars like Markdown or HTML.

The `HashVariant` enumeration provides the following options:

- `BlockTrimming` - trims the content block's leading and trailing whitespace
- `BlankLine` - all blank lines are removed
- `LeadingWhitespace` - all whitespace at the _beginning of each line_ is removed
- `TrailingWhitespace` - all whitespace at the _end of each line_ is removed
- `InteriorWhitespace` - all _extra_ space (aka, after the first whitespace character) _on each line_ is removed.
- `ReplacementMap(map)`
    - The replacement map variant requires that you provide a `HashMap<String,String>` as configuration
    - The hashmap provided represents a text replacement strategy where the _keys_ are the text we'll look for, and the _values_ are the text we'll replace it with
- `DropChars(chars)`
    - Removes all occurrences of the specified characters from the content before hashing
    - Useful for ignoring specific punctuation or symbols

### Using the `HashVariant` to Hash with xxHash

The `xx_hash(content)` always just hashes exactly what you give it but if you want to pre-process the content with one or more of the variants provided above you will use the `xx_hash_variant(content, [v1,v2,v3])` which takes one or more variants as input.

```rust
use biscuit_hash::{xx_hash_variant, HashVariant};

// Ignore leading/trailing whitespace and blank lines
let hash = xx_hash_variant(
    "  hello  \n\n  world  ",
    vec![
        HashVariant::LeadingWhitespace,
        HashVariant::TrailingWhitespace,
        HashVariant::BlankLine,
    ],
);
```
