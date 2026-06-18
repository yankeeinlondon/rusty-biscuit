use strum::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// File-type and folder icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum File {
    Markdown,
    Pdf,
    Json,
    Toml,
    Yaml,
    Xml,
    WordDoc,
    Spreadsheet,
    Image,
    Svg,
    Css,
    Html,
    Rust,
    Javascript,
    Typescript,
    Python,
    Folder,
    FolderFill,
}

impl DomainIcon for File {
    fn iconify_id(self) -> &'static str {
        match self {
            File::Markdown => "material-symbols:markdown",
            File::Pdf => "ant-design:file-pdf-filled",
            File::Json => "lucide:file-json",
            File::Toml => "file-icons:toml",
            File::Yaml => "file-icons:yaml-alt1",
            File::Xml => "mdi:file-xml-box",
            File::WordDoc => "teenyicons:ms-word-outline",
            File::Spreadsheet => "mdi:spreadsheet",
            File::Image => "material-symbols:image-rounded",
            File::Svg => "ci:file-svg",
            File::Css => "tabler:brand-css3",
            File::Html => "ci:file-html",
            File::Rust => "mdi:language-rust",
            File::Javascript => "proicons:javascript",
            File::Typescript => "mdi:language-typescript",
            File::Python => "mdi:language-python",
            File::Folder => "material-symbols-light:folder-outline-rounded",
            File::FolderFill => "material-symbols-light:folder-rounded",
        }
    }

    fn body(self) -> IconBody {
        body_for(self.iconify_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn display_string_round_trips_every_variant() {
        for variant in File::iter() {
            let s = variant.to_string();
            assert_eq!(File::from_str(&s).unwrap(), variant, "round trip for {s}");
        }
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in File::iter() {
            assert!(
                variant.iconify_id().contains(':'),
                "{variant:?} id must be prefix:name"
            );
        }
    }
}
