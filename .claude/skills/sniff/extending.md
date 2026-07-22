# Extending sniff

Adding new detection capabilities.

## Module Structure

```
sniff/lib/src/
├── lib.rs        # Public API: detect(), SniffConfig, DetectionPlan
├── request.rs    # Fine-grained detection control types
├── error.rs      # Error types
├── os/           # OS detection (distro, locale, time, package managers)
├── hardware/     # CPU, GPU, memory, storage, audio devices
├── network/      # Interface enumeration
├── filesystem/   # Git, repo, languages, docs, file types, blast radius, just
├── package/      # Package manager abstraction (110+)
├── programs/     # Program detection (10 categories; 8 installable)
├── remote/       # Remote repo inspection (GitHub, GitLab, Gitea, Bitbucket)
└── services/     # Init system and service detection

sniff/cli/src/
├── main.rs       # Entry point, tracing initialization
├── args.rs       # Clap subcommands and argument parsing
├── commands.rs   # Command execution logic
├── install.rs    # Program installation interface
└── output/       # Text/JSON rendering with per-topic modules
    ├── mod.rs
    ├── filesystem.rs, hardware.rs, network.rs, os.rs
    ├── programs.rs, services.rs, remote.rs, recent_commits.rs
    └── topics.rs, just.rs
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

4. Add field to `ProgramsInfo` in `programs/mod.rs`
5. Add CLI subcommand variant in `cli/src/args.rs`
6. Add output handling in `cli/src/output/programs.rs`

## Adding an Init System

1. Add variant to `InitSystem` enum in `services/mod.rs`
2. Implement detection logic in `detect_init()` / `detect_init_with_evidence()`
3. Implement service listing for the new init system
4. Update `ServiceManager::detect()` if needed

## Adding a CLI Subcommand

1. Add variant to `Commands` enum in `cli/src/args.rs`
2. Add `OutputFilter` variant and mapping in `cli/src/output/mod.rs`
3. Add rendering in the appropriate `cli/src/output/*.rs` module
4. Wire up execution in `cli/src/commands.rs`

## Testing

```bash
cargo test -p sniff                  # All lib tests
cargo test -p sniff-cli              # All CLI tests
cargo test -p sniff programs::       # Program module
cargo test -p sniff services::       # Services module
```

## Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Network permission denied | macOS sandbox | Check `NetworkInfo::permission_denied` flag |
| macOS app not found | Different bundle name | Check both PATH and `/Applications` |
| Service detection fails | Unknown init system | Add evidence tracking for debugging |
