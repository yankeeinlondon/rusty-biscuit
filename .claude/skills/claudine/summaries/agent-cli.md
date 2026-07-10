# CLI Identity and Installation Facts for Agentic Providers

Claudine wraps other agentic CLIs, so provider identity is operational state, not descriptive metadata. A wrapper cannot preflight "Claude" or "Gemini" in the abstract; it has to find a concrete executable, confirm that executable is the intended provider rather than a stale alias or legacy collision, and know which command shape can run without a TTY.

These facts matter in four places.

First, binary discovery depends on exact command identity. The user-facing command is usually short and stable on macOS and Linux, but Windows may expose native `.exe` files, npm `.cmd` and `.ps1` shims, package-manager wrappers, or PowerShell installer paths. Several providers also have package names that do not match their runtime command.

Second, install guidance has to be provider-specific. A failed `codex` probe should not produce generic "install the CLI" advice. It should point to the provider's current recommended installer for the user's OS, then secondary methods such as npm, Homebrew, WinGet, Chocolatey, Scoop, pnpm, Bun, Docker, or release archives where those are supported.

Third, documentation links are part of wrapper UX and maintenance. When Claudine cannot safely infer behavior, it needs stable links to provider homepages, repositories, install docs, CLI references, and diagnostics docs. Those links also give maintainers a fast path for refreshing stale catalog facts.

Fourth, switch synthesis only works when Claudine knows each provider's command-line grammar. Non-interactive execution, model selection, sandboxing, approval mode, MCP selection, JSON/JSONL output, system-prompt delivery, config overrides, image input, profile/session controls, and isolation all map differently. Claudine can expose normalized concepts, but the emitted argv must respect the provider's native command position, flag spelling, value shape, defaults, and conflicts.

This summary reflects the `agent-cli` research set: Claude Code, Codex, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and the researched onboarding providers Kilo and Pi. Roo is in Claudine's wider provider story, but this directory does not currently contain a fresh Roo CLI identity fact sheet, so this document does not make Roo-specific CLI assertions.

## Provider Shape

Claude Code, Codex, and Gemini show the baseline pattern.

Claude Code exposes `claude` on macOS and Linux, with Windows variants such as `claude.exe`, `claude`, or npm shims like `claude.cmd`. It has a first-party native installer, Homebrew cask, WinGet, Linux package-manager paths, and npm. Native installs auto-update; package-manager installs generally rely on the package manager unless users opt into package-manager auto-update. Its primary automation path is `claude --print` / `claude -p`.

Codex exposes `codex` as the official command, though release archives may contain platform-named assets that users rename or install as `codex`. It supports standalone installers, npm, Homebrew, and direct GitHub release downloads. Standalone installs are upgraded by rerunning the installer or using `codex update`; package-manager installs follow their package manager. Its primary automation path is `codex exec`, with strong machine introspection through `codex doctor --json`.

Gemini exposes `gemini`, with npm shims such as `gemini.cmd` and `gemini.ps1` on Windows. Its install story is npm-centered, with Homebrew/Linuxbrew, MacPorts, Docker images, source-tree execution, and `npx @google/gemini-cli` as additional paths. Its primary headless path is prompt mode, especially `gemini -p` / `--prompt`, with version drift between docs, installed help, and upstream latest a recurring caveat.

The remaining providers broaden the pattern.

Goose exposes `goose` on macOS and Linux and `goose.exe` in native Windows assets, with `goose` still the command users normally type when the directory is on `PATH`. It supports the official shell installer, Homebrew `block-goose-cli`, Windows PowerShell installation, Git Bash/MSYS2 installation on Windows, and WSL. Its installer may launch interactive configuration by default; CI and wrappers should use `CONFIGURE=false`. The primary automation path is `goose run`, preferably `goose run --output-format stream-json --no-session -t <prompt>` for live structured output.

Kimi Code exposes `kimi`; `kimi-code` is a package/formula name, not the normal runtime command. `kimi-cli` is a legacy Python command with a different surface and should be treated as a collision, not a safe alias. Kimi installs through shell and PowerShell installers, Homebrew `kimi-code`, npm `@moonshot-ai/kimi-code`, and pnpm. Windows requires Git for Windows or `KIMI_SHELL_PATH` for shell execution. Its primary automation path is `kimi -p <prompt>` / `kimi --prompt <prompt>`.

OpenCode exposes `opencode` on macOS, Linux, and Windows, with Windows variants such as `opencode.exe` and npm/package-manager shims. Its install surface is broad: official curl installer, npm, Bun, pnpm, Yarn, Homebrew tap, Arch packages, Docker, Chocolatey, Scoop, and Mise. Its primary automation path is `opencode run`; `opencode run --format json` emits NDJSON events.

Qwen Code exposes `qwen`, with npm shims such as `qwen.cmd` and `qwen.ps1` on Windows. It installs through standalone shell/PowerShell installers, npm `@qwen-code/qwen-code`, and Homebrew on macOS/Linux. Current npm releases require Node.js 22 or newer. Its default command is dual-mode: no prompt launches the TUI, while a positional prompt or `--prompt` runs headless. New wrapper calls should prefer positional prompts because `--prompt` is deprecated in current research.

Kilo exposes `kilo` with `kilocode` as an alternate npm bin. Windows npm installs normally expose `kilo.cmd`, `kilocode.cmd`, `kilo.ps1`, and `kilocode.ps1`; standalone archives may contain `kilo.exe`. It installs through npm, pnpm, Bun, Homebrew, curl installer, GitHub release archives, and Arch AUR. Its primary automation path is `kilo run --auto --format json <message>`.

Pi exposes `pi`, with Windows npm shims such as `pi.cmd` and `pi.ps1`. The current official package is `@earendil-works/pi-coding-agent`; research found a stale local `pi` shim from the older `@mariozechner/pi-coding-agent` namespace, so binary discovery should verify package identity where possible. Pi's primary install path is npm with `--ignore-scripts`, plus pnpm, Bun, Unix shell installer, and Windows PowerShell installer. Its automation modes are `pi -p` / `pi --print`, `pi --mode json`, and `pi --mode rpc`.

## Provider Differences

Binary names are the most stable identity facts, but aliases and shims are OS-sensitive. Claudine should catalog canonical command names and known alternate executable names per OS. The catalog should also distinguish package names from runtime commands: `@moonshot-ai/kimi-code` installs `kimi`, `@kilocode/cli` exposes `kilo` and `kilocode`, `@earendil-works/pi-coding-agent` exposes `pi`, and npm or PowerShell may add platform shims.

Install methods differ too much for a single `install_url`. Claude has native installers and package-manager paths. Codex has standalone installers, npm, Homebrew, and archives. Gemini is npm-centered but supports several secondary paths. Goose has shell/PowerShell installers and Homebrew. OpenCode and Kilo have especially broad package-manager coverage. Pi asks npm users to pass `--ignore-scripts`. Qwen and Kimi require modern Node versions for npm installs. Claudine needs install records keyed by provider, OS, and method type.

Auto-update behavior is install-method-specific, not provider-global. Claude native installs auto-update, while package-manager installs usually do not. Codex standalone upgrades are explicit. Goose exposes `goose update`, but it mutates the installation and may reconfigure. Kimi has update metadata and no-auto-update env vars. OpenCode has `OPENCODE_DISABLE_AUTOUPDATE`. Gemini, Kilo, Pi, and Qwen all expose update or upgrade surfaces, but those are not equivalent to a safe wrapper-time auto-update policy. Package-manager freshness also drifts independently from upstream latest.

Version discovery is usually stable enough to probe with `--version`, but the result does not prove current switch compatibility. Gemini, Kilo, Kimi, Pi, and Qwen research all found local installations older than upstream latest or package-manager stable. Kimi also showed documented JSON subcommands rejected by the older installed binary. The catalog should store version probe commands, not bake in "latest version" values as durable logic.

Documentation surfaces are relatively stable and worth compiling: homepage, repository, general docs, CLI reference, install docs, and sometimes release pages. They change less often than switch inventories and are useful for install guidance, error messages, and metadata refresh.

Switch inventories differ the most. Claude uses `--print`; Codex uses `exec`; Goose, OpenCode, and Kilo use `run`; Gemini, Qwen, Kimi, and Pi use prompt or mode flags on the top-level command. JSON output can mean final JSON, JSONL, NDJSON, stream JSON, or RPC JSONL depending on provider. Permission bypasses also vary: `--auto`, `--yolo`, `--approval-mode`, sandbox flags, tool allow/deny flags, and provider-specific safe, bare, pure, or isolated modes are not interchangeable.

Machine introspection is uneven. Codex has the strongest all-up probe with `doctor --json`. OpenCode has useful JSON for `debug config`, `debug skill`, `session list --format json`, DB queries, and OpenAPI generation. Kimi exposes provider/model catalog JSON and server schemas. Kilo exposes several JSON diagnostics but also has partly structured text. Goose has JSON sessions, recipes, and run output but no single `doctor --json`. Qwen has JSONL sessions and possible daemon capabilities, but many lists are text-only. Pi's strongest introspection is RPC mode; its simple list commands are mostly text.

Help output is not the whole truth. Research repeatedly found documented flags missing from local help, local commands missing from official docs, compact help omitting parser-supported flags, installed versions lagging current documentation, and local binaries accepting hidden commands. Claudine should record provenance for every fact: official docs, local help, local probe, package registry, release metadata, or source inspection.

## Stable Catalog Facts

These facts are stable enough to compile into Claudine's provider catalog:

- Canonical provider id and display name.
- Canonical binary name per OS.
- Known alternate binary names, package-manager shims, and legacy-collision warnings per OS.
- Homepage, repository, docs, install docs, CLI reference, and release URLs.
- Primary non-interactive entry point, such as `claude --print`, `codex exec`, `goose run`, `opencode run`, `kilo run`, `pi --print`, or prompt-mode top-level commands.
- Basic version probe command.
- Supported install method families per OS.
- Stable config and state isolation knobs such as `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`, `GOOSE_PATH_ROOT`, `KIMI_CODE_HOME`, `OPENCODE_CONFIG_DIR`, `PI_CODING_AGENT_DIR`, and `QWEN_HOME`; Kilo should be represented through its XDG-style paths and isolation controls rather than a single root override.
- Known structured introspection commands with explicit machine-readable contracts.
- High-level switch categories Claudine needs to synthesize: prompt, model, output format, session, sandbox, approval, tool restrictions, MCP, config override, system prompt, image/file input, profile, and state isolation.

These should be stored as structured metadata, not prose, so Claudine can use them for discovery, preflight checks, install guidance, help rendering, and wrapper argv generation.

## Facts That Drift Too Fast

These facts should be refreshed from research or runtime probes instead of treated as permanent truths:

- Latest upstream version numbers.
- Complete switch inventories.
- Exact help text.
- Hidden, preview, or experimental subcommands.
- Deprecated aliases and docs-only flags.
- Auto-update behavior for package-manager installs.
- Package-manager availability and freshness by channel.
- Local install paths.
- Platform release asset names.
- Full config schemas.
- Experimental ACP, daemon, app-server, editor, plugin, extension, and background-session controls.
- Whether a documented flag is accepted by a particular installed version.
- Whether local help includes all parser-supported flags.
- Whether a management command is truly non-interactive on the installed version.

Claudine can cache these facts with provenance and `last_verified` dates, but wrapper logic should tolerate drift. Probe installed binaries, bound management commands with timeouts, prefer JSON/JSONL when available, and emit actionable remediation when a flag is rejected.

## Point of View

Claudine should treat CLI identity as a two-layer catalog.

The first layer is durable provider identity: provider id, command names, OS aliases, install method families, docs links, config roots, version probes, and primary automation entry points. This belongs in the compiled provider catalog because it directly powers binary discovery, preflight checks, install guidance, docs links, and default wrapper behavior.

The second layer is observed CLI surface: latest versions, full switch catalogs, hidden commands, update mechanics, local quirks, version-gated JSON surfaces, help/doc mismatches, and package-manager freshness. This belongs in research-backed metadata with timestamps and provenance. Claudine should use it for guidance and wrapper support, but remain defensive at runtime.

The provider catalog should therefore be opinionated about identity and cautious about inventory. Binary names, docs links, config roots, version probes, and non-interactive entry points are stable enough to compile. Full flag catalogs, update behavior, package-manager state, preview command surfaces, and version-specific management commands drift too quickly and should be refreshed continuously from the `agent-cli` research documents and local probes.
