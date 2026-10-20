//! Template-mode shell execution through `md compose`.
//!
//! Each placeholder fills exactly one argument of a native program, every
//! value is checked and every filled command is checked against the
//! blacklists before anything spawns, and a refusal never shows the value.
//!
//! The program is the `darkmatter-argv-recorder` fixture copied into the
//! fixture `bin` as `argv-recorder`, so the evidence is the argument vector the
//! program itself received, on every platform.

use crate::common::{CliProcessFixture, unwrapped};
use darkmatter::markdown::compose::shell_expansion::normalize_command;
use darkmatter::markdown::compose::shell_expansion::template::{TemplateShape, parse_authored};
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

/// A fixture with the recorder on its `PATH` and a file it records into.
struct Recorder {
    fixture: CliProcessFixture,
    records: PathBuf,
}

impl Recorder {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let program = fixture
            .bin_dir()
            .join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"), &program)
            .expect("the recorder fixture binary is built with `test-fixtures`");
        let records = fixture.tmp_dir().join("argv.bin");
        Self { fixture, records }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        self.fixture.write_file(format!("cwd/{name}"), content)
    }

    fn whitelist(&self, rules: &str) {
        self.write(".darkmatter-shell-whitelist", rules);
    }

    fn blacklist(&self, rules: &str) {
        self.write(".darkmatter-shell-blacklist", rules);
    }

    /// `md compose doc.md` with `args` after the document.
    fn compose(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .arg("compose")
            .arg(self.fixture.cwd().join("doc.md"))
            .args(args)
            .assert()
    }

    /// Every argument vector the recorder received, in order; empty when it
    /// never ran.
    fn runs(&self) -> Vec<Vec<String>> {
        match read_argv_records(&self.records) {
            Ok(runs) => runs,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("unreadable recorder file: {error}"),
        }
    }
}

/// The `template` rule that approves `command`'s shape, every hole a broad
/// `string`.
fn template_rule(command: &str) -> String {
    let template = parse_authored(command).expect("a valid template");
    format!("template {}\n", TemplateShape::from_template(&template).canonical_text())
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    unwrapped(&String::from_utf8_lossy(&assert.get_output().stderr))
}

/// Whether `haystack` holds `needle`, ignoring where the error block wrapped
/// its lines.
fn mentions(haystack: &str, needle: &str) -> bool {
    let squeeze = |text: &str| text.split_whitespace().collect::<String>();
    squeeze(haystack).contains(&squeeze(needle))
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// Values a shell would read as syntax, an empty value, and two placeholders
/// in one word each reach the program as exactly the argument their word
/// forms. A line break a value inserts stays inside its argument and inside
/// the directive.
#[test]
fn each_placeholder_fills_exactly_one_argument_of_its_word() {
    let recorder = Recorder::new("template_one_argument_per_word");
    let command = r#"argv-recorder {{ spaced }} {{ semi }} {{ subst }} {{ tick }} {{ empty }} {{ spaced }}{{ semi }} "lit {{ quoted }}" {{ slash }} {{ uni }} {{ nl }} {{ pct }} {{ bang }} '%' "!""#;
    recorder.write(
        "doc.md",
        &format!(
            "{}::shell {command}\nafter\n",
            r#"---
spaced: "a b"
semi: "x; rm -rf ~"
subst: "{{ '$(' + 'id)' }}"
tick: "`id`"
empty: ""
quoted: "say \"hi\" 'there'"
slash: "back\\slash\\"
uni: "naïve ☃"
nl: "line1\nline2"
pct: "100%"
bang: "wow!"
---
before
"#
        ),
    );
    recorder.whitelist(&template_rule(command));

    let assert = recorder.compose(&[]).success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();

    assert_eq!(
        recorder.runs(),
        vec![strings(&[
            "a b",
            "x; rm -rf ~",
            "$(id)",
            "`id`",
            "",
            "a bx; rm -rf ~",
            "lit say \"hi\" 'there'",
            "back\\slash\\",
            "naïve ☃",
            "line1\nline2",
            "100%",
            "wow!",
            "%",
            "!",
        ])]
    );
    assert!(stdout.contains("before") && stdout.contains("after"), "{stdout}");
    assert!(!stdout.contains("line2"), "the directive's whole extent is replaced: {stdout}");
}

/// A container, a NUL byte, or a missing value refuses the command before it
/// spawns, with a typed error that names the placeholder but never the value.
/// The values come from setters, so the document text holds none of them.
#[test]
fn a_refused_value_runs_nothing_and_never_appears_in_the_error() {
    let cases = [
        (r#"v=["sekrit-array"]"#, Some("sekrit-array"), "is an array or object"),
        (r#"v={"sekrit-key":1}"#, Some("sekrit-key"), "is an array or object"),
        (r#"v="sekrit\u0000nul""#, Some("sekrit"), "contains a NUL byte"),
        ("v=null", None, "reads 'v', which is missing"),
    ];
    for (setter, marker, expected) in cases {
        let recorder = Recorder::new("template_refused_value");
        recorder.write("doc.md", "---\ntitle: refusals\n---\n::shell argv-recorder {{ v }}\n");
        recorder.whitelist(&template_rule("argv-recorder {{ v }}"));

        let assert = recorder.compose(&[setter]).failure();
        let stderr = stderr_of(&assert);

        assert!(recorder.runs().is_empty(), "{setter}: nothing may run");
        assert!(mentions(&stderr, expected), "{setter}: {stderr}");
        if let Some(marker) = marker {
            assert!(!mentions(&stderr, marker), "{setter}: the value leaked: {stderr}");
        }
    }
}

/// A broad `string` template cannot authorize a filled command the built-in
/// blacklist refuses. The second value keeps the line from parsing as bytes,
/// so only the runtime check of the filled command can catch it.
#[test]
fn a_string_template_cannot_authorize_a_builtin_blacklisted_fill() {
    let recorder = Recorder::new("template_builtin_blacklist");
    recorder.write("doc.md", "::shell git {{ sub }} {{ junk }}\n");
    recorder.whitelist(&template_rule("git {{ sub }} {{ junk }}"));

    let assert = recorder.compose(&["sub=reset", "junk=a;b"]).failure();

    let stderr = stderr_of(&assert);
    assert!(mentions(&stderr, "Blocked command"), "{stderr}");
    assert!(mentions(&stderr, "git {{ sub }} {{ junk }}"), "{stderr}");
}

/// User blacklists judge the filled command: an exact rule blocks the fill
/// it names, and a blocked later segment keeps the first from starting.
#[test]
fn a_user_blacklist_blocks_a_filled_chain_before_its_first_segment() {
    let recorder = Recorder::new("template_user_blacklist_chain");
    let command = "argv-recorder first && argv-recorder {{ v }} {{ junk }}";
    recorder.write("doc.md", &format!("::shell {command}\n"));
    recorder.whitelist(&template_rule(command));
    let filled = normalize_command("argv-recorder", &strings(&["reset", "a;b"]));
    recorder.blacklist(&format!("exact {filled}\n"));

    let assert = recorder.compose(&["v=reset", "junk=a;b"]).failure();

    assert!(mentions(&stderr_of(&assert), "user blacklist"), "{}", stderr_of(&assert));
    assert!(recorder.runs().is_empty(), "no segment of a blocked chain starts");
}

/// A shape-only `template` blacklist blocks commands without authored holes
/// too: a single command by its words, and a resolved-byte chain at run time.
#[test]
fn a_template_blacklist_blocks_resolved_byte_commands_and_chains() {
    let recorder = Recorder::new("template_blacklist_resolved");
    recorder.write("doc.md", "::shell argv-recorder deploy prod\n");
    recorder.whitelist("prefix argv-recorder\n");
    recorder.blacklist(&template_rule("argv-recorder deploy {{ target }}"));
    recorder.compose(&[]).failure();
    assert!(recorder.runs().is_empty());

    let recorder = Recorder::new("template_blacklist_resolved_chain");
    recorder.write("doc.md", "::shell argv-recorder a && argv-recorder b\n");
    recorder.whitelist("prefix argv-recorder\n");
    recorder.blacklist(&template_rule("argv-recorder a && argv-recorder {{ x }}"));
    let assert = recorder.compose(&[]).failure();
    assert!(mentions(&stderr_of(&assert), "user blacklist"), "{}", stderr_of(&assert));
    assert!(recorder.runs().is_empty(), "the first segment never starts");
}

/// One shape approval covers every value; a changed value is a fresh run,
/// while the same filled command is served from the compose cache.
#[test]
fn a_shape_approval_is_reused_but_changed_values_run_fresh() {
    let recorder = Recorder::new("template_fresh_results");
    recorder.write(
        "doc.md",
        "---\na: one\nb: two\n---\n::shell argv-recorder {{ a }}\n::shell argv-recorder {{ b }}\n::shell argv-recorder {{ a }}\n",
    );
    recorder.whitelist(&template_rule("argv-recorder {{ a }}"));

    recorder.compose(&[]).success();

    assert_eq!(recorder.runs(), vec![strings(&["one"]), strings(&["two"])]);
}

/// An exact rule runs the resolved bytes and a template rule the filled
/// template; with both, the exact rule wins. Neither authorizes the other's
/// mode: the bytes split a spaced value, the template keeps it whole.
#[test]
fn exact_and_template_rules_select_their_own_execution_mode() {
    let doc = "---\npair: a b\n---\n::shell argv-recorder {{ pair }}\n";
    let exact = "exact argv-recorder a b\n";
    let template = template_rule("argv-recorder {{ pair }}");
    let cases = [
        (exact.to_string(), strings(&["a", "b"])),
        (template.clone(), strings(&["a b"])),
        (format!("{template}{exact}"), strings(&["a", "b"])),
    ];
    for (rules, expected) in cases {
        let recorder = Recorder::new("template_mode_selection");
        recorder.write("doc.md", doc);
        recorder.whitelist(&rules);
        recorder.compose(&[]).success();
        assert_eq!(recorder.runs(), vec![expected], "{rules}");
    }
}

/// Template input failures are not command failures: `--when-error`, an
/// allowed timeout, and a transclusion notice never absorb them.
#[test]
fn template_input_failures_are_never_suppressed() {
    let recorder = Recorder::new("template_unsuppressed_directive");
    recorder.write("doc.md", "::shell --when-error fallback argv-recorder {{ v }}\n");
    recorder.whitelist(&template_rule("argv-recorder {{ v }}"));
    let assert = recorder.compose(&["v=[1]", "--allow-shell-timeout"]).failure();
    assert!(mentions(&stderr_of(&assert), "is an array or object"), "{}", stderr_of(&assert));
    assert!(!String::from_utf8_lossy(&assert.get_output().stdout).contains("fallback"));

    let recorder = Recorder::new("template_unsuppressed_transclusion");
    recorder.write("doc.md", "# Parent\n\n::file ./child.md\n");
    recorder.write("child.md", "---\nv: [1, 2]\n---\n::shell argv-recorder {{ v }}\n");
    recorder.whitelist(&template_rule("argv-recorder {{ v }}"));
    let assert = recorder.compose(&[]).failure();
    assert!(mentions(&stderr_of(&assert), "is an array or object"), "{}", stderr_of(&assert));
    assert!(recorder.runs().is_empty());
}

/// A template program that needs an interpreter is refused before it runs;
/// nothing falls back to a shell.
#[cfg(unix)]
#[test]
fn a_script_without_a_shebang_is_refused_and_a_shebang_script_runs() {
    use std::os::unix::fs::PermissionsExt as _;

    let recorder = Recorder::new("template_target_unix");
    let marker = recorder.fixture.cwd().join("ran-marker");
    let plain = recorder.fixture.bin_dir().join("plain-tool");
    std::fs::write(&plain, format!("touch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).unwrap();
    recorder.write("doc.md", "::shell plain-tool {{ v }}\n");
    recorder.whitelist(&template_rule("plain-tool {{ v }}"));

    let assert = recorder.compose(&["v=x"]).failure();

    assert!(
        mentions(&stderr_of(&assert), "neither a native executable nor a #! script"),
        "{}",
        stderr_of(&assert)
    );
    assert!(!marker.exists(), "the script never ran through a shell");

    let shebang = recorder.fixture.bin_dir().join("shebang-tool");
    std::fs::write(&shebang, "#!/bin/sh\nexec argv-recorder \"$@\"\n").unwrap();
    std::fs::set_permissions(&shebang, std::fs::Permissions::from_mode(0o755)).unwrap();
    recorder.write("doc.md", "::shell shebang-tool {{ v }}\n");
    recorder.whitelist(&template_rule("shebang-tool {{ v }}"));
    recorder.compose(&["v=a b"]).success();
    assert_eq!(recorder.runs(), vec![strings(&["a b"])]);
}

/// A template program that needs an interpreter is refused before it runs;
/// nothing falls back to `cmd.exe`.
#[cfg(windows)]
#[test]
fn a_batch_file_is_refused_as_a_template_target() {
    for extension in ["cmd", "bat"] {
        let recorder = Recorder::new("template_target_windows");
        let marker = recorder.fixture.cwd().join("ran-marker");
        let tool = recorder.fixture.bin_dir().join(format!("batch-tool.{extension}"));
        std::fs::write(&tool, format!("@echo ran> \"{}\"\r\n", marker.display())).unwrap();
        recorder.write("doc.md", "::shell batch-tool {{ v }}\n");
        recorder.whitelist(&template_rule("batch-tool {{ v }}"));

        let assert = recorder.compose(&["v=x"]).failure();

        assert!(
            mentions(&stderr_of(&assert), &format!("needs an interpreter (a .{extension} file)")),
            "{}",
            stderr_of(&assert)
        );
        assert!(!marker.exists(), "the batch file never ran");
    }
}


macro_rules! native_redirect_case {
    ($name:ident, $redirect:literal, $out:literal, $err:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new("template-native-redirection");
            let command = concat!("argv-recorder {{ v }} ", $redirect);
            recorder.write("doc.md", &format!("---\nv: a b\n---\n::shell {command}\n"));
            recorder.whitelist(&template_rule(command));
            let assert = recorder.fixture.command()
                .env(ARGV_RECORDER_FILE, &recorder.records)
                .env("ARGV_RECORDER_STDOUT", "stdout-probe")
                .env("ARGV_RECORDER_STDERR", "stderr-probe")
                .args(["compose", "doc.md"]).assert().success();
            let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
            assert_eq!(stdout.contains("stdout-probe"), $out, "{stdout}");
            assert_eq!(stdout.contains("stderr-probe"), $err, "{stdout}");
            assert_eq!(recorder.runs(), vec![strings(&["a b"])]);
        }
    };
}

native_redirect_case!(native_merge_then_null_preserves_the_original_stdout_capture, "2>&1 > /dev/null", false, true);
native_redirect_case!(native_null_then_merge_discards_both_streams, "> /dev/null 2>&1", false, false);
native_redirect_case!(native_stderr_null_keeps_stdout, "2> /dev/null", true, false);
native_redirect_case!(native_merged_capture_receives_both_streams, "2>&1", true, true);
native_redirect_case!(native_stderr_null_then_stdout_to_stderr_discards_both, "2> /dev/null >&2", false, false);


#[test]
fn a_native_result_keeps_the_original_channel_after_a_later_redirect() {
    let recorder = Recorder::new("template-native-result-channel");
    let command = "argv-recorder {{ word }} 2>&1 > /dev/null";
    recorder.write("doc.md", &format!("---\nword: a b\nresult: '$({command})::result'\n---\nCaptured: {{{{ result.stdout }}}}\nErrors: {{{{ result.stderr }}}}\n"));
    recorder.whitelist(&template_rule(command));
    let assert = recorder.fixture.command()
        .env(ARGV_RECORDER_FILE, &recorder.records)
        .env("ARGV_RECORDER_STDOUT", "stdout-probe")
        .env("ARGV_RECORDER_STDERR", "stderr-probe")
        .args(["compose", "doc.md"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("Captured: stderr-probe"), "{stdout}");
    assert!(!stdout.contains("Errors: stderr-probe"), "{stdout}");
    assert!(!stdout.contains("stdout-probe"), "{stdout}");
    assert_eq!(recorder.runs(), vec![strings(&["a b"])]);
}
