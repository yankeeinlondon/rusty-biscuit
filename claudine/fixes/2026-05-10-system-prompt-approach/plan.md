---
source_files_during_phase_1:
  - claudine/lib/src/provider/system_prompt.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/provider/gemini.rs
  - claudine/lib/src/provider/codex.rs
  - claudine/lib/src/provider/qwen.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/system_prompt.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/commands/wrap/profile/gemini.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/profile/qwen.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/profile/opencode.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/provider/system_prompt.rs
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/commands/wrap/profile/gemini.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/profile/qwen.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/docs/topics/system-prompt.md
  - claudine/docs/research/agent-cli/gemini-cli.md
  - claudine/docs/research/agent-cli/qwen-cli.md
  - claudine/docs/research/agent-cli/codex.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/SKILL.md
packages:
  - claudine
  - claudine-cli
---

# System Prompt Delivery Redesign

## Problem statement

Wrapping `gemini`, `codex`, or `qwen` through claudine clobbers the
child's view of the user's persistent provider state. The current
mechanism for injecting a per-invocation system prompt is to:

1. Create a fresh empty `TempDir`.
2. Write only `<provider_dir>/<overlay_file>` (e.g. `.gemini/GEMINI.md`)
   into it.
3. Set `HOME=<tempdir>` on the child.

Collateral damage observed in real sessions:

- `~/.gemini/settings.json` (trust list, auth method choice) is invisible
  to the child, so every Gemini run shows a 3-prompt wizard:
  1. "Trust folder (claudine)" / "Trust parent folder (rusty-biscuit)"
  2. "How would you like to authenticate?"
  3. "Enter Gemini API Key" (despite `GEMINI_API_KEY` being set)
- With API-key auth still being whitelisted via `allowed_env_keys`, the
  in-session API key flows through fine — but Gemini's persisted auth
  method choice is gone, so non-interactive runs appear to fall back to
  an unauthenticated quota tier and produce a `RESOURCE_EXHAUSTED` 429
  storm.
- Same shape for Codex (`~/.codex/`) and Qwen (`~/.qwen/`): any
  per-folder trust, MCP, hook, and history state is silently invisible
  to the child.

In parallel, the wrapper pollutes its own parent process with
`std::env::set_var("AGENT", …)` at three sites. The user's standing
rule is: **set context env vars on the spawn `Command`, not on the
parent process**. Today's AGENT propagation happens to work because
`build_child_env_with_launch` already adds AGENT to the spawn env, so
the parent `set_var` is redundant and leaky.

## Root cause

Two structural decisions combined to produce the regression.

### 1. HOME redirect is the wrong injection surface

The HOME-redirect mechanism in
[`create_ephemeral_overlay_home`](../../cli/src/commands/wrap/system_prompt.rs)
chose a sledgehammer: it gives the child a completely synthetic HOME
with only the overlay file present. Everything else under the real
provider directory (auth, settings, trust list, MCP, hooks, history) is
collateral damage.

### 2. Two of three affected providers have native non-HOME mechanisms

Per the per-provider research:

- **Gemini**: `GEMINI_SYSTEM_MD=<file path>` env var loads a custom
  system prompt from any path. Full replacement (no native append flag),
  but for claudine's overlay use case that's acceptable because we can
  pre-compose `<real ~/.gemini/GEMINI.md content> + <overlay>` before
  pointing the env var at it.
- **Qwen**: `--system-prompt <string>` (replace) and
  `--append-system-prompt <string>` (true append) both accept the
  prompt content inline as a string. No file needed at all.
- **Codex**: `-c model_instructions_file=<path>` for replace,
  `-c developer_instructions="<string>"` for inline supplement
  (append-ish).

The wrap code already encodes these mechanisms descriptively in
[`SystemPromptSpec`](../../lib/src/provider/system_prompt.rs) but does
not *consume* them — each profile hardcodes its own
`apply_system_prompt` instead.

### 3. Temp file scope was unbounded

The current code writes overlay files into `/tmp` (or `$TMPDIR`) via
`tempfile::NamedTempFile::new()`. The user's directive is to keep all
overlay content within the **trust boundary** the user is operating
under, i.e. inside the repo root when in a repo, or inside the launch
CWD tree otherwise.

## Goal

Restore reliable behavior for `claudine gemini`, `claudine codex`, and
`claudine qwen` such that:

1. The persistent provider state directory (`~/.gemini/`, `~/.codex/`,
   `~/.qwen/`) is fully visible to the child — no wizard popups, no
   false trust prompts, no 429 storms, no MCP/hooks regressions.
2. The injected system prompt content still reaches the child.
3. All overlay artifacts (temp files) live inside the user's trust
   boundary: `<repo_root>/.claudine/tmp/` when in a repo,
   `<launch_cwd>/.claudine-tmp/` otherwise. Drop-based cleanup.
4. System-prompt delivery metadata is the single source of truth in
   [`ProviderInfo::system_prompt`](../../lib/src/provider/mod.rs); the
   wrap profile dispatch becomes a thin reader of that spec.
5. The wrapper does not mutate its own parent process env for `AGENT`,
   `PATH`, or any other context variable — those are set on the
   `Command` spawn call and only there.
6. OpenCode's duplicate `--model` argv push is removed; `MODEL` is
   already plumbed through env.

Out of scope (deliberately):

- Adding a Gemini append flag where none exists. We accept that Gemini
  "Append" mode under the new scheme will silently drop Gemini's
  built-in default system prompt; the user's persistent `GEMINI.md`
  content is preserved by pre-composing it into the merged file.
- Refactoring `repo_home::needs_shadow_home` / `build_repo_home_env`
  (the *other* shadow-HOME path used by `--repo` isolation). That
  remains scoped to repo-isolation use cases.
- Removing the `Custom(CodexInstructionsFile)` enum tag immediately;
  it becomes unused after Phase 4 but is kept as a `#[deprecated]`
  marker for one release cycle.

## Fix shape

A spec-driven dispatcher in the wrap layer that reads
`SystemPromptSpec` and applies the correct mechanism per provider, plus
data-only spec updates per affected provider. No HOME redirect for
Gemini, Codex, or Qwen.

### Architecture summary

```
ProviderInfo::system_prompt: &'static SystemPromptSpec
       │
       ├── append:  SystemPromptDeliveryByMode { interactive, non_interactive }
       └── replace: SystemPromptDeliveryByMode { interactive, non_interactive }
                                         │
                                         └── one of:
                                             - InlineFlag       → push two argv tokens
                                             - FileFlag         → write scoped file, push two argv tokens
                                             - EnvVarFile       → write scoped file, push env var
                                             - ConfigKeyInline  → push `-c key="<content>"`     (new)
                                             - ConfigKeyFile    → write scoped file, push `-c key=<path>` (new)
                                             - ShadowHomeFile   → existing HOME-redirect (Kimi fallback)
                                             - Custom(tag)      → typed-tag dispatch (legacy)
                                             - Unsupported      → warning, no-op
```

### Scoped temp file helper

```rust
/// Resolve the directory where claudine should drop transient
/// per-invocation overlay files (system prompts, instruction files,
/// etc.). Always inside the user's trust boundary.
fn scoped_tmp_dir(launch_workspace: &LaunchWorkspaceContext) -> PathBuf {
    let base = launch_workspace
        .repo_root
        .clone()
        .unwrap_or_else(|| launch_workspace.launch_cwd.clone());

    let scoped = if launch_workspace.repo_root.is_some() {
        base.join(".claudine").join("tmp")
    } else {
        base.join(".claudine-tmp")
    };
    let _ = std::fs::create_dir_all(&scoped);
    scoped
}
```

All temp files are created via
`tempfile::Builder::new().prefix(...).suffix(".md").tempfile_in(&dir)`
so the `Drop` impl deletes them when the wrap call returns.

### Gemini Append composition

```rust
let overlay = prompt.composed_markdown.as_str();
let real_gemini_md = dirs::home_dir()
    .map(|h| h.join(".gemini").join("GEMINI.md"))
    .filter(|p| p.is_file());
let merged = match real_gemini_md {
    Some(path) => format!(
        "{}\n\n{}",
        std::fs::read_to_string(&path)?.trim_end(),
        overlay
    ),
    None => overlay.to_string(),
};
// Write to scoped_tmp_dir, set GEMINI_SYSTEM_MD=<path>.
```

### Codex Append argv-size cap

```rust
const CODEX_INLINE_LIMIT_BYTES: usize = 64 * 1024;
if content.len() > CODEX_INLINE_LIMIT_BYTES {
    bail!(
        "codex append-system-prompt exceeds {}KB argv limit ({}B); \
         shrink the system prompt or use --replace-system-prompt instead",
        CODEX_INLINE_LIMIT_BYTES / 1024,
        content.len()
    );
}
```

No silent fallback to Replace semantics — the user must decide.

## Phases

### Phase 1 — Extend `SystemPromptDelivery` enum

Add two new variants to
[`claudine/lib/src/provider/system_prompt.rs`](../../lib/src/provider/system_prompt.rs):

```rust
pub enum SystemPromptDelivery {
    // ... existing variants unchanged ...

    /// Provider config-override flag plus key, with the prompt content
    /// inlined as the value. E.g. Codex `-c developer_instructions="..."`.
    ConfigKeyInline {
        flag: &'static str,
        key: &'static str,
    },

    /// Provider config-override flag plus key, with the value being a
    /// path to a file containing the prompt content. E.g. Codex
    /// `-c model_instructions_file=<path>`.
    ConfigKeyFile {
        flag: &'static str,
        key: &'static str,
    },
}
```

Add `#[deprecated]` marker on the `SystemPromptCustomTag::CodexInstructionsFile`
variant (one-cycle removal).

Unit test: `ConfigKeyInline` and `ConfigKeyFile` round-trip through the
existing `Serialize` impl.

### Phase 2 — Update spec data per affected provider

Edit the static `*_SYSTEM_PROMPT` constants:

- `claudine/lib/src/provider/gemini.rs`: both append and replace become
  `EnvVarFile { env_var: "GEMINI_SYSTEM_MD" }` (both interactive and
  non-interactive modes).
- `claudine/lib/src/provider/qwen.rs`: append becomes
  `InlineFlag { flag: "--append-system-prompt" }`; replace becomes
  `InlineFlag { flag: "--system-prompt" }` (replacing the current
  `Unsupported`).
- `claudine/lib/src/provider/codex.rs`: append becomes
  `ConfigKeyInline { flag: "-c", key: "developer_instructions" }`;
  replace becomes
  `ConfigKeyFile { flag: "-c", key: "model_instructions_file" }`.

Provider invariants test (`provider/tests.rs` or similar): assert each
spec variant has the right discriminant for each affected provider so
the data shape doesn't drift.

### Phase 3 — Scoped temp file helper

In `claudine/cli/src/commands/wrap/system_prompt.rs`:

- Add `pub(crate) fn scoped_tmp_dir(launch_workspace: &LaunchWorkspaceContext) -> PathBuf`.
- Add `pub(crate) fn scoped_tempfile(base: &Path, prefix: &str) -> std::io::Result<NamedTempFile>`
  using `tempfile::Builder`.
- Best-effort `.gitignore` augmentation: when `repo_root.join(".gitignore")`
  exists and doesn't already contain `.claudine/tmp/`, append the line.
  Skip silently on failure (this is a courtesy, not a contract).

Unit tests:

- `scoped_tmp_dir` returns `<repo_root>/.claudine/tmp/` when
  `repo_root.is_some()`, `<launch_cwd>/.claudine-tmp/` otherwise.
- `scoped_tempfile` actually creates the file inside the requested base
  directory.

### Phase 4 — Spec-driven dispatcher + profile refactor

In `claudine/cli/src/commands/wrap/system_prompt.rs`:

```rust
pub(crate) fn apply_system_prompt_via_spec(
    spec: &SystemPromptSpec,
    mode: SystemPromptMode,
    interactive: bool,
    content: &str,
    real_provider_dir: Option<&Path>,
    scoped_tmp: &Path,
) -> Result<SystemPromptApplication> {
    let delivery_by_mode = match mode {
        SystemPromptMode::Append => &spec.append,
        SystemPromptMode::Replace => &spec.replace,
    };
    let delivery = if interactive {
        delivery_by_mode.interactive
    } else {
        delivery_by_mode.non_interactive
    };

    match delivery {
        SystemPromptDelivery::InlineFlag { flag } => { /* push flag + content */ }
        SystemPromptDelivery::FileFlag { flag } => { /* scoped file + flag */ }
        SystemPromptDelivery::EnvVarFile { env_var } => {
            // For Gemini Append: read real provider dir's overlay file,
            // concatenate with `content`, write to scoped temp file.
            // For Replace: write `content` only.
            // Set env var.
        }
        SystemPromptDelivery::ConfigKeyInline { flag, key } => {
            // Argv size check (64KB hard cap).
            // push `flag` then `format!("{key}=\"{escaped_content}\"")`.
        }
        SystemPromptDelivery::ConfigKeyFile { flag, key } => {
            // Write content to scoped temp file.
            // push `flag` then `format!("{key}={path}")`.
        }
        SystemPromptDelivery::ShadowHomeFile { relative_path } => {
            // Existing HOME-redirect path; retained for legacy callers.
            // After Phase 4 only Kimi uses this (if applicable); otherwise
            // dead code retained for the deprecation cycle.
        }
        SystemPromptDelivery::Custom(_) | SystemPromptDelivery::Unsupported => { /* ... */ }
    }
}
```

Refactor `apply_system_prompt` in each affected profile
(`gemini.rs`, `codex.rs`, `qwen.rs`) to call the dispatcher with
`profile.system_prompt_spec()` (added as a trait accessor on
`WrapperProfile`, defaulting to `provider_info(self.provider()).system_prompt`).

Per-provider apply tests:

- Gemini append composes real GEMINI.md + overlay into a single scoped
  file and sets `GEMINI_SYSTEM_MD`.
- Gemini replace writes only the overlay and sets `GEMINI_SYSTEM_MD`.
- Codex append errors out when the inline content exceeds 64KB.
- Codex append succeeds with `-c developer_instructions="..."` argv push
  for small content.
- Codex replace writes to a scoped file and pushes
  `-c model_instructions_file=<path>`.
- Qwen append/replace push the right inline flag.
- No `HOME` env var is set on any of these spawn `Command`s after the
  refactor.

### Phase 5 — Parent-env hygiene + OpenCode dedupe

Remove parent `std::env::set_var` calls in the wrap path:

- `claudine/cli/src/commands/wrap/mod.rs:346` (`AGENT`).
- `claudine/cli/src/commands/wrap/sequence.rs:328` (`AGENT`).
- `claudine/cli/src/commands/wrap/composition/mod.rs:540` (`AGENT`).

Compensate by:

- Identifying every template renderer that reads `env::var("AGENT")`
  *before* the spawn (search shows only `handle.rs:175`, which runs
  **inside** the spawned agent's environment and is unaffected by
  removing the parent set_var — confirmed via grep).
- For any pre-spawn template lookup that depends on AGENT, plumb the
  resolved provider identity explicitly through the lookup map already
  used by `EventMetaExpressionLookup` / `build_vars`.

In `claudine/cli/src/commands/wrap/mod.rs:1385,1390` the
`set_var("PATH", …)` calls scope to the wrapper's own process and may
be needed for `which` lookups. Audit: if they only feed the child
spawn env, move them to the `Command::env("PATH", …)` call instead.

In `claudine/cli/src/commands/wrap/profile/opencode.rs:91-92` remove
the duplicate `args.push("--model"); args.push(model)` since MODEL is
already added to `env_overrides` on line 93 and the OpenCode CLI reads
MODEL from env.

### Phase 6 — Delete dead `create_ephemeral_overlay_home` calls

After Phase 4 lands, the three callers of `create_ephemeral_overlay_home`
in `profile/gemini.rs`, `profile/codex.rs`, and `profile/qwen.rs` are
gone. Either:

- Delete `create_ephemeral_overlay_home` outright and the
  `ShadowHomeFile` enum variant becomes effectively unused, OR
- Keep both behind a single test-only call site to preserve the
  legacy path for Kimi / future providers that may need it.

Recommended: delete the function; if a future provider needs HOME
redirect we can resurrect it from git history. The `ShadowHomeFile`
enum variant stays as a typed descriptor (other providers' spec data
may still reference it) — but no wrap code consumes it after this fix.

### Phase 7 — Docs + skill

Update:

- [`claudine/docs/topics/system-prompt.md`](../../docs/topics/system-prompt.md)
  to describe the new dispatcher and the per-provider mechanism table.
- The provider research docs to call out the env-var/flag we use:
  - `claudine/docs/research/agent-cli/gemini-cli.md` —
    note `GEMINI_SYSTEM_MD` is the official non-shadow-HOME mechanism.
  - `claudine/docs/research/agent-cli/qwen-cli.md` —
    note `--append-system-prompt` / `--system-prompt` are the
    in-flight mechanisms.
  - `claudine/docs/research/agent-cli/codex.md` — note
    `-c developer_instructions` / `-c model_instructions_file`.
- [`.claude/skills/claudine/SKILL.md`](../../.claude/skills/claudine/SKILL.md)
  — replace the HOME-redirect description with the spec-driven
  dispatcher summary and add the scoped-tmp-dir paragraph.

## Verification checklist

Before merging:

- [ ] All existing wrap tests pass (`just test claudine-cli`).
- [ ] New per-provider apply tests pass.
- [ ] `claudine gemini -p "hello"` from a worktree does **not** show
      the trust / auth / API-key wizard prompts.
- [ ] `claudine gemini -p "hello"` does not produce 429 RESOURCE_EXHAUSTED
      with a valid `GEMINI_API_KEY` in the environment.
- [ ] `claudine codex -p "hello"` and `claudine qwen -p "hello"` work
      end-to-end without HOME redirect.
- [ ] No `HOME` env var is set on Gemini/Codex/Qwen child `Command`s
      (verified by grep + a new integration assertion).
- [ ] No `std::env::set_var("AGENT", …)` remains in the wrap path
      (verified by grep).
- [ ] `<repo_root>/.claudine/tmp/` is created on first wrap call and
      auto-cleaned after the wrap call returns (drop-based).
- [ ] `.gitignore` augmentation (best-effort) writes the new entry
      idempotently — running the wrap twice doesn't append duplicates.
- [ ] Codex append errors with a clear message when the system prompt
      exceeds 64KB.
- [ ] Manual reproduction of `claudine compose prompts/commit.md`
      against Gemini succeeds without the wizard or 429s.

## Risks

- **Gemini built-in prompt loss.** Append mode under the new scheme
  no longer benefits from Gemini's built-in default system prompt
  (which previously layered under `~/.gemini/GEMINI.md`). The user's
  persistent `GEMINI.md` content is still preserved by composition,
  but Gemini's hardcoded defaults are not. Document this in
  `docs/topics/system-prompt.md` and call it out in the changelog.
- **Codex argv-size cap on append.** Hard 64KB cap means very large
  system prompts will fail rather than degrade silently. The error
  message tells the user how to recover (`--replace-system-prompt`).
- **`.gitignore` augmentation.** Writing into the user's `.gitignore`
  is a side effect; keep it best-effort and idempotent. Skip silently
  on permission failure.
- **`PATH` set_var audit (Phase 5).** The PATH mutations at
  `wrap/mod.rs:1385,1390` may have non-obvious consumers. Audit
  carefully before moving them to the Command env; preserve current
  behavior if uncertain.
- **Symlink / mirror path is not used.** The previously considered
  "symlink mirror of real provider dir" approach is **rejected** in
  favor of the spec-driven non-HOME mechanism. If future providers
  need true append semantics without an env var or flag, resurrect
  the mirror approach as a fifth `SystemPromptDelivery` variant.

## Open questions

- Should `.gitignore` augmentation be a separate user-facing command
  (`claudine doctor --fix`) instead of a silent side effect? The plan
  currently does it inline; the conservative alternative is a
  startup warning telling the user to add the line themselves.
- Codex `developer_instructions` is documented as a "supplement" —
  semantically it concatenates *after* AGENTS.md content, which is
  close to but not identical to append-to-built-in. Verify behavior
  in Phase 4 testing; if the semantics diverge meaningfully, document
  the divergence rather than papering over it.
