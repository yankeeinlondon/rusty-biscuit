//! Object-spread sites: which `...operand` of an expression a runtime
//! failure belongs to.
//!
//! The evaluator runs the span-free [`Expr`], so a failing spread knows only
//! its own node. The expression's entry point turns that node into its
//! *ordinal*, its position among the expression's nodes as
//! [`expr_nodes`](super::member_check::expr_nodes) lists them. A reporter
//! holding the authored text parses it with spans and takes the node with
//! the same ordinal from
//! [`spanned_nodes`](super::member_check::spanned_nodes), which lists the
//! same nodes in the same order. A callback body evaluated once per item is
//! one node, so every iteration's failure maps to the one authored operand.

use simplified_schema::expression::ast::{Expr, SpannedExpr};

use super::member_check::{expr_nodes, spanned_nodes};
use crate::markdown::span::SourceSpan;

/// The spread operand an [`InvalidSpreadOperand`](super::ExpressionError::InvalidSpreadOperand)
/// failure came from.
///
/// The node identity is meaningful only while the evaluated expression is
/// alive; the entry point that evaluated it fills in the
/// [ordinal](Self::ordinal) before the error leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadSite {
    node: usize,
    ordinal: Option<usize>,
}

impl SpreadSite {
    /// The site of the evaluated operand `operand`.
    pub(crate) fn of(operand: &Expr) -> Self {
        Self { node: std::ptr::from_ref(operand).addr(), ordinal: None }
    }

    /// The operand's node ordinal in the evaluated expression; `None` when
    /// the error did not come through an expression entry point that could
    /// place it. [`spread_operand_span`] turns it into the authored span.
    pub fn ordinal(&self) -> Option<usize> {
        self.ordinal
    }

    /// Places the site within `root`, the expression that was evaluated.
    pub(crate) fn place_in(&mut self, root: &Expr) {
        if self.ordinal.is_none() {
            self.ordinal = expr_nodes(root).iter().position(|node| std::ptr::from_ref(*node).addr() == self.node);
        }
    }
}

/// The authored span of the node with `ordinal` in `root`, in the text
/// `root` was parsed from.
pub fn spread_operand_span(root: &SpannedExpr, ordinal: usize) -> Option<SourceSpan> {
    spanned_nodes(root).get(ordinal).map(|node| node.span.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplified_schema::expression::ast::ObjectEntry;
    use simplified_schema::expression::parser::{parse, parse_spanned};

    /// Every spread operand of `expr`, in node order.
    fn operands(expr: &Expr) -> Vec<&Expr> {
        expr_nodes(expr)
            .into_iter()
            .filter_map(|node| match node {
                Expr::ObjectLiteral(entries) => Some(entries),
                _ => None,
            })
            .flatten()
            .filter_map(|entry| match entry {
                ObjectEntry::Spread { operand } => Some(operand),
                ObjectEntry::Field { .. } => None,
            })
            .collect()
    }

    /// Every operand, found from its erased node, maps back to its own
    /// authored text, callback bodies and nested spreads included.
    #[test]
    fn each_erased_operand_maps_to_its_authored_operand() {
        let source = "map(xs, x => { ...x, inner: { ...(x.more), k: [ { ...base } ] } }) || { ...tail }";
        let spanned = parse_spanned(source).unwrap();
        let erased = spanned.erase();
        let mut texts: Vec<&str> = operands(&erased)
            .into_iter()
            .map(|operand| {
                let mut site = SpreadSite::of(operand);
                site.place_in(&erased);
                &source[spread_operand_span(&spanned, site.ordinal().unwrap()).unwrap()]
            })
            .collect();
        texts.sort_unstable();
        assert_eq!(texts, ["(x.more)", "base", "tail", "x"]);
    }

    #[test]
    fn a_node_outside_the_expression_has_no_ordinal() {
        let root = parse("{ ...a }").unwrap();
        let other = parse("a").unwrap();
        let mut site = SpreadSite::of(&other);
        site.place_in(&root);
        assert_eq!(site.ordinal(), None);
        assert_eq!(spread_operand_span(&parse_spanned("{ ...a }").unwrap(), 9), None);
    }
}
