# Frontmatter Schema Support

Research into providing JSON Schema-backed validation, autocomplete, and hover for YAML frontmatter in Markdown files, targeting both VSCode and Neovim.

## The Problem

Markdown files commonly embed structured metadata as YAML between `---` delimiters at the top of the file (frontmatter). There is no widely-adopted, cross-editor mechanism to apply JSON Schema validation to this embedded YAML. The core difficulty is that frontmatter lives inside a Markdown document, so standard YAML tooling does not see it.

## Current Ecosystem State (April 2026)

### The Upstream Gap

The Red Hat `yaml-language-server` (yamlls) and its VSCode extension `redhat.vscode-yaml` do **not** natively support applying schemas to YAML embedded inside Markdown files. This is tracked in an open issue since 2019:

- [redhat-developer/vscode-yaml#207](https://github.com/redhat-developer/vscode-yaml/issues/207) — "Support YAML front matter in Markdown files"

The maintainers have indicated this should ideally be solved by the Markdown extension using VSCode's [request forwarding](https://code.visualstudio.com/api/language-extensions/embedded-languages#request-forwarding) mechanism rather than by the YAML extension itself, but no implementation exists in the built-in Markdown extension.

### Recent Progress

1. **ocmrz/vscode-yaml fork** (March 2026) — A community fork of `redhat.vscode-yaml` that adds frontmatter support for `*.md` and related file types. Automatically rebases on upstream. Installable via `.vsix` from [GitHub Releases](https://github.com/ocmrz/vscode-yaml/releases). This is the closest to "it just works" for VSCode today.

2. **Front Matter CMS v10.10.0** (April 2026) — The `estruyf.front-matter` VSCode extension added JSON Schema validation for frontmatter in its v10.10.0 release ([issue #990](https://github.com/estruyf/vscode-front-matter/issues/990)). This validates against schemas you define in the extension's content-type configuration.

3. **remark-lint-frontmatter-schema** — A `remark-lint` plugin that validates frontmatter against JSON Schema. Works as a linter (CLI or editor integration via unified/remark tooling) but does not provide inline autocomplete. ([GitHub](https://github.com/JulianCataldo/remark-lint-frontmatter-schema))

4. **No Neovim-native solution** — There is no off-the-shelf plugin for Neovim that applies JSON Schema to Markdown frontmatter. The `yaml-language-server` only activates for YAML-typed buffers, not Markdown.

## Approaches

### Approach A: VSCode-Only with Forked YAML Extension

**How it works:** Install the `ocmrz/vscode-yaml` fork, which extends `yaml-language-server` to also activate for Markdown-family files. It extracts the frontmatter region and applies standard YAML schema validation, completion, and hover to it.

**Pros:**
- Near-zero configuration — reuses standard `yaml.schemas` settings
- Full autocomplete, hover, validation, and diagnostics
- Auto-rebases on upstream

**Cons:**
- VSCode only (not a language server change, so Neovim gets nothing)
- Requires installing a third-party forked `.vsix`
- May drift from upstream if the fork is abandoned

**VSCode setup:**
1. Download the `.vsix` from [ocmrz/vscode-yaml releases](https://github.com/ocmrz/vscode-yaml/releases)
2. Install: `code --install-extension vscode-yaml-*.vsix`
3. Configure `yaml.schemas` in `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "./schemas/my-frontmatter-schema.json": "*.md"
  }
}
```

### Approach B: VSCode Extension with Embedded Language Services

**How it works:** Build a VSCode extension that uses VSCode's embedded language support to extract the YAML frontmatter region and forward LSP requests to `yaml-language-server`. This is the approach VSCode's built-in HTML extension uses for embedded CSS/JS.

See: [VSCode Embedded Languages Guide](https://code.visualstudio.com/api/language-extensions/embedded-languages)

Two sub-approaches:

#### B1: Language Services (recommended for cross-editor)

Embed the YAML language service library directly in a custom language server. The server:
1. Parses Markdown to detect frontmatter bounds (lines between opening and closing `---`)
2. Creates a virtual YAML document from the frontmatter
3. Delegates completion, hover, and validation to the embedded YAML language service
4. Maps diagnostics/positions back to the original Markdown document offsets

**Pros:**
- Works in any LSP-compliant editor (Neovim, Helix, etc.) since it's a standalone language server
- Full control over the user experience
- Can be published as its own extension and LSP server

**Cons:**
- More development effort
- Must keep the embedded YAML language service updated

#### B2: Request Forwarding (VSCode-only, simpler)

Use VSCode's `middleware` API to intercept LSP requests for Markdown files and forward frontmatter-region requests to the installed YAML extension.

**Pros:**
- Less code to maintain
- Automatically benefits from YAML extension updates

**Cons:**
- VSCode-specific — does not help Neovim users
- Cannot pull diagnostics (VSCode API limitation)

### Approach C: Front Matter CMS Extension

**How it works:** Use the `estruyf.front-matter` extension, which added JSON Schema validation in v10.10.0. Define content types with associated schemas in the extension's configuration.

**Pros:**
- Purpose-built for frontmatter editing
- Includes tag management, snippets, preview panel
- Now has schema validation built in

**Cons:**
- VSCode only
- Heavier extension with a full CMS panel — may be overkill if you only want schema validation
- Schema configuration is extension-specific, not standard `yaml.schemas`

### Approach D: Remark/Unified Lint Pipeline

**How it works:** Use `remark-lint-frontmatter-schema` as part of a `remark` lint pipeline. Can run in CI, as a pre-commit hook, or via an editor integration.

**Pros:**
- Works in any editor that can display lint output (both VSCode and Neovim)
- Can enforce validation in CI
- Standard JSON Schema input

**Cons:**
- No autocomplete or hover — only validation diagnostics
- Requires Node.js toolchain and remark setup
- Configuration is more involved

### Approach E: Custom Neovim Setup (Neovim-only)

**How it works:** Write a Neovim Lua plugin or configuration that:
1. Detects when a Markdown buffer has frontmatter
2. Extracts the frontmatter YAML into a temporary virtual buffer or file
3. Triggers `yaml-language-server` against that virtual content
4. Maps diagnostics back to the original buffer

Alternatively, use `nvim-lspconfig` to configure yamlls with a custom `on_attach` that:
- Overrides `textDocument/completion` to extract frontmatter and delegate to yamlls
- Uses `vim.diagnostic` to display schema validation errors

**Pros:**
- Tailored Neovim experience

**Cons:**
- Significant custom code
- No autocomplete without deep LSP hacking
- Must be maintained alongside Neovim updates

## Schema Definition

Regardless of approach, the schema itself is a standard JSON Schema (Draft 7) document. Example:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Document Frontmatter",
  "type": "object",
  "required": ["title"],
  "properties": {
    "title": {
      "type": "string",
      "description": "The document title"
    },
    "date": {
      "type": "string",
      "format": "date",
      "description": "Publication date"
    },
    "draft": {
      "type": "boolean",
      "default": false,
      "description": "Mark as draft"
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "description": "List of tags"
    }
  },
  "additionalProperties": false
}
```

Schema files can be:
- Local paths (e.g., `./schemas/frontmatter.json`) — committed to the repo
- Remote URLs (e.g., hosted on a website or raw GitHub)
- Registered with [SchemaStore](https://www.schemastore.org/) for automatic discovery

## Recommended Strategy

For cross-editor support (VSCode + Neovim) with the best developer experience:

### Short Term (minimal effort)

| Editor | Strategy | Effort |
|--------|----------|--------|
| VSCode | Install `ocmrz/vscode-yaml` fork + local schema files in repo | Low |
| Neovim | Use `remark-lint-frontmatter-schema` via null-ls/none-ls for validation | Medium |

This gives VSCode users full autocomplete/validation and Neovim users at least validation diagnostics.

### Long Term (proper solution)

Build a standalone Markdown Frontmatter Language Server that:
1. Is a standard LSP server (works in any editor)
2. Extracts YAML frontmatter from Markdown
3. Delegates to `yaml-language-server` internals for completion, hover, validation
4. Maps positions and diagnostics back to the original document
5. Reads schema configuration from a standard location (e.g., `.vscode/settings.json` `yaml.schemas`, or a `.frontmatter-schema` file)

This is Approach B1 above. It would provide:
- **VSCode**: Full schema support via an extension wrapping the language server
- **Neovim**: Full schema support via `nvim-lspconfig` pointing to the language server binary
- **Other editors**: Any LSP-compliant editor benefits

### Neovim Configuration Sketch (Short Term)

Using yamlls for validation of extracted frontmatter via none-ls or a simple autocmd:

```lua
-- In lspconfig setup, yamlls already configured for .yaml/.yml files
-- For frontmatter validation in .md files, use remark-lint via none-ls:
local null_ls = require("null-ls")
null_ls.setup({
  sources = {
    null_ls.builtins.diagnostics.remark.with({
      extra_args = { "--use", "remark-lint-frontmatter-schema=./schemas/frontmatter.json" },
      filetypes = { "markdown" },
    }),
  },
})
```

### VSCode Configuration Sketch (Short Term)

```json
// .vscode/settings.json
{
  "yaml.schemas": {
    "./schemas/frontmatter-schema.json": "*.md"
  }
}
```

With the ocmrz fork installed, this will apply schema validation, autocomplete, and hover to the YAML frontmatter region of Markdown files.

## Key References

- [redhat-developer/vscode-yaml#207](https://github.com/redhat-developer/vscode-yaml/issues/207) — Upstream frontmatter support issue (open since 2019)
- [ocmrz/vscode-yaml](https://github.com/ocmrz/vscode-yaml) — Fork with frontmatter support
- [estruyf/vscode-front-matter](https://github.com/estruyf/vscode-front-matter) — Front Matter CMS extension (schema validation added v10.10.0)
- [remark-lint-frontmatter-schema](https://github.com/JulianCataldo/remark-lint-frontmatter-schema) — Remark lint plugin for frontmatter schema validation
- [VSCode Embedded Languages Guide](https://code.visualstudio.com/api/language-extensions/embedded-languages) — How to build embedded language support
- [JSON Schemas in Neovim](https://www.arthurkoziel.com/json-schemas-in-neovim/) — Comprehensive guide to yamlls + SchemaStore.nvim
- [SchemaStore.nvim](https://github.com/b0o/SchemaStore.nvim) — Neovim plugin for JSON Schema catalog
- [yaml-companion.nvim](https://github.com/someone-stole-my-name/yaml-companion.nvim) — Telescope picker for YAML schemas in Neovim
- [JSON Schema Store](https://www.schemastore.org/) — Public catalog of JSON Schemas
