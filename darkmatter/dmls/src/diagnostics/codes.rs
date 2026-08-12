//! Stable diagnostic source names and codes.
//!
//! These strings are part of the user-facing contract: editors key
//! suppression config on them, so renaming a code later breaks users. The
//! frontmatter/schema taxonomy (`dm.*`) is fixed by R-5 and the wiki taxonomy
//! (`wiki.*`) by R-8; Phase 4 ships only the Layer-0 Markdown link subset
//! (`darkmatter.links.*` source, `dm.links.*` codes). The rest are declared
//! here so later phases add rows without renaming existing ones.

/// Namespaced diagnostic `source` values (LSP `Diagnostic.source`).
pub mod source {
    /// CommonMark/GFM structural problems.
    pub const MARKDOWN: &str = "darkmatter.markdown";
    /// Link, anchor, and reference resolution problems.
    pub const LINKS: &str = "darkmatter.links";
    /// Wiki-link (`[[…]]`) resolution problems (R-8).
    pub const WIKI: &str = "darkmatter.wiki";
    /// Frontmatter (YAML) problems.
    pub const FRONTMATTER: &str = "darkmatter.frontmatter";
    /// Schema validation problems.
    pub const SCHEMA: &str = "darkmatter.schema";
    /// Compose-pipeline problems.
    pub const COMPOSE: &str = "darkmatter.compose";
    /// `style:` frontmatter problems.
    pub const STYLE: &str = "darkmatter.style";
    /// Shell-policy problems.
    pub const SECURITY: &str = "darkmatter.security";
}

/// Stable diagnostic `code` values (LSP `Diagnostic.code`).
pub mod code {
    /// A relative link path matched no indexed document.
    pub const BROKEN_PATH: &str = "dm.links.broken_path";
    /// A link resolved to a document but the `#fragment`/anchor was missing.
    pub const MISSING_ANCHOR: &str = "dm.links.missing_anchor";
    /// Two or more headings generate the same GitHub anchor slug.
    pub const DUPLICATE_HEADING: &str = "dm.links.duplicate_heading";

    /// A wiki file target matched no document.
    pub const WIKI_UNRESOLVED_TARGET: &str = "wiki.unresolved-target";
    /// A wiki file target matched multiple documents.
    pub const WIKI_AMBIGUOUS_TARGET: &str = "wiki.ambiguous-target";
    /// A wiki file resolved but its `#heading` fragment did not.
    pub const WIKI_HEADING_MISSING: &str = "wiki.heading-missing-in-target";
    /// An empty wiki target (`[[]]` / `[[|alias]]`).
    pub const WIKI_EMPTY_TARGET: &str = "wiki.empty-target";
    /// An empty wiki heading query (`[[target#]]`).
    pub const WIKI_EMPTY_HEADING: &str = "wiki.empty-heading";
    /// A v1-unsupported wiki form (embed, block ref, interwiki).
    pub const WIKI_UNSUPPORTED_SYNTAX: &str = "wiki.unsupported-syntax";
    /// Indexed paths collide under case-fold/NFC normalization.
    pub const WIKI_PORTABILITY_COLLISION: &str = "wiki.portability-collision";
    /// A malformed percent escape in a wiki target, treated literally.
    pub const WIKI_INVALID_PERCENT_ESCAPE: &str = "wiki.invalid-percent-escape";
    /// A wiki target resolved to a file whose name is visually confusing
    /// (`note.md.md`).
    pub const WIKI_CONFUSING_EXTENSION: &str = "wiki.confusing-extension";
    /// A wiki heading resolved by exact text but a different heading would
    /// match by slug.
    pub const WIKI_HEADING_SPELLING: &str = "wiki.ambiguous-heading-spelling";
    /// A rename would leave a wiki link with no unique replacement spelling
    /// (R-8 file-rename rule 11).
    pub const WIKI_AMBIGUOUS_AFTER_RENAME: &str = "wiki.ambiguous-after-rename";

    // ── Frontmatter / schema (R-5, source `darkmatter.frontmatter` /
    // `darkmatter.schema`) ──

    /// The frontmatter YAML could not be parsed.
    pub const FM_YAML_PARSE: &str = "dm.frontmatter.yaml_parse";
    /// The `$schema` value is not a valid schema shape.
    pub const SCHEMA_INVALID_SHAPE: &str = "dm.schema.invalid_schema_shape";
    /// A semantic `type-definition` value does not parse as one property definition.
    pub const SCHEMA_INVALID_TYPE_DEFINITION: &str = "dm.schema.invalid_type_definition";
    /// The schema could not be resolved, merged, or compiled.
    pub const SCHEMA_PREPARE: &str = "dm.schema.prepare";
    /// A value did not match its declared type.
    pub const SCHEMA_TYPE_MISMATCH: &str = "dm.schema.type_mismatch";
    /// A non-type constraint (range, length, pattern, enum, …) failed.
    pub const SCHEMA_CONSTRAINT: &str = "dm.schema.constraint";
    /// A required key is absent.
    pub const SCHEMA_MISSING_REQUIRED: &str = "dm.schema.missing_required";
    /// A key the schema does not declare is present (strict mode).
    pub const SCHEMA_UNKNOWN_KEY: &str = "dm.schema.unknown_key";
    /// A deprecated key is present.
    pub const SCHEMA_DEPRECATED_KEY: &str = "dm.schema.deprecated_key";
    /// A `file(...)`-typed value failed to parse, resolve, or match a file.
    pub const SCHEMA_INVALID_FILE_REFERENCE: &str = "dm.schema.invalid_file_reference";
    /// A `suggest(...)` candidate is invalid metadata: it fails its target
    /// schema's type, range, integer, length, not-empty, or pattern constraint,
    /// or uses invalid decimal syntax or an unsupported lossless representation.
    pub const SCHEMA_INVALID_SUGGESTION: &str = "dm.schema.invalid_suggestion";
    /// A recognized standalone SimplifiedSchema envelope is malformed (missing
    /// or non-mapping `types`, unsupported tagged-envelope keys, or an invalid
    /// payload).
    pub const SCHEMA_DOCUMENT_MALFORMED: &str = "dm.schema.document_malformed";

    /// A `style:` key the style schema does not recognize.
    pub const STYLE_UNKNOWN_KEY: &str = "dm.style.unknown_key";
    /// A deprecated `style:` key (a canonical replacement exists).
    pub const STYLE_DEPRECATED_KEY: &str = "dm.style.deprecated_key";

    // ── Layer-3 Darkmatter DSL overlay (source `darkmatter.compose` /
    // `darkmatter.security`) ──

    /// A `::` directive keyword the DSL does not recognize.
    pub const DIRECTIVE_UNKNOWN: &str = "dm.directive.unknown";
    /// A `::block` / `::shell-block` / disclosure triple left unclosed.
    pub const DIRECTIVE_UNCLOSED_BLOCK: &str = "dm.directive.unclosed_block";
    /// A `::end-block` closer with no matching opener.
    pub const DIRECTIVE_UNMATCHED_END: &str = "dm.directive.unmatched_end";
    /// An option key a directive family does not recognize.
    pub const DIRECTIVE_MALFORMED_OPTION: &str = "dm.directive.malformed_option";
    /// A `::disclosure` triple left structurally malformed.
    pub const DIRECTIVE_MALFORMED_DISCLOSURE: &str = "dm.directive.malformed_disclosure";
    /// A `::file`/`::code`/prologue/epilogue target that matched no file.
    pub const TRANSCLUSION_BROKEN_PATH: &str = "dm.transclusion.broken_path";
    /// A `::file`/`::code` transclusion cycle.
    pub const TRANSCLUSION_CYCLE: &str = "dm.transclusion.cycle";
    /// A malformed `{{ … }}` interpolation or `when=` expression.
    pub const EXPRESSION_MALFORMED: &str = "dm.expression.malformed";
    /// An interpolation identifier that names no frontmatter key, `ctx.*`,
    /// `env.*`, or expression function.
    pub const EXPRESSION_UNKNOWN_IDENTIFIER: &str = "dm.expression.unknown_identifier";
    /// A fenced-code info string whose language token no grammar recognizes.
    pub const FENCE_UNKNOWN_LANGUAGE: &str = "dm.fence.unknown_language";
    /// A `::shell` / `::shell-block` / `$()` command the shell policy disallows.
    pub const SECURITY_DISALLOWED_COMMAND: &str = "dm.security.disallowed_command";
}
