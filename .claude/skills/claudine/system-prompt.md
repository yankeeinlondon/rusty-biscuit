# System Prompt Handling

## Contents

- Command Surfaces
- CLI Contract
- Shared Pipeline
- Discovery Rules
- Composition Semantics
- Non-Interactive Appendix
- Shell restrictions
- Provider Delivery
- Spec-Driven Dispatcher
- Scoped Temporary Files
- Harness Integration

Use heading search to jump to the listed subsystem.


The system-prompt refactor moved prompt resolution into a shared library pipeline and turned provider delivery into a launch-plan mutation step. Claudine now resolves a file-backed system prompt once, composes it through Darkmatter, and then lets each wrapped provider apply it with its own runtime strategy.

Non-interactive sessions add one more layer on top of that shared pipeline: a mandatory safety appendix that tells the provider not to request permission or ask follow-up questions. This appendix is appended after any resolved system prompt content, or becomes the full effective prompt when no other system prompt exists.

## Command Surfaces

System prompt handling is available on all wrapped provider subcommands:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`
- `claudine kilo`
- `claudine pi`
- `claudine antigravity`

The same flags are also shared by all composition entry points because `compose`, `inline-compose`, and `sequence` all flow through the same wrapper-grade execution path:

- `claudine compose <file>`
- `claudine inline-compose <file>`
- `claudine sequence <file>`

## CLI Contract

Claudine no longer uses the old universal `--system-prompt <PROMPT|FILE>` switch on these paths. The current interface is:

- `--append-system-prompt <FILE>`
- `--replace-system-prompt <FILE>`
- visible aliases: `--asp` and `--rsp`

Behavior:

- both switches are file-only
- they are mutually exclusive
- explicit files are resolved through the shared `biscuit-file::FileReference` grammar against the launch context: a bare implicit reference is repository-root first, then the launch directory; an explicit `./`/`../` reference is the launch directory only; `@` is a magic-root search, `~/` is home-pinned, and absolute paths resolve to themselves
- if an explicit file is selected, standard `system-prompt.md` discovery is skipped
- direct provider wrappers additionally accept `--edit`, which opens the **user prompt** (not the system prompt) in an external editor before launch; composition entry points do not

Internally these switches map to `claudine::system_prompt::SystemPromptArgs`.

## Shared Pipeline

The library pipeline lives in `claudine/lib/src/system_prompt/`:

1. `InvocationContext` captures the launch CWD, HOME, environment, and launch
   repository observation once; `LaunchContext` is projected from those facts
2. `resolve_system_prompt_source_with_invocation()` picks either an explicit
   file or a discovered `system-prompt.md` using launch-scoped resolution
3. each selected file receives a source-derived `SourceContext` and
   `FileResolutionContext`
4. `ContextRequirements` scans the primary prompt and appendix candidates as a
   union; the invocation supplies one demand-driven runtime snapshot for their
   composition
5. `prepare_system_prompt()` composes the selected file through Darkmatter
6. non-interactive sessions append `.claudine/non-interactive.md`,
   `~/.claudine/non-interactive.md`, or the built-in fallback message
7. providers apply the prepared result through
   `WrapperProfile::apply_system_prompt()`

Candidate selection remains anchored to the launch context. Deriving a source
context for a prompt in another repository does not move automatic
`system-prompt.md` or non-interactive appendix discovery there. Repository,
HOME, and environment evidence comes from the invocation owner: same-worktree
sources reuse topology, sibling repositories get one request-local
observation, and trusted files outside Git retain explicit repository absence.

`EffectiveSystemPrompt` is the handoff type between resolution/preparation and runtime delivery:

- `None` means no file was found or specified
- `Disabled` means a file was found, but its composed body was empty
- `Ready` contains the final `PreparedSystemPrompt`

## Discovery Rules

When no explicit flag is given, Claudine searches for a standard `system-prompt.md` based on the launch CWD, not the composition source file path.

Inside a detected repo/monorepo the search order is:

1. package root
2. package-area root
3. repo root
4. `~/.claudine/system-prompt.md`

Outside a detected repo the local search collapses to:

1. current working directory
2. `~/.claudine/system-prompt.md`

### Discovered-file delivery mode

A discovered `system-prompt.md` selects its own delivery mode with a `mode` frontmatter property:

- absent / `null` / `append` → append (the default, and the prior behavior)
- `replace` → the composed body replaces the provider's built-in default system prompt

Any other value is rejected during compose by a baseline `SimplifiedSchema` claudine attaches to discovered files only; the failure surfaces as a `SystemPromptComposition` error (naming `mode` and the allowed values) rather than a silent fallback to append. The schema default is annotation-only — an absent `mode` key is never backfilled into the composed frontmatter, so claudine reads the mode back from the composed result and treats absence as append.

Explicit `--append-system-prompt` / `--replace-system-prompt` files ignore `mode` entirely: the flag is authoritative, discovery is skipped, and the explicit path composes without the baseline schema. A document-owned `$schema` that redefines `mode` opts out of the baseline enum (Darkmatter merge rule: document-side-wins), but claudine does not invent additional delivery modes from it — any value other than `replace` still resolves to append.

`LaunchContext` is built with `sniff` repo-structure detection and carries:

- `cwd`
- `repo_root`
- `package_area_root`
- `package_root`

## Composition Semantics

Selected prompt files are composed with Darkmatter before they ever reach the provider. That means the system prompt supports the same document-level composition features as other Claudine Markdown flows, including source-aware transclusion and interpolation.

Current preparation behavior:

- the source file path is passed into `ComposeOptions::with_source_file(...)`
- the source-derived `FileResolutionContext` is passed into
  `ComposeOptions::with_file_resolution_context(...)`
- primary and appendix composition share one supplied, demand-driven
  `ComposeContext`; it uses the invocation's frozen environment and does not
  fall back to ambient Git, topology, HOME, or host capture
- frontmatter is not forwarded to the provider
- the canonical output is Markdown as authored after composition
- if the composed body is empty or whitespace-only, Claudine treats that as an explicit disable for the selected scope

File resolution and shell execution deliberately use different roots.
References authored inside a prompt resolve from that prompt's source context,
including when it lives in a sibling repository or outside Git. `::shell`
directives in both the primary prompt and a file-backed appendix remain pinned
to the agent's **launch repository root**, not the selected template's parent
or repository. Outside a launch repository they use the launch CWD. The
built-in appendix has no fabricated file source and requires no file-resolution
context.

Important disable rule:

- an empty composed body stops the search and produces `EffectiveSystemPrompt::Disabled`
- Claudine does not continue to lower-priority `system-prompt.md` locations after that

## Non-Interactive Appendix

When the wrapped session is non-interactive, Claudine appends an extra safety prompt after the resolved system prompt body. The lookup order is:

1. `<repo-root>/.claudine/non-interactive.md`
2. `~/.claudine/non-interactive.md`
3. built-in fallback:

```md
**IMPORTANT:** this is a non-interactive prompt; do not request permission or ask the caller questions!

## Shell restrictions

Do not run commands that require an interactive terminal or follow-up stdin input.
Avoid REPLs, editors, pagers, prompts, and any command that waits for user input.
Prefer one-shot commands and explicit non-interactive flags.
If a task would require sending more input to a running command, choose a different approach.
```

Behavior:

- the appendix is composed through Darkmatter when it comes from a file
- if a discovered appendix file composes to an empty body, Claudine falls through to the next candidate
- if there is already an effective system prompt, the appendix is appended after it
- if there is no effective system prompt, the appendix becomes the full system prompt
- explicit `--replace-system-prompt` keeps replace-mode semantics even after the appendix is added

## Provider Delivery

After preparation, each provider mutates its launch plan with args, env vars, and temp artifacts. Temporary files are held alive until the wrap call returns (drop-based cleanup).

Delivery is driven by the `SystemPromptSpec` declared in `ProviderInfo::system_prompt`. The wrap layer reads the spec and applies the correct mechanism per provider; profile code is a thin reader of that spec.

| Provider | Append | Replace | Runtime strategy |
|---|---|---|---|
| Claude Code | Yes | Yes | Interactive uses native string flags; non-interactive writes temp files and uses `--append-system-prompt-file` or `--system-prompt-file` |
| Codex | Yes | Yes | Append pushes `-c developer_instructions="..."` (64 KB cap); replace writes scoped temp file and pushes `-c model_instructions_file=<path>` |
| Gemini CLI | Yes | Yes | Both modes write a scoped temp file and set `GEMINI_SYSTEM_MD=<path>`. Append pre-composes the user's real `~/.gemini/GEMINI.md` with the overlay |
| Kimi Code | No | Yes | Replace writes a temp prompt file plus a temp agent YAML and passes `--agent-file` |
| Qwen Code | Yes | Yes | Append pushes `--append-system-prompt <content>`; replace pushes `--system-prompt <content>` |
| OpenCode | Yes | Yes | Append sets `OPENCODE_CONFIG_CONTENT` with a temp instruction file; replace passes `--system <temp>` |
| Goose | Yes | No | Append passes `--system <markdown>` directly |
| Kilo | No | No | Both modes are currently unsupported |
| Pi | Yes | Yes | Append pushes `--append-system-prompt <content>`; replace pushes `--system-prompt <content>` |
| Antigravity | No | No | Both modes are currently unsupported |

Unsupported modes are skipped with warnings rather than hard failures.

## Spec-Driven Dispatcher

The wrap layer uses `apply_system_prompt_via_spec` in `claudine/cli/src/commands/wrap/system_prompt.rs` to dispatch on `SystemPromptDelivery` variants:

- `InlineFlag { flag }` — pushes `flag` then the prompt content as two argv tokens
- `FileFlag { flag }` — writes content to a scoped temp file, then pushes `flag` and the file path
- `EnvVarFile { env_var }` — writes content to a scoped temp file, then sets the env var to the file path
- `ConfigKeyInline { flag, key }` — pushes `flag` then `key="<escaped_content>"` (used by Codex append)
- `ConfigKeyFile { flag, key }` — writes content to a scoped temp file, then pushes `flag` then `key=<path>` (used by Codex replace)
- `ShadowHomeFile { relative_path }` — legacy HOME-redirect path, retained for providers that require it
- `Custom(tag)` / `Unsupported` — warning, no-op

## Scoped Temporary Files

All transient overlay artifacts live inside the user's trust boundary:

- inside a repo: `<repo_root>/.claudine/tmp/`
- outside a repo: `<launch_cwd>/.claudine-tmp/`

Files are created via `tempfile::Builder` with a `.md` suffix so the `Drop` impl deletes them when the wrap call returns. A best-effort `.gitignore` augmentation idempotently appends the entry naming whichever directory was created — `.claudine/tmp/` inside a repo, `.claudine-tmp/` outside one — when a `.gitignore` already exists alongside it.

Which of the two layouts applies is decided once, from the `repo_root` resolved at launch. A directory that gains a `.git` mid-session keeps its `.claudine-tmp/` for that invocation and switches to `.claudine/tmp/` on the next launch.

### Gemini Append Composition

Gemini has no native append flag for `GEMINI_SYSTEM_MD`, so Claudine pre-composes the user's persistent `GEMINI.md` with the overlay before writing the merged file:

```rust
let real_gemini_md = dirs::home_dir()
    .map(|h| h.join(".gemini").join("GEMINI.md"))
    .filter(|p| p.is_file());
let merged = match real_gemini_md {
    Some(path) => format!("{}\n\n{}", std::fs::read_to_string(&path)?.trim_end(), overlay),
    None => overlay.to_string(),
};
```

This preserves the user's persistent Gemini context but means Gemini's built-in default system prompt is silently dropped in append mode. Use `--replace-system-prompt` if you want full control.

### Codex Append Argv Limit

Codex append uses inline config (`-c developer_instructions="..."`), which is subject to the platform argv size limit. Claudine enforces a 64 KB hard cap:

```rust
const CODEX_INLINE_LIMIT_BYTES: usize = 64 * 1024;
```

If the composed prompt exceeds this, the wrap errors with a clear message telling the user to use `--replace-system-prompt` instead. There is no silent fallback to replace semantics.

## Harness Integration

The system prompt capability model also informs wrapper-harness source discovery. `find_wrapper_harness_source()` looks at the selected agent's `runtime.system_prompt.memory_files`, ignores home-relative entries such as `~/.gemini/GEMINI.md`, and searches the repo root or current working directory for the first provider-specific memory file that exists on disk.

This is separate from Claudine-managed prompt injection:

- provider memory files remain a provider-native signal
- `system-prompt.md` is Claudine's standard discovery surface
- both can coexist in the same wrapped session
