## CLI

The CLI uses `clap` and `clap_complete` (like all CLI's in this monorepo). 

```toml
clap = { version = "4.5", features = ["derive", "env", "unstable-ext"] }
clap_complete = { version = "4.5", features = ["unstable-dynamic"] }
```

In addition it will use:

- `color-eyre` for error reporting
- `tracing` and `tracing-subscriber` for spans, metrics, debugging

## Library
