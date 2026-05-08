use serde::{Deserialize, Serialize};

/// Represents common font ligatures that may be available in terminal fonts.
///
/// Font ligatures combine multiple characters into a single glyph for improved
/// readability. These are commonly found in programming fonts like Fira Code,
/// JetBrains Mono, Cascadia Code, and others.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::FontLigature;
///
/// let ligature = FontLigature::ArrowRight;
/// // Represents the "->" ligature commonly used for returns and member access
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontLigature {
    // Arrow ligatures
    /// `->` - Right arrow (member access, return type)
    ArrowRight,
    /// `=>` - Double arrow right (fat arrow, lambda, match arms)
    DoubleArrowRight,
    /// `<-` - Left arrow (assignment in Haskell)
    ArrowLeft,
    /// `<=` - Less than or equal to
    LessThanOrEqual,
    /// `>=` - Greater than or equal to
    GreaterThanOrEqual,
    /// `->>` - Long arrow right (pipeline operator)
    LongArrowRight,
    /// `=>>` - Long double arrow right
    LongDoubleArrowRight,
    /// `<<-` - Long arrow left
    LongArrowLeft,

    // Comparison and equality
    /// `==` - Equality comparison
    Equality,
    /// `===` - Strict equality (JavaScript)
    StrictEquality,
    /// `!=` - Not equal
    NotEqual,
    /// `!==` - Strict not equal (JavaScript)
    StrictNotEqual,
    /// `=~` - Regex match
    RegexMatch,
    /// `!~` - Regex not match
    RegexNotMatch,

    // Logical operators
    /// `&&` - Logical AND
    LogicalAnd,
    /// `||` - Logical OR
    LogicalOr,
    /// `!!` - Double negation / force unwrap
    DoubleBang,

    // Mathematical operators
    /// `*` - Multiplication (sometimes ligatured with context)
    Multiply,
    /// `**` - Exponentiation
    Exponentiation,
    /// `***` - Triple star
    TripleStar,
    /// `//` - Integer division or comment
    IntegerDivision,
    /// `///` - Doc comment
    DocComment,
    /// `++` - Increment or concatenation
    Increment,
    /// `+++` - Triple plus
    TriplePlus,
    /// `--` - Decrement
    Decrement,
    /// `---` - Triple minus
    TripleMinus,
    /// `<>` - Diamond operator / empty box
    Diamond,
    /// `><` - Reverse diamond
    ReverseDiamond,
    /// `><>` - Fish operator (Haskell)
    Fish,

    // Assignment operators
    /// `:=` - Walrus operator (assignment expression)
    Walrus,
    /// `=:` - Reverse walrus
    ReverseWalrus,
    /// `+=` - Plus equals
    PlusEquals,
    /// `-=` - Minus equals
    MinusEquals,
    /// `*=` - Multiply equals
    MultiplyEquals,
    /// `/=` - Divide equals
    DivideEquals,
    /// `%=` - Modulo equals
    ModuloEquals,
    /// `&=` - Bitwise AND equals
    BitwiseAndEquals,
    /// `|=` - Bitwise OR equals
    BitwiseOrEquals,
    /// `^=` - Bitwise XOR equals
    BitwiseXorEquals,
    /// `<<=` - Left shift equals
    LeftShiftEquals,
    /// `>>=` - Right shift equals
    RightShiftEquals,

    // Bitwise operators
    /// `<<` - Left shift
    LeftShift,
    /// `>>` - Right shift
    RightShift,
    /// `<<<` - Triple left shift
    TripleLeftShift,
    /// `>>>` - Triple right shift (unsigned)
    TripleRightShift,
    /// `&` - Bitwise AND
    BitwiseAnd,
    /// `|` - Bitwise OR
    BitwiseOr,
    /// `^` - Bitwise XOR
    BitwiseXor,
    /// `~` - Bitwise NOT
    BitwiseNot,

    // Punctuation and brackets
    /// `::` - Scope resolution operator
    ScopeResolution,
    /// `...` - Ellipsis / spread operator
    Ellipsis,
    /// `..` - Range operator
    Range,
    /// `..=` - Inclusive range (Rust)
    InclusiveRange,
    /// `..=` - Half-open range (Swift)
    HalfOpenRange,
    /// `||>` - Pipe operator (Elixir)
    PipeRight,
    /// `<||` - Reverse pipe operator
    PipeLeft,
    /// `|>` - Feed operator (F#, Elixir)
    FeedRight,
    /// `<|` - Feed left operator
    FeedLeft,
    /// `[|` - Opening bracket pipe
    BracketPipe,
    /// `|]` - Closing pipe bracket
    PipeBracket,
    /// `(|` - Opening paren pipe
    ParenPipe,
    /// `|)` - Closing pipe paren
    PipeParen,
    /// `<>` - Template placeholder (PHP, etc.)
    TemplatePlaceholder,
    /// `<~` - Bind operator (Haskell)
    Bind,
    /// `~>` - Reverse bind
    ReverseBind,

    // Hash and other symbols
    /// `#` - Hash / pound / sharp
    Hash,
    /// `##` - Double hash
    DoubleHash,
    /// `###` - Triple hash
    TripleHash,
    /// `#(` - Hash paren
    HashParen,
    /// `#{` - Hash brace
    HashBrace,
    /// `#?` - Hash question (debug macro)
    HashQuestion,
    /// `#!` - Shebang
    Shebang,

    // At sign variants
    /// `@` - At sign
    At,
    /// `@@` - Double at
    DoubleAt,

    // Dollar sign variants
    /// `$` - Dollar sign
    Dollar,
    /// `$$` - Double dollar
    DoubleDollar,

    // Percent variants
    /// `%` - Percent
    Percent,
    /// `%%` - Double percent
    DoublePercent,

    // Question mark variants
    /// `?` - Question mark
    Question,
    /// `??` - Null coalescing / optional chaining
    DoubleQuestion,
    /// `?.` - Optional chaining
    QuestionDot,
    /// `?:` - Elvis operator
    Elvis,
    /// `?!` - Question bang
    QuestionBang,
    /// `?=` - Question equals
    QuestionEquals,

    // Colon variants
    /// `:` - Colon
    Colon,
    /// `:::` - Triple colon
    TripleColon,

    // Semicolon variants
    /// `;` - Semicolon
    Semicolon,
    /// `;;` - Double semicolon
    DoubleSemicolon,

    // Other common ligatures
    /// `///=` - Doc comment equals (Rust)
    DocCommentEquals,
    /// `//=` - Comment equals
    CommentEquals,
    /// `/**` - JSDoc opening
    JsDocOpen,
    /// `*/` - JSDoc closing
    JsDocClose,
    /// `<!--` - HTML comment open
    HtmlCommentOpen,
    /// `-->` - HTML comment close
    HtmlCommentClose,
    /// `</` - HTML closing tag
    HtmlClosingTag,

    /// Catch-all for other ligatures not in this enumeration
    Other(String),
}

/// Represents the window size in pixels.
///
/// This is the pixel dimensions of the terminal window, which can be used
/// to calculate cell size (font size) when combined with grid dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSizePixels {
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
}

/// Represents the cell size in pixels.
///
/// The cell size is the width and height of a single character cell,
/// which is directly related to the font size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSize {
    /// Cell width in pixels
    pub width: u32,
    /// Cell height in pixels
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_size_pixels_struct() {
        let size = WindowSizePixels {
            width: 1920,
            height: 1080,
        };
        assert_eq!(size.width, 1920);
        assert_eq!(size.height, 1080);
    }

    #[test]
    fn test_cell_size_struct() {
        let size = CellSize {
            width: 10,
            height: 20,
        };
        assert_eq!(size.width, 10);
        assert_eq!(size.height, 20);
    }
}
