//! Review-11 finding 1 — the *initial* composition launch's file-backed
//! system-prompt artifacts must still be on disk when the first child starts.
//!
//! `construct_argv_and_system_prompt` writes the delivery temp file and records
//! only its **path** into the argv (Codex's `model_instructions_file`) or the
//! child environment (Gemini's `GEMINI_SYSTEM_MD`). Until `CommandPhase` took
//! ownership of the artifact vector, the `NamedTempFile` dropped when command
//! construction returned — before `provider_run_handoff` reached the spawn — and
//! the provider read nothing while the run still reported success.
//!
//! The seam is only observable from outside the wrapper by making the child
//! echo the *content* it finds at the recorded path: a recorder that logged the
//! variable or the flag alone passes just as happily against a dangling path.
//! These rows spawn the real `claudine` binary against fake providers, so they
//! exercise the production phase boundary rather than a reconstruction of it.
//!
//! The fake providers are `/bin/sh` scripts, so the whole file is Unix-only.

#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::time::Duration;
mod common;
use common::{CliProcessFixture, write, write_executable};

/// The sentinel the fake provider has to read back out of the delivered file.
const SENTINEL: &str = "SYSPROMPT-SURVIVED-TO-SPAWN";

/// A fake `gemini` that echoes the content of the file `GEMINI_SYSTEM_MD` names.
///
/// Gemini's delivery is `EnvVarFile` in both modes, so the environment variable
/// is the only channel and the file is the only place the bytes live.
fn write_gemini_reader(bin_dir: &Path, log: &Path) {
    write_executable(
        &bin_dir.join("gemini"),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; esac\n\
             if [ ! -t 0 ]; then while IFS= read -r _; do :; done; fi\n\
             printf 'sysprompt-path=%s\\n' \"${{GEMINI_SYSTEM_MD:-unset}}\" >> {log}\n\
             if [ -f \"$GEMINI_SYSTEM_MD\" ]; then\n  \
             printf 'sysprompt-read=' >> {log}\n  \
             while IFS= read -r line; do printf '%s ' \"$line\" >> {log}; done < \"$GEMINI_SYSTEM_MD\"\n  \
             printf '\\n' >> {log}\n\
             fi\nexit 0\n",
            log = log.display(),
        ),
    );
}

/// A fake `codex` that echoes the content of the file its replacement config
/// key names.
///
/// Codex's `replace` delivery is `ConfigKeyFile`, i.e. one argv token shaped
/// `model_instructions_file=<path>`.
fn write_codex_reader(bin_dir: &Path, log: &Path) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; esac\n\
             if [ ! -t 0 ]; then while IFS= read -r _; do :; done; fi\n\
             for a in \"$@\"; do\n  case \"$a\" in\n    \
             model_instructions_file=*)\n      f=\"${{a#model_instructions_file=}}\"\n      \
             printf 'sysprompt-path=%s\\n' \"$f\" >> {log}\n      \
             if [ -f \"$f\" ]; then\n        \
             printf 'sysprompt-read=' >> {log}\n        \
             while IFS= read -r line; do printf '%s ' \"$line\" >> {log}; done < \"$f\"\n        \
             printf '\\n' >> {log}\n      \
             fi\n      ;;\n  esac\ndone\nexit 0\n",
            log = log.display(),
        ),
    );
}

/// Both file-backed mechanisms a direct `claudine compose` can select, each
/// asserted on the bytes the child could actually read.
#[test]
fn direct_compose_keeps_its_file_backed_system_prompt_readable_at_spawn() {
    for (provider_flag, delivery_flag, write_reader) in [
        (
            "--gemini",
            "--append-system-prompt",
            write_gemini_reader as fn(&Path, &Path),
        ),
        (
            "--codex",
            "--replace-system-prompt",
            write_codex_reader as fn(&Path, &Path),
        ),
    ] {
        let fixture = CliProcessFixture::named("compose-system-prompt-lifetime");
        fixture.seed_user_config();
        // The scoped temp directory the delivery writes into is derived from the
        // launch workspace, so the run has to happen inside its own repo — both
        // to keep the artifacts out of the developer's checkout and so the path
        // the child is handed is the one this row staged.
        fixture.initialize_repository();

        let log = fixture.cwd().join("events.log");
        write_reader(fixture.bin_dir(), &log);

        let md_file = fixture.cwd().join("doc.md");
        write(&md_file, "---\ntitle: sysprompt lifetime\n---\nBody\n");
        let sysprompt = fixture.cwd().join("sysprompt.md");
        write(&sysprompt, &format!("{SENTINEL}\n"));

        fixture
            .command_builder()
            // Keep host-program discovery out of this process regression. The
            // fake providers use only `/bin/sh` built-ins after launch.
            .fake_only_path()
            .build()
            .args([
                "compose",
                provider_flag,
                delivery_flag,
                sysprompt.to_str().unwrap(),
                md_file.to_str().unwrap(),
            ])
            .timeout(Duration::from_secs(25))
            .assert()
            .success();

        let lines: Vec<String> = fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();

        // Fixture check: the row is only meaningful if this provider actually
        // chose a file-backed mechanism on this run.
        assert!(
            lines.iter().any(|l| l.starts_with("sysprompt-path=")
                && !l.ends_with("=unset")),
            "[{provider_flag}] fixture check: the provider must have been handed a \
             system-prompt *path*; log:\n{lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("sysprompt-read=") && l.contains(SENTINEL)),
            "[{provider_flag}] the child must still be able to read the file its \
             launch names; log:\n{lines:#?}"
        );
    }
}
