const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

/** @type {vscode.DiagnosticCollection} */
let diagnosticCollection;

/** @type {Map<string, SchemaProperty>} */
let schema;

/**
 * @typedef {{ type: string | string[], description: string, enum?: string[], format?: string }} SchemaProperty
 */

/**
 * Find the schema file by checking workspace .vscode/ directories,
 * then falling back to relative path from __dirname.
 */
function findSchemaPath() {
  // Check each workspace folder for .vscode/skill-frontmatter.schema.json
  const folders = vscode.workspace.workspaceFolders || [];
  for (const folder of folders) {
    const candidate = path.join(folder.uri.fsPath, ".vscode", "skill-frontmatter.schema.json");
    if (fs.existsSync(candidate)) return candidate;
  }
  // Fallback: relative to extension directory
  return path.resolve(__dirname, "..", "..", "skill-frontmatter.schema.json");
}

function loadSchema() {
  const schemaPath = findSchemaPath();
  try {
    const raw = fs.readFileSync(schemaPath, "utf-8");
    const parsed = JSON.parse(raw);
    const props = parsed.properties || {};
    const map = new Map();
    for (const [key, val] of Object.entries(props)) {
      map.set(key, val);
    }
    return map;
  } catch (e) {
    console.error(`[skill-schema] Failed to load schema from ${schemaPath}:`, e);
    return new Map();
  }
}

/**
 * Parse YAML frontmatter from document text.
 * Returns { keys: Map<string, {line, col, len}>, endLine } or null.
 */
function parseFrontmatter(text) {
  const lines = text.split("\n");
  if (lines.length < 2 || !lines[0].match(/^---\s*$/)) return null;

  const keys = new Map();
  let endLine = -1;

  for (let i = 1; i < lines.length; i++) {
    const line = lines[i];
    if (line.match(/^---\s*$/)) {
      endLine = i;
      break;
    }
    // Top-level key: starts at column 0, has a colon
    const keyMatch = line.match(/^([a-zA-Z_][a-zA-Z0-9_-]*)\s*:/);
    if (keyMatch) {
      keys.set(keyMatch[1], { line: i, col: 0, len: keyMatch[1].length });
    }
  }

  if (endLine === -1) return null;
  return { keys, endLine };
}

function isSkillFile(document) {
  return path.basename(document.fileName).toLowerCase() === "skill.md";
}

/**
 * Format a schema property into a Markdown hover string.
 */
function formatHover(key, prop) {
  const lines = [];

  // Type info
  let typeStr;
  if (prop.type) {
    typeStr = Array.isArray(prop.type) ? prop.type.join(" | ") : prop.type;
  }
  if (prop.oneOf) {
    const types = prop.oneOf.map((o) => {
      if (o.type === "array") return `${o.items?.type || "any"}[]`;
      return o.type || "any";
    });
    typeStr = types.join(" | ");
  }

  lines.push(`**\`${key}\`** — \`${typeStr || "any"}\``);
  lines.push("");

  // Description
  if (prop.description) {
    lines.push(prop.description);
  }

  // Enum values
  if (prop.enum) {
    lines.push("");
    lines.push(`Values: ${prop.enum.map((v) => `\`${v}\``).join(", ")}`);
  }

  // Format hint
  if (prop.format) {
    lines.push("");
    lines.push(`Format: \`${prop.format}\``);
  }

  return new vscode.MarkdownString(lines.join("\n"));
}

// ── Diagnostics ──────────────────────────────────────────────

function validateDocument(document) {
  if (!isSkillFile(document)) return;

  diagnosticCollection.delete(document.uri);

  const text = document.getText();
  const fm = parseFrontmatter(text);
  if (!fm) return;

  const diags = [];

  for (const [key, pos] of fm.keys) {
    if (!schema.has(key)) {
      const range = new vscode.Range(pos.line, pos.col, pos.line, pos.col + pos.len);
      const diag = new vscode.Diagnostic(
        range,
        `Unknown frontmatter key: '${key}'. Add it to skill-frontmatter.schema.json to allow it.`,
        vscode.DiagnosticSeverity.Warning
      );
      diag.source = "skill-schema";
      diags.push(diag);
    }
  }

  diagnosticCollection.set(document.uri, diags);
}

// ── Suppress Claude Code diagnostics ─────────────────────────

function suppressForeignDiagnostics(document) {
  if (!isSkillFile(document)) return;
  // If Claude Code assigned the "skill" language, switch to markdown
  // to clear its diagnostics. Our extension activates on both languages.
  if (document.languageId === "skill") {
    vscode.languages.setTextDocumentLanguage(document, "markdown");
  }
}

// ── Hover Provider ───────────────────────────────────────────

/** @type {vscode.HoverProvider} */
const hoverProvider = {
  provideHover(document, position) {
    if (!isSkillFile(document)) return null;

    const text = document.getText();
    const fm = parseFrontmatter(text);
    if (!fm) return null;

    // Only provide hovers inside the frontmatter block
    if (position.line < 1 || position.line >= fm.endLine) return null;

    // Check if cursor is on a frontmatter key
    for (const [key, pos] of fm.keys) {
      if (
        position.line === pos.line &&
        position.character >= pos.col &&
        position.character <= pos.col + pos.len
      ) {
        const prop = schema.get(key);
        if (prop) {
          const range = new vscode.Range(pos.line, pos.col, pos.line, pos.col + pos.len);
          return new vscode.Hover(formatHover(key, prop), range);
        }
        // Key exists in file but not in schema — still show something
        return new vscode.Hover(
          new vscode.MarkdownString(`**\`${key}\`** — *not in schema*\n\nAdd this key to \`.vscode/skill-frontmatter.schema.json\` to define its type and description.`),
          new vscode.Range(pos.line, pos.col, pos.line, pos.col + pos.len)
        );
      }
    }

    return null;
  },
};

// ── Activation ───────────────────────────────────────────────

function activate(context) {
  schema = loadSchema();
  diagnosticCollection = vscode.languages.createDiagnosticCollection("skill-schema");
  context.subscriptions.push(diagnosticCollection);

  // Register hover provider for both skill and markdown languages
  context.subscriptions.push(
    vscode.languages.registerHoverProvider({ language: "skill" }, hoverProvider),
    vscode.languages.registerHoverProvider({ language: "markdown", pattern: "**/SKILL.md" }, hoverProvider)
  );

  // Validate on open
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      validateDocument(doc);
    })
  );

  // Validate on change
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => {
      validateDocument(e.document);
    })
  );

  // Suppress Claude Code's diagnostics when they appear
  context.subscriptions.push(
    vscode.languages.onDidChangeDiagnostics((e) => {
      for (const uri of e.uris) {
        const doc = vscode.workspace.textDocuments.find(
          (d) => d.uri.toString() === uri.toString()
        );
        if (doc) suppressForeignDiagnostics(doc);
      }
    })
  );

  // Validate already-open documents
  for (const doc of vscode.workspace.textDocuments) {
    validateDocument(doc);
  }

  // Re-load schema when the schema file changes
  const schemaPattern = new vscode.RelativePattern(
    path.dirname(findSchemaPath()),
    "skill-frontmatter.schema.json"
  );
  const watcher = vscode.workspace.createFileSystemWatcher(schemaPattern);
  watcher.onDidChange(() => {
    schema = loadSchema();
    // Re-validate all open skill files
    for (const doc of vscode.workspace.textDocuments) {
      validateDocument(doc);
    }
  });
  context.subscriptions.push(watcher);
}

function deactivate() {}

module.exports = { activate, deactivate };
