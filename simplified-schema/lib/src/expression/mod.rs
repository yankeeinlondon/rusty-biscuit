//! Passive expression syntax: locating, tokenizing, and parsing `{{ … }}`
//! expressions without evaluating them.
//!
//! This module is *syntax only*. Parsing never resolves a variable, looks up
//! or calls a function, or runs a callback; a call to a function no host
//! defines (`unknown_fn(1)`) parses like any other. Evaluation, scopes, and
//! the function catalog belong to the host.
//!
//! - [`scan`]: where `{{ … }}` expressions sit in a text;
//! - [`lexer`]: the tokens of one expression;
//! - [`parser`] and [`ast`]: the expression tree, in interpolation or
//!   condition [`ParseMode`].

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod scan;

pub use ast::{
    BinaryOp, BindingPattern, CallArgument, Callback, Expr, SpannedBindingPattern, SpannedCallArgument, SpannedCallback,
    SpannedExpr, SpannedExprKind,
};
pub use lexer::{ComparisonOp, Lexer, LexerError, ParseMode, Token, identifier_prefix_start, lex_spanned};
pub use parser::{
    ParseError, Parser, parse, parse_callback, parse_callback_spanned, parse_condition, parse_condition_spanned,
    parse_spanned,
};
