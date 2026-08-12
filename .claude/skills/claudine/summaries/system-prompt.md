# System Prompt Support Across Agentic CLI Providers

The system prompt is the highest-leverage instruction surface in an agentic session because it shapes the agent before any user task, repository memory, tool result, or lifecycle event is considered. For coding agents, that layer usually contains the provider's operating contract: how to use tools, how to handle permissions, how to reason about files, how to report results, and which safety or product-specific behaviors must remain stable.

Appending and replacing are materially different operations.

Appending preserves the provider's built-in prompt and adds local policy, project conventions, workflow rules, or non-interactive constraints after it or alongside it. This is usually the safer default because the provider still supplies its tool-use and behavioral scaffolding.

Replacing discards the provider's base prompt where the provider exposes a base-prompt replacement surface. Replacement is useful when Claudine needs a deterministic role, a narrowed task identity, or a provider-neutral harness, but it is riskier: the replacement prompt must restate any tool, permission, safety, output, and coding behaviors the provider would otherwise have supplied. Some providers still load memory files, context files, extensions, skills, MCP instructions, or environment sections after replacement, so "replace" rarely means "only this text reaches the model."

Claudine's wrapper interface deliberately hides provider differences behind a file-backed contract:

- `--append-system-prompt` / `--asp`
- `--replace-system-prompt` / `--rsp`
- discovered `system-prompt.md` from the launch-CWD hierarchy

That interface maps cleanly to a few providers and only approximately to others. This summary focuses on the providers Claudine currently wraps: Claude Code, Codex, Gemini CLI, Goose, Kimi Code, OpenCode, and Qwen Code.

## Provider Matrix

| Provider    | Append support                    | Replace support                       | Best Claudine mapping                                                                                                                            | Main caveat                                                                                            |
|-------------|-----------------------------------|---------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| Claude Code | Native inline/file flag           | Native inline/file flag               | Pass resolved files through Claude's prompt-file flags                                                                                           | Replacement drops Claude Code's default engineering/tool guidance                                      |
| Codex       | Developer-role config layer       | Config/file replacement               | `-c developer_instructions=...` for append-like behavior; `-c model_instructions_file=<path>` for replace                                        | Append is a developer message, not a literal system-prompt append; replacement is provider-discouraged |
| Gemini CLI  | Context-file append workaround    | Env/file replacement                  | Temp `GEMINI.md` discovery for append; temp system file via `GEMINI_SYSTEM_MD` for replace                                                       | Append is not native; replacement may bypass Gemini's built-in prompt structure                        |
| Goose       | Native append on `goose run` only | Indirect replacement                  | `goose run --system <text>` for non-interactive append; isolated template/config or fragile internal env/ACP paths for replace                   | No `--system-file`; `--system` conflicts with `--recipe`; no equivalent on `goose session`             |
| Kimi Code   | File/context append               | Legacy agent spec only                | Temp `AGENTS.md` for append; legacy `kimi-cli --agent-file` with temp `agent.yaml` + `system.md` for replace                                     | Current Kimi Code has no per-session replace surface and no system-prompt flags                        |
| OpenCode    | Config temp-file append           | Agent-spec slot-0 replacement         | Inject temp prompt file through `OPENCODE_CONFIG_CONTENT.instructions`; inject wrapper-named primary agent and pass `--agent <name>` for replace | No prompt flags; agent `prompt` replaces only slot 0 while later layers still append                   |
| Qwen Code   | Native inline flag                | Native inline flag, env/file fallback | Read Claudine's file and pass `--append-system-prompt <text>` or `--system-prompt <text>`                                                        | No native append-file flag; replacement does not suppress `QWEN.md`/memory/context layers              |

## Claude Code

Claude Code has the strongest native fit. It supports both append and replace as first-class system prompt operations.

Append mechanisms include `--append-system-prompt <text>` and the accepted file form `--append-system-prompt-file <path>`. Replacement mechanisms include `--system-prompt <text>` and `--system-prompt-file <path>`. Claude also has adjacent prompt-shaping surfaces: `CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/*.md`, output styles, custom agents, managed settings, safe mode, and bare mode.

For Claudine, this is the cleanest mapping. A file-backed append can be delivered as Claude's append file flag, and a file-backed replace can be delivered as Claude's system prompt file flag. The main risk is semantic rather than mechanical: replacement removes Claude Code's built-in coding instructions, tool-use guidance, and safety posture. Project memory such as `CLAUDE.md` can still load, so replacement is not total context isolation.

## Codex

Codex exposes prompt control through configuration rather than dedicated prompt flags. The top-level `instructions` key and `model_instructions_file` replace the bundled base instructions. `developer_instructions` adds a separate developer-role message alongside the base prompt.

For Claudine append, `developer_instructions` is the practical runtime mapping. It preserves Codex's bundled base instructions while adding Claudine's content. This is close to append in behavior, but it is not literally appended to the system prompt; it is an additional developer message. That distinction matters when debugging prompt ordering or provider behavior.

For Claudine replace, `model_instructions_file` is the best file-backed delivery path. Claudine can write a temporary prompt file and pass it through `-c model_instructions_file=<path>` without mutating `~/.codex/config.toml`.

Codex's own documentation warns against replacing sanctioned model instructions because performance can degrade. Codex also has substantial adjacent prompt surfaces: `AGENTS.md`, `AGENTS.override.md`, profiles, project `.codex/config.toml`, custom agents, skills, feature flags, memories, personality, and model-specific bundled base prompts.

## Gemini CLI

Gemini has a supported replacement path through `GEMINI_SYSTEM_MD`. Claudine can write the resolved prompt to a temporary Markdown file and set `GEMINI_SYSTEM_MD=<path>` for the wrapped invocation. When the variable is set to `1` or `true`, Gemini reads the default `.gemini/system.md` path instead. Missing replacement files are fatal.

Append is weaker. Gemini does not provide a first-class append flag comparable to Claude or Qwen. Its native append-like surface is `GEMINI.md`: global, project, and just-in-time directory context files are loaded into the prompt's context block. For Claudine append, the least invasive mapping is to write a temporary `GEMINI.md` file and make Gemini discover it through a temporary included directory or shadow workspace, rather than mutating user or project memory files.

That mapping is serviceable but leaky. A `GEMINI.md` append is a memory/context layer, not a system-prompt append at a precise position in Gemini's base prompt. A replacement through `GEMINI_SYSTEM_MD` can also bypass Gemini's built-in prompt structure unless Claudine deliberately preserves dynamic substitution slots such as available tools, agent skills, and subagents.

Gemini also has prompt-affecting modes such as plan mode, YOLO/auto-edit approval modes, sandbox preamble, extensions, subagents, model config overrides, and available-tool substitution. Claudine should treat Gemini append as a compatibility strategy, not a true native append.

## Goose

Goose has a native append path, but it is narrower than Claude or Qwen. The `goose run --system <TEXT>` flag adds the supplied text as a `system_prompt_extra` under the `additional` key, rendered below the base template under `# Additional Instructions:`. This is a good behavioral match for Claudine append in non-interactive runs, but it is inline-text only: Goose has no `--system-file` equivalent, the flag is available on `goose run` rather than `goose session`, and it conflicts with `--recipe`.

Goose's effective prompt is assembled from a Markdown/Jinja2 `system.md` template, extension instructions, and appended extras. `.goosehints` and `AGENTS.md` files feed the `hints` extra; extension and builtin instructions render into the template's extension section; recipes add task-specific `instructions` and `prompt` content without replacing the core system prompt. `GOOSE_MOIM_MESSAGE_TEXT` and `GOOSE_MOIM_MESSAGE_FILE` are also important, but they are not ordinary append: MOIM content is injected into working memory and re-read every turn, which makes it stronger and more persistent than one-shot `--system` text.

Replacement is indirect. The supported public replacement mechanism is a custom prompt template file: `~/.config/goose/prompts/system.md` on macOS/Linux or `%APPDATA%\Block\goose\config\prompts\system.md` on Windows. That replaces Goose's built-in main-session template for new sessions, but it is persistent user configuration, not an ephemeral per-launch CLI flag. Goose also has a separate `subagent_system.md` override for subagents.

There are programmatic replacement paths, but they should be treated carefully. `GOOSE_SYSTEM_PROMPT_FILE_PATH` is wired in Goose's code as a session-time replacement file, but it is undocumented and absent from the public CLI/env-var surface. Goose ACP exposes `set_session_system_prompt` with `Set` and `Append` modes, but that is available through `goose acp`, not the normal CLI wrapper path.

For Claudine, Goose append should map to `goose run --system <TEXT>` when running non-interactively and when the composed prompt is small enough for platform argv limits. Interactive append needs a fallback such as a shadow `.goosehints` / `AGENTS.md` context file or `GOOSE_MOIM_MESSAGE_FILE`, with the caveat that those are not identical to `--system`. Claudine replace should remain marked indirect or unsupported unless the wrapper deliberately creates an isolated config/template overlay or opts into the fragile `GOOSE_SYSTEM_PROMPT_FILE_PATH` path. It should not mutate the user's real Goose prompt template.

## Kimi Code

Kimi Code does not expose `--system-prompt`, `--append-system-prompt`, or `--replace-system-prompt` flags. Its effective prompt is assembled from a built-in base template plus layered file and runtime inputs: AGENTS.md content, the skills catalog, working-directory listings, additional directories, model capabilities, and session state.

Append is file-based. The supported customization surface is AGENTS.md-style instruction files: `$KIMI_CODE_HOME/AGENTS.md`, `~/.agents/AGENTS.md`, project `.kimi/AGENTS.md`, project `AGENTS.md` or `agents.md`, and the documented `.kimi-code/AGENTS.md` alias. Skills also influence the assembled prompt through Kimi's skills catalog, but they are not a direct system-prompt override. For Claudine append, the least invasive mapping is to write the composed prompt into a temporary `AGENTS.md` or `.kimi/AGENTS.md` in the launch work tree or a shadow work tree, optionally relocating Kimi state with `KIMI_CODE_HOME` so user config and sessions are not mutated.

Replacement depends on which Kimi implementation is running. The legacy Python `kimi-cli` supports per-session replacement through an agent spec: `kimi-cli --agent-file <agent.yaml>`, where the YAML points at a sibling Markdown `system.md`. That replaces the built-in agent system prompt for that legacy session. Current Kimi Code no longer exposes that per-session agent-file replacement path; its documented customization path is AGENTS.md, so Claudine should treat replace as unsupported for current Kimi Code until upstream exposes a per-session replacement surface.

Kimi's notable risks are mostly around hidden assembly behavior rather than argv mechanics. AGENTS.md content may be merged with existing user and project instructions, not isolated from them. The legacy runtime concatenates AGENTS.md files root-to-leaf, annotates their source paths, and applies a 32 KiB total cap. Its prompt template is rendered with Jinja2 `StrictUndefined`, so stray `${...}` placeholders in appended or replacement Markdown can abort the session. Session resume, `--add-dir`, `--skills-dir`, model aliases, plan/auto/yolo modes, and subagent execution can all affect the effective context without being system-prompt controls.

Claudine's append interface maps reasonably to Kimi as a temporary AGENTS.md delivery strategy. Claudine's replace interface should be split by implementation: supported only for legacy `kimi-cli` via temporary `agent.yaml` plus `system.md`, and unsupported for current Kimi Code.

## OpenCode

OpenCode has no native `--system-prompt` or `--append-system-prompt` flag. The only CLI parameter that directly changes the assembled system text is `--agent <name>`, and that only changes slot 0 when the selected agent defines a `prompt`.

The stock slot-0 prompt is selected from embedded provider/model prompt files based on the model ID. A custom agent `prompt` replaces that stock provider/model prompt for slot 0; it does not replace the full effective system prompt. OpenCode still appends the remaining layers: `opencode.json` `instructions`, discovered `AGENTS.md` or Claude-compatible fallback files, environment context, skills, MCP instructions, and any per-session user system layer.

Append is config-driven. `opencode.json` supports an `instructions` array of file paths, globs, and remote URLs; those entries are combined with AGENTS.md-style rules after slot 0. For Claudine, the correct append mapping is to write the resolved prompt to a temporary Markdown file and inject that path through `OPENCODE_CONFIG_CONTENT.instructions`. This avoids mutating `~/.config/opencode/opencode.json` or repository `AGENTS.md`.

Replacement is agent-spec driven. Claudine should write the replacement prompt to a temporary file, inject a wrapper-named primary agent through `OPENCODE_CONFIG_CONTENT.agent.<name>.prompt` using `{file:<tmp>}`, and launch OpenCode with `--agent <name>`. This is native OpenCode behavior, but it is not equivalent to a dedicated base-prompt replacement flag: it replaces only the stock provider/model prompt slot, not the later appended layers.

`OPENCODE_CONFIG_CONTENT` is also used by Claudine for OpenCode MCP injection and permission overlays, so prompt delivery must merge overlays into one config blob instead of overwriting the variable. Managed configuration can still win: macOS MDM preferences sit above `OPENCODE_CONFIG_CONTENT`, and managed config may constrain or override user/project settings.

Important limitations: OpenCode does not currently expose a stable way to inspect or export the effective assembled system prompt; `--pure` disables external plugins only and does not disable AGENTS.md, instructions, agent prompts, MCP, or skills; there is no per-run flag to skip AGENTS.md discovery; and model-ID substring routing can silently change which embedded stock prompt is used.

## Qwen Code

Qwen Code is a strong native fit for Claudine's append/replace contract, with one delivery caveat: the documented prompt flags accept inline text, not files.

For append, Qwen exposes `--append-system-prompt <TEXT>`. This appends extra instructions to the main-session prompt for the current invocation. Its ordering matters: Qwen loads the built-in or replacement base prompt, then memory/context such as `QWEN.md`, then the append flag content. That means Claudine append maps directly, but not to an arbitrary position inside Qwen's prompt stack.

For replacement, Qwen exposes `--system-prompt <TEXT>`. This replaces the built-in main-session system prompt for the run. It is not whole-context isolation: `QWEN.md`, managed memory, extension context, git-status reminders, and explicit appended prompt text can still follow the replacement.

Qwen also has an implementation-supported file replacement path through `QWEN_SYSTEM_MD`. Setting it to `1` or `true` reads `.qwen/system.md`; setting it to a path reads that file. This is useful as a fallback for large Claudine replacement prompts that might exceed argv limits, especially on Windows, but it is less prominent in user-facing docs and has stricter failure behavior: a missing replacement file is fatal. `QWEN_WRITE_SYSTEM_MD` can export the rendered base prompt for inspection, but it writes to disk and should not be used for normal wrapper delivery.

Other prompt-affecting layers include global and project `QWEN.md`, managed memory, settings, extensions, skills, MCP servers, subagents, `--include-directories`, and `--all-files`. `--safe-mode` and `-e none` are useful when Claudine needs a more deterministic run because they suppress many discovery/customization layers while leaving explicit `--system-prompt` and `--append-system-prompt` active.

For Claudine, the normal mapping should be:

- Append: read the file-backed Claudine prompt and pass its contents via `--append-system-prompt`.
- Replace: read the file-backed Claudine prompt and pass its contents via `--system-prompt`.
- Large replacement fallback: write a temporary Markdown file and launch with `QWEN_SYSTEM_MD=<temp-file>`.
- Large append fallback: there is no native append-file equivalent; use inline text when possible, or a controlled temporary `QWEN.md`/shadow `QWEN_HOME` strategy only when file-backed appended context is more important than exact append-flag semantics.

Claudine should not mutate the user's `~/.qwen/settings.json`, `~/.qwen/QWEN.md`, project `QWEN.md`, or project `.qwen/system.md` for wrapper prompt delivery.

## Point Of View

Claudine's uniform interface is the right abstraction, but it should be understood as a behavioral contract rather than a promise that every provider has identical native semantics.

The best native fits are Claude Code and Qwen Code, but they are not identical. Claude is both semantically and file-delivery friendly because it has prompt-file flags. Qwen has equally direct append/replace semantics for the main session, but its documented flags are inline-text only, so Claudine's file-backed interface maps by reading the file and passing text on argv. For large replacement prompts, Qwen's `QWEN_SYSTEM_MD` gives Claudine a file-backed fallback; for large append prompts, Qwen has no equivalent append-file mechanism.

Codex is a good operational fit, but append is implemented as a developer-role instruction layer, and replacement is explicitly discouraged by the provider for normal use. Goose is a strong fit for non-interactive append but not for uniform replacement: its native CLI append is `goose run`-only and inline-only, while replacement is either persistent template configuration, ACP-only, or an undocumented internal file path.

Gemini, Kimi, and OpenCode require broader translation. Gemini append means context-file discovery, and replace means a custom system file. Kimi is file-native for append through AGENTS.md, but replacement is implementation-sensitive: legacy `kimi-cli` can replace through an agent spec, while current Kimi Code has no documented per-session replacement mechanism. OpenCode is more capable than a pure workaround, but its fit is still indirect: append should be delivered as a temporary `instructions` file through `OPENCODE_CONFIG_CONTENT`, while replace should be delivered as a temporary primary agent whose `prompt` replaces only slot 0 of the assembled system text.

The safest default remains append. Replacement should be treated as an advanced, provider-sensitive mode because on several providers it drops valuable built-in behavior, shifts execution into an agent-spec path, requires configuration isolation, or replaces only the provider's base prompt rather than all later instruction context.

That last distinction is important for Claudine's documentation: `--replace-system-prompt` means "replace the provider's base system prompt where possible," not "guarantee this is the only instruction context the model receives." Context files, managed memory, skills, MCP instructions, extensions, environment sections, subagent prompts, and provider-managed policy may still apply after replacement.

Claudine's file-backed `--append-system-prompt` and `--replace-system-prompt` design still pays off. Users get one stable interface, prompt bodies can be composed through the same Markdown pipeline, and Claudine can choose the least invasive native delivery mechanism per provider. The implementation burden belongs in Claudine's provider specs, not in every user's shell scripts.
