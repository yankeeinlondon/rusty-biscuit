/// A component capable of rendering itself to an abstract syntax tree
/// representation.
///
/// The AST node model is intentionally not specified yet — this trait
/// exists so AST becomes a first-class render target alongside
/// `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`.
/// The single method returns a serialized AST string; a typed node
/// model is deferred until a concrete consumer needs it.
///
/// ## Notes
///
/// - This is a placeholder surface. It will gain a typed node return
///   value in a future project once the AST representation is designed.
pub trait AstRenderable {
    /// Renders the component to a serialized AST string.
    fn render_ast(&self) -> String;
}
