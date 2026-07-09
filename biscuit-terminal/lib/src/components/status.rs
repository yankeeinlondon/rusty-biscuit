use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::discovery::detection::{ColorDepth, ColorMode};
use crate::terminal::Terminal;
use crate::utils::block_constraint::wrap_lines;
use crate::utils::color::{Color, Tailwind, TailwindColorWrapper};
use crate::utils::layout::Layout;
use crate::utils::layout::RenderableWrapper;
use crate::utils::wrap_policy::WordWrap;

// ── Nerd Font icons ── Circular theme ──────────────────────────────────────

const NERD_CIRCULAR_NOT_STARTED: &str = "\u{f4aa}";
const NERD_CIRCULAR_ACTIVE: &str = "\u{f0ec2}";
const NERD_CIRCULAR_SUCCESS: &str = "\u{f05e0}";
const NERD_CIRCULAR_FAILURE: &str = "\u{f057}";
const NERD_CIRCULAR_WARNING: &str = "\u{f0028}";
const NERD_CIRCULAR_INFO: &str = "\u{f449}";
const NERD_CIRCULAR_TOOL_USE: &str = "\u{f425}";
const NERD_CIRCULAR_SUBAGENT: &str = "\u{f0012}";

// ── Nerd Font icons ── Rounded theme ───────────────────────────────────────

const NERD_ROUNDED_NOT_STARTED: &str = "\u{ea72}";
const NERD_ROUNDED_ACTIVE: &str = "\u{f1500}";
const NERD_ROUNDED_SUCCESS: &str = "\u{f14a}";
const NERD_ROUNDED_FAILURE: &str = "\u{f136e}";
const NERD_ROUNDED_WARNING: &str = "\u{f0af}";
const NERD_ROUNDED_INFO: &str = "\u{f0bd4}";
const NERD_ROUNDED_TOOL_USE: &str = "\u{f425}";
const NERD_ROUNDED_SUBAGENT: &str = "\u{f0012}";

// ── Nerd Font icons ── Timeline theme ──────────────────────────────────────

const NERD_TIMELINE_NOT_STARTED: &str = "\u{f0bd2}";
const NERD_TIMELINE_ACTIVE: &str = "\u{f0bd1}";
const NERD_TIMELINE_SUCCESS: &str = "\u{f1532}";
const NERD_TIMELINE_FAILURE: &str = "\u{f1537}";
const NERD_TIMELINE_WARNING: &str = "\u{f0f95}";
const NERD_TIMELINE_INFO: &str = "\u{f0bd4}";
const NERD_TIMELINE_TOOL_USE: &str = "\u{f425}";
const NERD_TIMELINE_SUBAGENT: &str = "\u{f0012}";

// ── Unicode fallback icons (shared across all themes) ──────────────────────

const FB_NOT_STARTED: &str = "\u{25fb}"; // ◻
const FB_ACTIVE: &str = "\u{25fd}"; // ◽
const FB_SUCCESS: &str = "\u{2713}"; // ✓
// U+292B (RISING DIAGONAL CROSSING FALLING DIAGONAL — `⤫`). The previous
// constant pointed at U+2A2B (`⨫`, "slanted equal to or less-than"), which
// shipped only because the inline comment looked identical to the intended
// glyph. `StatusBlock::severity_icon` already uses `\u{292b}`; sharing the
// value here keeps the bespoke and tree-rendering paths visually consistent.
const FB_FAILURE: &str = "\u{292b}"; // ⤫
const FB_WARNING: &str = "\u{26a0}"; // ⚠
const FB_INFO: &str = "\u{2139}"; // ℹ
const FB_TOOL_USE: &str = "\u{1f527}"; // 🔧
const FB_SUBAGENT: &str = "\u{1f916}"; // 🤖

/// The state of a status item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusState {
    NotStarted,
    Active,
    Success,
    #[serde(alias = "Failure")]
    Error,
    #[serde(skip_deserializing)]
    #[deprecated(note = "use StatusState::Error instead")]
    Failure,
    Warning,
    Info,
    ToolUse,
    Subagent,
}

impl StatusState {
    /// Canonical Tailwind color for this variant.
    pub fn default_color(&self) -> Color {
        match self {
            Self::NotStarted => Color::Tailwind(Tailwind::Gray500),
            Self::Active => Color::Tailwind(Tailwind::Gray600),
            Self::Success => Color::Tailwind(Tailwind::Green500),
            Self::Error => Color::Tailwind(Tailwind::Red500),
            #[allow(deprecated)]
            Self::Failure => Color::Tailwind(Tailwind::Red500),
            Self::Warning => Color::Tailwind(Tailwind::Orange500),
            Self::Info => Color::Tailwind(Tailwind::Blue500),
            Self::ToolUse => Color::Tailwind(Tailwind::Purple700),
            Self::Subagent => Color::Tailwind(Tailwind::Violet500),
        }
    }
}

/// Visual theme controlling the icon set used by [`Status`].
///
/// - **Circular** and **Rounded** use circle or rounded-square shaped icons.
/// - **Timeline** adds a vertical bar to the left so stacked items form a timeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StatusTheme {
    #[default]
    Circular,
    Rounded,
    Timeline,
}

/// Internal icon definition for one (theme, state) combination.
struct StatusIconDef {
    nerd: &'static str,
    fallback: &'static str,
    color: Tailwind,
    /// Alternative color used when the terminal is in light mode.
    /// Corresponds to the second value in `gray-600/400` style spec notation.
    color_alt: Option<Tailwind>,
}

/// Lookup table for all 24 (theme, state) icon definitions.
static ICON_LOOKUP: LazyLock<HashMap<(StatusTheme, StatusState), StatusIconDef>> =
    LazyLock::new(|| {
        let mut m = HashMap::with_capacity(24);

        // ── Circular ───────────────────────────────────────────────────
        m.insert(
            (StatusTheme::Circular, StatusState::NotStarted),
            StatusIconDef {
                nerd: NERD_CIRCULAR_NOT_STARTED,
                fallback: FB_NOT_STARTED,
                color: Tailwind::Gray500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Active),
            StatusIconDef {
                nerd: NERD_CIRCULAR_ACTIVE,
                fallback: FB_ACTIVE,
                color: Tailwind::Gray600,
                color_alt: Some(Tailwind::Gray400),
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Success),
            StatusIconDef {
                nerd: NERD_CIRCULAR_SUCCESS,
                fallback: FB_SUCCESS,
                color: Tailwind::Green500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Error),
            StatusIconDef {
                nerd: NERD_CIRCULAR_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        #[allow(deprecated)]
        m.insert(
            (StatusTheme::Circular, StatusState::Failure),
            StatusIconDef {
                nerd: NERD_CIRCULAR_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Warning),
            StatusIconDef {
                nerd: NERD_CIRCULAR_WARNING,
                fallback: FB_WARNING,
                color: Tailwind::Orange500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Info),
            StatusIconDef {
                nerd: NERD_CIRCULAR_INFO,
                fallback: FB_INFO,
                color: Tailwind::Blue500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::ToolUse),
            StatusIconDef {
                nerd: NERD_CIRCULAR_TOOL_USE,
                fallback: FB_TOOL_USE,
                color: Tailwind::Purple700,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Circular, StatusState::Subagent),
            StatusIconDef {
                nerd: NERD_CIRCULAR_SUBAGENT,
                fallback: FB_SUBAGENT,
                color: Tailwind::Violet500,
                color_alt: None,
            },
        );

        // ── Rounded ────────────────────────────────────────────────────
        m.insert(
            (StatusTheme::Rounded, StatusState::NotStarted),
            StatusIconDef {
                nerd: NERD_ROUNDED_NOT_STARTED,
                fallback: FB_NOT_STARTED,
                color: Tailwind::Gray500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Active),
            StatusIconDef {
                nerd: NERD_ROUNDED_ACTIVE,
                fallback: FB_ACTIVE,
                color: Tailwind::Gray600,
                color_alt: Some(Tailwind::Gray400),
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Success),
            StatusIconDef {
                nerd: NERD_ROUNDED_SUCCESS,
                fallback: FB_SUCCESS,
                color: Tailwind::Green500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Error),
            StatusIconDef {
                nerd: NERD_ROUNDED_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        #[allow(deprecated)]
        m.insert(
            (StatusTheme::Rounded, StatusState::Failure),
            StatusIconDef {
                nerd: NERD_ROUNDED_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Warning),
            StatusIconDef {
                nerd: NERD_ROUNDED_WARNING,
                fallback: FB_WARNING,
                color: Tailwind::Orange500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Info),
            StatusIconDef {
                nerd: NERD_ROUNDED_INFO,
                fallback: FB_INFO,
                color: Tailwind::Orange500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::ToolUse),
            StatusIconDef {
                nerd: NERD_ROUNDED_TOOL_USE,
                fallback: FB_TOOL_USE,
                color: Tailwind::Purple700,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Rounded, StatusState::Subagent),
            StatusIconDef {
                nerd: NERD_ROUNDED_SUBAGENT,
                fallback: FB_SUBAGENT,
                color: Tailwind::Violet500,
                color_alt: None,
            },
        );

        // ── Timeline ───────────────────────────────────────────────────
        m.insert(
            (StatusTheme::Timeline, StatusState::NotStarted),
            StatusIconDef {
                nerd: NERD_TIMELINE_NOT_STARTED,
                fallback: FB_NOT_STARTED,
                color: Tailwind::Gray500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Active),
            StatusIconDef {
                nerd: NERD_TIMELINE_ACTIVE,
                fallback: FB_ACTIVE,
                color: Tailwind::Gray600,
                color_alt: Some(Tailwind::Gray400),
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Success),
            StatusIconDef {
                nerd: NERD_TIMELINE_SUCCESS,
                fallback: FB_SUCCESS,
                color: Tailwind::Green500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Error),
            StatusIconDef {
                nerd: NERD_TIMELINE_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        #[allow(deprecated)]
        m.insert(
            (StatusTheme::Timeline, StatusState::Failure),
            StatusIconDef {
                nerd: NERD_TIMELINE_FAILURE,
                fallback: FB_FAILURE,
                color: Tailwind::Red500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Warning),
            StatusIconDef {
                nerd: NERD_TIMELINE_WARNING,
                fallback: FB_WARNING,
                color: Tailwind::Orange500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Info),
            StatusIconDef {
                nerd: NERD_TIMELINE_INFO,
                fallback: FB_INFO,
                color: Tailwind::Orange500,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::ToolUse),
            StatusIconDef {
                nerd: NERD_TIMELINE_TOOL_USE,
                fallback: FB_TOOL_USE,
                color: Tailwind::Purple700,
                color_alt: None,
            },
        );
        m.insert(
            (StatusTheme::Timeline, StatusState::Subagent),
            StatusIconDef {
                nerd: NERD_TIMELINE_SUBAGENT,
                fallback: FB_SUBAGENT,
                color: Tailwind::Violet500,
                color_alt: None,
            },
        );

        m
    });

/// A status indicator with themed icons for terminal rendering.
///
/// Status represents a validation or action-item state rendered with
/// configurable icon themes. Unlike [`Todo`](super::todo::Todo), Status
/// uses plain Unicode/Nerd Font icons (no GFM square brackets) and
/// supports multiple visual themes.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// let status = Status::new("Deploy to production")
///     .state(StatusState::Success)
///     .theme(StatusTheme::Timeline);
///
/// let output = status.render_optimistic(Some(80));
/// assert!(output.contains("Deploy to production"));
/// ```
///
/// ## Themes
///
/// | Theme    | Description |
/// |----------|-------------|
/// | Circular | Circle-shaped icons (default) |
/// | Rounded  | Rounded-square shaped icons |
/// | Timeline | Vertical-bar icons for stacked timeline display |
///
/// ## States
///
/// | State      | Fallback | Color      |
/// |------------|----------|------------|
/// | NotStarted | ◻        | gray-500   |
/// | Active     | ◽        | gray-600/400 |
/// | Success    | ✓        | green-500  |
/// | Failure    | ⤫        | red-500    |
/// | Warning    | ⚠        | orange-500 |
/// | Info       | ℹ        | blue-500   |
/// | ToolUse    | 🔧        | purple-700 |
/// | Subagent   | 🤖        | violet-500 |
///
/// ## Layout & Style Contract
///
/// `Status` is classified as an **inline badge** (spec C7). A status line is
/// a single inline run (icon + description) — it does not own a block box,
/// so the layout-box properties are **N/A** here. When `Status` is composed
/// into a block container (e.g. via `Compose` or `Section`), the containing
/// block owns the box and `Status`'s content flows inline within it.
///
/// The bespoke terminal render path (`to_terminal`) honors icon color from
/// `StatusState::default_color` and routes the description through `Prose`
/// when `use_prose = true`, so inherited `color` and `emphasis` flow through.
/// Only `Layout::word_wrap` is applied by [`TerminalRenderable::render`];
/// margins, alignment, `max_width`, `width`, and `padding` are ignored.
///
/// | Property | Status | Rationale |
/// |----------|--------|-----------|
/// | `Layout::word_wrap` | **Honored** | Wraps the full status line (icon + description) using the configured strategy. Default `WrapProse` with a 2-cell hanging indent for the plain constructor; `WrapProse(Some(8), Some(2))` for `from_prose`. |
/// | `Layout::margin` / `alignment` / `max_width` / `width` / `padding` | **N/A** | Inline badge — no block box. Compose inside a `Section` / `Compose` for block placement. |
/// | `Style::color` | **Honored** (via the state's `default_color` and the Tailwind icon palette) | The icon color flows from `StatusState::default_color`; the description inherits the terminal's current color. |
/// | `Style::emphasis` | **Honored** (via `Prose` when `use_prose = true`) | The `from_prose` constructor enables markup like `<b>` / `<red>` which lowers to `emphasis` / `color`. |
/// | `Style::background` | **N/A** | Inline background has no padding box; the icon glyph and description are the inline content. |
/// | `Style::border` | **N/A** | No inline-border design exists; the status glyph is its own visual terminator. |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    state: StatusState,
    theme: StatusTheme,
    description: String,
    color_icons: bool,
    #[serde(skip)]
    use_prose: bool,
    #[serde(skip)]
    layout: Layout,
}

impl Default for Status {
    fn default() -> Self {
        let mut layout = Layout::default();
        layout.word_wrap = layout.word_wrap.with_hanging_indent(2);
        Self {
            state: StatusState::NotStarted,
            theme: StatusTheme::Circular,
            description: String::new(),
            color_icons: true,
            use_prose: false,
            layout,
        }
    }
}

impl Status {
    /// Create a new status item with the given description.
    ///
    /// Defaults to `NotStarted` state with the `Circular` theme.
    pub fn new<T: Into<String>>(desc: T) -> Self {
        Self {
            description: desc.into(),
            ..Self::default()
        }
    }

    /// Create a new status item with a prose-formatted description.
    ///
    /// The description is rendered through [`Prose`] at render time,
    /// enabling markup like `<b>bold</b>` and `<red>color</red>`.
    pub fn from_prose<T: Into<String>>(desc: T) -> Self {
        Self {
            description: desc.into(),
            use_prose: true,
            layout: Layout {
                word_wrap: WordWrap::WrapProse(Some(8), Some(2)),
                ..Layout::default()
            },
            ..Self::default()
        }
    }

    /// Set the status state.
    pub fn state(mut self, state: StatusState) -> Self {
        self.state = state;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: StatusTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Disable icon colorization.
    ///
    /// By default, icons are rendered with Tailwind-based colors.
    /// Calling this method renders icons without color even when the
    /// terminal supports it.
    pub fn no_color_icons(mut self) -> Self {
        self.color_icons = false;
        self
    }

    /// Render the status line for the given terminal.
    fn to_terminal(&self, term: &Terminal) -> String {
        let key = (self.theme.clone(), self.state.clone());
        let fallback_key = (StatusTheme::Circular, StatusState::NotStarted);
        let icon_def = ICON_LOOKUP
            .get(&key)
            .unwrap_or_else(|| &ICON_LOOKUP[&fallback_key]);

        let icon_char = match term.is_nerd_font {
            Some(true) => icon_def.nerd,
            _ => icon_def.fallback,
        };

        let has_color = term.color_depth != ColorDepth::None;
        let should_colorize = self.color_icons && has_color;

        let icon = if should_colorize {
            let tw_color = match (term.color_mode(), icon_def.color_alt) {
                (ColorMode::Light, Some(alt)) => alt,
                _ => icon_def.color,
            };
            TailwindColorWrapper(tw_color).fallback_render(icon_char, term)
        } else {
            icon_char.to_string()
        };

        let desc = if self.use_prose {
            Prose::new(&self.description).render(term)
        } else {
            self.description.clone()
        };

        format!("{icon} {desc}")
    }
}

impl TerminalRenderable for Status {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.to_terminal(term);
        if width == 0 {
            return String::new();
        }
        // `Status` is an inline badge (spec C7): only `Layout::word_wrap` is
        // honored here. Margin, alignment, max_width, width, and padding are
        // N/A — the containing block owns the box when Status is composed
        // inside a block component.
        match &self.layout.word_wrap {
            WordWrap::None => content,
            strategy => wrap_lines(vec![content], strategy, width).join("\n"),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    fn no_color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    fn color_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::TrueColor)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    fn nerd_terminal() -> Terminal {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::TrueColor)
            .build();
        term.is_nerd_font = Some(true);
        term
    }

    // ── No-color fallback tests ────────────────────────────────────────

    #[test]
    fn no_color_not_started() {
        let term = no_color_terminal();
        let status = Status::new("Pending review");
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_NOT_STARTED} Pending review"));
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn no_color_active() {
        let term = no_color_terminal();
        let status = Status::new("Running").state(StatusState::Active);
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_ACTIVE} Running"));
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn no_color_success() {
        let term = no_color_terminal();
        let status = Status::new("Passed").state(StatusState::Success);
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_SUCCESS} Passed"));
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn no_color_failure() {
        let term = no_color_terminal();
        let status = Status::new("Failed").state(StatusState::Failure);
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_FAILURE} Failed"));
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn no_color_warning() {
        let term = no_color_terminal();
        let status = Status::new("Degraded").state(StatusState::Warning);
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_WARNING} Degraded"));
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn no_color_info() {
        let term = no_color_terminal();
        let status = Status::new("FYI").state(StatusState::Info);
        let result = status.to_terminal(&term);
        assert_eq!(result, format!("{FB_INFO} FYI"));
        assert!(!result.contains('\x1b'));
    }

    // ── Color rendering tests ──────────────────────────────────────────

    #[test]
    fn color_success_has_ansi() {
        let term = color_terminal();
        let status = Status::new("Done").state(StatusState::Success);
        let result = status.to_terminal(&term);
        assert!(
            result.contains('\x1b'),
            "Expected ANSI codes in colored output: {:?}",
            result
        );
        assert!(result.contains("Done"));
    }

    #[test]
    fn color_failure_has_ansi() {
        let term = color_terminal();
        let status = Status::new("Broken").state(StatusState::Failure);
        let result = status.to_terminal(&term);
        assert!(result.contains('\x1b'));
        assert!(result.contains("Broken"));
    }

    // ── Nerd font tests ────────────────────────────────────────────────

    #[test]
    fn nerd_font_uses_nerd_icons() {
        let term = nerd_terminal();
        let status = Status::new("Task").state(StatusState::Success);
        let result = status.to_terminal(&term);
        // The nerd icon for circular success is \u{f05e0}
        assert!(
            result.contains(NERD_CIRCULAR_SUCCESS),
            "Expected nerd icon in output: {:?}",
            result
        );
    }

    #[test]
    fn non_nerd_uses_fallback_icons() {
        let term = color_terminal();
        let status = Status::new("Task").state(StatusState::Success);
        let result = status.to_terminal(&term);
        assert!(
            result.contains(FB_SUCCESS),
            "Expected fallback icon in output: {:?}",
            result
        );
    }

    // ── Theme variation tests ──────────────────────────────────────────

    #[test]
    fn rounded_theme_uses_different_nerd_icons() {
        let term = nerd_terminal();
        let circular = Status::new("Task")
            .state(StatusState::Success)
            .to_terminal(&term);
        let rounded = Status::new("Task")
            .state(StatusState::Success)
            .theme(StatusTheme::Rounded)
            .to_terminal(&term);
        // Different nerd icons for different themes
        assert_ne!(circular, rounded);
        assert!(circular.contains(NERD_CIRCULAR_SUCCESS));
        assert!(rounded.contains(NERD_ROUNDED_SUCCESS));
    }

    #[test]
    fn timeline_theme_uses_different_nerd_icons() {
        let term = nerd_terminal();
        let circular = Status::new("Task")
            .state(StatusState::Success)
            .to_terminal(&term);
        let timeline = Status::new("Task")
            .state(StatusState::Success)
            .theme(StatusTheme::Timeline)
            .to_terminal(&term);
        assert_ne!(circular, timeline);
        assert!(timeline.contains(NERD_TIMELINE_SUCCESS));
    }

    // ── Builder tests ──────────────────────────────────────────────────

    #[test]
    fn no_color_icons_disables_coloring() {
        let term = color_terminal();
        let status = Status::new("Test")
            .state(StatusState::Success)
            .no_color_icons();
        let result = status.to_terminal(&term);
        assert!(
            !result.contains('\x1b'),
            "Expected no ANSI codes with no_color_icons: {:?}",
            result
        );
    }

    #[test]
    fn default_state_is_not_started_circular() {
        let status = Status::new("New item");
        assert_eq!(status.state, StatusState::NotStarted);
        assert_eq!(status.theme, StatusTheme::Circular);
        assert!(status.color_icons);
    }

    #[test]
    fn builder_chaining() {
        let status = Status::new("Chained")
            .state(StatusState::Warning)
            .theme(StatusTheme::Timeline)
            .no_color_icons();
        assert_eq!(status.state, StatusState::Warning);
        assert_eq!(status.theme, StatusTheme::Timeline);
        assert!(!status.color_icons);
        assert_eq!(status.description, "Chained");
    }

    // ── TerminalRenderable trait tests ─────────────────────────────────────────

    #[test]
    fn render_optimistic_works() {
        let status = Status::new("Optimistic test").state(StatusState::Info);
        let result = status.render_optimistic(Some(80));
        assert!(result.contains("Optimistic test"));
    }

    #[test]
    fn render_with_terminal_works() {
        let term = color_terminal();
        let status = Status::new("Terminal test").state(StatusState::Active);
        let result = status.render(&term);
        assert!(result.contains("Terminal test"));
    }

    #[test]
    fn display_appends_newline() {
        let term = no_color_terminal();
        let status = Status::new("Newline test");
        let result = status.display(&term);
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn default_color_error_is_red500() {
        assert_eq!(
            StatusState::Error.default_color(),
            Color::Tailwind(Tailwind::Red500)
        );
    }

    #[test]
    fn failure_alias_deserializes_as_error() {
        let state = serde_json::from_str::<StatusState>("\"Failure\"").unwrap();
        assert_eq!(state, StatusState::Error);
    }
}
