//! `HrStyle` — horizontal-rule bucket.
//!
//! The legacy per-block `style: waves` attribute is migrated to
//! `style.hr.kind` by sub-spec #6. v1 schema carries `kind` as an opaque
//! `Option<String>`.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HrStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    pub kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kind() {
        let h: HrStyle = serde_json::from_str(r#"{"kind": "waves"}"#).unwrap();
        assert_eq!(h.kind.as_deref(), Some("waves"));
    }

    #[test]
    fn empty_yields_default() {
        let h: HrStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(h, HrStyle::default());
    }
}
