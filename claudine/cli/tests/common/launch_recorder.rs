//! A shell-stub provider that records every launch Claudine makes, so a test
//! can assert the exact child argv and the prompt it was given.
//!
//! The stubs are `/bin/sh` scripts, so this module is Unix-only.

use std::fs;
use std::path::{Path, PathBuf};

use super::{CliProcessFixture, write_executable};

/// Install a stub `binary` in the fixture's `bin` directory that records each
/// launch, then runs `behavior`; returns the record directory.
///
/// Launch `n` (0-based, in launch order, shared by every stub in the fixture)
/// writes the binary name and then its arguments to `launches/launch-NNN`
/// (`\x1f` separated), its `AGENT_PARAMS` to `launches/params-N`, and its
/// stdin, which is where the composed prompt arrives, to
/// `launches/prompt-NNN`. `behavior` is shell code that can read `$n`.
pub fn install(fixture: &CliProcessFixture, binary: &str, behavior: &str) -> PathBuf {
    let dir = fixture.home().join("launches");
    fs::create_dir_all(&dir).unwrap();
    write_executable(
        &fixture.bin_dir().join(binary),
        &format!(
            "#!/bin/sh\n\
             dir='{dir}'\n\
             n=$(cat \"$dir/count\" 2>/dev/null || echo 0)\n\
             echo $((n + 1)) > \"$dir/count\"\n\
             file=$(printf 'launch-%03d' \"$n\")\n\
             {{ printf '%s\\037' '{binary}'; for arg in \"$@\"; do printf '%s\\037' \"$arg\"; done; }} > \"$dir/$file\"\n\
             printf '%s' \"$AGENT_PARAMS\" > \"$dir/params-$n\"\n\
             cat > \"$dir/$(printf 'prompt-%03d' \"$n\")\"\n\
             {behavior}\n",
            dir = dir.display()
        ),
    );
    dir
}

/// Every recorded launch's argv (binary name first), in launch order; empty
/// when nothing was launched.
pub fn launches(dir: &Path) -> Vec<Vec<String>> {
    recorded(dir, "launch-")
        .iter()
        .map(|text| text.split('\u{1f}').filter(|arg| !arg.is_empty()).map(str::to_owned).collect())
        .collect()
}

/// Every recorded launch's stdin, in launch order.
pub fn prompts(dir: &Path) -> Vec<String> {
    recorded(dir, "prompt-")
}

fn recorded(dir: &Path, prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with(prefix))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.iter().map(|name| fs::read_to_string(dir.join(name)).unwrap()).collect()
}

/// How many times the contiguous `run` appears in `args`.
pub fn occurrences(args: &[String], run: &[&str]) -> usize {
    args.windows(run.len())
        .filter(|window| window.iter().zip(run).all(|(arg, want)| arg == want))
        .count()
}
