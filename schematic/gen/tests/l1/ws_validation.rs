//! WebSocket definition validation through the shared diagnostics format,
//! and the composed handshake URLs of every shipped WebSocket definition.

use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};
use schematic_gen::diagnostics::{Feature, Output, Severity, has_errors};
use schematic_gen::errors::GeneratorError;
use schematic_gen::output::ws_modules::{
    generate_ws_definition_modules, render_ws_api_module, shipped_ws_apis, validate_ws_apis,
    validate_ws_definition_modules,
};
use schematic_gen::ws_codegen::plan::lower_to_plan;

fn fixture_api(base_url: &str, path: &str) -> WebSocketApi {
    WebSocketApi {
        name: "FixtureWs".to_string(),
        description: "Fixture".to_string(),
        base_url: base_url.to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_mapping: None,
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Channel".to_string(),
            path: path.to_string(),
            description: "Fixture channel".to_string(),
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![MessageSchema::new(
                "Frame",
                MessageDirection::Server,
                Schema::new("serde_json::Value"),
            )],
            ..WebSocketEndpoint::default()
        }],
        runtime: None,
    }
}

#[test]
fn every_shipped_endpoint_composes_its_documented_url() {
    // Dock: the official UCD2 AsyncAPI server is `ws://uc-dock.local:946`;
    // the definition's `ws://dock.local` is logged as out-of-scope for this
    // change and asserted as it stands.
    let expected = [
        ("ElevenLabsTTS", "TextToSpeech", "wss://api.elevenlabs.io/v1/text-to-speech/{voice_id}/stream-input"),
        ("ElevenLabsTTS", "MultiContextTextToSpeech", "wss://api.elevenlabs.io/v1/text-to-speech/{voice_id}/multi-stream-input"),
        ("UnfoldedCircleCoreWs", "CoreWs", "ws://remote.local/ws"),
        ("UnfoldedCircleCoreWs", "CoreIntegrations", "ws://remote.local/intg"),
        ("UnfoldedCircleCoreWs", "CoreProfiles", "ws://remote.local/profiles"),
        ("UnfoldedCircleCoreWs", "CoreEvents", "ws://remote.local/events"),
        ("UnfoldedCircleDockWs", "DockRoot", "ws://dock.local/"),
        ("UnfoldedCircleIntegrationWs", "Integration", "ws://remote.local/intg"),
        ("SamsungSmartTvRemote", "RemoteControl", "wss://192.168.1.1:8002/api/v2/channels/samsung.remote.control"),
        ("SamsungSmartTvRemote", "ArtMode", "wss://192.168.1.1:8002/api/v2/channels/com.samsung.art-app"),
    ];

    let mut composed = Vec::new();
    for api in shipped_ws_apis() {
        let plan = lower_to_plan(&api).expect("shipped definitions lower");
        for endpoint in &plan.endpoints {
            // The generated connect methods join `base_url` and the
            // substituted path with `format!("{}{}", ..)`.
            composed.push((
                plan.api_name.clone(),
                endpoint.id.clone(),
                format!("{}{}", plan.base_url, endpoint.path),
            ));
        }
    }
    let expected: Vec<(String, String, String)> = expected
        .iter()
        .map(|(api, id, url)| (api.to_string(), id.to_string(), url.to_string()))
        .collect();
    let mut sorted_composed = composed.clone();
    sorted_composed.sort();
    let mut sorted_expected = expected;
    sorted_expected.sort();
    assert_eq!(sorted_composed, sorted_expected);
}

#[test]
fn shipped_definitions_report_no_warning_or_error() {
    let validation = validate_ws_definition_modules();
    assert!(
        validation.diagnostics.is_empty(),
        "shipped WebSocket definitions must lower cleanly: {:#?}",
        validation
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
}

#[test]
fn every_shipped_module_is_generated() {
    let modules = generate_ws_definition_modules().expect("shipped modules generate");
    let names: Vec<&str> = modules.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "elevenlabs_ws.rs",
            "unfolded_circle_core_ws.rs",
            "unfolded_circle_dock_ws.rs",
            "unfolded_circle_integration_ws.rs",
            "samsung_smart_tv_remote_ws.rs",
        ]
    );
    let core = &modules[1].1;
    assert!(
        core.contains(r#"Self::with_base_url("ws://remote.local")"#),
        "the generated Core client defaults to the origin"
    );
}

#[test]
fn an_exactly_duplicated_boundary_segment_warns_without_blocking() {
    let api = fixture_api("ws://remote.local/ws", "/ws");

    let validation = validate_ws_apis(std::slice::from_ref(&api));
    let rendered: Vec<String> = validation.diagnostics.iter().map(ToString::to_string).collect();
    assert_eq!(
        rendered,
        [
            "severity=warning api=FixtureWs endpoint=Channel feature=websocket-definition \
             output=rust: endpoints/Channel/path: base_url `ws://remote.local/ws` ends with \
             segment `ws` and path `/ws` starts with it; the composed URL repeats `/ws/ws`"
        ]
    );
    assert!(!has_errors(&validation.diagnostics));
    let module = render_ws_api_module(&api).expect("a warning never blocks the Rust output");
    assert!(module.contains("pub struct ChannelClient"), "{module}");
}

#[test]
fn a_legitimately_repeated_segment_is_only_a_warning() {
    let api = fixture_api("https://example.test/api", "/api/stream");

    let validation = validate_ws_apis(std::slice::from_ref(&api));
    assert_eq!(validation.diagnostics.len(), 1, "{:?}", validation.diagnostics);
    assert_eq!(validation.diagnostics[0].severity, Severity::Warning);
    render_ws_api_module(&api).expect("repeated segments never fail generation");
}

#[test]
fn a_substring_or_prefix_overlap_emits_nothing() {
    for (base, path) in [
        ("https://example.test/api", "/apikeys"),
        ("https://example.test/apikeys", "/api"),
        ("https://example.test/api", "/v1/api"),
        ("ws://remote.local", "/ws"),
    ] {
        let validation = validate_ws_apis(&[fixture_api(base, path)]);
        assert!(
            validation.diagnostics.is_empty(),
            "{base} + {path}: {:?}",
            validation.diagnostics
        );
    }
}

#[test]
fn a_lowering_error_reaches_the_shared_format_and_blocks_the_rust_output() {
    // A path embedding a query string: a lowering error that would
    // previously have been dropped together with the whole module.
    let mut api = fixture_api("wss://example.test", "/rooms?room=1");
    api.endpoints[0].messages.clear();

    let validation = validate_ws_apis(std::slice::from_ref(&api));
    let rendered: Vec<String> = validation.diagnostics.iter().map(ToString::to_string).collect();
    assert_eq!(
        rendered,
        [
            "severity=warning api=FixtureWs endpoint=Channel feature=websocket-definition \
             output=rust: endpoints/Channel: Endpoint has no messages defined",
            "severity=error api=FixtureWs endpoint=Channel feature=websocket-definition \
             output=rust: endpoints/Channel/path: path `/rooms?room=1` embeds a query or \
             fragment; declare query parameters in `params.query`",
        ]
    );
    assert!(has_errors(&validation.diagnostics));
    assert!(validation.diagnostics.iter().all(|d| d.feature == Feature::WebSocketDefinition));

    match render_ws_api_module(&api) {
        Err(GeneratorError::Validation { output, diagnostics }) => {
            assert_eq!(output, Output::Rust);
            assert_eq!(diagnostics, validation.diagnostics, "every finding, warnings included");
        }
        other => panic!("expected a Rust validation error, got {other:?}"),
    }
}

#[test]
fn info_notes_stay_out_of_the_shared_diagnostics() {
    // Auth messages with a handshake credential and no `AuthFlowHints` are an
    // info note, not a diagnostic.
    let mut api = fixture_api("wss://example.test", "/stream");
    api.auth = AuthStrategy::ApiKey { header: "X-Key".to_string(), value_prefix: None };
    api.endpoints[0].messages = vec![
        MessageSchema::new("Auth", MessageDirection::Client, Schema::new("serde_json::Value")),
        MessageSchema::new("Frame", MessageDirection::Server, Schema::new("serde_json::Value")),
    ];
    let validation = validate_ws_apis(&[api]);
    assert!(validation.diagnostics.is_empty(), "{:?}", validation.diagnostics);
    assert_eq!(
        validation.notes,
        [
            "severity=info api=FixtureWs feature=websocket-definition output=rust: \
             auth: Auth messages detected; using header-based auth. Add AuthFlowHints for \
             message-based auth flow."
        ]
    );
}

#[test]
fn receive_timeout_is_enforced_so_no_shipped_api_has_a_note_about_it() {
    let validation = validate_ws_apis(&[fixture_api("wss://example.test", "/stream")]);
    assert!(validation.diagnostics.is_empty());
    assert!(validation.notes.is_empty(), "{:?}", validation.notes);
}

#[test]
fn the_validate_command_keeps_info_notes_out_of_its_default_output() {
    let run = |verbose: bool| {
        let mut command = std::process::Command::new(biscuit_test_harness::bin_exe!("schematic-gen"));
        if verbose {
            command.arg("-v");
        }
        let output = command
            .args(["validate", "--api", "gitea"])
            .env("NO_COLOR", "1")
            .output()
            .expect("run schematic-gen");
        assert!(output.status.success(), "{output:?}");
        String::from_utf8_lossy(&output.stderr).into_owned()
    };

    let quiet = run(false);
    assert!(!quiet.contains("severity=info"), "{quiet}");
    // No shipped WebSocket definition produces an info note any more, so the
    // verbose run is checked only for the absence of the retired
    // receive-timeout reminder.
    let verbose = run(true);
    assert!(!verbose.contains("receive_timeout"), "{verbose}");
}
