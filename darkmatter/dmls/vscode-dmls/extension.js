// Thin VS Code extension that launches the native `dmls` binary over stdio.
//
// It contains no language logic: all Markdown/Darkmatter intelligence lives in
// `dmls`. Mirrors the Zed shim in `../zed-dmls/`. Written in plain JavaScript so
// there is no TypeScript build step.

const { workspace } = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

/** @type {import('vscode-languageclient/node').LanguageClient | undefined} */
let client;

function activate() {
  const config = workspace.getConfiguration('dmls');
  const command = config.get('server.path') || 'dmls';
  const args = config.get('server.args') || [];

  const serverOptions = {
    run: { command, args, transport: TransportKind.stdio },
    debug: { command, args, transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'markdown' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.{md,markdown,mdown,mkdn}'),
    },
  };

  client = new LanguageClient('dmls', 'Darkmatter Language Server', serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
