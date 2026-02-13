use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Keywords
    Keyword(String),  // e.g., "auto", "none", "inherit"

    // Numeric types (Preserve units for calculation/serialization)
    Length(f32, Unit),
    Percentage(f32),

    // Colors (Structured data allows manipulation)
    Color(Rgba),

    // Complex/Compound values
    // e.g., "1px solid black" -> Vec<Value> or a specialized Struct
    Border(Box<Border>),

    // Fallback for raw/unparsed CSS
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
    Em,
    Rem,
    Vw,
    Vh,
    Percent, // Often handled by Value::Percentage, but useful here too
    None,    // Unitless (e.g., line-height: 1.5)
}

// Example of a structured Color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Property {
    // == Layout & Display ==
    Display,
    Position,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,

    // == Flexbox ==
    FlexDirection,
    FlexWrap,
    JustifyContent,
    AlignItems,
    AlignContent,
    Gap,

    // == Grid ==
    GridTemplateColumns,
    GridTemplateRows,
    GridArea,

    // == Box Model (Dimensions & Spacing) ==
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,

    // Margins
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,

    // Padding
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,

    // == Typography ==
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    LineHeight,
    TextAlign,
    TextDecoration,
    Color,

    // == Background & Borders ==
    BackgroundColor,
    BackgroundImage,
    BackgroundSize,
    BackgroundPosition,

    Border,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderRadius,
    BorderCollapse,

    // == Effects ==
    Opacity,
    BoxShadow,
    Visibility,
    Overflow,
    OverflowX,
    OverflowY,
    Cursor,
    Transition,
    Transform,

    // == CSS Variables & Fallbacks ==
    /// For custom properties like `--my-var`
    Custom(String),

    /// For unknown or non-standard properties (keeps the system from breaking)
    Unknown(String),
}


#[derive(Debug, Clone)]
pub struct Stylesheet {
    rules: IndexMap<Property, Value>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self { rules: IndexMap::new() }
    }

    // Type-safe setter
    pub fn set(&mut self, prop: Property, val: Value) -> &mut Self {
        self.rules.insert(prop, val);
        self
    }

    // Helper for easy construction
    pub fn margin(&mut self, val: Value) -> &mut Self {
        self.set(Property::Margin, val)
    }

    // To CSS String output
    pub fn to_css(&self) -> String {
        self.rules
            .iter()
            .map(|(k, v)| format!("{}: {};", k.to_css_string(), v.to_css_string()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Value {
    fn to_css_string(&self) -> String {
        match self {
            Value::Keyword(s) => s.clone(),
            Value::Length(n, Unit::Px) => format!("{}px", n),
            Value::Length(n, Unit::Em) => format!("{}em", n),
            Value::Color(c) => format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, c.a),
            Value::Raw(s) => s.clone(),
            _ => "unknown".to_string(),
        }
    }
}
