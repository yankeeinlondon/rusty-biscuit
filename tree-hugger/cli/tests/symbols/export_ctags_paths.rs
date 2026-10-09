//! Tag paths always lead back to the declaration file. A route from the scan
//! root through a name that is not valid Unicode (Linux bytes, a Windows
//! unpaired surrogate) cannot be written to the UTF-8 tag file without
//! naming another file, so its tags are skipped with a warning and exit 1,
//! while every other file's tags are written unchanged. A scan root whose
//! own spelling is not valid Unicode, and a name that really contains
//! U+FFFD, are ordinary.
//!
//! Linux cases are `cfg(target_os = "linux")` because APFS rejects names
//! that are not UTF-8; Windows cases build names with `from_wide`. `hug`'s
//! file arguments are UTF-8 strings, so the outside-root case reaches its
//! file through a validly named link, as an editor-supplied path would.

use std::path::Path;
use std::process::Output;

use crate::Scratch;

const JAVA: &str = include_str!("../../../lib/tests/fixtures/symbols/multi/java.java");
const SAFE: &str = "public class Safe {\n    public void keep() {}\n    public static int total;\n}\n";
const HEADER: &str = "!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t0\t/0=unsorted/\n";

/// A tag without its path: name, line and the fields after the address.
type Entry = (&'static str, usize, &'static str);

/// One declaration command and what it selects from the Java fixture and
/// from [`SAFE`], in file order.
struct Case {
    args: &'static [&'static str],
    java: &'static [Entry],
    safe: &'static [Entry],
}

const SAFE_CLASS: Entry = ("Safe", 1, "kind:class");
const SAFE_KEEP: Entry = ("keep", 2, "kind:method\tclass:Safe");
const SAFE_TOTAL: Entry = ("total", 3, "kind:field\tclass:Safe");
const COUNTER: Entry = ("Counter", 3, "kind:class");
const COUNT: Entry = ("count", 4, "kind:field\tclass:Counter");
const CONSTRUCTOR: Entry = ("Counter", 6, "kind:constructor\tclass:Counter");
const START: Entry = ("start", 6, "kind:parameter\tclass:Counter.Counter");
const INCREMENT: Entry = ("increment", 10, "kind:method\tclass:Counter");
const ZERO: Entry = ("zero", 14, "kind:method\tclass:Counter");
const RESETTABLE: Entry = ("Resettable", 19, "kind:interface");
const RESET: Entry = ("reset", 20, "kind:method\tclass:Resettable");

const CASES: &[Case] = &[
    Case { args: &["functions"], java: &[CONSTRUCTOR, INCREMENT, ZERO, RESET], safe: &[SAFE_KEEP] },
    Case { args: &["types"], java: &[COUNTER, RESETTABLE], safe: &[SAFE_CLASS] },
    Case {
        args: &["symbols"],
        java: &[COUNTER, COUNT, CONSTRUCTOR, INCREMENT, ZERO, RESETTABLE, RESET],
        safe: &[SAFE_CLASS, SAFE_KEEP, SAFE_TOTAL],
    },
    Case {
        args: &["symbols", "--all-scopes"],
        java: &[COUNTER, COUNT, CONSTRUCTOR, START, INCREMENT, ZERO, RESETTABLE, RESET],
        safe: &[SAFE_CLASS, SAFE_KEEP, SAFE_TOTAL],
    },
    Case {
        args: &["classes"],
        java: &[COUNTER, CONSTRUCTOR, INCREMENT, ZERO, RESETTABLE, RESET],
        safe: &[SAFE_CLASS, SAFE_KEEP],
    },
    Case { args: &["classes", "--static-only"], java: &[COUNTER, ZERO, RESETTABLE], safe: &[SAFE_CLASS] },
    Case {
        args: &["classes", "--instance-only"],
        java: &[COUNTER, CONSTRUCTOR, INCREMENT, RESETTABLE, RESET],
        safe: &[SAFE_CLASS, SAFE_KEEP],
    },
];

/// `entries` as tag lines at `path`, with `Safe` renamed to `class`.
fn tags(entries: &[Entry], path: &str, class: &str) -> String {
    entries
        .iter()
        .map(|(name, line, fields)| {
            let line = format!("{name}\t{path}\t{line};\"\t{fields}\n");
            line.replace("Safe", class)
        })
        .collect()
}

/// Runs `case` as a tags export from `dir` with `extra` arguments.
fn run(scratch: &Scratch, dir: &Path, case: &Case, extra: &[&str]) -> Output {
    let mut args = case.args.to_vec();
    args.extend(["--format", "ctags"]);
    args.extend(extra);
    scratch.process(&args).current_dir(dir).output().expect("run hug")
}

/// Every tag's path, joined onto the scan root, is the file `declarations`
/// says declares that tag's name: a tag never leads to another file.
fn assert_tags_navigate(stdout: &str, root: &Path, declarations: &[(&str, &Path)]) {
    for line in stdout.lines().skip(2) {
        let fields: Vec<&str> = line.split('\t').collect();
        let target = std::fs::canonicalize(root.join(fields[1])).unwrap_or_else(|error| {
            panic!("tag path {:?} leads nowhere ({error}): {line}", fields[1])
        });
        let declared = declarations
            .iter()
            .find(|(name, _)| *name == fields[0])
            .unwrap_or_else(|| panic!("unexpected tag {line}"));
        assert_eq!(target, std::fs::canonicalize(declared.1).unwrap(), "{line}");
    }
}

/// Every Java entry is skipped with a warning naming `shown`, the companion
/// tags (`companions`: their path and class name) are written exactly, and
/// the run exits 1.
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
fn assert_rejected(output: Output, case: &Case, shown: &str, companions: &[(&str, &str)]) -> String {
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 tags");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 warnings");
    let expected_tags: String = companions.iter().map(|(path, class)| tags(case.safe, path, class)).collect();
    assert_eq!(stdout, format!("{HEADER}{expected_tags}"), "{:?}\n{stderr}", case.args);
    let warnings: String = case
        .java
        .iter()
        .map(|(_, line, _)| {
            format!("warning: ctags skipped file name that is not valid Unicode in {shown} at line {line}\n")
        })
        .collect();
    assert_eq!(stderr, warnings, "{:?}", case.args);
    assert_eq!(output.status.code(), Some(1), "{:?}", case.args);
    stdout
}

/// The Java entries at `java_path` and the [`SAFE`] entries at `safe_path`,
/// in the order given by `java_first`, are written exactly, with no warning.
fn assert_accepted(output: Output, case: &Case, java_path: &str, safe_path: &str, java_first: bool) -> String {
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 tags");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (java, safe) = (tags(case.java, java_path, "Safe"), tags(case.safe, safe_path, "Safe"));
    let body = if java_first { format!("{java}{safe}") } else { format!("{safe}{java}") };
    assert_eq!(stdout, format!("{HEADER}{body}"), "{:?}\n{stderr}", case.args);
    assert!(stderr.is_empty(), "{:?}: {stderr}", case.args);
    assert_eq!(output.status.code(), Some(0), "{:?}", case.args);
    stdout
}

/// The declarations each name is tagged at, for [`assert_tags_navigate`].
fn declared_in<'a>(java: &'a Path, safe: &'a Path) -> Vec<(&'static str, &'a Path)> {
    let java_names = ["Counter", "count", "start", "increment", "zero", "Resettable", "reset"];
    let safe_names = ["Safe", "keep", "total"];
    java_names.map(|name| (name, java)).into_iter().chain(safe_names.map(|name| (name, safe))).collect()
}

/// R2-F1.C2: a file really named with U+FFFD is an ordinary Unicode name.
#[test]
fn a_real_replacement_character_file_name_is_tagged() {
    let name = "bad\u{FFFD}.java";
    let scratch = Scratch::new(&[(name, JAVA), ("Safe.java", SAFE)]);
    let root = scratch.repo.path();
    for case in CASES {
        let stdout = assert_accepted(run(&scratch, root, case, &[]), case, name, "Safe.java", false);
        assert_tags_navigate(&stdout, root, &declared_in(&root.join(name), &root.join("Safe.java")));
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    use super::*;

    const SHOWN: &str = "bad\u{FFFD}.java";

    fn invalid(suffix: &str) -> PathBuf {
        let mut bytes = b"bad\xff".to_vec();
        bytes.extend_from_slice(suffix.as_bytes());
        PathBuf::from(OsString::from_vec(bytes))
    }

    /// Writes the Java fixture at `relative` below the scratch repository.
    fn write_java(scratch: &Scratch, relative: &Path) -> PathBuf {
        let path = scratch.repo.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, JAVA).unwrap();
        path
    }

    /// R2-F1.P1, D2–D8: the declaration file's own name holds byte `FF`.
    #[test]
    fn a_file_name_that_is_not_utf8_is_skipped_and_other_files_are_kept() {
        let scratch = Scratch::new(&[("Safe.java", SAFE)]);
        let java = write_java(&scratch, &invalid(".java"));
        let root = scratch.repo.path();
        for case in CASES {
            let stdout = assert_rejected(run(&scratch, root, case, &[]), case, SHOWN, &[("Safe.java", "Safe")]);
            assert_tags_navigate(&stdout, root, &declared_in(&java, &root.join("Safe.java")));
        }
    }

    /// R2-F1.P1, D8: with only the rejected file the report is the header.
    #[test]
    fn a_report_whose_every_entry_is_unrepresentable_is_header_only() {
        let scratch = Scratch::new(&[]);
        write_java(&scratch, &invalid(".java"));
        for case in CASES {
            assert_rejected(run(&scratch, scratch.repo.path(), case, &[]), case, SHOWN, &[]);
        }
    }

    /// R2-F1.P2: the parent directory holds byte `FF`.
    #[test]
    fn a_directory_name_that_is_not_utf8_is_skipped() {
        let scratch = Scratch::new(&[("Safe.java", SAFE)]);
        write_java(&scratch, &invalid("").join("Example.java"));
        for case in CASES {
            let shown = "bad\u{FFFD}/Example.java";
            assert_rejected(run(&scratch, scratch.repo.path(), case, &[]), case, shown, &[("Safe.java", "Safe")]);
        }
    }

    /// R2-F1.P3: the declaration lies outside the scan root (`sub/`) and its
    /// name holds byte `FF`; the route `../bad\xff.java` is validated.
    #[test]
    fn an_outside_root_route_that_is_not_utf8_is_skipped() {
        let scratch = Scratch::new(&[("sub/Safe.java", SAFE)]);
        let target = write_java(&scratch, &invalid(".java"));
        let sub = scratch.repo.path().join("sub");
        std::os::unix::fs::symlink(&target, sub.join("link.java")).unwrap();
        for case in CASES {
            let output = run(&scratch, &sub, case, &["--file", "link.java", "--file", "Safe.java"]);
            assert_rejected(output, case, "../bad\u{FFFD}.java", &[("Safe.java", "Safe")]);
        }
    }

    /// R2-F1.P4, D9: the replacement spelling names an existing file with
    /// other declarations. Its own tags are written; the rejected file's
    /// entries never name it.
    #[test]
    fn an_existing_replacement_spelling_never_receives_another_files_tags() {
        let decoy = SAFE.replace("Safe", "Decoy");
        let scratch = Scratch::new(&[("Safe.java", SAFE), (SHOWN, &decoy)]);
        write_java(&scratch, &invalid(".java"));
        let root = scratch.repo.path();
        let decoy_path = root.join(SHOWN);
        for case in CASES {
            let output = run(&scratch, root, case, &[]);
            let stdout = assert_rejected(output, case, SHOWN, &[("Safe.java", "Safe"), (SHOWN, "Decoy")]);
            for line in stdout.lines().skip(2) {
                let fields: Vec<&str> = line.split('\t').collect();
                let target = std::fs::canonicalize(root.join(fields[1])).unwrap();
                let owner = if line.contains("Decoy") { decoy_path.clone() } else { root.join("Safe.java") };
                assert_eq!(target, std::fs::canonicalize(owner).unwrap(), "{line}");
                for (java_name, _, _) in case.java {
                    assert!(
                        !(fields[0] == *java_name && fields[1] == SHOWN),
                        "the decoy received {java_name}: {line}"
                    );
                }
            }
        }
    }

    /// R2-F1.P5: a validly named link resolves to a declaration file whose
    /// name holds byte `FF`; the recorded (resolved) route is validated.
    #[test]
    fn a_link_to_a_file_name_that_is_not_utf8_is_skipped() {
        let scratch = Scratch::new(&[("Safe.java", SAFE)]);
        let target = write_java(&scratch, &invalid(".java"));
        let root = scratch.repo.path();
        std::os::unix::fs::symlink(&target, root.join("alias.java")).unwrap();
        for case in CASES {
            let output = run(&scratch, root, case, &["--file", "alias.java", "--file", "Safe.java"]);
            assert_rejected(output, case, SHOWN, &[("Safe.java", "Safe")]);
        }
    }

    /// R2-F1.C1, D9: only the scan root's own name holds byte `FF`; the
    /// emitted routes are valid and navigate to the declarations.
    #[test]
    fn a_scan_root_that_is_not_utf8_keeps_its_valid_routes() {
        let scratch = Scratch::new(&[]);
        let java = write_java(&scratch, &invalid("").join("Example.java"));
        let root = java.parent().unwrap().to_path_buf();
        std::fs::write(root.join("Safe.java"), SAFE).unwrap();
        for case in CASES {
            let stdout = assert_accepted(run(&scratch, &root, case, &[]), case, "Example.java", "Safe.java", true);
            assert_tags_navigate(&stdout, &root, &declared_in(&java, &root.join("Safe.java")));
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    use super::*;

    /// `bad` followed by an unpaired high surrogate and `suffix`.
    fn invalid(suffix: &str) -> PathBuf {
        let mut wide: Vec<u16> = "bad".encode_utf16().collect();
        wide.push(0xD800);
        wide.extend(suffix.encode_utf16());
        PathBuf::from(OsString::from_wide(&wide))
    }

    fn write_java(scratch: &Scratch, relative: &Path) -> PathBuf {
        let path = scratch.repo.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, JAVA).unwrap();
        path
    }

    /// R2-F1.P6, D2–D8: the file name holds an unpaired surrogate.
    #[test]
    fn a_file_name_with_an_unpaired_surrogate_is_skipped_and_other_files_are_kept() {
        let scratch = Scratch::new(&[("Safe.java", SAFE)]);
        let java = write_java(&scratch, &invalid(".java"));
        let root = scratch.repo.path();
        for case in CASES {
            let stdout =
                assert_rejected(run(&scratch, root, case, &[]), case, "bad\u{FFFD}.java", &[("Safe.java", "Safe")]);
            assert_tags_navigate(&stdout, root, &declared_in(&java, &root.join("Safe.java")));
        }
        let only = Scratch::new(&[]);
        write_java(&only, &invalid(".java"));
        for case in CASES {
            assert_rejected(run(&only, only.repo.path(), case, &[]), case, "bad\u{FFFD}.java", &[]);
        }
    }

    /// R2-F1.P7: the parent directory holds an unpaired surrogate.
    #[test]
    fn a_directory_name_with_an_unpaired_surrogate_is_skipped() {
        let scratch = Scratch::new(&[("Safe.java", SAFE)]);
        write_java(&scratch, &invalid("").join("Example.java"));
        for case in CASES {
            let shown = "bad\u{FFFD}/Example.java";
            assert_rejected(run(&scratch, scratch.repo.path(), case, &[]), case, shown, &[("Safe.java", "Safe")]);
        }
    }

    /// R2-F1.C7, D9: only the scan root holds an unpaired surrogate.
    #[test]
    fn a_scan_root_with_an_unpaired_surrogate_keeps_its_valid_routes() {
        let scratch = Scratch::new(&[]);
        let java = write_java(&scratch, &invalid("").join("Example.java"));
        let root = java.parent().unwrap().to_path_buf();
        std::fs::write(root.join("Safe.java"), SAFE).unwrap();
        for case in CASES {
            let stdout = assert_accepted(run(&scratch, &root, case, &[]), case, "Example.java", "Safe.java", true);
            assert_tags_navigate(&stdout, &root, &declared_in(&java, &root.join("Safe.java")));
        }
    }
}
