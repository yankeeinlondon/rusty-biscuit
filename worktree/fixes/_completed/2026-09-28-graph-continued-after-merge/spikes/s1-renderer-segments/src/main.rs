//! Spike S1: renderer segment semantics for a lane merged mid-lane.
//!
//! Prints each case's laid-out commits (layout index, lane, id, parents) and
//! renders the SVG of cases 1 and 3 beside this crate.

use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
use biscuit_visualized::mermaid::MermaidDiagram;

const CASES: &[(&str, &str)] = &[
    (
        "1-mid-lane-merge",
        r#"gitGraph
    commit id: "r"
    branch b
    checkout b
    commit id: "B"
    checkout main
    commit id: "P"
    merge b id: "C"
    checkout b
    commit id: "N"
    checkout main"#,
    ),
    (
        "2-square-after-source",
        r#"gitGraph
    commit id: "r"
    branch b
    checkout b
    commit id: "B"
    checkout main
    commit id: "P"
    merge b id: "C"
    checkout b
    commit id: "+2" type: HIGHLIGHT
    commit id: "N"
    checkout main"#,
    ),
    (
        "3-merged-twice",
        r#"gitGraph
    commit id: "r"
    branch b
    checkout b
    commit id: "B1"
    checkout main
    commit id: "P1"
    merge b id: "C1"
    checkout b
    commit id: "B2"
    checkout main
    commit id: "P2"
    merge b id: "C2"
    checkout b
    commit id: "N"
    checkout main"#,
    ),
    (
        "4-child-at-source",
        r#"gitGraph
    commit id: "r"
    branch b
    checkout b
    commit id: "B" tag: "fix/sniff-pr"
    branch c
    checkout c
    commit id: "c1"
    checkout b
    checkout main
    commit id: "P"
    merge b id: "C"
    checkout b
    commit id: "N"
    checkout main"#,
    ),
    (
        "5-merge-into-sibling",
        r#"gitGraph
    commit id: "r"
    branch s
    checkout s
    commit id: "s1"
    branch a
    checkout a
    commit id: "A"
    checkout s
    commit id: "s2"
    merge a id: "S3"
    checkout a
    commit id: "a2"
    checkout s
    commit id: "s4"
    checkout main
    commit id: "P"
    merge s id: "C"
    checkout main"#,
    ),
];

fn main() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, text) in CASES {
        println!("== {name}");
        let diagram = MermaidDiagram::new(*text);
        match diagram.gitgraph_geometry() {
            Ok(Some(geometry)) => {
                for commit in &geometry.commits {
                    let tags: Vec<&str> = commit.tags.iter().map(|tag| tag.text.as_str()).collect();
                    println!("  #{:<2} {:<3} {:<4} parents={:?} tags={:?}", commit.index, commit.lane, commit.id, commit.parents, tags);
                }
            }
            Ok(None) => println!("  no gitGraph geometry"),
            Err(error) => println!("  ERROR: {error}"),
        }
        if name.starts_with('1') || name.starts_with('3') || name.starts_with('5') {
            let request = RenderRequest {
                format: OutputFormat::Svg,
                ..RenderRequest::default()
            };
            match diagram.render(&request) {
                Ok(artifact) => {
                    let target = here.join(format!("{name}.svg"));
                    std::fs::copy(&artifact.path, &target).expect("copy svg");
                    println!("  svg: {}", target.display());
                }
                Err(error) => println!("  RENDER ERROR: {error}"),
            }
        }
    }
}
