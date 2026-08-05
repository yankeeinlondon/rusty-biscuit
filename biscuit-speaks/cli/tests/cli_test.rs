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

/// Assert the CLI accepted an invocation and carried it through to the TTS stack.
///
/// Argument handling can only be exercised by a command that then tries to
/// speak, and exit 0 requires a TTS engine — macOS has `say`, a stock Linux or
/// Windows CI runner has nothing. Acceptance is observable without one, because
/// everything the CLI rejects it rejects *before* consulting a provider: clap
/// exits 2 on an unknown flag, an invalid value, or a conflicting pair, and the
/// pre-flight checks name the input or provider slug they turned down. A run
/// that reaches the provider stack has therefore parsed its arguments and built
/// its config, which is all argument handling promises.
///
/// That speech actually comes out is a separate requirement needing a real
/// engine; `real_cli_speaks_with_default_provider` covers it.
fn assert_reached_tts_stack(output: &Output, what: &str) {
    if output.status.success() {
        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(2),
        "{what}: rejected by argument parsing\n{stderr}"
    );
    assert!(
        !stderr.contains("No input provided"),
        "{what}: the text never reached the TTS stack\n{stderr}"
    );
    assert!(
        !stderr.contains("Unknown provider"),
        "{what}: provider selection rejected the invocation\n{stderr}"
    );
}

#[test]
fn test_cli_with_arguments() {
    let output = cli().arg("test").output().expect("Failed to execute");

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
    let mut child = cli()
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
    let output = cli()
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
    let output = cli()
        .args(["Hello", "世界", "🚀"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept unicode arguments");
}

#[test]
fn test_cli_special_chars_args() {
    let output = cli()
        .args(["Hello,", "world!", "How's", "it", "going?"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept special characters in arguments");
}

#[test]
fn test_cli_gender_flag_male() {
    let output = cli()
        .args(["--gender", "male", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --gender male flag");
}

#[test]
fn test_cli_gender_flag_female() {
    let output = cli()
        .args(["--gender", "female", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --gender female flag");
}

#[test]
fn test_cli_gender_flag_short() {
    let output = cli()
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
    let output = cli()
        .args(["--voice", "Samantha", "test"])
        .output()
        .expect("Failed to execute");

    // "Samantha" is a macOS-only voice; on platforms whose providers do not
    // offer it, every provider fails and the CLI exits non-zero. We only verify
    // the flag is accepted (parsed without a usage error), not that playback of
    // an OS-specific voice succeeds.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("Usage:"),
        "CLI should accept --voice option without a parse error"
    );
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
    let output = cli()
        .args(["--provider", "say", "test"])
        .output()
        .expect("Failed to execute");

    // This may or may not succeed depending on whether Say is available
    // We just verify it doesn't crash
    let _ = output.status;
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
    let output = cli()
        .args(["--loud", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --loud flag");
}

#[test]
fn test_cli_soft_flag() {
    let output = cli()
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
    let output = cli()
        .args(["--fast", "test"])
        .output()
        .expect("Failed to execute");

    assert_reached_tts_stack(&output, "CLI should accept --fast flag");
}

// The next two names end at `slow` rather than reading `..._slow_flag` /
// `..._and_slow_conflict`: the L1 tier filter excludes `test(/slow_/)`, which is
// a substring match, so any name containing `slow_` is silently dropped from
// `just test` and from CI. Both spent their whole lives unrun that way.
#[test]
fn test_cli_speed_flag_slow() {
    let output = cli()
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
    let output = cli()
        .args(["--background", "background test"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should exit with code 0 when using --background"
    );

    // In background mode, stdout/stderr are redirected to null on the child process,
    // so the parent should produce no output.
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

/// `real_` tier: the exit-0 end-to-end assertion the argument tests above used
/// to carry. Synthesizing anything needs an installed TTS engine, so this runs
/// under `just test-real` only and skips cleanly where no provider is detected.
#[test]
fn real_cli_speaks_with_default_provider() {
    let providers = cli()
        .arg("list-providers")
        .output()
        .expect("Failed to execute");

    if String::from_utf8_lossy(&providers.stdout).contains("No TTS providers") {
        eprintln!("skipping: no TTS provider available on this host");
        return;
    }

    let output = cli().arg("test").output().expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should exit 0 once a provider has spoken: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_cli_background_with_refresh_cache_is_allowed() {
    let output = cli()
        .args(["--background", "--refresh-cache", "test"])
        .output()
        .expect("Failed to execute");

    // --background with --refresh-cache is now allowed (background refresh)
    assert!(
        output.status.success(),
        "CLI should allow --background with --refresh-cache"
    );
}
