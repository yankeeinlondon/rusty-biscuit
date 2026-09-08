use std::fs;
use std::process::Command;

#[test]
fn generation_stops_before_writes_when_steering_gate_fails() {
    let area = tempfile::tempdir().expect("temporary area");
    let research = area.path().join("docs/research/steering");
    fs::create_dir_all(&research).expect("steering directory");
    fs::write(
        area.path().join("docs/providers.yaml"),
        "list:\n  - slug: codex\n",
    )
    .expect("roster");
    fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/research/steering/_schema.yaml"),
        research.join("_schema.yaml"),
    )
    .expect("schema");
    fs::write(
        research.join("codex.md"),
        "---\n$schema: ./_schema.yaml\nschema_revision: 2\n---\n",
    )
    .expect("invalid research document");

    let output = Command::new(env!("CARGO_BIN_EXE_claudine-gen"))
        .args(["--area", area.path().to_str().expect("UTF-8 path"), "generate", "--yes"])
        .output()
        .expect("run claudine-gen");

    assert!(!output.status.success());
    assert!(!area.path().join("docs/providers/catalog.json").exists());
    assert!(!area.path().join("lib/src/provider/codex/data.rs").exists());
}
