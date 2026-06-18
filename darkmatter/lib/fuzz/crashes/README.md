# Fuzz crash corpus

Each subdirectory holds **minimized** crash inputs for a single fuzz target,
committed as regression fixtures per D10 of the testing-best-practices spec.

## Layout

```
crashes/
└── markdown_parser/   # crashes found by fuzz_targets/markdown_parser.rs
```

## Adding a new fixture

When the nightly fuzz workflow finds a crash:

1. Minimize the input:
   ```bash
   cargo +nightly fuzz tmin <target> <raw-crash-file>
   ```
2. Move the minimized file into `crashes/<target>/` with a short,
   descriptive filename (e.g. `panic-on-unterminated-codeblock.md`).
3. Commit the fixture and a fix in the same PR. The fuzz workflow replays
   every file under `crashes/<target>/` before starting new exploration so
   the regression cannot silently return.

## Policy

- Keep fixtures **small** (≤ a few KB). Larger inputs should be summarized
  in a unit test and the raw bytes excluded.
- Never commit secrets or PII. Markdown inputs derived from real documents
  must be scrubbed first.
- One fixture per distinct crash signature; if two inputs trip the same
  panic, keep the smaller one.
