//! `cli_switches` generation: contract revision 2 switch records, the
//! whole-provider gap for older documents, and the input robustness matrix.
//!
//! Every case drives the real pipeline ([`claudine_gen::generate_for_area`])
//! over Codex's real inputs, with `docs/research/agent-cli/codex.md`
//! replaced by a revision-2 fixture derived from the committed document
//! (`tests/fixtures/agent-cli-r2/codex.md`). Each matrix case makes one edit
//! to that fixture and asserts the generated `cli_switches` value, which is
//! the value `data.rs` and `catalog.json` are emitted from.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use claudine_gen::{GenError, Generation};
use serde_json::{Value, json};

use crate::pipeline::Fixture;

const FIXTURE: &str = include_str!("../fixtures/agent-cli-r2/codex.md");
const DOC: &str = "docs/research/agent-cli/codex.md";
const LEGACY_CONTRACT: &str = "docs/research/agent-cli/_schema.r1.yaml";

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

fn generate_cell(cell: &Cell) -> Result<Generation, GenError> {
    let fixture = Fixture::for_slug("codex").with_real_overrides();
    fixture.write(DOC, &edited(cell.from, &cell.to));
    if cell.legacy_contract {
        fixture.write(LEGACY_CONTRACT, include_str!("../fixtures/agent-cli-r1/_schema.r1.yaml"));
    }
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
    /// Generation fails because no contract exists for the document's
    /// revision.
    Unsupported,
    /// Generation fails because the contract the document's revision names
    /// refuses it.
    RejectedByContract,
    /// Generation fails because the frontmatter is not valid YAML.
    Unparsable,
    /// Generation succeeds and the catalog satisfies the check.
    Accepted(fn(&Value)),
}

struct Cell {
    name: &'static str,
    from: &'static str,
    to: String,
    expect: Expect,
    /// Also write the frozen revision-1 contract beside the document.
    legacy_contract: bool,
}

fn cell(name: &'static str, from: &'static str, to: impl Into<String>, expect: Expect) -> Cell {
    Cell {
        name,
        from,
        to: to.into(),
        expect,
        legacy_contract: false,
    }
}

impl Cell {
    fn with_legacy_contract(mut self) -> Self {
        self.legacy_contract = true;
        self
    }
}

/// The whole `cli_switches` list as the fixture writes it.
fn switch_list() -> &'static str {
    let start = FIXTURE.find("cli_switches:\n").expect("fixture has cli_switches");
    let end = FIXTURE.find("config_paths:").expect("fixture has config_paths");
    &FIXTURE[start..end]
}

/// The `--remote` record's gap line.
const REMOTE_GAP: &str = "    gap: \"The reference shows a placeholder but not whether the address may be omitted; run codex --remote with no value to check.\"";
/// The `--oss` record's description line.
const OSS_DESCRIPTION: &str = "    description: \"Use a local open-source model provider.\"\n";
/// The `--json` record's scope entry.
const JSON_SCOPE: &str = "      - applies_to: command\n        command: [exec]\n    scope: [\"exec\", \"output\"]";
/// The `--image` record's second scope entry.
const IMAGE_EXEC_SCOPE: &str = "      - applies_to: command\n        command: [exec]\n    scope: [\"interactive\", \"exec\", \"input\"]";
/// The `--config` record's spellings and typed lines up to its scope.
const CONFIG_HEAD: &str = "    aliases: [\"-c\"]\n    value: \"<key=value>\"\n    value_type: string\n    value_optional: false\n    attachment: [space, equals, short_attached]\n";
/// The `--model` record's typed lines and first scope entry.
const MODEL_TYPED: &str = "    value_type: string\n    value_optional: false\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: command\n        command: []\n";
const GAP_TEXT: &str = "The CLI prints no help without a login; run codex --help after signing in.";

/// One edit per cell of the robustness table, for every field that changes
/// the generated catalog or whether generation succeeds: the document's
/// `schema_revision`, `cli_switches`, and `cli_switches_gap`, and each
/// record's `flag`, `aliases`, `value_type`, `value_optional`,
/// `variadic_min`, `attachment`, `invocation_scope`, `applies_to`,
/// `command`, `description`, and `gap`. The shapes are absent, null, wrong
/// type, one or every element wrong, empty, duplicate key, and invalid
/// trailing content on the field's line; a shape that cannot occur for a
/// field (an element of a scalar) has no cell. Shapes whose outcome depends on another field (removing the only
/// short alias, a second root path) get a refusing cell and a valid control.
#[test]
fn every_matrix_cell_has_its_defined_outcome() {
    use Expect::*;
    let cells = [
        // schema_revision: absent means revision 1, which needs its frozen
        // contract; with it, that contract judges the document (a revision-2
        // body fails it). The legacy positive control is
        // `a_revision_one_document_generates_an_explicit_gap`.
        cell("schema_revision absent without its legacy contract", "schema_revision: 2\n", "", Unsupported),
        cell("schema_revision absent, judged by its legacy contract", "schema_revision: 2\n", "", RejectedByContract).with_legacy_contract(),
        cell("schema_revision 1, judged by its legacy contract", "schema_revision: 2\n", "schema_revision: 1\n", RejectedByContract).with_legacy_contract(),
        cell("schema_revision null", "schema_revision: 2\n", "schema_revision: null\n", Rejected),
        cell("schema_revision wrong type", "schema_revision: 2\n", "schema_revision: {revision: 2}\n", Rejected),
        cell("schema_revision as text", "schema_revision: 2\n", "schema_revision: \"2\"\n", RejectedByGenerator("`schema_revision` is `\"2\"`")),
        cell("schema_revision empty", "schema_revision: 2\n", "schema_revision: \"\"\n", Rejected),
        cell("schema_revision zero", "schema_revision: 2\n", "schema_revision: 0\n", Unsupported),
        cell("schema_revision with no contract", "schema_revision: 2\n", "schema_revision: 3\n", Unsupported),
        cell("schema_revision duplicate key", "schema_revision: 2\n", "schema_revision: 2\nschema_revision: 2\n", Rejected),
        // cli_switches (present with records is the control row)
        cell("cli_switches absent", switch_list(), "", Rejected),
        cell("cli_switches null", switch_list(), "cli_switches: null\n", Rejected),
        cell("cli_switches wrong type", switch_list(), "cli_switches: 7\n", Rejected),
        cell("cli_switches one record wrong", "cli_switches:\n  - flag: --config\n", "cli_switches:\n  - 7\n  - flag: --config\n", Rejected),
        cell("cli_switches every record wrong", switch_list(), "cli_switches: [7, \"--config\"]\n", Rejected),
        cell("cli_switches empty without a gap", switch_list(), "cli_switches: []\n", RejectedByGenerator("`cli_switches_gap` must say why")),
        cell("cli_switches empty with a gap", switch_list(), format!("cli_switches: []\ncli_switches_gap: \"{GAP_TEXT}\"\n"), Accepted(|c| assert_eq!(c, &json!({"unknown": {"gap": GAP_TEXT}})))),
        cell("cli_switches duplicate key", "cli_switches:\n", "cli_switches: []\ncli_switches:\n", Rejected),
        // cli_switches_gap (absent beside records is the control row; absent
        // beside an empty list is "cli_switches empty without a gap")
        cell("cli_switches_gap null", switch_list(), "cli_switches: []\ncli_switches_gap: null\n", RejectedByGenerator("`cli_switches_gap` is null")),
        cell("cli_switches_gap wrong type", switch_list(), "cli_switches: []\ncli_switches_gap: 7\n", RejectedByGenerator("`cli_switches_gap` must be non-empty text")),
        cell("cli_switches_gap empty", switch_list(), "cli_switches: []\ncli_switches_gap: \"\"\n", Rejected),
        cell("cli_switches_gap blank", switch_list(), "cli_switches: []\ncli_switches_gap: \"   \"\n", Rejected),
        cell("cli_switches_gap duplicate key", switch_list(), format!("cli_switches: []\ncli_switches_gap: \"{GAP_TEXT}\"\ncli_switches_gap: \"Another reason.\"\n"), Rejected),
        cell("cli_switches_gap beside records", "schema_revision: 2\n", format!("schema_revision: 2\ncli_switches_gap: \"{GAP_TEXT}\"\n"), RejectedByGenerator("`cli_switches` has records")),
        // flag
        cell("flag absent", "  - flag: --oss\n    value_type: none\n", "  - value_type: none\n", Rejected),
        cell("flag null", "  - flag: --oss\n", "  - flag: null\n", Rejected),
        cell("flag wrong type", "  - flag: --oss\n", "  - flag: 7\n", Rejected),
        cell("flag empty", "  - flag: --oss\n", "  - flag: \"\"\n", Rejected),
        cell("flag not a spelling", "  - flag: --oss\n", "  - flag: oss\n", Rejected),
        cell("flag duplicate key", "  - flag: --oss\n", "  - flag: --oss\n    flag: --local\n", Rejected),
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
        cell("value_optional empty", CONFIG_TYPED, "    value_type: string\n    value_optional: \"\"\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("value_optional true", CONFIG_TYPED, "    value_type: string\n    value_optional: true\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", Accepted(|c| assert_eq!(lookup(c, "-c").unwrap()["value"], json!({"string": {"optional": true}})))),
        // variadic_min
        cell("variadic_min absent", "    variadic_min: 1\n", "", RejectedByGenerator("variadic_min")),
        cell("variadic_min null", "    variadic_min: 1\n", "    variadic_min: null\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min wrong type", "    variadic_min: 1\n", "    variadic_min: \"two\"\n", Rejected),
        cell("variadic_min fraction", "    variadic_min: 1\n", "    variadic_min: 1.5\n", Rejected),
        cell("variadic_min zero", "    variadic_min: 1\n", "    variadic_min: 0\n", Rejected),
        cell("variadic_min duplicate key", "    variadic_min: 1\n", "    variadic_min: 1\n    variadic_min: 2\n", Rejected),
        cell("variadic_min empty", "    variadic_min: 1\n", "    variadic_min: \"\"\n", Rejected),
        cell("variadic_min on a string", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope:\n      - applies_to: global\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min null on a none switch", OSS_TYPED, "    value_type: none\n    variadic_min: null\n    attachment: []\n", RejectedByGenerator("variadic_min")),
        cell("variadic_min boolean", "    variadic_min: 1\n", "    variadic_min: true\n", Rejected),
        cell("variadic_min list", "    variadic_min: 1\n", "    variadic_min: [1]\n", Rejected),
        cell("variadic_min mapping", "    variadic_min: 1\n", "    variadic_min: {min: 1}\n", Rejected),
        cell("variadic_min large", "    variadic_min: 1\n", "    variadic_min: 123\n", Accepted(|c| assert_eq!(lookup(c, "-i").unwrap()["value"], json!({"variadic": {"min": 123}})))),
        cell("variadic_min unknown without a gap", "    variadic_min: 1\n", "    variadic_min: unknown\n", RejectedByGenerator("`gap`")),
        cell("variadic_min unknown with a gap", "    variadic_min: 1\n", "    variadic_min: unknown\n    gap: \"The reference does not say whether -i may be given with no file.\"\n", Accepted(|c| assert_eq!(lookup(c, "-i").unwrap()["value"], json!({"variadic": {"min": "unknown"}})))),
        // aliases (absent = none is asserted by the control row on --json)
        cell("aliases null", OSS_TYPED, "    aliases: null\n    value_type: none\n    attachment: []\n", RejectedByGenerator("aliases")),
        cell("aliases wrong type", "    aliases: [\"-c\"]\n", "    aliases: \"-c\"\n", Rejected),
        cell("aliases one element wrong", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", 123]\n", Rejected),
        cell("aliases every element wrong", "    aliases: [\"-c\"]\n", "    aliases: [123]\n", Rejected),
        cell("aliases empty", OSS_TYPED, "    aliases: []\n    value_type: none\n    attachment: []\n", Accepted(|c| assert_eq!(lookup(c, "--oss").unwrap()["aliases"], json!([])))),
        cell("aliases empty beside short_attached", "    aliases: [\"-c\"]\n", "    aliases: []\n", RejectedByGenerator("short_attached")),
        cell("aliases repeat a spelling", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", \"-c\"]\n", RejectedByGenerator("listed twice")),
        cell("aliases repeat the flag", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\", \"--config\"]\n", RejectedByGenerator("listed twice")),
        cell("aliases duplicate key", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\"]\n    aliases: [\"-x\"]\n", Rejected),
        cell("alias claimed by two records where both apply", "    aliases: [\"-m\"]\n", "    aliases: [\"-m\", \"-c\"]\n", RejectedByGenerator("also claimed")),
        // Removing the only short spelling fails while short_attached remains;
        // the valid control drops both together.
        cell("aliases absent beside short_attached", CONFIG_HEAD, "    value: \"<key=value>\"\n    value_type: string\n    value_optional: false\n    attachment: [space, equals, short_attached]\n", RejectedByGenerator("short_attached")),
        cell("aliases absent without short_attached", CONFIG_HEAD, "    value: \"<key=value>\"\n    value_type: string\n    value_optional: false\n    attachment: [space, equals]\n", Accepted(|c| {
            let config = lookup(c, "--config").unwrap();
            assert_eq!(config["aliases"], json!([]));
            assert_eq!(config["attachments"], json!(["space", "equals"]));
            assert!(lookup(c, "-c").is_none());
        })),
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
        cell("attachment duplicate key", CONFIG_TYPED, "    value_type: string\n    value_optional: false\n    attachment: [space]\n    attachment: [equals]\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        cell("attachment empty for a none switch", MODEL_TYPED, "    value_type: none\n    attachment: []\n    invocation_scope:\n      - applies_to: command\n        command: []\n", Accepted(|c| {
            let model = lookup(c, "-m").unwrap();
            assert_eq!(model["value"], json!("none"));
            assert_eq!(model["attachments"], json!([]));
        })),
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
        cell("applies_to duplicate key", "      - applies_to: global\n", "      - applies_to: global\n        applies_to: command\n", Rejected),
        cell("global beside a command path", "      - applies_to: global\n", "      - applies_to: global\n      - applies_to: command\n        command: [exec]\n", RejectedByGenerator("global")),
        cell("scope one entry wrong", IMAGE_EXEC_SCOPE, "      - applies_to: command\n        command: [exec]\n      - global\n    scope: [\"interactive\", \"exec\", \"input\"]", Rejected),
        cell("scope every entry wrong", IMAGE_TYPED, "    value_type: variadic\n    variadic_min: 1\n    attachment: [space, equals, short_attached]\n    invocation_scope: [global, exec]\n", Rejected),
        cell("scope duplicate key", "    invocation_scope:\n      - applies_to: global\n", "    invocation_scope:\n      - applies_to: global\n    invocation_scope:\n      - applies_to: global\n", Rejected),
        // applies_to
        cell("applies_to absent", JSON_SCOPE, "      - command: [exec]\n    scope: [\"exec\", \"output\"]", Rejected),
        cell("applies_to null", "      - applies_to: global\n", "      - applies_to: null\n", Rejected),
        cell("applies_to wrong type", "      - applies_to: global\n", "      - applies_to: [global]\n", Rejected),
        cell("applies_to empty", "      - applies_to: global\n", "      - applies_to: \"\"\n", Rejected),
        cell("applies_to not a member", "      - applies_to: global\n", "      - applies_to: everywhere\n", Rejected),
        // command (an empty path is the root, asserted by the control row and
        // here; a second root is a repeated scope, not a refusal of roots)
        cell("command absent on a command entry", JSON_SCOPE, "      - applies_to: command\n    scope: [\"exec\", \"output\"]", RejectedByGenerator("`command` is required")),
        cell("command wrong type", JSON_SCOPE, "      - applies_to: command\n        command: exec\n    scope: [\"exec\", \"output\"]", Rejected),
        cell("command empty is the root", JSON_SCOPE, "      - applies_to: command\n        command: []\n    scope: [\"exec\", \"output\"]", Accepted(|c| assert_eq!(lookup(c, "--json").unwrap()["scopes"], json!([{"command": []}])))),
        cell("command empty beside another root", IMAGE_EXEC_SCOPE, "      - applies_to: command\n        command: []\n    scope: [\"interactive\", \"exec\", \"input\"]", RejectedByGenerator("listed twice")),
        cell("command duplicate key", JSON_SCOPE, "      - applies_to: command\n        command: [exec]\n        command: [exec, resume]\n    scope: [\"exec\", \"output\"]", Rejected),
        // description
        cell("description absent", "    description: \"Use a local open-source model provider.\"\n", "", Rejected),
        cell("description empty", "    description: \"Use a local open-source model provider.\"\n", "    description: \"\"\n", Rejected),
        cell("description blank", "    description: \"Use a local open-source model provider.\"\n", "    description: \"   \"\n", Rejected),
        cell("description null", OSS_DESCRIPTION, "    description: null\n", Rejected),
        cell("description wrong type", OSS_DESCRIPTION, "    description: 7\n", RejectedByGenerator("`description` must be non-empty text")),
        cell("description duplicate key", OSS_DESCRIPTION, "    description: \"Use a local open-source model provider.\"\n    description: \"Something else.\"\n", Rejected),
        // gap (absent where nothing is unknown is the control row)
        cell("unknown value type without a gap", "    gap: \"The reference shows a placeholder but not whether the address may be omitted; run codex --remote with no value to check.\"", "", RejectedByGenerator("`gap`")),
        cell("gap where nothing is unknown", OSS_TYPED, "    value_type: none\n    attachment: []\n    gap: \"Nothing is unknown here.\"\n", RejectedByGenerator("`gap`")),
        cell("gap null", REMOTE_GAP, "    gap: null", RejectedByGenerator("`gap` is null")),
        cell("gap wrong type", REMOTE_GAP, "    gap: 7", RejectedByGenerator("`gap` must be non-empty text")),
        cell("gap empty", REMOTE_GAP, "    gap: \"\"", Rejected),
        cell("gap blank", REMOTE_GAP, "    gap: \"   \"", Rejected),
        cell("gap duplicate key", REMOTE_GAP, format!("{REMOTE_GAP}\n    gap: \"Another gap.\""), Rejected),
        // the document
        cell("trailing invalid content", "schema_revision: 2\n", "schema_revision: 2\n: : garbage [\n", Rejected),
        // Invalid trailing content on each field's own line: the frontmatter
        // does not parse, whatever the field.
        cell("schema_revision trailing content", "schema_revision: 2\n", "schema_revision: 2: x\n", Unparsable),
        cell("cli_switches trailing content", "cli_switches:\n", "cli_switches: []: x\n", Unparsable),
        cell("cli_switches_gap trailing content", switch_list(), format!("cli_switches: []\ncli_switches_gap: \"{GAP_TEXT}\": x\n"), Unparsable),
        cell("flag trailing content", "  - flag: --oss\n", "  - flag: --oss: x\n", Unparsable),
        cell("aliases trailing content", "    aliases: [\"-c\"]\n", "    aliases: [\"-c\"]: x\n", Unparsable),
        cell("value_type trailing content", "    value_type: string\n", "    value_type: string: x\n", Unparsable),
        cell("value_optional trailing content", "    value_optional: false\n", "    value_optional: false: x\n", Unparsable),
        cell("variadic_min trailing content", "    variadic_min: 1\n", "    variadic_min: 1: x\n", Unparsable),
        cell("attachment trailing content", "    attachment: [space, equals, short_attached]\n", "    attachment: [space, equals, short_attached]: x\n", Unparsable),
        cell("invocation_scope trailing content", "    invocation_scope:\n      - applies_to: global\n", "    invocation_scope: x: y\n      - applies_to: global\n", Unparsable),
        cell("applies_to trailing content", "      - applies_to: global\n", "      - applies_to: global: x\n", Unparsable),
        cell("command trailing content", "        command: [exec]\n", "        command: [exec]: x\n", Unparsable),
        cell("description trailing content", OSS_DESCRIPTION, "    description: \"Use a local open-source model provider.\": x\n", Unparsable),
        cell("gap trailing content", REMOTE_GAP, format!("{REMOTE_GAP}: x"), Unparsable),
    ];

    run_matrix(&cells, |cell| cell.name.to_string(), judge);
}

/// Judges every cell across threads, since each generates a whole provider,
/// and fails listing every cell whose outcome differed.
fn run_matrix<C: Sync>(
    cells: &[C],
    name: impl Fn(&C) -> String + Sync,
    judge: impl Fn(&C) -> Result<(), String> + Sync,
) {
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(cell) = cells.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(message) = judge(cell) {
                        failures.lock().unwrap().push(format!("{}: {message}", name(cell)));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    assert!(failures.is_empty(), "matrix cells failed:\n{}", failures.join("\n"));
}

fn judge(cell: &Cell) -> Result<(), String> {
    let outcome = generate_cell(cell);
    match (&cell.expect, &outcome) {
        (Expect::Rejected, Err(_)) => Ok(()),
        (Expect::RejectedByGenerator(needle), Err(GenError::CliSwitchInvalid { message, .. }))
            if message.contains(needle) =>
        {
            Ok(())
        }
        (Expect::RejectedByGenerator(_), Err(other)) => {
            Err(format!("expected the switch coercion to refuse it, got: {other}"))
        }
        (Expect::Unsupported, Err(GenError::ResearchRevisionUnsupported { .. }))
        | (Expect::RejectedByContract, Err(GenError::ResearchInvalid { .. }))
        | (Expect::Unparsable, Err(GenError::Markdown { .. })) => Ok(()),
        (Expect::Unsupported | Expect::RejectedByContract | Expect::Unparsable, Err(other)) => {
            Err(format!("refused for another reason: {other}"))
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
    }
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
    let snapshot = darkmatter::markdown::compose::RequestSnapshot::new(&area);
    for generation in claudine_gen::generate_all(&area, &snapshot).unwrap() {
        let catalog = cli_switches(&generation);
        let researched = catalog["researched"].as_array().is_some_and(|records| !records.is_empty());
        let gap = catalog["unknown"]["gap"].as_str().is_some_and(|gap| !gap.trim().is_empty());
        assert!(researched || gap, "{}: {catalog}", generation.slug);
    }
}

/// The override input path: `docs/providers/overrides/codex.yaml` with a
/// `cli_switches` entry written in the generated catalog shape. Every record
/// field is one line whose value is JSON (YAML's flow form), so a cell edits
/// exactly one line and the file stays readable YAML.
#[derive(Clone)]
struct OverrideDoc {
    /// Replaces the whole entry body (`cli_switches: <raw>`).
    entry_raw: Option<String>,
    reason: Option<String>,
    value: OverrideValue,
    /// `(from, to)` replacements applied to the rendered file.
    text_edits: Vec<(&'static str, &'static str)>,
    /// Lines appended after the entry.
    appended: String,
}

#[derive(Clone)]
enum OverrideValue {
    Absent,
    /// The whole `value:` as flow YAML.
    Text(String),
    Records(Vec<OverrideRecord>),
}

#[derive(Clone)]
enum OverrideRecord {
    /// `(key, flow YAML)` lines in order.
    Fields(Vec<(String, String)>),
    /// A record that is not a mapping.
    Raw(String),
}

/// The catalog record's keys, in the order the matrix writes them.
const RECORD_KEYS: [&str; 7] = ["flag", "aliases", "value", "attachments", "scopes", "description", "gap"];
const CONFIG: &str = "--config";
const IMAGE: &str = "--image";
const JSON_FLAG: &str = "--json";
const MODEL: &str = "--model";
const OSS: &str = "--oss";
const REMOTE: &str = "--remote";
const WHOLE_GAP: &str = r#"{"unknown": {"gap": "Not yet researched."}}"#;

impl OverrideDoc {
    /// The unedited fixture's research projection, as an override.
    fn from_catalog(catalog: &Value) -> Self {
        let records = catalog["researched"]
            .as_array()
            .expect("the fixture is researched")
            .iter()
            .map(|record| {
                let fields = record.as_object().unwrap();
                assert_eq!(fields.len(), RECORD_KEYS.len(), "catalog record keys: {record}");
                OverrideRecord::Fields(
                    RECORD_KEYS
                        .iter()
                        .map(|key| ((*key).to_string(), fields[*key].to_string()))
                        .collect(),
                )
            })
            .collect();
        Self {
            entry_raw: None,
            reason: Some("\"Matrix case.\"".into()),
            value: OverrideValue::Records(records),
            text_edits: Vec::new(),
            appended: String::new(),
        }
    }

    fn render(&self) -> String {
        let mut text = String::from("cli_switches:");
        if let Some(raw) = &self.entry_raw {
            text.push_str(&format!(" {raw}\n"));
        } else {
            text.push('\n');
            if let Some(reason) = &self.reason {
                text.push_str(&format!("    reason: {reason}\n"));
            }
            match &self.value {
                OverrideValue::Absent => {}
                OverrideValue::Text(value) => text.push_str(&format!("    value: {value}\n")),
                OverrideValue::Records(records) => {
                    text.push_str("    value:\n        researched:\n");
                    for record in records {
                        match record {
                            OverrideRecord::Raw(raw) => text.push_str(&format!("            - {raw}\n")),
                            OverrideRecord::Fields(fields) => {
                                for (index, (key, value)) in fields.iter().enumerate() {
                                    let lead = if index == 0 { "            - " } else { "              " };
                                    text.push_str(&format!("{lead}{key}: {value}\n"));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (from, to) in &self.text_edits {
            assert!(text.contains(from), "override edit: `{from}` not found in\n{text}");
            text = text.replacen(from, to, 1);
        }
        text.push_str(&self.appended);
        text
    }

    fn records(&mut self) -> &mut Vec<OverrideRecord> {
        match &mut self.value {
            OverrideValue::Records(records) => records,
            _ => panic!("the cell replaced the records"),
        }
    }

    fn position(&mut self, flag: &str) -> usize {
        let quoted = Value::from(flag).to_string();
        self.records()
            .iter()
            .position(|record| match record {
                OverrideRecord::Fields(fields) => fields.iter().any(|(k, v)| k == "flag" && *v == quoted),
                OverrideRecord::Raw(_) => false,
            })
            .unwrap_or_else(|| panic!("no record `{flag}`"))
    }

    fn fields(&mut self, flag: &str) -> &mut Vec<(String, String)> {
        let at = self.position(flag);
        match &mut self.records()[at] {
            OverrideRecord::Fields(fields) => fields,
            OverrideRecord::Raw(_) => unreachable!(),
        }
    }

    /// Replaces a field's value, or adds the field at the end.
    fn set(&mut self, flag: &str, key: &str, value: &str) {
        let fields = self.fields(flag);
        match fields.iter_mut().find(|(k, _)| k == key) {
            Some(field) => field.1 = value.to_string(),
            None => fields.push((key.to_string(), value.to_string())),
        }
    }

    fn remove(&mut self, flag: &str, key: &str) {
        let fields = self.fields(flag);
        let before = fields.len();
        fields.retain(|(k, _)| k != key);
        assert_ne!(before, fields.len(), "no field `{key}` on `{flag}`");
    }

    /// Writes the key a second time, with `value`, right after the first.
    fn repeat(&mut self, flag: &str, key: &str, value: &str) {
        let fields = self.fields(flag);
        let at = fields.iter().position(|(k, _)| k == key).expect("the field exists");
        fields.insert(at + 1, (key.to_string(), value.to_string()));
    }

    /// Appends content YAML cannot read after the field's closing quote or
    /// bracket.
    fn trail(&mut self, flag: &str, key: &str) {
        let fields = self.fields(flag);
        let field = fields.iter_mut().find(|(k, _)| k == key).expect("the field exists");
        assert!(field.1.ends_with(['"', ']', '}']), "`{key}` is not quoted or bracketed");
        field.1.push_str(" x");
    }

    fn raw_record(&mut self, flag: &str, raw: &str) {
        let at = self.position(flag);
        self.records()[at] = OverrideRecord::Raw(raw.to_string());
    }

    fn whole(&mut self, value: &str) {
        self.value = OverrideValue::Text(value.to_string());
    }

    fn text(&mut self, from: &'static str, to: &'static str) {
        self.text_edits.push((from, to));
    }

    /// The `value:` the file holds, as generation reads it.
    fn parsed_value(&self) -> Value {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&self.render()).unwrap();
        serde_json::to_value(yaml).unwrap()["cli_switches"]["value"].clone()
    }
}

/// The override matrix's shapes; the grid shapes are listed in [`SHAPES`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Absent,
    Null,
    WrongType,
    OneBad,
    EveryBad,
    Empty,
    DuplicateKey,
    Trailing,
    /// A control or a rule outside the grid.
    Also(&'static str),
}

const SHAPES: [Shape; 8] = [
    Shape::Absent,
    Shape::Null,
    Shape::WrongType,
    Shape::OneBad,
    Shape::EveryBad,
    Shape::Empty,
    Shape::DuplicateKey,
    Shape::Trailing,
];

/// What generation does with each grid shape of each override column, in
/// [`SHAPES`] order: `R` refuses, `A` accepts and keeps the meaning, `-` the
/// shape cannot occur (an element of a scalar). The matrix must hold exactly
/// one agreeing cell for each `R` and `A`; `docs/topics/provider-metadata.md`
/// publishes the same table.
const OVERRIDE_OUTCOMES: &[(&str, &str)] = &[
    //                absent null wrong one every empty dup trailing
    ("entry value",   "R R R - - R R R"),
    ("entry reason",  "R R R - - R R R"),
    ("researched",    "R R R R R R R R"),
    ("unknown",       "R R R - - R R R"),
    ("unknown.gap",   "R R R - - R R R"),
    ("flag",          "R R R - - R R R"),
    ("aliases",       "R R R R R R R R"),
    ("value",         "R R R - - R R R"),
    ("optional",      "R R R - - R R R"),
    ("min",           "R R R - - R R R"),
    ("attachments",   "R R R R R R R R"),
    ("scopes",        "R R R R R R R R"),
    ("command",       "R R R R R A R R"),
    ("description",   "R R R - - R R R"),
    ("gap",           "R R R - - R R R"),
];

/// Where a refusal comes from.
#[derive(Debug)]
enum Gate {
    /// The overrides file is not valid YAML (`GenError::Yaml`).
    Loader,
    /// The override entry lacks `value:`/`reason:` or has another key.
    Envelope,
    /// The switch rules refuse the value, with a message naming this text.
    Rule(&'static str),
}

#[derive(Debug)]
enum Want {
    Reject(Gate),
    /// Generation succeeds and emits exactly the override's value.
    Accept,
}

struct OverrideCell {
    column: &'static str,
    shape: Shape,
    edit: fn(&mut OverrideDoc),
    want: Want,
}

fn row(column: &'static str, shape: Shape, edit: fn(&mut OverrideDoc), want: Want) -> OverrideCell {
    OverrideCell { column, shape, edit, want }
}

/// An override reaches emission without the coercion, so the same rules
/// judge it: it must be the canonical projection of valid records. Each cell
/// makes one edit to the unedited fixture's projection written as an
/// override and generates Codex through the pipeline `claudine-gen
/// validate` runs; the `outer` column holds the controls, the outer
/// discriminators, extra members, and canonical ordering.
#[test]
fn every_override_matrix_cell_has_its_defined_outcome() {
    use Gate::*;
    use Shape::*;
    use Want::*;
    let cells = [
        // outer: controls
        row("outer", Also("control: the research projection"), |_| {}, Accept),
        row("outer", Also("control: a whole-provider gap"), |d| d.whole(WHOLE_GAP), Accept),
        row("outer", Also("a record removed"), |d| { let at = d.position(REMOTE); d.records().remove(at); }, Accept),
        row("outer", Also("record keys in another order"), |d| d.fields(CONFIG).reverse(), Accept),
        // outer: discriminators, extra members, ordering
        row("outer", Also("both members"), |d| d.whole(r#"{"researched": [], "unknown": {"gap": "x"}}"#), Reject(Rule("must be `{researched"))),
        row("outer", Also("a member beside researched"), |d| d.text("        researched:\n", "        extra: 1\n        researched:\n"), Reject(Rule("must be `{researched"))),
        row("outer", Also("a member beside unknown"), |d| d.whole(r#"{"unknown": {"gap": "x"}, "extra": 1}"#), Reject(Rule("must be `{researched"))),
        row("outer", Also("neither member, as text"), |d| d.whole(r#""none""#), Reject(Rule("must be `{researched"))),
        row("outer", Also("a record member the catalog lacks"), |d| d.set(CONFIG, "default", r#""x""#), Reject(Rule("canonical form"))),
        row("outer", Also("a record in research spelling"), |d| { d.remove(CONFIG, "value"); d.set(CONFIG, "value_type", r#""string""#); d.set(CONFIG, "value_optional", "false"); }, Reject(Rule("canonical form"))),
        row("outer", Also("records unsorted"), |d| d.records().swap(0, 1), Reject(Rule("canonical form"))),
        row("outer", Also("a spelling claimed twice"), |d| d.set(MODEL, "aliases", r#"["-m", "-c"]"#), Reject(Rule("also claimed"))),
        row("outer", Also("an entry key beside value and reason"), |d| d.text("    reason:", "    note: \"x\"\n    reason:"), Reject(Envelope)),
        row("outer", Also("an entry that is not a mapping"), |d| d.entry_raw = Some("7".into()), Reject(Envelope)),
        row("outer", Also("the entry written twice"), |d| d.appended = format!("cli_switches:\n    value: {WHOLE_GAP}\n    reason: \"Again.\"\n"), Reject(Loader)),
        // entry value (the override envelope's `value:`)
        row("entry value", Absent, |d| d.value = OverrideValue::Absent, Reject(Envelope)),
        row("entry value", Null, |d| d.whole("null"), Reject(Rule("must be `{researched"))),
        row("entry value", WrongType, |d| d.whole("7"), Reject(Rule("must be `{researched"))),
        row("entry value", Empty, |d| d.whole("{}"), Reject(Rule("must be `{researched"))),
        row("entry value", DuplicateKey, |d| d.text("    value:\n", "    value: {}\n    value:\n"), Reject(Loader)),
        row("entry value", Trailing, |d| d.text("    value:\n", "    value: x: y\n"), Reject(Loader)),
        // entry reason
        row("entry reason", Absent, |d| d.reason = None, Reject(Envelope)),
        row("entry reason", Null, |d| d.reason = Some("null".into()), Reject(Envelope)),
        row("entry reason", WrongType, |d| d.reason = Some("7".into()), Reject(Envelope)),
        row("entry reason", Empty, |d| d.reason = Some("\"\"".into()), Reject(Envelope)),
        row("entry reason", DuplicateKey, |d| d.text("    reason:", "    reason: \"Again.\"\n    reason:"), Reject(Loader)),
        row("entry reason", Trailing, |d| d.reason = Some("\"Matrix case.\" x".into()), Reject(Loader)),
        // researched
        row("researched", Absent, |d| d.whole("{}"), Reject(Rule("must be `{researched"))),
        row("researched", Null, |d| d.whole(r#"{"researched": null}"#), Reject(Rule("must be `{researched"))),
        row("researched", WrongType, |d| d.whole(r#"{"researched": "--config"}"#), Reject(Rule("must be `{researched"))),
        row("researched", OneBad, |d| d.raw_record(CONFIG, "7"), Reject(Rule("expected a mapping"))),
        row("researched", EveryBad, |d| d.whole(r#"{"researched": [7, "--config"]}"#), Reject(Rule("expected a mapping"))),
        row("researched", Empty, |d| d.whole(r#"{"researched": []}"#), Reject(Rule("`cli_switches_gap` must say why"))),
        row("researched", DuplicateKey, |d| d.text("        researched:\n", "        researched: []\n        researched:\n"), Reject(Loader)),
        row("researched", Trailing, |d| d.text("        researched:\n", "        researched: x: y\n"), Reject(Loader)),
        // unknown (the whole-provider gap's discriminator)
        row("unknown", Absent, |d| d.whole("{}"), Reject(Rule("must be `{researched"))),
        row("unknown", Null, |d| d.whole(r#"{"unknown": null}"#), Reject(Rule("must say why"))),
        row("unknown", WrongType, |d| d.whole(r#"{"unknown": "Not yet researched."}"#), Reject(Rule("must say why"))),
        row("unknown", Empty, |d| d.whole(r#"{"unknown": {}}"#), Reject(Rule("must say why"))),
        row("unknown", DuplicateKey, |d| d.whole(r#"{"unknown": {"gap": "a"}, "unknown": {"gap": "b"}}"#), Reject(Loader)),
        row("unknown", Trailing, |d| d.whole(r#"{"unknown": {"gap": "a"} x}"#), Reject(Loader)),
        // unknown.gap
        row("unknown.gap", Absent, |d| d.whole(r#"{"unknown": {"why": "Not yet researched."}}"#), Reject(Rule("must say why"))),
        row("unknown.gap", Null, |d| d.whole(r#"{"unknown": {"gap": null}}"#), Reject(Rule("`cli_switches_gap` is null"))),
        row("unknown.gap", WrongType, |d| d.whole(r#"{"unknown": {"gap": 7}}"#), Reject(Rule("`cli_switches_gap` must be non-empty text"))),
        row("unknown.gap", Empty, |d| d.whole(r#"{"unknown": {"gap": ""}}"#), Reject(Rule("`cli_switches_gap` must be non-empty text"))),
        row("unknown.gap", DuplicateKey, |d| d.whole(r#"{"unknown": {"gap": "a", "gap": "b"}}"#), Reject(Loader)),
        row("unknown.gap", Trailing, |d| d.whole(r#"{"unknown": {"gap": "a" x}}"#), Reject(Loader)),
        row("unknown.gap", Also("blank"), |d| d.whole(r#"{"unknown": {"gap": "  "}}"#), Reject(Rule("`cli_switches_gap` must be non-empty text"))),
        row("unknown.gap", Also("beside another key"), |d| d.whole(r#"{"unknown": {"gap": "a", "why": "b"}}"#), Reject(Rule("canonical form"))),
        // flag (on --config)
        row("flag", Absent, |d| d.remove(CONFIG, "flag"), Reject(Rule("`flag` is required"))),
        row("flag", Null, |d| d.set(CONFIG, "flag", "null"), Reject(Rule("`flag` is required"))),
        row("flag", WrongType, |d| d.set(CONFIG, "flag", "7"), Reject(Rule("is not a switch spelling"))),
        row("flag", Empty, |d| d.set(CONFIG, "flag", r#""""#), Reject(Rule("is not a switch spelling"))),
        row("flag", DuplicateKey, |d| d.repeat(CONFIG, "flag", r#""--cfg""#), Reject(Loader)),
        row("flag", Trailing, |d| d.trail(CONFIG, "flag"), Reject(Loader)),
        row("flag", Also("not a spelling"), |d| d.set(CONFIG, "flag", r#""config""#), Reject(Rule("is not a switch spelling"))),
        // aliases (on --config, whose `-c` carries short_attached)
        row("aliases", Absent, |d| d.remove(CONFIG, "aliases"), Reject(Rule("short_attached"))),
        row("aliases", Null, |d| d.set(CONFIG, "aliases", "null"), Reject(Rule("`aliases` is null"))),
        row("aliases", WrongType, |d| d.set(CONFIG, "aliases", r#""-c""#), Reject(Rule("`aliases` must be a list"))),
        row("aliases", OneBad, |d| d.set(CONFIG, "aliases", r#"["-c", 7]"#), Reject(Rule("alias `7`"))),
        row("aliases", EveryBad, |d| d.set(CONFIG, "aliases", "[7]"), Reject(Rule("alias `7`"))),
        row("aliases", Empty, |d| d.set(CONFIG, "aliases", "[]"), Reject(Rule("short_attached"))),
        row("aliases", DuplicateKey, |d| d.repeat(CONFIG, "aliases", r#"["-x"]"#), Reject(Loader)),
        row("aliases", Trailing, |d| d.trail(CONFIG, "aliases"), Reject(Loader)),
        row("aliases", Also("empty once short_attached goes too"), |d| { d.set(CONFIG, "aliases", "[]"); d.set(CONFIG, "attachments", r#"["space", "equals"]"#); }, Accept),
        row("aliases", Also("absent where nothing needs a short spelling"), |d| d.remove(JSON_FLAG, "aliases"), Reject(Rule("canonical form"))),
        row("aliases", Also("a spelling repeated"), |d| d.set(CONFIG, "aliases", r#"["-c", "-c"]"#), Reject(Rule("listed twice"))),
        // value (on --config)
        row("value", Absent, |d| d.remove(CONFIG, "value"), Reject(Rule("`value_type` is required"))),
        row("value", Null, |d| d.set(CONFIG, "value", "null"), Reject(Rule("`value_type` is required"))),
        row("value", WrongType, |d| d.set(CONFIG, "value", r#"["string"]"#), Reject(Rule("`value_type` must be a string"))),
        row("value", Empty, |d| d.set(CONFIG, "value", "{}"), Reject(Rule("`value_type` must be a string"))),
        row("value", DuplicateKey, |d| d.set(CONFIG, "value", r#"{"string": {"optional": false}, "string": {"optional": true}}"#), Reject(Loader)),
        row("value", Trailing, |d| d.trail(CONFIG, "value"), Reject(Loader)),
        row("value", Also("not a member"), |d| d.set(CONFIG, "value", r#""list""#), Reject(Rule("is not one of"))),
        row("value", Also("two members"), |d| d.set(CONFIG, "value", r#"{"string": {"optional": false}, "number": {"optional": false}}"#), Reject(Rule("`value_type` must be a string"))),
        // scalar optional (on --config)
        row("optional", Absent, |d| d.set(CONFIG, "value", r#"{"string": {}}"#), Reject(Rule("`value_optional` is required"))),
        row("optional", Null, |d| d.set(CONFIG, "value", r#"{"string": {"optional": null}}"#), Reject(Rule("`value_optional` is required"))),
        row("optional", WrongType, |d| d.set(CONFIG, "value", r#"{"string": {"optional": "no"}}"#), Reject(Rule("`value_optional` must be a boolean"))),
        row("optional", Empty, |d| d.set(CONFIG, "value", r#"{"string": {"optional": ""}}"#), Reject(Rule("`value_optional` must be a boolean"))),
        row("optional", DuplicateKey, |d| d.set(CONFIG, "value", r#"{"string": {"optional": false, "optional": true}}"#), Reject(Loader)),
        row("optional", Trailing, |d| d.set(CONFIG, "value", r#"{"string": {"optional": false} x}"#), Reject(Loader)),
        row("optional", Also("true"), |d| d.set(CONFIG, "value", r#"{"string": {"optional": true}}"#), Accept),
        row("optional", Also("a payload that is not a mapping"), |d| d.set(CONFIG, "value", r#"{"string": true}"#), Reject(Rule("`value_optional` is required"))),
        row("optional", Also("beside a minimum"), |d| d.set(CONFIG, "value", r#"{"string": {"optional": false, "min": 1}}"#), Reject(Rule("variadic_min"))),
        row("optional", Also("on a no-value record"), |d| d.set(OSS, "value", r#"{"none": {"optional": false}}"#), Reject(Rule("`value_optional` is present only"))),
        // variadic min (on --image)
        row("min", Absent, |d| d.set(IMAGE, "value", r#"{"variadic": {}}"#), Reject(Rule("`variadic_min` is required"))),
        row("min", Null, |d| d.set(IMAGE, "value", r#"{"variadic": {"min": null}}"#), Reject(Rule("`variadic_min` is required"))),
        row("min", WrongType, |d| d.set(IMAGE, "value", r#"{"variadic": {"min": "two"}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Empty, |d| d.set(IMAGE, "value", r#"{"variadic": {"min": ""}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", DuplicateKey, |d| d.set(IMAGE, "value", r#"{"variadic": {"min": 1, "min": 2}}"#), Reject(Loader)),
        row("min", Trailing, |d| d.set(IMAGE, "value", r#"{"variadic": {"min": 1} x}"#), Reject(Loader)),
        row("min", Also("boolean"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": true}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Also("list"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": [1]}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Also("mapping"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": {"at_least": 1}}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Also("zero"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": 0}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Also("fraction"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": 1.5}}"#), Reject(Rule("`variadic_min` must be"))),
        row("min", Also("123"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": 123}}"#), Accept),
        row("min", Also("unknown with a gap"), |d| { d.set(IMAGE, "value", r#"{"variadic": {"min": "unknown"}}"#); d.set(IMAGE, "gap", r#""The reference does not say whether -i may be given with no file.""#); }, Accept),
        row("min", Also("unknown without a gap"), |d| d.set(IMAGE, "value", r#"{"variadic": {"min": "unknown"}}"#), Reject(Rule("`gap` must describe"))),
        // attachments (on --config)
        row("attachments", Absent, |d| d.remove(CONFIG, "attachments"), Reject(Rule("`attachment` is required"))),
        row("attachments", Null, |d| d.set(CONFIG, "attachments", "null"), Reject(Rule("`attachment` is required"))),
        row("attachments", WrongType, |d| d.set(CONFIG, "attachments", r#""space""#), Reject(Rule("`attachment` must be a list"))),
        row("attachments", OneBad, |d| d.set(CONFIG, "attachments", r#"["space", "glued"]"#), Reject(Rule("is not one of"))),
        row("attachments", EveryBad, |d| d.set(CONFIG, "attachments", r#"["glued", 7]"#), Reject(Rule("is not one of"))),
        row("attachments", Empty, |d| d.set(CONFIG, "attachments", "[]"), Reject(Rule("needs at least one"))),
        row("attachments", DuplicateKey, |d| d.repeat(CONFIG, "attachments", r#"["space"]"#), Reject(Loader)),
        row("attachments", Trailing, |d| d.trail(CONFIG, "attachments"), Reject(Loader)),
        row("attachments", Also("a form repeated"), |d| d.set(CONFIG, "attachments", r#"["space", "space"]"#), Reject(Rule("twice"))),
        row("attachments", Also("set on a no-value record"), |d| d.set(OSS, "attachments", r#"["space"]"#), Reject(Rule("must be empty"))),
        row("attachments", Also("empty on a no-value record"), |d| { d.set(MODEL, "value", r#""none""#); d.set(MODEL, "attachments", "[]"); }, Accept),
        // scopes (on --image, at the root and at exec)
        row("scopes", Absent, |d| d.remove(IMAGE, "scopes"), Reject(Rule("`invocation_scope` is required"))),
        row("scopes", Null, |d| d.set(IMAGE, "scopes", "null"), Reject(Rule("`invocation_scope` is required"))),
        row("scopes", WrongType, |d| d.set(IMAGE, "scopes", r#""global""#), Reject(Rule("`invocation_scope` must be a list"))),
        row("scopes", OneBad, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, "everywhere"]"#), Reject(Rule("is not a mapping"))),
        row("scopes", EveryBad, |d| d.set(IMAGE, "scopes", r#"["everywhere", 7]"#), Reject(Rule("is not a mapping"))),
        row("scopes", Empty, |d| d.set(IMAGE, "scopes", "[]"), Reject(Rule("needs at least one entry"))),
        row("scopes", DuplicateKey, |d| d.repeat(IMAGE, "scopes", r#"["global"]"#), Reject(Loader)),
        row("scopes", Trailing, |d| d.trail(IMAGE, "scopes"), Reject(Loader)),
        row("scopes", Also("unsorted"), |d| d.set(IMAGE, "scopes", r#"[{"command": ["exec"]}, {"command": []}]"#), Reject(Rule("canonical form"))),
        row("scopes", Also("an entry repeated"), |d| d.set(IMAGE, "scopes", r#"[{"command": ["exec"]}, {"command": ["exec"]}]"#), Reject(Rule("listed twice"))),
        row("scopes", Also("global beside a command path"), |d| d.set(IMAGE, "scopes", r#"["global", {"command": ["exec"]}]"#), Reject(Rule("global scope already covers"))),
        row("scopes", Also("an entry in research spelling"), |d| d.set(CONFIG, "scopes", r#"[{"applies_to": "global"}]"#), Reject(Rule("canonical form"))),
        // nested command (--image's exec entry; --json for the root control)
        row("command", Absent, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {}]"#), Reject(Rule("`applies_to` is required"))),
        row("command", Null, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": null}]"#), Reject(Rule("`command` is required"))),
        row("command", WrongType, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": "exec"}]"#), Reject(Rule("`command` must be a list"))),
        row("command", OneBad, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": ["exec", 7]}]"#), Reject(Rule("command word `7`"))),
        row("command", EveryBad, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": [7, "-x"]}]"#), Reject(Rule("command word `7`"))),
        row("command", Empty, |d| d.set(JSON_FLAG, "scopes", r#"[{"command": []}]"#), Accept),
        row("command", DuplicateKey, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": ["exec"], "command": []}]"#), Reject(Loader)),
        row("command", Trailing, |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": ["exec"] x}]"#), Reject(Loader)),
        row("command", Also("empty beside another root"), |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": []}]"#), Reject(Rule("listed twice"))),
        row("command", Also("beside another key"), |d| d.set(IMAGE, "scopes", r#"[{"command": []}, {"command": ["exec"], "global": true}]"#), Reject(Rule("`applies_to` is required"))),
        // description (on --config)
        row("description", Absent, |d| d.remove(CONFIG, "description"), Reject(Rule("`description` is required"))),
        row("description", Null, |d| d.set(CONFIG, "description", "null"), Reject(Rule("`description` is required"))),
        row("description", WrongType, |d| d.set(CONFIG, "description", "7"), Reject(Rule("`description` must be non-empty text"))),
        row("description", Empty, |d| d.set(CONFIG, "description", r#""""#), Reject(Rule("`description` must be non-empty text"))),
        row("description", DuplicateKey, |d| d.repeat(CONFIG, "description", r#""Again.""#), Reject(Loader)),
        row("description", Trailing, |d| d.trail(CONFIG, "description"), Reject(Loader)),
        row("description", Also("blank"), |d| d.set(CONFIG, "description", r#""   ""#), Reject(Rule("`description` must be non-empty text"))),
        // record gap (on --remote, whose value is unknown; --config is known)
        row("gap", Absent, |d| d.remove(REMOTE, "gap"), Reject(Rule("`gap` must describe"))),
        row("gap", Null, |d| d.set(REMOTE, "gap", "null"), Reject(Rule("`gap` must describe"))),
        row("gap", WrongType, |d| d.set(REMOTE, "gap", "7"), Reject(Rule("`gap` must be non-empty text"))),
        row("gap", Empty, |d| d.set(REMOTE, "gap", r#""""#), Reject(Rule("`gap` must be non-empty text"))),
        row("gap", DuplicateKey, |d| d.repeat(REMOTE, "gap", r#""Again.""#), Reject(Loader)),
        row("gap", Trailing, |d| d.trail(REMOTE, "gap"), Reject(Loader)),
        row("gap", Also("absent on a known record"), |d| d.remove(CONFIG, "gap"), Reject(Rule("canonical form"))),
        row("gap", Also("text on a known record"), |d| d.set(CONFIG, "gap", r#""Nothing is unknown.""#), Reject(Rule("nothing is unknown"))),
    ];
    check_override_grid(&cells);

    let base = OverrideDoc::from_catalog(cli_switches(&generate_with(FIXTURE).unwrap()));
    // The fixed records the edits name, so a fixture change fails here first.
    let flags: Vec<String> = match &base.value {
        OverrideValue::Records(records) => records
            .iter()
            .map(|record| match record {
                OverrideRecord::Fields(fields) => fields[0].1.clone(),
                OverrideRecord::Raw(raw) => raw.clone(),
            })
            .collect(),
        _ => unreachable!(),
    };
    assert_eq!(flags, [CONFIG, IMAGE, JSON_FLAG, MODEL, OSS, REMOTE].map(|f| Value::from(f).to_string()));

    run_matrix(
        &cells,
        |cell| match cell.shape {
            Shape::Also(text) => format!("{} / {text}", cell.column),
            shape => format!("{} / {shape:?}", cell.column),
        },
        |cell| judge_override(&base, cell),
    );
}

/// Every `R` and `A` of [`OVERRIDE_OUTCOMES`] has exactly one cell that
/// agrees with it, and no cell sits on a `-` or outside a known column.
fn check_override_grid(cells: &[OverrideCell]) {
    let mut problems = Vec::new();
    for (column, letters) in OVERRIDE_OUTCOMES {
        let letters: Vec<&str> = letters.split_whitespace().collect();
        assert_eq!(letters.len(), SHAPES.len(), "{column}: one letter per shape");
        for (shape, letter) in SHAPES.iter().zip(letters) {
            let found: Vec<&OverrideCell> = cells
                .iter()
                .filter(|cell| cell.column == *column && cell.shape == *shape)
                .collect();
            match (letter, found.as_slice()) {
                ("-", []) => {}
                ("R", [cell]) if matches!(cell.want, Want::Reject(_)) => {}
                ("A", [cell]) if matches!(cell.want, Want::Accept) => {}
                (letter, found) => problems.push(format!(
                    "{column} / {shape:?}: the table says `{letter}`, the matrix has {:?}",
                    found.iter().map(|cell| &cell.want).collect::<Vec<_>>()
                )),
            }
        }
    }
    for cell in cells {
        let known = cell.column == "outer" || OVERRIDE_OUTCOMES.iter().any(|(column, _)| *column == cell.column);
        if !known {
            problems.push(format!("cell column `{}` is not in the table", cell.column));
        }
    }
    assert!(problems.is_empty(), "override grid:\n{}", problems.join("\n"));
}

fn judge_override(base: &OverrideDoc, cell: &OverrideCell) -> Result<(), String> {
    let mut doc = base.clone();
    (cell.edit)(&mut doc);
    let fixture = Fixture::for_slug("codex").with_real_overrides();
    fixture.write(DOC, FIXTURE);
    let overrides = fixture.path("docs/providers/overrides/codex.yaml");
    let mut text = std::fs::read_to_string(&overrides).unwrap();
    text.push_str(&doc.render());
    std::fs::write(&overrides, text).unwrap();
    let outcome = fixture.generate();
    match (&cell.want, &outcome) {
        (Want::Accept, Ok(generation)) => {
            let expected = doc.parsed_value();
            if cli_switches(generation) == &expected {
                Ok(())
            } else {
                Err(format!("generated {} instead of {expected}", cli_switches(generation)))
            }
        }
        (Want::Reject(Gate::Loader), Err(GenError::Yaml { path, .. })) if path == &overrides => Ok(()),
        (
            Want::Reject(Gate::Envelope),
            Err(GenError::OverrideMissingValue { field }
            | GenError::OverrideInvalidReason { field, .. }
            | GenError::OverrideUnknownKey { field, .. }),
        ) if field == "cli_switches" => Ok(()),
        (Want::Reject(Gate::Rule(needle)), Err(GenError::CliSwitchInvalid { message, .. })) if message.contains(needle) => {
            Ok(())
        }
        (want, Err(err)) => Err(format!("expected {want:?}, refused with: {err}")),
        (want, Ok(generation)) => Err(format!("expected {want:?}, generated: {}", cli_switches(generation))),
    }
}
