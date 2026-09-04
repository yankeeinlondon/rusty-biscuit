# DMLS

DMLS is Darkmatter's language server. Use this reference for protocol,
workspace, completion, hover, and diagnostic work.

## Contents

- [Architecture](#architecture)
- [Passive-analysis contract](#passive-analysis-contract)
- [Features](#features)
- [Rollout chronology](#rollout-chronology)
- [Verification](#verification)

## Architecture

The package area has two editor-facing crates:

- `dmls`: the Language Server Protocol server and workspace/document state.
- `zed-dmls`: the Zed extension that launches and configures DMLS.

DMLS owns protocol transport, document snapshots, workspace indexing,
capability negotiation, and publication. Darkmatter owns Markdown parsing,
frontmatter spans, SimplifiedSchema, expression parsing, schema selection, and
validation. Do not fork those authorities in the server.

## Passive-analysis contract

Editor analysis must be safe for every keystroke:

- No shell execution or interpolation effects.
- No remote fetch, credential lookup, or implicit network access.
- No file mutation.
- No ambient CWD/repository recapture for an already-open document.
- No target-specific rendering required to classify a problem.

Use source text plus captured document/workspace context. Keep incomplete syntax
recoverable where possible and publish typed diagnostics with stable ranges.

## Features

DMLS projects Darkmatter's typed surfaces into LSP:

- Frontmatter and schema diagnostics with source spans.
- Schema-aware completion for keys, values, suggestions, literal-discriminated
  union arms, and imported types.
- Expression completion, hover, and parse diagnostics inside
  expression-typed frontmatter.
- Hover/documentation from typed descriptor catalogs.
- Workspace dependency tracking for referenced schemas and documents.

Completion and validation must call the same schema-arm selection and parser
authorities. A completion-only reconstruction is drift.

## Rollout chronology

The DMLS stream was delivered in dependency order:

1. Span-aware validation and style diagnostics in the library.
2. Passive standalone schema classification.
3. Server transport and document/workspace state.
4. Schema selection, imports, completion, hover, and diagnostics.
5. Suggestion, literal-discriminant, expression, and meta-schema support.
6. Zed packaging and end-to-end editor integration.

This chronology is historical routing, not a second architecture. Current code
and public types are authoritative if an older phase note disagrees.

## Verification

Use package-scoped DMLS and Zed gates. Protocol behavior needs integration tests
that open real documents through the server's normal request path. Schema or
expression changes also require passive shipped-artifact corpus coverage in the
Darkmatter library, so editor tests are not the only proof of grammar behavior.

- `just test` runs the cross-platform L1 extension manifest and crate-shape
  contract alongside the DMLS protocol suite.
- `just check-zed` compiles `zed-dmls` for Zed's `wasm32-wasip2` target and
  requires that target to have been provisioned explicitly.
- `just zed-verify` adds Zed's pinned official packager and artifact contract;
  CI runs this as an Ubuntu companion gate, not as L2.
- `just zed-doctor` diagnoses the host's native binary, stable dev-extension
  registration, manifest, and recent Zed log evidence without launching Zed.
