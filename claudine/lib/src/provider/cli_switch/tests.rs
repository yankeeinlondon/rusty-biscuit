use super::*;

const IMAGE_GAP: &str = "minimum not established";

/// A small researched catalog: `-c`/`--config` everywhere in every form,
/// `--image` variadic at the root and `exec` with only a separate value,
/// `--json` only at `exec`, and `--search` with an unknown type.
static SWITCHES: &[CliSwitch] = &[
    CliSwitch {
        flag: "--config",
        aliases: &["-c"],
        value: SwitchValue::String { optional: false },
        attachments: &[
            SwitchAttachment::Space,
            SwitchAttachment::Equals,
            SwitchAttachment::ShortAttached,
        ],
        scopes: &[SwitchScope::Global],
        description: "Override a configuration value.",
        gap: None,
    },
    CliSwitch {
        flag: "--image",
        aliases: &["-i"],
        value: SwitchValue::Variadic {
            min: VariadicMin::Unknown,
        },
        attachments: &[SwitchAttachment::Space],
        scopes: &[SwitchScope::Command(&[]), SwitchScope::Command(&["exec"])],
        description: "Attach images.",
        gap: Some(IMAGE_GAP),
    },
    CliSwitch {
        flag: "--json",
        aliases: &[],
        value: SwitchValue::None,
        attachments: &[],
        scopes: &[SwitchScope::Command(&["exec"])],
        description: "Print events as JSON Lines.",
        gap: None,
    },
    CliSwitch {
        flag: "--search",
        aliases: &[],
        value: SwitchValue::Unknown,
        attachments: &[],
        scopes: &[SwitchScope::Global],
        description: "Enable web search.",
        gap: Some("value consumption not established"),
    },
];
const CATALOG: CliSwitchCatalog = CliSwitchCatalog::Researched(SWITCHES);
const GAP: CliSwitchCatalog = CliSwitchCatalog::Unknown { gap: "not researched" };

fn flag(lookup: SwitchLookup) -> Option<&'static str> {
    match lookup {
        SwitchLookup::Known(switch) => Some(switch.flag),
        SwitchLookup::NotInCatalog | SwitchLookup::CatalogGap { .. } => None,
    }
}

/// A canonical spelling and an alias find the same record; nothing else
/// does, including a prefix or a different case.
#[test]
fn a_spelling_or_alias_finds_its_record_exactly() {
    assert_eq!(flag(lookup_in(CATALOG, &["exec"], "--config")), Some("--config"));
    assert_eq!(flag(lookup_in(CATALOG, &["exec"], "-c")), Some("--config"));
    for other in ["--conf", "-C", "config", "--config=x", "-cx", "--"] {
        assert_eq!(lookup_in(CATALOG, &["exec"], other), SwitchLookup::NotInCatalog, "{other}");
    }
}

/// Global records apply everywhere; command records only at their exact
/// path, so the resume entrypoint has its own answer.
#[test]
fn a_record_applies_only_at_its_command_paths() {
    assert_eq!(flag(lookup_in(CATALOG, &["exec"], "--json")), Some("--json"));
    for path in [&[][..], &["exec", "resume"], &["review"]] {
        assert_eq!(lookup_in(CATALOG, path, "--json"), SwitchLookup::NotInCatalog, "{path:?}");
    }
    assert_eq!(flag(lookup_in(CATALOG, &[], "-i")), Some("--image"));
    assert_eq!(lookup_in(CATALOG, &["exec", "resume"], "-i"), SwitchLookup::NotInCatalog);
    assert_eq!(flag(lookup_in(CATALOG, &["exec", "resume"], "-c")), Some("--config"));
}

/// A gap is not an empty inventory: every spelling reports the gap and
/// reads as an unknown value, never as a switch that takes none.
#[test]
fn a_catalog_gap_establishes_nothing() {
    let lookup = lookup_in(GAP, &[], "--json");
    assert_eq!(lookup, SwitchLookup::CatalogGap { gap: "not researched" });
    assert_eq!(lookup.value(), SwitchValue::Unknown);
    assert_eq!(SwitchLookup::NotInCatalog.value(), SwitchValue::Unknown);
    assert_eq!(lookup_in(CATALOG, &[], "--search").value(), SwitchValue::Unknown);
    assert_eq!(lookup_in(CATALOG, &["exec"], "--json").value(), SwitchValue::None);
}

/// Attached values split off only in a form the research established.
#[test]
fn a_token_splits_only_in_researched_attachment_forms() {
    let config = |token| match_token_in(CATALOG, &["exec"], token);
    let matched = config("-c").unwrap();
    assert_eq!((matched.switch.flag, matched.spelling, matched.attached), ("--config", "-c", None));
    let matched = config("-csk-secret").unwrap();
    assert_eq!(matched.spelling, "-c");
    assert_eq!(matched.attached, Some((SwitchAttachment::ShortAttached, "sk-secret")));
    let matched = config("--config=model=o3").unwrap();
    assert_eq!(matched.attached, Some((SwitchAttachment::Equals, "model=o3")));
    // `=` may be attached text of a short switch too.
    let matched = config("-c=x").unwrap();
    assert_eq!(matched.attached, Some((SwitchAttachment::Equals, "x")));
    let matched = config("--config=").unwrap();
    assert_eq!(matched.attached, Some((SwitchAttachment::Equals, "")));

    // `--image` takes only a separate value.
    assert_eq!(match_token_in(CATALOG, &[], "-ia.png"), None);
    assert_eq!(match_token_in(CATALOG, &[], "--image=a.png"), None);
    // Not a switch, a long switch that is not researched, a non-ASCII short.
    for token in ["c", "--frob=1", "--cx", "-é", "-éx", "-", "--"] {
        assert_eq!(match_token_in(CATALOG, &["exec"], token), None, "{token}");
    }
    assert_eq!(match_token_in(GAP, &["exec"], "-cx"), None);
}

/// Candidates keep every provider's answer; they agree only on an equal
/// value, and unknown never agrees with known.
#[test]
fn candidates_keep_each_answer_and_agree_only_on_equal_values() {
    let union = |arms: Vec<SwitchLookup>| CandidateSwitch {
        arms: arms
            .into_iter()
            .zip([Provider::Codex, Provider::Claude, Provider::Gemini])
            .map(|(lookup, provider)| (provider, lookup))
            .collect(),
    };
    let config = lookup_in(CATALOG, &[], "-c");
    let agreed = union(vec![config, config]);
    assert_eq!(agreed.agreed_value(), Some(SwitchValue::String { optional: false }));
    assert_eq!(agreed.arms()[1].0, Provider::Claude);

    let mixed = union(vec![config, SwitchLookup::NotInCatalog]);
    assert_eq!(mixed.agreed_value(), None);
    assert_eq!(mixed.arms()[1], (Provider::Claude, SwitchLookup::NotInCatalog));

    let unknown = union(vec![SwitchLookup::NotInCatalog, SwitchLookup::CatalogGap { gap: "g" }]);
    assert_eq!(unknown.agreed_value(), Some(SwitchValue::Unknown));
    assert_eq!(union(vec![]).agreed_value(), None);
}
