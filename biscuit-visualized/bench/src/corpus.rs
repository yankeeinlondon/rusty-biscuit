//! The benchmark corpus: one input per Mermaid family, the same fixtures the
//! golden tests use, embedded so a cold one-shot child reads no files.

macro_rules! families {
    ($($name:literal),* $(,)?) => {
        /// Every family, keyed by fixture stem.
        pub const FAMILIES: &[(&str, &str)] = &[
            $(($name, include_str!(concat!("../../src/src/tests/fixtures/merman/families/", $name, ".mmd")))),*
        ];
    };
}

families!(
    "agentflow",
    "architecture",
    "block",
    "c4",
    "class",
    "cynefin",
    "er",
    "eventmodeling",
    "flowchart",
    "gantt",
    "gitgraph",
    "info",
    "ishikawa",
    "journey",
    "kanban",
    "mindmap",
    "packet",
    "pie",
    "quadrantchart",
    "radar",
    "railroad",
    "requirement",
    "sankey",
    "sequence",
    "state",
    "swimlane",
    "timeline",
    "treemap",
    "treeView",
    "usecase",
    "venn",
    "wardley",
    "xychart",
);

/// The fixture source of `family`, or `None` for a name not in the corpus.
pub fn source(family: &str) -> Option<&'static str> {
    FAMILIES
        .iter()
        .find(|(name, _)| *name == family)
        .map(|(_, source)| *source)
}

pub fn names() -> impl Iterator<Item = &'static str> {
    FAMILIES.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixture added to the families folder must be added to the list above.
    #[test]
    fn corpus_lists_every_family_fixture() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/src/tests/fixtures/merman/families");
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
            .expect("families fixture dir")
            .map(|entry| entry.expect("dir entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "mmd"))
            .map(|path| {
                path.file_stem()
                    .expect("stem")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        on_disk.sort();
        let mut listed: Vec<String> = names().map(str::to_owned).collect();
        listed.sort();
        assert_eq!(listed, on_disk);
    }
}
