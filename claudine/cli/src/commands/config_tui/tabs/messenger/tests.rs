//! Messenger tab tests: provider sorting, modal key flows, webhook URL
//! validation, test-connection workflow, and rendering redaction invariants.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::routes::messenger_fields;
use super::*;
use claudine::config::claudine_config::ClaudineConfig;
use claudine::messaging::MessagingRouteConfig;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn test_app() -> App {
    App::new(ClaudineConfig::default(), None, None, false, None, None)
}

#[test]
fn messenger_names_are_sorted_alphabetically() {
    let mut app = test_app();
    app.config.messenger = Some(ClaudineMessengerConfig {
        active_config: None,
        configurations: std::collections::HashMap::from([
            (
                "zeta".to_string(),
                MessengerProviderConfig::Slack {
                    channel_id: "1".to_string(),
                    bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                },
            ),
            (
                "alpha".to_string(),
                MessengerProviderConfig::Discord {
                    channel_id: "2".to_string(),
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            ),
        ]),
    });

    assert_eq!(
        sorted_messenger_names(&app),
        vec!["alpha".to_string(), "zeta".to_string()]
    );
}

#[test]
fn s_hotkey_opens_select_modal() {
    let mut app = test_app();
    app.config.messenger = Some(ClaudineMessengerConfig {
        active_config: Some("alpha".to_string()),
        configurations: std::collections::HashMap::from([(
            "alpha".to_string(),
            MessengerProviderConfig::Discord {
                channel_id: "2".to_string(),
                bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
            },
        )]),
    });

    handle_key(&mut app, key(KeyCode::Char('s')));

    assert!(matches!(
        app.modal,
        Some(ModalState::MessengerSelect {
            highlighted: 1,
            for_repo: false
        })
    ));
}

#[test]
fn webhook_provider_field_definitions() {
    let discord_fields = messenger_fields("discord_webhook");
    assert_eq!(discord_fields.len(), 2);
    assert_eq!(discord_fields[0].0, "Webhook URL");
    assert!(discord_fields[0].2); // is_secret
    assert_eq!(discord_fields[0].1, ""); // default empty
    assert_eq!(discord_fields[1].0, "Webhook URL Env Var");
    assert!(!discord_fields[1].2); // not secret
    assert_eq!(discord_fields[1].1, "DISCORD_WEBHOOK_URL");

    let slack_fields = messenger_fields("slack_webhook");
    assert_eq!(slack_fields.len(), 2);
    assert_eq!(slack_fields[0].0, "Webhook URL");
    assert!(slack_fields[0].2); // is_secret
    assert_eq!(slack_fields[1].0, "Webhook URL Env Var");
    assert!(!slack_fields[1].2); // not secret
    assert_eq!(slack_fields[1].1, "SLACK_WEBHOOK_URL");
}

#[test]
fn build_messenger_from_fields_discord_webhook() {
    let fields = vec![
        (
            "Webhook URL".to_string(),
            "https://discord.com/api/webhooks/123/abc".to_string(),
        ),
        (
            "Webhook URL Env Var".to_string(),
            "DISCORD_WEBHOOK_URL".to_string(),
        ),
    ];
    let config = build_messenger_from_fields("discord_webhook", &fields).unwrap();
    match config {
        MessengerProviderConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(
                webhook_url,
                Some("https://discord.com/api/webhooks/123/abc".to_string())
            );
            assert_eq!(webhook_url_env, "DISCORD_WEBHOOK_URL");
        }
        _ => panic!("expected DiscordWebhook"),
    }
}

#[test]
fn build_messenger_from_fields_slack_webhook_env_only() {
    let fields = vec![
        ("Webhook URL".to_string(), "".to_string()),
        (
            "Webhook URL Env Var".to_string(),
            "SLACK_WEBHOOK_URL".to_string(),
        ),
    ];
    let config = build_messenger_from_fields("slack_webhook", &fields).unwrap();
    match config {
        MessengerProviderConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(webhook_url, None); // blank becomes None
            assert_eq!(webhook_url_env, "SLACK_WEBHOOK_URL");
        }
        _ => panic!("expected SlackWebhook"),
    }
}

#[test]
fn add_modal_includes_webhook_providers() {
    let mut app = test_app();
    handle_key(&mut app, key(KeyCode::Char('a')));

    assert!(matches!(
        app.modal,
        Some(ModalState::MessengerAdd { highlighted: 0 })
    ));

    // The providers array should have 6 entries including webhooks
    // We verify by checking that discord_webhook and slack_webhook
    // can be selected
    if let Some(ModalState::MessengerAdd { .. }) = &app.modal {
        // Move down to discord_webhook (index 1)
        handle_messenger_add_modal(&mut app, key(KeyCode::Down));
        assert_eq!(app.modal_highlighted(), 1);

        // Select it
        handle_messenger_add_modal(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerInput {
                provider,
                field_index: 0,
                label,
                is_secret: false,
                error: None,
                ..
            }) if provider == "discord_webhook" && label == "Configuration Name"
        ));
    }
}

#[test]
fn webhook_url_validation_rejects_invalid_url() {
    let mut app = test_app();
    // Start a discord_webhook input flow
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 1, // Webhook URL field (index 0 is name, index 1 is URL)
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "not-a-valid-url".to_string(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

    // Should stay on the same field with an error
    assert!(matches!(
        app.modal,
        Some(ModalState::MessengerInput {
            provider,
            field_index: 1,
            error: Some(_),
            is_secret: true,
            ..
        }) if provider == "discord_webhook"
    ));
}

#[test]
fn webhook_url_validation_accepts_valid_url() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "https://discord.com/api/webhooks/123/abc".to_string(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

    // Should advance to the next field (env var)
    assert!(matches!(
        app.modal,
        Some(ModalState::MessengerInput {
            provider,
            field_index: 2,
            label,
            is_secret: false,
            error: None,
            ..
        }) if provider == "discord_webhook" && label == "Webhook URL Env Var"
    ));
}

#[test]
fn env_only_webhook_skips_url_validation() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "slack_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "".to_string(), // empty URL
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

    // Should advance to env var field since URL is blank
    assert!(matches!(
        app.modal,
        Some(ModalState::MessengerInput {
            provider,
            field_index: 2,
            label,
            is_secret: false,
            error: None,
            ..
        }) if provider == "slack_webhook" && label == "Webhook URL Env Var"
    ));
}

#[test]
fn no_desktop_provider_in_add_modal() {
    let mut app = test_app();
    handle_key(&mut app, key(KeyCode::Char('a')));

    if let Some(ModalState::MessengerAdd { .. }) = &app.modal {
        // Try to find "desktop" in the provider list by scrolling through all
        for i in 0..PROVIDERS.len() {
            app.set_modal_highlighted(i);
            handle_messenger_add_modal(&mut app, key(KeyCode::Enter));
            if let Some(ModalState::MessengerInput { provider, .. }) = &app.modal {
                assert_ne!(provider, "desktop");
                // Cancel and reopen
                app.modal = Some(ModalState::MessengerAdd { highlighted: i });
            }
        }
    }
}

// =====================================================================
// Webhook test connection workflow (Phase 5)
// =====================================================================

#[test]
fn build_test_route_from_modal_uses_current_buffer() {
    let fields = vec![("Configuration Name".to_string(), "test".to_string())];
    let route = build_test_route_from_modal(
        "discord_webhook",
        &fields,
        "https://discord.com/api/webhooks/123/abc",
        1,
    );
    assert!(route.is_some());
    match route.unwrap() {
        MessagingRouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(
                webhook_url,
                Some("https://discord.com/api/webhooks/123/abc".to_string())
            );
            assert_eq!(webhook_url_env, "DISCORD_WEBHOOK_URL");
        }
        _ => panic!("expected DiscordWebhook"),
    }
}

#[test]
fn build_test_route_from_modal_uses_collected_fields() {
    let fields = vec![
        ("Configuration Name".to_string(), "test".to_string()),
        (
            "Webhook URL".to_string(),
            "https://hooks.slack.com/services/T1/B1/X".to_string(),
        ),
    ];
    let route = build_test_route_from_modal("slack_webhook", &fields, "MY_SLACK_URL", 2);
    assert!(route.is_some());
    match route.unwrap() {
        MessagingRouteConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            assert_eq!(
                webhook_url,
                Some("https://hooks.slack.com/services/T1/B1/X".to_string())
            );
            assert_eq!(webhook_url_env, "MY_SLACK_URL");
        }
        _ => panic!("expected SlackWebhook"),
    }
}

#[test]
fn build_test_route_from_modal_returns_none_for_non_webhook() {
    let fields = vec![("Configuration Name".to_string(), "test".to_string())];
    let route = build_test_route_from_modal("discord", &fields, "123", 1);
    assert!(route.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_key_sets_status_for_webhook() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "not-a-valid-url".to_string(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                test_status: Some(_),
                ..
            })
        ),
        "expected test_status to be set, got {:?}",
        app.modal
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_key_does_not_mark_dirty() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "slack_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "bad-url".to_string(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
    );

    assert!(!app.dirty, "test connection should not mark config dirty");
}

#[test]
fn test_connection_key_ignored_for_non_webhook() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "123".to_string(),
        label: "Channel ID".to_string(),
        is_secret: false,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                test_status: None,
                ..
            })
        ),
        "test_status should remain None for non-webhook provider"
    );
}

#[test]
fn test_connection_key_ignored_before_url_field() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 0, // still on config name
        fields: vec![],
        buffer: "test".to_string(),
        label: "Configuration Name".to_string(),
        is_secret: false,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                test_status: None,
                ..
            })
        ),
        "test_status should remain None before URL field"
    );
}

#[test]
fn typing_clears_test_status() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: String::new(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: Some("previous result".to_string()),
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                test_status: None,
                ..
            })
        ),
        "typing should clear test_status"
    );
}

#[test]
fn messenger_render_does_not_expose_raw_webhook_url() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = test_app();
    app.mode = AppMode::Detail;
    app.config.messenger = Some(ClaudineMessengerConfig {
        active_config: Some("alerts".to_string()),
        configurations: std::collections::HashMap::from([
            (
                "alerts".to_string(),
                MessengerProviderConfig::DiscordWebhook {
                    webhook_url: Some(
                        "https://discord.com/api/webhooks/123/abc_SECRET_TOKEN_xyz".to_string(),
                    ),
                    webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
                },
            ),
            (
                "deploys".to_string(),
                MessengerProviderConfig::SlackWebhook {
                    webhook_url: Some(
                        "https://hooks.slack.com/services/T00/B00/SECRET_TOKEN_xyz".to_string(),
                    ),
                    webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
                },
            ),
        ]),
    });

    terminal
        .draw(|f| {
            let area = f.area();
            render(f, area, &app);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    assert!(
        !text.contains("https://discord.com/api/webhooks/"),
        "raw Discord webhook URL must not appear in rendered TUI buffer"
    );
    assert!(
        !text.contains("https://hooks.slack.com/services/"),
        "raw Slack webhook URL must not appear in rendered TUI buffer"
    );
    assert!(
        !text.contains("abc_SECRET_TOKEN_xyz"),
        "raw webhook token must not appear in rendered TUI buffer"
    );
    assert!(
        !text.contains("SECRET_TOKEN_xyz"),
        "raw secret text must not appear in rendered TUI buffer"
    );
    // Masking should show asterisks instead
    assert!(
        text.contains("********"),
        "masked placeholder should appear in rendered TUI buffer"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_key_sets_testing_status_immediately() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 1,
        fields: vec![("Configuration Name".to_string(), "test".to_string())],
        buffer: "not-a-valid-url".to_string(),
        label: "Webhook URL".to_string(),
        is_secret: true,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                test_status: Some(status),
                ..
            }) if status == "Testing…"
        ),
        "test_status should be 'Testing…' immediately after T keypress, got {:?}",
        app.modal
    );

    assert!(
        app.pending_test.is_some(),
        "pending_test receiver should be set after T keypress"
    );

    // Give the background task time to complete and the event loop
    // would poll the receiver on the next iteration.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Simulate what the event loop does: poll the receiver
    if let Some(ref rx) = app.pending_test {
        match rx.try_recv() {
            Ok(result) => {
                app.pending_test = None;
                if let Some(ModalState::MessengerInput {
                    test_status, error, ..
                }) = &mut app.modal
                {
                    *test_status = Some(match result {
                        Ok(()) => "✓ Test connection successful".to_string(),
                        Err(e) => format!("✗ {}", e),
                    });
                    *error = None;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Task may still be running; that's fine for this test
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                app.pending_test = None;
            }
        }
    }
}

#[test]
fn modal_back_navigation_returns_to_previous_field() {
    let mut app = test_app();
    // Start on the Configuration Name field
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 0,
        fields: vec![],
        buffer: "deploys".to_string(),
        label: "Configuration Name".to_string(),
        is_secret: false,
        error: None,
        test_status: None,
    });

    // Press Enter to advance to Webhook URL field
    handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                field_index: 1,
                label,
                is_secret: true,
                ..
            }) if label == "Webhook URL"
        ),
        "should advance to Webhook URL field"
    );

    // Type some characters (avoid 't'/'T' since those trigger test)
    handle_messenger_input_modal(&mut app, key(KeyCode::Char('a')));
    handle_messenger_input_modal(&mut app, key(KeyCode::Char('b')));
    handle_messenger_input_modal(&mut app, key(KeyCode::Char('c')));

    // Press BackTab to go back to Configuration Name
    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                field_index: 0,
                label,
                buffer,
                is_secret: false,
                ..
            }) if label == "Configuration Name" && buffer == "deploys"
        ),
        "should return to Configuration Name field with previous value, got {:?}",
        app.modal
    );
}

#[test]
fn modal_back_navigation_from_second_provider_field() {
    let mut app = test_app();
    // Simulate being on the env-var field (field_index 2) with both
    // previous fields committed.
    app.modal = Some(ModalState::MessengerInput {
        provider: "discord_webhook".to_string(),
        field_index: 2,
        fields: vec![
            ("Configuration Name".to_string(), "alerts".to_string()),
            (
                "Webhook URL".to_string(),
                "https://discord.com/api/webhooks/123/abc".to_string(),
            ),
        ],
        buffer: "DISCORD_WEBHOOK_URL".to_string(),
        label: "Webhook URL Env Var".to_string(),
        is_secret: false,
        error: None,
        test_status: None,
    });

    // Press BackTab to go back to Webhook URL field
    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                field_index: 1,
                label,
                buffer,
                is_secret: true,
                error: None,
                test_status: None,
                ..
            }) if label == "Webhook URL"
                && buffer == "https://discord.com/api/webhooks/123/abc"
        ),
        "should return to Webhook URL field with secret=true and previous value, got {:?}",
        app.modal
    );

    // The fields vec should now only have Configuration Name
    if let Some(ModalState::MessengerInput { fields, .. }) = &app.modal {
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, "Configuration Name");
        assert_eq!(fields[0].1, "alerts");
    }
}

#[test]
fn modal_back_navigation_ignored_on_first_field() {
    let mut app = test_app();
    app.modal = Some(ModalState::MessengerInput {
        provider: "slack_webhook".to_string(),
        field_index: 0,
        fields: vec![],
        buffer: "test".to_string(),
        label: "Configuration Name".to_string(),
        is_secret: false,
        error: None,
        test_status: None,
    });

    handle_messenger_input_modal(
        &mut app,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
    );

    assert!(
        matches!(
            &app.modal,
            Some(ModalState::MessengerInput {
                field_index: 0,
                label,
                buffer,
                ..
            }) if label == "Configuration Name" && buffer == "test"
        ),
        "should stay on first field when BackTab pressed"
    );
}

#[test]
fn masked_input_render_buffer_masks_secrets() {
    assert_eq!(masked_input::render_buffer("hello", true), "●●●●●");
    assert_eq!(masked_input::render_buffer("hello", false), "hello");
    assert_eq!(masked_input::render_buffer("", true), "");
}
