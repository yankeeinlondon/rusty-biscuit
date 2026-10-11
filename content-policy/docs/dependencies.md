# content-policy Dependencies

The library is read by tools that evaluate many documents, and by Darkmatter as
a later consumer, so its graph stays small and must never reach Darkmatter.
`just deps-check` (run by `just lint`) enforces two rules for the default
features and for `--all-features`:

- no PDF crate (`pdf-extract`, `lopdf`, `pdfium-render`, or anything else with
  `pdf` in its name)
- no `darkmatter*` crate

## Library (`content-policy`)

| Crate | Features | Why |
| --- | --- | --- |
| `serde`, `serde_json` | `derive` | The normalized policy and the evaluation report serialize to JSON |
| `chrono` | `std`, `clock`, `now` only | Calendar dates, UTC evaluation instants, and month arithmetic |
| `biscuit-hash` | `xx_hash`, `blake3` | `xx_hash` names a policy's identity and a renewal plan's fingerprint; `blake3` computes `FileChanged` content fingerprints |
| `biscuit-file` | `yaml` only (`default-features = false`) | Frontmatter parsing and byte-exact YAML location and edits. Its default features pull in PDF crates, which a policy check never needs |

The optional `file-adapter` feature enables `biscuit-file/file-reference`
(which brings `gix`, `walkdir`, `dirs`, and `url`) for the bundled `FileChanged`
file provider. It is off by default.

Renewal writes a document atomically (a sibling temporary file renamed over
it) with the standard library alone, so it adds no dependency.

Development only, never shipped and not seen by `just deps-check`:

| Crate | Why |
| --- | --- |
| `tempfile` | Temporary directories for the renewal and file-adapter tests that write files |
| `gix` (`=0.84.0`, `sha1` only) | Creates fixture repositories for the file-adapter tests; pinned and featured as Biscuit File's own, so it adds no crate to the build |

## CLI (`content-policy-cli`)

| Crate | Why |
| --- | --- |
| `content-policy` with `file-adapter` | The CLI evaluates `FileChanged` rules against files on disk |
| `clap` (`derive`, `env`, `wrap_help`) | Argument parsing, as in the repository's other CLIs; `env` gives each setting its `CONTENT_POLICY_*` fallback |
| `clap_complete` (`unstable-dynamic`) | Dynamic shell completions (`COMPLETE=<shell> policy`), the repository's CLI convention |
| `biscuit-terminal` | The styled report and renewal preview: a `Prose` summary line and a `Table` of entries |
| `chrono` (`std`, `clock`, `now`) | `--at`, `--on`, and today's UTC date |

Development only:

| Crate | Why |
| --- | --- |
| `biscuit-test-harness` | `bin_exe!` locates the `policy` binary when tests run from a CI archive |
| `serde_json` | The tests parse `--json` output |
| `tempfile` | Each test runs `policy` in its own temporary directory |
