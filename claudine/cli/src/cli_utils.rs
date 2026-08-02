use biscuit_terminal::components::table::table::TableCellContent;
use chrono::NaiveDate;
use claudine::events::AgenticEvent;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use color_eyre::eyre::{Result, eyre};
use std::path::{Path, PathBuf};
use url::Url;

/// Convert a local path into a file URL, resolving relative paths against the
/// process working directory.
pub(crate) fn file_url(path: &Path) -> Option<Url> {
    let absolute = if path.is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    Url::from_file_path(absolute).ok()
}

pub(crate) fn parse_provider(name: &str) -> Result<Provider> {
    parse_provider_clap(name).map_err(|message| eyre!(message))
}

pub(crate) fn parse_provider_clap(name: &str) -> std::result::Result<Provider, String> {
    if let Some(provider) = Provider::parse_cli_name(name) {
        return Ok(provider);
    }

    let supported = PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(Provider::as_slug)
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("Unknown provider: {name}. Supported: {supported}"))
}

pub(crate) fn parse_naive_date(value: &str) -> std::result::Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| format!("invalid date `{value}`: {error}"))
}

pub(crate) fn event_name_pascal(slug: &str) -> String {
    AgenticEvent::from_slug(slug)
        .map(|event| event.as_pascal_case().to_string())
        .unwrap_or_else(|| slug.to_string())
}

pub(crate) fn bool_indicator(value: bool) -> TableCellContent {
    if value {
        "\u{2705}".into()
    } else {
        "\u{274C}".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_percent_encodes_reserved_path_characters() {
        let path = std::env::current_dir()
            .expect("current directory")
            .join("dir with space")
            .join("a#b%.md");
        let url = file_url(&path).expect("absolute local path should become a file URL");
        let rendered = url.as_str();
        assert!(rendered.starts_with("file://"), "{rendered}");
        assert!(rendered.contains("dir%20with%20space"), "{rendered}");
        assert!(rendered.contains("a%23b%25.md"), "{rendered}");
    }

    #[cfg(windows)]
    #[test]
    fn file_url_uses_windows_drive_uri_shape() {
        let url = file_url(Path::new(r"C:\docs\a b.md")).expect("drive path should convert");
        assert_eq!(url.as_str(), "file:///C:/docs/a%20b.md");
    }

    #[cfg(windows)]
    #[test]
    fn portable_text_declines_unc_namespace_rewriting() {
        assert!(biscuit_file::try_portable_string(Path::new(r"\\server\share\f.md")).is_none());
    }
}
