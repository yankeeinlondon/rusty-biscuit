//! The guard facts a body `::block when="…"` gives the sites inside it.
//!
//! Composition removes a false block before any later body stage runs, so a
//! `::shell`, `::shell-block` command, or transclusion target inside a block
//! is reached only when every enclosing `when` held. [`BodyGuards`] records
//! those regions from the authored body so each site's
//! [`TypeScope`] can [`assume`](TypeScope::assume_at) the true side of every
//! enclosing guard, outermost first.
//!
//! `BodyGuards` is the conditional projection of
//! [`BodyScopes`](crate::markdown::compose::collection_blocks::BodyScopes),
//! the body's one scope inventory, which layers `::map` / `::filter` region
//! locals over the same guards
//! ([`BodyScopes::conditional`](crate::markdown::compose::collection_blocks::BodyScopes::conditional)
//! returns this view). A document with no collection region answers
//! identically through either. Shell discovery, shell execution, and DMLS
//! still read the guards alone, so a shell site inside a collection body sees
//! its enclosing `::block` facts but not the region's locals; the composition
//! call check and transclusion-target analysis read `BodyScopes`.
//!
//! A site gets the same facts whichever consumer asks; a shell site's
//! approval shape follows its guard refinements, so discovery and execution
//! must agree.
//!
//! Scanning is passive: it parses the block markers and their conditions and
//! evaluates nothing.
//!
//! ```
//! use darkmatter::markdown::compose::expression::{DeclaredTypes, TypeScope, parse};
//! use darkmatter::markdown::compose::page_blocks::BodyGuards;
//!
//! let body = "::block when=\"name\"\n::shell echo {{ name }}\n::end-block\n::shell echo {{ name }}\n";
//! let guards = BodyGuards::scan(body);
//! let declared = DeclaredTypes::new(None);
//! let name = parse("name").unwrap();
//!
//! let inside = guards.scope_at_line(TypeScope::unguarded(&declared), 2);
//! assert!(inside.type_of_expr(&name).unwrap().guard_presence().is_some());
//! let outside = guards.scope_at_line(TypeScope::unguarded(&declared), 4);
//! assert!(!outside.type_of_expr(&name).unwrap().proves_present());
//! ```

use std::path::PathBuf;

use super::parser::parse_page_blocks;
use super::types::PageBlockRegion;
use crate::markdown::compose::expression::{Expr, GuardOrigin, Polarity, TypeScope, parse_condition};
use crate::markdown::span::SourceSpan;

/// One body `::block` whose `when` condition parsed.
#[derive(Debug, Clone)]
pub struct BodyGuard {
    /// The parsed `when` condition.
    pub condition: Expr,
    /// Byte span of the block's body (between its marker lines) in the
    /// scanned text.
    pub body_span: SourceSpan,
    /// 1-based line of the `::block` marker in the scanned text.
    pub line: usize,
    origin: GuardOrigin,
}

impl BodyGuard {
    /// Where the guard came from: the marker's line and authored condition.
    /// The label is the same for every reader of one body.
    pub fn origin(&self) -> &GuardOrigin {
        &self.origin
    }

    /// `scope` on the true side of this guard.
    pub fn assume<'d>(&self, scope: &TypeScope<'d>) -> TypeScope<'d> {
        scope.assume_at(&self.condition, Polarity::True, self.origin.clone())
    }
}

/// Every guarded body region of one text, outermost first within each
/// nesting chain.
#[derive(Debug, Clone, Default)]
pub struct BodyGuards {
    guards: Vec<BodyGuard>,
    /// Byte offset of the start of each line; index 0 is line 1.
    line_starts: Vec<usize>,
}

impl BodyGuards {
    /// Scans `body` for `::block when="…"` regions, pairing markers exactly as
    /// composition does.
    ///
    /// A body whose blocks do not pair, and a `when` condition that does not
    /// parse, contribute no guard: composition reports those as errors, and a
    /// missing fact only makes analysis more conservative.
    pub fn scan(body: &str) -> Self {
        Self::scan_body(body, 0)
    }

    /// [`Self::scan`] for a document body that starts after
    /// `frontmatter_lines` lines of its file, so each guard's label names the
    /// marker's file line, as shell origins and DMLS do. Lookups by line
    /// ([`Self::enclosing_line`]) stay relative to `body`.
    pub fn scan_body(body: &str, frontmatter_lines: usize) -> Self {
        let context = biscuit_terminal::errors::SourceContext::new(PathBuf::new(), PathBuf::new(), "");
        let mut guards = Vec::new();
        if let Ok(regions) = parse_page_blocks(body, context) {
            collect(&regions, frontmatter_lines, &mut guards);
        }
        let line_starts = std::iter::once(0)
            .chain(body.match_indices('\n').map(|(index, _)| index + 1))
            .collect();
        Self { guards, line_starts }
    }

    /// Whether the body has no guarded region.
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// The guards whose body holds byte `offset`, outermost first.
    pub fn enclosing(&self, offset: usize) -> Vec<&BodyGuard> {
        self.guards
            .iter()
            .filter(|guard| guard.body_span.start <= offset && offset < guard.body_span.end)
            .collect()
    }

    /// The guards whose body holds the 1-based `line`, outermost first.
    pub fn enclosing_line(&self, line: usize) -> Vec<&BodyGuard> {
        match line.checked_sub(1).and_then(|index| self.line_starts.get(index)) {
            Some(&offset) => self.enclosing(offset),
            None => Vec::new(),
        }
    }

    /// `site` narrowed by every guard enclosing byte `offset`.
    pub fn scope_at<'d>(&self, site: TypeScope<'d>, offset: usize) -> TypeScope<'d> {
        within(&self.enclosing(offset), site)
    }

    /// `site` narrowed by every guard enclosing the 1-based `line`.
    pub fn scope_at_line<'d>(&self, site: TypeScope<'d>, line: usize) -> TypeScope<'d> {
        within(&self.enclosing_line(line), site)
    }
}

/// `site` on the true side of each of `guards`, applied in order (outermost
/// first), so an inner guard is checked under its outer guards' facts.
pub fn within<'d>(guards: &[&BodyGuard], site: TypeScope<'d>) -> TypeScope<'d> {
    guards.iter().fold(site, |scope, guard| guard.assume(&scope))
}

/// Pre-order, so a parent precedes its children.
fn collect(regions: &[PageBlockRegion], label_offset: usize, guards: &mut Vec<BodyGuard>) {
    for region in regions {
        if let Some(when) = &region.options.when_expr
            && let Ok(condition) = parse_condition(when)
        {
            guards.push(BodyGuard {
                condition,
                body_span: region.body_span.clone(),
                line: region.start_line,
                origin: GuardOrigin::new(format!("::block when=\"{when}\" (line {})", region.start_line + label_offset))
                    .with_span(region.span.start..region.body_span.start),
            });
        }
        collect(&region.children, label_offset, guards);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::{DeclaredTypes, parse};

    fn proven(scope: &TypeScope<'_>, path: &str) -> bool {
        scope.type_of_expr(&parse(path).unwrap()).unwrap().guard_presence().is_some()
    }

    #[test]
    fn nested_blocks_compose_and_end_with_their_region() {
        let body = "::block when=\"a\"\n::block when=\"b\"\ninner\n::end-block\nouter\n::end-block\nafter\n";
        let guards = BodyGuards::scan(body);
        let declared = DeclaredTypes::new(None);
        let site = |line| guards.scope_at_line(TypeScope::unguarded(&declared), line);

        assert!(proven(&site(3), "a") && proven(&site(3), "b"));
        assert!(proven(&site(5), "a") && !proven(&site(5), "b"));
        assert!(!proven(&site(7), "a"));
        // The marker lines themselves are not inside a body.
        assert!(!proven(&site(1), "a"));
        assert!(guards.enclosing_line(99).is_empty());
    }

    #[test]
    fn an_unpaired_or_unparsable_block_gives_no_guard() {
        assert!(BodyGuards::scan("::block when=\"a\"\nbody\n").is_empty());
        assert!(BodyGuards::scan("::block when=\"a ==\"\nbody\n::end-block\n").is_empty());
        assert!(BodyGuards::scan("::block\nbody\n::end-block\n").is_empty());
    }

    #[test]
    fn the_origin_names_the_marker_line_and_its_condition() {
        let guards = BodyGuards::scan("text\n::block when=\"x\"\nbody\n::end-block\n");
        let [guard] = guards.enclosing_line(3)[..] else {
            panic!("one guard");
        };
        assert_eq!(guard.origin().label(), "::block when=\"x\" (line 2)");
        assert_eq!(guard.line, 2);
    }
}
