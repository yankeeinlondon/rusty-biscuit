//! The code points and weights a static SVG draws, per family, which decide
//! what the embedded font must carry.
//!
//! A label's font comes through the CSS cascade: Merman sets most weights in
//! class rules of the SVG's `<style>`, not on the `<text>` itself. So the
//! SVG is read with `roxmltree` and cascaded with `simplecss`, the parser
//! and selector matcher usvg itself uses, with usvg's precedence: a
//! presentation attribute, then the stylesheet's rules in specificity order,
//! then the `style` attribute, an `!important` declaration keeping its place
//! against a later one.

use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Node;
use simplecss::{Declaration, DeclarationTokenizer, StyleSheet};

/// The code points one family draws, by weight.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WeightGlyphs {
    pub(crate) regular: BTreeSet<char>,
    /// Text whose computed `font-weight` is `bold`, `bolder`, or 600 and up.
    pub(crate) bold: BTreeSet<char>,
}

/// What one static SVG draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UsedGlyphs {
    /// Keyed by the first family of the text's computed `font-family`,
    /// unquoted and unescaped: the face the HTML static post-process leads
    /// every label with. Text with no `font-family` at all is not recorded.
    pub(crate) families: BTreeMap<String, WeightGlyphs>,
}

impl UsedGlyphs {
    /// The code points drawn in `family` (matched ASCII case-insensitively,
    /// as CSS matches family names).
    pub(crate) fn for_family(&self, family: &str) -> impl Iterator<Item = &WeightGlyphs> {
        self.families
            .iter()
            .filter(move |(name, _)| name.eq_ignore_ascii_case(family))
            .map(|(_, glyphs)| glyphs)
    }
}

/// The SVG is not well-formed XML.
#[derive(Debug, thiserror::Error)]
#[error("the static SVG does not parse: {0}")]
pub(crate) struct GlyphsError(#[from] roxmltree::Error);

/// Collects the Unicode scalar values in the character data of every
/// `<text>` (its `<tspan>`s included) of `svg`, per family and weight.
///
/// Control characters (newlines and tabs included) are dropped, and U+0020 is
/// added to every family and weight that draws any text, because SVG turns
/// white space into spaces. Code points are not checked against any face;
/// the subsetter skips those the face does not map.
pub(crate) fn collect_used_glyphs(svg: &str) -> Result<UsedGlyphs, GlyphsError> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    let document = roxmltree::Document::parse_with_options(svg, options)?;
    let mut sheet = StyleSheet::new();
    for style in document.descendants().filter(|node| node.has_tag_name("style")) {
        if matches!(style.attribute("type"), None | Some("text/css"))
            && let Some(css) = style.text()
        {
            sheet.parse_more(css);
        }
    }
    let mut used = UsedGlyphs::default();
    collect(document.root(), &Computed::default(), false, &sheet, &mut used);
    Ok(used)
}

/// The inherited font properties of an element.
#[derive(Debug, Clone, Default)]
struct Computed {
    family: Option<String>,
    bold: bool,
}

fn collect(node: Node<'_, '_>, parent: &Computed, in_text: bool, sheet: &StyleSheet<'_>, used: &mut UsedGlyphs) {
    for child in node.children() {
        if child.is_text() {
            if in_text {
                record(child.text().unwrap_or_default(), parent, used);
            }
        } else if child.is_element() {
            let computed = cascade(child, parent, sheet);
            let in_text = in_text || child.tag_name().name() == "text";
            collect(child, &computed, in_text, sheet, used);
        }
    }
}

fn record(text: &str, computed: &Computed, used: &mut UsedGlyphs) {
    let Some(family) = &computed.family else { return };
    let drawn: Vec<char> = text.chars().filter(|c| !c.is_control()).collect();
    // Markup indentation between elements draws nothing.
    if drawn.iter().all(|&c| c == ' ') {
        return;
    }
    let glyphs = used.families.entry(family.clone()).or_default();
    let set = if computed.bold { &mut glyphs.bold } else { &mut glyphs.regular };
    set.insert(' ');
    set.extend(drawn);
}

/// One property's cascaded value: `(value, important)`.
type Cascaded = Option<(String, bool)>;

/// A later declaration replaces an earlier one unless the earlier one is
/// `!important` (usvg's rule, which an important later one also obeys).
fn declare(slot: &mut Cascaded, value: &str, important: bool) {
    if !slot.as_ref().is_some_and(|(_, earlier_important)| *earlier_important) {
        *slot = Some((value.to_string(), important));
    }
}

fn cascade(element: Node<'_, '_>, parent: &Computed, sheet: &StyleSheet<'_>) -> Computed {
    let mut family: Cascaded = element.attribute("font-family").map(|value| (value.to_string(), false));
    let mut weight: Cascaded = element.attribute("font-weight").map(|value| (value.to_string(), false));
    let mut apply = |declaration: &Declaration<'_>| {
        let (value, important) = (declaration.value, declaration.important);
        match declaration.name {
            "font-family" => declare(&mut family, value, important),
            "font-weight" => declare(&mut weight, value, important),
            "font" => {
                if let Ok(shorthand) = svgtypes::FontShorthand::from_str(value) {
                    declare(&mut family, shorthand.font_family, important);
                    declare(&mut weight, shorthand.font_weight.unwrap_or("normal"), important);
                }
            }
            _ => {}
        }
    };
    for rule in &sheet.rules {
        if rule.selector.matches(&XmlNode(element)) {
            rule.declarations.iter().for_each(&mut apply);
        }
    }
    if let Some(style) = element.attribute("style") {
        DeclarationTokenizer::from(style).for_each(|declaration| apply(&declaration));
    }
    Computed {
        family: match family {
            Some((value, _)) if value.trim() != "inherit" => first_family(&value).or_else(|| parent.family.clone()),
            _ => parent.family.clone(),
        },
        bold: weight.map_or(parent.bold, |(value, _)| is_bold(value.trim(), parent.bold)),
    }
}

/// Whether `value` (a `font-weight`) is bold, given the parent's weight. A
/// value CSS does not define (a `var()` included) is ignored, as a browser
/// ignores an invalid declaration, so the parent's weight holds.
fn is_bold(value: &str, parent_bold: bool) -> bool {
    match value {
        "bold" | "bolder" => true,
        "normal" | "lighter" => false,
        number => number.parse::<f32>().map_or(parent_bold, |weight| weight >= 600.0),
    }
}

/// The first family of a CSS `font-family` list, unquoted and with CSS
/// escapes resolved; `None` for an empty list.
pub(crate) fn first_family(list: &str) -> Option<String> {
    let mut chars = list.trim_start().chars().peekable();
    let mut name = String::new();
    let quote = chars.next_if(|&c| c == '"' || c == '\'');
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let mut hex = String::new();
                while hex.len() < 6
                    && let Some(digit) = chars.next_if(char::is_ascii_hexdigit)
                {
                    hex.push(digit);
                }
                if hex.is_empty() {
                    name.extend(chars.next());
                } else {
                    chars.next_if(|c| c.is_whitespace());
                    name.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('\u{FFFD}'));
                }
            }
            c if Some(c) == quote => break,
            ',' if quote.is_none() => break,
            c => name.push(c),
        }
    }
    let name = if quote.is_some() {
        name
    } else {
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    (!name.is_empty()).then_some(name)
}

/// `simplecss` selector matching over a `roxmltree` element, as usvg does.
struct XmlNode<'a, 'input>(Node<'a, 'input>);

impl simplecss::Element for XmlNode<'_, '_> {
    fn parent_element(&self) -> Option<Self> {
        self.0.parent_element().map(XmlNode)
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.0.prev_sibling_element().map(XmlNode)
    }

    fn has_local_name(&self, local_name: &str) -> bool {
        self.0.tag_name().name() == local_name
    }

    fn attribute_matches(&self, local_name: &str, operator: simplecss::AttributeOperator<'_>) -> bool {
        self.0.attribute(local_name).is_some_and(|value| operator.matches(value))
    }

    fn pseudo_class_matches(&self, class: simplecss::PseudoClass<'_>) -> bool {
        matches!(class, simplecss::PseudoClass::FirstChild) && self.prev_sibling_element().is_none()
    }
}
