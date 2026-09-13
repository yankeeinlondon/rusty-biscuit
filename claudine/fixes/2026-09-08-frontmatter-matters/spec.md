---
status: implemented
created: 2026-09-08
updated: 2026-09-08
area: claudine
implemented: true
packages:
    - claudine
    - claudine-cli
related:
    - claudine/features/_completed/2026-07-02-provider-metadata/spec.md
    - darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md
---

# Frontmatter matters: a document's `model:` must reach the provider launch

## Summary

A composition document declares its launch identity in frontmatter. Its
`model:` is resolved, shown in the `--dry-run` table, and exported as `MODEL`
to the composed prompt, and is then **silently discarded** before argv
assembly whenever the value is not in Claudine's compiled model-catalog
baseline for the provider. For OpenCode that baseline is the 50 `opencode/*`
aggregator ids, so every user-configured OpenCode provider (`minimax/…`,
`zai-coding-plan/…`, `kimi-for-coding/…`, a local vLLM entry) is rejected with
no warning. The provider then runs on its own default, and Claudine reports a
*third* model in `MODEL` that it read from a legacy config file OpenCode no
longer uses.

The fix has three parts, and one design rule behind all of them: **one model
path for every provider**, with provider differences expressed as catalog data
or one small profile hook, never as a provider-keyed branch.

1. The library resolution chain forwards the document's `model:`; the catalog
   becomes a warning source, not a gate.
2. The CLI resolves the model through one helper on every route (compose,
   inline-compose, dry-run, sequence review, sequence dry-run, retry/resume
   rebuild) instead of seven hand-rolled call sites.
3. The shared prep stage that maps the resolved model onto argv is the same
   for every provider. The OpenCode-only stage (`apply_opencode_model_resolution`
   and its config snapshot) is deleted; "this provider must have a model in
   non-interactive mode" stays catalog data, and "this is how the provider
   spells its own config file" becomes a `WrapperProfile` hook that only
   OpenCode implements.

Frontmatter `yolo:` was raised in the same report. It is deliberately **not**
part of this fix: Ken is folding it into a broader permissions spec. For the
record, no code path reads a frontmatter `yolo` key today; the only YOLO
intent signals are `--yolo` / `-y` and `CLAUDINE_YOLO`.

## Observed behavior (2026-09-08)

Ken ran the following in a repository on this host, where `compose` is an
alias for `claudine compose` and `~/.claudine/prompts/commit.md` carries
`agent: opencode` and `model: minimax/MiniMax-M3`:

```text
$ git add . && compose ~/.claudine/prompts/commit.md

Claudine ▸ OpenCode  Compose  prompt sourced from /Users/ken/.claudine/prompts/commit.md
…
 llm_call_start zai-coding-plan/glm-5.2 (mode=primary, agent=build)
┃ API Error
┃ Usage limit reached for glm-5.2 (zai-coding-plan); resets at 2026-09-08 23:30:46
```

Provider selection is right (OpenCode). The model is wrong: OpenCode's own
default from `~/.config/opencode/opencode.jsonc`, not the document's
`minimax/MiniMax-M3`.

### Reproduction with a stub provider

A stub `opencode` placed first on `PATH` that records its argv and environment
reproduces the symptom against both the installed binary (built 2026-09-08
07:09 from `/Volumes/coding/personal/rusty-biscuit`) and a fresh
`cargo build -p claudine-cli` of this worktree at `973d1d918`:

| Document frontmatter | Child argv contains | Child `MODEL` env |
|---|---|---|
| `model: minimax/MiniMax-M3` | no `--model` | `minimax/MiniMax-M2.7-highspeed` |
| `model: minimax/minimax-m3` | no `--model` | `minimax/MiniMax-M2.7-highspeed` |
| `model: zai-coding-plan/glm-5.2` | no `--model` | `minimax/MiniMax-M2.7-highspeed` |
| `model: opencode/gpt-5.2-codex` | `--model opencode/gpt-5.2-codex` | `opencode/gpt-5.2-codex` |
| CLI `--model minimax/MiniMax-M3` | `--model minimax/MiniMax-M3` | `minimax/MiniMax-M3` |

`opencode models` on the same host lists `minimax/MiniMax-M3` and
`zai-coding-plan/glm-5.2`. `minimax/MiniMax-M2.7-highspeed` is the `model` key
of `~/.config/opencode/config.json`, a legacy filename OpenCode no longer
reads as its global config (that is `opencode.json` / `opencode.jsonc`, which
on this host names `zai-coding-plan/glm-5.2`).

`claudine compose ~/.claudine/prompts/commit.md --dry-run` renders
`Model │ minimax/MiniMax-M3` on both binaries.

## Root causes

### R1. Frontmatter `model:` is gated by a baseline that cannot contain user-configured models

`resolve_model_with_env` (`claudine/lib/src/composition/select.rs`) resolves
`--model` → provider env var → `MODEL` → frontmatter → provider default. Step 4
accepts a single frontmatter hint only when
`ModelCatalogService::is_valid(provider, model)` holds: membership in the
compiled `expected_offerings` baseline plus user overrides, or an
`offering_sources` namespace prefix (`llamacpp/`, `lmstudio/`, … for
OpenCode). An invalid single hint **falls through to `ProviderDefault` with no
diagnostic**; a list hint with no valid member does the same.

That gate was tightened by the provider-metadata workstream's Phase F "staged
demotion" (2026-07-06, recorded in
`claudine/features/_completed/2026-07-02-provider-metadata/implementation-plan.md`):
the `opencode models` listing cache "no longer feeds validation" and only
feeds the drift channel. The plan calls OpenCode "the one real demotion".
Before it, a cold-cache run blocked on `opencode models` and a user-configured
provider id validated; after it, nothing a user adds through `opencode.jsonc`'s
`provider` map can ever be in the baseline. `zai-coding-plan/glm-5.2`, the
model OpenCode actually ran, is itself "invalid" to Claudine.

The CLI resolution routes call `resolve_model_with_hints` directly and bypass
the library's `resolve_target_non_tty_with_env`, the only place that errors
when OpenCode ends up with no model, so the drop is silent on every shipped
route.

### R2. The OpenCode-only model stage then reports a default OpenCode does not use

With `target.model == None`, `exec_prep::resolve_model_and_validate` runs
`apply_opencode_model_resolution` (`wrap/profile/resolve.rs`): `--model` →
`OPENCODE_MODEL` → the `model` key of `~/.config/opencode/config.json` →
error. `config.json` is OpenCode's legacy filename; the current global file
is `opencode.json` / `opencode.jsonc`
(`claudine/docs/research/agent-models/opencode.md` § precedence). The
`ConfigDefault` arm sets `MODEL` in the child environment and pushes no
`--model`, so the value is advisory and, on this host, wrong. The dispatch
reporter, the session log, and `env.MODEL` inside the prompt all carry it.

This stage is also the kind of provider-keyed fork the codebase is trying to
retire: the shared prep stage is "provider-neutral" in name, then immediately
special-cases OpenCode with its own snapshot type, its own env lookup, and its
own error.

### R3. Dry-run resolves without the catalog, so it cannot show the drop

`eagerly_resolve_target` passes `catalog_ref = None` under `--dry-run`
(hermetic dry-run: no listing refresh). The catalog *baseline* is compiled in
and needs no refresh, so the dry-run could resolve exactly as the live run
does. Today it renders the frontmatter value verbatim and disagrees with the
launch.

### R4. Side findings recorded, not fixed here

`~/.claudine/prompts/commit.md` also references `{{ctx.staged_files_list}}`
and `{{ctx.staged_packages_list}}`, which Darkmatter's single-sourcing work
removed on 2026-07-08 (the catalog test `removed_list_twins_are_absent` pins
it). The surviving keys are `ctx.staged_files` and `ctx.staged_packages`. The
body of a document gets a did-you-mean warning for such a reference, but the
same reference inside a deferred lifecycle value (`initialize.stderr`) gets
none and renders empty. Separately, `ctx.repo` renders empty from the
launch-anchored evidence projection while `claudine context --values` fills
it, and a document naming a `Repo`-group key without any `Git`-group key
trips `Partial runtime capture for git`, because
`invocation_context::project_evidence` supplies git identity only for the
`Git` group while Darkmatter's `from_evidence` demands it for every repository
group. See [Non-goals](#non-goals).

## Design

### D1. The library chain forwards the document's model; the catalog warns

The provider is the authority on which model ids it accepts. Claudine's
catalog is a *drift* signal (the Phase F ruling), so a catalog miss must not
change what launches.

- A single frontmatter `model` value is the resolved model whenever nothing
  higher in the chain (`--model`, provider env var, `MODEL`) is set, with
  `ModelResolutionReason::FrontmatterSingle`, whether or not the catalog
  recognizes it.
- A list-valued `model` resolves to the first catalog-recognized entry when
  one exists, else to the first entry; `FrontmatterList` either way.
- `resolve_model_with_env` keeps its signature. The catalog parameter now
  only orders list hints; it never empties a resolution.

### D2. One CLI helper resolves the model on every route

`wrap/composition/target.rs` gains `resolve_document_model`, the single place
the CLI turns (provider, hints, `--model`, catalog) into a model. It runs the
existing refresh gate (`refresh_for_model_validation`, async, skipped when a
CLI/env source already decided), resolves through D1 with the catalog, and
when the resolved model came from frontmatter and the catalog does not
recognize it, emits one warning on the `[model]` facet naming the value and
the provider. Callers pass whether to refresh (dry-run never does) and whether
to warn (the retry/resume rebuild re-reads a document that already warned, so
it does not).

Every current call site uses it: `eagerly_resolve_target` (explicit,
dry-run, and live branches), `resolve_live_target_with_tty` (both branches),
`resolve_execution_target` (both branches), the sequence review's per-step
draft, `dry_run_sequence_target`, and `target_launch::resolve_launch_model`.
Under `--dry-run` the helper is still given the catalog (compiled baseline
plus configured overrides, no refresh), so the dry-run `Model` row is the
model the live run would launch with, and the warning renders in dry-run too.

### D3. One shared prep stage for every provider

`exec_prep::resolve_model_and_validate` becomes provider-neutral:

1. An explicit model (the resolved composition model, or the direct wrapper's
   `--model`) goes through `profile.apply_model` for every provider. That is
   the universal `--model` mapping plus the `MODEL` env export, unchanged.
2. With no explicit model, in non-interactive mode, and only when the catalog
   says `model_required_in_non_tty`, Claudine looks for one on the provider's
   behalf: the catalog's `model_env_vars`, in order, then
   `profile.configured_default_model()`. An env var hit is applied exactly
   like step 1 (the provider may read that variable itself, but the argv is
   then explicit and `MODEL` is right). A configured default sets only the
   `MODEL` env and reports its source; the provider already reads it. Nothing
   found is the existing no-model error.
3. `profile.validate_non_interactive_requirements` runs for every provider
   in non-interactive mode (OpenCode's is the default no-op, so this is not a
   behavior change).

`WrapperProfile::configured_default_model(&self) -> Option<ConfiguredModel>`
defaults to `None`. OpenCode's override reads its global config the way
OpenCode does: `opencode.jsonc`, `opencode.json`, then the legacy
`config.json`, under `$XDG_CONFIG_HOME/opencode` or `~/.config/opencode`,
parsed JSONC-tolerant through `biscuit_file::Json5`. The project-level
`opencode.json` layer is deliberately not read: it changes nothing about the
launch (no flag is pushed for a configured default), and reading it would
need the child CWD inside a stage that is otherwise CWD-free.

`OpenCodeModelSource` / `OpenCodeEnvSnapshot` / `resolve_opencode_model` /
`apply_opencode_model_resolution` are replaced by a provider-neutral
`ModelSource` (`CliSwitch`, `ProviderEnv { var, model }`,
`ConfigDefault { model, path }`) whose preamble and error-report strings name
the variable and file that actually decided. The wrapper's no-model report
keeps its OpenCode wording but names `opencode.json` and derives the env var
from the catalog.

### D4. Docs

`.claude/skills/claudine/composition.md` and `claudine/docs/topics/composition.md`
§ "Frontmatter `agent` and `model`" / "Model Resolution" / "OpenCode Non-TTY
Requirement" describe the forward-and-warn behavior and the provider-neutral
requirement; `claudine/cli/README.md` line 248 likewise; the
`exec_prep` module docs drop the `config.json` claim; a timeline entry
records the change and the Phase F origin.

## Scope

- `claudine/lib/src/composition/select.rs` (+ `select/tests.rs`).
- `claudine/cli/src/commands/wrap/composition/target.rs` (+ `tests.rs`),
  `wrap/sequence/{mod,resolve}.rs`,
  `wrap/harness_orch/loop_control/target_launch.rs`.
- `claudine/cli/src/commands/exec_prep/mod.rs`,
  `wrap/profile/{mod,resolve,opencode}.rs` (+ `profile/tests/apply_yolo.rs`),
  `wrap/{mod,wrapper_stages}.rs`, `wrap/composition/pipeline.rs`,
  `output/error_report.rs`.
- New L1 test `claudine/cli/tests/compose_frontmatter_model.rs` (compose,
  `--silent`, `--dry-run`, and sequence rows).
- Docs listed in D4.

## Acceptance criteria

1. With `agent: opencode`, `model: minimax/MiniMax-M3`, and no `--model` or
   model env vars, the child argv contains `--model minimax/MiniMax-M3` and
   the child env has `MODEL=minimax/MiniMax-M3`. L1 through
   `CliProcessFixture` with a fixture `opencode` that records argv and env.
2. The same run prints one `warning: [model] …` naming
   `minimax/MiniMax-M3` and OpenCode; `--silent` suppresses it. With
   `model: opencode/gpt-5.2-codex` no such warning is printed.
3. `resolve_model_with_env` unit tests: an unrecognized single hint resolves
   to the hint with `FrontmatterSingle`; a list with no recognized entry
   resolves to its first entry with `FrontmatterList`; a list whose second
   entry is recognized resolves to that entry; `--model`, provider env, and
   `MODEL` still win in that order and never warn.
4. `--dry-run` for the document in (1) renders `Model │ minimax/MiniMax-M3`
   and prints the same `[model]` warning, and issues no catalog refresh.
5. The direct wrapper: `claudine opencode …` with `OPENCODE_MODEL=x` and no
   `--model` still launches with `--model x` and `MODEL=x`; with neither and
   no configured default it still fails before launch with
   `No model specified!` (existing `wrap_opencode.rs` rows keep passing).
6. OpenCode default discovery unit tests over an explicit config directory:
   `opencode.jsonc` with a comment and a trailing comma parses; `opencode.jsonc`
   wins over `opencode.json` wins over `config.json`; an empty `model` is not
   a default; a missing directory is `None`.
7. A two-step sequence whose root document declares the model in (1)
   launches both steps with `--model minimax/MiniMax-M3` and prints the
   `[model]` notice once, not once per step. (Sequence tasks inherit the
   document-level target; per-task models are not a surface today.)
8. `just lint`, `just test`, and `just test-l2` pass in `claudine/`; the
   dispatch-inventory guard is untouched (no new `match Provider`);
   `claudine-gen -- check` is clean.
9. Docs in D4 updated in the same change.

## Verification record (2026-09-08, macOS host, worktree `fix/cli-slow-tests`)

- Reproduction against the fixed build: the child received
  `--model minimax/MiniMax-M3` and `MODEL=minimax/MiniMax-M3`, one
  `warning: [model] …` line printed, and `--dry-run` rendered the same model
  and warning.
- `just test-cli --test compose_frontmatter_model`: 5/5.
- Library `select` tests and CLI unit tests for `exec_prep`, `profile`,
  `error_report`, and composition target resolution: green.
- `just test` (full L1, `--no-fail-fast`): 6866 passed, 11 failed, 1 timed
  out. Every failure predates this change: `test_placement` names four files
  and the guard itself that were already modified in the worktree, and the
  ten `claudine-gen` drift/check rows compare generator inputs that were
  already modified in the worktree (`claudine/gen/src/*`) against committed
  provider data. The timed-out `context::values_report_captures_context_exactly_once`
  passes in isolation in 8 s against the 30 s cap (host load 30–74 during the
  run), matching the documented load-contention pattern. Three library tests
  timed out the same way on an earlier run and also pass in isolation.
- `just lint`: exit 0.
- `dispatch-inventory.json` re-blessed: one provider-keyed conditional fewer
  (the OpenCode branch in `profile/tests/positional.rs`), two direct
  references added by tests, no new dispatch. `spawn-seam-inventory.json`
  re-blessed for line shifts in files this change did not touch, so it now
  also absorbs the worktree's other uncommitted edits.
- `just test-l2` narrowed to the two dry-run capture binaries under tmux:
  `level2_dry_run_pty` 2/2; `level2_dry_run_metadata_capture` 6/6 in
  owned-pane mode (`BISCUIT_L2_THREADS=2`). In the default shared-pane mode
  one row, `level2_dry_run_not_installed_renders_yellow_dim_in_tmux`, fails
  deterministically with the no-agent cell; the same hermetic scenario
  renders `Agent Not Installed:(Qwen Code)` outside the harness with both the
  fresh and the installed binary, in a scrubbed environment and with this
  session's environment kept. That is shared-pane state leaking between rows
  of the file, not this change; the file and the harness (`common/mod.rs`)
  are already modified in the worktree.

## Non-goals

- Frontmatter `yolo:` (deferred to the permissions spec).
- Restoring the `_list` context twins; `commit.md` should move to
  `{{ctx.staged_files}}` and `{{ctx.staged_packages}}`.
- Did-you-mean warnings for unknown `ctx.*` inside deferred lifecycle values.
- The `ctx.repo` evidence gap and the `Partial runtime capture for git`
  warning (candidate fix: supply git identity whenever any repository group is
  requested in `project_evidence`, and confirm `GitRequest::summary()`
  populates `GitInfo::repo` on the launch entry).
- Making the `opencode models` listing feed validation again, or changing
  the compiled baseline.
- Reading a project-level `opencode.json` for the reported default.

## Reproduction notes

Stub provider used for every table row above:

```sh
#!/bin/sh
OUT="${STUB_OUT:-/tmp/opencode-stub.log}"
{
  echo "ARGV: $*"
  echo "MODEL=$MODEL"
  echo "OPENCODE_MODEL=$OPENCODE_MODEL"
  echo "OPENCODE_CONFIG_CONTENT=$OPENCODE_CONFIG_CONTENT"
} > "$OUT"
cat > /dev/null
exit 0
```

Run from inside any git repository with a staged file:

```sh
PATH=/path/to/stubdir:$PATH STUB_OUT=/tmp/stub.log \
  claudine compose ~/.claudine/prompts/commit.md
cat /tmp/stub.log
```
