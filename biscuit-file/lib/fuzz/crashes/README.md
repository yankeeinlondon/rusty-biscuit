# Fuzz crash corpus

Each subdirectory holds **minimized** crash inputs for a single fuzz target,
committed as regression fixtures per D10 of the testing-best-practices spec.

## Layout

```
crashes/
├── pdf_extract/       # crashes found by fuzz_targets/pdf_extract.rs
├── toml_roundtrip/    # crashes found by fuzz_targets/toml_roundtrip.rs
├── yaml_roundtrip/    # crashes found by fuzz_targets/yaml_roundtrip.rs
└── json5_roundtrip/   # crashes found by fuzz_targets/json5_roundtrip.rs
```

## Adding a new fixture

When the nightly fuzz workflow finds a crash:

1. Minimize the input:
   ```bash
   cargo +nightly fuzz tmin <target> <raw-crash-file>
   ```
2. Move the minimized file into the matching `crashes/<target>/` directory
   with a short, descriptive filename (e.g. `oom-on-empty-stream.bin`).
3. Commit the fixture and a fix in the same PR. The fuzz workflow replays
   every file under `crashes/<target>/` before starting new exploration so
   the regression cannot silently return.

## Policy

- Keep fixtures **small** (≤ a few KB). Anything larger should be summarized
  in a unit test and the raw bytes excluded.
- Never commit secrets or PII. PDF/JSON5/YAML/TOML inputs from real user
  files must be scrubbed first.
- One fixture per distinct crash signature; if two inputs trip the same
  panic, keep the smaller one.
