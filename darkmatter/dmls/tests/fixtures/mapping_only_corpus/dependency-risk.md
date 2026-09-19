---
dir: "{{ctx.repo_root}}/reviews/{{}}"
success:
    message: "A dependency"
---
Evaluate whether this Rust workspace has dependency risk or bloat.

Pay special attention to:

- duplicate versions
- unnecessary default features
- crates that could move from normal deps to dev-deps
- crates that should become optional features
- git/path/native/proc-macro/build-script dependencies
- unsafe-heavy transitive dependencies
- known advisories
- license incompatibilities
- dependencies that overlap in purpose
- dependencies that should be centralized in `[workspace.dependencies]`
- crates with broad transitive fan-out
- places where `rustls`/`native-tls`/`openssl` choices are inconsistent
- places where `serde`, `tokio`, `reqwest`, `syn`, `clap`, `tracing`, etc. features are broader than needed

Produce concrete recommended changes and cite the evidence file for each finding.
