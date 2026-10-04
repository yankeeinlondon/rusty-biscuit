use super::*;

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| name.to_string()).collect()
}

#[cfg(windows)]
const ROOT: &str = r"C:\opt";
#[cfg(not(windows))]
const ROOT: &str = "/opt";

fn under(rest: &str) -> String {
    format!("{ROOT}{}{rest}", std::path::MAIN_SEPARATOR)
}

// ---- declaration side -------------------------------------------------------

#[test]
fn the_list_is_trimmed_deduplicated_and_merged_with_builder_names() {
    let declared = portable_names(
        &env(&[(PORTABLE_ENV_VARIABLES, " CONFIG_DIR , OBSIDIAN_VAULT,CONFIG_DIR")]),
        &["EXTRA".to_string(), "CONFIG_DIR".to_string()],
    );
    assert_eq!(declared.names, names(&["CONFIG_DIR", "EXTRA", "OBSIDIAN_VAULT"]));
    assert!(declared.invalid.is_empty());
}

#[test]
fn absent_and_empty_lists_declare_nothing_and_report_nothing() {
    for env in [env(&[]), env(&[(PORTABLE_ENV_VARIABLES, "")]), env(&[(PORTABLE_ENV_VARIABLES, " , ,")])] {
        assert_eq!(portable_names(&env, &[]), PortableNames::default());
    }
}

#[test]
fn invalid_entries_are_skipped_and_recorded_once_each() {
    let declared = portable_names(
        &env(&[(PORTABLE_ENV_VARIABLES, "CONFIG_DIR,bad-name,,lower,bad-name")]),
        &[" PADDED".to_string(), String::new()],
    );
    assert_eq!(declared.names, names(&["CONFIG_DIR"]));
    assert_eq!(declared.invalid, ["bad-name", "lower", " PADDED", ""]);
}

#[test]
fn every_entry_invalid_leaves_no_portable_name() {
    let declared = portable_names(&env(&[(PORTABLE_ENV_VARIABLES, "a,b-c,$X")]), &[]);
    assert!(declared.names.is_empty());
    assert_eq!(declared.invalid, ["a", "b-c", "$X"]);
}

// ---- value side -------------------------------------------------------------

#[test]
fn a_value_must_be_absolute_on_this_host() {
    assert_eq!(anchor_dir(None), Err(EnvAnchorProblem::Unset));
    assert_eq!(
        anchor_dir(Some("")),
        Err(EnvAnchorProblem::NotAbsolute { value: String::new() })
    );
    assert_eq!(
        anchor_dir(Some("../shared")),
        Err(EnvAnchorProblem::NotAbsolute { value: "../shared".into() })
    );
    #[cfg(not(windows))]
    let foreign = r"C:\config";
    #[cfg(windows)]
    let foreign = "/opt/config";
    assert_eq!(
        anchor_dir(Some(foreign)),
        Err(EnvAnchorProblem::ForeignAbsolute { value: foreign.into() })
    );
    assert_eq!(anchor_dir(Some(&under("config"))), Ok(PathBuf::from(under("config"))));
}

#[test]
fn the_deepest_whole_component_prefix_wins_and_name_order_breaks_ties() {
    let target = PathIdentity::new(Path::new(&under("config/app/x.json")));
    let declared = PortableNames {
        names: names(&["APP", "B_SAME", "A_SAME", "OLD", "UNSET"]),
        invalid: Vec::new(),
    };
    let values = env(&[
        ("APP", &under("config/app")),
        ("A_SAME", &under("config")),
        ("B_SAME", &under("config/")),
        ("OLD", &under("conf")),
    ]);
    let evaluation = evaluate_anchors(&declared, &values, &target);
    let eligible: Vec<&str> = evaluation.eligible.iter().map(|anchor| anchor.name.as_str()).collect();
    assert_eq!(eligible, ["APP", "A_SAME", "B_SAME"]);
    assert_eq!(evaluation.eligible[0].rest, [OsString::from("x.json")]);
    assert_eq!(
        evaluation.rejected,
        [
            (
                "OLD".to_string(),
                EnvAnchorProblem::NotAPrefix { value: PathBuf::from(under("conf")) }
            ),
            ("UNSET".to_string(), EnvAnchorProblem::Unset),
        ]
    );
}
