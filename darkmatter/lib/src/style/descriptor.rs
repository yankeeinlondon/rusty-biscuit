//! Static catalog of every leaf path the schema understands.
//!
//! Used by the canonicalization walker (pass 1 of the parser) to detect
//! unknown keys and snake-case aliases. Add a row here whenever a field is
//! added to any per-bucket schema struct.

/// A single schema leaf: its canonical kebab-case path plus any accepted
/// snake-case alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaLeaf {
    /// Dotted canonical path under `style.` (e.g. `"page.left-margin"`).
    pub canonical: &'static str,
    /// Snake-case alias path, if any (e.g. `"page.left_margin"`).
    pub alias: Option<&'static str>,
    /// Sub-spec number that wires this key to rendering. `1` indicates
    /// "parsed only in v1; never wired" — currently no leaf is marked 1.
    /// All other values point to one of sub-specs #2..#7.
    pub sub_spec: u8,
}

/// The complete schema catalog. Every leaf reachable through any per-bucket
/// struct must appear here.
pub const SCHEMA: &[SchemaLeaf] = &[
    // ── page ────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "page.left-margin",    alias: Some("page.left_margin"),    sub_spec: 2 },
    SchemaLeaf { canonical: "page.right-margin",   alias: Some("page.right_margin"),   sub_spec: 2 },
    SchemaLeaf { canonical: "page.top-margin",     alias: Some("page.top_margin"),     sub_spec: 2 },
    SchemaLeaf { canonical: "page.bottom-margin",  alias: Some("page.bottom_margin"),  sub_spec: 2 },
    SchemaLeaf { canonical: "page.left-padding",   alias: Some("page.left_padding"),   sub_spec: 2 },
    SchemaLeaf { canonical: "page.right-padding",  alias: Some("page.right_padding"),  sub_spec: 2 },
    SchemaLeaf { canonical: "page.top-padding",    alias: Some("page.top_padding"),    sub_spec: 2 },
    SchemaLeaf { canonical: "page.bottom-padding", alias: Some("page.bottom_padding"), sub_spec: 2 },
    SchemaLeaf { canonical: "page.max-width",      alias: Some("page.max_width"),      sub_spec: 2 },
    SchemaLeaf { canonical: "page.alignment",      alias: None,                        sub_spec: 2 },
    SchemaLeaf { canonical: "page.color",          alias: None,                        sub_spec: 5 },
    SchemaLeaf { canonical: "page.bg-color",       alias: Some("page.bg_color"),       sub_spec: 5 },
    SchemaLeaf { canonical: "page.background",     alias: None,                        sub_spec: 2 },
    SchemaLeaf { canonical: "page.stylesheet",     alias: None,                        sub_spec: 7 },
    SchemaLeaf { canonical: "page.meta",           alias: None,                        sub_spec: 7 },
    SchemaLeaf { canonical: "page.code.theme",     alias: None,                        sub_spec: 7 },

    // ── table ───────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "table.width",     alias: None,                    sub_spec: 3 },
    SchemaLeaf { canonical: "table.max-width", alias: Some("table.max_width"), sub_spec: 3 },
    SchemaLeaf { canonical: "table.alignment", alias: None,                    sub_spec: 3 },
    SchemaLeaf { canonical: "table.color",     alias: None,                    sub_spec: 5 },
    SchemaLeaf { canonical: "table.bg-color",  alias: Some("table.bg_color"),  sub_spec: 5 },

    // ── block-quote ─────────────────────────────────────────────────────
    SchemaLeaf { canonical: "block-quote.width",     alias: Some("block_quote.width"),     sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.max-width", alias: Some("block_quote.max_width"), sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.alignment", alias: Some("block_quote.alignment"), sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.color",     alias: Some("block_quote.color"),     sub_spec: 5 },
    SchemaLeaf { canonical: "block-quote.bg-color",  alias: Some("block_quote.bg_color"),  sub_spec: 5 },

    // ── ul ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ul.width",       alias: None,                   sub_spec: 4 },
    SchemaLeaf { canonical: "ul.max-width",   alias: Some("ul.max_width"),   sub_spec: 4 },
    SchemaLeaf { canonical: "ul.alignment",   alias: None,                   sub_spec: 4 },
    SchemaLeaf { canonical: "ul.color",       alias: None,                   sub_spec: 5 },
    SchemaLeaf { canonical: "ul.bg-color",    alias: Some("ul.bg_color"),    sub_spec: 5 },
    SchemaLeaf { canonical: "ul.left-margin", alias: Some("ul.left_margin"), sub_spec: 4 },

    // ── ol ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ol.width",     alias: None,                  sub_spec: 4 },
    SchemaLeaf { canonical: "ol.max-width", alias: Some("ol.max_width"),  sub_spec: 4 },
    SchemaLeaf { canonical: "ol.alignment", alias: None,                  sub_spec: 4 },
    SchemaLeaf { canonical: "ol.color",     alias: None,                  sub_spec: 5 },
    SchemaLeaf { canonical: "ol.bg-color",  alias: Some("ol.bg_color"),   sub_spec: 5 },

    // ── li ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "li.width",     alias: None,                  sub_spec: 4 },
    SchemaLeaf { canonical: "li.max-width", alias: Some("li.max_width"),  sub_spec: 4 },
    SchemaLeaf { canonical: "li.alignment", alias: None,                  sub_spec: 4 },
    SchemaLeaf { canonical: "li.color",     alias: None,                  sub_spec: 5 },
    SchemaLeaf { canonical: "li.bg-color",  alias: Some("li.bg_color"),   sub_spec: 5 },

    // ── hyperlinks ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hyperlinks.width",                 alias: None,                                            sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.max-width",             alias: Some("hyperlinks.max_width"),                    sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.alignment",             alias: None,                                            sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.color",                 alias: None,                                            sub_spec: 5 },
    SchemaLeaf { canonical: "hyperlinks.bg-color",              alias: Some("hyperlinks.bg_color"),                     sub_spec: 5 },
    SchemaLeaf { canonical: "hyperlinks.local-style.width",     alias: Some("hyperlinks.local_style.width"),            sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.max-width", alias: Some("hyperlinks.local_style.max_width"),        sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.alignment", alias: Some("hyperlinks.local_style.alignment"),        sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.color",     alias: Some("hyperlinks.local_style.color"),            sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.bg-color",  alias: Some("hyperlinks.local_style.bg_color"),         sub_spec: 7 },

    // ── images ──────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "images.width",                 alias: None,                                       sub_spec: 3 },
    SchemaLeaf { canonical: "images.max-width",             alias: Some("images.max_width"),                   sub_spec: 3 },
    SchemaLeaf { canonical: "images.alignment",             alias: None,                                       sub_spec: 3 },
    SchemaLeaf { canonical: "images.color",                 alias: None,                                       sub_spec: 5 },
    SchemaLeaf { canonical: "images.bg-color",              alias: Some("images.bg_color"),                    sub_spec: 5 },
    SchemaLeaf { canonical: "images.local-style.width",     alias: Some("images.local_style.width"),           sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.max-width", alias: Some("images.local_style.max_width"),       sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.alignment", alias: Some("images.local_style.alignment"),       sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.color",     alias: Some("images.local_style.color"),           sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.bg-color",  alias: Some("images.local_style.bg_color"),        sub_spec: 7 },

    // ── hr ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hr.width",     alias: None,                 sub_spec: 6 },
    SchemaLeaf { canonical: "hr.max-width", alias: Some("hr.max_width"), sub_spec: 6 },
    SchemaLeaf { canonical: "hr.alignment", alias: None,                 sub_spec: 6 },
    SchemaLeaf { canonical: "hr.color",     alias: None,                 sub_spec: 6 },
    SchemaLeaf { canonical: "hr.bg-color",  alias: Some("hr.bg_color"),  sub_spec: 6 },
    SchemaLeaf { canonical: "hr.kind",      alias: None,                 sub_spec: 6 },
];

/// Return the canonical schema leaf for `raw_path` if it matches either a
/// canonical entry or an alias. Returns `None` for unknown paths.
pub fn canonicalize(raw_path: &str) -> Option<&'static SchemaLeaf> {
    SCHEMA
        .iter()
        .find(|leaf| leaf.canonical == raw_path || leaf.alias == Some(raw_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_lookup_finds_kebab_form() {
        let leaf = canonicalize("page.left-margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.sub_spec, 2);
    }

    #[test]
    fn canonical_lookup_finds_snake_alias() {
        let leaf = canonicalize("page.left_margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.alias, Some("page.left_margin"));
    }

    #[test]
    fn unknown_path_returns_none() {
        assert!(canonicalize("page.lft-margin").is_none());
        assert!(canonicalize("planet.left-margin").is_none());
    }

    #[test]
    fn schema_paths_are_unique() {
        // Detect duplicate canonical or alias entries — would cause double-
        // counted warnings.
        let mut seen = std::collections::BTreeSet::new();
        for leaf in SCHEMA {
            assert!(
                seen.insert(leaf.canonical),
                "duplicate canonical: {}",
                leaf.canonical
            );
            if let Some(alias) = leaf.alias {
                assert!(seen.insert(alias), "duplicate alias: {}", alias);
            }
        }
    }
}
