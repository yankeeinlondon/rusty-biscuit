//! Schema-aware setter completion for a committed prompt argument, through
//! the real `claudine __complete` process.
//!
//! The committed prompt is located with the same file-reference grammar and
//! request context composition uses, so every form `claudine compose` accepts
//! (bare, `./`, absolute, `&`, `^`, `@`, `~/`) yields the prompt's setter names
//! and enum values, from the repository root and from a nested package. An
//! explicit `./` never searches the repository root.

use std::path::{Path, PathBuf};

use crate::common;
use common::CliProcessFixture;
use common::completion::{seed_cargo_workspace_members, write_file};

const PROMPT: &str = concat!(
    "---\n",
    "$schema:\n",
    "  zebra: enum(red, blue)\n",
    "  apple: enum(one, two)\n",
    "---\n",
    "Body\n",
);

/// The review's reproduction topology: a workspace with `area/lib` and
/// `area/cli` members, a separate fixture home, and identical prompts at the
/// repository root, the repository prompt root, the home root, and the user
/// prompt root. Nothing named `prompt.md` exists inside the package.
struct Topology {
    fixture: CliProcessFixture,
}

impl Topology {
    fn create(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        seed_cargo_workspace_members(fixture.cwd(), &["area/lib", "area/cli"]);
        fixture.initialize_repository();
        for path in [
            fixture.cwd().join("prompt.md"),
            fixture.cwd().join("prompts/prompt.md"),
            fixture.home().join("prompt.md"),
            fixture.home().join(".claudine/prompts/prompt.md"),
        ] {
            write_file(&path, PROMPT);
        }
        Self { fixture }
    }

    fn repo(&self) -> PathBuf {
        self.fixture.cwd().to_path_buf()
    }

    fn package(&self) -> PathBuf {
        self.fixture.cwd().join("area/lib")
    }

    /// `claudine __complete` with the cursor on the last element of `words`.
    fn complete(&self, launch: &Path, words: &[&str]) -> Vec<String> {
        let output = self
            .fixture
            .command_builder()
            // The launch directory decides what each committed form names,
            // which is this test's subject.
            .ambient_context(launch)
            .build()
            .args(["__complete", "--current", &words.len().to_string(), "--", "claudine"])
            .args(words)
            .output()
            .expect("run claudine __complete");
        assert!(
            output.status.success(),
            "completion failed for {words:?}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8(output.stdout)
            .expect("completion stdout is UTF-8")
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
enum Launch {
    RepositoryRoot,
    Package,
}

/// One committed prompt form and whether it names a prompt from `launch`.
struct Case {
    argument: fn(&Topology) -> String,
    label: &'static str,
    found_from_package: bool,
}

const CASES: [Case; 7] = [
    Case { argument: |_| "prompt.md".into(), label: "bare", found_from_package: true },
    // The only `./prompt.md` candidate from the package is absent; the
    // repository-root copy must not answer for it.
    Case { argument: |_| "./prompt.md".into(), label: "explicit relative", found_from_package: false },
    Case {
        argument: |topology| topology.repo().join("prompt.md").to_string_lossy().into_owned(),
        label: "absolute",
        found_from_package: true,
    },
    Case { argument: |_| "&prompt.md".into(), label: "repository root", found_from_package: true },
    Case { argument: |_| "^prompt.md".into(), label: "repository scoped", found_from_package: true },
    Case { argument: |_| "@prompt.md".into(), label: "magic", found_from_package: true },
    Case { argument: |_| "~/prompt.md".into(), label: "home", found_from_package: true },
];

/// Every case, from both launch directories, for one command: the setter
/// name for the partial `z` and the enum values after `zebra=`.
fn assert_schema_completion_for(command: &str) {
    let topology = Topology::create(&format!("committed-prompt-schema-{command}"));
    let mut failures = Vec::new();
    for case in &CASES {
        let argument = (case.argument)(&topology);
        for launch in [Launch::RepositoryRoot, Launch::Package] {
            let dir = match launch {
                Launch::RepositoryRoot => topology.repo(),
                Launch::Package => topology.package(),
            };
            let found = matches!(launch, Launch::RepositoryRoot) || case.found_from_package;
            let (names, values): (Vec<String>, Vec<String>) = if found {
                (vec!["zebra=".into()], vec!["zebra='blue'".into(), "zebra='red'".into()])
            } else {
                (Vec::new(), Vec::new())
            };

            let got_names = topology.complete(&dir, &[command, &argument, "z"]);
            if got_names != names {
                failures.push(format!(
                    "{} `{argument}` from {launch:?}: names {got_names:?}, expected {names:?}",
                    case.label
                ));
            }
            let mut got_values = topology.complete(&dir, &[command, &argument, "zebra="]);
            got_values.sort();
            if got_values != values {
                failures.push(format!(
                    "{} `{argument}` from {launch:?}: values {got_values:?}, expected {values:?}",
                    case.label
                ));
            }
        }
    }
    assert!(failures.is_empty(), "`{command}` schema completion:\n{}", failures.join("\n"));
}

#[test]
fn compose_schema_completion_resolves_every_committed_prompt_form() {
    assert_schema_completion_for("compose");
}

#[test]
fn inline_compose_schema_completion_resolves_every_committed_prompt_form() {
    assert_schema_completion_for("inline-compose");
}

#[test]
fn sequence_schema_completion_resolves_every_committed_prompt_form() {
    assert_schema_completion_for("sequence");
}

#[test]
fn emitted_magic_prompt_token_feeds_schema_completion_unchanged() {
    let topology = Topology::create("committed-prompt-schema-round-trip");
    for launch in [topology.repo(), topology.package()] {
        let emitted = topology.complete(&launch, &["compose", "@promp"]);
        assert_eq!(emitted, vec!["@prompt.md".to_string()], "from {}", launch.display());
        let token = &emitted[0];

        let names = topology.complete(&launch, &["compose", token, "z"]);
        assert_eq!(names, vec!["zebra=".to_string()], "from {}", launch.display());
        let mut values = topology.complete(&launch, &["compose", token, "zebra="]);
        values.sort();
        assert_eq!(
            values,
            vec!["zebra='blue'".to_string(), "zebra='red'".to_string()],
            "from {}",
            launch.display(),
        );
    }
}

/// The authored property order of a referenced `$schema` file is read through
/// the same grammar as the document's own reference, so a prefixed reference
/// keeps `zeta` ahead of `alpha` rather than falling back to alphabetical.
#[test]
fn referenced_schema_order_follows_every_reference_form() {
    let topology = Topology::create("committed-prompt-schema-order");
    write_file(
        &topology.repo().join("schemas/order.yaml"),
        "$schema:\n  zeta: enum(x, y)\n  alpha: enum(x, y)\n",
    );
    write_file(
        &topology.repo().join("prompts/order.yaml"),
        "$schema:\n  zeta: enum(x, y)\n  alpha: enum(x, y)\n",
    );
    let mut failures = Vec::new();
    for (index, reference) in ["./order.yaml", "&schemas/order.yaml", "^schemas/order.yaml"]
        .into_iter()
        .enumerate()
    {
        let prompt = topology.repo().join(format!("prompts/ordered-{index}.md"));
        write_file(&prompt, &format!("---\n$schema: '{reference}'\n---\nBody\n"));
        let argument = prompt.to_string_lossy().into_owned();
        for launch in [topology.repo(), topology.package()] {
            let got = topology.complete(&launch, &["compose", &argument, "a"]);
            if got != ["zeta=", "alpha="] {
                failures.push(format!("`{reference}` from {}: {got:?}", launch.display()));
            }
        }
    }
    assert!(failures.is_empty(), "authored order lost:\n{}", failures.join("\n"));
}

/// A committed directory walks the shared expansion's roots in resolution
/// order — the launch directory, then the repository root — and the token it
/// emits composes to the file it was offered for.
#[test]
fn committed_prompt_directory_walks_the_launch_directory_first() {
    let topology = Topology::create("committed-prompt-directory");
    let package = topology.package();
    write_file(&package.join("prompts/local.md"), "PACKAGE_LOCAL_PROMPT\n");
    write_file(&package.join("prompts/shared.md"), "PACKAGE_SHARED_PROMPT\n");
    write_file(&topology.repo().join("prompts/shared.md"), "REPOSITORY_SHARED_PROMPT\n");

    let candidates = topology.complete(&package, &["compose", "prompts/"]);
    for expected in ["prompts/local.md", "prompts/shared.md", "prompts/prompt.md"] {
        assert!(
            candidates.iter().any(|candidate| candidate == expected),
            "`{expected}` missing from {candidates:?}",
        );
    }
    assert_eq!(
        candidates.iter().filter(|candidate| *candidate == "prompts/shared.md").count(),
        1,
        "{candidates:?}",
    );

    for (token, marker) in [
        ("prompts/local.md", "PACKAGE_LOCAL_PROMPT"),
        ("prompts/shared.md", "PACKAGE_SHARED_PROMPT"),
    ] {
        let output = topology
            .fixture
            .command_builder()
            // The package launch decides which `prompts/` wins.
            .ambient_context(&package)
            .build()
            .args(["compose", "--dry-run", token])
            .output()
            .expect("run claudine compose --dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains(marker),
            "`{token}` must compose the offered file; status {}, stdout:\n{stdout}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

// ---------------------------------------------------------------------
// Template schemas: names never evaluate; a slot-bearing property's value
// resolves the template with non-prompting grants only
// ---------------------------------------------------------------------

/// The template schema: `os` holds a template constraint, `status` a
/// template type, and `kind` is static.
const TEMPLATE_SCHEMA: &str = concat!(
    "$schema:\n",
    "  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }}; required)\n",
    "  status: '{{ load_data(\"./status.yaml\", \"{ type: string }\").type }}'\n",
    "  kind: enum(a, b)\n",
);

/// A repository holding a prompt whose `$schema` file is [`TEMPLATE_SCHEMA`].
struct TemplatePrompt {
    fixture: CliProcessFixture,
}

/// What one `claudine __complete` printed.
struct Completed {
    candidates: Vec<String>,
    stderr: String,
}

impl TemplatePrompt {
    fn create(name: &str, schema: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let cwd = fixture.cwd();
        write_file(&cwd.join("os.yaml"), "- macOS\n- Linux\n");
        write_file(&cwd.join("status.yaml"), "type: enum(draft, published)\n");
        write_file(&cwd.join("prompt.schema.yaml"), schema);
        write_file(&cwd.join("prompt.md"), "---\n$schema: ./prompt.schema.yaml\n---\nBody\n");
        Self { fixture }
    }

    /// Commits every file, so the automatic grant covers the slots' reads.
    fn commit(&self) {
        for args in [&["init", "-q"][..], &["add", "-A"][..], &["commit", "-q", "-m", "fixture"][..]] {
            let output = common::helper_command("git")
                .arg("-C")
                .arg(self.fixture.cwd())
                .args(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
                .args(args)
                .output()
                .expect("run git in the fixture");
            assert!(output.status.success(), "git {args:?}: {output:?}");
        }
    }

    fn write_grants(&self, lines: &str) {
        write_file(&self.fixture.home().join(".config/darkmatter/schema-read-policy"), lines);
    }

    /// The canonical spelling of a fixture file, as a grant names it.
    fn canonical(&self, relative: &str) -> String {
        biscuit_file::canonicalize_simplified(&self.fixture.cwd().join(relative))
            .expect("fixture file exists")
            .display()
            .to_string()
    }

    /// `claudine __complete` with the cursor on the last element of `words`.
    fn complete(&self, words: &[&str]) -> Completed {
        let output = self
            .fixture
            .command_builder()
            // The prompt and its schema live in the fixture repository.
            .ambient_context(self.fixture.cwd())
            .build()
            .args(["__complete", "--current", &words.len().to_string(), "--", "claudine"])
            .args(words)
            .output()
            .expect("run claudine __complete");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "completion failed for {words:?}: {stderr}");
        let candidates = String::from_utf8(output.stdout)
            .expect("completion stdout is UTF-8")
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        Completed { candidates, stderr }
    }

    fn values(&self, setter: &str) -> Vec<String> {
        let completed = self.complete(&["compose", "prompt.md", setter]);
        assert_eq!(completed.stderr, "", "a Tab never writes to the terminal");
        completed.candidates
    }
}

#[test]
fn template_names_complete_from_the_authored_template() {
    let prompt = TemplatePrompt::create("template-schema-names", TEMPLATE_SCHEMA);
    // No repository: no slot could resolve, and names still complete.
    let completed = prompt.complete(&["compose", "prompt.md", "stat"]);
    assert_eq!(completed.candidates, ["status="]);
    assert_eq!(completed.stderr, "");
}

#[test]
fn template_constraint_value_resolves_on_a_clean_commit() {
    let prompt = TemplatePrompt::create("template-schema-constraint", TEMPLATE_SCHEMA);
    prompt.commit();
    assert_eq!(prompt.values("os="), ["os='macOS'", "os='Linux'"]);
}

#[test]
fn template_type_value_resolves_on_a_clean_commit() {
    let prompt = TemplatePrompt::create("template-schema-type", TEMPLATE_SCHEMA);
    prompt.commit();
    assert_eq!(prompt.values("status="), ["status='draft'", "status='published'"]);
}

#[test]
fn an_ungranted_template_offers_nothing_and_prints_nothing() {
    let prompt = TemplatePrompt::create("template-schema-ungranted", TEMPLATE_SCHEMA);
    prompt.commit();
    // Dirty after the commit: outside the automatic grant, and no persisted
    // grant covers it.
    write_file(&prompt.fixture.cwd().join("os.yaml"), "- macOS\n- Linux\n- Windows\n");
    assert!(prompt.values("os=").is_empty());
    // The template resolves as a whole: the other slot's property offers
    // nothing either.
    assert!(prompt.values("status=").is_empty());
}

#[test]
fn a_static_property_of_a_template_completes_without_evaluation() {
    let prompt = TemplatePrompt::create("template-schema-static", TEMPLATE_SCHEMA);
    // No repository and no grant: resolving would fail, so offered values
    // prove the static declaration answered alone.
    assert_eq!(prompt.values("kind="), ["kind='a'", "kind='b'"]);
}

#[test]
fn a_persisted_user_grant_is_honored() {
    let prompt = TemplatePrompt::create("template-schema-persisted", TEMPLATE_SCHEMA);
    assert!(prompt.values("os=").is_empty(), "control: nothing grants the reads yet");
    prompt.write_grants(&format!(
        "allow file {}\nallow file {}\n",
        prompt.canonical("os.yaml"),
        prompt.canonical("status.yaml"),
    ));
    assert_eq!(prompt.values("os=L"), ["os='Linux'"]);
    assert_eq!(prompt.values("status="), ["status='draft'", "status='published'"]);
}

#[test]
fn a_url_reading_template_offers_nothing_even_when_its_site_is_granted() {
    let schema = concat!(
        "$schema:\n",
        "  os: enum({{ load_data(\"https://schemas.example.invalid/os.yaml\", \"string[]\") }})\n",
        "  kind: enum(a, b)\n",
    );
    let prompt = TemplatePrompt::create("template-schema-url", schema);
    prompt.commit();
    prompt.write_grants("allow site https://schemas.example.invalid\n");
    assert!(prompt.values("os=").is_empty(), "completion has no remote runtime and fetches nothing");
    assert_eq!(prompt.values("kind="), ["kind='a'", "kind='b'"]);
}
