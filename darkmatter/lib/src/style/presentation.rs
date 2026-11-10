//! Theme, code-block mode, and line-number defaults, resolved after the style
//! layers merge.
//!
//! A render's presentation has three sources, in precedence order:
//!
//! 1. the effective style (`page.theme`, `page.code.theme`, `page.code.mode`,
//!    `code-block.line-numbers`);
//! 2. the request's captured environment (`THEME`, `CODE_THEME`), held by
//!    [`PresentationDefaults`];
//! 3. the built-in defaults: the `one-half` prose theme, a code theme derived
//!    from the effective prose theme, the `inverse` code-block mode, and no
//!    line numbers.
//!
//! A fence's own `theme=` and a directly constructed code block's theme sit
//! above all of these; they are read at render time and never pass through
//! here. Color mode is not decided here either: it comes from the
//! [`Terminal`](biscuit_terminal::terminal::Terminal) or output options the
//! page was built from.
//!
//! ## Examples
//!
//! ```
//! use std::collections::HashMap;
//!
//! use darkmatter::markdown::highlighting::ThemePair;
//! use darkmatter::style::{PresentationDefaults, StyleFrontmatter};
//!
//! let env = HashMap::from([("THEME".to_string(), "dracula".to_string())]);
//! let resolved = PresentationDefaults::from_env(&env).resolve(&StyleFrontmatter::default());
//! assert_eq!(resolved.prose_theme, ThemePair::Dracula);
//! assert_eq!(resolved.code_theme, ThemePair::Dracula);
//! ```

use std::collections::HashMap;

use biscuit_file::FileResolutionContext;

use crate::layout::DarkmatterPage;
use crate::markdown::compose::RequestSnapshot;
use crate::markdown::highlighting::{CodeBlockMode, ThemePair, get_code_theme_for_prose};
use crate::style::schema::StyleFrontmatter;

/// The prose and code theme defaults a request captured from its environment.
///
/// Built from a request snapshot's environment (never the live process
/// environment), so a render reads `THEME` and `CODE_THEME` as they were when
/// the request was prepared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresentationDefaults {
    theme: Option<String>,
    code_theme: Option<String>,
}

impl PresentationDefaults {
    /// Defaults from the raw `THEME` and `CODE_THEME` values; `None` is an
    /// unset variable.
    pub fn new(theme: Option<String>, code_theme: Option<String>) -> Self {
        Self { theme, code_theme }
    }

    /// Defaults from a captured environment map.
    pub fn from_env(env: &HashMap<String, String>) -> Self {
        Self::new(env.get("THEME").cloned(), env.get("CODE_THEME").cloned())
    }

    /// Defaults from a request snapshot's captured environment.
    pub fn from_snapshot(snapshot: &RequestSnapshot) -> Self {
        Self::from_env(snapshot.env())
    }

    /// Defaults from a resolution context's captured environment.
    pub fn from_context(context: &FileResolutionContext) -> Self {
        Self::from_env(context.env())
    }

    /// Resolves the presentation of `style`, an effective (merged) style.
    ///
    /// A present environment value is read with
    /// [`ThemePair::from_str_or_default`], so an unknown name falls back to
    /// `one-half` as it always has; an absent `CODE_THEME` derives the code
    /// theme from the effective prose theme, including one the style set.
    pub fn resolve(&self, style: &StyleFrontmatter) -> Presentation {
        let page = style.page.as_ref();
        let prose_theme = page
            .and_then(|page| page.theme)
            .or_else(|| self.theme.as_deref().map(ThemePair::from_str_or_default))
            .unwrap_or(ThemePair::OneHalf);
        let style_code_theme = page.and_then(|page| page.code.as_ref()).and_then(|code| code.theme);
        let code_theme = style_code_theme
            .or_else(|| self.code_theme.as_deref().map(ThemePair::from_str_or_default))
            .unwrap_or_else(|| get_code_theme_for_prose(prose_theme));
        let code_block_mode = page
            .and_then(|page| page.code.as_ref())
            .and_then(|code| code.mode)
            .unwrap_or(CodeBlockMode::Inverse);
        let line_numbers = style
            .code_block
            .as_ref()
            .and_then(|code_block| code_block.line_numbers)
            .unwrap_or(false);
        Presentation {
            prose_theme,
            code_theme,
            code_block_mode,
            line_numbers,
            page_code_theme: style_code_theme,
        }
    }
}

/// A render's resolved presentation, from [`PresentationDefaults::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presentation {
    pub prose_theme: ThemePair,
    pub code_theme: ThemePair,
    pub code_block_mode: CodeBlockMode,
    pub line_numbers: bool,
    /// The code theme the style itself set, if any. The page keeps it as its
    /// page code theme, which a direct component theme still outranks.
    pub page_code_theme: Option<ThemePair>,
}

impl Presentation {
    /// Sets these values on `page`, which carries them to both the terminal
    /// and the browser fold.
    pub fn apply_to(self, page: DarkmatterPage) -> DarkmatterPage {
        let page = page
            .with_prose_theme(self.prose_theme.kebab_name())
            .with_code_theme(self.code_theme.kebab_name())
            .with_code_block_mode(self.code_block_mode)
            .with_line_numbers(self.line_numbers);
        match self.page_code_theme {
            Some(theme) => page.with_page_code_theme(theme),
            None => page,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(json: serde_json::Value) -> StyleFrontmatter {
        serde_json::from_value(json).expect("style")
    }

    #[test]
    fn absent_everything_resolves_to_built_in_defaults() {
        let resolved = PresentationDefaults::default().resolve(&StyleFrontmatter::default());
        assert_eq!(resolved.prose_theme, ThemePair::OneHalf);
        assert_eq!(resolved.code_theme, get_code_theme_for_prose(ThemePair::OneHalf));
        assert_eq!(resolved.code_block_mode, CodeBlockMode::Inverse);
        assert!(!resolved.line_numbers);
        assert_eq!(resolved.page_code_theme, None);
    }

    #[test]
    fn unknown_environment_names_fall_back_like_from_str_or_default() {
        let defaults = PresentationDefaults::new(Some("neon".into()), Some(String::new()));
        let resolved = defaults.resolve(&StyleFrontmatter::default());
        assert_eq!(resolved.prose_theme, ThemePair::OneHalf);
        assert_eq!(resolved.code_theme, ThemePair::OneHalf);
    }

    #[test]
    fn style_prose_theme_drives_the_derived_code_theme() {
        let defaults = PresentationDefaults::new(Some("github".into()), None);
        let resolved = defaults.resolve(&style(serde_json::json!({ "page": { "theme": "nord" } })));
        assert_eq!(resolved.prose_theme, ThemePair::Nord);
        assert_eq!(resolved.code_theme, get_code_theme_for_prose(ThemePair::Nord));
    }

    #[test]
    fn a_present_code_theme_default_beats_the_derived_one() {
        let defaults = PresentationDefaults::new(None, Some("gruvbox".into()));
        let resolved = defaults.resolve(&style(serde_json::json!({ "page": { "theme": "nord" } })));
        assert_eq!(resolved.code_theme, ThemePair::Gruvbox);
    }
}
