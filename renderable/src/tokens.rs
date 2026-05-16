//! Compiler-checked semantic CSS-variable tokens.
//!
//! Three families ship: colors, spacing, typography. Each token is a
//! typed enum variant that knows its `--name`, its default value, and
//! how to reference itself as `var(--name)`. The page declares these in
//! its `:root` block; components consume them via [`SemanticToken::var`]
//! and friends. Components must reference this semantic layer, not the
//! open palette layer (`Color::Var`).

/// A curated semantic color token. Defaults reference the palette layer
/// so a caller re-themes by overriding the semantic token, not the
/// palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticToken {
    /// Page background color.
    Bg,
    /// Primary foreground / text color.
    Fg,
    /// Muted / secondary foreground color.
    FgMuted,
    /// Accent color for interactive elements.
    Accent,
    /// Error / danger color.
    Error,
    /// Warning color.
    Warning,
    /// Success color.
    Success,
    /// Default border color.
    Border,
}

impl SemanticToken {
    /// Every semantic color token, in declaration order.
    pub const ALL: [SemanticToken; 8] = [
        SemanticToken::Bg,
        SemanticToken::Fg,
        SemanticToken::FgMuted,
        SemanticToken::Accent,
        SemanticToken::Error,
        SemanticToken::Warning,
        SemanticToken::Success,
        SemanticToken::Border,
    ];

    /// The custom-property name, without the leading `--`.
    pub fn name(&self) -> &'static str {
        match self {
            SemanticToken::Bg => "color-bg",
            SemanticToken::Fg => "color-fg",
            SemanticToken::FgMuted => "color-fg-muted",
            SemanticToken::Accent => "color-accent",
            SemanticToken::Error => "color-error",
            SemanticToken::Warning => "color-warning",
            SemanticToken::Success => "color-success",
            SemanticToken::Border => "color-border",
        }
    }

    /// The default CSS value for this token. Defaults reference Tailwind
    /// palette hex values directly so the semantic layer is self-contained.
    pub fn default_value(&self) -> &'static str {
        match self {
            SemanticToken::Bg => "#ffffff",
            SemanticToken::Fg => "#1f2937",
            SemanticToken::FgMuted => "#6b7280",
            SemanticToken::Accent => "#3b82f6",
            SemanticToken::Error => "#ef4444",
            SemanticToken::Warning => "#f59e0b",
            SemanticToken::Success => "#22c55e",
            SemanticToken::Border => "#e5e7eb",
        }
    }

    /// The `var(--name)` reference form for use in component CSS.
    pub fn var(&self) -> String {
        format!("var(--{})", self.name())
    }
}

/// A spacing-scale token. Values follow a Tailwind-style rem scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpaceToken {
    /// `0.25rem`
    One,
    /// `0.5rem`
    Two,
    /// `0.75rem`
    Three,
    /// `1rem`
    Four,
    /// `1.5rem`
    Six,
    /// `2rem`
    Eight,
}

impl SpaceToken {
    /// Every spacing token, in scale order.
    pub const ALL: [SpaceToken; 6] = [
        SpaceToken::One,
        SpaceToken::Two,
        SpaceToken::Three,
        SpaceToken::Four,
        SpaceToken::Six,
        SpaceToken::Eight,
    ];

    /// The custom-property name, without the leading `--`.
    pub fn name(&self) -> &'static str {
        match self {
            SpaceToken::One => "space-1",
            SpaceToken::Two => "space-2",
            SpaceToken::Three => "space-3",
            SpaceToken::Four => "space-4",
            SpaceToken::Six => "space-6",
            SpaceToken::Eight => "space-8",
        }
    }

    /// The default CSS length value for this token.
    pub fn default_value(&self) -> &'static str {
        match self {
            SpaceToken::One => "0.25rem",
            SpaceToken::Two => "0.5rem",
            SpaceToken::Three => "0.75rem",
            SpaceToken::Four => "1rem",
            SpaceToken::Six => "1.5rem",
            SpaceToken::Eight => "2rem",
        }
    }

    /// The `var(--name)` reference form for use in component CSS.
    pub fn var(&self) -> String {
        format!("var(--{})", self.name())
    }
}

/// A typography token (font family / size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontToken {
    /// Default sans-serif font stack.
    Sans,
    /// Monospace font stack.
    Mono,
    /// Base body font size.
    SizeBase,
    /// Small font size.
    SizeSm,
    /// Large font size.
    SizeLg,
}

impl FontToken {
    /// Every typography token, in declaration order.
    pub const ALL: [FontToken; 5] = [
        FontToken::Sans,
        FontToken::Mono,
        FontToken::SizeBase,
        FontToken::SizeSm,
        FontToken::SizeLg,
    ];

    /// The custom-property name, without the leading `--`.
    pub fn name(&self) -> &'static str {
        match self {
            FontToken::Sans => "font-sans",
            FontToken::Mono => "font-mono",
            FontToken::SizeBase => "font-size-base",
            FontToken::SizeSm => "font-size-sm",
            FontToken::SizeLg => "font-size-lg",
        }
    }

    /// The default CSS value for this token.
    pub fn default_value(&self) -> &'static str {
        match self {
            FontToken::Sans => {
                "ui-sans-serif, system-ui, -apple-system, \
                 'Segoe UI', sans-serif"
            }
            FontToken::Mono => {
                "ui-monospace, SFMono-Regular, 'SF Mono', \
                 Menlo, monospace"
            }
            FontToken::SizeBase => "1rem",
            FontToken::SizeSm => "0.875rem",
            FontToken::SizeLg => "1.125rem",
        }
    }

    /// The `var(--name)` reference form for use in component CSS.
    pub fn var(&self) -> String {
        format!("var(--{})", self.name())
    }
}

/// Returns the full `(name, value)` list of semantic-layer defaults
/// across all three token families, in declaration order.
///
/// `HtmlPage` emits these as the page-level `:root { … }` block unless a
/// caller overrides specific tokens via `PageOptions`. Each pair is
/// `(custom-property-name-without-leading-dashes, css-value)`.
pub fn root_defaults() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for token in SemanticToken::ALL {
        out.push((token.name().to_string(), token.default_value().to_string()));
    }
    for token in SpaceToken::ALL {
        out.push((token.name().to_string(), token.default_value().to_string()));
    }
    for token in FontToken::ALL {
        out.push((token.name().to_string(), token.default_value().to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_token_var_form_is_well_formed() {
        assert_eq!(SemanticToken::Bg.var(), "var(--color-bg)");
        assert_eq!(SemanticToken::Error.var(), "var(--color-error)");
    }

    #[test]
    fn space_token_var_form_is_well_formed() {
        assert_eq!(SpaceToken::Two.var(), "var(--space-2)");
    }

    #[test]
    fn font_token_var_form_is_well_formed() {
        assert_eq!(FontToken::Mono.var(), "var(--font-mono)");
    }

    #[test]
    fn every_color_token_name_uses_the_color_prefix() {
        for token in SemanticToken::ALL {
            assert!(
                token.name().starts_with("color-"),
                "semantic token {token:?} must use the color- prefix"
            );
        }
    }

    #[test]
    fn root_defaults_covers_every_token() {
        let defaults = root_defaults();
        let expected =
            SemanticToken::ALL.len() + SpaceToken::ALL.len() + FontToken::ALL.len();
        assert_eq!(defaults.len(), expected);
        // Names are unique.
        let mut names: Vec<&str> = defaults.iter().map(|(n, _)| n.as_str()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique, "token names must be unique");
    }

    #[test]
    fn root_defaults_values_are_non_empty() {
        for (name, value) in root_defaults() {
            assert!(!name.is_empty(), "token name must not be empty");
            assert!(!value.is_empty(), "token {name} value must not be empty");
        }
    }
}
