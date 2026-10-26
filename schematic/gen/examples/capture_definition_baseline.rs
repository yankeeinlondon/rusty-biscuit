//! Explicit one-shot capture tool; migration-check and tests never write fixtures.
#[path = "support/definition_snapshot.rs"]
mod definition_snapshot;

fn main() {
    let output = std::env::args_os().nth(1).expect("destination directory required");
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir(&output).expect("destination must be a new directory with an existing parent");
    let mut total = 0;
    for api in schematic_gen::pipeline::resolve_all_apis() {
        let value = definition_snapshot::canonical(definition_snapshot::snapshot(&api));
        let text = serde_json::to_string_pretty(&value).unwrap() + "\n";
        total += text.len();
        let path = output.join(format!("{}.json",
            schematic_definitions::registry::registry_key_for(&api.name)));
        std::fs::write(&path, text).unwrap();
        let restored = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let mut changed = Vec::new();
        definition_snapshot::differences(&value, &restored, &api.name, &mut changed);
        assert!(changed.is_empty(), "capture changed during persistence: {changed:?}");
    }
    eprintln!("Captured {total} bytes");
}
