//! Private-seam tests: prepared-state reuse and, with `file`, the
//! context-capture boundary (criterion 15). Both observe state no public
//! API exposes: the identity of the prepared validators and the
//! thread-local capture and read counters.

use serde_json::json;

use super::SimplifiedSchema;

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).expect("schema parses")
}

mod prepared_state {
    use super::*;

    #[test]
    fn validation_prepares_once_and_reuses_it() {
        let person = schema("$schema:\n  name: string(required)\n");
        assert!(person.prepared_identity().is_none(), "building compiles nothing");
        person.validate_strict(&json!({ "name": "a" })).unwrap();
        let first = person.prepared_identity().expect("prepared on first use");
        person.validate_soft(&json!({ "name": 1 })).unwrap();
        person.validate_strict(&json!({})).unwrap_err();
        assert_eq!(person.prepared_identity(), Some(first));
    }

    #[test]
    fn a_failed_layering_keeps_the_receivers_definition_and_prepared_state() {
        let valid = schema("$schema:\n  extra: number\n");
        let union = schema("$schema:\n  - kind: string\n");
        let mut receiver = schema("$schema:\n  name: string\n");
        receiver.validate_strict(&json!({})).unwrap();
        let (definition, prepared) = (std::sync::Arc::as_ptr(&receiver.definition), receiver.prepared_identity());
        assert!(receiver.layer_under(&[&valid, &union]).is_err());
        assert_eq!(std::sync::Arc::as_ptr(&receiver.definition), definition);
        assert_eq!(receiver.prepared_identity(), prepared);
        assert_eq!(receiver.revision(), 0);
    }

    #[test]
    fn a_successful_change_drops_the_prepared_state_and_an_unchanged_one_keeps_it() {
        let mut receiver = schema("$schema:\n  name: string\n");
        receiver.validate_strict(&json!({})).unwrap();
        let prepared = receiver.prepared_identity();
        receiver.layer_under(&[&schema("$schema:\n  name: number\n")]).unwrap();
        assert_eq!(receiver.prepared_identity(), prepared, "nothing incoming survived");
        receiver.layer_under(&[&schema("$schema:\n  count: number\n")]).unwrap();
        assert!(receiver.prepared_identity().is_none(), "the next validation prepares the new revision");
        receiver.validate_strict(&json!({})).unwrap();
        assert!(receiver.prepared_identity().is_some());
    }

    #[test]
    fn a_clone_shares_the_prepared_state() {
        let person = schema("$schema:\n  name: string\n");
        person.validate_strict(&json!({})).unwrap();
        assert_eq!(person.clone().prepared_identity(), person.prepared_identity());
    }
}

#[cfg(feature = "file")]
mod context_boundary {
    use std::path::{Path, PathBuf};

    use biscuit_file::FileResolutionContext;
    use serial_test::serial;

    use super::super::{SchemaSource, capture, source};
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// Runs `body` with the process in `dir`, restoring the directory after.
    fn in_dir<T>(dir: &Path, body: impl FnOnce() -> T) -> T {
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        let out = body();
        std::env::set_current_dir(previous).unwrap();
        out
    }

    #[test]
    #[serial]
    fn a_raw_path_captures_once_and_nothing_downstream_captures_again() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "people.yaml", "$schema:\n  Person:\n    name: string\n");
        write(dir.path(), "notes.md", "n\n");
        let main = write(dir.path(), "main.yaml", "$schema:\n  owner: Person@./people.yaml\n  notes: file(eager)\n");
        let (captures, reads) = (capture::captures(), source::reads());
        let schema = in_dir(dir.path(), || SimplifiedSchema::try_from_yaml(main.as_path())).unwrap();
        assert_eq!(capture::captures() - captures, 1, "one capture at factory entry, none for the import");
        assert_eq!(source::reads() - reads, 1, "the input file is read once");
        schema.validate_strict(&json!({ "owner": { "name": "a" }, "notes": "./notes.md" })).unwrap();
        schema.validate_soft(&json!({ "owner": { "name": "a" }, "notes": "./notes.md" })).unwrap();
        let _ = (schema.query("owner.name"), schema.to_simplified_yaml(), schema.to_json_schema(), schema.properties());
        let mut layered = schema.clone();
        layered.layer_under(&[&SimplifiedSchema::try_from("$schema:\n  extra: number\n").unwrap()]).unwrap();
        layered.validate_strict(&json!({ "notes": "./notes.md" })).unwrap();
        assert_eq!(capture::captures() - captures, 1, "validation, queries, export, and layering capture nothing");
    }

    #[test]
    fn a_bound_source_and_content_capture_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let main = write(dir.path(), "main.md", "---\n$schema:\n  a: string\n---\n");
        let captures = capture::captures();
        let schema =
            SimplifiedSchema::try_from_markdown(SchemaSource::from(&main).bound(FileResolutionContext::new(dir.path())))
                .unwrap();
        schema.validate_strict(&json!({ "a": "x" })).unwrap();
        let content = SimplifiedSchema::try_from("$schema:\n  a: string\n").unwrap();
        content.validate_soft(&json!({ "a": 1 })).unwrap();
        let _ = (content.to_simplified_yaml(), content.query("a"));
        assert_eq!(capture::captures(), captures);
    }

    #[test]
    fn supplied_text_is_never_read_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        // The file on disk disagrees with the buffer: the buffer wins.
        let path = write(dir.path(), "draft.yaml", "$schema:\n  a: number\n");
        let reads = source::reads();
        let source = SchemaSource::from(&path)
            .bound(FileResolutionContext::new(dir.path()))
            .with_text("$schema:\n  a: string\n");
        let schema = SimplifiedSchema::try_from_yaml(source).unwrap();
        assert_eq!(source::reads(), reads);
        assert_eq!(schema.property("a").unwrap().type_text(), "string");
    }

    /// The directory, home, and environment are captured at entry; changing
    /// them afterwards does not move where the schema resolves.
    #[test]
    #[serial]
    fn process_changes_after_capture_do_not_retarget() {
        let schemas = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        write(home.path(), "home-note.md", "h\n");
        write(schemas.path(), "local-note.md", "l\n");
        write(schemas.path(), "main.yaml", "$schema:\n  doc: file(eager)\n");
        let home_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let previous_home = std::env::var_os(home_var);
        // SAFETY: the test is `#[serial]`, and nextest runs it in its own
        // process; no other thread reads the environment meanwhile.
        unsafe { std::env::set_var(home_var, home.path()) };
        let schema = in_dir(schemas.path(), || SimplifiedSchema::try_from_yaml(Path::new("main.yaml"))).unwrap();
        // Move the process and its home after the capture.
        unsafe { std::env::set_var(home_var, elsewhere.path()) };
        let accepted = in_dir(elsewhere.path(), || {
            (
                schema.validate_strict(&json!({ "doc": "~/home-note.md" })).is_ok(),
                schema.validate_strict(&json!({ "doc": "./local-note.md" })).is_ok(),
                schema.validate_strict(&json!({ "doc": "./absent.md" })).is_ok(),
            )
        });
        match previous_home {
            Some(value) => unsafe { std::env::set_var(home_var, value) },
            None => unsafe { std::env::remove_var(home_var) },
        }
        assert_eq!(accepted, (true, true, false));
    }

    /// Parsing, querying, exporting, and attempting to lower or validate a
    /// template evaluates no slot and reads nothing but schema files: the
    /// slots name data files that do not exist, which any read would fail on.
    #[test]
    fn a_template_reads_and_evaluates_nothing_but_its_schema_files() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  Kind: enum(a, {{ load_data('./missing-kinds.yaml') }})\n");
        let main = write(
            dir.path(),
            "main.yaml",
            "$schema:\n  os: enum(none, {{ load_data('./missing-os.yaml', 'string[]') }}; required)\n  \
             kind: Kind@./types.yaml\n  tries: number(max({{ env.MAX }}))\n",
        );
        let reads = source::reads();
        #[cfg(feature = "work-counters")]
        let parsed = crate::work_counters::snapshot();
        let schema = SimplifiedSchema::try_from_yaml(SchemaSource::from(&main).bound(FileResolutionContext::new(dir.path())))
            .expect("a template is admitted");
        assert_eq!(source::reads() - reads, 1, "the input file is read once; slots name no schema file");
        assert_eq!(schema.pending_slots().len(), 3);
        #[cfg(feature = "work-counters")]
        {
            use crate::work_counters::{Work, snapshot};
            // Parsing a slot parses its expression (to locate a syntax
            // error); nothing after that touches one again.
            assert!(snapshot().since(&parsed).get(Work::ExpressionParse) >= 3);
            let before = snapshot();
            let _ = (
                schema.query("os"),
                schema.property("kind").map(|kind| kind.pending_slots()),
                schema.properties(),
                schema.to_simplified_yaml().expect("exports"),
                schema.to_json_schema().expect_err("unresolved"),
                schema.validate_strict(&json!({ "os": "none" })).expect_err("unresolved"),
                schema.validate_soft(&json!({ "os": "none" })).expect_err("unresolved"),
            );
            let work = snapshot().since(&before);
            assert_eq!(work.get(Work::ExpressionParse), 0, "no slot is parsed again, let alone evaluated");
            assert_eq!(work.get(Work::ValidatorCompile), 0, "no validator is compiled for a template");
        }
        assert_eq!(source::reads() - reads, 1);
        let provenance = schema.provenance();
        for listed in [provenance.referenced_files(), provenance.searched(), provenance.imports()] {
            assert!(
                listed.iter().all(|path| !path.to_string_lossy().contains("missing-")),
                "a slot's data file is never a dependency of the template: {listed:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_path_with_a_drive_resolves() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "main.yaml", "$schema:\n  a: string\n");
        assert!(path.to_str().unwrap().contains(":\\"));
        let schema =
            SimplifiedSchema::try_from_yaml(SchemaSource::from(path.to_str().unwrap()).bound(FileResolutionContext::new(dir.path())))
                .unwrap();
        assert_eq!(schema.provenance().path().map(Path::to_path_buf), Some(path));
    }
}
