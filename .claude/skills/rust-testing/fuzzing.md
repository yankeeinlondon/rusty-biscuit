# Fuzz Testing with cargo-fuzz

Fuzz testing feeds semi-random input to code to find crashes, panics, and security vulnerabilities.

## Setup

Requires nightly Rust:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Switch to nightly (or use rustup override)
rustup default nightly

# Initialize fuzzing in project
cargo fuzz init
```

This creates:

```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    └── fuzz_target_1.rs
```

## Basic Fuzz Target

```rust
// fuzz/fuzz_targets/fuzz_parser.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use my_crate::parse;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes
    if let Ok(s) = std::str::from_utf8(data) {
        // Parser should never panic on any input
        let _ = parse(s);
    }
});
```

## Running Fuzzer

```bash
# Run fuzzer (runs indefinitely)
cargo fuzz run fuzz_parser

# Run with timeout
cargo +nightly fuzz run fuzz_parser -- -max_total_time=300

# Run with specific corpus
cargo fuzz run fuzz_parser corpus/

# List available targets
cargo fuzz list
```

## Structured Input with Arbitrary

Generate structured data instead of raw bytes:

```toml
# fuzz/Cargo.toml
[dependencies]
arbitrary = { version = "1", features = ["derive"] }
```

```rust
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Config {
    name: String,
    value: u32,
    enabled: bool,
}

fuzz_target!(|config: Config| {
    // Fuzzer generates valid Config structs
    let _ = my_crate::process_config(&config);
});
```

## Corpus Management

The fuzzer builds a corpus of interesting inputs:

```bash
# Corpus stored in fuzz/corpus/fuzz_parser/

# Minimize corpus (remove redundant inputs)
cargo fuzz cmin fuzz_parser

# Merge new inputs into corpus
cargo fuzz run fuzz_parser corpus/ -- -merge=1
```

## Reproducing Crashes

When fuzzer finds a crash:

```bash
# Crashes saved in fuzz/artifacts/fuzz_parser/

# Reproduce specific crash
cargo fuzz run fuzz_parser fuzz/artifacts/fuzz_parser/crash-abc123

# Get minimized crash input
cargo fuzz tmin fuzz_parser fuzz/artifacts/fuzz_parser/crash-abc123
```

## Coverage-Guided Fuzzing

cargo-fuzz uses LLVM's libFuzzer which:

- Tracks code coverage
- Prioritizes inputs that explore new paths
- Mutates inputs to increase coverage

View coverage:

```bash
cargo fuzz coverage fuzz_parser
```

## What to Fuzz

**Good candidates:**

- Parsers (JSON, YAML, custom formats)
- Decoders (images, audio, compression)
- Protocol handlers
- Deserializers
- Any code handling untrusted input

**Example targets:**

```rust
// Fuzz a parser
fuzz_target!(|data: &[u8]| {
    let _ = json::parse(data);
});

// Fuzz a decoder
fuzz_target!(|data: &[u8]| {
    let _ = image::load_from_memory(data);
});

// Fuzz with structured input
fuzz_target!(|ops: Vec<Operation>| {
    let mut state = State::new();
    for op in ops {
        state.apply(op);
        assert!(state.is_valid());
    }
});
```

## Writing Effective Fuzz Targets

```rust
fuzz_target!(|data: &[u8]| {
    // 1. Convert bytes to appropriate type
    let Ok(input) = std::str::from_utf8(data) else {
        return;  // Skip invalid UTF-8
    };

    // 2. Call the code under test
    let result = my_crate::parse(input);

    // 3. Optionally verify invariants (but don't check specific values)
    if let Ok(parsed) = result {
        // Verify round-trip works
        let serialized = parsed.to_string();
        let reparsed = my_crate::parse(&serialized).unwrap();
        assert_eq!(parsed, reparsed);
    }
});
```

## Integration with CI

Run fuzzing as part of CI (with time limit):

```yaml
# .github/workflows/fuzz.yml
name: Fuzz
on:
  schedule:
    - cron: '0 0 * * *'  # Daily

jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install nightly
        uses: dtolnay/rust-toolchain@nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Run fuzzer
        run: cargo +nightly fuzz run fuzz_parser -- -max_total_time=600

      - name: Upload crashes
        uses: actions/upload-artifact@v3
        if: failure()
        with:
          name: fuzz-crashes
          path: fuzz/artifacts/
```

## OSS-Fuzz Integration

For open source projects, consider [OSS-Fuzz](https://google.github.io/oss-fuzz/) for continuous fuzzing.

## Related

- [Property Testing](./property-testing.md) - Testing invariants with random input
- [Unit Tests](./unit-tests.md) - Testing specific cases
