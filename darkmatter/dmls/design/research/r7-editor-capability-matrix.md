---
prompt: |-
  DMLS (the Darkmatter Language Server) targets VS Code, Zed, Neovim, and
  Helix as primary editors (see
  @darkmatter/features/2026-07-04-dmls/spec.md). DMLS gates optional
  behavior on a per-client `ClientProfile`
  (@darkmatter/features/2026-07-04-dmls/design.md, "Client Capability
  Profiles"). Build the capability matrix that profile needs.

  For each of VS Code (latest stable), Zed (latest stable), Neovim (0.10+
  built-in LSP client), and Helix (latest stable), determine current
  support status for:

  1. Position encoding negotiation (`positionEncoding`: utf-8/utf-16/utf-32).
  2. `WorkspaceEdit` resource operations (CreateFile/RenameFile/DeleteFile)
     and `ChangeAnnotation` previews.
  3. `codeAction/resolve` and lazy-edit population; `completionItem/resolve`.
  4. Snippet syntax in completion items (tab stops, choices).
  5. `workspace/configuration` requests and `didChangeConfiguration`.
  6. Dynamic capability registration, especially file watchers
     (`workspace/didChangeWatchedFiles`) — who watches, client or server?
  7. `workspace/willRenameFiles` / file-operation notifications.
  8. Pull diagnostics (`textDocument/diagnostic`) vs push.
  9. Inlay hints, document links, folding (line-only?), selection ranges,
     linked editing ranges, work-done progress rendering.
  10. `workspace/executeCommand` round trips and how each editor exposes
      server commands to users.
  11. Markdown rendering fidelity in hover popovers (images? tables?).
  12. Known client quirks affecting Markdown servers (cite issues — e.g.
      the Helix selection-range quirk IWES special-cases).

  Prefer primary sources: editor docs, changelogs, and source code over
  blog posts. Note the version each finding applies to.

  Deliverables: a feature × editor support matrix with version notes, a
  list of features DMLS must capability-gate or provide fallbacks for, and
  per-editor registration/config snippets for launching a stdio LSP named
  `dmls` against Markdown files.
last_updated: 2026-07-06
hash: aea12d5644d74044-814023627725f3bf
---
# R-7: Editor Capability Matrix (VS Code, Zed, Neovim, Helix)

## Version Scope

Findings apply to:

| Editor  | Version basis                                                                                                |
|---------|--------------------------------------------------------------------------------------------------------------|
| VS Code | 1.127 stable, released 2026-07-01; capability details from `vscode-languageclient` main.                     |
| Zed     | 1.9.0 stable, released 2026-07-01; capability details from Zed `main`.                                       |
| Neovim  | Current stable 0.12.4, with 0.10+ notes where behavior changed; capability details from built-in LSP source. |
| Helix   | 25.07.1 latest release listing; capability details from Helix `master` LSP client source.                    |

## Capability Matrix

| Feature                      | VS Code                                                                                                                                                  | Zed                                                                                                                                                                                                       | Neovim 0.10+                                                                                                                                                                          | Helix                                                                                                                                                                                 |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Position encoding            | Advertises `utf-16` only via `vscode-languageclient`; DMLS should select UTF-16.                                                                         | Advertises `utf-16` only; DMLS should select UTF-16.                                                                                                                                                      | 0.10 supports server `positionEncoding`; current capabilities advertise `utf-8`, `utf-16`, `utf-32`. DMLS may select UTF-8 for modern Nvim, but test 0.10 separately.                 | Advertises `utf-8`, `utf-32`, `utf-16` in that order; DMLS should select UTF-8 and avoid publishing diagnostics before initialize completes.                                          |
| `WorkspaceEdit` resource ops | Supports create/rename/delete, document changes, change annotations, metadata, snippet edits.                                                            | Supports create/rename/delete and document changes; no explicit change-annotation support in the inspected capability block.                                                                              | Supports create/rename/delete, document changes, change annotations, and honors change annotations for code actions/rename.                                                           | Supports create/rename/delete and document changes; `change_annotation_support = None`, rename does not honor annotations.                                                            |
| `codeAction/resolve`         | Supported by language client feature set; lazy edit population is suitable.                                                                              | Supported; resolves `kind`, `diagnostics`, `isPreferred`, `disabled`, `edit`, `command`.                                                                                                                  | Supported; current docs note command resolution, source advertises `edit` and `command`.                                                                                              | Supported for `edit` and `command`; Helix resolves only when either is missing.                                                                                                       |
| `completionItem/resolve`     | Supported by language client feature set.                                                                                                                | Supported for `additionalTextEdits`, `command`, `detail`, `documentation`; intentionally not `textEdit`.                                                                                                  | Supported for `additionalTextEdits`, `command`, `documentation`, `detail`.                                                                                                            | Supported for `documentation`, `detail`, `additionalTextEdits`.                                                                                                                       |
| Completion snippets          | Supported if extension/client enables snippet handling.                                                                                                  | Advertises `snippetSupport = true`.                                                                                                                                                                       | Advertises `snippetSupport = true`; actual expansion quality depends on built-in completion or completion plugin path.                                                                | Controlled by Helix `enable_snippets`; advertise only when snippets are enabled.                                                                                                      |
| `workspace/configuration`    | Supported by language client configuration feature.                                                                                                      | Supported; Zed stores and returns language-server configuration.                                                                                                                                          | Advertises `workspace.configuration = true`.                                                                                                                                          | Advertises `configuration = true`; config is supplied from `languages.toml` initialization/config payload.                                                                            |
| `didChangeConfiguration`     | Supported, generally dynamically registered.                                                                                                             | Advertises dynamic registration.                                                                                                                                                                          | Advertises non-dynamic configuration changes.                                                                                                                                         | Advertises non-dynamic configuration changes.                                                                                                                                         |
| Dynamic registration         | Broad support through language-client features.                                                                                                          | Broad support for definitions, completions, formatting, file watchers, file ops, etc.                                                                                                                     | Supported since 0.10; check `client:supports_method()` rather than static capabilities.                                                                                               | Conservative: many text-document features are non-dynamic, but file watchers are dynamic.                                                                                             |
| File watching                | Client watches when server dynamically registers `workspace/didChangeWatchedFiles`.                                                                      | Client watches; advertises dynamic registration and relative patterns. Zed extension can also provide watched server files for extension-managed binaries.                                                | Client watches on macOS/Windows; docs warn file watching is disabled/limited on Linux. DMLS should keep a server-side fallback watcher or rescan path.                                | Client watches after dynamic registration; advertises relative-pattern support.                                                                                                       |
| File operations              | Supports did/will create, rename, delete through VS Code workspace file events.                                                                          | Supports did/will rename in capability block; source advertises did/will rename, not all create/delete operations in the inspected block.                                                                 | Current capability table sets file operations false; do not rely on `workspace/willRenameFiles`.                                                                                      | Advertises will/did create, rename, delete.                                                                                                                                           |
| Diagnostics                  | Push diagnostics supported; pull diagnostics available through language-client diagnostic feature if server advertises it.                               | Push supported; pull supported when Zed enables `pull_diagnostics`.                                                                                                                                       | 0.10 implemented `textDocument/diagnostic`; current source advertises pull and push.                                                                                                  | Supports pull diagnostics and push diagnostics.                                                                                                                                       |
| Inlay hints                  | Supported by VS Code API/language client.                                                                                                                | Supported, including resolve for tooltip/text edits/label command/location.                                                                                                                               | 0.10 implemented inlay hints; current source advertises resolve support.                                                                                                              | Supported, no resolve support.                                                                                                                                                        |
| Document links               | Supported.                                                                                                                                               | Supported with tooltip support.                                                                                                                                                                           | Supported, no tooltip support.                                                                                                                                                        | Supported, no tooltip support.                                                                                                                                                        |
| Folding                      | Supported.                                                                                                                                               | Supports non-line-only folding and `collapsedText`.                                                                                                                                                       | Advertises `lineFoldingOnly = true`; current 0.11+ has `vim.lsp.foldexpr()`.                                                                                                          | No folding-range client capability in inspected initialize block; use editor/tree-sitter folding fallback.                                                                            |
| Selection ranges             | Supported.                                                                                                                                               | Supported in current Zed capability block.                                                                                                                                                                | Advertises `selectionRange`.                                                                                                                                                          | No `selectionRange` capability in inspected initialize block; Helix has editor-native selection expansion, not LSP selection ranges.                                                  |
| Linked editing ranges        | Supported.                                                                                                                                               | Supported in current Zed capability block.                                                                                                                                                                | Advertises `linkedEditingRange`.                                                                                                                                                      | No linked-editing capability in inspected initialize block.                                                                                                                           |
| Work-done progress           | Supported/rendered by VS Code progress UI.                                                                                                               | Advertises `window.workDoneProgress = true`.                                                                                                                                                              | Advertises `window.workDoneProgress = true`; `vim.lsp.status()` consumes progress.                                                                                                    | Advertises `work_done_progress = true`; rendered in Helix status/progress UI.                                                                                                         |
| `workspace/executeCommand`   | Supported; server commands are normally exposed through VS Code contributed commands, code actions, command palette entries, or command links.           | Advertises dynamic execute-command support; user exposure is via Zed actions, code actions, command palette integration, or extension-provided UI.                                                        | If a command is not defined client-side, Nvim forwards it to the server as `workspace/executeCommand`; users can also map Lua wrappers.                                               | Advertises execute-command but non-dynamic; exposed as the `workspace-command` language-server feature and through code actions/commands.                                             |
| Markdown hover fidelity      | Best of the four. `vscode-languageclient` declares `marked`; HTML tags such as tables and images are allowed only when extension opts into HTML support. | Markdown hover blocks supported. Treat images/tables as display-dependent; do not require rich media in hover.                                                                                            | Markdown hover is terminal/floating-window rendered. No inline images; tables should be plain/styled text, not a layout contract.                                                     | Markdown hover requested, but terminal rendering is conservative. No images; keep hover content text-first.                                                                           |
| Markdown-server quirks       | UTF-16-only from common VS Code LSP client; do not assume UTF-8 even though LSP 3.17 allows it.                                                          | Do not resolve `completionItem.textEdit`; Zed source explicitly avoids it for performance. Zed cannot register an arbitrary new LSP from settings alone; DMLS needs an extension for first-class support. | Watchers vary by OS; dynamic registration means static `server_capabilities` checks are insufficient. 0.10 differs from newer 0.11/0.12 for UTF-8/UTF-32 and built-in config helpers. | IWES special-cases Helix one-character code-action selections as empty selections at the start. Keep that as a named `ClientProfile` quirk if DMLS ships selection-sensitive actions. |

## DMLS Profile Gates and Fallbacks

DMLS should make these `ClientProfile` fields explicit:

| Profile field                       | Gate / fallback                                                                                                                                                                     |
|-------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `position_encoding`                 | Select from initialize capabilities. Prefer UTF-8 for Helix and modern Neovim when advertised; otherwise UTF-16. Never publish ranges before negotiation is complete.               |
| `supports_workspace_resource_ops`   | Use resource ops for create/rename/delete only when advertised; otherwise return text edits and ask the user to create/delete files manually.                                       |
| `supports_change_annotations`       | Attach `ChangeAnnotation`s only for VS Code and Neovim. For Zed/Helix, make code-action titles explicit because preview grouping may be absent.                                     |
| `supports_code_action_resolve`      | Lazy-populate expensive edits only where resolve is advertised. Otherwise include edits eagerly or omit expensive actions.                                                          |
| `supports_completion_resolve`       | Lazy documentation/details are broadly safe; avoid resolving `textEdit` for Zed.                                                                                                    |
| `supports_snippets`                 | Emit snippet insert text only when `snippetSupport` is true; otherwise emit plain insert text.                                                                                      |
| `supports_workspace_configuration`  | Prefer `workspace/configuration`; fall back to `.dmls.toml` plus initialization options if the request fails.                                                                       |
| `client_watches_files`              | Register file watchers when supported. Keep a server-side rescan/hash fallback for Neovim on Linux and for clients that ignore watch registrations.                                 |
| `supports_file_operations`          | Use `workspace/willRenameFiles` for VS Code, Zed, and Helix; do not rely on it for Neovim. Also keep rename commands/code actions for clients without file-operation notifications. |
| `diagnostics_mode`                  | Push diagnostics for v1 everywhere. Add pull diagnostics later for Neovim/Zed/Helix/VS Code only after validating refresh behavior.                                                 |
| `folding_line_only`                 | For Neovim, emit line-safe folding ranges. For Zed/VS Code, full ranges are OK. For Helix, assume no LSP folding range support unless validated against the exact release.          |
| `supports_selection_range`          | Enable LSP selection ranges for VS Code, Zed, Neovim. Disable for Helix.                                                                                                            |
| `supports_linked_editing`           | Enable for VS Code, Zed, Neovim. Disable for Helix.                                                                                                                                 |
| `supports_work_done_progress`       | Startup indexing progress is safe for all four, but keep messages concise for terminal clients.                                                                                     |
| `hover_markdown_profile`            | Default to text-first Markdown. Use images or HTML tables only for VS Code with trusted/HTML-enabled client options; never make images required for diagnostics or navigation.      |
| `helix_one_char_selection_is_empty` | Apply only for Helix and only for selection-sensitive code actions, matching the IWES quirk.                                                                                        |

## Registration Snippets

### VS Code Extension

```ts
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
  const serverOptions: ServerOptions = {
    command: 'dmls',
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'markdown' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{md,markdown,mdown,mkdn}'),
    },
  };

  client = new LanguageClient('dmls', 'Darkmatter Language Server', serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
```

### Zed Extension

`extension.toml`:

```toml
id = "dmls"
name = "DMLS"
version = "0.0.1"
schema_version = 1

[language_servers.dmls]
name = "Darkmatter Language Server"
languages = ["Markdown"]
```

`src/lib.rs`:

```rust
use zed_extension_api as zed;

struct DmlsExtension;

impl zed::Extension for DmlsExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: "dmls".into(),
            args: Vec::new(),
            env: Default::default(),
        })
    }
}

zed::register_extension!(DmlsExtension);
```

### Neovim

Neovim 0.11+ native config:

```lua
vim.lsp.config('dmls', {
  cmd = { 'dmls' },
  filetypes = { 'markdown' },
  root_markers = { '.dmls.toml', '.git' },
})

vim.lsp.enable('dmls')
```

Neovim 0.10-compatible direct start:

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'markdown',
  callback = function(args)
    local root = vim.fs.root(args.buf, { '.dmls.toml', '.git' }) or vim.fn.getcwd()
    vim.lsp.start({
      name = 'dmls',
      cmd = { 'dmls' },
      root_dir = root,
    })
  end,
})
```

### Helix

`~/.config/helix/languages.toml` or project `.helix/languages.toml`:

```toml
[language-server.dmls]
command = "dmls"

[[language]]
name = "markdown"
language-servers = ["dmls"]
```

## Source Basis

- [VS Code 1.127 release notes](https://code.visualstudio.com/updates/v1_127)
- [`vscode-languageclient` capability construction](https://github.com/microsoft/vscode-languageserver-node/blob/main/client/src/common/client.ts)
- [`vscode-languageclient` file-operation support](https://github.com/microsoft/vscode-languageserver-node/blob/main/client/src/common/fileOperations.ts)
- [VS Code language-server extension guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)
- [Zed 1.9.0 stable release notes](https://zed.dev/releases/stable/latest)
- [Zed language-extension docs](https://zed.dev/docs/extensions/languages)
- [Zed LSP initialize capabilities](https://github.com/zed-industries/zed/blob/main/crates/lsp/src/lsp.rs)
- [Zed LSP command/hover/diagnostic handling](https://github.com/zed-industries/zed/blob/main/crates/project/src/lsp_command.rs)
- [Neovim 0.10 LSP news](https://neovim.io/doc/user/news-0.10/)
- [Neovim 0.11 LSP news](https://neovim.io/doc/user/news-0.11/)
- [Neovim LSP docs](https://neovim.io/doc/user/lsp/)
- [Neovim LSP protocol capabilities source](https://github.com/neovim/neovim/blob/master/runtime/lua/vim/lsp/protocol.lua)
- [Helix releases](https://github.com/helix-editor/helix/releases)
- [Helix language-server configuration docs](https://docs.helix-editor.com/languages.html)
- [Helix LSP initialize capabilities](https://github.com/helix-editor/helix/blob/master/helix-lsp/src/client.rs)
- [Helix LSP command handlers](https://github.com/helix-editor/helix/blob/master/helix-term/src/commands/lsp.rs)
- [DMLS IWES integration notes for the Helix selection quirk](../../dmls/design/research/r1-iwes-integration-boundary.md)
