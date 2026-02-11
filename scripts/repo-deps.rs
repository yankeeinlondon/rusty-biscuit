use cargo_metadata::{MetadataCommand, PackageId};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    let meta = MetadataCommand::new().exec()?;
    let resolve = meta.resolve.as_ref().expect("missing resolve graph");

    let ws: BTreeSet<&PackageId> = meta.workspace_members.iter().collect();
    let name: BTreeMap<&PackageId, &str> = meta
        .packages
        .iter()
        .map(|p| (&p.id, p.name.as_str()))
        .collect();

    let mut lines = Vec::new();

    for node in &resolve.nodes {
        if !ws.contains(&node.id) {
            continue;
        }
        let from = name[&node.id];

        let mut tos: Vec<&str> = node
            .deps
            .iter()
            .map(|d| &d.pkg)
            .filter(|id| ws.contains(id))
            .map(|id| name[id])
            .collect();

        tos.sort();
        tos.dedup();

        lines.push(if tos.is_empty() {
            format!("{from}: (none)")
        } else {
            format!("{from}: {}", tos.join(", "))
        });
    }

    lines.sort();
    for l in lines {
        println!("{l}");
    }

    Ok(())
}
