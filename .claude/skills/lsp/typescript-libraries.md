---
prompt: |-
	Your task is to research npm libraries which help Javascript/Typescript authors create an LSP and write your findings to the body of this document.
    
    - what is the functional footprint and goal of this library?
    - provide a very simple Typescript code example of creating a bespoke LSP
    - describe how easily this library could be used to _extend_ an existing LSP implementation and how that would work
        - use `Markdown` as the example language you're focusing on

last_updated: 2026-04-16
---
# LSP Libraries for Typescript

## Overview

There are two primary npm ecosystem libraries for building Language Server Protocol (LSP) implementations in TypeScript/JavaScript:

1. **`vscode-languageserver`** (Microsoft) -- the foundational, low-level SDK
2. **`langium`** (Eclipse/TypeFox) -- a high-level DSL framework built on top of it

A third approach worth noting is **`vscode-languageserver-protocol`** (also from Microsoft), which provides only the TypeScript type definitions for the protocol itself, useful when you want to bring your own transport and message handling.

---

## 1. `vscode-languageserver`

**npm:** `vscode-languageserver` (current major: 9.x)
**Source:** <https://github.com/microsoft/vscode-languageserver-node>

### Functional Footprint and Goal

This is the canonical, Microsoft-maintained SDK for implementing an LSP server in Node.js. It ships as a family of packages:

| Package                              | Purpose                                                                                              |
|--------------------------------------|------------------------------------------------------------------------------------------------------|
| `vscode-languageserver`              | High-level server creation API (`createConnection`, `TextDocuments`, feature handlers)               |
| `vscode-languageserver-protocol`     | TypeScript type definitions for every LSP request, notification, and data structure (protocol v3.17) |
| `vscode-languageserver-textdocument` | A simple `TextDocument` implementation with incremental update support                               |
| `vscode-languageserver-types`        | Shared data types (`Position`, `Range`, `Diagnostic`, etc.)                                          |
| `vscode-jsonrpc`                     | The underlying JSON-RPC 2.0 transport (stdio, IPC, socket, pipe)                                     |

The goal is to give you **full, granular control** over every LSP capability without imposing any parser, AST, or language design decisions. You register handlers for individual LSP requests (`textDocument/completion`, `textDocument/hover`, etc.) and implement the logic yourself.

### Simple TypeScript Example: A Bespoke Markdown LSP

```typescript
import {
  createConnection,
  TextDocuments,
  Diagnostic,
  DiagnosticSeverity,
  ProposedFeatures,
  InitializeParams,
  TextDocumentSyncKind,
} from "vscode-languageserver/node.js";
import { TextDocument } from "vscode-languageserver-textdocument";

const connection = createConnection(ProposedFeatures.all);
const documents = new TextDocuments(TextDocument);

connection.onInitialize((params: InitializeParams) => {
  return {
    capabilities: {
      textDocumentSync: TextDocumentSyncKind.Incremental,
      completionProvider: { resolveProvider: false },
      hoverProvider: true,
    },
  };
});

documents.onDidChangeContent((change) => {
  validateMarkdown(change.document);
});

function validateMarkdown(doc: TextDocument): void {
  const text = doc.getText();
  const diagnostics: Diagnostic[] = [];
  const lines = text.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // Detect broken link syntax: [text](
    const brokenLinkRegex = /[([^]]*)\]\(\s*\)/g;
    let match;
    while ((match = brokenLinkRegex.exec(line)) !== null) {
      diagnostics.push({
        severity: DiagnosticSeverity.Warning,
        range: {
          start: { line: i, character: match.index },
          end: { line: i, character: match.index + match[0].length },
        },
        message: `Empty link target for "${match[1]}"`,
        source: "md-lsp",
      });
    }
  }

  connection.sendDiagnostics({ uri: doc.uri, diagnostics });
}

connection.onCompletion((textDocumentPosition) => {
  const doc = documents.get(textDocumentPosition.textDocument.uri);
  if (!doc) return [];

  const line = doc.getText().split("\n")[textDocumentPosition.position.line];
  const prefix = line.slice(0, textDocumentPosition.position.character);

  if (prefix.trimStart().startsWith("#")) {
    return [
      { label: "## ", kind: 15, detail: "Level 2 heading" },
      { label: "### ", kind: 15, detail: "Level 3 heading" },
      { label: "#### ", kind: 15, detail: "Level 4 heading" },
    ];
  }

  return [];
});

connection.onHover((params) => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;

  const line = doc.getText().split("\n")[params.position.line];
  const boldRegex = /\*\*([^*]+)\*\*/g;
  let match;
  while ((match = boldRegex.exec(line)) !== null) {
    const start = match.index;
    const end = start + match[0].length;
    if (params.position.character >= start && params.position.character <= end) {
      return {
        contents: {
          kind: "markdown",
          value: `**Bold text:** \`${match[1]}\``,
        },
      };
    }
  }
  return null;
});

documents.listen(connection);
connection.listen();
```

### Extending an Existing LSP with `vscode-languageserver`

The library is inherently **composition-oriented**. To extend an existing Markdown LSP (e.g., one that already provides diagnostics), you would:

1. **Middleware pattern:** Wrap the existing server's handlers. Since `vscode-languageserver` uses plain function handlers registered via `connection.onXxx()`, you can intercept requests, augment them, and delegate:

   ```typescript
   // Wrap an existing handler
   const originalCompletion = existingServer.onCompletion.bind(existingServer);
   
   connection.onCompletion(async (params) => {
     // Get completions from the base server
     const baseItems = await originalCompletion(params);
   
     // Add your own custom completions
     const extraItems = [
       { label: "> blockquote", kind: 15, detail: "Blockquote" },
       { label: "---", kind: 15, detail: "Horizontal rule" },
     ];
   
     return [...(baseItems ?? []), ...extraItems];
   });
   ```

2. **Proxy/bridge approach:** If the existing LSP runs as a separate process, start it as a child process and proxy messages through `vscode-jsonrpc`, intercepting and augmenting specific requests before forwarding them. This is the pattern used by projects like `vscode-html-languageservice` and unified-language-server.
3. **Direct handler override:** Since handlers are registered imperatively, a later registration replaces an earlier one. You can call the original handler internally for fallback behavior.

**Difficulty:** Moderate. The library provides all the hooks, but you must manage the composition yourself -- there is no built-in plugin or middleware system at the protocol level.

---

## 2. `langium`

**npm:** `langium` (current: 4.x)
**Source:** <https://github.com/eclipse-langium/langium>

### Functional Footprint and Goal

Langium is a **full-stack language engineering framework** that sits on top of `vscode-languageserver`. Rather than making you build a parser and AST from scratch, Langium provides:

- **Grammar Language** -- An EBNF-like DSL (`.langium` files) for declaring your language's syntax
- **Parser Generation** -- Powered by [Chevrotain](https://chevrotain.io); generates a parser from your grammar
- **AST Generation** -- `langium-cli` generates TypeScript interfaces from your grammar rules
- **Cross-reference Resolution** -- Built-in scoping and linking system
- **Validation Framework** -- Register checks per AST node type
- **LSP Integration** -- Out-of-the-box handlers for completion, hover, folding ranges, document symbols, find references, rename, formatting, and more
- **Dependency Injection** -- Every service is overridable via a module system

The goal is to go from **grammar definition to working LSP in minutes**, with sensible defaults for every LSP feature. It is best suited for **new DSLs or custom languages** where you own the grammar.

### Simple TypeScript Example: A Markdown-like Heading Language in Langium

First, define the grammar (`markdown-heading.langium`):

```langium
entry MarkdownDocument:
    blocks+=Block*;

Block:
    Heading | Paragraph;

Heading:
    level=HeadingMark content=HEADING_TEXT;

Paragraph:
    text+=TEXT+;

terminal HEADING_TEXT: /[^\n\r]+/;
terminal TEXT: /[^#\n\r][^\n\r]*/;
hidden terminal WS: /\s+/;

enum HeadingMark:
    H1='#' | H2='##' | H3='###' | H4='####';
```

Generate the AST and then wire up the server:

```typescript
import { startLanguageServer } from "langium/lsp";
import { createServices, MarkdownHeadingModule } from "./language/module.js";
import { NodeFileSystem } from "langium/node";

const services = createServices(NodeFileSystem);
startLanguageServer(services);
```

Add custom validation in the module:

```typescript
import { ValidationAcceptor, ValidationChecks } from "langium";
import { MarkdownHeadingServices } from "./module.js";
import { MarkdownDocument, Heading } from "./generated/ast.js";

export function registerValidationChecks(services: MarkdownHeadingServices) {
  const registry = services.validation.ValidationRegistry;
  const validator = services.validation.MarkdownHeadingValidator;
  const checks: ValidationChecks<MarkdownHeadingServices> = {
    Heading: validator.checkHeadingLevel,
  };
  registry.register(checks, validator);
}

export class MarkdownHeadingValidator {
  checkHeadingLevel(heading: Heading, accept: ValidationAcceptor): void {
    if (heading.level === "H1" && heading.$containerIndex! > 0) {
      accept("warning", "Only one H1 heading should exist per document", {
        node: heading,
        property: "level",
      });
    }
  }
}
```

### Extending an Existing LSP with Langium

Langium's DI-based service architecture makes extension straightforward:

1. **Override individual services** via the module system. Every LSP handler (`CompletionProvider`, `HoverProvider`, `FoldingRangeProvider`, etc.) is a replaceable service:

   ```typescript
   import { DefaultCompletionProvider } from "langium/lsp";
   import { CompletionList, CompletionItem } from "vscode-languageserver-protocol";
   
   class ExtendedMarkdownCompletionProvider extends DefaultCompletionProvider {
     override async getCompletionItems(): Promise<CompletionItem[]> {
       const baseItems = await super.getCompletionItems();
       return [
         ...baseItems,
         { label: "> blockquote", kind: 15 },
         { label: "---", kind: 15 },
         { label: "| table |", kind: 15 },
       ];
     }
   }
   
   export const ExtendedModule = {
     lsp: {
       CompletionProvider: (services) =>
         new ExtendedMarkdownCompletionProvider(services),
     },
   };
   ```

2. **Compose with an external LSP** by running it as a child process and merging results. Langium does not provide this out of the box, but since it produces a standard `vscode-languageserver` connection underneath, you can wrap the connection with a proxy that delegates to the external server for features you don't override.
3. **Multi-language support** is built-in. Langium's `ServiceRegistry` can host multiple languages in a single server, each with independent grammars and service sets.

**Difficulty:** Easy for new languages (grammar-first approach). Moderate for extending an existing non-Langium LSP -- you'd need to bridge between Langium's service layer and the foreign server's protocol messages.

---

## Comparison Summary

| Aspect                   | `vscode-languageserver`                        | `langium`                                |
|--------------------------|------------------------------------------------|------------------------------------------|
| **Abstraction level**    | Low-level (protocol handlers)                  | High-level (grammar + generated AST)     |
| **Parser**               | Bring your own                                 | Generated from grammar                   |
| **AST**                  | You define it                                  | Generated from grammar                   |
| **Default LSP features** | None (all opt-in)                              | Most provided out-of-the-box             |
| **Extensibility model**  | Function wrapping / proxy                      | Dependency injection modules             |
| **Best for**             | Extending/wrapping existing LSPs; full control | New DSLs and languages from scratch      |
| **Learning curve**       | Lower (fewer concepts)                         | Higher (grammar language, DI, lifecycle) |
| **Bundle size**          | ~small (per-package)                           | ~larger (framework + Chevrotain)         |
| **Browser support**      | Yes (browser entry points)                     | Yes (with bundling)                      |
| **Protocol version**     | 3.17 (latest)                                  | 3.17                                     |

---

## Recommendation for Markdown

For **extending an existing Markdown LSP** (e.g., adding custom completions, diagnostics, or hover logic on top of what VS Code already provides):

- Use **`vscode-languageserver`** directly. Start the existing Markdown language server as a child process, proxy all messages through `vscode-jsonrpc`, and intercept/augment only the requests you care about. This is the most common pattern in the VS Code extension ecosystem.

For **building a bespoke Markdown LSP from scratch** (e.g., a custom Markdown dialect with domain-specific syntax):

- Use **`langium`**. Define your Markdown variant's grammar, get an AST, validation framework, and most LSP features for free, then customize only where needed via DI overrides.
