//! Canonical inline-namespace trees provide member ranges without expanding
//! the combinations of omitted namespace segments into aliases.

use std::collections::{HashMap, HashSet};

use crate::analysis::counters::count_shared_context_steps;

pub(super) struct InlineNames {
    declared: HashSet<String>,
    /// The preorder range of each namespace with an inline namespace below
    /// it, and of each inline namespace; no other scope has a range.
    ranges: HashMap<String, (usize, usize)>,
    /// The direct members of the ranged scopes, by segment, at their
    /// scope's preorder point (ascending).
    members: HashMap<String, Vec<(usize, String)>>,
}

impl InlineNames {
    pub(super) fn new<'a>(names: impl Iterator<Item = &'a str>, children: &HashMap<String, Vec<String>>) -> Self {
        let inline: HashSet<&str> = children.values().flatten().map(String::as_str).collect();
        let ranged = |scope: &str| children.contains_key(scope) || inline.contains(scope);
        let mut declared = HashSet::new();
        let mut direct: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for name in names {
            count_shared_context_steps(1);
            if !declared.insert(name.to_string()) {
                continue;
            }
            let (parent, segment) = name.rsplit_once("::").unwrap_or(("", name));
            if ranged(parent) {
                direct.entry(parent.to_string()).or_default().push((segment.to_string(), name.to_string()));
            }
        }
        let mut members: HashMap<String, Vec<(usize, String)>> = HashMap::new();
        let mut ranges: HashMap<String, (usize, usize)> = HashMap::new();
        let mut next = 0;
        for root in children.keys().filter(|name| !inline.contains(name.as_str())) {
            let mut stack = vec![(root.as_str(), false)];
            while let Some((name, exit)) = stack.pop() {
                count_shared_context_steps(1);
                if exit {
                    ranges.get_mut(name).expect("visited namespace").1 = next;
                } else {
                    ranges.insert(name.to_string(), (next, next));
                    // Preorder insertion keeps each segment's member points
                    // sorted without another input-sized sorting pass.
                    for (segment, member) in direct.remove(name).unwrap_or_default() {
                        count_shared_context_steps(1);
                        members.entry(segment).or_default().push((next, member));
                    }
                    next += 1;
                    stack.push((name, true));
                    if let Some(nested) = children.get(name) {
                        stack.extend(nested.iter().rev().map(|child| (child.as_str(), false)));
                    }
                }
            }
        }
        Self { declared, ranges, members }
    }

    /// The declared names called `segment` in `scope` or in an inline
    /// namespace below it. A scope with no inline namespace below it has at
    /// most its own direct member, found by name; only a range that spans
    /// inline namespaces is searched.
    pub(super) fn members<'a>(&'a self, scope: &str, segment: &str) -> impl Iterator<Item = &'a String> {
        count_shared_context_steps(1);
        let entries = match self.ranges.get(scope) {
            Some(&(start, end)) if end > start + 1 => self.members.get(segment).map(|entries| {
                let first = entries.partition_point(|(point, _)| { count_shared_context_steps(1); *point < start });
                let after = entries.partition_point(|(point, _)| { count_shared_context_steps(1); *point < end });
                &entries[first..after]
            }),
            _ => {
                let exact = if scope.is_empty() { segment.to_string() } else { format!("{scope}::{segment}") };
                return self.declared.get(&exact).into_iter().chain([].iter().map(member_name));
            }
        };
        None.into_iter().chain(entries.unwrap_or_default().iter().map(member_name))
    }
}

fn member_name(entry: &(usize, String)) -> &String { &entry.1 }
