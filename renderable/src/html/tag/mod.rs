use url::Url;

use crate::browser::fragment::ComposableNode;
use crate::html::attribute::{
    ClassDefinition, DomId, HtmlDataAttribute,
    aria::{AriaAttribute, AriaRole},
    rel::RelAttribute,
};
use crate::stylesheet::CssStyle;

/// Placeholder type for HTML attribute values.
pub type Value = String;

pub mod link;
pub mod meta;

/// represents all the variants of "void elements" in HTML which
/// can not have closing tags. These tags can either be represents
/// solely with their tag name (e.g., `<br>`) or alternatively with
/// the more explicit `/>` variant (e.g., `<br/>`).
///
/// > **Note:** the more explicit format with the `/` character has
/// > no additional meaning. The implicit and explicit are semantically
/// > identical.
pub enum VoidTag {
    /// `<area>` — defines a hot-spot region within an image `<map>`,
    /// optionally linked via `href` to another resource.
    Area,
    /// `<base>` — specifies the document-wide base URL and/or default
    /// target for all relative URLs. Must appear inside `<head>`.
    Base,
    /// `<br>` — produces a line break within flow content.
    Br,
    /// `<col>` — declares column properties for a `<colgroup>` within
    /// a `<table>`.
    Col,
    /// `<embed>` — embeds an external (typically non-HTML) resource
    /// such as a plugin or interactive content.
    Embed,
    /// `<hr>` — represents a thematic break between paragraph-level
    /// content (rendered as a horizontal rule by default).
    Hr,
    /// `<img>` — embeds an image into the document.
    Img,
    /// `<input>` — a typed form control; behavior is selected via the
    /// `type` attribute (see [`HtmlType`]).
    Input,
    /// `<link>` — links the document to an external resource (most
    /// commonly a stylesheet, icon, or preload hint). Must appear
    /// inside `<head>`.
    Link,
    /// `<meta>` — document-level metadata not expressible via other
    /// `<head>` elements (charset, viewport, description, etc.).
    Meta,
    /// `<param>` — supplies a named parameter to an `<object>`
    /// element. Deprecated in HTML5.
    Param,
    /// `<source>` — declares an alternative media resource for a
    /// `<picture>`, `<audio>`, or `<video>` parent.
    Source,
    /// `<track>` — declares a timed text track (captions, subtitles,
    /// descriptions, chapters) for an `<audio>` or `<video>` parent.
    Track,
    /// `<wbr>` — marks a position where the browser may optionally
    /// break a line if needed (word-break opportunity).
    Wbr,
}

pub enum BlockTag {
    A,
    Abbr,
    Address,
    Article,
    Aside,
    Audio,
    B,
    Bdi,
    Bdo,
    Blockquote,
    Body,
    Button,
    Canvas,
    Caption,
    Cite,
    Code,
    Colgroup,
    Data,
    Datalist,
    Dd,
    Details,
    Dfn,
    Dialog,
    Div,
    Dl,
    Dt,
    Em,
    Fieldset,
    FigCaption,
    Figure,
    Footer,
    Form,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Head,
    Header,
    Hgroup,
    Html,
    I,
    Iframe,
    Ins,
    Kbd,
    Li,
    Main,
    Map,
    Mark,
    Menu,
    Meter,
    Nav,
    Noscript,
    Object,
    Ol,
    OptGroup,
    Option,
    Output,
    P,
    Picture,
    Pre,
    Progress,
    Q,
    Rp,
    Rt,
    Ruby,
    S,
    Samp,
    Script,
    Search,
    Section,
    Select,
    Slot,
    Small,
    Span,
    Strong,
    Style,
    Sub,
    Summary,
    Sup,
    Svg,
    Table,
    Tbody,
    Td,
    Template,
    Textarea,
    Tfoot,
    Th,
    Thead,
    Time,
    Tr,
    U,
    Ul,
    Var,
    Video,
}

/// Values for the HTML `type` attribute. Different elements use
/// `type` with different meanings — most notably the many input
/// control types for `<input type="...">`, but also `<button>`
/// behaviors, `<ol>` list numbering styles, `<script>` module
/// markers, and MIME types on `<link>`, `<style>`, `<source>`,
/// `<embed>`, `<object>`, and `<a>`.
///
/// The `Mime(String)` and `Other(String)` outlets cover values not
/// represented by a strong variant.
pub enum HtmlType {
    // --- <input> control types ---
    /// `<input type="button">` — generic clickable button with no
    /// default behavior. Also valid as `<button type="button">`.
    Button,
    /// `<input type="checkbox">` — multi-select toggle.
    Checkbox,
    /// `<input type="color">` — color picker.
    Color,
    /// `<input type="date">` — date picker (no time component).
    Date,
    /// `<input type="datetime-local">` — date and time picker without
    /// timezone information.
    DatetimeLocal,
    /// `<input type="email">` — email address field with built-in
    /// format validation.
    Email,
    /// `<input type="file">` — file selector for uploads.
    File,
    /// `<input type="hidden">` — value not displayed to the user but
    /// submitted with the form.
    Hidden,
    /// `<input type="image">` — graphical submit button.
    Image,
    /// `<input type="month">` — year-and-month picker.
    Month,
    /// `<input type="number">` — numeric input with optional min, max,
    /// and step.
    Number,
    /// `<input type="password">` — masked single-line text input.
    Password,
    /// `<input type="radio">` — single-select option within a named
    /// group.
    Radio,
    /// `<input type="range">` — slider for selecting an approximate
    /// numeric value.
    Range,
    /// `<input type="reset">` — button that resets the form to its
    /// initial values. Also valid as `<button type="reset">`.
    Reset,
    /// `<input type="search">` — single-line text input intended for
    /// search queries.
    Search,
    /// `<input type="submit">` — button that submits the form. Also
    /// valid as `<button type="submit">` and is the default `<button>`
    /// behavior.
    Submit,
    /// `<input type="tel">` — telephone number field (no format
    /// validation by default).
    Tel,
    /// `<input type="text">` — single-line free-form text. Default
    /// type for `<input>`.
    Text,
    /// `<input type="time">` — time picker (no date component).
    Time,
    /// `<input type="url">` — URL field with built-in format
    /// validation.
    Url,
    /// `<input type="week">` — year-and-week picker.
    Week,

    // --- <ol> list numbering styles ---
    /// `<ol type="1">` — decimal numbers (1, 2, 3, ...). Default for
    /// `<ol>`.
    Decimal,
    /// `<ol type="a">` — lowercase letters (a, b, c, ...).
    LowerAlpha,
    /// `<ol type="A">` — uppercase letters (A, B, C, ...).
    UpperAlpha,
    /// `<ol type="i">` — lowercase Roman numerals (i, ii, iii, ...).
    LowerRoman,
    /// `<ol type="I">` — uppercase Roman numerals (I, II, III, ...).
    UpperRoman,

    // --- <script> module markers ---
    /// `<script type="module">` — JavaScript with ES module
    /// semantics.
    Module,
    /// `<script type="importmap">` — JSON import map declaring
    /// module specifier resolution.
    ImportMap,

    // --- MIME and escape hatches ---
    /// Arbitrary MIME type (e.g. `text/css`, `image/png`,
    /// `application/json`). Used with `<link>`, `<style>`, `<source>`,
    /// `<embed>`, `<object>`, and `<a>` to declare the media type of
    /// the referenced resource.
    Mime(String),
    /// Escape hatch for any `type` value not represented by a strong
    /// variant.
    Other(String),
}

impl HtmlType {
    /// The `type` attribute value for this variant.
    ///
    /// Most variants map to a static token; `Mime` and `Other` carry an
    /// owned string, hence the [`Cow`](std::borrow::Cow) return.
    pub fn as_value(&self) -> std::borrow::Cow<'_, str> {
        use std::borrow::Cow;
        match self {
            HtmlType::Button => Cow::Borrowed("button"),
            HtmlType::Checkbox => Cow::Borrowed("checkbox"),
            HtmlType::Color => Cow::Borrowed("color"),
            HtmlType::Date => Cow::Borrowed("date"),
            HtmlType::DatetimeLocal => Cow::Borrowed("datetime-local"),
            HtmlType::Email => Cow::Borrowed("email"),
            HtmlType::File => Cow::Borrowed("file"),
            HtmlType::Hidden => Cow::Borrowed("hidden"),
            HtmlType::Image => Cow::Borrowed("image"),
            HtmlType::Month => Cow::Borrowed("month"),
            HtmlType::Number => Cow::Borrowed("number"),
            HtmlType::Password => Cow::Borrowed("password"),
            HtmlType::Radio => Cow::Borrowed("radio"),
            HtmlType::Range => Cow::Borrowed("range"),
            HtmlType::Reset => Cow::Borrowed("reset"),
            HtmlType::Search => Cow::Borrowed("search"),
            HtmlType::Submit => Cow::Borrowed("submit"),
            HtmlType::Tel => Cow::Borrowed("tel"),
            HtmlType::Text => Cow::Borrowed("text"),
            HtmlType::Time => Cow::Borrowed("time"),
            HtmlType::Url => Cow::Borrowed("url"),
            HtmlType::Week => Cow::Borrowed("week"),
            HtmlType::Decimal => Cow::Borrowed("1"),
            HtmlType::LowerAlpha => Cow::Borrowed("a"),
            HtmlType::UpperAlpha => Cow::Borrowed("A"),
            HtmlType::LowerRoman => Cow::Borrowed("i"),
            HtmlType::UpperRoman => Cow::Borrowed("I"),
            HtmlType::Module => Cow::Borrowed("module"),
            HtmlType::ImportMap => Cow::Borrowed("importmap"),
            HtmlType::Mime(value) => Cow::Borrowed(value.as_str()),
            HtmlType::Other(value) => Cow::Borrowed(value.as_str()),
        }
    }
}

/// Enumerates the most common HTML Attributes, providing
/// strong types. Allows an `Other((String, Value))` outlet
/// if a less commonly used attribute is needed.
pub enum HtmlAttribute {
    /// `id` — unique document identifier (no spaces allowed).
    Id(DomId),
    /// `class` — one or more space-separated class names.
    Class(ClassDefinition),
    /// `style` — inline CSS declarations.
    Style(CssStyle),
    /// `title` — advisory tooltip text.
    Title(String),
    /// `data-*` — custom user-defined data attribute keyed by the
    /// `data-{name}` suffix.
    Data(HtmlDataAttribute, String),
    /// `hidden` — boolean attribute that removes the element from
    /// rendering and the accessibility tree.
    Hidden(bool),
    /// `role` — ARIA landmark or widget role assigned to the element
    /// (e.g. `role="button"`, `role="navigation"`).
    Role(AriaRole),
    /// `aria-*` — accessibility attribute fitting the
    /// `aria-{name}="{value}"` pattern (e.g. `aria-label`,
    /// `aria-describedby`, `aria-expanded`). Each [`AriaAttribute`]
    /// variant carries the value type required by the WAI-ARIA spec.
    Aria(AriaAttribute),

    /// `href` — hyperlink reference. Used on `<a>`, `<area>`,
    /// `<link>`, and `<base>`.
    Href(Url),
    /// `src` — resource URL. Used on `<img>`, `<script>`, `<iframe>`,
    /// `<audio>`, `<video>`, `<source>`, `<embed>`, `<track>`, and
    /// `<input type="image">`.
    Src(Url),
    /// `alt` — text alternative for non-text content. Required on
    /// `<img>` for accessibility.
    Alt(String),
    /// `type` — element type or MIME marker; see [`HtmlType`] for the
    /// many element-specific meanings.
    Type(HtmlType),
    /// `name` — form control or metadata name; the key under which a
    /// control's value is submitted. Also used on `<meta>`,
    /// `<iframe>`, `<map>`, and `<param>`.
    Name(String),
    /// `value` — current value of a form control, `<option>`, or
    /// `<meter>`/`<progress>`.
    Value(Value),
    /// `placeholder` — hint text shown inside an empty form control.
    Placeholder(String),
    /// `disabled` — boolean attribute marking a form control as
    /// non-interactive and excluded from submission.
    Disabled(bool),
    /// `checked` — boolean attribute for `<input type="checkbox">`
    /// and `<input type="radio">` indicating initial selected state.
    Checked(bool),
    /// `selected` — boolean attribute on `<option>` indicating the
    /// initially selected entry within a `<select>`.
    Selected(bool),
    /// `target` — browsing context for hyperlink navigation
    /// (`_self`, `_blank`, `_parent`, `_top`, or a named frame).
    Target(String),
    /// `rel` — relationship between the document and the linked
    /// resource. Used on `<a>`, `<area>`, `<link>`, and `<form>`.
    Rel(RelAttribute),

    /// Escape hatch for any attribute not represented by a strong
    /// variant.
    Other(String, Value),
}

/// A single node within an HTML element tree, restricted to the three
/// "pure HTML" shapes (no nested components).
///
/// Each variant represents one of the three node shapes that may appear
/// as a direct child of an element:
///
/// - a non-void element with its own attributes and children
///   ([`BlockTag`](HtmlNode::BlockTag)),
/// - a void element with attributes but no children
///   ([`VoidTag`](HtmlNode::VoidTag)), or
/// - a run of text content ([`TextFragment`](HtmlNode::TextFragment)).
///
/// [`InnerHtml`] holds [`ComposableNode`] rather than `HtmlNode` so child
/// content can include nested `BrowserRenderable` components in addition
/// to these pure shapes.
#[allow(private_interfaces)]
pub enum HtmlNode {
    /// A non-void element — a tag with an opening and closing pair
    /// that may itself contain attributes and nested child nodes
    /// (e.g. `<div class="foo">…</div>`).
    BlockTag(HtmlBlockTag),
    /// A void element — a self-contained tag that carries attributes
    /// but cannot have children (e.g. `<img src="…" alt="…">`).
    VoidTag(HtmlVoidTag),
    /// A run of literal text content placed between sibling elements.
    ///
    /// The renderer is responsible for HTML-escaping the contained
    /// string when emitting output.
    TextFragment(String),
}

/// Ordered child content of an HTML element.
///
/// Wraps the sequence of [`ComposableNode`] values that appear between an
/// element's opening and closing tags. Children are rendered in the order
/// stored, and may include nested `BrowserRenderable` components via
/// [`ComposableNode::Component`].
pub struct InnerHtml {
    /// The element's direct children, in document order.
    pub(crate) children: Vec<ComposableNode>,
}

impl InnerHtml {
    pub(crate) fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

/// A non-void HTML element — a tag with an opening and closing pair
/// that may carry attributes and nested children.
///
/// Examples: `<div>`, `<section>`, `<a href="…">…</a>`,
/// `<button type="submit">…</button>`.
///
/// Crate-private: constructed by the `browser::fragment` builders. External
/// users compose fragments rather than hand-rolling these structs.
#[allow(dead_code)]
pub(crate) struct HtmlBlockTag {
    /// Which non-void tag this element is (e.g. [`BlockTag::Div`]).
    pub(crate) tag: BlockTag,
    /// The component's wrapper class — used to namespace any
    /// `ComponentStylesheet` rules under a descendant selector.
    pub(crate) base_class: String,
    /// The element's child nodes, in document order.
    pub(crate) content: InnerHtml,
    /// Attributes attached to this element's opening tag.
    pub(crate) attributes: Vec<HtmlAttribute>,
}

impl HtmlBlockTag {
    pub(crate) fn new(tag: BlockTag, base_class: String) -> Self {
        Self {
            tag,
            base_class,
            content: InnerHtml::new(),
            attributes: Vec::new(),
        }
    }
}

/// A void HTML element — a self-contained tag that may carry
/// attributes but has no children and no closing tag.
///
/// Examples: `<br>`, `<hr>`, `<img src="…" alt="…">`,
/// `<input type="text" name="…">`.
///
/// > **Note:** the trailing-slash form (`<br/>`) is semantically
/// > identical to the bare form (`<br>`); see [`VoidTag`].
#[allow(dead_code)]
pub(crate) struct HtmlVoidTag {
    /// Which void tag this element is (e.g. [`VoidTag::Img`]).
    pub(crate) tag: VoidTag,
    /// Attributes attached to this element's tag.
    pub(crate) attributes: Vec<HtmlAttribute>,
}

impl HtmlVoidTag {
    pub(crate) fn new(tag: VoidTag) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
        }
    }
}

impl VoidTag {
    /// The lowercase HTML tag name for this void element.
    pub fn name(&self) -> &'static str {
        match self {
            VoidTag::Area => "area",
            VoidTag::Base => "base",
            VoidTag::Br => "br",
            VoidTag::Col => "col",
            VoidTag::Embed => "embed",
            VoidTag::Hr => "hr",
            VoidTag::Img => "img",
            VoidTag::Input => "input",
            VoidTag::Link => "link",
            VoidTag::Meta => "meta",
            VoidTag::Param => "param",
            VoidTag::Source => "source",
            VoidTag::Track => "track",
            VoidTag::Wbr => "wbr",
        }
    }
}

impl BlockTag {
    /// The lowercase HTML tag name for this non-void element.
    pub fn name(&self) -> &'static str {
        match self {
            BlockTag::A => "a",
            BlockTag::Abbr => "abbr",
            BlockTag::Address => "address",
            BlockTag::Article => "article",
            BlockTag::Aside => "aside",
            BlockTag::Audio => "audio",
            BlockTag::B => "b",
            BlockTag::Bdi => "bdi",
            BlockTag::Bdo => "bdo",
            BlockTag::Blockquote => "blockquote",
            BlockTag::Body => "body",
            BlockTag::Button => "button",
            BlockTag::Canvas => "canvas",
            BlockTag::Caption => "caption",
            BlockTag::Cite => "cite",
            BlockTag::Code => "code",
            BlockTag::Colgroup => "colgroup",
            BlockTag::Data => "data",
            BlockTag::Datalist => "datalist",
            BlockTag::Dd => "dd",
            BlockTag::Details => "details",
            BlockTag::Dfn => "dfn",
            BlockTag::Dialog => "dialog",
            BlockTag::Div => "div",
            BlockTag::Dl => "dl",
            BlockTag::Dt => "dt",
            BlockTag::Em => "em",
            BlockTag::Fieldset => "fieldset",
            BlockTag::FigCaption => "figcaption",
            BlockTag::Figure => "figure",
            BlockTag::Footer => "footer",
            BlockTag::Form => "form",
            BlockTag::H1 => "h1",
            BlockTag::H2 => "h2",
            BlockTag::H3 => "h3",
            BlockTag::H4 => "h4",
            BlockTag::H5 => "h5",
            BlockTag::H6 => "h6",
            BlockTag::Head => "head",
            BlockTag::Header => "header",
            BlockTag::Hgroup => "hgroup",
            BlockTag::Html => "html",
            BlockTag::I => "i",
            BlockTag::Iframe => "iframe",
            BlockTag::Ins => "ins",
            BlockTag::Kbd => "kbd",
            BlockTag::Li => "li",
            BlockTag::Main => "main",
            BlockTag::Map => "map",
            BlockTag::Mark => "mark",
            BlockTag::Menu => "menu",
            BlockTag::Meter => "meter",
            BlockTag::Nav => "nav",
            BlockTag::Noscript => "noscript",
            BlockTag::Object => "object",
            BlockTag::Ol => "ol",
            BlockTag::OptGroup => "optgroup",
            BlockTag::Option => "option",
            BlockTag::Output => "output",
            BlockTag::P => "p",
            BlockTag::Picture => "picture",
            BlockTag::Pre => "pre",
            BlockTag::Progress => "progress",
            BlockTag::Q => "q",
            BlockTag::Rp => "rp",
            BlockTag::Rt => "rt",
            BlockTag::Ruby => "ruby",
            BlockTag::S => "s",
            BlockTag::Samp => "samp",
            BlockTag::Script => "script",
            BlockTag::Search => "search",
            BlockTag::Section => "section",
            BlockTag::Select => "select",
            BlockTag::Slot => "slot",
            BlockTag::Small => "small",
            BlockTag::Span => "span",
            BlockTag::Strong => "strong",
            BlockTag::Style => "style",
            BlockTag::Sub => "sub",
            BlockTag::Summary => "summary",
            BlockTag::Sup => "sup",
            BlockTag::Svg => "svg",
            BlockTag::Table => "table",
            BlockTag::Tbody => "tbody",
            BlockTag::Td => "td",
            BlockTag::Template => "template",
            BlockTag::Textarea => "textarea",
            BlockTag::Tfoot => "tfoot",
            BlockTag::Th => "th",
            BlockTag::Thead => "thead",
            BlockTag::Time => "time",
            BlockTag::Tr => "tr",
            BlockTag::U => "u",
            BlockTag::Ul => "ul",
            BlockTag::Var => "var",
            BlockTag::Video => "video",
        }
    }
}
