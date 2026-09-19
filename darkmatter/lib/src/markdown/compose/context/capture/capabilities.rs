//! Deferred capability planning for the lazy roots and expression functions.
//!
//! Two planning products stay distinct (decision D4):
//!
//! - [`ContextRequirements`](super::ContextRequirements) — the **eager** `ctx.*`
//!   groups a source needs captured before composition starts.
//! - [`DeferredCapabilities`] — the **lazy** `current.<key>` keys,
//!   `current_env.<KEY>` names, and expression functions a source *can* reach.
//!
//! A `current.*` reference never adds an eager requirement, and an eager
//! requirement never answers a `current.*` read. Planning a deferred capability
//! costs nothing: it records descriptor keys and function names, and observes
//! no value. A reference the evaluator never reaches — an unchosen ternary
//! branch, a short-circuited fallback — is still planned here and still
//! observes nothing, which is what lets preflight report a document's reachable
//! context reads without performing them.

use std::collections::BTreeSet;

use crate::markdown::compose::expression::expression_function_descriptors;

use super::groups::scan_literal_spans;

/// The lazy reads and function calls one source can reach.
///
/// Recorded as names only. Nothing here is observed until evaluation reaches
/// the reference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeferredCapabilities {
    current: BTreeSet<String>,
    current_env: BTreeSet<String>,
    functions: BTreeSet<String>,
}

impl DeferredCapabilities {
    /// Scans content for lazy-root references and expression function calls.
    pub fn for_content(content: &str) -> Self {
        let literal_spans = scan_literal_spans(content);
        let mut planned = Self::default();
        planned.scan_root(content, &literal_spans, "current.", |planned, key| {
            // A cataloged key is the capability a provider must hold; anything
            // else is an unknown path the evaluator rejects, never a capability
            // an invocation could be asked to supply.
            if super::ContextGroup::for_key(key).is_some() {
                planned.current.insert(key.to_string());
            }
        });
        planned.scan_root(content, &literal_spans, "current_env.", |planned, key| {
            planned.current_env.insert(key.to_string());
        });
        planned.scan_functions(content, &literal_spans);
        planned
    }

    /// Scans both authored frontmatter values and the document body.
    pub fn for_document(document: &crate::markdown::Markdown) -> Self {
        let frontmatter =
            serde_json::to_string(document.frontmatter().as_map()).unwrap_or_default();
        Self::for_content(&format!("{frontmatter}\n{}", document.content()))
    }

    /// The cataloged `ctx` keys a `current.<key>` reference could refresh.
    pub fn current_keys(&self) -> impl Iterator<Item = &str> {
        self.current.iter().map(String::as_str)
    }

    /// The environment names a `current_env.<KEY>` reference could reread.
    pub fn current_env_keys(&self) -> impl Iterator<Item = &str> {
        self.current_env.iter().map(String::as_str)
    }

    /// The cataloged expression functions this source could call.
    pub fn functions(&self) -> impl Iterator<Item = &str> {
        self.functions.iter().map(String::as_str)
    }

    /// Whether this source reaches no lazy read and no function call.
    pub fn is_empty(&self) -> bool {
        self.current.is_empty() && self.current_env.is_empty() && self.functions.is_empty()
    }

    /// Every capability planned by this set or by `other`.
    pub fn union(mut self, other: &Self) -> Self {
        self.current.extend(other.current.iter().cloned());
        self.current_env.extend(other.current_env.iter().cloned());
        self.functions.extend(other.functions.iter().cloned());
        self
    }

    fn scan_root(
        &mut self,
        content: &str,
        literal_spans: &[std::ops::Range<usize>],
        prefix: &str,
        mut record: impl FnMut(&mut Self, &str),
    ) {
        let mut pos = 0;
        while let Some(offset) = content[pos..].find(prefix) {
            let start = pos + offset;
            let key_start = start + prefix.len();
            let key_end = content[key_start..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|offset| key_start + offset)
                .unwrap_or(content.len());
            if key_end > key_start
                && is_root_reference(content, start)
                && !literal_spans.iter().any(|span| span.start <= start && key_end <= span.end)
            {
                record(self, &content[key_start..key_end]);
            }
            pos = key_end.max(key_start);
        }
    }

    fn scan_functions(&mut self, content: &str, literal_spans: &[std::ops::Range<usize>]) {
        let bytes = content.as_bytes();
        let is_ident = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
        let mut pos = 0;
        while pos < bytes.len() {
            if !is_ident(bytes[pos]) {
                pos += 1;
                continue;
            }
            let start = pos;
            while pos < bytes.len() && is_ident(bytes[pos]) {
                pos += 1;
            }
            if !is_root_reference(content, start) {
                continue;
            }
            let name = &content[start..pos];
            if !content[pos..].trim_start().starts_with('(') {
                continue;
            }
            if literal_spans.iter().any(|span| span.start <= start && pos <= span.end) {
                continue;
            }
            if expression_function_descriptors()
                .iter()
                .any(|descriptor| descriptor_name(descriptor) == name)
            {
                self.functions.insert(name.to_string());
            }
        }
    }
}

/// The bare function name of a descriptor whose display key is its signature.
fn descriptor_name(
    descriptor: &crate::markdown::compose::expression::ExpressionFunctionDescriptor,
) -> &str {
    descriptor
        .signature
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(descriptor.signature)
}

/// Whether the identifier at `start` is a root and not a trailing path segment.
fn is_root_reference(content: &str, start: usize) -> bool {
    let Some(previous) = content[..start].chars().next_back() else {
        return true;
    };
    !(previous.is_alphanumeric() || previous == '_' || previous == '.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ContextRequirements;
    use crate::markdown::compose::context::capture::ContextGroup;

    fn keys(planned: &DeferredCapabilities) -> Vec<&str> {
        planned.current_keys().collect()
    }

    #[test]
    fn a_lazy_reference_is_planned_as_a_capability_and_never_as_an_eager_requirement() {
        let content = "{{ current.branch }} and {{ current.recent_commits }}";
        let planned = DeferredCapabilities::for_content(content);

        assert_eq!(keys(&planned), ["branch", "recent_commits"]);
        assert_eq!(
            ContextRequirements::for_content(content),
            ContextRequirements::from_groups([ContextGroup::DateTime]),
            "a `current.*` reference must add no eager capture",
        );
    }

    #[test]
    fn the_removed_nesting_names_no_capability_and_no_eager_requirement() {
        let content = "{{ current.ctx.agent }} {{ current.env.HOME }}";
        let planned = DeferredCapabilities::for_content(content);

        assert!(keys(&planned).is_empty(), "`ctx` is not a cataloged context key");
        assert_eq!(
            ContextRequirements::for_content(content),
            ContextRequirements::from_groups([ContextGroup::DateTime]),
            "a trailing `ctx.` path segment is not a `ctx` root",
        );
    }

    #[test]
    fn environment_names_and_function_calls_are_planned_separately() {
        let planned = DeferredCapabilities::for_content(
            "{{ current_env.HOME }} {{ recent_commits(3) }} {{ length(ipv4()) }}",
        );

        assert_eq!(planned.current_env_keys().collect::<Vec<_>>(), ["HOME"]);
        assert_eq!(
            planned.functions().collect::<Vec<_>>(),
            ["ipv4", "length", "recent_commits"],
        );
        assert!(keys(&planned).is_empty());
    }

    #[test]
    fn inert_literals_and_longer_identifiers_plan_nothing() {
        let planned = DeferredCapabilities::for_content(
            "{{{ current.branch }}} {{{ ipv4() }}} a.current.branch my_current.branch currently",
        );
        assert!(planned.is_empty(), "{planned:?}");
    }

    #[test]
    fn an_unreached_branch_is_still_planned_because_planning_observes_nothing() {
        let planned =
            DeferredCapabilities::for_content("{{ false ? current.branch : current.os }}");
        assert_eq!(keys(&planned), ["branch", "os"]);
    }

    #[test]
    fn a_document_plans_its_frontmatter_and_its_body() {
        let document: crate::markdown::Markdown = "---\nbranch: '{{ current.branch }}'\n---\n\
             host is {{ current_env.HOSTNAME }}\n"
            .into();
        let planned = DeferredCapabilities::for_document(&document);

        assert_eq!(keys(&planned), ["branch"]);
        assert_eq!(planned.current_env_keys().collect::<Vec<_>>(), ["HOSTNAME"]);
    }

    #[test]
    fn a_union_keeps_every_planned_capability() {
        let left = DeferredCapabilities::for_content("{{ current.branch }}");
        let right = DeferredCapabilities::for_content("{{ current.os }} {{ current_env.PATH }}");
        let merged = left.union(&right);

        assert_eq!(keys(&merged), ["branch", "os"]);
        assert_eq!(merged.current_env_keys().collect::<Vec<_>>(), ["PATH"]);
    }
}
