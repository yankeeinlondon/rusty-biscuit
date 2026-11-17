//! The invocation's configuration on `md`'s result channels: a missing user
//! file is one `dm.config.missing` warning per run (stderr for a formatted
//! output, the envelope's `diagnostics` for a data output), never for help or a
//! usage error; a repository without `.darkmatter.yaml` is normal; a user file
//! that does not load stops a command that reads it with a located error; an
//! invalid `permissions` block refuses even a `--allow host=HOST` URL.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use simplified_schema::SimplifiedSchema;
use std::path::Path;
use std::process::Output;

const DOCUMENT: &str = "---\ntitle: T\n---\n# Head\n\nbody\n";
const MISSING: &str = "dm.config.missing";

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A fixture with `doc.md` in its launch directory and no user configuration.
fn without_user_config(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name).without_user_config();
    fixture.write_file("cwd/doc.md", DOCUMENT);
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

/// Stdout's one JSON value, judged against `schema_text`, with stderr empty.
fn data_value(output: &Output, schema_text: &str) -> Value {
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", stderr_of(output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    value
}

fn missing_notices(value: &Value) -> Vec<&Value> {
    value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["code"] == MISSING).collect()
}

/// The formatted warning names the file and the command that creates it.
fn assert_one_missing_warning(fixture: &CliProcessFixture, output: &Output) {
    let stderr = stderr_of(output);
    let path = fixture.home().join(".darkmatter.yaml").display().to_string();
    assert_eq!(stderr.matches("does not exist").count(), 1, "{stderr}");
    assert!(stderr.contains(&path) && stderr.contains("md init"), "{stderr}");
}

#[test]
fn a_composing_run_warns_once_on_stderr() {
    let fixture = without_user_config("config-missing-compose");

    let output = run(&fixture, &["compose", "doc.md"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_one_missing_warning(&fixture, &output);
}

#[test]
fn a_composing_data_output_carries_the_warning_in_its_diagnostics() {
    let fixture = without_user_config("config-missing-compose-json");

    let output = run(&fixture, &["compose", "doc.md", "--output", "json"]);

    assert!(output.status.success());
    let value = data_value(&output, include_str!("../../schemas/output/compose.yaml"));
    let notices = missing_notices(&value);
    assert_eq!(notices.len(), 1, "{value:#}");
    assert_eq!(notices[0]["severity"], "warning");
    assert_eq!(
        notices[0]["source"]["file"].as_str(),
        Some(fixture.home().join(".darkmatter.yaml").to_str().unwrap())
    );
}

#[test]
fn a_command_that_composes_nothing_warns_too() {
    let fixture = without_user_config("config-missing-hash");

    let formatted = run(&fixture, &["hash", "doc.md"]);
    let data = run(&fixture, &["hash", "doc.md", "--output", "json"]);

    assert!(formatted.status.success(), "{}", stderr_of(&formatted));
    assert_one_missing_warning(&fixture, &formatted);
    let value = data_value(&data, include_str!("../../schemas/output/hash.yaml"));
    assert_eq!(missing_notices(&value).len(), 1, "{value:#}");
}

#[test]
fn help_and_usage_errors_report_no_configuration() {
    let fixture = without_user_config("config-missing-help");

    for args in [&["--help"][..], &["compose", "--help"], &["compose", "doc.md", "--no-such-flag"]] {
        let output = run(&fixture, args);
        assert!(!stderr_of(&output).contains("does not exist"), "{args:?}: {}", stderr_of(&output));
        assert!(!String::from_utf8_lossy(&output.stdout).contains(MISSING), "{args:?}");
    }
}

#[test]
fn a_repository_without_configuration_is_normal() {
    let fixture = CliProcessFixture::named("config-repository-absent");
    fixture.write_file("cwd/doc.md", DOCUMENT);
    if !fixture.initialize_repository_at(fixture.cwd()) {
        return;
    }

    let output = run(&fixture, &["compose", "doc.md"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
}

#[test]
fn a_user_file_that_does_not_load_stops_a_command_that_reads_it() {
    let fixture = CliProcessFixture::named("config-user-broken");
    fixture.write_file("cwd/doc.md", DOCUMENT);
    let config = fixture.home().join(".darkmatter.yaml");
    // A key the closed schema does not declare fails the whole file.
    std::fs::write(&config, "register: []\nnot-a-setting: 1\n").unwrap();

    let formatted = run(&fixture, &["compose", "doc.md"]);
    let data = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    let unread = run(&fixture, &["hash", "doc.md"]);

    assert_eq!(formatted.status.code(), Some(1));
    let stderr = stderr_of(&formatted);
    assert!(stderr.contains(&format!("{}:2:", config.display())), "file and line: {stderr}");
    assert!(formatted.stdout.is_empty(), "nothing was composed");
    assert_eq!(data.status.code(), Some(1));
    let value = data_value(&data, include_str!("../../schemas/output/compose.yaml"));
    let error = &value["diagnostics"][0];
    assert_eq!(error["code"], "dm.config.invalid", "{value:#}");
    assert_eq!(error["severity"], "error");
    assert_eq!(error["source"]["file"].as_str(), Some(config.to_str().unwrap()));
    assert_eq!(error["source"]["line"], 2, "{value:#}");
    assert!(error["source"]["column"].is_u64(), "{value:#}");
    assert!(unread.status.success(), "a command that reads no configuration runs: {}", stderr_of(&unread));
}

// ── An invalid policy outranks `--allow host=HOST` ──────────────────────

/// `md compose doc.md --allow host=127.0.0.1 --output json` over a loopback
/// `::file`, with the user file `config`: the run's output and how many
/// requests reached the loopback server.
fn compose_with_exact_host(name: &str, config: &str) -> (CliProcessFixture, Output, usize) {
    let fixture = CliProcessFixture::named(name);
    std::fs::write(fixture.home().join(".darkmatter.yaml"), config).unwrap();
    let server = common::mock_http_server(vec![common::MockHttpResponse {
        status: 200,
        body: "Remote child.\n",
        cache_control: None,
    }]);
    fixture.write_file("cwd/doc.md", &format!("# Doc\n\n::file {}\n", server.url("/child.md")));
    let output = run(&fixture, &["compose", "doc.md", "--allow", "host=127.0.0.1", "--output", "json"]);
    let requests = server.request_count();
    server.shutdown();
    (fixture, output, requests)
}

/// The run failed without a request, and an error diagnostic names the
/// user file and each of `also_named` (given the fixture home).
fn assert_refused_naming(name: &str, config: &str, also_named: fn(&Path) -> Vec<String>) {
    let (fixture, output, requests) = compose_with_exact_host(name, config);
    let also_named = also_named(fixture.home());
    assert_eq!(requests, 0, "an invalid configuration requests nothing");
    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/compose.yaml"));
    let location = fixture.home().join(".darkmatter.yaml").display().to_string();
    let errors: Vec<&str> = value["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .filter(|record| record["severity"] == "error")
        .filter_map(|record| record["message"].as_str())
        .collect();
    assert!(
        errors.iter().any(|message| message.contains(&location) && also_named.iter().all(|named| message.contains(named.as_str()))),
        "no error names {location} and {also_named:?}: {value:#}"
    );
}


#[test]
fn a_missing_permissions_reference_outranks_an_exact_host() {
    assert_refused_naming("config-reference-missing-host", "permissions: ./missing.yaml\n", |home| {
        vec![home.join("missing.yaml").display().to_string()]
    });
}

#[test]
fn omitted_permissions_fetch_an_exact_host() {
    assert_fetched("config-permissions-omitted-host", "{}\n");
}

/// A `null` is the same as leaving the key out: its default applies.
#[test]
fn null_permissions_and_a_null_network_fetch_an_exact_host_as_if_omitted() {
    assert_fetched("config-permissions-null-host", "permissions: null\n");
    assert_fetched("config-network-null-host", "permissions:\n  network: null\n");
}

/// The run composed the loopback `::file` with one request.
fn assert_fetched(name: &str, config: &str) {
    let (_fixture, output, requests) = compose_with_exact_host(name, config);
    assert!(output.status.success(), "{config:?}: {}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/compose.yaml"));
    assert!(value["document"].as_str().is_some_and(|text| text.contains("Remote child.")), "{value:#}");
    assert_eq!(requests, 1, "{config:?}");
}
