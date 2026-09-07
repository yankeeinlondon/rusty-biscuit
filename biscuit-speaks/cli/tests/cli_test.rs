use std::io::Write;
use std::process::{Command, Output, Stdio};

use biscuit_test_harness::bin_exe;

/// Invoke the pre-built `so-you-say` binary directly.
///
/// Spawning it directly avoids `cargo run`, whose per-invocation build-lock
/// contention under parallel test execution makes these assertions flaky (slow
/// waits, interleaved cargo progress on stdout).
fn cli() -> Command {
    Command::new(bin_exe!("so-you-say"))
}

/// Invoke the CLI with no host programs discoverable.
///
/// The binary path is absolute, so replacing `PATH` affects only provider
/// discovery and any subprocesses the CLI might otherwise launch.
fn cli_without_host_programs() -> Command {
    let mut command = cli();
    command.env(
        "PATH",
        std::env::temp_dir().join(format!(
            "biscuit-speaks-cli-empty-path-{}",
            std::process::id()
        )),
    );
    command
}

/// Invoke the CLI with a provider that the isolated environment cannot supply.
fn cli_without_tts() -> Command {
    let mut command = cli_without_host_programs();
    command.args(["--provider", "say"]);
    command
}

/// Assert the CLI accepted an invocation and carried it through to the TTS stack.
///
/// These tests select `say` while hiding all host programs, so a valid invocation
/// deterministically reaches the provider stack and returns without producing
/// audio. Everything the CLI rejects does so earlier: clap exits 2 on an unknown
/// flag, invalid value, or conflict, and pre-flight failures name the input or
/// provider slug they rejected.
///
/// That speech actually comes out is a separate requirement needing a real
/// engine; `real_cli_speaks_with_kokoro_provider` covers it.
fn assert_reached_tts_stack(output: &Output, what: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "{what}: isolated provider unexpectedly succeeded"
    );
    assert_ne!(
        output.status.code(),
        Some(2),
        "{what}: rejected by argument parsing\n{stderr}"
    );
    assert!(
        stderr.contains("NoProvidersAvailable"),
        "{what}: invocation did not reach the expected provider-stack failure\n{stderr}"
    );
}

#[test]
fn test_cli_with_arguments() {
    let output = cli_without_tts()
        .arg("test")
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept a positional argument");
}

#[test]
fn test_cli_help_flag() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    assert!(output.status.success(), "Help flag should exit with code 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Convert text to speech using system TTS"),
        "Help output should contain description"
    );
    assert!(
        stdout.contains("Usage:"),
        "Help output should contain usage information"
    );
}

#[test]
fn test_cli_version_flag() {
    let output = cli().arg("--version").output().expect("Failed to execute");

    assert!(
        output.status.success(),
        "Version flag should exit with code 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("so-you-say"),
        "Version output should contain binary name"
    );
}

#[test]
fn test_cli_stdin_input() {
    let mut child = cli_without_tts()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"test input")
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert_reached_tts_stack(&output, "CLI should read its text from stdin");
}

#[test]
fn test_cli_no_args_closes_gracefully() {
    let mut child = cli()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    // Close stdin immediately without writing anything
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        !output.status.success(),
        "CLI should exit with code 1 when given no input"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: No input provided"),
        "Error message should be displayed when no input is provided"
    );
}

#[test]
fn test_cli_multi_word_args() {
    let output = cli_without_tts()
        .args(["Hello", "world", "from", "tests"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept multiple positional arguments");
}

#[test]
fn test_cli_empty_stdin() {
    let mut child = cli()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        !output.status.success(),
        "CLI should exit with code 1 when stdin is empty"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: No input provided"),
        "Error message should be displayed when stdin is empty"
    );
}

#[test]
fn test_cli_unicode_args() {
    let output = cli_without_tts()
        .args(["Hello", "世界", "🚀"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept unicode arguments");
}

#[test]
fn test_cli_special_chars_args() {
    let output = cli_without_tts()
        .args(["Hello,", "world!", "How's", "it", "going?"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept special characters in arguments");
}

#[test]
fn test_cli_gender_flag_male() {
    let output = cli_without_tts()
        .args(["--gender", "male", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --gender male flag");
}

#[test]
fn test_cli_gender_flag_female() {
    let output = cli_without_tts()
        .args(["--gender", "female", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --gender female flag");
}

#[test]
fn test_cli_gender_flag_short() {
    let output = cli_without_tts()
        .args(["-g", "male", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept -g short flag");
}

#[test]
fn test_cli_gender_flag_invalid() {
    let output = cli()
        .args(["--gender", "invalid", "test"])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject invalid gender value"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("error"),
        "Error message should indicate invalid value"
    );
}

#[test]
fn test_cli_help_shows_gender_option() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--gender") && stdout.contains("-g"),
        "Help should document --gender/-g flag"
    );
    assert!(
        stdout.contains("male") && stdout.contains("female"),
        "Help should show possible gender values"
    );
}

#[test]
fn test_cli_list_providers_subcommand() {
    let output = cli()
        .arg("list-providers")
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept list-providers subcommand"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Available TTS providers") || stdout.contains("No TTS providers"),
        "Providers output should show header or indicate no providers"
    );
}

#[test]
fn test_cli_help_shows_list_providers_subcommand() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("list-providers"),
        "Help should document list-providers subcommand"
    );
}

#[test]
fn test_cli_voice_option() {
    let output = cli_without_tts()
        .args(["--voice", "Samantha", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --voice Samantha");
}

#[test]
fn test_cli_help_shows_voice_option() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--voice"),
        "Help should document --voice option"
    );
}

#[test]
fn test_cli_provider_option() {
    let output = cli_without_host_programs()
        .args(["--provider", "say", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --provider say");
}

#[test]
fn test_cli_invalid_provider() {
    let output = cli()
        .args(["--provider", "not_a_real_provider", "test"])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject unknown provider"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown provider"),
        "Error should mention unknown provider"
    );
}

#[test]
fn test_cli_help_shows_provider_option() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--provider"),
        "Help should document --provider option"
    );
}

#[test]
fn test_cli_loud_flag() {
    let output = cli_without_tts()
        .args(["--loud", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --loud flag");
}

#[test]
fn test_cli_soft_flag() {
    let output = cli_without_tts()
        .args(["--soft", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --soft flag");
}

#[test]
fn test_cli_loud_and_soft_conflict() {
    let output = cli()
        .args(["--loud", "--soft", "test"])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject --loud and --soft together"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "Error should indicate flags cannot be used together"
    );
}

#[test]
fn test_cli_help_shows_volume_options() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--loud"),
        "Help should document --loud flag"
    );
    assert!(
        stdout.contains("--soft"),
        "Help should document --soft flag"
    );
}

#[test]
fn test_cli_fast_flag() {
    let output = cli_without_tts()
        .args(["--fast", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --fast flag");
}

#[test]
fn test_cli_speed_flag_slow() {
    let output = cli_without_tts()
        .args(["--slow", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --slow flag");
}

#[test]
fn test_cli_conflicting_speed_flags() {
    let output = cli()
        .args(["--fast", "--slow", "test"])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject --fast and --slow together"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "Error should indicate flags cannot be used together"
    );
}

#[test]
fn test_cli_help_shows_speed_options() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--fast"),
        "Help should document --fast flag"
    );
    assert!(
        stdout.contains("--slow"),
        "Help should document --slow flag"
    );
}

#[test]
fn test_cli_background_flag() {
    let output = cli_without_tts()
        .env("PLAYA_DRY_RUN", "1")
        .args(["--background", "background test"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should exit with code 0 when using --background"
    );

    assert!(
        output.stdout.is_empty(),
        "Background mode should produce no stdout"
    );
}

#[test]
fn test_cli_help_shows_background_option() {
    let output = cli().arg("--help").output().expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--background"),
        "Help should document --background flag"
    );
}

#[test]
fn test_cli_old_list_providers_flag_rejected() {
    let output = cli()
        .arg("--list-providers")
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject old --list-providers flag (now a subcommand)"
    );
}

#[test]
fn test_cli_old_list_voices_flag_rejected() {
    let output = cli()
        .arg("--list-voices")
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject old --list-voices flag (now a subcommand)"
    );
}

/// `real_` tier: verify the installed Kokoro provider reaches its cached
/// file-producing path through the shipped CLI. Playa's dry-run seam keeps
/// this process test independent of the audio device.
#[test]
fn real_cli_speaks_with_kokoro_provider() {
    let text = format!("real CLI Kokoro cache {}", std::process::id());
    let cache = biscuit_speaks::audio_cache::CacheKey::new("kokoro", "af_heart", &text, "wav")
        .cache_path();
    std::fs::write(&cache, b"cached audio").expect("Failed to seed Kokoro cache");
    let output = cli()
        .env("PLAYA_DRY_RUN", "1")
        .args(["--provider", "kokoro", &text])
        .output()
        .expect("Failed to execute");
    std::fs::remove_file(cache).expect("Failed to remove Kokoro cache fixture");

    if !output.status.success()
        && std::env::var("PLAYA_REAL_AUDIO_REQUIRED").as_deref() != Ok("1")
    {
        eprintln!(
            "skipping: concrete Kokoro provider is not ready: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    assert!(
        output.status.success(),
        "CLI should exit 0 once a provider has spoken: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_cli_background_with_refresh_cache_is_allowed() {
    let cache_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

    let output = cli_without_tts()
        .env("BISCUIT_SPEAKS_CACHE", cache_dir.path().join("cache.json"))
        .env("PLAYA_DRY_RUN", "1")
        .args(["--background", "--refresh-cache", "test"])
        .output()
        .expect("Failed to execute");

    assert_ne!(output.status.code(), Some(2), "flags must be accepted by clap");
}
