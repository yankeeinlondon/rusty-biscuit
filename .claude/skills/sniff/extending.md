# Extending sniff

Adding new detection capabilities.

## Module Structure

```
sniff/lib/src/
├── lib.rs        # Public API: detect(), SniffConfig
├── os.rs         # OS detection (distribution, locale, timezone)
├── hardware.rs   # CPU, GPU, memory, storage
├── network.rs    # Interface enumeration
├── filesystem/   # Git, repo, languages
├── package/      # Package manager abstraction
├── programs/     # Installed program detection
└── services/     # Init system and service detection
```

## Adding a Program Category

1. Create enum in `programs/enums.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum MyProgram {
    Tool1,
    Tool2,
}
```

2. Implement `ProgramMetadata` trait:

```rust
impl ProgramMetadata for MyProgram {
    fn display_name(&self) -> &'static str { ... }
    fn binary_name(&self) -> &'static str { ... }
    fn description(&self) -> &'static str { ... }
    fn website(&self) -> Option<&'static str> { ... }
}
```

3. Create detection file `programs/my_category.rs`:

```rust
pub fn detect_my_programs() -> Vec<MyProgram> {
    MyProgram::iter()
        .filter(|p| find_program(p.binary_name()).is_some())
        .collect()
}
```

4. Add to `ProgramsInfo` in `programs/mod.rs`
5. Add CLI flags in `cli/src/main.rs`
6. Add output handling in `cli/src/output.rs`

## Adding an Init System

1. Add variant to `InitSystem` enum in `services/mod.rs`
2. Implement detection logic in `detect_init_system()`
3. Implement `list_services()` for the new init system

## Testing

```bash
cargo test -p sniff-lib              # All lib tests
cargo test -p sniff-cli              # All CLI tests
cargo test -p sniff-lib programs::   # Program module
cargo test -p sniff-lib services::   # Services module
```

## Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Network permission denied | macOS sandbox | Check `NetworkInfo::permission_denied` flag |
| macOS app not found | Different bundle name | Check both PATH and `/Applications` |
| Service detection fails | Unknown init system | Add evidence tracking for debugging |
