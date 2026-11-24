# Calling Claudine from Rust

There are two crates you might depend on:

| Crate | Path in rusty-biscuit | Use it to |
|---|---|---|
| `claudine` | `claudine/lib` | compose prompt documents, look up providers, list or link shared resources, read config, and parse events |
| `claudine-contract` | `claudine/contract` | **run an agent** for one non-interactive, tool-free, filesystem-isolated turn, through `biscuit_contract::inference::InferenceAdapter` |

**The `claudine` library has no public API that launches an agent.**
Wrapped execution lives in the CLI. To get an answer from an agent in your
own code, use `claudine-contract`. Its v1 supports Claude and Codex; other
providers return `Unsupported`.

```toml
[dependencies]
claudine = { path = "../claudine/lib" }
claudine-contract = { path = "../claudine/contract" }
biscuit-contract = { path = "../biscuit-contract/lib" }
biscuit-file = { path = "../biscuit-file/lib", features = ["file-reference"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Each snippet below compiled against the source of 2026-11-08. The three
synchronous ones also ran in a scratch repository. The contract snippet only
compiled, because running it would start a real Claude session.

## Compose a prompt document

```rust
use claudine::composition::{PrepareOptions, prepare_direct, resolve_composition_source_in_context};

let root = std::env::current_dir()?;
let ctx = biscuit_file::FileResolutionContext::from_snapshot(root, None, Default::default());
let source = resolve_composition_source_in_context("prompts/review.md", &ctx)?; // any file-reference form
let prepared = prepare_direct(&source, PrepareOptions::new(ctx))?;
println!("{}", prepared.prompt);           // the composed body: what the agent would receive
let fm = &prepared.effective_frontmatter;   // resolved frontmatter
```

This runs the same Darkmatter pipeline as `claudine compose`, but launches
nothing and fires no lifecycle event.

- **The environment.** `FileResolutionContext::from_snapshot(dir, home,
  env)` with `None` and an empty map gives **no home and no environment**,
  so `~` references fail and `env.*` is empty. That was the case in the run
  above (`env.AGENT` rendered `unknown`). Pass a home directory and an env
  map when the document needs them.
- **Shell.** Any shell commands the document contains still go through
  approval.
- **More control.** The Darkmatter library offers more (see the
  `use-darkmatter` skill, `library.md`).

## Providers

```rust
use claudine::provider::{Provider, provider_info, all_providers};

let p = Provider::fuzzy_match_cli_name("cod").unwrap();   // exact → prefix → substring
let info = provider_info(p);                               // .display_name, .slug, .binary
for i in all_providers() { println!("{}", i.slug); }       // claude, codex, gemini, goose, kimi, opencode, qwen, kilo, pi, antigravity
```

`Provider` has 10 variants. The same enum is also exported as
`claudine::provider_id::Provider`.

## Shared skills

```rust
use claudine::linking::{ProviderSkillPaths, list_skills};

let report = list_skills(&ProviderSkillPaths::new(), &[])?;   // second arg: name filters
for s in &report.skills { println!("{} {:?} {}", s.name, s.scope, s.skill_md_path.display()); }
```

## Read the configuration

```rust
use claudine::dispatch::loader::load_claudine_config;

let repo_root = std::env::current_dir()?;
let config = load_claudine_config(None, Some(repo_root.as_path()))?; // None: ~/.claudine/config.yaml
for (event, actions) in &config.actions { println!("{event:?}: {} actions", actions.len()); }
```

(This snippet was written from the signatures and has not been compiled.)

- **Load through the loader, not serde.** It validates both files against
  the shipped schema, fills every default (optional ones too, and in place
  of a `null`), converts a legacy `config.json`, and merges the repository
  overrides. `serde_yaml` straight into `ClaudineConfig` skips all of that.
- **The types didn't change.** `HookAction` and `MessengerProviderConfig`
  are the same enums, and serializing one on its own still gives the tagged
  form (`type: bash`, `provider: slack`). Only the `actions` and messenger
  `configurations` fields of the config structs read and write the file's
  keyed form (`- bash: { command: … }`).
- **`RepoOverrideConfig::active_messenger` is `Option<String>`.** `None`
  (missing or `null` in the file) inherits the user's route; there is no
  "messaging off" value.

## Ask an agent (`claudine-contract`)

```rust
use biscuit_contract::inference::*;
use claudine::provider_id::Provider;
use claudine_contract::ClaudineInferenceAdapter;

let adapter = ClaudineInferenceAdapter::new(Provider::Claude).build();   // .with_model("…") optional
let resp = tokio::time::timeout(
    std::time::Duration::from_secs(300),                                 // the adapter has no timeout of its own
    adapter.infer(InferenceRequest {
        prompt: "Summarize the release notes".into(),
        output: InferenceOutput::Prose,
        profile: InferenceProfile::default(),
    }),
).await??;
if let InferenceData::Prose(text) = resp.data { println!("{text}"); }
```

- **What the agent can do.** It runs tool-free in an isolated directory: it
  can't read your repository. Put everything it needs in the prompt.
- **Supported providers.** `claudine_contract::support_matrix()` reports
  which providers are supported.
- **Depending on the contract.** Consumers such as Reaper and Darkmatter
  take `Arc<dyn InferenceAdapter>`, so they never depend on `claudine`
  directly.

## Other modules worth knowing

| Module | What it has |
|---|---|
| `claudine::composition` | preparation, the source resolution used above, sequence models, and `registration` (format 3 host registration inventory and publisher) |
| `claudine::doctor` | non-interactive setup inspection and `--fix` repair (configuration, format 3 registration, permissions handoff) |
| `claudine::config` | reading `~/.claudine/config.yaml` and the repo config |
| `claudine::linking` | skill, command, and agent discovery and linking |
| `claudine::mcp` | the MCP catalog |
| `claudine::events`, `claudine::actions` | the unified hook-event model and hook actions |
| `claudine::protect` | the Protect pattern engine (Allow or Block) |
| `claudine::reporting` | log reporting |

`claudine/lib/src/lib.rs` is the authoritative list of what's exported.
