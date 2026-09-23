//! Login-shell classification probes behind `has_alias`, `has_builtin_function`,
//! `has_user_function`, and `can_execute` (more-context spec R8-R10, R17).
//!
//! One probe starts the request's login shell with its profile loaded and asks
//! which kinds of command a name is. The name travels to the shell only as the
//! value of an environment variable, and every dialect's query reads it as
//! data through its own introspection builtin (`type`, `whence`, `functions`,
//! `Get-Command`), so the name is never parsed as shell syntax and the named
//! command is never run. Every failure — no shell, unsupported dialect, spawn
//! error, timeout, non-zero exit — reads as "not that kind of command".

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::launcher::run_shell_query;

/// Per-query bound, the same two seconds alias resolution allows.
const SHELL_QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Carries the probed name into the shell as data.
const NAME_VARIABLE: &str = "DARKMATTER_SHELL_PROBE_NAME";

/// Prefix of each answer line; the suffix is the kind of command.
const ANSWER_PREFIX: &str = "darkmatter-shell-probe:";

const BASH_QUERY: &str = r#"for kind in $(type -a -t -- "$DARKMATTER_SHELL_PROBE_NAME" 2>/dev/null); do printf 'darkmatter-shell-probe:%s\n' "$kind"; done"#;

const ZSH_QUERY: &str = r#"whence -wa -- "$DARKMATTER_SHELL_PROBE_NAME" 2>/dev/null | while IFS= read -r line; do print -r -- "darkmatter-shell-probe:${line##*: }"; done"#;

// Fish implements `alias` as a function whose description starts with
// `alias `; that description is the only thing distinguishing the two.
const FISH_QUERY: &str = r#"set -l name $DARKMATTER_SHELL_PROBE_NAME
if functions -q -- $name
    set -l details (functions --details --verbose -- $name)
    if string match -q -- 'alias *' "$details[5]"
        echo darkmatter-shell-probe:alias
    else
        echo darkmatter-shell-probe:function
    end
end
if contains -- $name (builtin --names)
    echo darkmatter-shell-probe:builtin
end"#;

// `Get-Command -Name` takes wildcards, so the name is escaped into a literal
// pattern. Cmdlets are PowerShell's builtin commands (R9).
const POWERSHELL_QUERY: &str = r#"$name = [Management.Automation.WildcardPattern]::Escape($env:DARKMATTER_SHELL_PROBE_NAME)
foreach ($command in @(Get-Command -Name $name -All -CommandType Alias,Function,Filter,Cmdlet -ErrorAction SilentlyContinue)) {
    switch ("$($command.CommandType)") {
        'Alias' { 'darkmatter-shell-probe:alias' }
        'Function' { 'darkmatter-shell-probe:function' }
        'Filter' { 'darkmatter-shell-probe:function' }
        'Cmdlet' { 'darkmatter-shell-probe:builtin' }
    }
}"#;

/// The kinds of command one name is in the login shell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ShellClasses {
    pub(crate) alias: bool,
    pub(crate) builtin: bool,
    pub(crate) function: bool,
}

impl ShellClasses {
    pub(crate) fn any(self) -> bool {
        self.alias || self.builtin || self.function
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Dialect {
    /// The dialect named by a shell executable, or `None` for any other shell.
    fn of(shell: &str) -> Option<Self> {
        let file_name = shell.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
        let stem = file_name.strip_suffix(".exe").unwrap_or(&file_name);
        match stem {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "pwsh" | "powershell" => Some(Self::PowerShell),
            _ => None,
        }
    }

    fn arguments(self) -> Vec<String> {
        match self {
            Self::Bash => vec!["-ic".into(), BASH_QUERY.into()],
            Self::Zsh => vec!["-ic".into(), ZSH_QUERY.into()],
            Self::Fish => vec!["-i".into(), "-c".into(), FISH_QUERY.into()],
            // `-EncodedCommand` sidesteps Windows command-line quoting; the
            // profile still loads because `-NoProfile` is absent.
            Self::PowerShell => vec![
                "-NoLogo".into(),
                "-NonInteractive".into(),
                "-EncodedCommand".into(),
                encode_powershell_command(POWERSHELL_QUERY),
            ],
        }
    }
}

/// The request's shell-probe authority.
///
/// Built from the request's captured environment; the default authority is
/// disabled and never launches a shell, so a resolution context that was not
/// given one cannot fall back to ambient process state.
#[derive(Debug, Clone)]
pub(crate) struct ShellProbe {
    /// `None` disables probing.
    request: Option<ShellRequest>,
    timeout: Duration,
    launches: Arc<AtomicUsize>,
    #[cfg(test)]
    child_environment: Vec<(String, Option<String>)>,
}

#[derive(Debug, Clone)]
struct ShellRequest {
    /// `$SHELL`, when set and non-empty.
    shell: Option<String>,
    /// `PATH`, used only for the Windows PowerShell fallback lookup.
    #[cfg_attr(not(windows), allow(dead_code))]
    path: Option<String>,
}

impl Default for ShellProbe {
    fn default() -> Self {
        Self {
            request: None,
            timeout: SHELL_QUERY_TIMEOUT,
            launches: Arc::default(),
            #[cfg(test)]
            child_environment: Vec::new(),
        }
    }
}

impl ShellProbe {
    /// An authority for the login shell named by `environment`.
    pub(crate) fn from_environment(environment: &HashMap<String, String>) -> Self {
        let lookup = |wanted: &str| {
            environment
                .iter()
                .find(|(key, _)| {
                    if cfg!(windows) { key.eq_ignore_ascii_case(wanted) } else { key.as_str() == wanted }
                })
                .map(|(_, value)| value.clone())
        };
        Self {
            request: Some(ShellRequest {
                shell: lookup("SHELL").filter(|shell| !shell.is_empty()),
                path: lookup("PATH"),
            }),
            ..Self::default()
        }
    }

    /// Classifies `name`, launching the login shell at most once.
    pub(crate) fn classify(&self, name: &str) -> ShellClasses {
        if name.is_empty() || name.contains('\0') {
            return ShellClasses::default();
        }
        let Some((program, dialect)) = self.request.as_ref().and_then(ShellRequest::resolve) else {
            return ShellClasses::default();
        };
        self.launches.fetch_add(1, Ordering::Relaxed);
        let mut command = Command::new(program);
        command.args(dialect.arguments()).env(NAME_VARIABLE, name);
        #[cfg(test)]
        for (key, value) in &self.child_environment {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }
        match run_shell_query(&mut command, self.timeout) {
            Some((status, stdout)) if status.success() => parse_answers(&stdout),
            _ => ShellClasses::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets (or, with `None`, removes) a variable in the shell's environment,
    /// so tests can point profile lookup at a fixture home.
    #[cfg(all(test, unix))]
    pub(crate) fn with_child_environment(mut self, key: &str, value: Option<&str>) -> Self {
        self.child_environment.push((key.to_string(), value.map(str::to_string)));
        self
    }

    /// Shell launches this authority (and its clones) has made.
    #[cfg(test)]
    pub(crate) fn launches(&self) -> usize {
        self.launches.load(Ordering::Relaxed)
    }
}

impl ShellRequest {
    fn resolve(&self) -> Option<(PathBuf, Dialect)> {
        if let Some(shell) = &self.shell {
            return Dialect::of(shell).map(|dialect| (PathBuf::from(shell), dialect));
        }
        #[cfg(windows)]
        {
            // R17: `pwsh` first, then Windows PowerShell. The bare
            // `powershell` name resolves `powershell.exe` through `PATHEXT`.
            let path = self.path.as_ref()?;
            let cwd = std::env::temp_dir();
            ["pwsh", "powershell"]
                .into_iter()
                .find_map(|name| which::which_in(name, Some(path), &cwd).ok())
                .map(|program| (program, Dialect::PowerShell))
        }
        #[cfg(not(windows))]
        None
    }
}

fn parse_answers(stdout: &[u8]) -> ShellClasses {
    let mut classes = ShellClasses::default();
    for line in String::from_utf8_lossy(stdout).lines() {
        match line.trim().strip_prefix(ANSWER_PREFIX) {
            Some("alias") => classes.alias = true,
            Some("builtin") => classes.builtin = true,
            Some("function") => classes.function = true,
            _ => {}
        }
    }
    classes
}

/// Base64 of the UTF-16LE script, as `-EncodedCommand` requires.
fn encode_powershell_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0_u32, |value, (index, byte)| {
            value | u32::from(*byte) << (16 - 8 * index)
        });
        for position in 0..4 {
            if position <= chunk.len() {
                encoded.push(char::from(ALPHABET[(value >> (18 - 6 * position) & 0x3f) as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialects_are_recognized_by_executable_name_only() {
        assert_eq!(Dialect::of("/bin/bash"), Some(Dialect::Bash));
        assert_eq!(Dialect::of("/usr/local/bin/zsh"), Some(Dialect::Zsh));
        assert_eq!(Dialect::of("fish"), Some(Dialect::Fish));
        assert_eq!(Dialect::of(r"C:\Program Files\PowerShell\7\pwsh.exe"), Some(Dialect::PowerShell));
        assert_eq!(Dialect::of("POWERSHELL.EXE"), Some(Dialect::PowerShell));
        assert_eq!(Dialect::of("/bin/sh"), None);
        assert_eq!(Dialect::of("/bin/dash"), None);
        assert_eq!(Dialect::of("/opt/bash-tools/tcsh"), None);
    }

    #[test]
    fn answers_are_whole_marker_lines() {
        let stdout = b"welcome\r\ndarkmatter-shell-probe:alias\r\n  darkmatter-shell-probe:builtin \nsay darkmatter-shell-probe:function\ndarkmatter-shell-probe:file\n";
        assert_eq!(
            parse_answers(stdout),
            ShellClasses { alias: true, builtin: true, function: false }
        );
    }

    #[test]
    fn powershell_encoding_is_base64_of_utf16le() {
        // `[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes('dir'))`
        assert_eq!(encode_powershell_command("dir"), "ZABpAHIA");
        assert_eq!(encode_powershell_command("ab"), "YQBiAA==");
        assert_eq!(encode_powershell_command(""), "");
    }

    #[test]
    fn a_default_authority_never_launches() {
        let probe = ShellProbe::default();
        assert_eq!(probe.classify("cd"), ShellClasses::default());
        assert_eq!(probe.launches(), 0);
    }

    /// AC10: with `$SHELL` unset, Unix has no login shell to ask.
    #[cfg(unix)]
    #[test]
    fn missing_shell_is_false_without_a_launch_on_unix() {
        for environment in [HashMap::new(), HashMap::from([("SHELL".to_string(), String::new())])] {
            let probe = ShellProbe::from_environment(&environment);
            assert_eq!(probe.classify("cd"), ShellClasses::default());
            assert_eq!(probe.launches(), 0);
        }
    }

    #[test]
    fn unsupported_shells_and_unrepresentable_names_are_false_without_a_launch() {
        let probe = ShellProbe::from_environment(&HashMap::from([("SHELL".to_string(), "/bin/tcsh".to_string())]));
        assert_eq!(probe.classify("cd"), ShellClasses::default());
        let probe = ShellProbe::from_environment(&HashMap::from([("SHELL".to_string(), "/bin/bash".to_string())]));
        assert_eq!(probe.classify(""), ShellClasses::default());
        assert_eq!(probe.classify("a\0b"), ShellClasses::default());
        assert_eq!(probe.launches(), 0);
    }

    #[test]
    fn a_shell_that_cannot_launch_is_false() {
        let dir = tempfile::tempdir().unwrap();
        let shell = dir.path().join("bash");
        let probe = ShellProbe::from_environment(&HashMap::from([(
            "SHELL".to_string(),
            shell.to_string_lossy().into_owned(),
        )]));
        assert_eq!(probe.classify("cd"), ShellClasses::default());
        assert_eq!(probe.launches(), 1);
    }

    /// AC10 and AC34 against the real bash, zsh, fish, and PowerShell
    /// executables, each loading only a profile written into a fixture home.
    ///
    /// A shell that is not installed on this host is skipped with a message:
    /// the dialect is then covered only by the stub tests. PowerShell runs on
    /// Unix only, where `XDG_CONFIG_HOME` relocates its profile; on Windows the
    /// profile lives in the Documents known folder, which no environment
    /// variable can point at a fixture.
    #[cfg(unix)]
    mod real_shells {
        use std::path::PathBuf;
        use std::time::Instant;

        use super::*;

        const NONE: ShellClasses = ShellClasses { alias: false, builtin: false, function: false };

        struct Fixture {
            _home: tempfile::TempDir,
            home: PathBuf,
            probe: ShellProbe,
        }

        /// `profile` is written to the dialect's per-user startup file under
        /// a fresh home; every variable that could redirect startup to a real
        /// profile points into, or is removed from, that home.
        fn home_fixture(shell: &str, profile: &str) -> Option<Fixture> {
            let Ok(program) = which::which(shell) else {
                eprintln!("skipping: `{shell}` is not installed on this host");
                return None;
            };
            let home_dir = tempfile::tempdir().unwrap();
            let home = home_dir.path().to_path_buf();
            let config = home.join(".config");
            let profile_path = match Dialect::of(shell).unwrap() {
                Dialect::Bash => home.join(".bashrc"),
                Dialect::Zsh => home.join(".zshrc"),
                Dialect::Fish => config.join("fish").join("config.fish"),
                Dialect::PowerShell => config.join("powershell").join("Microsoft.PowerShell_profile.ps1"),
            };
            std::fs::create_dir_all(profile_path.parent().unwrap()).unwrap();
            std::fs::write(&profile_path, profile).unwrap();
            let home_text = home.to_string_lossy().into_owned();
            let config_text = config.to_string_lossy().into_owned();
            let probe = ShellProbe::from_environment(&HashMap::from([(
                "SHELL".to_string(),
                program.to_string_lossy().into_owned(),
            )]))
            .with_timeout(Duration::from_secs(30))
            .with_child_environment("HOME", Some(&home_text))
            .with_child_environment("ZDOTDIR", Some(&home_text))
            .with_child_environment("XDG_CONFIG_HOME", Some(&config_text))
            .with_child_environment("XDG_DATA_HOME", Some(&config_text))
            .with_child_environment("BASH_ENV", None)
            .with_child_environment("ENV", None);
            Some(Fixture { _home: home_dir, home, probe })
        }

        fn classes(alias: bool, builtin: bool, function: bool) -> ShellClasses {
            ShellClasses { alias, builtin, function }
        }

        const POSIX_PROFILE: &str = "alias ll='ls -l'\n\
            alias cdp='cd -P'\n\
            alias gone='definitely-missing-binary-zzz'\n\
            mkcd() { mkdir -p \"$1\" && cd \"$1\"; }\n\
            pushd() { builtin pushd \"$@\"; }\n";

        const FISH_PROFILE: &str = "alias ll 'ls -l'\n\
            alias gone 'definitely-missing-binary-zzz'\n\
            function mkcd; mkdir -p $argv[1]; and cd $argv[1]; end\n";

        const POWERSHELL_PROFILE: &str = "Set-Alias -Name ll -Value Get-ChildItem\n\
            Set-Alias -Name gone -Value Definitely-Missing-Zzz\n\
            function mkcd { param($p) New-Item -ItemType Directory $p; Set-Location $p }\n";

        /// Aliases to binaries, builtins, and missing binaries; builtins; user
        /// functions, including one that shadows a builtin.
        #[test]
        fn posix_shells_classify_aliases_builtins_and_functions() {
            for shell in ["bash", "zsh"] {
                let Some(fixture) = home_fixture(shell, POSIX_PROFILE) else { continue };
                for (name, expected) in [
                    ("ll", classes(true, false, false)),
                    ("cdp", classes(true, false, false)),
                    ("gone", classes(true, false, false)),
                    ("cd", classes(false, true, false)),
                    ("mkcd", classes(false, false, true)),
                    ("pushd", classes(false, true, true)),
                    ("definitely-not-a-command-zzz", NONE),
                ] {
                    assert_eq!(fixture.probe.classify(name), expected, "{shell}: {name}");
                }
            }
        }

        #[test]
        fn fish_classifies_aliases_builtins_and_functions() {
            let Some(fixture) = home_fixture("fish", FISH_PROFILE) else { return };
            assert_eq!(fixture.probe.classify("ll"), classes(true, false, false));
            assert_eq!(fixture.probe.classify("gone"), classes(true, false, false));
            assert_eq!(fixture.probe.classify("mkcd"), classes(false, false, true));
            assert!(fixture.probe.classify("set").builtin);
            assert_eq!(fixture.probe.classify("definitely-not-a-command-zzz"), NONE);
        }

        #[test]
        fn powershell_classifies_aliases_cmdlets_and_functions() {
            let Some(fixture) = home_fixture("pwsh", POWERSHELL_PROFILE) else { return };
            assert!(fixture.probe.classify("ll").alias);
            assert!(fixture.probe.classify("gone").alias);
            assert!(fixture.probe.classify("Get-ChildItem").builtin);
            assert!(fixture.probe.classify("mkcd").function);
            assert_eq!(fixture.probe.classify("definitely-not-a-command-zzz"), NONE);
            assert_eq!(fixture.probe.classify("Get-*"), NONE, "a wildcard is a literal name");
        }

        /// AC34: names shaped like command substitution, separators, quotes,
        /// newlines, and options are data. None may create the marker.
        #[test]
        fn injection_shaped_names_never_execute() {
            for (shell, profile) in
                [("bash", POSIX_PROFILE), ("zsh", POSIX_PROFILE), ("fish", FISH_PROFILE), ("pwsh", POWERSHELL_PROFILE)]
            {
                let Some(fixture) = home_fixture(shell, profile) else { continue };
                let marker = fixture.home.join("pwned");
                let m = marker.to_string_lossy();
                let names = [
                    format!("$(touch {m})"),
                    format!("`touch {m}`"),
                    format!(";touch {m}"),
                    format!("x; touch {m}"),
                    format!("x && touch {m}"),
                    format!("x | touch {m}"),
                    format!("'; touch {m}; '"),
                    format!("\"; touch {m}; \""),
                    format!("\ntouch {m}"),
                    format!("x\ntouch {m}\n"),
                    format!("(touch {m})"),
                    format!("x=$(touch {m})"),
                    format!("$(New-Item {m})"),
                    format!("x; New-Item {m}"),
                    "-rm".to_string(),
                    "--help".to_string(),
                    "*".to_string(),
                ];
                for name in &names {
                    let classified = fixture.probe.classify(name);
                    assert!(!marker.exists(), "{shell} executed {name:?}");
                    assert_eq!(classified, NONE, "{shell}: {name:?}");
                }
            }
        }

        /// A profile that floods stdout still yields the answer, which is
        /// printed after it.
        #[test]
        fn a_noisy_profile_still_answers() {
            let noise = "i=0; while [ $i -lt 20000 ]; do echo 'profile noise darkmatter-shell-probe'; i=$((i+1)); done\n";
            for shell in ["bash", "zsh"] {
                let Some(fixture) = home_fixture(shell, &format!("{noise}{POSIX_PROFILE}")) else { continue };
                assert_eq!(fixture.probe.classify("ll"), classes(true, false, false), "{shell}");
            }
        }

        #[test]
        fn a_hanging_or_exiting_profile_is_false_within_the_bound() {
            for shell in ["bash", "zsh"] {
                let Some(fixture) = home_fixture(shell, &format!("{POSIX_PROFILE}sleep 60\n")) else { continue };
                let probe = fixture.probe.clone().with_timeout(Duration::from_millis(500));
                let started = Instant::now();
                assert_eq!(probe.classify("ll"), NONE, "{shell}");
                assert!(started.elapsed() < Duration::from_secs(10), "{shell}: {:?}", started.elapsed());

                let Some(fixture) = home_fixture(shell, &format!("{POSIX_PROFILE}exit 0\n")) else { continue };
                assert_eq!(fixture.probe.classify("ll"), NONE, "{shell}");
            }
        }

        /// A background job the profile starts is killed with the query.
        #[test]
        fn a_detached_profile_child_is_cleaned_up() {
            for shell in ["bash", "zsh"] {
                let dir = tempfile::tempdir().unwrap();
                let pid_file = dir.path().join("child.pid");
                let profile = format!("{POSIX_PROFILE}sleep 60 &\necho $! > '{}'\n", pid_file.display());
                let Some(fixture) = home_fixture(shell, &profile) else { continue };

                assert_eq!(fixture.probe.classify("ll"), classes(true, false, false), "{shell}");

                let pid: libc::pid_t = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
                wait_for_exit(pid, shell);
            }
        }

        fn wait_for_exit(pid: libc::pid_t, shell: &str) {
            let deadline = Instant::now() + Duration::from_secs(10);
            // SAFETY: signal 0 only checks for existence.
            while unsafe { libc::kill(pid, 0) } == 0 {
                assert!(Instant::now() < deadline, "{shell}: profile child {pid} survived the probe");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    /// AC10 (Windows): `pwsh`, then `powershell`, then nothing. The stubs are
    /// `.cmd` files resolved through `PATHEXT`, so no real PowerShell or user
    /// profile is involved.
    #[cfg(windows)]
    #[test]
    fn windows_falls_back_from_pwsh_to_powershell_to_false() {
        let both = tempfile::tempdir().unwrap();
        let powershell_only = tempfile::tempdir().unwrap();
        let neither = tempfile::tempdir().unwrap();
        std::fs::write(both.path().join("pwsh.cmd"), "@echo darkmatter-shell-probe:alias\r\n").unwrap();
        std::fs::write(both.path().join("powershell.cmd"), "@echo darkmatter-shell-probe:builtin\r\n").unwrap();
        std::fs::write(powershell_only.path().join("powershell.cmd"), "@echo darkmatter-shell-probe:builtin\r\n").unwrap();

        let probe = |dir: &std::path::Path| {
            ShellProbe::from_environment(&HashMap::from([(
                "Path".to_string(),
                dir.to_string_lossy().into_owned(),
            )]))
            .with_timeout(Duration::from_secs(30))
        };

        let pwsh = probe(both.path());
        assert_eq!(pwsh.classify("ls"), ShellClasses { alias: true, ..ShellClasses::default() });
        let powershell = probe(powershell_only.path());
        assert_eq!(powershell.classify("ls"), ShellClasses { builtin: true, ..ShellClasses::default() });
        let none = probe(neither.path());
        assert_eq!(none.classify("ls"), ShellClasses::default());
        assert_eq!(none.launches(), 0);
    }
}
