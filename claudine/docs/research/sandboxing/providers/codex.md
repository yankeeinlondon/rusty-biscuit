---
$schema: ./_schema.yaml
agent: codex
changes: []
content_policy:
- ValidFor(14d)
controls:
- effect: Select legacy local execution policy. Danger-full-access disables Codex isolation without itself selecting the approval policy.
  evidence_ids:
  - local-help
  - default-source
  - security-docs
  example: codex --sandbox read-only --ask-for-approval on-request
  id: sandbox-mode
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --sandbox
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - read-only
  - workspace-write
  - danger-full-access
- effect: Persistent legacy policy; presence in a loaded config takes priority over ordinary default_permissions selection.
  evidence_ids:
  - profiles-docs
  - config-reference
  example: sandbox_mode = "workspace-write"
  id: sandbox-config
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox_mode
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - read-only
  - workspace-write
  - danger-full-access
- effect: Independent interactive command approval policy. Never does not remove isolation; exec can use -c approval_policy instead.
  evidence_ids:
  - local-help
  - security-docs
  - headless-source
  example: codex --sandbox workspace-write --ask-for-approval never
  id: approval-mode
  kind: cli_flag
  mechanism_ids: []
  name: --ask-for-approval
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - on-request
  - never
- effect: Select approval routing/categories independently of isolation. Explicit untrusted value is retired; project-derived untrusted behavior still exists.
  evidence_ids:
  - security-docs
  - config-source
  example: approval_policy = "on-request"
  id: approval-config
  kind: config_key
  mechanism_ids: []
  name: approval_policy
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - on-request
  - never
  - '{ granular = { sandbox_approval = boolean, rules = boolean, mcp_elicitations = boolean, request_permissions = boolean, skill_approval = boolean } }'
- effect: 'Relaxation: removes Codex sandbox and command approval prompts; --yolo is alias. Does not remove an external container or automatically bypass separate hook trust.'
  evidence_ids:
  - local-help
  - security-docs
  - hooks-docs
  example: codex --yolo
  id: yolo
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --dangerously-bypass-approvals-and-sandbox
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: Select workspace-write with automatic review of approval requests; inner sandbox remains active.
  evidence_ids:
  - local-help
  - headless-docs
  - headless-source
  example: codex --approve-for-me
  id: auto-approve
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --approve-for-me
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: Choose reviewer for eligible on-request/granular approvals; automatic review adds model usage and fails closed on errors/timeouts.
  evidence_ids:
  - security-docs
  - config-reference
  example: approvals_reviewer = "auto_review"
  id: reviewer
  kind: config_key
  mechanism_ids: []
  name: approvals_reviewer
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - user
  - auto_review
- effect: Set local reviewer instructions, not OS access grants.
  evidence_ids:
  - config-reference
  - security-docs
  example: auto_review.policy = "Review requested actions against the authorized task."
  id: review-policy
  kind: config_key
  mechanism_ids: []
  name: auto_review.policy
  precedence: Managed guardian_policy_config overrides local policy. Reviewer decisions cannot override enforced requirements.
  values:
  - nonempty Markdown policy string
- effect: Select working root (-C); does not restrict all reads to that path.
  evidence_ids:
  - local-help
  - policy-source
  example: codex -C /work/project --sandbox workspace-write
  id: workspace-cwd
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --cd
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - directory path
- effect: 'Relaxation: add writable workspace roots; does not narrow reading elsewhere.'
  evidence_ids:
  - local-help
  - policy-source
  example: codex --sandbox workspace-write --add-dir /work/shared
  id: add-root
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --add-dir
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - directory path; repeatable
- effect: Add writable roots under legacy workspace-write policy.
  evidence_ids:
  - config-reference
  - default-source
  example: |-
    [sandbox_workspace_write]
    writable_roots = ["/work/shared"]
  id: legacy-roots
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox_workspace_write.writable_roots
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - array of paths
- effect: 'Relaxation when true: enable command networking; domain restrictions require an active proxy.'
  evidence_ids:
  - config-reference
  - profiles-docs
  example: |-
    [sandbox_workspace_write]
    network_access = true
  id: legacy-network
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox_workspace_write.network_access
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Exclude $TMPDIR from ambient writable roots when true; defaults false.
  evidence_ids:
  - config-reference
  - default-source
  example: |-
    [sandbox_workspace_write]
    exclude_tmpdir_env_var = true
  id: tmpdir
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox_workspace_write.exclude_tmpdir_env_var
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Exclude /tmp from ambient writable roots when true; defaults false.
  evidence_ids:
  - config-reference
  - default-source
  example: |-
    [sandbox_workspace_write]
    exclude_slash_tmp = true
  id: slash-tmp
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox_workspace_write.exclude_slash_tmp
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Select beta filesystem/network profile; danger-full-access disables local isolation.
  evidence_ids:
  - profiles-docs
  - config-source
  example: default_permissions = ":workspace"
  id: default-permissions
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: default_permissions
  precedence: Legacy sandbox settings win when mixed, except managed allowed_permission_profiles forces profiles. Managed allowlist compatibility begins 0.138.0; ordinary config precedence applies.
  values:
  - :read-only
  - :workspace
  - :danger-full-access
  - custom profile name
- effect: Define exact read/write/deny access, runtime aliases, roots and network policy. Deny globs snapshot at launch outside macOS; exact directory denials avoid new-file timing gaps.
  evidence_ids:
  - profiles-docs
  - policy-source
  example: |-
    [permissions.limited]
    extends = ":workspace"
    [permissions.limited.filesystem.":workspace_roots"]
    "secrets" = "deny"
  id: permission-profile
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: permissions
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - named profile tables with extends, workspace_roots, filesystem and network
- effect: Activate command proxy; enabling command networking and adding domain rules alone is insufficient. Client/API/MCP transport excluded.
  evidence_ids:
  - profiles-docs
  - security-docs
  example: |-
    [features]
    network_proxy = true
  id: network-proxy
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: features.network_proxy
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
  - table with enabled and domain settings
- effect: Select native Windows backend; elevated needs admin-approved setup, unelevated has weaker network/read isolation, explicit MXC is strict.
  evidence_ids:
  - windows-docs
  - windows-selection
  example: |-
    [windows]
    sandbox = "elevated"
  id: windows-backend
  kind: config_key
  mechanism_ids:
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: windows.sandbox
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - mxc
  - elevated
  - unelevated
- effect: 'Default-off standalone optional preference: compatible MXC else legacy selection/setup, never backend fallback after command failure.'
  evidence_ids:
  - features-source
  - mxc-source
  - windows-docs
  example: |-
    [features]
    prefer_mxc = true
  id: prefer-mxc
  kind: config_key
  mechanism_ids:
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: features.prefer_mxc
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Windows-only IT-led privileged provisioning; persists elevated backend, then normal users run without administrator rights.
  evidence_ids:
  - windows-docs
  example: codex sandbox setup --elevated --user 'DOMAIN\alice' --codex-home 'C:\Users\alice\.codex'
  id: windows-provision
  kind: cli_flag
  mechanism_ids:
  - windows-elevated
  name: sandbox setup --elevated
  precedence: Run in an explicitly elevated deployment process on native Windows. Documented form was not executed here; administrator restrictions still apply.
  values:
  - --user <Windows identity>
  - --codex-home <path>
- effect: Deprecated; true is rejected for filesystem-restricted policies because Landlock cannot isolate app-server Unix sockets. Not a supported equivalent replacement.
  evidence_ids:
  - linux-source
  - profiles-docs
  example: |-
    [features]
    use_legacy_landlock = false
  id: legacy-landlock
  kind: config_key
  mechanism_ids:
  - bubblewrap-seccomp
  name: features.use_legacy_landlock
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Executor-startup control. Inherit relaxes PID isolation/same-UID signaling but retains remaining sandbox layers.
  evidence_ids:
  - linux-source
  example: codex exec-server --linux-sandbox-pid-namespace=inherit
  id: pid-namespace
  kind: cli_flag
  mechanism_ids:
  - bubblewrap-seccomp
  name: --linux-sandbox-pid-namespace
  precedence: Trusted exec-server provisioning only; repository configuration and command environment cannot enable it. Updated helper must be deployed with the flag.
  values:
  - isolate
  - inherit
- effect: Mac-only default false; opt already-authorized roots beneath CODEX_HOME into following potentially external/changing symlink targets; adds no root.
  evidence_ids:
  - config-reference
  - seatbelt-source
  example: allow_symlinked_codex_home = true
  id: symlink-home
  kind: config_key
  mechanism_ids:
  - seatbelt
  name: allow_symlinked_codex_home
  precedence: Only the execution host's CODEX_HOME/config.toml at startup, not project/session override.
  values:
  - 'true'
  - 'false'
- effect: Default true; false rejects login-shell requests and defaults omitted login to non-login semantics. Does not replace OS sandbox.
  evidence_ids:
  - config-reference
  example: allow_login_shell = false
  id: login-shell
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: allow_login_shell
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - 'true'
  - 'false'
- effect: Tighten subprocess environment disclosure. ignore_default_excludes defaults true, so KEY/SECRET/TOKEN filtering is skipped unless false. Set can restore exclusions; final include filter can remove them.
  evidence_ids:
  - config-basic
  - config-advanced
  example: |-
    [shell_environment_policy]
    inherit = "core"
    ignore_default_excludes = false
    [shell_environment_policy.filters]
    "AWS_*" = "exclude"
  id: shell-env
  kind: config_key
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: shell_environment_policy
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - inherit = all | core | none
  - ignore_default_excludes = boolean
  - set = name-to-value table
  - filters = pattern-to-include-or-exclude table
- effect: Relocate CLI config/state/auth; not an automatic writable root or confidentiality control.
  evidence_ids:
  - config-advanced
  - config-reference
  example: CODEX_HOME=/work/codex-state codex --sandbox read-only
  id: codex-home
  kind: environment_variable
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: CODEX_HOME
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - directory path
- effect: Layer CODEX_HOME/<name>.config.toml; distinct from permission-profile names.
  evidence_ids:
  - local-help
  - config-basic
  - config-advanced
  example: codex --profile readonly_quiet
  id: config-file-profile
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --profile
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - config profile name
- effect: Override ordinary config for invocation; -c alias. Does not override managed constraints.
  evidence_ids:
  - local-help
  - config-basic
  - profiles-docs
  example: codex -c 'default_permissions=":read-only"'
  id: config-override
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --config
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - dotted.key=TOML-value; repeatable
- effect: Headless flag skips base CODEX_HOME/config.toml; auth still uses CODEX_HOME and this is not a universal isolation/clean-room switch.
  evidence_ids:
  - headless-docs
  - local-help
  example: codex exec --ignore-user-config --sandbox read-only "Review the repository"
  id: ignore-user
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --ignore-user-config
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: Headless flag skips user/project execpolicy rules; adjacent policy relaxation, not a sandbox disable switch.
  evidence_ids:
  - headless-docs
  - local-help
  example: codex exec --ignore-rules --sandbox read-only "Review the repository"
  id: ignore-rules
  kind: cli_flag
  mechanism_ids: []
  name: --ignore-rules
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: 'Relaxation: bypass persisted trust for enabled hooks this invocation. Hook commands remain separate host execution; distinct from YOLO.'
  evidence_ids:
  - local-help
  - hooks-docs
  - hooks-source
  example: codex --dangerously-bypass-hook-trust
  id: hook-trust
  kind: cli_flag
  mechanism_ids: []
  name: --dangerously-bypass-hook-trust
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: Separate hosted search control; does not grant shell egress. Cached is ordinary local default; full access defaults live; indexed is an additional documented index-gated option.
  evidence_ids:
  - config-basic
  - security-docs
  example: web_search = "disabled"
  id: hosted-search
  kind: config_key
  mechanism_ids: []
  name: web_search
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values:
  - cached
  - live
  - disabled
  - indexed
- effect: Change active session permissions/approval selection; plan/read-only discussion alone is not enforcement.
  evidence_ids:
  - security-docs
  example: In an interactive codex session, enter /permissions.
  id: session-permissions
  kind: session_command
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: /permissions
  precedence: Session-scoped selection subject to managed constraints; resumed sessions may preserve acknowledged authority.
  values:
  - interactive policy selection
- effect: Inspect active policy/roots; /debug-config can diagnose effective configuration. Inspection changes no boundary.
  evidence_ids:
  - security-docs
  - windows-docs
  example: In an interactive codex session, enter /status.
  id: session-status
  kind: session_command
  mechanism_ids: []
  name: /status
  precedence: CLI overrides ordinary config; managed requirements constrain allowed values. Normal config order is CLI, trusted project, selected profile file, user, cloud defaults, system, built-ins.
  values: []
- effect: Run diagnostic command with host-selected backend; -P alias. macOS --log-denials reports OS denials. --include-managed-config includes requirements for explicit profiles.
  evidence_ids:
  - local-help
  - linux-source
  - windows-docs
  example: codex sandbox --include-managed-config --permission-profile :workspace -- <program> <args>
  id: diagnostic
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox --permission-profile
  precedence: Explicit diagnostic profile has its own resolution. Include managed config when testing requirements; current help differs from older platform-subcommand docs.
  values:
  - built-in or custom permission profile
- effect: A cooperating MCP server can contain selected subprocesses using opaque state and a separately verified launcher; server itself retains original authority.
  evidence_ids:
  - local-help
  - mcp-state-source
  - mcp-state-tests
  example: <verified-codex-executable> sandbox --sandbox-state-json '<opaque-state>' -- <program> <args>
  id: mcp-sandbox-state
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: sandbox --sandbox-state-json
  precedence: Experimental negotiated capability; forward state unchanged, preserve unknown fields. Remote/HTTP connections omit verified local launcher; absence is not authorization for host execution.
  values:
  - opaque JSON from MCP codex/sandbox-state metadata
- effect: Constrain user/CLI selection and integrations; profile allowlists deny omitted built-ins/new profiles. Requirements are not merely defaults.
  evidence_ids:
  - managed-docs
  - profiles-docs
  - config-source
  example: |-
    allowed_sandbox_modes = ["read-only", "workspace-write"]
    allowed_approval_policies = ["on-request"]
  id: managed-requirements
  kind: managed_policy
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: requirements.toml
  precedence: Local system < cloud Agent Security < reinterpreted legacy managed config < macOS MDM requirements. Field-specific merges; defaults have different precedence. System path is /etc/codex/requirements.toml or %ProgramData%\OpenAI\Codex\requirements.toml.
  values:
  - allowed_approval_policies
  - allowed_approvals_reviewers
  - allowed_sandbox_modes
  - allowed_permission_profiles
  - default_permissions
  - filesystem restrictions
  - mcp_servers
  - feature and command/network requirements
- effect: Constrain legacy backend choice; does not itself prohibit MXC.
  evidence_ids:
  - windows-docs
  - windows-selection
  example: |-
    [windows]
    allowed_sandbox_implementations = ["elevated"]
  id: managed-windows
  kind: managed_policy
  mechanism_ids:
  - windows-elevated
  - windows-unelevated
  name: windows.allowed_sandbox_implementations
  precedence: Administrator requirements constrain legacy defaults and fallback. Separate windows.allow_mxc controls MXC.
  values:
  - array of elevated and/or unelevated
- effect: False blocks automatic and explicit MXC selection.
  evidence_ids:
  - windows-docs
  - windows-selection
  example: |-
    [windows]
    allow_mxc = false
  id: managed-mxc-deny
  kind: managed_policy
  mechanism_ids:
  - windows-mxc
  name: windows.allow_mxc
  precedence: Managed requirement blocks CLI/user MXC preference or explicit backend selection.
  values:
  - 'true'
  - 'false'
- effect: Source validates that true requires effective local MXC after complete config bootstrap.
  evidence_ids:
  - windows-selection
  example: |-
    [windows]
    require_mxc = true
  id: managed-mxc-require
  kind: managed_policy
  mechanism_ids:
  - windows-mxc
  name: windows.require_mxc
  precedence: Validated after managed/cloud bootstrap; not just a local preference. Source-derived, no native runtime verification.
  values:
  - 'true'
  - 'false'
- effect: Admin requirements can start/constrain the command proxy without local feature activation; full permitted escalation may bypass it. Not a global API/MCP network filter.
  evidence_ids:
  - managed-docs
  - profiles-docs
  example: |-
    [network_proxy]
    enabled = true
  id: managed-network
  kind: managed_policy
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: network_proxy
  precedence: 0.166.0 accepts network_proxy and experimental_network; use one per layer and consider older validators. Managed constraints override ordinary defaults; profile/network composition is field-specific.
  values:
  - enabled and domain/socket/local-binding restrictions; experimental_network alias
- effect: Explicit experimental remote task submission; does not isolate the submitting local CLI or make local sandbox policy a cloud-wide policy.
  evidence_ids:
  - local-help
  - cli-docs
  - cloud-docs
  - cloud-environment-docs
  - cloud-legacy-docs
  example: codex cloud exec --env <ENV_ID> "Review the code"
  id: cloud-submit
  kind: cli_flag
  mechanism_ids:
  - codex-cloud
  name: cloud exec --env
  precedence: Cloud environment settings govern remote work. Account access and environment-generation routing require verification; local OS defaults remain separate.
  values:
  - configured cloud environment identifier
- effect: Connect TUI to an app server; transport alone supplies no new container/VM boundary. Execution host supplies policy enforcement.
  evidence_ids:
  - local-help
  - process-source
  - windows-selection
  example: codex --remote unix:///path/to/server.sock
  id: remote-app-server
  kind: cli_flag
  mechanism_ids:
  - seatbelt
  - bubblewrap-seccomp
  - windows-elevated
  - windows-unelevated
  - windows-mxc
  name: --remote
  precedence: Invocation connection selection; execution-side config and managed policy govern its tools.
  values:
  - ws://host:port
  - wss://host:port
  - unix://
  - unix://PATH
created: 2026-11-04
default_profiles:
- environment: macos
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - seatbelt-source
  id: macos-interactive
  launch_mode: interactive
  mechanism_ids:
  - seatbelt
  notes: Clean codex starts with read-only before trust/onboarding. Normal trusted project Auto becomes workspace-write/on-request; untrusted project status changes command approvals and disables project config. Broad reads remain possible.
  status: enabled
- environment: macos
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - seatbelt-source
  id: macos-non-interactive
  launch_mode: non_interactive
  mechanism_ids:
  - seatbelt
  notes: Clean codex exec defaults to read-only and never approvals. Existing project trust/config can select workspace-write; auto-review changes headless approval routing. Broad reads remain possible.
  status: enabled
- environment: linux
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - linux-source
  id: linux-interactive
  launch_mode: interactive
  mechanism_ids:
  - bubblewrap-seccomp
  notes: Clean codex starts with read-only before trust/onboarding. Normal trusted project Auto becomes workspace-write/on-request; untrusted project status changes command approvals and disables project config. Broad reads remain possible.
  status: enabled
- environment: linux
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - linux-source
  id: linux-non-interactive
  launch_mode: non_interactive
  mechanism_ids:
  - bubblewrap-seccomp
  notes: Clean codex exec defaults to read-only and never approvals. Existing project trust/config can select workspace-write; auto-review changes headless approval routing. Broad reads remain possible.
  status: enabled
- environment: windows
  evidence_ids:
  - default-source
  - features-source
  - windows-selection
  - windows-onboarding
  - process-source
  - headless-source
  - windows-docs
  id: windows-interactive
  launch_mode: interactive
  mechanism_ids: []
  notes: Before first-run backend selection, clean standalone CLI has no enforcer. TUI offers default elevated provisioning or policy-permitted unelevated; after setup, Auto can provide workspace-write/on-request. Optional MXC is not standalone default. Unavailable describes unconfigured startup, not lack of native Windows support.
  status: unavailable
- environment: windows
  evidence_ids:
  - default-source
  - features-source
  - windows-selection
  - windows-onboarding
  - process-source
  - headless-source
  - windows-docs
  id: windows-non-interactive
  launch_mode: non_interactive
  mechanism_ids: []
  notes: Clean codex exec derives read-only intent and never approvals but selects no Windows backend and cannot do TUI provisioning; sandbox-intended operations fail. Preconfigure/provision a native backend before automation. Existing config/trust are not clean defaults.
  status: unavailable
- environment: wsl2
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - linux-source
  id: wsl2-interactive
  launch_mode: interactive
  mechanism_ids:
  - bubblewrap-seccomp
  notes: Clean codex starts with read-only before trust/onboarding. Normal trusted project Auto becomes workspace-write/on-request; untrusted project status changes command approvals and disables project config. Broad reads remain possible. Uses Linux CLI inside WSL2, not native Windows; namespace support is required.
  status: enabled
- environment: wsl2
  evidence_ids:
  - default-source
  - config-source
  - security-docs
  - headless-docs
  - headless-source
  - linux-source
  id: wsl2-non-interactive
  launch_mode: non_interactive
  mechanism_ids:
  - bubblewrap-seccomp
  notes: Clean codex exec defaults to read-only and never approvals. Existing project trust/config can select workspace-write; auto-review changes headless approval routing. Broad reads remain possible. Uses Linux CLI inside WSL2, not native Windows; namespace support is required.
  status: enabled
evidence:
- claim: Installation and authentication/provider setup are distinct from local OS sandbox enforcement.
  id: cli-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/codex/cli
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Latest stable release at examination was 0.166.0, published 2026-10-29; resume/fork permission preservation changed.
  id: release
  limitations: Official release metadata, not proof of runtime isolation; development main may differ.
  location: https://github.com/openai/codex/releases/tag/rust-v0.166.0
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Local version and help identify 0.166.0 flags, YOLO alias, host-selected diagnostic backend, and current feature-list observation. Additional cloud exec and exec-server help establish optional remote submission and PID-namespace startup controls.
  id: local-help
  limitations: Sanitized observations recorded in this report; no user config or credentials read and no model session or sandbox denial test performed.
  location: codex.md#scope-and-evidence
  method: local_inspection
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Recommended Auto/read-only behavior, approval separation, protected paths, automatic review, diagnostics and optional Dev Containers.
  id: security-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/agent-approvals-security
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Beta permission profiles, precedence, read/write/deny rules, proxy activation, platform enforcement and launch-time deny-glob limitations.
  id: profiles-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/permissions
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Native Windows backend setup, MXC capability/version prerequisites, legacy enforcement differences and administrator controls.
  id: windows-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/windows/windows-sandbox
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Clean exec defaults to read-only; current headless flags and retirement of full-auto.
  id: headless-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/non-interactive-mode
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Default-layer precedence, project trust, independent hosted search, and environment filtering default.
  id: config-basic
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/config-file/config-basic
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: CODEX_HOME, configuration profile files, and shell environment filtering semantics.
  id: config-advanced
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/config-file/config-advanced
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Exact sandbox, temp exclusions, login-shell and symlinked-CODEX_HOME keys and scopes.
  id: config-reference
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/config-file/config-reference
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Admin-enforced requirements, source priority, profile/backend restrictions and network constraints.
  id: managed-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/enterprise/managed-configuration
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Hook trust, plugin-bundled hooks, managed trust and invocation-only hook-trust bypass.
  id: hooks-docs
  limitations: Living official documentation checked on the research date; not a runtime enforcement test.
  location: https://learn.chatgpt.com/docs/hooks
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Trust-decision-dependent policy defaults, read-only fallback and native Windows workspace-write downgrade without a configured backend.
  id: default-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/config/src/config_toml.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Explicit approval policy versus project-derived untrusted policy and configuration validation.
  id: config-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/config/mod.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Standalone prefer_mxc is default-off; legacy Windows feature switches are removed/default-off.
  id: features-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/features/src/lib.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Absent configuration selects no Windows backend; MXC preference/strict selection and require_mxc validation are separate.
  id: windows-selection
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/config/windows_sandbox_config.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Interactive default elevated setup, policy-permitted unelevated alternative, setup-failure choices and blocked input during setup.
  id: windows-onboarding
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/tui/src/chatwidget/windows_sandbox_prompts.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Headless sets never approvals except the automatic-review rebuild path; ordinary configuration/trust still participates.
  id: headless-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec/src/lib.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Sandbox-intended process execution is rejected if a local executor cannot supply an enforcer.
  id: process-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/process_sandbox.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Seatbelt policy construction, restricted network routing, writable-root symlink validation and host-home opt-in.
  id: seatbelt-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/seatbelt.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Deny-default macOS policy permits exec/fork and restricts process signaling/info to the same sandbox.
  id: seatbelt-base
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/seatbelt_base_policy.sbpl
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Default bubblewrap/seccomp, bundled fallback, namespace requirements, protected mounts, PID/proc behavior and rejected legacy Landlock filesystem policies.
  id: linux-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/linux-sandbox/README.md
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Native PSEC process isolation, strict versus preferred selection, loopback and alias/hardlink limits, and descendant termination.
  id: mxc-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/mxc-sandbox/README.md
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Filesystem operations use OS-sandboxed helpers and reject unavailable enforcement.
  id: filesystem-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/fs_sandbox.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Patches use approved authority and explicit executor filesystem sandbox context.
  id: patch-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/tools/runtimes/apply_patch.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Child runtime approval/permission snapshots and intersection bound inherited authority.
  id: subagent-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/agent/control/spawn.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Tests encode restrictions for extension work dispatched through executor process/filesystem APIs.
  id: extension-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/tests/suite/extension_sandbox.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Local trusted command hooks spawn directly outside the model-command sandbox; environment hooks can use a separate executor.
  id: hooks-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/hooks/src/engine/command_runner.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Ordinary local MCP child launch and executor MCP launch do not apply the command sandbox; executor launch supplies sandbox None.
  id: mcp-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rmcp-client/src/stdio_server_launcher.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Experimental cooperative MCP sandbox-state metadata and separate verified local launcher contract.
  id: mcp-state-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/codex-mcp/src/sandbox_metadata.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Integration test describes a cooperating server forwarding sandbox state to real CLI subprocesses.
  id: mcp-state-tests
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/tests/suite/mcp_sandbox_tests.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Escalated retries require policy checks and applicable approval; never mode does not silently retry unsandboxed.
  id: retry-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/tools/orchestrator.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Read-only/workspace filesystem rules and protected metadata encode broad reads with limited writes.
  id: policy-source
  limitations: Tagged release source inspection establishes intended implementation, not runtime enforcement on any host.
  location: https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/protocol/src/permissions.rs
  method: source_code
  observed_on: 2026-11-04
  version: 0.166.0
- claim: Official optional CLI submission uses a configured cloud environment and account/repository access.
  id: cloud-docs
  limitations: Living official documentation; no cloud task was submitted and exact CLI environment-generation routing/entitlement remains unresolved.
  location: https://learn.chatgpt.com/docs/cloud
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Current cloud tasks have isolated VM workspaces, explicit public-network/VPN policy and network-secret proxy substitution.
  id: cloud-environment-docs
  limitations: Living official documentation; no cloud task was submitted and exact CLI environment-generation routing/entitlement remains unresolved.
  location: https://learn.chatgpt.com/docs/environments/cloud-environments
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
- claim: Legacy cloud tasks use containers, setup-time secrets and default offline agent phase; distinct from current cloud.
  id: cloud-legacy-docs
  limitations: Living official documentation; no cloud task was submitted and exact CLI environment-generation routing/entitlement remains unresolved.
  location: https://learn.chatgpt.com/docs/environments/cloud-environment
  method: official_docs
  observed_on: 2026-11-04
  version: unknown
gaps:
- detail: Source and documented setup choices establish unconfigured startup and intended behavior, but no native first-run session was exercised.
  location: default_profiles/windows-interactive
  next_check: On a disposable native Windows VM with isolated CODEX_HOME, observe trust/setup, verify pre-setup refusal, then harmless read/write/network probes after each supported backend setup.
- detail: Clean no-backend refusal is source-derived, not a native headless runtime observation; configuration or resumed state can change results.
  location: default_profiles/windows-non-interactive
  next_check: Run codex exec on a disposable native Windows host before and after administrator provisioning with clean config and synthetic credentials/provider fixture.
- detail: No runtime denial matrix was executed on macOS, Linux, WSL2 or Windows; source inspection and tests read do not prove runtime enforcement.
  location: mechanisms
  next_check: Use temporary fixtures on each OS to check allowed/denied reads/writes, metadata, symlinks/hardlinks, subprocess inheritance, long-lived glob timing and direct/proxied networking.
- detail: Executor extension tests establish routed work only, not universal coverage of every plugin process host.
  location: mechanisms/seatbelt/coverage
  next_check: Trace each enabled extension's process/filesystem/network routes and probe executor-routed versus direct host operations.
- detail: Live security docs still show platform diagnostic subcommands and plural permissions-profile, unlike installed 0.166.0 help and tagged README.
  location: controls/diagnostic
  next_check: Recheck codex sandbox --help on the next release and update runnable examples against exact installed syntax.
- detail: Unknown exact CLI environment-generation routing, plan entitlement and failed-submission behavior; current VM and legacy container guides differ and no task was submitted.
  location: mechanisms/codex-cloud
  next_check: With authorized disposable cloud environment/account, submit a harmless CLI task and verify environment generation, effective network/secrets policy, account eligibility and a failed submission's returned error without assuming local fallback.
last_updated: 2026-11-04
mechanisms:
- activation: Default backend for restricted local execution; policy begins read-only before trust and normally becomes workspace-write after onboarding.
  backend: System /usr/bin/sandbox-exec with generated deny-default Seatbelt policy.
  coverage: Shells and descendants and sandbox-context filesystem/patch helpers are covered. Subagents inherit bounded authority; extension executor work uses it. Host orchestrator/API transport, trusted local command hooks and ordinary local/remote MCP servers remain separate. Cooperative MCP can sandbox selected subprocesses.
  enforcement: os_sandbox
  environments:
  - macos
  escape_hatches: Explicit danger-full-access removes the Codex sandbox; YOLO also disables command approvals. Never approvals alone retains isolation. Policy-permitted approved escalation may execute unsandboxed or grant narrower authority; managed requirements constrain selection.
  evidence_ids:
  - seatbelt-source
  - seatbelt-base
  - profiles-docs
  - config-reference
  - process-source
  - filesystem-source
  - patch-source
  - subagent-source
  - extension-source
  - hooks-source
  - mcp-source
  - retry-source
  failure_behavior: fail_closed
  failure_notes: Unrepresentable policy, invalid writable-root links, unavailable executor enforcement or OS denial returns an error. On-request can request authorized escalation; headless never has no human retry prompt.
  filesystem: Built-ins permit broad identity-readable files; writes only to workspace/additional/temp roots, with protected metadata carveouts. Custom profiles can deny reads. Writable roots reject user-controlled symlink components; host-only allow_symlinked_codex_home can opt existing grants into links.
  id: seatbelt
  name: macOS Seatbelt
  network: Default command IP networking denied. Optional enabled networking or proxy allowlist; socket grants are separate. Client model/auth/search/MCP transport is outside this boundary.
  prerequisites:
  - macOS system sandbox-exec and a policy the OS can enforce.
  processes: Exec/fork allowed and descendants inherit policy; signals and process info limited to same sandbox. Host CLI remains outside. Environment and readable credentials need separate hardening.
- activation: Default Linux and WSL2 backend for restricted execution; WSL1 unsupported starting 0.115.
  backend: bubblewrap mount/user/PID/network namespaces with seccomp and PR_SET_NO_NEW_PRIVS; filesystem-restricted policies cannot use legacy Landlock.
  coverage: Shells and descendants and sandbox-context filesystem/patch helpers are covered. Subagents inherit bounded authority; extension executor work uses it. Host orchestrator/API transport, trusted local command hooks and ordinary local/remote MCP servers remain separate. Cooperative MCP can sandbox selected subprocesses.
  enforcement: os_sandbox
  environments:
  - linux
  - wsl2
  escape_hatches: Explicit danger-full-access removes the Codex sandbox; YOLO also disables command approvals. Never approvals alone retains isolation. Policy-permitted approved escalation may execute unsandboxed or grant narrower authority; managed requirements constrain selection.
  evidence_ids:
  - linux-source
  - profiles-docs
  - security-docs
  - process-source
  - filesystem-source
  - patch-source
  - subagent-source
  - hooks-source
  - mcp-source
  - retry-source
  failure_behavior: fail_closed
  failure_notes: Missing system bwrap warns and uses bundled helper; older bwrap has an argv0 compatibility path. Unavailable namespace operations or filesystem-restricted legacy Landlock fails rather than silently running on host. Headless never reports denials.
  filesystem: Read-only base filesystem with writable workspace/additional/temp mounts and protected read-only submounts. Restricted-read profiles and launch-time deny-glob masks supported; new matches are not added to running processes. Protected symlink/missing components are masked.
  id: bubblewrap-seccomp
  name: Linux bubblewrap and seccomp
  network: Restricted networking uses an isolated net namespace and seccomp. Proxy mode bridges only proxy endpoints; Unix sockets need grants. Model/API and ordinary MCP transports remain outside.
  prerequisites:
  - Linux kernel/host permits required namespace and seccomp operations; WSL2 rather than WSL1.
  - System bwrap outside cwd is preferred; packaged codex-resources/bwrap is fallback.
  processes: Default user/PID isolation; fresh proc mount can fall back to inherited proc while retaining PID isolation. Trusted exec-server startup can explicitly inherit PID namespace, allowing same-UID signaling. Descendants retain policy; io_uring blocked.
- activation: Default interactive setup choice and preferred legacy fallback; standalone clean config has no backend before setup. Configure windows.sandbox=elevated.
  backend: Dedicated lower-privilege Windows users, restricted filesystem permissions, firewall rules, local policy and private desktop.
  coverage: Shells and descendants and sandbox-context filesystem/patch helpers are covered. Subagents inherit bounded authority; extension executor work uses it. Host orchestrator/API transport, trusted local command hooks and ordinary local/remote MCP servers remain separate. Cooperative MCP can sandbox selected subprocesses.
  enforcement: os_sandbox
  environments:
  - windows
  escape_hatches: Explicit danger-full-access removes the Codex sandbox; YOLO also disables command approvals. Never approvals alone retains isolation. Policy-permitted approved escalation may execute unsandboxed or grant narrower authority; managed requirements constrain selection.
  evidence_ids:
  - windows-docs
  - windows-selection
  - windows-onboarding
  - default-source
  - features-source
  - process-source
  - profiles-docs
  - mxc-source
  - filesystem-source
  - patch-source
  - subagent-source
  - hooks-source
  - mcp-source
  - retry-source
  failure_behavior: conditional
  failure_notes: Interactive setup failures offer retry, permitted unelevated fallback or quit. A clean headless run cannot provision through TUI and sandbox-intended execution fails without a backend. Execution failure does not automatically select another backend.
  filesystem: Selected policy bounds reads/writes under dedicated identity; built-in broad reads do not promise confidentiality. Workspace/temp writes retain metadata protection. Denied reads supported; glob masks snapshot at process launch.
  id: windows-elevated
  name: Windows elevated sandbox
  network: Dedicated offline-user firewall provides restricted-network enforcement. Optional managed proxy/domain rules and permitted enabled networking; CLI/model/MCP service traffic remains separate.
  prerequisites:
  - Native Windows with supported console facilities; Windows 11 recommended, updated Windows 10 best effort.
  - Administrator-approved provisioning of accounts, filesystem and firewall/local policy; enterprise policy must permit it.
  processes: Provisioning is elevated; workload is not administrator. Children inherit restricted execution; private desktop by default. Detached descendants can survive normal foreground exit, unlike MXC.
- activation: Explicit windows.sandbox=unelevated or permitted interactive fallback after elevated setup is unavailable.
  backend: Restricted token derived from current Windows user, ACL boundaries and private desktop; environment-level offline controls.
  coverage: Shells and descendants and sandbox-context filesystem/patch helpers are covered. Subagents inherit bounded authority; extension executor work uses it. Host orchestrator/API transport, trusted local command hooks and ordinary local/remote MCP servers remain separate. Cooperative MCP can sandbox selected subprocesses.
  enforcement: os_sandbox
  environments:
  - windows
  escape_hatches: Explicit danger-full-access removes the Codex sandbox; YOLO also disables command approvals. Never approvals alone retains isolation. Policy-permitted approved escalation may execute unsandboxed or grant narrower authority; managed requirements constrain selection.
  evidence_ids:
  - windows-docs
  - profiles-docs
  - windows-selection
  - process-source
  - mxc-source
  - filesystem-source
  - patch-source
  - subagent-source
  - hooks-source
  - mcp-source
  - retry-source
  failure_behavior: fail_closed
  failure_notes: Unsupported denied-read/split policies are refused. Missing selected enforcement yields a tool error; no implicit host retry. Interactive selection is subject to managed legacy-backend allowlists.
  filesystem: ACL/token write boundary follows active policy; denied-read paths and unsupported split carveouts cannot be enforced and are refused. Broad built-in reads are not secret isolation.
  id: windows-unelevated
  name: Windows unelevated sandbox
  network: 'Weaker offline controls than elevated: environment-level controls do not establish equivalent OS-enforced network denial. Client model transport and MCP networking remain separate.'
  prerequisites:
  - Supported native Windows console/runtime.
  - Organization permits the weaker legacy fallback; no administrator provisioning required for this selection.
  processes: Restricted current-user token and normally private desktop; children run with restricted authority. Host CLI/hooks and credentials available through readable files/environment remain separate concerns; detached descendants differ from MXC.
- activation: Standalone prefer_mxc is default-off. Explicit windows.sandbox=mxc is strict; features.prefer_mxc=true conditionally prefers it with legacy selection/setup fallback.
  backend: Native Microsoft BaseContainerRunner/PSEC process isolation, not an AppContainer dispatcher or a VM.
  coverage: Shells and descendants and sandbox-context filesystem/patch helpers are covered. Subagents inherit bounded authority; extension executor work uses it. Host orchestrator/API transport, trusted local command hooks and ordinary local/remote MCP servers remain separate. Cooperative MCP can sandbox selected subprocesses.
  enforcement: os_sandbox
  environments:
  - windows
  escape_hatches: Explicit danger-full-access removes the Codex sandbox; YOLO also disables command approvals. Never approvals alone retains isolation. Policy-permitted approved escalation may execute unsandboxed or grant narrower authority; managed requirements constrain selection.
  evidence_ids:
  - windows-docs
  - mxc-source
  - features-source
  - windows-selection
  - process-source
  - filesystem-source
  - patch-source
  - subagent-source
  - hooks-source
  - mcp-source
  - retry-source
  failure_behavior: conditional
  failure_notes: Explicit unavailable/prohibited MXC or missing deny capability fails before launch. Preference can fall back only during selection to configured legacy/setup flow; command errors never trigger backend fallback.
  filesystem: Canonical permission profile with metadata carveouts and symlink/junction alias precedence. Hard-link names retain independent permissions; volume-root alias expansion has documented exceptions. Deny globs snapshot per process. Unrelated network shares excluded from default read expansion.
  id: windows-mxc
  name: Microsoft Execution Containers (MXC)
  network: Restricted managed networking denies direct non-loopback egress and general inbound access but allows host loopback both ways; proxied domains remain constrained and direct DNS denied. Local-binding true removes additional proxy private-IP checks. Separate client/API/MCP transport remains outside.
  prerequisites:
  - Compatible Windows 11 capabilities; docs cite 24H2 build 26100.9278 and 25H2 build 26200.9278, but runtime create/close probe is decisive.
  - Native deny paths need PSE_SUPPORT_FS_DENY; managed network mode requires effective allow_local_binding=true.
  processes: No new accounts, host ACL edits, elevation or VM setup. Child starts suspended and enters kill-on-close job; native policy retained through execution. Foreground exit terminates remaining descendants; environment bootstrap retains validated host SystemDrive/LOCALAPPDATA.
- activation: Official optional deployment integration; opt-in rather than CLI default.
  backend: User-provisioned Docker/compatible Dev Container outer environment; Codex does not create it on ordinary launch.
  coverage: Whole locally installed CLI, local hooks and local MCP can be contained if run inside the container; remote MCP/connectors retain service authority. Inner tools retain Codex OS restrictions only when enabled and supported.
  enforcement: container
  environments:
  - macos
  - linux
  - windows
  - wsl2
  escape_hatches: danger-full-access/YOLO removes inner Codex isolation, not the outer container. Writable host mounts, privileged execution or host-service sockets can undermine the outer boundary.
  evidence_ids:
  - security-docs
  - linux-source
  - profiles-docs
  failure_behavior: conditional
  failure_notes: Container setup failures belong to the runtime. If inner bubblewrap cannot start, docs recommend explicitly relying on the correctly provisioned outer container with inner sandbox disabled; no automatic inner unsandboxed fallback is established.
  filesystem: Outer boundary exposes only configured container files/mounts; anything mounted or credentialed inside is available according to container and selected inner policy. No fixed universal mount/secret defaults are established.
  id: dev-container
  name: Dev Containers
  network: Container networking/firewall is deployment-defined; full-access inner execution relies on those controls. No automatic guarantee of offline networking.
  prerequisites:
  - Configured container runtime and compatible Dev Containers tooling.
  - Explicit choice of mounts, network and credentials; compatible capabilities if retaining inner bubblewrap sandbox.
  processes: Whole CLI and its subprocesses can run inside outer container. Container privileges, mounts and service sockets determine residual host reach.
- activation: Experimental codex cloud exec --env <ENV_ID> submits remote work explicitly. Plain codex/exec does not launch this environment.
  backend: Remote task workspace; current docs describe VMs, legacy docs describe containers. Exact environment generation selected by an arbitrary CLI environment ID is unknown.
  coverage: Remote task tools use remote environment authority. Local CLI, hooks, local MCP and local apply do not become contained just because a cloud task is submitted; remote integrations retain their own authority.
  enforcement: remote_environment
  environments:
  - macos
  - linux
  - windows
  - wsl2
  escape_hatches: Local YOLO is not a general switch disabling the remote environment. Cloud network/secret configuration and remote permissions determine task access; local apply separately changes the checkout.
  evidence_ids:
  - cli-docs
  - local-help
  - cloud-docs
  - cloud-environment-docs
  - cloud-legacy-docs
  failure_behavior: unknown
  failure_notes: No cloud task submission/failure probe was performed. Do not infer local unsandboxed fallback from failed remote submission; exact routing and CLI error handling are unresolved.
  filesystem: Remote task uses provisioned repository/environment filesystem rather than automatically inheriting the local home. Local CLI remains on host; apply writes a resulting diff locally. Exact environment generation determines saved state and secret exposure.
  id: codex-cloud
  name: Codex Cloud task environment
  network: Current cloud public access/domain policy and VPN are configured independently; network secrets use proxy substitution. Legacy setup has internet and agent phase is offline by default. Local CLI sandbox/network keys are not global cloud policies.
  prerequisites:
  - Authenticated account with access to a configured cloud environment and repositories.
  - Exact CLI account/plan entitlement and environment generation require verification; no local container runtime is implied.
  processes: Remote task runs separately from the submitting host CLI. Current VM and legacy container lifecycle/security models differ; no task was launched to verify which route applies.
model: gpt-6.1-sol
provider: codex
schema_revision: 5
summary: Codex CLI provides OS-enforced per-operation sandboxing with broad built-in reads, restricted workspace/temp writes and normally blocked command networking. Interactive trust/setup affects defaults; clean exec is read-only/never. Native Windows requires backend selection/provisioning and optional MXC is not standalone default. Host orchestration, trusted command hooks and ordinary MCP retain separate authority.
support: native
versions_examined:
- 0.166.0
---

# Codex CLI sandboxing

Codex CLI supplies OS-enforced isolation for local command execution. Its normal editing policy permits workspace and temporary-directory writes while blocking command networking. This is primarily an integrity boundary: the built-in policies allow broad reads of files the executing identity can access. They do not make the home directory or credentials confidential. Approvals are a separate decision layer that can authorize broader execution. MCP servers, trusted hooks, hosted tools, and Codex's own API connection must be considered separately.

## Scope and evidence

Examined **2026-11-04 UTC**: the official CLI documentation starting at [the requested entry point](https://developers.openai.com/codex/cli), its current destination [Codex CLI](https://learn.chatgpt.com/docs/codex/cli), and the official repository at **`rust-v0.166.0`**, commit **`6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`**. GitHub's latest stable release was [0.166.0, published October 29, 2026](https://github.com/openai/codex/releases/tag/rust-v0.166.0). Release notes include preservation of acknowledged permissions across resume/fork, a reason to distinguish a resumed session from a fresh launch.

Read-only local inspection returned `codex-cli 0.166.0` from `codex --version`. `codex --help`, `codex exec --help`, `codex sandbox --help`, `codex --yolo --help`, `codex exec-server --help`, and `codex cloud exec --help` corroborated the command surfaces below. `codex features list` reported `prefer_mxc` as under development and disabled in this installation; the tagged feature definitions independently establish its clean-install default. No user configuration or credential contents were inspected. No model session or sandbox enforcement experiment was run on any OS. Thus **documented** means current official documentation, **source-derived** means inspected tagged implementation, and **observed** means only the local version/help observations—not runtime security verification.

## Defaults and first run

The comparison assumes a new standalone CLI, no managed policy, no saved project trust or permission selection, and a working OS backend. An interactive launch includes onboarding; `codex exec` does not. A trust decision or existing configuration changes later defaults, so “always workspace-write” and “always read-only” are both incomplete descriptions. The source derives workspace-write after either a trusted or untrusted project decision, otherwise read-only; an explicitly untrusted project also derives stricter command approvals. See [default resolution](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/config/src/config_toml.rs#L834) and [approval resolution](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/config/mod.rs#L3954).

| Platform | Interactive `codex` | Non-interactive `codex exec` | Enforcer and prerequisite |
|---|---|---|---|
| macOS | Onboarding/trust choice; normal trusted-project Auto is workspace-write with on-request approval. Before a decision, read-only. | Clean default read-only, approvals never; existing trust/config can select workspace-write. | Seatbelt through system `sandbox-exec`; no container or administrator setup. |
| Linux | Same policy/trust behavior as macOS. | Same clean read-only/never behavior. | Bubblewrap namespaces plus seccomp; host must permit required namespace and syscall operations. |
| Native Windows | Backend setup is a first-run prerequisite. Normal setup offers legacy elevated, or policy-permitted unelevated; after setup Auto can use workspace-write/on-request. | No backend is selected in a clean standalone installation. Read-only policy intent and never approvals do **not** provision enforcement: sandbox-intended tools fail until a backend is configured and, where necessary, provisioned. | Native restricted-token/ACL backends; MXC is an optional supported alternative, not the standalone CLI's default. |
| WSL2 | Linux CLI inside WSL2 follows Linux policy/trust behavior. | Linux clean read-only/never behavior. | Linux bubblewrap/seccomp inside the WSL2 guest; its required kernel facilities must work. |

The [security guide](https://learn.chatgpt.com/docs/agent-approvals-security#defaults-and-recommendations) recommends Auto for version-controlled folders and read-only for others. These are onboarding recommendations, not proof that merely finding `.git` grants writes before trust. Explicit untrusted project status skips project-local configuration and requires command approval unless rules permit execution; it does not mean a distinct OS sandbox. The retired explicit `approval_policy = "untrusted"` now prevents startup, although project-derived untrusted approval behavior still exists. [Migration and trust behavior](https://learn.chatgpt.com/docs/agent-approvals-security#migrate-from-the-retired-untrusted-approval-policy).

The [non-interactive guide](https://learn.chatgpt.com/docs/non-interactive-mode#permissions-and-safety) documents read-only as the default. Source sets `AskForApproval::Never` for headless operation, with a rebuild path when automatic review is selected. Saved project decisions and configuration still participate; `--ignore-user-config` skips the base user file but is not a universal clean-room switch. `--ignore-rules` skips user/project execution-policy rules, a separate guardrail; it does not disable the sandbox. `--skip-git-repo-check` relaxes the repository check, not the sandbox. See [exec initialization](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec/src/lib.rs#L590).

Native Windows needs special care: the feature defaults leave MXC preference and legacy feature switches off; absent `windows.sandbox`, configuration selects no backend and downgrades legacy workspace-write intent to read-only. The TUI then offers setup. The executor returns an error when sandbox intent has no enforcer rather than silently executing it on the host. This source-derived qualification is stronger than the documentation's general statement that local execution uses an OS sandbox. [Feature defaults](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/features/src/lib.rs#L1462), [backend selection](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/config/windows_sandbox_config.rs), [setup UI](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/tui/src/chatwidget/windows_sandbox_prompts.rs), [executor refusal](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/process_sandbox.rs#L250).

## What is actually isolated

The CLI/orchestrator remains outside its per-operation sandbox. It loads configuration, holds authentication, requests model inference, and dispatches tools. Shells and their descendants enter the selected OS boundary. Built-in filesystem operations use sandboxed helpers where a filesystem sandbox context is required. An external container can instead contain the whole CLI, including its credentials and integrations, but that is explicit deployment setup.

```mermaid
flowchart TD
    U[User flags, config, managed requirements] --> C[Host Codex CLI / local app server]
    C --> A[Model and authentication service transport]
    C --> P[Policy and approval decision]
    P --> S
    subgraph S[Per-operation OS sandbox]
        E[Shell / exec command] --> D[Child processes]
        F[Filesystem helper / patch operations]
    end
    C --> H[Trusted command hooks: host execution]
    C --> M[MCP servers: own process or remote transport]
    M -. cooperating server can launch sandboxed work .-> S
    C --> W[Hosted search / connector services: separate controls]
```

This boundary follows the [sandbox manager](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/manager.rs) and [filesystem helper runner](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/fs_sandbox.rs). It is neither an agent-wide VM nor a guarantee that every integration shares shell restrictions.

On **macOS**, Codex constructs a Seatbelt policy and starts commands through `/usr/bin/sandbox-exec`. Its base policy denies by default, then permits process execution/forking, signals to processes in the same sandbox, selected system facilities, and the filesystem/network grants. Descendants inherit the policy. Restricted-read profiles add curated runtime allowances rather than indiscriminately exposing `/System`. Unsupported policy construction fails closed. [Seatbelt construction](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/seatbelt.rs), [base policy](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/seatbelt_base_policy.sbpl).

On **Linux and WSL2**, the default filesystem backend is bubblewrap, not Landlock. It layers writable mounts over a read-only filesystem view, reapplies protected carveouts, and uses user, PID, and, for restricted networking, network namespaces. Seccomp and `PR_SET_NO_NEW_PRIVS` constrain syscalls; filters also block `io_uring` bypasses. Managed proxy networking uses an isolated namespace and a routing bridge. Codex prefers system `bwrap` outside the working directory, supports older system versions without `--argv0`, and otherwise uses its bundled helper. A missing system binary can therefore produce a warning without disabling enforcement. Namespace restrictions imposed by a container or host can still prevent startup. [Linux implementation guide](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/linux-sandbox/README.md).

Linux normally isolates PIDs and mounts a fresh `/proc`; a denied proc mount can retain inherited `/proc` while retaining PID isolation. Trusted provisioning can explicitly select `codex exec-server --linux-sandbox-pid-namespace=inherit`; that relaxes same-UID process signaling isolation and is not a repository/environment-variable switch. WSL1 ceased support with the bubblewrap migration in 0.115; WSL2 support is independently documented and does not establish native Windows behavior. [Linux backend details](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/linux-sandbox/README.md), [OS support](https://learn.chatgpt.com/docs/agent-approvals-security#os-level-sandbox).

On **native Windows**, the documented supported choices are:

- `elevated`: administrator-approved provisioning creates dedicated lower-privilege users and installs filesystem permissions, firewall rules, and required local policy. The workload itself is not an administrator process. Both legacy backends normally use a private desktop.
- `unelevated`: a restricted token derived from the current user and ACL-based boundaries, with environment-level offline controls. Its network isolation is weaker, and denied-read policies are unsupported; do not treat it as equivalent to kernel-enforced offline execution.
- `mxc`: Microsoft Execution Containers **native process isolation**, requiring a working PSEC environment. It does not invoke an AppContainer dispatcher, create accounts, modify host ACLs, request elevation, or start a VM. Current docs identify Windows 11 24H2 build 26100.9278 and 25H2 build 26200.9278 as introduction points, but a runtime create/close probe determines actual availability. Deny paths need an additional capability. [Windows setup and prerequisites](https://learn.chatgpt.com/docs/windows/windows-sandbox), [MXC implementation](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/mxc-sandbox/README.md).

MXC is strict when explicitly selected; `features.prefer_mxc = true` selects it only when compatible and otherwise retains legacy selection/setup. This fallback is selection-time compatibility handling: command failures do not trigger a different backend. MXC stops remaining descendants when the foreground process exits, unlike the legacy backends' detached-server behavior. Managed MXC networking permits host loopback and requires effective `allow_local_binding = true`; proxied domain rules remain, but additional proxy private-destination checks are removed. Its hard-link names have independent permissions, and volume-root expansion has documented alias limitations. These are material limits on confidentiality and process-lifetime assumptions. [MXC selection and limits](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/mxc-sandbox/README.md).

## Access boundaries

**Files and credentials.** Read-only permits ordinary file reads and command execution while blocking filesystem mutation; workspace-write additionally permits writes in effective workspace roots and normal system temporary locations. Neither built-in profile is a workspace-only read policy. A shell can therefore read home files or credential files allowed by the executing identity and OS permissions. The orchestrator separately reads its authentication/configuration state outside these tool boundaries. Read-only is not a confidentiality guarantee. [Filesystem policy representation](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/protocol/src/permissions.rs), [built-in profiles and restricted-read examples](https://learn.chatgpt.com/docs/permissions#file-access-limited-to-workspace).

Workspace selection uses `-C`/`--cd`; `--add-dir` adds writable directories rather than narrowing reads. Legacy `[sandbox_workspace_write]` supports `writable_roots`, `network_access`, `exclude_tmpdir_env_var`, and `exclude_slash_tmp`; the two exclusions default false. Thus `/tmp` and `$TMPDIR` may be writable outside the project. Under a writable root, `.git` (including pointer files and the resolved Git directory), existing `.agents`, `.codex`, and `.aws` directories remain recursively read-only. These are write protections, not secret-file denials. [Configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference), [protected paths](https://learn.chatgpt.com/docs/agent-approvals-security#protected-paths-in-writable-roots).

Permission profiles can replace broad reads with `:minimal` runtime paths plus explicit roots, and can deny exact paths or glob matches. More-specific paths override ancestors; equally specific `deny` outranks write/read. On Linux, WSL2, MXC, and elevated Windows, deny-glob matches are snapshotted **at each process launch**: a matching file created later can remain readable to that running process. A long-lived shell is not refreshed with each stdin write. `glob_scan_max_depth` bounds expansion, so an insufficient depth is not a universal secret-file rule. [Profile semantics and timing](https://learn.chatgpt.com/docs/permissions#deny-reads-with-exact-paths-or-globs).

**Links and aliases.** OS enforcement is not a string-prefix workspace check. Linux masks symlink or missing components of protected paths; macOS rejects user-controlled symlink components in writable roots by default. The macOS-only `allow_symlinked_codex_home = true`, read only from the execution host's `$CODEX_HOME/config.toml` at startup, permits already-authorized roots beneath that home to follow links, including changing targets outside it; it adds no writable roots. MXC resolves symlink/junction aliases with strictest equivalent-path access but explicitly does not unify hard-link permissions. [Seatbelt path checks](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/sandboxing/src/seatbelt.rs#L414), [configuration scope](https://learn.chatgpt.com/docs/config-file/config-reference), [MXC alias limits](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/mxc-sandbox/README.md).

**Processes and environment.** The sandbox permits compilers, shells, test runners, and other programs; it constrains their effects rather than providing a universal executable allowlist. Rules may separately allow, prompt for, or forbid commands. Environment handling is a further disclosure surface: current docs say `shell_environment_policy.ignore_default_excludes` defaults to `true`, so automatic filtering of names containing KEY/SECRET/TOKEN is skipped. Set it false and use `inherit = "core"` or `"none"` plus keyed filters for a tighter subprocess environment. `allow_login_shell = false` rejects login-shell requests. These controls neither remove secrets from readable files nor sandbox the host orchestrator. [Environment policy](https://learn.chatgpt.com/docs/config-file/config-advanced#shell-environment-policy).

**Networking.** The built-in restricted modes block ordinary command networking; enabling legacy `network_access = true` grants network access and does not itself create a destination allowlist. Profile `network.enabled = true` likewise needs `features.network_proxy = true`, or enabled managed network requirements, before domain rules enforce anything. The active proxy denies absent allow rules, honors denies over allows, and normally blocks local/private destinations; Unix sockets require explicit grants. A socket such as Docker's can expose a much stronger host service. MXC and unelevated exceptions are described above. [Network permissions](https://learn.chatgpt.com/docs/permissions#network-permissions).

The command proxy does **not** govern model/authentication traffic, hosted web search, connectors, MCP transports, browser/computer-use traffic, or cloud tasks. API access can therefore work while shell `curl` fails. Hosted search normally uses cached results; `--search` or `web_search = "live"` selects live access, and `web_search = "disabled"` disables it. Full-access mode defaults search to live. The security guide also documents `web_search = "indexed"` for index-gated external access. These are separate service controls, not a hole in the subprocess network namespace. [Explicit coverage exclusions](https://learn.chatgpt.com/docs/permissions#what-the-network-proxy-does-not-control), [search defaults](https://learn.chatgpt.com/docs/config-file/config-basic#web-search-mode).

## Which tools share the boundary

| Surface | Coverage in the examined implementation |
|---|---|
| Shell / `exec_command` / continued subprocess input | Initial workload and descendants inherit OS restrictions; a long-running process retains its launch-time authority. |
| Built-in patches and filesystem operations | `apply_patch` uses approval assessment and an explicit filesystem sandbox context; executor helpers enforce reads/writes and refuse missing enforcement. Do not generalize this to every orchestrator disk read. |
| Subagents | Runtime permission and approval snapshots are inherited; the spawn path bounds changed authority by intersection. A subagent is not automatically granted full host access. |
| Skills / extensions | Instruction text supplies no OS isolation. Work dispatched through restricted executor process/filesystem APIs uses those boundaries; an extension is not universally a separate container. |
| Command hooks, including plugin-bundled hooks | Trusted local command hooks spawn directly outside the model-command sandbox. Trust review gates execution; it does not contain it. Environment-targeted hooks can use their environment's executor. |
| Local stdio MCP servers | Ordinary configured server launch is outside the command sandbox; executor-based launch explicitly uses `sandbox: None`. A process-lifetime Job Object is not the command filesystem/network policy. |
| Remote MCP / connectors | Run under the server/service's authority and transport. Local shell restrictions do not confine their effects. |

These are source-derived findings from [patch runtime](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/tools/runtimes/apply_patch.rs), [sandboxed filesystem](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/sandboxed_file_system.rs), [child authority](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/agent/control/spawn.rs), [extension executor tests](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/tests/suite/extension_sandbox.rs), [hook launcher](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/hooks/src/engine/command_runner.rs), and [MCP launcher](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rmcp-client/src/stdio_server_launcher.rs). Inspection of tests is evidence of intended coverage, not an assertion that they were run here.

MCP has a supported experimental **cooperative** integration: a server advertises `codex/sandbox-state` (deprecated alias `codex/sandbox-state-meta`), receives opaque state in call metadata, and can forward it unchanged to `codex sandbox --sandbox-state-json`. A separate `codex/sandbox-launcher` supplies a verified local CLI path when available; remote/HTTP connections omit that launcher. Only work the server routes through it is sandboxed. The server itself retains its original authority and must cooperate; metadata alone is not enforcement. [Metadata contract](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/codex-mcp/src/sandbox_metadata.rs), [integration test](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/tests/suite/mcp_sandbox_tests.rs).

Non-managed hooks must be reviewed and trusted against their current definition; changed hooks are skipped pending review. Managed hooks are trusted by policy. `--dangerously-bypass-hook-trust` bypasses that separate gate for one invocation. Plugin-bundled hooks use the same trust system, and bundled MCP servers retain MCP semantics. Installing a plugin does not cause all its execution to inherit the shell sandbox. [Hooks and trust](https://learn.chatgpt.com/docs/hooks#review-and-trust-hooks).

## Configure the boundary and approvals

These examples are alternatives; do not combine legacy sandbox keys with permission-profile settings.

```sh
# Explicit interactive editing, with escalation requests going to the user.
codex --sandbox workspace-write --ask-for-approval on-request

# Headless review: blocked actions remain blocked, without human prompts.
codex exec --sandbox read-only -c approval_policy='"never"' "Review the code"

# Headless editing, with one additional writable root.
codex exec -C /work/project --sandbox workspace-write --add-dir /work/shared "Run the checks"

# Approval requests go through automatic review; workspace-write remains active.
codex --approve-for-me

# No Codex sandbox and no command approval prompts; outer isolation is external.
codex --dangerously-bypass-approvals-and-sandbox
```

`--sandbox`/`-s` accepts `read-only`, `workspace-write`, or `danger-full-access`. `--ask-for-approval`/`-a` currently exposes `on-request` and `never` in interactive help; use `-c approval_policy=...` for deterministic exec configuration rather than assuming every interactive flag is accepted after the subcommand. `--yolo` is the supported alias for the dangerous bypass. `-a never` alone leaves the selected sandbox intact; `danger-full-access` removes it while approval policy remains independently configurable. `--full-auto` is retired in current documentation; use workspace-write or `--approve-for-me`. [Current command documentation](https://learn.chatgpt.com/docs/developer-commands?surface=cli), [non-interactive migration](https://learn.chatgpt.com/docs/non-interactive-mode#permissions-and-safety), and local 0.166.0 help observations.

```toml
# Legacy policy, ~/.codex/config.toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"
approvals_reviewer = "user"

[sandbox_workspace_write]
writable_roots = ["/work/shared"]
network_access = false
exclude_tmpdir_env_var = true
exclude_slash_tmp = true
```

For finer reads and destinations, use the beta profile system instead:

```toml
default_permissions = "project-limited"
approval_policy = "on-request"

[features]
network_proxy = true

[permissions.project-limited]
extends = ":workspace"

[permissions.project-limited.filesystem]
":root" = "deny"
":minimal" = "read"

[permissions.project-limited.filesystem.":workspace_roots"]
"." = "write"
"secrets" = "deny"

[permissions.project-limited.network]
enabled = true

[permissions.project-limited.network.domains]
"api.openai.com" = "allow"
```

This preserves inherited metadata protection, explicitly restricts reads, and routes allowed command networking through the proxy. Exact denied directories avoid the new-file timing limitation of globs. Profiles also support additional `[permissions.<name>.workspace_roots]` path-to-boolean entries. Built-ins are `:read-only`, `:workspace`, and `:danger-full-access`; full access cannot be an inheritance parent. [Profile definition and examples](https://learn.chatgpt.com/docs/permissions).

Legacy settings win if any loaded config has `sandbox_mode`, if `--sandbox` is supplied, or if the selected config profile sets it. Managed `allowed_permission_profiles` is the exception that activates the newer system; administrators should remove mixed legacy settings. Managed profile compatibility begins at 0.138.0. Select a permission profile for a normal session with `-c 'default_permissions="project-limited"'`; the current diagnostic subcommand separately exposes `--permission-profile`/`-P`. Do not confuse a permission profile with `--profile`/`-p`, which loads `$CODEX_HOME/<name>.config.toml`. [Profile migration](https://learn.chatgpt.com/docs/permissions#migrate-from-older-sandbox-settings), [configuration profiles](https://learn.chatgpt.com/docs/config-file/config-advanced#profiles).

For native Windows, configure and provision before unattended work:

```toml
[windows]
sandbox = "elevated"

[features]
prefer_mxc = true
```

Here MXC is preferred when compatible, with elevated as fallback. Explicit `sandbox = "mxc"` instead fails if unavailable. IT can provision the legacy fallback from an elevated deployment process using `codex sandbox setup --elevated --user 'DOMAIN\alice' --codex-home 'C:\Users\alice\.codex'`; users subsequently run normally. These Windows-only setup forms were documented, not executed on this Mac. [Windows configuration](https://learn.chatgpt.com/docs/windows/windows-sandbox#configure-the-windows-sandbox).

Configuration defaults resolve, highest first: CLI/`-c` overrides; trusted project `.codex/config.toml` layers closest to cwd first; selected profile file; user config; cloud-managed defaults; system config; built-ins. `$CODEX_HOME` defaults to `~/.codex` and relocates config/state/auth lookup—it does not grant a filesystem root. Requirements constrain what these defaults may select, so a command-line bypass cannot override administrator-enforced requirements. [Config precedence](https://learn.chatgpt.com/docs/config-file/config-basic#configuration-precedence), [state directory](https://learn.chatgpt.com/docs/config-file/config-advanced#config-and-state-locations).

Administrators can constrain `allowed_approval_policies`, `allowed_approvals_reviewers`, legacy `allowed_sandbox_modes`, `allowed_permission_profiles`, managed filesystem restrictions, approved `mcp_servers`, feature settings, and command/network rules. Native Windows separately supports `windows.allowed_sandbox_implementations` for legacy choices and `windows.allow_mxc = false` to prohibit MXC. The source also validates `windows.require_mxc = true` after full config bootstrap. In 0.166.0, managed `[network_proxy]` requirements (alias `[experimental_network]`) can activate the command proxy without the local feature switch; use one spelling per layer and account for older policy validators. Local requirements load from `/etc/codex/requirements.toml` on Unix or `%ProgramData%\OpenAI\Codex\requirements.toml` on Windows. In ascending priority: system requirements, cloud Agent Security requirements, reinterpreted legacy managed configuration, then macOS MDM `com.openai.codex:requirements_toml_base64`; merges are field-specific. Cloud administration needs an authorized workspace administrator, while local OS requirements are device provisioning, not a consumer subscription entitlement. [Managed configuration](https://learn.chatgpt.com/docs/enterprise/managed-configuration#locations-and-precedence), [MXC requirement validation](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/config/windows_sandbox_config.rs).

The ordinary OS sandbox is not a paid cloud environment and does not require a special account tier. Running model tasks still requires a supported authentication/provider setup; supported local-model providers have their own prerequisites. Docker/Dev Containers are officially documented **optional outer isolation**, requiring a configured container runtime and chosen mounts/network. Keeping inner Linux sandboxing needs compatible capabilities; explicitly using danger-full-access relies on the container boundary and exposes anything mounted or credentialed inside it. Neither a VM nor a remote cloud task is created by plain `codex`. [CLI installation/authentication](https://learn.chatgpt.com/docs/codex/cli), [Dev Containers integration](https://learn.chatgpt.com/docs/agent-approvals-security#run-codex-in-dev-containers).

An additional optional CLI integration is **`codex cloud exec --env <ENV_ID> "Task"`**. Local 0.166.0 help labels it experimental and confirms submission, status, diff, and apply commands. The [official CLI overview](https://learn.chatgpt.com/docs/codex/cli) links this workflow to [Codex Cloud](https://learn.chatgpt.com/docs/cloud). It requires sign-in and access to a configured environment/repositories; it submits remote work rather than containing the already-running local CLI. Local diff application still changes the local checkout.

The [current cloud guide](https://learn.chatgpt.com/docs/environments/cloud-environments) describes an isolated task workspace in a VM, configured public-network access and independent VPN access; environment variables expose actual values, while network secrets use proxy substitution. The [legacy cloud guide](https://learn.chatgpt.com/docs/environments/cloud-environment) instead describes containers, setup-time secrets and offline-by-default agent networking. These are distinct documented environments. No cloud task was submitted here, so the precise current-versus-legacy routing, entitlement and failure behavior for a CLI environment ID remain unresolved; neither model is assumed to be the default local sandbox. Likewise `codex --remote` connects to another app server; it does not itself provision isolation. The execution host supplies the boundary.

## Denials, escalation, and diagnosis

An OS denial or unsupported policy is returned as a tool error; read-only still permits commands whose effects stay inside policy. A missing backend, failed namespace construction, unavailable explicit MXC selection, or unsupported unelevated denied-read policy does not authorize host execution. Linux's bundled-bwrap compatibility path, MXC preference fallback, and inherited-proc fallback each retain an enforcement boundary; they are distinct from an approved unsandboxed retry. Setting deprecated `features.use_legacy_landlock` cannot substitute for bubblewrap on filesystem-restricted policies because it cannot isolate app-server Unix sockets. [Enforcement failures](https://learn.chatgpt.com/docs/permissions#how-enforcement-works), [process refusal](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/exec-server/src/process_sandbox.rs), [Linux compatibility](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/linux-sandbox/README.md).

With on-request approvals the agent can request broader authority. Approval may grant a narrow path/network permission or explicitly allow an execution outside the sandbox; a full escalation can bypass the command proxy. A failed command is not silently rerun unsandboxed merely because it failed. The orchestrator checks policy, retry scope, and required approval before an escalated attempt. With never approvals, it reports the denial and the model can attempt an in-boundary alternative. `approvals_reviewer = "auto_review"` changes the reviewer, not the baseline sandbox; `--approve-for-me` combines that reviewer with workspace-write. Reviewer errors and timeouts fail closed and review adds model usage. Local `[auto_review].policy` supplies reviewer instructions, with managed `guardian_policy_config` taking precedence; it does not grant OS access. [Retry orchestration](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/tools/orchestrator.rs), [automatic review](https://learn.chatgpt.com/docs/agent-approvals-security#automatic-approval-reviews).

Project trust, plan mode, AGENTS.md instructions, command rules, MCP tool enable/approval settings, and hook review are not substitutes for OS isolation. A writable script can also be executed by a user or service later outside the original boundary. In danger-full-access there is **no Codex filesystem/process sandbox**: ordinary host/container permissions, independent integration permissions, managed restrictions that still apply, and the configured approval layer are the actual guardrails. YOLO additionally removes command approval prompts; it is not just a convenience switch for already-sandboxed work. Separate hook trust still has its own explicit bypass. [Boundary scope](https://learn.chatgpt.com/docs/permissions#what-profiles-control), [bypass semantics](https://learn.chatgpt.com/docs/agent-approvals-security#common-sandbox-and-approval-combinations).

For diagnostics, inspect `/status` and `/debug-config` in an existing interactive session. Current 0.166.0 local help uses **`codex sandbox [OPTIONS] -- COMMAND...`**, with the host choosing the backend, and `-P`/`--permission-profile`; it also offers macOS `--log-denials`. Some current documentation still shows `codex sandbox macos|linux|windows --permissions-profile ...`. Prefer installed help for runnable syntax rather than those older examples. A harmless explicit diagnostic form is `codex sandbox --permission-profile :workspace -- <program> <args>`. On Windows 0.162.0+, docs give `codex -c windows.sandbox=mxc sandbox --include-managed-config --permission-profile :workspace -- cmd.exe /d /c echo MXC_OK`. Explicit diagnostic profiles need `--include-managed-config` when requirements should be included; do not assume a diagnostic invocation proves a managed session's effective policy. [Diagnostic documentation](https://learn.chatgpt.com/docs/agent-approvals-security#test-the-sandbox-locally), [current backend guide](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/linux-sandbox/README.md), and local help observations.

## Unresolved checks and research limits

- **Native Windows first-run end-to-end behavior:** tagged selection, setup UI, and refusal paths establish intended behavior; no native session was launched here. On a clean Windows VM with isolated `CODEX_HOME`, observe trust/setup and run a harmless read/write/network matrix before and after elevated, unelevated, and MXC setup. Do not use WSL results as native evidence.
- **Runtime enforcement across hosts:** no denial probe was run on this Mac, Linux, or WSL2. In temporary fixtures, exercise allowed/denied reads and writes, protected metadata, links, direct/proxied networking, and child-process inheritance using the exact installed binary. Include MXC hard links and long-lived-process glob timing where relevant.
- **Extension coverage:** inspected executor tests show restrictions for work routed through those APIs, not a universal guarantee for every present or future plugin host. Inspect and probe each enabled extension's process/FS/network route; plugin hooks and MCP already have the separate authority documented above.
- **Cloud integration:** confirm which environment generation a CLI task ID uses, its account entitlement, saved network/secret policy and failed-submission behavior with an authorized disposable task; current cloud VM docs must not be silently substituted for legacy container behavior.
- **Document/help drift:** the live security page's platform-specific diagnostic syntax disagrees with installed 0.166.0 help and tagged Linux README. Recheck subsequent releases' `codex sandbox --help` before updating examples. Live docs may evolve independently of the pinned release.

## Changelog

- **2026-11-04 UTC:** Created the report from current official documentation, release 0.166.0 source, and local version/help inspection. Distinguished trust-dependent interactive defaults, headless read-only/never intent, native Windows setup and optional MXC, broader host surfaces, beta profiles, and documented/source-derived limitations.
