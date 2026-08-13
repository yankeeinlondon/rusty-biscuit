---    
description: Provides a review of a package area's build and test operations with an eye toward making them more performanct without losing any of the functional footprint.
area: "{{ctx.current_package_area}}"
process: "test-and-build-optimization"
---
## Context

::file ./_senior-reviewer.md

Your responsibility is to perform a review on the "{{area}}" package area and:

1. Look for ways the testing process can be made faster without sacrificing functional coverage
2. Look for ways that the compilation of this package area's compilation could be made faster

### Packages found in the **`{{area}}`** Package Area

{{ raw_markdown(as_unordered_list(ctx.current_packages)) }}

And in this monorepo it may be helpful to understand the general dependency relationships which
exist between this package area's package and the rest of the monorepo:

{{ctx.current_deps}}

## Detection Strategy

### Build Times

Here are some common reasons a Rust's build may become slower:

In a Rust project, build performance usually degrades because more work is being invalidated, more code is being compiled, or compilation is becoming less parallel/cache-friendly.

| Rank | Cause | Why it hurts | Typical signals |
|---:|---|---|---|
| 1 | **Feature creep across dependencies** | Cargo features are additive. One crate enabling `serde`, `regex`, `tokio/full`, `openssl`, `chrono`, etc. can expand the build graph for everyone. | `cargo tree -e features` shows unexpectedly broad features. |
| 2 | **Large dependency graph growth** | Every new crate may bring many transitive crates, proc macros, build scripts, native deps, or feature combinations. | `cargo tree`, `cargo metadata`, or `cargo build -vv` shows many new crates compiling. |
| 3 | **Proc macro expansion cost** | Crates like `syn`, `quote`, `serde_derive`, `thiserror`, `async_trait`, `clap_derive`, `strum_macros`, etc. can be expensive, especially when used heavily. | Build spends time in macro-heavy crates or your own crate after macro expansion. |
| 4 | **Build scripts becoming expensive or frequently rerun** | `build.rs` can run arbitrary code, invoke C compilers, generate code, scan directories, probe system state, or accidentally invalidate on every build. | `cargo build -vv` repeatedly shows `Running build.rs`; `rerun-if-changed` / `rerun-if-env-changed` is missing or too broad. |
| 5 | **Poor incremental compilation reuse** | Touching widely imported modules, changing public APIs, changing feature sets, switching branches, changing environment variables, or using codegen-heavy crates can invalidate large portions of the dependency graph. | Small edits trigger many recompiles; `target/debug/incremental` churns heavily. |
| 6 | **Monomorphization explosion from generics** | Deep generic abstractions, many concrete type instantiations, large iterator chains, generic async functions, and type-level programming can create lots of codegen work. | Long compile times in your crate, large binaries, lots of LLVM/codegen time. |
| 7 | **Async state-machine and trait abstraction complexity** | `async fn`, especially generic async code, nested futures, `async_trait`, and tower-style middleware can generate large hidden types. | Type-checking and codegen slow down in async-heavy modules. |
| 8 | **Debug info and codegen settings** | High `debuginfo`, many codegen units, LTO, `panic=abort` changes, optimization levels, or profile overrides can significantly affect build time. | `Cargo.toml` profile changes correlate with slowdown; release builds become much slower. |
| 9 | **Native dependencies and linking cost** | `openssl-sys`, `ring`, `sqlite`, `zstd`, `rocksdb`, `bindgen`, C/C++ builds, and large static links can dominate compile/link time. | Build pauses in `cc`, `cmake`, `bindgen`, linker, or `-sys` crates. |
| 10 | **Cargo workspace / target layout issues** | Large workspaces, many binaries/examples/tests/benches, broad default members, duplicated crate versions, or unnecessary dev-dependencies can expand what gets built. | `cargo test --workspace` or IDE checks compile far more packages than expected. |

#### Especially common sneaky causes

```toml
# Expensive in dev if applied too broadly
[profile.dev]
debug = 2

# Expensive if accidentally enabled for local builds
[profile.release]
lto = true
codegen-units = 1

# Can be expensive if many crates inherit lots of features
[dependencies.tokio]
version = "..."
features = ["full"]
```

#### Useful first checks

```sh
cargo tree
cargo tree -e features
cargo tree --duplicates
cargo build -vv
cargo check --timings
cargo build --timings
```

#### For a monorepo

Start with:

```sh
cargo tree --workspace --duplicates
cargo tree --workspace -e features
cargo check --workspace --timings
```

Then inspect whether the slowdown is dominated by dependency compilation, your own crate type-checking, build scripts, proc macros, or linking.

## Useful references

- [The Cargo Book: Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [The Cargo Book: Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [The Cargo Book: Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [The Cargo Book: Timings](https://doc.rust-lang.org/cargo/reference/timings.html)
- [The Cargo Book: Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rustc Dev Guide: Incremental Compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
