use std::sync::OnceLock;

use termini::{NumberCapability, TermInfo};

/// Standard terminal tab interval used when terminfo does not report one.
pub const DEFAULT_TAB_WIDTH: usize = 8;

static TAB_WIDTH: OnceLock<usize> = OnceLock::new();

/// Detects the terminal's initial horizontal-tab interval.
///
/// The value comes from terminfo's `init_tabs` capability and falls back to
/// the standard eight-column interval. Terminals do not expose a portable way
/// to query tab stops changed after startup.
pub fn tab_width() -> usize {
    *TAB_WIDTH.get_or_init(|| {
        let detected = TermInfo::from_env()
            .ok()
            .and_then(|term_info| term_info.number_cap(NumberCapability::InitTabs));
        resolve_tab_width(detected)
    })
}

fn resolve_tab_width(detected: Option<i32>) -> usize {
    detected
        .and_then(|width| usize::try_from(width).ok())
        .filter(|width| *width > 0)
        .unwrap_or(DEFAULT_TAB_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_positive_terminfo_width() {
        assert_eq!(resolve_tab_width(Some(4)), 4);
        assert_eq!(resolve_tab_width(Some(12)), 12);
    }

    #[test]
    fn falls_back_for_missing_or_invalid_width() {
        assert_eq!(resolve_tab_width(None), DEFAULT_TAB_WIDTH);
        assert_eq!(resolve_tab_width(Some(0)), DEFAULT_TAB_WIDTH);
        assert_eq!(resolve_tab_width(Some(-1)), DEFAULT_TAB_WIDTH);
    }
}
