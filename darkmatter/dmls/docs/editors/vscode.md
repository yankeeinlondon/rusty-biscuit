# VS Code

VS Code is the reference client for DMLS feature testing — it has the richest
capability surface of the four targets. Position encoding is UTF-16 only.

## Option A — generic LSP config

Any generic LSP bridge extension that lets you register a stdio server for a
language works. Point it at `dmls` for `markdown` documents. Exact settings keys
vary by bridge extension; the server command is simply:

```
dmls
```

## Option B — thin extension

The most reliable path is a small extension that starts the server with
`vscode-languageclient`. This is the whole activation:

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

## What you get

- Full navigation (definition, references, document/workspace symbols),
  document links, and folding.
- Broken-link/anchor and duplicate-heading diagnostics; wiki-link diagnostics.
- Path, anchor, and fence-language completion; wiki-link completion.
- Frontmatter schema diagnostics, completion, and hover (base schema +
  configured extensions + document `$schema`).
- Directive/transclusion/interpolation intelligence and read-only shell-policy
  hover (`dmls` never executes shell or fetches remote content).
- File + heading rename with workspace-wide reference updates
  (`workspace/willRenameFiles` supported), the v1 code-action set, and
  whole-document formatting.

## Notes

- Hover renders full Markdown; DMLS keeps hover content text-first and does not
  require images.
- Change annotations and resource operations (create/rename/delete) are fully
  supported, so rename previews and "create missing file" actions work.
