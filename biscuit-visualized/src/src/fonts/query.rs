//! What a piece of text asks for: a family stack, weight, style, and stretch.

use resvg::usvg::{self, fontdb};

/// The five CSS generic families that usvg (through svgtypes) recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GenericFamily {
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

impl GenericFamily {
    pub(crate) const ALL: [GenericFamily; 5] = [
        GenericFamily::Serif,
        GenericFamily::SansSerif,
        GenericFamily::Monospace,
        GenericFamily::Cursive,
        GenericFamily::Fantasy,
    ];

    fn from_keyword(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|generic| generic.keyword().eq_ignore_ascii_case(name))
    }

    pub(crate) fn keyword(self) -> &'static str {
        match self {
            GenericFamily::Serif => "serif",
            GenericFamily::SansSerif => "sans-serif",
            GenericFamily::Monospace => "monospace",
            GenericFamily::Cursive => "cursive",
            GenericFamily::Fantasy => "fantasy",
        }
    }
}

/// One entry of a font stack.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum FamilyName {
    Named(String),
    Generic(GenericFamily),
}

impl FamilyName {
    /// A family from its name. A name equal to a generic keyword, ignoring
    /// ASCII case, is that generic, quoted or not.
    ///
    /// CSS would keep a quoted `"serif"` a name, but usvg hands the resolver
    /// a bare string for any name it does not recognize (svgtypes matches the
    /// keywords case-sensitively), so the quoting is lost on the rasterizer
    /// side. Treating every keyword spelling as the generic keeps the
    /// measurer's parse and the rasterizer's parse identical.
    pub(crate) fn from_name(name: &str) -> Self {
        match GenericFamily::from_keyword(name) {
            Some(generic) => FamilyName::Generic(generic),
            None => FamilyName::Named(name.to_string()),
        }
    }
}

/// A font request, resolved by [`FontChain::resolve`](super::FontChain::resolve).
///
/// Built from a `usvg::Font` by the rasterizer and from CSS values by the
/// text measurer, so both sides ask the chain the same question.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FontQuery {
    pub(crate) families: Vec<FamilyName>,
    pub(crate) weight: u16,
    pub(crate) style: fontdb::Style,
    pub(crate) stretch: fontdb::Stretch,
}

impl FontQuery {
    /// A regular (400, upright, normal-width) query for `families`.
    pub(crate) fn new(families: Vec<FamilyName>) -> Self {
        FontQuery {
            families,
            weight: 400,
            style: fontdb::Style::Normal,
            stretch: fontdb::Stretch::Normal,
        }
    }

    /// A query from a CSS `font-family` value (see [`parse_font_family_list`]),
    /// a numeric weight, and an italic flag.
    pub(crate) fn from_css(font_family: &str, weight: u16, italic: bool) -> Self {
        let mut query = FontQuery::new(parse_font_family_list(font_family)).with_weight(weight);
        if italic {
            query.style = fontdb::Style::Italic;
        }
        query
    }

    pub(crate) fn with_weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    pub(crate) fn with_style(mut self, style: fontdb::Style) -> Self {
        self.style = style;
        self
    }

    pub(crate) fn with_stretch(mut self, stretch: fontdb::Stretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// The query usvg's `<text>` layout makes for `font`.
    pub(crate) fn from_usvg(font: &usvg::Font) -> Self {
        let families = font
            .families()
            .iter()
            .map(|family| match family {
                usvg::FontFamily::Named(name) => FamilyName::from_name(name),
                usvg::FontFamily::Serif => FamilyName::Generic(GenericFamily::Serif),
                usvg::FontFamily::SansSerif => FamilyName::Generic(GenericFamily::SansSerif),
                usvg::FontFamily::Monospace => FamilyName::Generic(GenericFamily::Monospace),
                usvg::FontFamily::Cursive => FamilyName::Generic(GenericFamily::Cursive),
                usvg::FontFamily::Fantasy => FamilyName::Generic(GenericFamily::Fantasy),
            })
            .collect();
        FontQuery {
            families,
            weight: font.weight(),
            style: font.style().into(),
            stretch: font.stretch().into(),
        }
    }
}

/// Parses a CSS `font-family` list such as
/// `"trebuchet ms", verdana, arial, sans-serif`.
///
/// Entries are separated by commas outside quotes. A quoted entry (`"` or
/// `'`) is taken up to its closing quote and trimmed; an unquoted entry has its
/// runs of whitespace collapsed to one space, as svgtypes joins identifiers.
/// Generic keywords are recognized ignoring case (see
/// [`FamilyName::from_name`]). Empty entries are dropped, so malformed input
/// never fails, it only yields fewer families.
pub(crate) fn parse_font_family_list(list: &str) -> Vec<FamilyName> {
    let mut families = Vec::new();
    let mut chars = list.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let Some(&first) = chars.peek() else { break };
        let name = if first == '"' || first == '\'' {
            chars.next();
            let quoted: String = chars.by_ref().take_while(|&c| c != first).collect();
            // Anything between the closing quote and the next comma is junk.
            for c in chars.by_ref() {
                if c == ',' {
                    break;
                }
            }
            quoted.trim().to_string()
        } else {
            let raw: String = chars.by_ref().take_while(|&c| c != ',').collect();
            raw.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        if !name.is_empty() {
            families.push(FamilyName::from_name(&name));
        }
    }
    families
}
