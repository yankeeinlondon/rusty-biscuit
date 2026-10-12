//! The API-level cross-origin policy, request-id header, and secret names of
//! every shipped definition.
//!
//! The table is exhaustive: a new definition fails here until its row is added.

use schematic_define::CrossOrigin;
use schematic_definitions::apis_by_module;

/// `(API name, request-id header, secret names)`; every shipped API is same-origin.
const EXPECTED: [(&str, Option<&str>, &[&str]); 18] = [
    ("Anthropic", None, &[]),
    ("ArtificialAnalysisData", None, &[]),
    ("ArtificialAnalysisCritPt", None, &[]),
    ("Bitbucket", None, &[]),
    ("OpenAI", Some("x-request-id"), &[]),
    ("ElevenLabs", None, &[]),
    ("Gitea", None, &[]),
    ("GitHub", Some("x-github-request-id"), &[]),
    ("GitLab", None, &[]),
    ("HuggingFaceHub", None, &[]),
    ("LmStudio", None, &[]),
    ("OllamaNative", None, &[]),
    ("OllamaOpenAI", None, &[]),
    ("EmqxBasic", None, &[]),
    ("EmqxBearer", None, &[]),
    ("Eversolo", None, &[]),
    ("SamsungSmartTv", None, &[]),
    ("UnfoldedCircleCoreRest", None, &[]),
];

#[test]
fn every_shipped_api_declares_the_expected_policy_fields() {
    let apis: Vec<_> = apis_by_module().into_values().flatten().collect();

    assert_eq!(
        apis.len(),
        EXPECTED.len(),
        "add the new definition to EXPECTED"
    );

    for api in &apis {
        let (_, request_id_header, secret_names) = EXPECTED
            .iter()
            .find(|(name, _, _)| *name == api.name)
            .unwrap_or_else(|| panic!("API {:?} has no row in EXPECTED", api.name));

        assert_eq!(api.cross_origin, CrossOrigin::same_origin(), "{}", api.name);
        assert_eq!(
            api.request_id_header.as_deref(),
            *request_id_header,
            "{}",
            api.name
        );
        assert_eq!(api.secret_names, *secret_names, "{}", api.name);
    }
}
