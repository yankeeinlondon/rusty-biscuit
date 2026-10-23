//! The shipped REST corpus uses the supported typed authoring surface.

#[test]
fn provider_authoring_uses_builders_and_typed_registrations() {
    let root = biscuit_test_harness::manifest_dir!().join("src");
    let mut pending = vec![root];
    let mut scanned = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if path.is_dir() {
                if name != "registry" && name != "types" && !name.ends_with("_ws") { pending.push(path); }
                continue;
            }
            if !["mod.rs", "branches.rs", "issues.rs", "merge_requests.rs", "pipelines.rs", "projects.rs", "releases.rs"].contains(&name.as_ref()) { continue; }
            let source = std::fs::read_to_string(&path).unwrap();
            let authored = source.split("#[cfg(test)]").next().unwrap();
            let authored = authored.split("pub fn define_elevenlabs_websocket_api").next().unwrap();
            for line in authored.lines().filter(|line| !line.trim_start().starts_with("//")) {
                for forbidden in ["SuccessShape::", "ApiRequest::json", "Schema::new(", "Schema::with_path(", "json_type("] {
                    assert!(!line.contains(forbidden), "{}: {line}", path.display());
                }
                for literal in ["Endpoint {", "RestApi {"] {
                    assert!(!line.contains(literal) || line.contains("->"), "{}: {line}", path.display());
                }
                assert!(!(line.contains(".register::<") && line.contains(">(\"")), "{}: {line}", path.display());
            }
            scanned += 1;
        }
    }
    assert!(scanned >= 20, "the shipped REST corpus must be scanned");
}
