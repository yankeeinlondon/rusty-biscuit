# Configuration: hooks, actions, TTS, Protect, MCP, shared resources

## Config files

- **Locations.** `~/.claudine/config.yaml` (user) and `<repo>/.claudine/config.yaml`
  (repo). Both are YAML, validated against the shipped schema
  `claudine/schemas/claudine-config.yaml`. A legacy JSON5 `config.json` beside
  either is converted on the next load and renamed `config.json.bak` (its
  comments are not carried over). The conversion writes actions and routes
  in the keyed form below; if the schema refuses the converted value, nothing
  is written and `config.json` is left as it was.
- **Creating it.** The first run creates the user file. Edit it with
  `claudine config`, a TUI.
- **Shape.** The format is flat:

  ```yaml
  tts: true                       # or false (default), or { provider, voice, gender, prewarm }
  messenger: { … }                # chat routes for `message` actions
  logging: true
  protect: true                   # or { enabled, rules: { <group>: false | {enabled, allow_paths} }, custom_patterns: [...] }
  actions:
    "<event>": [ … ]              # hook actions; a written list needs at least one
  matchers:
    "<event>": "expr or regex"
  preferred_agent: claude         # `favorite_agent` is accepted too
  canonical_provider: claude      # the provider whose skills/commands/agents are the source of truth
  prompt_for_missing: false       # ask on a TTY for missing required document values
  steering:
    automatic:
      enabled: true
  ```

- **Old format.** A legacy `config.json` with `version` and `providers` keys is
  treated as the old format: it's renamed to `config.json.bak` and setup runs
  again.
- **`null` is the same as leaving a key out**, in both files. In the user
  file, a missing or `null` setting takes its default, and every default is
  filled in, optional ones too: `tts: { provider: say }` reads as
  `{ provider: say, gender: female, prewarm: true }`, and a missing
  `steering` reads as `{ automatic: { enabled: true } }`.
- **Empty values are errors**: an event list `[]`, a blank matcher `""`, or
  a blank TTS `provider`. Leave the key out instead.

### Repository overrides

The repository file holds overrides only. A missing or `null` override
inherits the user's value; a written one replaces it.

```yaml
# <repo>/.claudine/config.yaml
canonical_provider: codex
active_messenger: team          # one of the user's routes; null inherits
actions:
  session_start:
    - bash: { command: ./repo-notify.sh }   # replaces the user's session_start actions
steering:
  automatic: { enabled: false }
```

- **An empty override is an error**: `actions: {}`, `matchers: {}`,
  `steering: {}`, `steering: { automatic: {} }`, and `session_start: []`.
- **A repository can't turn messaging off.** `active_messenger: null`
  inherits the user's route, and `claudine config` has no "(disabled)"
  choice. It also can't clear an event's actions, only replace them.

## Hook events and actions

Claudine registers a hook for every event of every installed agent. It maps
those native events onto one **unified** event set.

- `claudine hooks` shows the providers and which events each supports.
- `claudine hooks --support`, `--mapping`, `--describe`, and `--variables`
  show the support levels, the mappings, the payload schemas, and the
  variables you can use.
- `claudine actions` shows what you've configured.

Each entry under `actions.<event>` is a one-key mapping: the key is one of six
types and holds that type's fields. Every type also accepts `when`, and
unknown fields are rejected. The older flat form (`- type: bash` beside
`command:`) no longer loads; the error spells the keyed form.

```yaml
actions:
  "<event>":
    - sound_effect: { effect: crowd-applause, volume: 0.5 }
    - speak: { message: "{{ hook.tool_name }} finished" }
    - bash: { command: notify-send done }                        # fixed 3 s timeout
    - call: { command: ./gate.sh, mapper: { type: exit_code } }  # the ONLY blocking action: 0 = allow, 2 = deny
    - report: {}
    - message: { message: Agent needs you }
```

Messenger routes are keyed the same way:
`messenger.configurations.<name>: { slack: { channel_id: C012345ABC } }`.
The route kinds are `discord`, `slack`, `signal`, `whatsapp`,
`discord_webhook`, and `slack_webhook` (snake_case only). An action or route
with no fields to set is written `{}` (`report: {}`, `slack_webhook: {}`);
`null` in place of the fields is an error.

- `call` mappers: `exit_code`, `json_field`, `json_object`, and `regex`.
- `hook.*` holds the event payload.
- Event names come from `claudine hooks --describe`.

Event logging isn't an action type: the top-level `logging` setting (on by
default) writes every event to `~/.claudine/logs/`.

These hook actions are for **agent hook events**. They're a different thing
from **document lifecycle events** in a prompt's frontmatter
([lifecycle.md](lifecycle.md)).

### TTS

| `tts` value | Hook `speak` | Lifecycle `say` |
|---|---|---|
| absent or `false` | off | auto-detect the best local TTS |
| `true` | on, auto-detect | auto-detect |
| `{ provider, voice, gender, prewarm }` (all optional) | on, with these settings | these settings |

`voice` may be `{ male, female }`. `null` means the same as leaving a key
out; unknown keys are rejected. `prewarm` (default on) prepares early phrases
ahead of time.

### Protect

Protect is **on by default**. It checks shell commands, write paths, and MCP
responses against built-in rule groups:

- `rm -rf`
- `git push --force`
- `curl | bash`
- credential exfiltration
- prompt injection
- sensitive paths
- and others

The result is **Allow** or **Block**. A block becomes the provider's deny
response, and there's no per-run bypass. To allow something, disable that
group or add `allow_paths` under
`protect.rules.<group>`. `custom_patterns: [{ name, pattern, surface? }]`
adds your own rules.

## Shared resources

```bash
claudine skills            # report: canonical provider, resources by scope, problems (Missing, Invalid, BrokenLink, NoLinks)
claudine skills --fix      # create directories and symlinks so every installed agent sees every skill
claudine commands [--fix]  # slash commands
claudine agents [--fix]    # agent / subagent definitions
claudine skills rust-testing   # one skill's structure
```

- **Scope.** Resources are always read from **user** scope, and also from
  **repo** scope inside a git repository.
- **Filters.** `x` selects, `-x` or `!x` excludes, and `x!` matches
  exactly.
- **The canonical provider.** `--fix` needs one set for the repo. Set it in
  `claudine config` → Preferences.
- **Formats vary by provider.** Goose and Kimi agents are YAML and can't be
  the canonical provider. Kimi commands are built-in only.
- **`claudine sync` touches hooks only**, not these resources.
- **`--repo` on a wrapper** (`claudine claude --repo`) shows the agent only
  repo-scoped resources.

## MCP

- **Files.** The catalog is `~/.claudine/mcp/catalog.json`, with
  `defaults.json` and `provider-state.json` beside it. A repo
  `.claudine/mcp.json` replaces the user defaults.
- **Commands:**

  ```bash
  claudine mcp                       # list (same as `mcp list [filter] [--alias]`)
  claudine mcp init | check | sync   # sync PULLS from provider configs into the catalog
  claudine mcp add local|remote …
  claudine mcp config <q> | default [--repo] [ids] | alias <name> <alias> | remove <q>
  claudine mcp export <provider> [--scope user|repo] [--apply]   # PUSH into a provider's own config
  ```

  Add `--json` to any of them.

- **At launch.** `--mcp` turns on the effective defaults. `--use id,alias`
  adds specific servers and implies `--mcp`. A non-interactive prompt can
  name servers with `#tag`, and `--strict` makes an unknown tag an error.

| Provider | Import/export | Injected at launch |
|---|---|---|
| Claude | ✅ | ❌: use `mcp export claude --apply` |
| Codex, Gemini | ✅ | ✅ (per-launch overlay) |
| OpenCode | ✅ | ✅ (`OPENCODE_CONFIG_CONTENT`) |
| Kilo | — | ✅ (`KILO_CONFIG_CONTENT`) |
| Goose, Kimi, Qwen | ❌ | ❌ |

Remember the direction: `sync` pulls from provider configs into the
catalog, and `export` pushes the catalog into a provider's config.

## Making Claudine's schemas visible to `md` and DMLS

The `claudine` binary has its trigger schemas built in. `md` and DMLS read
them **from disk only**.

- **Under `claudine/`** in the repo, documents find them automatically.
- **Elsewhere**, copy the bundle as regular files (symlinks are ignored)
  into `~/schemas/`, an absolute `$SCHEMAS_DIR`, or `<repo>/schemas/`:

```bash
cd claudine/schemas
cp action.yaml lifecycle.yaml sequence-types.yaml linking.yaml document.yaml \
   claudine.yaml inline-compose.yaml sequence.yaml \
   globals.yaml globals.lifecycle.json ~/schemas/
```

The first eight files are the trigger bundle; they import each other. The
last two give the editor Claudine's globals and lifecycle checks.
