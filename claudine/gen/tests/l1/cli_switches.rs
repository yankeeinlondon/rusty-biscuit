//! `cli_switches` generation: contract revision 2 switch records, the
//! whole-provider gap for older documents, and the input robustness matrix.
//!
//! Every case drives the real pipeline ([`claudine_gen::generate_for_area`])
//! over Codex's real inputs, with `docs/research/agent-cli/codex.md`
//! replaced by a revision-2 fixture derived from the committed document
//! (`tests/fixtures/agent-cli-r2/codex.md`). Each matrix case makes one edit
//! to that fixture and asserts the generated `cli_switches` value, which is
//! the value `data.rs` and `catalog.json` are emitted from.

use claudine_gen::{GenError, Generation};
use serde_json::{Value, json};

use crate::pipeline::Fixture;

const FIXTURE: &str = include_str!("../fixtures/agent-cli-r2/codex.md");
const DOC: &str = "docs/research/agent-cli/codex.md";

/// The `--config` record's typed lines, as the fixture writes them.
const CONFIG_TYPED: &str = "    value_type: string\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n";
/// The `--image` record's typed lines.
const IMAGE_TYPED: &str = "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: command\n        command: []\n      - applies_to: command\n        command: [exec]\n";
/// The `--oss` record's typed lines.
const OSS_TYPED: &str = "    value_type: none\n    attachment: []\n";

fn generate_with(document: &str) -> Result<Generation, GenError> {
    let fixture = Fixture::for_slug("codex").with_real_overrides();
    fixture.write(DOC, document);
    fixture.generate()
}

fn cli_switches(generation: &Generation) -> &Value {
    &generation
        .fields
        .iter()
        .find(|field| field.field == "cli_switches")
        .expect("cli_switches resolves")
        .value
}

/// The researched record a spelling names (canonical or alias), the lookup a
/// consumer of the generated catalog performs.
fn lookup<'v>(catalog: &'v Value, spelling: &str) -> Option<&'v Value> {
    catalog["researched"].as_array()?.iter().find(|record| {
        record["flag"] == spelling
            || record["aliases"]
                .as_array()
                .is_some_and(|aliases| aliases.iter().any(|alias| alias == spelling))
    })
}

fn edited(from: &str, to: &str) -> String {
    assert!(FIXTURE.contains(from), "fixture edit: `{from}` not found");
    FIXTURE.replacen(from, to, 1)
}

/// The unedited fixture: `-c` is Codex's `--config`, a required string
/// accepted in every form at every command path.
#[test]
fn control_row_types_dash_c_as_a_string() {
    let generation = generate_with(FIXTURE).unwrap();
    let catalog = cli_switches(&generation);
    let config = lookup(catalog, "-c").expect("-c is researched");
    assert_eq!(config["flag"], "--config");
    assert_eq!(config["value"], json!({"string": {"optional": false}}));
    assert_eq!(config["attachments"], json!(["space", "equals", "short_attached"]));
    assert_eq!(config["scopes"], json!(["global"]));
    assert_eq!(config["gap"], Value::Null);

    let image = lookup(catalog, "-i").unwrap();
    assert_eq!(image["value"], json!({"variadic": {"min": 1}}));
    // An empty command path is the root entrypoint.
    assert_eq!(image["scopes"], json!([{"command": []}, {"command": ["exec"]}]));
    // Absent aliases mean none.
    assert_eq!(lookup(catalog, "--json").unwrap()["aliases"], json!([]));
    assert_eq!(lookup(catalog, "--oss").unwrap()["value"], json!("none"));
    let remote = lookup(catalog, "--remote").unwrap();
    assert_eq!(remote["value"], json!("unknown"));
    assert!(remote["gap"].as_str().is_some_and(|gap| !gap.is_empty()));

    // The emitted Rust carries the same record.
    assert!(generation.data_rs.contains("flag: \"--config\""));
    assert!(generation.data_rs.contains("aliases: &[\"-c\"]"));
    assert!(generation.data_rs.contains("value: SwitchValue::String { optional: false }"));
    assert!(generation.data_rs.contains("value: SwitchValue::Variadic { min: VariadicMin::AtLeast(1) }"));
    assert!(generation.data_rs.contains("SwitchScope::Command(&[\"exec\"])"));
}

/// What one matrix edit must produce.
enum Expect {
    /// Generation fails (any gate may catch it).
    Rejected,
    /// Generation fails in the switch coercion itself, with an error naming
    /// this text: Darkmatter's shape check accepts the shape, so the
    /// generator is what refuses it, and for this reason.
    RejectedByGenerator(&'static str),
    /// Generation succeeds and the catalog satisfies the check.
    Accepted(fn(&Value)),
}

struct Cell {
    name: &'static str,
    from: &'static str,
    to: &'static str,
    expect: Expect,
}

fn cell(name: &'static str, from: &'static str, to: &'static str, expect: Expect) -> Cell {
    Cell {
        name,
        from,
        to,
        expect,
    }
}

/// One edit per cell of the plan's robustness table, for each load-bearing
/// field: `value_type`, `value_optional`, `variadic_min`, `aliases`,
/// `attachment`, and the invocation scope.
#[test]
fn every_matrix_cell_has_its_defined_outcome() {
    use Expect::*;
    let cells = [
        // value_type
        cell("value_type absent", "    value_type: string\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", "    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type null", CONFIG_TYPED, "    value_type: null\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type wrong type", CONFIG_TYPED, "    value_type: [string]\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type number", CONFIG_TYPED, "    value_type: 3\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type empty", CONFIG_TYPED, "    value_type: \"\"\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type not a member", CONFIG_TYPED, "    value_type: list\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_type duplicate key", CONFIG_TYPED, "    value_type: string\n    value_type: none\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        // value_optional
        cell("value_optional absent on a string", CONFIG_TYPED, "    value_type: string\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("value_optional")),
        cell("value_optional null", CONFIG_TYPED, "    value_type: string\n    value_optional: null\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("value_optional")),
        cell("value_optional wrong type", CONFIG_TYPED, "    value_type: string\n    value_optional: \"no\"\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_optional null on a none switch", OSS_TYPED, "    value_type: none\n    value_optional: null\n    attachment: []\n", RejectedByGenerator("value_optional")),
        cell("value_optional on a none switch", OSS_TYPED, "    value_type: none\n    value_optional: false\n    attachment: []\n", RejectedByGenerator("value_optional")),
        cell("value_optional duplicate key", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    value_optional: true\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_optional true", CONFIG_TYPED, "    value_type: string\n    value_optional: true\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Accepted(|c| assert_eq!(lookup(c, "-c").unwrap()["value"], json!({"string": {"optional": true}})))),
        // variadic_min
        cell("variadic_min absent", "    variadic_min: 1\n", "", RejectedByGenerator("variadic_min")),
        cell("variadic_min null", "    variadic_min: 1\n", "    variadic_min: null\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min wrong type", "    variadic_min: 1\n", "    variadic_min: \"two\"\n", Rejected),
        cell("variadic_min fraction", "    variadic_min: 1\n", "    variadic_min: 1.5\n", Rejected),
        cell("variadic_min zero", "    variadic_min: 1\n", "    variadic_min: 0\n", Rejected),
        cell("variadic_min duplicate key", "    variadic_min: 1\n", "    variadic_min: 1\n    variadic_min: 2\n", Rejected),
        cell("variadic_min on a string", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min null on a none switch", OSS_TYPED, "    value_type: none\n    variadic_min: null\n    attachment: []\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min unknown without a gap", "    variadic_min: 1\n", "    variadic_min: unknown\n", RejectedByGenerator("`gap`")),
        cell("variadic_min unknown with a gap", "    variadic_min: 1\n", "    variadic_min: unknown\n    gap: \"The reference does not say whether -i may be given with no file.\"\n", Accepted(|c| assert_eq!(lookup(c, "-i").unwrap()["value"], json!({"variadic": {"min": "unknown"}})))),
        // aliases (absent = none is asserted by the control row on --json)
        cell("aliases null", OSS_TYPED, "    aliases: null\n    value_type: none\n    attachment: []\n", RejectedByGenerator("aliases")),
        cell("aliases wrong type", "    aliases: [\"-c\"]\n", "    aliases: \"-c\"\n", Rejected),
        cell("aliases one element wrong", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", 123]\n", Rejected),
        cell("aliases every element wrong", "    aliases: [\"-c\"]\n", "    aliases: [123]\n", Rejected),
        cell("aliases empty", OSS_TYPED, "    aliases: []\n    value_type: none\n    attachment: []\n", Accepted(|c| assert_eq!(lookup(c, "--oss").unwrap()["aliases"], json!([])))),
        cell("aliases repeat a spelling", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", \"-c\"]\n", RejectedByGenerator("listed twice")),
        cell("aliases repeat the flag", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", \"--config\"]\n", RejectedByGenerator("listed twice")),
        cell("aliases duplicate key", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\"]\n    aliases: [\"-x\"]\n", Rejected),
        cell("alias claimed by two records where both apply", "    aliases: [\"-m\"]\n", "    aliases: [\"-m\", \"-c\"]\n", RejectedByGenerator("also claimed")),
        // attachment
        cell("attachment absent", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment null", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: null\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment wrong type", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: space\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment one element wrong", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: [space, glued]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment every element wrong", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: [glued]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment empty for a string", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: []\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("attachment")),
        cell("attachment set for a none switch", OSS_TYPED, "    value_type: none\n    attachment: [space]\n", RejectedByGenerator("attachment")),
        cell("attachment repeats a form", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: [space, space]\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("twice")),
        cell("short_attached without a short spelling", "    aliases: [\"-c\"]\n", "    aliases: [\"--cfg\"]\n", RejectedByGenerator("short_attached")),
        cell("attachment space only", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: [space]\n    invocation_scope:\n      - applies_to: global\n", Accepted(|c| assert_eq!(lookup(c, "-c").unwrap()["attachments"], json!(["space"])))),
        // invocation scope
        cell("scope absent", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n", Rejected),
        cell("scope null", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope: null\n", Rejected),
        cell("scope wrong type", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope: global\n", Rejected),
        cell("scope empty list", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope: []\n", RejectedByGenerator("invocation_scope")),
        cell("command path one word wrong", "        command: [exec]\n    scope: [\"interactive\", \"exec\", \"input\"]", "        command: [exec, 7]\n    scope: [\"interactive\", \"exec\", \"input\"]", RejectedByGenerator("command word")),
        cell("command path every word wrong", "        command: [exec]\n    scope: [\"interactive\", \"exec\", \"input\"]", "        command: [7]\n    scope: [\"interactive\", \"exec\", \"input\"]", RejectedByGenerator("command word")),
        cell("command path null", "        command: [exec]\n    scope: [\"interactive\", \"exec\", \"input\"]", "        command: null\n    scope: [\"interactive\", \"exec\", \"input\"]", RejectedByGenerator("command")),
        cell("command path on a global entry", "      - applies_to: global\n", "      - applies_to: global\n        command: [exec]\n", RejectedByGenerator("command")),
        cell("scope entry repeated", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: command\n        command: [exec]\n      - applies_to: command\n        command: [exec]\n", RejectedByGenerator("listed twice")),
        cell("scope duplicate key", "      - applies_to: global\n", "      - applies_to: global\n        applies_to: command\n", Rejected),
        cell("global beside a command path", "      - applies_to: global\n", "      - applies_to: global\n      - applies_to: command\n        command: [exec]\n", RejectedByGenerator("global")),
        // description
        cell("description absent", "    description: \"Use a local open-source model provider.\"\n", "", Rejected),
        cell("description empty", "    description: \"Use a local open-source model provider.\"\n", "    description: \"\"\n", Rejected),
        cell("description blank", "    description: \"Use a local open-source model provider.\"\n", "    description: \"   \"\n", Rejected),
        // the gap and the document
        cell("unknown value type without a gap", "    gap: \"The reference shows a placeholder but not whether the address may be omitted; run codex --remote with no value to check.\"", "", RejectedByGenerator("`gap`")),
        cell("gap where nothing is unknown", OSS_TYPED, "    value_type: none\n    attachment: []\n    gap: \"Nothing is unknown here.\"\n", RejectedByGenerator("`gap`")),
        cell("revision null", "schema_revision: 2\n", "schema_revision: null\n", Rejected),
        cell("revision with no contract", "schema_revision: 2\n", "schema_revision: 3\n", Rejected),
        cell("trailing invalid content", "schema_revision: 2\n", "schema_revision: 2\n: : garbage [\n", Rejected),
    ];

    let mut failures = Vec::new();
    for cell in &cells {
        let outcome = generate_with(&edited(cell.from, cell.to));
        let verdict = match (&cell.expect, &outcome) {
            (Expect::Rejected, Err(_)) => Ok(()),
            (Expect::RejectedByGenerator(needle), Err(GenError::CliSwitchInvalid { message, .. }))
                if message.contains(needle) =>
            {
                Ok(())
            }
            (Expect::RejectedByGenerator(_), Err(other)) => {
                Err(format!("expected the switch coercion to refuse it, got: {other}"))
            }
            (Expect::Accepted(check), Ok(generation)) => {
                check(cli_switches(generation));
                Ok(())
            }
            (Expect::Accepted(_), Err(err)) => Err(format!("expected success, got: {err}")),
            (_, Ok(generation)) => Err(format!(
                "expected an error, generated: {}",
                cli_switches(generation)
            )),
        };
        if let Err(message) = verdict {
            failures.push(format!("{}: {message}", cell.name));
        }
    }
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));
}

/// Record order in the research document never changes the output.
#[test]
fn output_order_does_not_depend_on_document_order() {
    let forward = generate_with(FIXTURE).unwrap();
    // Move the --config record after --remote.
    let start = FIXTURE.find("  - flag: --config\n").unwrap();
    let end = FIXTURE.find("  - flag: --image\n").unwrap();
    let config = &FIXTURE[start..end];
    let without = format!("{}{}", &FIXTURE[..start], &FIXTURE[end..]);
    let tail = without.find("config_paths:").unwrap();
    let reordered = format!("{}{}{}", &without[..tail], config, &without[tail..]);
    let reordered = generate_with(&reordered).unwrap();
    assert_eq!(cli_switches(&forward), cli_switches(&reordered));
    assert_eq!(forward.data_rs, reordered.data_rs);
    let flags: Vec<&str> = cli_switches(&forward)["researched"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["flag"].as_str().unwrap())
        .collect();
    let mut sorted = flags.clone();
    sorted.sort_unstable();
    assert_eq!(flags, sorted);
}

/// One spelling may name two records when no command path accepts both.
#[test]
fn a_spelling_may_repeat_at_disjoint_command_paths() {
    let mcp_json = "  - flag: --json\n    value_type: none\n    attachment: []\n    invocation_scope:\n      - applies_to: command\n        command: [mcp, list]\n    description: \"Print the server list as JSON.\"\n    evidence_ids: [codex-cli-reference]\n";
    let document = edited("  - flag: --remote\n", &format!("{mcp_json}  - flag: --remote\n"));
    let generation = generate_with(&document).unwrap();
    let json_records: Vec<&Value> = cli_switches(&generation)["researched"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["flag"] == "--json")
        .collect();
    assert_eq!(json_records.len(), 2);

    // The same record at a path both accept is a conflict.
    let clash = edited(
        "  - flag: --remote\n",
        &format!("{}  - flag: --remote\n", mcp_json.replace("command: [mcp, list]", "command: [exec]")),
    );
    assert!(matches!(generate_with(&clash), Err(GenError::CliSwitchInvalid { .. })));
}

/// An empty inventory is a gap, never "no switches": it needs a reason.
#[test]
fn an_empty_inventory_needs_a_stated_gap() {
    let start = FIXTURE.find("cli_switches:\n").unwrap();
    let end = FIXTURE.find("config_paths:").unwrap();
    let gap = "The CLI prints no help without a login; run codex --help after signing in.";
    let with_gap = format!(
        "{}cli_switches: []\ncli_switches_gap: \"{gap}\"\n{}",
        &FIXTURE[..start],
        &FIXTURE[end..]
    );
    let generation = generate_with(&with_gap).unwrap();
    assert_eq!(cli_switches(&generation), &json!({"unknown": {"gap": gap}}));

    let without_gap = format!("{}cli_switches: []\n{}", &FIXTURE[..start], &FIXTURE[end..]);
    assert!(matches!(generate_with(&without_gap), Err(GenError::CliSwitchInvalid { .. })));

    let gap_beside_records = edited("schema_revision: 2\n", &format!("schema_revision: 2\ncli_switches_gap: \"{gap}\"\n"));
    assert!(matches!(generate_with(&gap_beside_records), Err(GenError::CliSwitchInvalid { .. })));
}

/// A document written before revision 2 validates against the frozen
/// revision-1 contract and generates an explicit gap. Every committed
/// document is now revision 2, so the last revision-1 Codex document and its
/// contract are kept as fixtures to prove the mechanism.
#[test]
fn a_revision_one_document_generates_an_explicit_gap() {
    let fixture = Fixture::for_slug("codex").with_real_overrides();
    fixture.write(DOC, include_str!("../fixtures/agent-cli-r1/codex.md"));
    fixture.write(
        "docs/research/agent-cli/_schema.r1.yaml",
        include_str!("../fixtures/agent-cli-r1/_schema.r1.yaml"),
    );
    let generation = fixture.generate().unwrap();
    let gap = cli_switches(&generation)["unknown"]["gap"].as_str().unwrap();
    assert!(gap.contains("revision 2"), "{gap}");
    assert!(generation.data_rs.contains("cli_switches: CliSwitchCatalog::Unknown {"));

    // Without the frozen contract the document cannot be validated at all.
    std::fs::remove_file(fixture.path("docs/research/agent-cli/_schema.r1.yaml")).unwrap();
    match fixture.generate() {
        Err(GenError::ResearchRevisionUnsupported { found, current, .. }) => {
            assert_eq!((found, current), (1, 2));
        }
        other => panic!("expected ResearchRevisionUnsupported, got {other:?}"),
    }
}

/// The schema gate refuses a contract that stops declaring a field the
/// coercion reads.
#[test]
fn the_gate_requires_every_field_the_coercion_reads() {
    let fixture = Fixture::for_slug("codex").with_real_overrides();
    let types = fixture.path("docs/research/agent-cli/_types.yaml");
    let text = std::fs::read_to_string(&types).unwrap();
    let without_gap: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("gap:"))
        .map(|line| format!("{line}\n"))
        .collect();
    std::fs::write(&types, without_gap).unwrap();
    match fixture.generate() {
        Err(GenError::SchemaIncompatible { field, found, .. }) => {
            assert_eq!(field, "cli_switches");
            assert!(found.contains("gap"), "{found}");
        }
        other => panic!("expected SchemaIncompatible, got {other:?}"),
    }
}

/// Every compiled provider has switch records or an explicit gap.
#[test]
fn every_compiled_provider_has_switches_or_a_gap() {
    let area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf();
    for generation in claudine_gen::generate_all(&area).unwrap() {
        let catalog = cli_switches(&generation);
        let researched = catalog["researched"].as_array().is_some_and(|records| !records.is_empty());
        let gap = catalog["unknown"]["gap"].as_str().is_some_and(|gap| !gap.trim().is_empty());
        assert!(researched || gap, "{}: {catalog}", generation.slug);
    }
}
