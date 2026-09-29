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
| `biscuit-hash` | `xx_hash`, `blake3` | `xx_hash` names a policy's identity; `blake3` computes `FileChanged` content fingerprints |
| `biscuit-file` | `yaml` only (`default-features = false`) | Frontmatter parsing and byte-exact YAML location and edits. Its default features pull in PDF crates, which a policy check never needs |

The optional `file-adapter` feature enables `biscuit-file/file-reference`
(which brings `gix`, `walkdir`, `dirs`, and `url`) for the bundled `FileChanged`
file provider. It is off by default.

## CLI (`content-policy-cli`)

| Crate | Why |
| --- | --- |
| `content-policy` with `file-adapter` | The CLI evaluates `FileChanged` rules against files on disk |
| `clap` (`derive`, `wrap_help`) | Argument parsing, as in the repository's other CLIs |
