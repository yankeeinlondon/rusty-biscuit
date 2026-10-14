# Lifecycle schema spike (throwaway)

Proof-of-concept for the [Schemas](../spec.md#schemas) section of
`2026-09-21-lifecycle-ergonomics`. Nothing here is production code; the
findings live in [`../spike-results.md`](../spike-results.md).

## Layout

| path | what |
|---|---|
| `gen-action-schema.py` | generator: builds every file under `schemas/` except `claudine.yaml` |
| `schemas/action.yaml` | generated `action` (exactly one verb key) and `bundle` (verbs + `when` + `no_error`) tables: 21 hand verbs, 12 side-effect verbs from `darkmatter/lib/src/effects/catalog.rs`, 110 expression functions from `darkmatter/docs/schemas/expression-functions.yaml` |
| `schemas/action-collapsed.yaml` | variant: hand verbs typed, generated verbs collapsed to `<string>: any` |
| `schemas/lifecycle.yaml` | stack grammar unrolled to depth 5, plus `loop` |
| `schemas/lifecycle-d{1..4}.yaml` | reduced-depth variants (growth curve) |
| `schemas/lifecycle-collapsed*.yaml` | depth 5 and depth 3 over the collapsed action table |
| `schemas/claudine.yaml` | hand-written `kind: trigger-schema`; matches `any:` of the six events, payload imports `stack`/`loop` from `lifecycle.yaml` |
| `fixtures/*.md` | trigger-discovered fixtures (no `$schema:`) |
| `fixtures/variants/*.md` | inline `$schema:` pointers at each depth variant, for `--no-trigger-schemas` timing |

## Reproduce

```sh
REPO=/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement
cd $REPO && cargo build -p darkmatter-cli --release          # binary: target/release/md
python3 claudine/features/2026-09-21-lifecycle-ergonomics/spike/gen-action-schema.py
cd claudine/features/2026-09-21-lifecycle-ergonomics/spike/fixtures
$REPO/target/release/md schema triggers depth1.md            # discovery + shadowing + match
for f in *.md; do $REPO/target/release/md schema validate $f; done
$REPO/target/release/md schema validate --no-trigger-schemas variants/lifecycle-d3.md
```

Resolved-JSON sizes and the in-process (DMLS proxy) timings came from a
scratch crate built outside the repo. Its `main.rs` is kept here as
`schema-harness.rs`; the `Cargo.toml` was:

```toml
[package]
name = "schema-harness"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
darkmatter = { path = "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/darkmatter/lib" }
serde_json = { version = "1", features = ["preserve_order"] }
```

built with `CARGO_TARGET_DIR=$REPO/target cargo build --release` after copying
the repo's `Cargo.lock` next to it. `schema-harness size <lib.yaml> [out.json]`
prints the resolved JSON size; `schema-harness validate <doc.md> <repo-root>
[iters]` times discovery + effective schema + validate. It calls
`darkmatter::markdown::schemas::resolve::resolve_schema` and
`DarkmatterSchemas::with_trigger_discovery(..).effective_for(..).validate(..)`.

Requires PyYAML (`python3 -c 'import yaml'`).
