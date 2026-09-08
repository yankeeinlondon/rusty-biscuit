//! AC3's contamination probes: an exported variable on the developer's machine
//! cannot change what an L1 test observes.
//!
//! `cli_process_fixture.rs` proves the builder *removes* each family, by
//! recording the environment a provider stub actually received. That is the
//! mechanism. This binary proves the consequence: with each family exported in
//! the parent process, an ordinary `claudine compose` run — the shape most of
//! the migrated L1 suite is — still produces the same observable result.
//!
//! The two are not redundant. A scrub that removed the right keys but left the
//! run reading them from somewhere else would pass the recording tests and fail
//! here; a run that happened not to consult a leaked key would pass here and
//! fail there.
//!
//! ## What each probe may touch
//!
//! Disposable state only. Every path a probe writes is inside the fixture
//! workspace or a throwaway directory the probe creates under the system temp
//! directory, and the process-environment mutations are safe because nextest
//! gives each test its own process — `#[test]` here means "own process", so an
//! export cannot reach another probe or another binary. Nothing writes to the
//! rusty-biscuit checkout or to the user's configuration.
//!
//! The one exception is [`a_checkout_ancestor_temp_dir_is_refused_at_construction`],
//! which has to put the temp directory *inside* the checkout to reproduce the
//! condition at all. It uses `target/` — gitignored build output — creates one
//! directory, and removes it before returning, including on the panic path.

#![cfg(unix)]

use std::path::{Path, PathBuf};

mod common;
use common::{CliProcessFixture, strip_ansi, write, write_executable};

/// The line the `success` stack emits.
///
/// Longer than 44 columns and shorter than 80, so "the phrase arrived on one
/// line" discriminates a leaked `COLUMNS=44` from the documented 80-column
/// fallback rather than being a tautology.
const MARKER: &str = "probe-marker the success stack ran to completion";

/// What the staged provider stub prints, and therefore what `compose` writes to
/// stdout. A decoy provider winning selection changes this line, so it is the
/// `PATH` probe's discriminator.
const PROVIDER_TEXT: &str = "the-staged-stub-is-the-provider-that-ran";

/// Body of the user system prompt the fixture home carries.
const HOME_SENTINEL: &str = "probe-user-system-prompt-from-the-fixture-home\n";

/// The `loop.max` [`observed_iteration_cap`] authors. A leaked
/// `CLAUDINE_MAX_ITERATIONS` replaces it, which is how the
/// application-variable family becomes observable at all.
const AUTHORED_CAP: usize = 3;

/// What an unrelated migrated test depends on: the exit status, the composed
/// body on stdout, and the lifecycle line on stderr.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    stdout: String,
    /// The single stderr line carrying [`MARKER`], with framing trimmed.
    marker_line: String,
    /// Number of stderr rows carrying [`MARKER`].
    marker_rows: usize,
    /// Whether the pre-flight system-prompt report carried [`HOME_SENTINEL`],
    /// i.e. whether the run read the fixture home rather than another one.
    read_the_fixture_home: bool,
    /// What that line has to say for the run to have been uncontaminated.
    expected_marker_line: String,
    /// What stdout has to carry: the composed body — naming the repository the
    /// fixture built, which leaked Git plumbing would move — then the staged
    /// stub's own identity, which a decoy on `PATH` would replace.
    expected_stdout: String,
    /// Whether the stderr row carrying [`MARKER`] arrived styled.
    ///
    /// Anchored on that row rather than on the whole stream: claudine emits
    /// cursor and hyperlink sequences elsewhere even under `NO_COLOR`, so
    /// "stderr contains no escape" is false on a clean run and would make this
    /// field a constant rather than a discriminator.
    marker_row_styled: bool,
}

/// Run the representative composition and report what came back.
///
/// Deliberately an ordinary `CliProcessFixture` run with no escape: the claim
/// under test is about what the *default* gives every migrated binary.
fn observed_outcome() -> Outcome {
    let fixture = CliProcessFixture::named("contamination-probe");
    fixture.seed_user_config();
    fixture.initialize_repository();
    // A *user* system prompt in the fixture home. The pre-flight report names
    // the file the system prompt was composed from, so this is what makes the
    // run sensitive to `HOME`: the builder *sets* `HOME` rather than scrubbing
    // it, and a decoy home carries no such file.
    fixture.write_user_system_prompt(HOME_SENTINEL);
    // The stub echoes the `probe-body` line out of whatever the wrapper
    // delivered — argv, a prompt file, or stdin — and then identifies itself.
    // That puts the *composed* body on stdout, which is how `ctx.repo_root`
    // becomes observable to the Git-plumbing probe.
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            r#"#!/bin/sh
{{
  for a in "$@"; do
    if [ -f "$a" ]; then cat "$a"; else printf '%s\n' "$a"; fi
  done
  cat
}} 2>/dev/null | grep '^probe-body'
printf '%s\n' '{PROVIDER_TEXT}'
exit 0
"#
        ),
    );

    let doc = fixture.cwd().join("doc.md");
    write(
        &doc,
        &format!(
            "---\ntitle: contamination probe\nsuccess:\n  info: '{MARKER}'\n---\nprobe-body repo={{{{ ctx.repo_root }}}}\n"
        ),
    );

    let output = fixture
        .command()
        .args(["compose", "--goose", doc.to_str().unwrap()])
        .output()
        .expect("the probe composition must run");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let marker_row_styled = stderr
        .lines()
        .find(|line| strip_ansi(line).contains(MARKER))
        .is_some_and(|line| line.contains('\u{1b}'));
    let plain = strip_ansi(&stderr);
    let marker_rows = plain.lines().filter(|line| line.contains(MARKER)).count();
    let marker_line = plain
        .lines()
        .find(|line| line.contains(MARKER))
        .map(|line| {
            line.trim()
                .trim_start_matches(['┃', 'ℹ'])
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| {
            panic!("the probe's `success` line never reached stderr; stderr:\n{plain}")
        });

    Outcome {
        code: output.status.code(),
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        marker_line,
        marker_rows,
        read_the_fixture_home: plain.replace(['┃', '\n'], "").contains(HOME_SENTINEL.trim()),
        expected_marker_line: MARKER.to_string(),
        expected_stdout: format!(
            "probe-body repo={repo}\n{PROVIDER_TEXT}\n",
            repo = biscuit_file::to_portable_string(&canonical(fixture.cwd()))
        ),
        marker_row_styled,
    }
}

/// Assert the observed outcome is the uncontaminated one.
///
/// Every probe below calls this after exporting its family, so a leak shows up
/// as a difference in the run's own result rather than as a missing key in a
/// recording.
fn assert_uncontaminated(what: &str) {
    let observed = observed_outcome();
    assert_eq!(observed.code, Some(0), "{what}: the probe run must succeed");
    assert_eq!(
        observed.marker_rows, 1,
        "{what}: the `success` stack must run exactly once"
    );
    assert!(
        observed.read_the_fixture_home,
        "{what}: the system prompt must be composed from the fixture home's own \
         `system-prompt.md`, so the run read the home the builder pinned"
    );
    assert_eq!(
        observed.stdout, observed.expected_stdout,
        "{what}: the composed body must name the repository the fixture built, and the \
         staged stub must be the provider that ran"
    );
    assert_eq!(
        observed.marker_line, observed.expected_marker_line,
        "{what}: the `success` line must arrive whole — a leaked render width would \
         wrap it"
    );
    assert!(
        !observed.marker_row_styled,
        "{what}: the `success` row must stay unstyled — `NO_COLOR=1` is a fixture \
         default and nothing inherited may out-vote it"
    );
}

/// The iteration cap the wrapper reports for a document whose `loop` never
/// satisfies its `until`.
///
/// The rest of this binary's observables are insensitive to the `CLAUDINE_*`
/// family — a single `compose` run consults none of those variables in a way
/// that reaches stdout, stderr, or the exit status — so this is the one
/// discriminator that family has. `CLAUDINE_MAX_ITERATIONS` overrides the
/// authored `max`, and the cap-exceeded diagnostic names the cap that applied.
fn observed_iteration_cap() -> usize {
    let fixture = CliProcessFixture::named("contamination-probe-cap");
    fixture.seed_user_config();
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf 'ok\\n'\nexit 0\n",
    );
    let doc = fixture.cwd().join("loop.md");
    write(
        &doc,
        &format!(
            "---\ntitle: cap probe\nloop:\n  until: 'false'\n  max: {AUTHORED_CAP}\n---\nBody.\n"
        ),
    );

    let output = fixture
        .command()
        .args(["compose", "--goose", doc.to_str().unwrap()])
        .output()
        .expect("the cap probe must run");
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr)).replace(['┃', '\n'], " ");
    let (_, tail) = stderr
        .split_once("cap is ")
        .unwrap_or_else(|| panic!("the cap-exceeded diagnostic must name the cap; stderr:\n{stderr}"));
    tail.trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|error| panic!("cap is not a number ({error}); tail: {tail:?}"))
}

/// The control. Without it, every probe below could be passing because the
/// expectations describe a broken run rather than a clean one.
#[test]
fn an_uncontaminated_run_matches_the_expectations_every_probe_asserts() {
    assert_uncontaminated("baseline");
    assert_eq!(observed_iteration_cap(), AUTHORED_CAP);
}

/// Application variables. An exported `CLAUDINE_TIMEOUT` / `CLAUDINE_STEP_TIMEOUT`
/// re-parameterizes every timeout-shaped test in the suite; a value this small
/// would end the run before its `success` stack fired.
#[test]
fn exported_claudine_application_variables_do_not_change_the_result() {
    unsafe {
        // Short enough to end the run before its `success` stack fires: the
        // grammar takes fractional seconds, so this is 10 ms.
        std::env::set_var("CLAUDINE_TIMEOUT", "0.01s");
        std::env::set_var("CLAUDINE_STEP_TIMEOUT", "0.01s");
        // Caps the probe document's `loop` below its authored `max`, which is
        // what makes this probe discriminating rather than merely green.
        std::env::set_var("CLAUDINE_MAX_ITERATIONS", "1");
    }
    assert_uncontaminated("CLAUDINE_* application variables");
    assert_eq!(
        observed_iteration_cap(),
        AUTHORED_CAP,
        "an inherited CLAUDINE_MAX_ITERATIONS replaced the document's authored cap"
    );
}

/// Git plumbing. `GIT_DIR`/`GIT_WORK_TREE` override cwd-based discovery
/// outright, so a leaked pair points the run at the throwaway repository below
/// instead of at the one the fixture built — and defeats the pinned
/// `current_dir` without touching it.
#[test]
fn exported_git_plumbing_pointed_at_a_throwaway_repo_does_not_change_the_result() {
    // Disposable: a repository this probe creates under the system temp
    // directory, never the rusty-biscuit checkout.
    let decoy = throwaway_dir("contamination-probe-decoy-repo");
    assert!(
        common::init_git_repo(&decoy),
        "the probe needs a real throwaway repository to point the plumbing at"
    );
    unsafe {
        std::env::set_var("GIT_DIR", decoy.join(".git"));
        std::env::set_var("GIT_WORK_TREE", &decoy);
        std::env::set_var("GIT_INDEX_FILE", decoy.join(".git/index"));
    }

    assert_uncontaminated("GIT_* plumbing");

    let _ = std::fs::remove_dir_all(&decoy);
}

/// Home and cache relocation. The user's real `~/.claudine/config.json` is the
/// classic contaminant: a `preferred_agent` there changes which provider a run
/// with no explicit flag selects.
#[test]
fn exported_home_and_cache_relocation_does_not_change_the_result() {
    let elsewhere = throwaway_dir("contamination-probe-home");
    std::fs::create_dir_all(elsewhere.join(".claudine")).expect("throwaway home");
    // A `preferred_agent` the fixture never staged: if this config were read,
    // provider selection would pick `claude` and find nothing to run.
    write(
        &elsewhere.join(".claudine/config.json"),
        "{\"preferred_agent\": \"claude\", \"tts\": true}\n",
    );
    unsafe {
        std::env::set_var("HOME", &elsewhere);
        std::env::set_var("XDG_CONFIG_HOME", elsewhere.join("config"));
        std::env::set_var("XDG_CACHE_HOME", elsewhere.join("cache"));
        std::env::set_var("HOMEDRIVE", "Z:");
        std::env::set_var("HOMEPATH", "\\sentinel-homepath");
    }

    assert_uncontaminated("HOME / XDG relocation");

    let _ = std::fs::remove_dir_all(&elsewhere);
}

/// `PATH`. A host with a real `goose` installed must not be able to win
/// provider selection from the stub the probe staged.
#[test]
fn an_exported_path_carrying_a_decoy_provider_does_not_change_the_result() {
    let decoy_bin = throwaway_dir("contamination-probe-decoy-bin");
    // A decoy `goose` that would fail the run outright if it were selected.
    write_executable(
        &decoy_bin.join("goose"),
        "#!/bin/sh\nprintf 'DECOY-PROVIDER-WON\\n'\nexit 9\n",
    );
    let host = std::env::var_os("PATH").unwrap_or_default();
    let mut entries = vec![decoy_bin.clone()];
    entries.extend(std::env::split_paths(&host));
    unsafe {
        std::env::set_var(
            "PATH",
            std::env::join_paths(entries).expect("decoy PATH must join"),
        );
    }

    assert_uncontaminated("PATH carrying a decoy provider");

    let _ = std::fs::remove_dir_all(&decoy_bin);
}

/// Inherited width and color. `COLUMNS=44` is the exact value that reddened
/// `compose_schema_cli` and `composition_outputs` before Phase 5A; `FORCE_COLOR=1`
/// would out-vote the fixture's `NO_COLOR=1` if it reached the child.
#[test]
fn exported_render_width_and_color_do_not_change_the_result() {
    unsafe {
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("TERM_WIDTH", "44");
        std::env::set_var("FORCE_COLOR", "1");
    }
    assert_uncontaminated("COLUMNS / TERM_WIDTH / FORCE_COLOR");
}

/// A relocated temp directory is not a contaminant: the fixture workspace moves
/// with it and the run is unchanged.
///
/// The refusal half — a temp directory *inside* the checkout — is
/// [`a_checkout_ancestor_temp_dir_is_refused_at_construction`].
#[test]
fn a_relocated_temp_dir_outside_the_checkout_does_not_change_the_result() {
    let relocated = throwaway_dir("contamination-probe-tmpdir");
    unsafe {
        std::env::set_var("TMPDIR", &relocated);
    }
    assert_eq!(
        std::env::temp_dir(),
        relocated,
        "the probe is vacuous unless `temp_dir()` actually moved"
    );

    assert_uncontaminated("relocated TMPDIR");

    let _ = std::fs::remove_dir_all(&relocated);
}

/// The checkout-ancestor `TMPDIR` case, end to end.
///
/// `cli_process_fixture.rs` exercises `checkout_containment_error` as a pure
/// function, which cannot say that `CliProcessFixture::named` calls it. This
/// does, by reproducing the real condition: `TMPDIR` inside the checkout, so
/// `std::env::temp_dir()` — and therefore the fixture workspace — lands there.
///
/// It writes one directory under `target/`, which is gitignored build output
/// rather than the checkout's tracked content, and removes it on both the
/// panicking and non-panicking paths.
#[test]
fn a_checkout_ancestor_temp_dir_is_refused_at_construction() {
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .expect("this test binary was compiled from a checkout")
        .to_path_buf();
    // Under `target/`: gitignored, disposable, and outside the crate.
    let inside = checkout.join("target/contamination-probe-tmpdir");
    std::fs::create_dir_all(&inside).expect("throwaway temp dir inside the checkout");
    unsafe {
        std::env::set_var("TMPDIR", &inside);
    }

    let outcome = std::panic::catch_unwind(|| CliProcessFixture::named("inside-the-checkout"));
    let _ = std::fs::remove_dir_all(&inside);

    let panic = outcome.err().expect(
        "a fixture workspace inside the checkout must be refused at construction — otherwise \
         every 'isolated' L1 run walks back out to the checkout",
    );
    let message = panic
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| String::from("<non-string panic>"));
    assert!(
        message.contains("fixture precondition") && message.contains("TMPDIR"),
        "the refusal must name the cause and the variable to change; got: {message}"
    );
}

/// A throwaway directory under the *current* system temp directory.
///
/// Resolved before any probe moves `TMPDIR`, so the directory a probe cleans up
/// is the one it created.
fn throwaway_dir(prefix: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path: PathBuf =
        std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&path).expect("throwaway directory");
    canonical(&path)
}

/// macOS hands out `/var/...` where `temp_dir()` later reports `/private/var/...`,
/// so the `TMPDIR` probe's equality assertion needs the canonical spelling.
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
