//! The parent of every node of a tree, from one cursor pass.
//!
//! [`Node::parent`] descends from the root on every call, so a climb from a
//! node `d` levels deep costs `O(d²)` steps, each a scan of a child list. A
//! file's analysis climbs from nearly every capture, which made parent
//! navigation the largest single cost of cold analysis. [`with_parent_index`]
//! records every node's parent once, in `O(nodes)`, and installs the index
//! on the calling thread for the duration of one closure; inside it,
//! [`parent`] (and [`IndexedParent::indexed_parent`]) is one map lookup.
//!
//! The index is reached through the thread rather than passed down, because
//! its readers are the extraction helpers of every language (about two
//! hundred call sites), and a lookup on any other tree, on another thread
//! or outside the closure falls back to [`Node::parent`]. Either way the
//! answer is the same node.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use tree_sitter::{Node, Tree, ffi};

/// Hashes a node id, which is an address: one multiply spreads its
/// low-entropy alignment bits across the word.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn write_usize(&mut self, value: usize) {
        self.0 = (value as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    }
}

/// Every node's parent in one tree, by node id.
struct ParentIndex {
    tree: *const ffi::TSTree,
    root: usize,
    parents: HashMap<usize, ffi::TSNode, BuildHasherDefault<IdHasher>>,
}

impl ParentIndex {
    /// Visits every node of `tree` once, recording the node the cursor
    /// descended from.
    fn build(tree: &Tree) -> Self {
        let root = tree.root_node();
        let capacity = root.descendant_count().saturating_sub(1);
        let mut parents = HashMap::with_capacity_and_hasher(capacity, BuildHasherDefault::default());
        let mut path: Vec<ffi::TSNode> = Vec::new();
        let mut cursor = root.walk();
        loop {
            let node = cursor.node();
            if let Some(&parent) = path.last() {
                parents.insert(node.id(), parent);
            }
            if cursor.goto_first_child() {
                path.push(node.into_raw());
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return Self { tree: root.into_raw().tree, root: root.id(), parents };
                }
                path.pop();
            }
        }
    }
}

/// What the installed index knows of one node.
enum Indexed {
    /// The node is not of the installed tree, or none is installed.
    Unknown,
    Root,
    Parent(ffi::TSNode),
}

thread_local! {
    static INSTALLED: RefCell<Option<ParentIndex>> = const { RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// This thread's lookups: answered by the index, and fallen back to
    /// [`Node::parent`].
    static LOOKUPS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

/// Runs `f` with `tree`'s parent index installed on this thread, built on
/// entry. When the index of `tree` is already installed (a nested call),
/// `f` runs with it as is. The previous index is restored on exit, panics
/// included.
pub(crate) fn with_parent_index<R>(tree: &Tree, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<ParentIndex>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            INSTALLED.with_borrow_mut(|slot| *slot = previous);
        }
    }

    let tree_ptr = tree.root_node().into_raw().tree;
    if INSTALLED.with_borrow(|slot| slot.as_ref().is_some_and(|index| index.tree == tree_ptr)) {
        return f();
    }
    let index = ParentIndex::build(tree);
    let _restore = Restore(INSTALLED.with_borrow_mut(|slot| slot.replace(index)));
    f()
}

/// `node`'s parent: [`Node::parent`], read from the installed index when it
/// is `node`'s tree's.
pub(crate) fn parent(node: Node<'_>) -> Option<Node<'_>> {
    let raw = node.into_raw();
    let indexed = INSTALLED.with_borrow(|slot| match slot {
        Some(index) if index.tree == raw.tree => {
            let id = raw.id as usize;
            if id == index.root {
                Indexed::Root
            } else {
                index.parents.get(&id).map_or(Indexed::Unknown, |&parent| Indexed::Parent(parent))
            }
        }
        _ => Indexed::Unknown,
    });
    #[cfg(test)]
    LOOKUPS.with(|lookups| {
        let (answered, fallen_back) = lookups.get();
        lookups.set(match indexed {
            Indexed::Unknown => (answered, fallen_back + 1),
            _ => (answered + 1, fallen_back),
        });
    });
    match indexed {
        Indexed::Root => None,
        // SAFETY: `parent` was recorded from the installed tree, and `node`
        // belongs to that tree: the index is installed only while
        // `with_parent_index` borrows it, so it is alive and no other live
        // tree has its address. The result borrows the tree for as long as
        // `node` does.
        Indexed::Parent(parent) => Some(unsafe { Node::from_raw(parent) }),
        Indexed::Unknown => node.parent(),
    }
}

/// [`parent`] as a method, for chains.
pub(crate) trait IndexedParent<'tree> {
    fn indexed_parent(&self) -> Option<Node<'tree>>;
}

impl<'tree> IndexedParent<'tree> for Node<'tree> {
    fn indexed_parent(&self) -> Option<Node<'tree>> {
        parent(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("rust grammar");
        parser.parse(source, None).expect("parse")
    }

    fn every_node(tree: &Tree) -> Vec<Node<'_>> {
        let mut nodes = Vec::new();
        let mut cursor = tree.walk();
        loop {
            nodes.push(cursor.node());
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return nodes;
                }
            }
        }
    }

    #[test]
    fn indexed_parents_are_node_parents_inside_and_outside_the_scope() {
        let tree = parse("fn f() { let x = (1 + ; }\nstruct S;\n");
        let nodes = every_node(&tree);
        assert!(nodes.iter().any(|node| node.is_missing() || node.is_error()), "the fixture holds an error");
        with_parent_index(&tree, || {
            for node in &nodes {
                assert_eq!(parent(*node), node.parent(), "{node:?}");
            }
        });
        for node in &nodes {
            assert_eq!(parent(*node), node.parent(), "{node:?}");
        }
    }

    /// Node kinds seen by [`assert_indexed_parents`].
    #[derive(Default)]
    struct Seen {
        anonymous: usize,
        aliased: usize,
        missing: usize,
        errors: usize,
    }

    fn assert_indexed_parents(tree_file: &crate::file::tree_file::TreeFile, seen: &mut Seen) {
        let root = tree_file.root_node();
        let mut nodes = Vec::new();
        let mut cursor = root.walk();
        'walk: loop {
            nodes.push(cursor.node());
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break 'walk;
                }
            }
        }
        tree_file.with_parent_index(|| {
            for node in &nodes {
                assert_eq!(parent(*node), node.parent(), "{} {node:?}", tree_file.file.display());
                seen.anonymous += usize::from(!node.is_named());
                seen.aliased += usize::from(node.kind_id() != node.grammar_id());
                seen.missing += usize::from(node.is_missing());
                seen.errors += usize::from(node.is_error());
            }
        });
    }

    /// Every node of every `symbols/` fixture, plus sources that parse with
    /// missing and error nodes, has its [`Node::parent`] as indexed parent.
    #[test]
    fn indexed_parents_match_node_parents_over_the_symbols_fixtures() {
        let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/symbols");
        let mut pending = vec![fixtures];
        let mut files = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("fixture directory") {
                let path = entry.expect("fixture entry").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        files.sort();
        let mut seen = Seen::default();
        let mut parsed = 0;
        for path in &files {
            let Ok(tree_file) = crate::file::tree_file::TreeFile::new(path) else { continue };
            assert_indexed_parents(&tree_file, &mut seen);
            parsed += 1;
        }
        assert!(parsed >= 40, "parsed {parsed} of {} fixtures", files.len());
        let scratch = tempfile::TempDir::new().expect("temp");
        for (name, source) in [
            ("missing.rs", "fn f() { let x = (1 + ; }\n"),
            ("missing.ts", "function f( { return [1, 2; }\n"),
            ("broken.py", "def f(:\n    return )\n"),
        ] {
            let path = scratch.path().join(name);
            std::fs::write(&path, source).expect("source");
            assert_indexed_parents(&crate::file::tree_file::TreeFile::new(&path).expect("parse"), &mut seen);
        }
        assert!(seen.anonymous > 0 && seen.aliased > 0 && seen.missing > 0 && seen.errors > 0,
            "anonymous {}, aliased {}, missing {}, errors {}", seen.anonymous, seen.aliased, seen.missing, seen.errors);
    }

    /// The shape of `schematic/definitions/src/openai/mod.rs`, whose
    /// `--json-full` analysis took 536 s of root descents before the index:
    /// one function building a long array of struct literals full of
    /// `vec![…]` token trees. Every parent lookup of a complete analysis is
    /// answered by the index; none descends from the root.
    #[test]
    fn a_complete_analysis_of_a_wide_generated_file_never_descends_from_the_root() {
        use crate::file::query::SymbolQuery;
        let entries: String = (0..120)
            .map(|idx| {
                format!(
                    "        Endpoint {{ id: \"e{idx}\", tags: vec![\"a\", \"b\", vec![{idx}, {idx}].len().to_string()], \
                     params: vec![Param {{ name: \"p{idx}\", kinds: vec![Kind::A, Kind::B] }}], run: |x: u8| x + {idx} }},\n"
                )
            })
            .collect();
        let source = format!(
            "struct Endpoint {{ id: &'static str }}\nstruct Param;\nenum Kind {{ A, B }}\n\
             pub fn endpoints() -> Vec<Endpoint> {{\n    Vec::from([\n{entries}    ])\n}}\n"
        );
        let scratch = tempfile::TempDir::new().expect("temp");
        let path = scratch.path().join("generated.rs");
        std::fs::write(&path, source).expect("source");
        let tree_file = crate::file::tree_file::TreeFile::new(&path).expect("parse");
        LOOKUPS.with(|lookups| lookups.set((0, 0)));
        let result = tree_file.query(&SymbolQuery::complete_index()).expect("analysis");
        let (answered, fallen_back) = LOOKUPS.with(std::cell::Cell::get);
        assert!(!result.index.occurrences.is_empty());
        assert!(answered > 10_000, "{answered} lookups answered by the index");
        assert_eq!(fallen_back, 0, "{fallen_back} lookups descended from the root");
    }

    #[test]
    fn another_trees_nodes_fall_back_and_the_scope_restores_on_exit() {
        let (outer, inner) = (parse("fn a() { b(); }"), parse("mod m { fn c() {} }"));
        let inner_nodes = every_node(&inner);
        with_parent_index(&outer, || {
            for node in &inner_nodes {
                assert_eq!(parent(*node), node.parent());
            }
            with_parent_index(&inner, || {
                for node in &inner_nodes {
                    assert_eq!(parent(*node), node.parent());
                }
            });
            assert!(INSTALLED.with_borrow(|slot| slot.as_ref().is_some_and(|index| index.root == outer.root_node().id())));
        });
        assert!(INSTALLED.with_borrow(Option::is_none));
    }

    #[test]
    fn a_panic_inside_the_scope_uninstalls_the_index() {
        let tree = parse("fn a() {}");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_parent_index(&tree, || panic!("inside"));
        }));
        assert!(result.is_err());
        assert!(INSTALLED.with_borrow(Option::is_none));
    }
}
