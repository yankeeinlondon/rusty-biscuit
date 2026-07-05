---
status: draft
source: darkmatter/dmls/design/design-strategy-2.md
---

# DMLS Research Areas

This document lists the areas that need deeper design, prototypes, or decision
records before DMLS can be implemented with acceptable risk. The strategy in
`darkmatter/dmls/design/design-strategy-2.md` is sound, but several parts need
more granularity before they become implementation tasks.

## 1. IWES Integration Boundary

**Risk:** DMLS depends on IWES graph behavior, but upstream IWES is not designed
as a generic extension framework. A shallow wrapper will not expose enough state
for Darkmatter diagnostics, source maps, completions, or refactors.

**Research questions:**

- Which `liwe` graph/index APIs can be used directly?
- Which IWES server/router modules need to be forked, vendored, or rewritten?
- What is the smallest IWES-derived subset that gives DMLS Markdown graph
  parity without carrying unrelated behavior?
- What seams would be upstreamable later?

**Deliverables:**

- API inventory of usable `liwe` and `iwes` modules.
- Spike branch or prototype that indexes a small Markdown workspace and answers
  symbols, definition, references, and document links.
- Decision record for dependency, vendoring, or local adaptation.

## 2. Provider Registry Semantics

**Risk:** The strategy says IWES providers run first and Darkmatter providers
augment or override them, but it does not define conflict rules. Without that,
features such as definition, hover, completion, and code actions can produce
duplicate or inconsistent results.

**Research questions:**

- What provider result type supports merge, override, suppression, and related
  locations cleanly?
- Which providers should short-circuit when Darkmatter has a precise answer?
- How are provider errors surfaced without failing the whole LSP request?
- How are slow providers deferred or resolved lazily?

**Deliverables:**

- Provider trait design for each capability group.
- Merge policy matrix for diagnostics, completion, hover, definition,
  references, document links, code actions, symbols, folding, and inlay hints.
- Test fixtures for duplicate link/directive scenarios.

## 3. Source Maps And Position Encoding

**Risk:** LSP positions use UTF-16 by default, while Rust string offsets are byte
offsets and parser crates vary in their coordinate systems. Incorrect range
mapping will corrupt every visible editor feature.

**Research questions:**

- Should DMLS store line indexes as byte offsets, UTF-16 offsets, or both?
- How should CRLF, lone CR, multibyte Unicode, and astral-plane characters be
  represented?
- How are frontmatter-relative and virtual-document ranges projected into host
  Markdown documents?
- What document-version identity prevents stale maps after rapid edits?

**Deliverables:**

- Source-map API proposal.
- Unit-test matrix covering ASCII, Unicode, CRLF, frontmatter, code fences, and
  virtual embedded documents.
- Performance measurements for map construction on large Markdown files.

## 4. Position-Aware YAML Frontmatter Parser

**Risk:** Darkmatter can semantically parse frontmatter today, but DMLS needs
precise source ranges for nested YAML keys, values, sequences, comments, and
schema error mapping. The parser choice affects every frontmatter feature.

**Research questions:**

- Do `serde-saphyr`, `rlsp-yaml-parser`, or another parser provide enough span
  fidelity?
- Can parser spans be mapped reliably to LSP UTF-16 ranges?
- Does the parser preserve comments and original scalar spelling where code
  actions or formatting need them?
- How does the parser handle anchors, aliases, multiline scalars, duplicate
  keys, flow mappings, and malformed YAML?
- Should position-aware parsing live in `darkmatter` or remain private to DMLS?

**Deliverables:**

- Prototype parser comparison with the same frontmatter fixture suite.
- `FrontmatterAst` abstraction proposal.
- Decision record for parser choice and ownership boundary.

## 5. Schema Diagnostics To Source Ranges

**Risk:** Schema validators usually report paths or abstract errors, not precise
YAML spans. DMLS must map schema errors to key or value ranges authors can fix.

**Research questions:**

- What error shape does Darkmatter's SimplifiedSchema validation expose today?
- Does validation return enough path data to locate nested YAML values?
- How should missing required keys be ranged when no source node exists?
- How should coercion diagnostics be represented when the source value is a
  string but the semantic value is boolean, number, file, or another type?
- How are `file(eager)` path rewrites explained without mutating the editor
  buffer during diagnostics?

**Deliverables:**

- Schema diagnostic error taxonomy with stable diagnostic codes.
- JSON Pointer or dotted-path mapping from validation errors to
  `FrontmatterAst` ranges.
- Fixtures for required fields, unknown keys, deprecated style keys, invalid
  style values, nested sequences, and file references.

## 6. Darkmatter DSL Parser Fidelity

**Risk:** DMLS needs static understanding of directives, interpolation,
conditions, shell tokens, and horizontal-rule attributes. If the LSP parser
drifts from the compose pipeline, authors will see false diagnostics and bad
quick fixes.

**Research questions:**

- Which Darkmatter parsers already expose reusable syntax trees or spans?
- Which syntax needs new library APIs for position-aware parsing?
- How should the parser recover from malformed input while the user is typing?
- How are nested directive blocks and unclosed blocks represented?
- How are deprecated or migration-only constructs surfaced?

**Deliverables:**

- DSL grammar inventory mapped to existing Darkmatter APIs.
- Error-recovery strategy for incomplete authoring states.
- Golden fixtures comparing DMLS parsing against compose behavior.

## 7. File Reference Resolution

**Risk:** Markdown links, wiki-links, Darkmatter directives, frontmatter
`file(...)`, style assets, and schema references use overlapping but not
identical resolution rules. Cross-platform path behavior can easily diverge.

**Research questions:**

- How does each surface define relative path base, workspace root behavior, and
  allowed extensions?
- Where must DMLS call `biscuit-file::FileReference`?
- How should Windows drive letters, URI encoding, path separators, symlinks, and
  case sensitivity be normalized?
- How are remote URLs represented in the graph when validation is disabled?

**Deliverables:**

- Resolution matrix for each reference surface.
- Cross-platform fixture set for POSIX paths, Windows paths, spaces, URI
  encoding, symlinks, and case collisions.
- Shared resolver API used by diagnostics, completion, document links,
  definition, references, and rename.

## 8. Wiki-Link And Markdown Link Semantics

**Risk:** IWES provides wiki-style graph behavior, while Darkmatter must also be
safe for ordinary Markdown projects. Ambiguous wiki-link resolution can make
definition, references, backlinks, and rename unsafe.

**Research questions:**

- Which wiki-link forms should DMLS support initially?
- How are aliases, frontmatter titles, shortest unique paths, and directory
  index files ranked?
- What is the anchor slug algorithm for Markdown links and wiki heading links?
- When should ambiguity produce multiple locations versus a diagnostic?

**Deliverables:**

- Link-resolution decision record.
- Fixture workspace covering duplicate filenames, aliases, titles, headings,
  missing files, and ambiguous links.
- Rename safety rules for wiki-links and Markdown anchors.

## 9. Graph Invalidation And Performance

**Risk:** The initial full-sync model is simpler, but naive full-workspace
rebuilds will not scale for large documentation sets or rapid typing.

**Research questions:**

- What workspace sizes should DMLS target for first release?
- How expensive is indexing with the IWES graph plus Darkmatter overlay?
- Which changes require local re-indexing versus transitive dependency
  invalidation?
- Are debounce windows needed per feature?
- When, if ever, does DMLS need `ropey` and incremental text sync?

**Deliverables:**

- Benchmark workspace corpus and latency budget.
- Invalidation dependency matrix for links, transclusions, schemas, style
  assets, and interpolation references.
- Recommendation for full sync duration and incremental sync threshold.

## 10. Diagnostics Scheduling

**Risk:** Diagnostics need to be prompt but not noisy or stale. Some diagnostics
are cheap syntax checks, while graph, schema, and external-reference checks are
slower.

**Research questions:**

- Which diagnostics run on every change, on debounce, on save, or only by
  command?
- How are stale diagnostics cleared when an index job is superseded?
- Should DMLS publish partial diagnostic sets by source or wait for a complete
  pass?
- How are related-information chains represented for cycles and duplicates?

**Deliverables:**

- Diagnostic scheduler design.
- Diagnostic severity and source taxonomy.
- Fixtures for rapid edit sequences and stale diagnostic cleanup.

## 11. Security And Side-Effect Boundaries

**Risk:** Darkmatter supports shell expansion and remote content. An LSP runs
inside editors and agents, so accidental execution during hover, completion, or
diagnostics would be a serious design failure.

**Research questions:**

- Which operations are allowed during passive LSP requests?
- How does DMLS explain shell policy without executing commands?
- Where are shell approval files discovered?
- How are remote URL status and cache metadata shown without fetching?
- What explicit commands may perform side effects, and what policy checks are
  required first?

**Deliverables:**

- Static-analysis versus execution policy document.
- Command allowlist and threat model.
- Tests proving passive LSP requests do not execute shell commands or make
  network requests.

## 12. Code Actions And Workspace Edits

**Risk:** Code actions can mutate many files. Incorrect edits can break links,
  schemas, or documents, especially when clients do not support file resource
  operations.

**Research questions:**

- Which code actions are quick fixes, refactors, source actions, or commands?
- Which edits can be constructed eagerly versus through `codeAction/resolve`?
- How should DMLS annotate edits for client preview?
- What fallback is available when a client lacks resource-operation support?
- How are create-file actions templated?

**Deliverables:**

- Code action catalog with safety level and client capability requirements.
- WorkspaceEdit construction rules.
- Fixtures for create file, migrate style key, close directive block, add schema
  key, extract transclusion, and inline transclusion.

## 13. Rename Safety

**Risk:** Rename crosses graph, filesystem, frontmatter, interpolation, and
directive references. Partial rename support is worse than refusal if it leaves
the workspace inconsistent.

**Research questions:**

- What symbols are renameable in phase one?
- How does rename detect ambiguous wiki targets, duplicate anchors, and reserved
  roots?
- How are file renames coordinated with client resource-operation capability?
- How are frontmatter key-path renames propagated through interpolation,
  `set.NAME`, schema references, and nested values?

**Deliverables:**

- Rename support matrix.
- Refusal and confirmation rules.
- Multi-file edit fixtures for files, headings, link reference labels, and
  frontmatter key paths.

## 14. Formatting Boundaries

**Risk:** IWES formatting and Darkmatter cleanup have different semantics. DMLS
must format documents using Darkmatter rules without executing unsafe compose
phases or surprising users.

**Research questions:**

- Which cleanup operations are safe for `textDocument/formatting`?
- What range-formatting contexts can be supported without damaging surrounding
  Markdown?
- How should table formatting interact with GFM and existing Darkmatter cleanup?
- Which normalization actions should be explicit source actions instead of
  format-on-save behavior?

**Deliverables:**

- Formatting capability matrix.
- Round-trip fixtures for cleanup, list normalization, table formatting,
  frontmatter preservation, and directives.
- Decision record for whole-document versus range formatting in phase one.

## 15. Embedded Language Support

**Risk:** Code fences can host many languages, but implementing a polyglot LSP
inside DMLS would balloon scope and performance risk.

**Research questions:**

- Which embedded languages get syntax-only validation first?
- How should code-fence ranges map to virtual documents?
- What is the minimum useful support for JSON, YAML, TOML, Mermaid, and Rust
  fences?
- Should sidecar LSP delegation be postponed until after core DMLS stability?

**Deliverables:**

- Tiered embedded-language support plan.
- Virtual-document range mapping prototype.
- Fixtures for parse diagnostics shifted back into host Markdown ranges.

## 16. Editor Capability Matrix

**Risk:** VS Code, Neovim, Helix, and Zed differ in LSP capability support,
dynamic registration behavior, code action resolve support, resource operations,
and position encoding preferences.

**Research questions:**

- Which LSP 3.17 capabilities can be relied on across target editors?
- Which features need client-specific fallback behavior?
- How should DMLS expose custom commands for preview or shell approval without
  requiring a bespoke client extension?
- Does Zed impose WASM or packaging constraints that affect server design?

**Deliverables:**

- Capability matrix for target editors.
- Client-profile code design.
- Manual or automated smoke-test checklist per editor.

## 17. Configuration Model

**Risk:** DMLS behavior depends on workspace roots, schema defaults, wiki-link
rules, strict modes, shell policy, remote URL policy, formatting preferences,
and Claudine extension settings. Underspecified config creates unpredictable
behavior across editors.

**Research questions:**

- What file names and locations should DMLS read?
- Which settings come from LSP `workspace/configuration` versus repo files?
- How are config changes watched and applied?
- What is the precedence order between editor config, repo config, document
  frontmatter, and command options?

**Deliverables:**

- `DmlsConfig` schema.
- Config precedence rules.
- Reload and invalidation behavior.

## 18. Claudine Extension Model

**Risk:** Claudine is a first-class consumer, but hardcoding Claudine details
into DMLS would make future extensions harder and increase coupling.

**Research questions:**

- What Claudine lifecycle schemas must be supported initially?
- Can Claudine schemas be expressed as Darkmatter SimplifiedSchema values?
- What extension API lets Claudine add schema, diagnostics, completion, hover,
  and code actions without special cases everywhere?
- Which Claudine behavior is generic extension infrastructure versus
  Claudine-specific policy?

**Deliverables:**

- Claudine schema fixture suite.
- Extension provider interface proposal.
- Decision record separating generic DMLS extension hooks from Claudine-specific
  adapters.

## 19. Test Strategy

**Risk:** LSP behavior is stateful and cross-feature. Without focused test
infrastructure, regressions in source maps, indexing, and provider merging will
be difficult to diagnose.

**Research questions:**

- What can be tested with pure unit tests versus in-memory `lsp-server`
  connections?
- Which fixtures need real filesystem behavior?
- How should cross-platform path tests run on macOS, Windows, and Linux?
- What belongs in L1 versus L2 for DMLS?

**Deliverables:**

- DMLS test pyramid.
- In-memory LSP harness using `lsp-server::Connection::memory()` if suitable.
- Fixture layout for source maps, frontmatter, schemas, graph edges, providers,
  code actions, and rename.

## 20. Packaging And Invocation

**Risk:** Editors need to spawn `dmls` reliably across macOS, Windows, and Linux.
Installation, binary naming, logging, and configuration discovery must be clear
before editor support can be validated.

**Research questions:**

- Should the binary be named `dmls`, `darkmatter-lsp`, or both?
- How should logging be routed when stdio is reserved for LSP messages?
- What command-line flags are needed for root override, log level, config path,
  and feature gates?
- How will VS Code, Neovim, Helix, and Zed discover the binary?

**Deliverables:**

- CLI invocation contract.
- Logging and crash-reporting behavior.
- Editor setup snippets for target editors.

## Suggested Research Order

1. Source maps and position encoding.
2. IWES integration boundary.
3. Position-aware YAML parser and `FrontmatterAst`.
4. Schema diagnostics to source ranges.
5. File reference and link resolution.
6. Darkmatter DSL parser fidelity.
7. Provider registry semantics.
8. Diagnostics scheduling and performance.
9. Rename, code actions, formatting, and editor capability hardening.

This order retires foundational correctness risks before adding broad feature
surface area.

