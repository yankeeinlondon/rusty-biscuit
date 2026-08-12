# Agentic CLI Permission Models and Claudine Policy Normalization

Claudine wraps provider CLIs that were designed around different assumptions about trust, interactivity, and local control. Permission normalization matters because Claudine needs predictable behavior when the same high-level policy is applied to Claude, Codex, Gemini, Goose, Kimi, OpenCode, or Qwen.

This report covers the wrappable provider CLIs represented by the current agent-permissions research set: Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, and Qwen Code. Roo Code is present in Claudine’s provider enum, but it is a VS Code extension rather than a normal wrapper target and was not part of the current agent-permissions research set. Pi and Kilo Code were researched ahead of current compiled support, so they are excluded from the supported-provider fit rankings below.

The first requirement is predictable non-interactive execution. A headless wrapper cannot depend on an approval dialog appearing in a terminal. If a provider turns “ask” into “deny” in headless mode, auto-approves in print mode, auto-replies only once, or exposes a programmatic approval channel only through a server protocol, Claudine has to know that before launch. Otherwise a run can hang, fail unexpectedly, or perform more work than the caller intended.

The second requirement is an honest safety posture. “Do not ask” is not the same as “deny.” “YOLO” is not the same as “sandboxed.” A provider may auto-approve tool calls while still running shell commands with the invoking user’s full filesystem and network privileges. Conversely, a provider may have an OS/container sandbox that limits command effects even when the approval mode is permissive. Claudine’s PolicyEngine has to preserve that distinction instead of flattening everything into one “safe” or “unsafe” switch.

The third requirement is policy portability. Claudine users should be able to say “read-only,” “ask before edits,” “allow this command,” “deny this path,” or “disable MCP except this server” once, then have Claudine project that intent into provider-native switches, config overlays, environment variables, or generated temporary files. Perfect portability is not possible, but the wrapper can still provide a stable policy vocabulary if it knows which provider semantics are exact, approximate, or unsupported.

## How Providers Differ

### CLI Switches and Environment Variables

Claude Code has the most direct CLI vocabulary for permission policy. `--permission-mode` selects a coarse mode, `--allowedTools` and `--disallowedTools` add session rules, `--tools ""` can remove built-in tools, `--bare` and `--safe-mode` reduce customization loading, and `--dangerously-skip-permissions` enters bypass mode. Its `--permission-prompt-tool` is especially relevant for non-interactive automation because it can route permission prompts to an MCP tool.

Codex splits the problem into sandbox and approval flags. `--sandbox` controls filesystem/process isolation posture, while `--ask-for-approval` controls when the CLI asks. `--dangerously-bypass-approvals-and-sandbox` combines full access with no prompts. `-c` config overrides and `--profile` give Claudine a clean way to apply session-scoped policy without editing user files. For `codex exec`, `--ignore-user-config` and `--ignore-rules` are important because persisted user config and execpolicy rules otherwise affect headless behavior.

Gemini and Qwen expose similar families of controls: approval mode, sandbox enablement, tool include/exclude filters, MCP server filters, extension selectors, safe/bare mode, and headless output modes. Gemini has `--approval-mode`, `--sandbox`, `--policy`, `--admin-policy`, `--allowed-mcp-server-names`, deprecated `--allowed-tools`, `--exclude-tools`, and extension controls. Qwen has `--approval-mode` with `plan`, `default`, `auto-edit`, `auto`, and `yolo`; `--yolo`; `--allowed-tools`; `--exclude-tools`; `--safe-mode`; `--bare`; `--sandbox`; `--allowed-mcp-server-names`; `--mcp-config`; `--extensions`; and `--disabled-slash-commands`. Qwen also adds unattended-run guardrails that matter for wrappers, especially `--max-tool-calls 0`, `--max-wall-time`, `--max-session-turns`, and `--max-subagent-depth`.

Qwen’s environment surface is also policy-relevant. `QWEN_HOME` can relocate user settings and state, `QWEN_RUNTIME_DIR` separates runtime output, `QWEN_CODE_SAFE_MODE` enables safe mode, `QWEN_SANDBOX` can enable or disable sandboxing or force `docker`, `podman`, or `sandbox-exec`, and `QWEN_SANDBOX_IMAGE` supplies the container image unless `--sandbox-image` is passed. `QWEN_CODE_SYSTEM_SETTINGS_PATH`, `QWEN_CODE_SYSTEM_DEFAULTS_PATH`, and `QWEN_CODE_TRUSTED_FOLDERS_PATH` redirect policy-bearing files. `QWEN_DISABLED_SLASH_COMMANDS` extends slash-command denial. `QWEN_CODE_SUPPRESS_YOLO_WARNING` only suppresses the headless YOLO-without-sandbox warning; it does not change permissions.

Goose is much less CLI-driven for approval. Its primary approval selector is `GOOSE_MODE`, with `auto`, `approve`, `smart_approve`, and `chat`; there is no native `--mode`, `--approval-mode`, `--yolo`, `--allowed-tools`, or `--disallowed-tools` flag. CLI switches mostly shape the visible extension/tool surface and adjacent execution posture: `--no-profile`, `--with-builtin`, `--with-extension`, `--with-streamable-http-extension`, `--container`, `--interactive`, `--no-session`, `--max-tool-repetitions`, `--max-turns`, and `--debug`. This means a Claudine launch policy for Goose must be expressed through environment variables, extension selection, and temporary config roots more than argv permission rules.

Kimi is mode-driven rather than rule-driven. `--yolo`, `--afk`, `--plan`, `--print`, `--config`, `--config-file`, `--agent`, `--agent-file`, `--mcp-config-file`, `--mcp-config`, `--skills-dir`, and `--acp` are the relevant controls. `KIMI_SHARE_DIR` changes the config, MCP, session, and credential root, while provider/model env vars such as `KIMI_API_KEY`, `KIMI_BASE_URL`, and `OPENAI_API_KEY` do not grant tool permissions. There is no native CLI deny-all rule grammar. The closest precise restriction is to use a generated agent file to reduce tool visibility, a temporary config to avoid permissive defaults, and optionally PreToolUse hooks for runtime blocking.

OpenCode has a compact provider-native policy overlay through `OPENCODE_PERMISSION` and `OPENCODE_CONFIG_CONTENT`. Its run-time flags include `--auto`, hidden YOLO aliases `--yolo` and `--dangerously-skip-permissions`, `--agent`, `--dir`, `--attach`, `--username`, `--password`, and global `--pure`. Because `OPENCODE_PERMISSION='{"*":"deny"}'` can create a session-scoped deny-all baseline, OpenCode is one of the easier providers for Claudine to lock down without mutating user config. It still lacks a first-class `run` CLI allowlist or `--no-tools` switch, so adding permissions back requires encoding the whole overlay in `OPENCODE_PERMISSION` or `OPENCODE_CONFIG_CONTENT`.

### Config Files and Scope

Claude, Codex, Gemini, Qwen, and OpenCode all have both user and repo/project configuration surfaces. These are the easiest providers for Claudine to reason about structurally, but they differ in merge behavior.

Claude uses user, shared project, local project, managed, and CLI/session settings. Permission arrays merge across scopes, and deny rules beat allow rules. Managed settings can lock behavior above user and project choices.

Codex uses `$CODEX_HOME/config.toml`, profile files, project `.codex/config.toml`, rules directories, managed requirements/defaults, and CLI `-c` overrides. Project config loads only for trusted projects. Managed requirements constrain lower sources rather than merely overriding them.

Gemini and Qwen both have user/project settings plus system defaults and system overrides. They also have policy or rule layers, extension-provided surfaces, MCP settings, and safe/trust gates that can disable project-local behavior. Gemini’s TOML policy tiers are particularly complex. Qwen’s main settings file is `.qwen/settings.json` at user and project scope, with system defaults and system overrides in platform-specific locations. Its settings merge deeply, permission arrays merge by decision type, and runtime conflict priority is deny, then ask, then allow, then mode/default behavior.

Qwen’s source-loading gates are important. `--safe-mode` and `QWEN_CODE_SAFE_MODE` disable settings-sourced permission rules, MCP servers, extensions, skills, hooks, memory features, custom subagents, and sandbox settings, while still honoring explicit CLI approval flags such as `--approval-mode`, `--yolo`, `--allowed-tools`, and `--exclude-tools`. `--bare` is even narrower: it skips implicit startup auto-discovery and honors only explicit CLI inputs. Folder trust can ignore project-local surfaces and force privileged approval modes down to `default`. CLI-injected MCP servers via `--mcp-config` sit above settings and project `.mcp.json` and are not gated by project MCP approval; `--allowed-mcp-server-names` overrides settings-level MCP allow/exclude filters for that session.

OpenCode has user JSON/JSONC config, repo `opencode.json` or `opencode.jsonc`, `.opencode` directories, agent frontmatter, managed config, macOS MDM preferences, console account config, custom config via `OPENCODE_CONFIG`, config-directory override via `OPENCODE_CONFIG_DIR`, inline config via `OPENCODE_CONFIG_CONTENT`, and inline permission overlay via `OPENCODE_PERMISSION`. Project config can be disabled with `OPENCODE_DISABLE_PROJECT_CONFIG`, and `--pure` or `OPENCODE_PURE` disables external plugins for the process. Its effective rule order matters because later matching rules can override earlier ones, and public legacy permission objects are migrated into ordered internal rules.

Goose is user-scoped for permissions. Its user config resolves through platform config directories, with current macOS source using `Library/Application Support/Block/goose/config.yaml`, Linux using `.config/goose/config.yaml`, and Windows using `AppData\Roaming\Block\goose\config\config.yaml`. `permission.yaml` lives next to `config.yaml` and stores exact-name tool decisions plus SmartApprove cached classifications. System config and `GOOSE_ADDITIONAL_CONFIG_FILES` can provide lower-precedence defaults, but no built-in repo-scoped permission file or immutable managed policy layer was found. `GOOSE_PATH_ROOT` is therefore important for Claudine because it can move Goose config, data, state, plugin, and agent paths for an isolated wrapper run.

Kimi is also more user-scoped. It uses `~/.kimi/config.toml`, `~/.kimi/mcp.json`, and per-session state under `~/.kimi/sessions/.../state.json`; `KIMI_SHARE_DIR` can relocate that whole tree. No repo-scoped permission config, folder trust gate, managed policy layer, or native safe mode was found for the researched Kimi CLI target.

### Defaults When Unspecified

The defaults vary enough that Claudine cannot infer policy from provider name alone.

Claude defaults to allowing read-only exploration and asking for state-changing tools such as Bash, edits, writes, web access, and other mutating actions.

Codex defaults depend on surface and trust. Interactive sessions tend toward Auto for trusted version-controlled folders, while `codex exec` defaults to read-only sandboxing unless config or flags grant more.

Gemini defaults to read/search/context tools allowed, mutating tools and shell asking interactively, and approval-required actions denied in headless mode. Sandboxing is off by default.

Qwen defaults to Ask Permissions mode, represented as `tools.approvalMode: "default"`. Read-only and metadata tools run without confirmation. Read-only shell commands may be auto-allowed by shell analysis. Risky shell, edit, network, MCP, and subagent actions ask or follow their tool defaults. Sandboxing is separate and is not implied by the default approval mode.

Goose defaults to `auto` in current source, so visible tools are auto-approved. Its default visible surface includes default-enabled built-in extensions such as developer, analyze, todo, apps, extensionmanager, summon, top-of-mind, and skills. This is one of the most permissive defaults among the researched providers and makes wrapper-side normalization important.

Kimi defaults to an interactive approval runtime where read/search/fetch-style tools generally run, while Shell, writes, task stopping, plan exit, and MCP tools ask. Sensitive-file filters apply at the tool layer for targets such as dotenv files, SSH private keys, and cloud credentials, but there is no OS sandbox and no static rule file.

OpenCode’s default build agent is permissive: most actions allow, `external_directory` and `doom_loop` ask, and question/plan control permissions vary by active agent. Current source asks before reading `.env` and `.env.*` files by default while allowing `.env.example`; older “deny `.env`” descriptions are stale.

### YOLO and Auto-Approval

Every researched supported provider has some YOLO or auto-approval path, but the semantics differ.

Claude’s `bypassPermissions` skips most prompts and safety checks, but some circuit breakers and MCP user-interaction requirements can still prompt. It may be refused when running as root on macOS/Linux outside a recognized sandbox.

Codex’s `--dangerously-bypass-approvals-and-sandbox` is the clearest “true YOLO” flag: it disables both approval prompts and sandboxing. The equivalent manual combination is full-access sandbox mode plus never approval.

Gemini and Qwen have explicit YOLO approval modes. They auto-approve tool calls but do not automatically enable sandboxing. Admin settings, safe mode, trust gates, explicit denies, tool budgets, and sandbox failures can still constrain behavior.

Qwen’s YOLO can be entered with `--yolo`, `--approval-mode yolo`, `/approval-mode yolo`, keyboard mode cycling, or `tools.approvalMode: "yolo"` in settings. In headless mode, YOLO without sandbox prints a warning unless `QWEN_CODE_SUPPRESS_YOLO_WARNING` is set. That warning is diagnostic only. Qwen also has `auto` mode, but it is classifier-driven rather than a deterministic allow rule: over-broad allow rules may be stripped while Auto mode is active, and the runtime can fail closed or fall back to manual approval after repeated blocked or unavailable results.

Goose `auto` mode auto-approves all visible tools and ignores `permission.yaml` user rules. It can be selected through `GOOSE_MODE=auto`, config, or the interactive `/mode auto` command, and it is also the current source default. It is not coupled to OS isolation. `approve` and `smart_approve` are less permissive, but in non-interactive `goose run` any action that still needs user approval has no documented programmatic approval channel; a wrapper must preconfigure rules, switch to `chat`, or use `--interactive` when human approval is expected.

Kimi separates `yolo`, `afk`, and `print`. `yolo` auto-approves regular tool actions while leaving some user-question behavior reachable, and `ExitPlanMode` still asks under yolo. `afk` is more unattended: user questions auto-dismiss and plan exit auto-approves. `--print` is the intended non-interactive path and applies runtime AFK, but that runtime AFK does not persist. Approve-for-session choices persist as action strings in session state, so resumed sessions can carry prior approvals.

OpenCode `--auto` and its hidden YOLO aliases auto-reply once to promptable permission requests. Explicit deny still blocks. Because OpenCode has no sandbox, auto mode is approval-only, not isolation. Attached runs are different again: `opencode run --attach` delegates permission prompts and saved approvals to the existing server/session, authenticated with `--username`, `--password`, or the `OPENCODE_SERVER_*` variables.

### Sandbox and Approval Coupling

Codex has the cleanest explicit coupling because approval policy and sandbox mode are separate first-class launch axes, and its dangerous bypass flag intentionally disables both.

Claude has both permission rules and a separate sandbox subsystem for Bash subprocesses. Its permission model is strong, but sandbox details remain separate from allow/ask/deny rules and include administrative controls.

Gemini and Qwen both have optional sandboxing with macOS Seatbelt and container backends. Their approval modes do not imply sandboxing, and sandbox configuration can include images, mounts, network profiles, proxy commands, and failure behavior. In Qwen, `QWEN_SANDBOX` can override both CLI and settings for sandbox enablement, while sandbox image precedence is `--sandbox-image`, then `QWEN_SANDBOX_IMAGE`, then `tools.sandboxImage`. This is richer than PolicyEngine’s current static permission axes.

Goose has optional Desktop macOS sandboxing via `GOOSE_SANDBOX` and related sandbox environment variables, plus `--container` for running stdio and built-in extension processes inside a Docker container. These are separate from `GOOSE_MODE`. The Desktop sandbox is not the ordinary CLI approval model, and `--container` isolates extension processes rather than the whole Goose CLI or every tool effect. Ordinary Goose permissions are client-side approval and exact tool-name rules, not an OS-enforced sandbox.

Kimi and OpenCode do not provide an OS-enforced sandbox in the researched targets. Kimi’s source prompt explicitly says the operating environment is not sandboxed; Shell runs with the launching user’s host permissions, and network access is not sandboxed. Kimi adds client-side approval prompts, session approval state, agent tool visibility, fail-open PreToolUse hooks, and tool-layer sensitive-file filters. OpenCode adds ordered allow/ask/deny rules and runtime prompt replies. In both cases, strong isolation must come from an external container, VM, OS sandbox, or wrapper.

## Provider Fit for Claudine PolicyEngine

### Claude Code

Claude is the best fit for Claudine’s current PolicyEngine axes. It has native allow/ask/deny rules, tool and MCP tool matchers, command/path specifiers, permission modes, CLI session overlays, agent-scoped permissions, and meaningful safe/bare modes.

The fit is still imperfect. Auto mode is classifier-driven, protected-path circuit breakers are hard-coded, hooks can preempt permission evaluation, and sandbox policy is separate from static permission rules. Claudine should treat Claude as high-coverage but not fully lossless.

### Codex

Codex fits Claudine well for high-level posture: read-only, workspace-write, full-access, ask-on-request, never-ask, and YOLO are all expressible. It is strong for predictable non-interactive runs because `codex exec` can ignore user config and rules.

The poor fit is detail. Codex has beta permission profiles, Starlark execpolicy rules, managed requirements, granular approval categories, feature flags, MCP tool approval settings, and custom agents that can override runtime config. PolicyEngine can represent the headline posture but not the full native model without provider-specific extensions.

### Gemini CLI

Gemini is powerful but awkward for PolicyEngine. It has approval modes, policy files, tool filters, MCP filters, extension controls, sandboxing, trust, and admin policy. That gives Claudine many control points.

The poor fit is that Gemini’s policy tiering, `ask_user` headless behavior, regex argument matchers, safety checker hooks, dynamic settings-derived priority bands, and sandbox expansion are not flat allow/ask/deny rules. PolicyEngine can launch conservative Gemini sessions, but exact round-tripping would require a Gemini-specific policy backend.

### Goose

Goose is a poor fit for fine-grained PolicyEngine projection today. Its native rule model is a single user-scoped `permission.yaml` with exact exposed tool-name lists: `always_allow`, `ask_before`, and `never_allow`. Those rules are consulted in `approve` and `smart_approve`, but ignored in `auto`. There is no CLI approval-mode flag, no repo-scoped permission file, no deny-all-and-add-back CLI grammar, and no command/path/domain matcher grammar.

Claudine can still normalize coarse modes: `chat` for no tool execution, `approve` for prompt-first behavior, `smart_approve` for read-only/classifier-assisted approval, and `auto` for YOLO. It can also control the visible tool surface with `--no-profile`, explicit extension flags, extension `available_tools`, `GOOSE_ALLOWLIST`, and temporary roots via `GOOSE_PATH_ROOT`. But this is mode and tool-visibility normalization, not full static policy normalization. A Goose backend should report fine-grained command/path policy as unsupported or approximate, and it should model Desktop sandbox and extension-container posture as separate axes from approval.

### Kimi

Kimi is a poor fit for static PolicyEngine rules. Its controls are runtime modes, session-persisted approval actions, agent YAML tool visibility, MCP loading, and hooks. It lacks a native static allow/ask/deny grammar, a CLI deny-all baseline, repo-scoped permission config, managed policy, folder trust, safe mode, and an OS sandbox.

The most useful Claudine mapping is through generated temporary config, generated agent files, invocation-scoped MCP config, and optional PreToolUse hooks. That projection is approximate: hooks fail open on timeout/crash/engine error, plan mode is workflow state rather than a strict permission boundary, and approve-for-session state can change future resumed behavior. ACP is the one precise programmatic approval transport: Kimi forwards permission requests to the ACP client with approve once, approve for this session, and reject options. PolicyEngine can express the user intent, but an accurate Kimi backend needs first-class axes for tool visibility, runtime approval state, session persistence, and external sandboxing.

### OpenCode

OpenCode fits the allow/ask/deny part of PolicyEngine better than many providers because it has explicit permission rules and a clean environment overlay. `OPENCODE_PERMISSION='{"*":"deny"}'` is a strong session-scoped lockdown primitive, and `OPENCODE_CONFIG_CONTENT` can carry a fuller one-process overlay for permissions, agents, MCP, plugins, and related security controls. `--pure` is useful alongside those overlays because it removes external plugin tools and hooks from the process.

The fit is poor around isolation, ordering, and execution context. OpenCode has no OS sandbox, public legacy permission grammar is migrated into ordered internal rules, tool visibility is derived from whole-tool deny rules rather than a separate allowlist, MCP resources use `read` patterns such as `mcp:server:resource`, and saved approvals can persist by project or live in an attached server session. Agent and subagent permissions also inherit parent denies and external-directory rules, so they are not simple per-agent overrides. Claudine should treat OpenCode as good for policy projection but weak for safety enforcement unless paired with external sandboxing.

### Qwen Code

Qwen is a medium-to-good fit for Claudine’s PolicyEngine. It has native approval modes, allow/ask/deny permission arrays, session-scoped `--allowed-tools` and `--exclude-tools`, agent/subagent permission frontmatter, MCP server filters, extension controls, safe and bare modes, folder trust, and optional sandboxing. Common policies such as read-mostly, deny mutation, allow a known shell command, block MCP tools, or force a conservative headless run are expressible.

The strongest fit is at the coarse posture and rule-array level. Qwen’s `permissions.allow`, `permissions.ask`, and `permissions.deny` map naturally to PolicyEngine decisions, and runtime conflict priority is understandable: deny wins over ask, ask wins over allow, and mode/default behavior fills the rest. Agent frontmatter can also carry `approvalMode`, `tools`, and `disallowedTools`, which gives Claudine a provider-native path for subagent-scoped policy.

The gaps are in exact semantics. There is no true CLI deny-all-and-add-back primitive. `--max-tool-calls 0` is useful for a hard no-tool-execution budget, but it aborts on the first attempted tool call and cannot then selectively permit reads or one shell command. `--approval-mode plan` is conservative but still permits read/info behavior. `--exclude-tools` can deny named surfaces, aliases, command patterns, path rules, and MCP tool names, but it is not the same as starting from an empty capability set.

Qwen’s matcher model is also provider-specific. Rule targets include built-in tool aliases such as `Read`, `Edit`, and `Bash`; command patterns for `Bash(...)`, `Shell(...)`, and `Monitor(...)`; path patterns with absolute, home-relative, project-root-relative, and cwd-relative forms; MCP names such as `mcp__server` and `mcp__server__tool`; MCP resources; the `Agent` tool; and subagent names. Shell matching includes command splitting, word-boundary behavior, and virtual file/network operations. Tool visibility is separate from approval through core-tool registry filtering, MCP include/exclude filters, extensions, slash-command denial, and safe/bare source loading.

Qwen’s sandbox is another separate axis. `--sandbox`, `QWEN_SANDBOX`, `SEATBELT_PROFILE`, `SANDBOX_FLAGS`, `QWEN_SANDBOX_PROXY_COMMAND`, and image settings describe OS/container isolation rather than approval. A Qwen policy that says “YOLO in a restrictive sandbox” and one that says “YOLO with no sandbox” are materially different and must stay different in Claudine’s model.

## Point of View

Claudine should model provider permissions as several axes, not one:

- Approval decision: allow, ask, deny, auto-approve, auto-deny.
- Tool visibility: whether a tool is visible to the model at all.
- Sandbox posture: none, read-only, workspace-write, full access, provider-specific OS/container profile.
- Source loading: user config, repo config, managed policy, extensions, skills, hooks, MCP, agents.
- Non-interactive behavior: prompt, deny, auto-approve, auto-reply once, programmatic approval channel, fail, or budget-abort.
- Persistence: session-only, project, user, managed, attached server/session, or generated temporary overlay.

The provider set establishes the pattern. Claude maps closest to a portable allow/ask/deny PolicyEngine. Codex proves sandbox and approval must remain separate axes. Gemini proves config-tier and trust/sandbox behavior can be too rich for a flat policy model. Goose shows that a provider can have named allow/ask/deny lists while still being a poor fine-grained fit if the lists are user-scoped, exact-tool-only, ignored by the default auto mode, and uncoupled from sandboxing. Kimi shows the same problem from a runtime-state direction: modes, agent files, hooks, and session approvals can express intent, but not as durable static policy. OpenCode shows that a strong allow/ask/deny overlay is still not a safety boundary without isolation. Qwen gives Claudine useful session switches and rule arrays, but its real behavior also depends on source-loading gates, matcher semantics, tool visibility, unattended budgets, subagent inheritance, folder trust, and sandbox backend settings.

Across the full supported wrapper set, the best Claudine strategy is to produce provider-native overlays when exact, warn when approximate, and refuse to claim sandbox safety when the provider only supplies client-side approval. Claude, Codex, OpenCode, and Qwen can cover many PolicyEngine policies directly. Gemini can cover them with substantial provider-specific translation. Goose and Kimi require coarser mode/tool-visibility projections and should be labeled as partial coverage rather than treated as exact PolicyEngine backends; for Kimi specifically, Claudine should model generated agent files, invocation-scoped config, ACP approval transport, session approval persistence, and the absence of native sandboxing as explicit provider metadata.
