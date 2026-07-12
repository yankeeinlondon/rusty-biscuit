//! The frozen V1 semantic-token wire legend (spec `DMLS Semantic Tokens`).
//!
//! [`TokenType`], [`modifier`], and [`legend`] are one wire contract: the enum
//! discriminants and modifier bit positions **are** the legend indices encoded
//! on the wire, so all three are frozen in lock-step here. Reordering any of
//! them invalidates every client's token cache and is effectively a protocol
//! migration.
//!
//! This is a leaf module (only `lsp-types`) because it has two consumers on
//! opposite sides of the crate's dependency direction: [`crate::capabilities`]
//! advertises the legend in the server capabilities, and
//! [`crate::providers::semantic_tokens`] encodes against it. Keeping it
//! upstream of both means capability advertisement never imports a provider.

use lsp_types::SemanticTokensLegend;

/// Standard LSP token types used by the V1 legend, in protocol order.
///
/// The discriminant **is** the legend index encoded on the wire, so the
/// declaration order is frozen. `function`, `variable`, `number`, and
/// `operator` are unused by the V1 families (F1/F2/F4) but are reserved so the
/// fine-grained expression phase (F3) never has to reorder the legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TokenType {
    /// `macro` — interpolation spans and directive keywords.
    Macro = 0,
    /// `function` — expression function names (reserved for F3).
    Function = 1,
    /// `variable` — expression identifiers (reserved for F3).
    Variable = 2,
    /// `property` — directive option keys.
    Property = 3,
    /// `string` — directive option values and wiki inner segments.
    Str = 4,
    /// `number` — expression number literals (reserved for F3).
    Number = 5,
    /// `operator` — expression operators (reserved for F3).
    Operator = 6,
}

impl TokenType {
    /// The wire legend index for this type.
    pub(crate) fn index(self) -> u32 {
        self as u32
    }
}

/// Custom + standard token modifiers, as single-bit masks.
///
/// The bit position **is** the legend index (bit `n` ⇒ legend entry `n`), so
/// this order is frozen alongside [`TokenType`]. The five custom modifiers come
/// first (the V1 targeting surface), then the two standard modifiers required
/// by the future fine-grained phase.
pub mod modifier {
    /// Every token inside (and including) a `{{ }}` / `{{{ }}}` span.
    pub const INTERPOLATION: u32 = 1 << 0;
    /// A `{{{ … }}}` literal span (same family as interpolation, distinguishable).
    pub const INERT: u32 = 1 << 1;
    /// Every token on a directive line.
    pub const DIRECTIVE: u32 = 1 << 2;
    /// The three structural closers (`::end-block`, `::details`, `::end-disclosure`).
    pub const CLOSER: u32 = 1 << 3;
    /// Every token in a `[[wiki]]` link.
    pub const WIKI: u32 = 1 << 4;
    /// Reserved: transcluded-target paths (future).
    pub const INJECTED: u32 = 1 << 5;
    /// Standard `defaultLibrary` — `ctx.*` / `env.*` roots and functions (F3).
    pub const DEFAULT_LIBRARY: u32 = 1 << 6;
    /// Standard `readonly` — `ctx.*` (F3).
    pub const READONLY: u32 = 1 << 7;
}

/// The frozen V1 legend, in the exact wire order [`TokenType`] and [`modifier`]
/// encode.
///
/// Reordering either list is a protocol migration; keep this in lock-step with
/// the discriminants above.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: [
            "macro", "function", "variable", "property", "string", "number", "operator",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        token_modifiers: [
            "interpolation",
            "inert",
            "directive",
            "closer",
            "wiki",
            "injected",
            "defaultLibrary",
            "readonly",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
    }
}
