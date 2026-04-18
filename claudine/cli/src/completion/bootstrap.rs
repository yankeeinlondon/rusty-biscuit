//! Bootstrap snippet rendering for `claudine completions <shell>`.
//!
//! Phase 4 of the `2026-04-17-file-completion` feature replaces the
//! current static `clap_complete::generate(...)` output with one-liner
//! bootstrap snippets that activate dynamic completion:
//!
//! | Shell        | Output shape                                                        |
//! | ------------ | ------------------------------------------------------------------- |
//! | `bash`       | `source <(COMPLETE=bash claudine)`                                  |
//! | `zsh`        | `source <(COMPLETE=zsh claudine)`                                   |
//! | `fish`       | `COMPLETE=fish claudine | source`                                   |
//! | `powershell` | `$env:COMPLETE="powershell"; claudine | Out-String | ...`           |
//! | `elvish`     | `eval (E:COMPLETE=elvish claudine | slurp)`                         |
//!
//! Phase 1 only reserves the module name. The renderer itself lands in
//! Phase 4 once validators and end-to-end integration tests are in place,
//! so the retirement of the old static generator happens in one atomic
//! rewrite.
