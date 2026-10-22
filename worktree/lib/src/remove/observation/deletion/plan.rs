//! The order in which [`delete_content`](super::delete_content) deletes an
//! inventory, shared with the journal's size bound
//! ([`journal_size`](crate::remove::recovery::journal::journal_size)) so the
//! two can never group entries differently.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::remove::observation::inventory::{DeletionInventory, InventoryEntry};
use crate::remove::recovery::journal::MAX_BATCH_ENTRIES;

/// Every batch one deletion attempt can journal, grouped by the depth level
/// after which the engine drains its observer.
///
/// Ordinary content comes first and `.git` last; within each, the deepest
/// level first, then directories in path order, then each directory's
/// entries in inventory order, in chunks of at most [`MAX_BATCH_ENTRIES`].
/// A batch never spans two directories, so the checkout root's content and
/// its `.git` are separate batches although they share a parent.
///
/// Every inventory entry is planned exactly once. The journal's writer only
/// checks a batch against the inventory, not against earlier batches, so this
/// invariant is what makes the attempt's largest journal computable before it
/// starts.
#[derive(Debug)]
pub(crate) struct BatchPlan<'a> {
    levels: Vec<Vec<Vec<&'a InventoryEntry>>>,
}

/// The entries of one directory, by depth then directory.
type Levels<'a> = BTreeMap<usize, BTreeMap<PathBuf, Vec<&'a InventoryEntry>>>;

impl<'a> BatchPlan<'a> {
    pub(crate) fn of(inventory: &'a DeletionInventory) -> Self {
        let (content, git): (Vec<&InventoryEntry>, Vec<&InventoryEntry>) =
            inventory.entries().iter().partition(|entry| !entry.path.as_path().starts_with(".git"));
        let mut levels = Vec::new();
        for entries in [content, git] {
            for (_, directories) in by_level(&entries).into_iter().rev() {
                let mut level = Vec::new();
                for (_, children) in directories {
                    level.extend(children.chunks(MAX_BATCH_ENTRIES).map(<[_]>::to_vec));
                }
                levels.push(level);
            }
        }
        Self { levels }
    }

    /// The batches, one inner list per level in deletion order.
    pub(crate) fn levels(&self) -> &[Vec<Vec<&'a InventoryEntry>>] {
        &self.levels
    }

    /// Every batch with the number the journal gives it, from 1.
    pub(crate) fn numbered(&self) -> impl Iterator<Item = (u64, &[&'a InventoryEntry])> {
        self.levels.iter().flatten().zip(1..).map(|(batch, number)| (number, batch.as_slice()))
    }
}

fn by_level<'a>(entries: &[&'a InventoryEntry]) -> Levels<'a> {
    let mut levels: Levels<'a> = BTreeMap::new();
    for entry in entries {
        let path = entry.path.as_path();
        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
        levels.entry(path.components().count()).or_default().entry(parent).or_default().push(entry);
    }
    levels
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sniff::filesystem::query::FileIdentity;

    use super::*;

    fn listing(files: &[&str]) -> DeletionInventory {
        DeletionInventory::listing(FileIdentity::Unix { device: 1, inode: 1 }, files)
    }

    fn paths(batch: &[&InventoryEntry]) -> Vec<String> {
        batch.iter().map(|entry| entry.path.display().into_owned()).collect()
    }

    /// Every entry exactly once; content before `.git`; deepest level
    /// first; directories in path order; a directory's entries in inventory
    /// order and in chunks of at most `MAX_BATCH_ENTRIES`; the root's content
    /// and its `.git` in separate batches.
    #[test]
    fn every_entry_is_planned_once_in_the_engines_order() {
        let mut files = vec!["README.md".to_string(), ".git/HEAD".into(), ".git/refs/main".into(), "a/x".into(), "b/y".into()];
        files.extend((0..MAX_BATCH_ENTRIES + 3).map(|n| format!("wide/{n:04}")));
        files.push(".gitignore".into());
        let inventory = listing(&files.iter().map(String::as_str).collect::<Vec<_>>());
        let plan = BatchPlan::of(&inventory);

        let mut seen: HashMap<&Path, usize> = HashMap::new();
        for (_, batch) in plan.numbered() {
            for entry in batch {
                *seen.entry(entry.path.as_path()).or_default() += 1;
            }
        }
        assert_eq!(seen.len(), inventory.entries().len());
        assert!(seen.values().all(|count| *count == 1), "{seen:?}");

        let batches: Vec<(u64, Vec<String>)> = plan.numbered().map(|(number, batch)| (number, paths(batch))).collect();
        let numbers: Vec<u64> = batches.iter().map(|(number, _)| *number).collect();
        assert_eq!(numbers, (1..=batches.len() as u64).collect::<Vec<_>>());
        let wide: Vec<usize> = batches.iter().filter(|(_, batch)| batch[0].starts_with("wide/")).map(|(_, batch)| batch.len()).collect();
        assert_eq!(wide, vec![MAX_BATCH_ENTRIES, 3]);
        // Within a directory, inventory order (here the listing's insertion
        // order), never re-sorted; `.gitignore` is content, not `.git`.
        let expected = [
            vec!["a/x".to_string()],
            vec!["b/y".into()],
            vec!["README.md".into(), "a".into(), "b".into(), "wide".into(), ".gitignore".into()],
            vec![".git/refs/main".into()],
            vec![".git/HEAD".into(), ".git/refs".into()],
            vec![".git".into()],
        ];
        let without_wide: Vec<Vec<String>> = batches.into_iter().map(|(_, batch)| batch).filter(|batch| !batch[0].starts_with("wide/")).collect();
        assert_eq!(without_wide, expected);
        // Levels: depth 2 content, depth 1 content, then `.git` depths 3, 2, 1.
        let level_sizes: Vec<usize> = plan.levels().iter().map(Vec::len).collect();
        assert_eq!(level_sizes, vec![4, 1, 1, 1, 1]);
    }

    #[test]
    fn an_empty_inventory_plans_no_batch() {
        let inventory = DeletionInventory::empty(FileIdentity::Unix { device: 1, inode: 1 });
        let plan = BatchPlan::of(&inventory);
        assert!(plan.levels().is_empty());
        assert_eq!(plan.numbered().count(), 0);
    }
}
