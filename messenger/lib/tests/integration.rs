//! Smoke tests for real provider sends.
//!
//! These tests are `#[ignore]` and require real credentials via env vars.
//! Run manually: `cargo test -p messenger --test integration -- --ignored`

use messenger::{Dispatch, Message, Messenger, Target};

#[cfg(feature = "slack")]
#[tokio::test]
#[ignore]
async fn smoke_test_slack_send() {
    use messenger::provider::slack::{SlackConfig, SlackProvider};
    use secrecy::SecretString;

    let token = std::env::var("SLACK_BOT_TOKEN").expect("SLACK_BOT_TOKEN must be set");
    let channel = std::env::var("SLACK_TEST_CHANNEL").expect("SLACK_TEST_CHANNEL must be set");

    let provider = SlackProvider::new(SlackConfig {
        bot_token: SecretString::from(token),
        api_base_url: None,
    });

    let mut messenger = Messenger::new();
    messenger.register(Box::new(provider));

    let message = Message::markdown("**Smoke test** from `messenger` library");
    let dispatch = Dispatch::to(Target::slack_channel(channel));

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    println!("Slack send receipt: {receipt:?}");
}

#[cfg(feature = "discord")]
#[tokio::test]
#[ignore]
async fn smoke_test_discord_send() {
    use messenger::provider::discord::{DiscordConfig, DiscordProvider};
    use secrecy::SecretString;

    let token = std::env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set");
    let channel = std::env::var("DISCORD_TEST_CHANNEL").expect("DISCORD_TEST_CHANNEL must be set");

    let provider = DiscordProvider::new(DiscordConfig {
        bot_token: SecretString::from(token),
    });

    let mut messenger = Messenger::new();
    messenger.register(Box::new(provider));

    let message = Message::markdown("**Smoke test** from `messenger` library");
    let dispatch = Dispatch::to(Target::discord_channel(channel));

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    println!("Discord send receipt: {receipt:?}");
}
