//! The `dmls` binary: argument parsing, logging setup, and the stdio server.
//!
//! stdout is reserved for LSP framing; logging goes to stderr or, with
//! `--log-file`, to a file. The only stdout writes outside the LSP loop are
//! `--version` / `--help`, which exit before the server starts.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lsp_server::Connection;
use tracing_subscriber::EnvFilter;

const USAGE: &str = "\
dmls — the Darkmatter Language Server (LSP 3.17 over stdio)

USAGE:
    dmls [OPTIONS]

OPTIONS:
    --stdio               Accepted for client compatibility (stdio is the
                          only transport; this flag is a no-op)
    --config <PATH>       Use PATH instead of discovering .dmls.toml at the
                          workspace root
    --log-level <LEVEL>   Log filter (error|warn|info|debug|trace, or any
                          RUST_LOG-style directive); overrides RUST_LOG
    --log-file <PATH>     Append logs to PATH instead of stderr
    --bench-index <DIR>   Index DIR and print R-6 stage timings, then exit
                          (add --json for the machine-readable shape)
    --json                With --bench-index, emit JSON instead of a summary
    --gen-corpus <TIER> <DIR>
                          Write a deterministic synthetic corpus (tiny-100,
                          small-1k, vault-5k, dense-5k, pathological-1k,
                          large-20k) into DIR, then exit
    -V, --version         Print version and exit
    -h, --help            Print this help and exit
";

#[derive(Debug, Default)]
struct CliArgs {
    version: bool,
    help: bool,
    log_level: Option<String>,
    log_file: Option<PathBuf>,
    config: Option<PathBuf>,
    bench_index: Option<PathBuf>,
    json: bool,
    gen_corpus: Option<(String, PathBuf)>,
}

impl CliArgs {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self::default();
        let mut args = args;
        while let Some(arg) = args.next() {
            let (flag, inline_value) = match arg.split_once('=') {
                Some((flag, value)) => (flag.to_string(), Some(value.to_string())),
                None => (arg, None),
            };
            let mut take_value = || match inline_value.clone() {
                Some(value) => Ok(value),
                None => args.next().ok_or(format!("{flag} requires a value")),
            };
            match flag.as_str() {
                "--stdio" => {}
                "-V" | "--version" => parsed.version = true,
                "-h" | "--help" => parsed.help = true,
                "--log-level" => parsed.log_level = Some(take_value()?),
                "--log-file" => parsed.log_file = Some(PathBuf::from(take_value()?)),
                "--config" => parsed.config = Some(PathBuf::from(take_value()?)),
                "--bench-index" => parsed.bench_index = Some(PathBuf::from(take_value()?)),
                "--json" => parsed.json = true,
                "--gen-corpus" => {
                    let tier = take_value()?;
                    let dir = args.next().ok_or("--gen-corpus requires TIER and DIR")?;
                    parsed.gen_corpus = Some((tier, PathBuf::from(dir)));
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(parsed)
    }
}

fn init_logging(args: &CliArgs) -> Result<(), String> {
    let filter = match &args.log_level {
        Some(level) => {
            EnvFilter::try_new(level).map_err(|e| format!("invalid --log-level: {e}"))?
        }
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false);
    match &args.log_file {
        Some(path) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("cannot open --log-file {}: {e}", path.display()))?;
            builder.with_writer(std::sync::Mutex::new(file)).init();
        }
        None => builder.with_writer(std::io::stderr).init(),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = match CliArgs::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if args.help {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.version {
        println!("dmls {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if let Err(message) = init_logging(&args) {
        eprintln!("error: {message}");
        return ExitCode::from(2);
    }

    if let Some((tier_name, dir)) = &args.gen_corpus {
        return run_gen_corpus(tier_name, dir);
    }
    if let Some(dir) = &args.bench_index {
        return run_bench_index(dir, args.json);
    }

    let (connection, io_threads) = Connection::stdio();
    let options = dmls::RunOptions {
        config_path: args.config,
    };
    let outcome = dmls::run_server(connection, options);
    let joined = io_threads.join();
    match (outcome, joined) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (Err(error), _) => {
            tracing::error!("server terminated: {error}");
            eprintln!("dmls: {error}");
            ExitCode::FAILURE
        }
        (Ok(()), Err(error)) => {
            eprintln!("dmls: I/O threads failed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs `--bench-index`: index a directory and print timings, then exit.
///
/// Bench output goes to stdout (there is no LSP session in this mode).
fn run_bench_index(dir: &Path, json: bool) -> ExitCode {
    if !dir.is_dir() {
        eprintln!("error: --bench-index path is not a directory: {}", dir.display());
        return ExitCode::from(2);
    }
    let report = dmls::bench_index(dir);
    if json {
        println!("{}", report.to_json());
    } else {
        println!("{}", report.to_summary());
    }
    ExitCode::SUCCESS
}

/// Runs `--gen-corpus`: materialize a synthetic corpus tier, then exit.
fn run_gen_corpus(tier_name: &str, dir: &Path) -> ExitCode {
    let Some(tier) = dmls::CorpusTier::parse(tier_name) else {
        eprintln!(
            "error: unknown corpus tier: {tier_name}\n  known tiers: {}",
            dmls::CorpusTier::NAMES.join(", ")
        );
        return ExitCode::from(2);
    };
    match dmls::generate_corpus(tier, dir) {
        Ok(count) => {
            println!("wrote {count} files to {}", dir.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: failed to generate corpus: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<CliArgs, String> {
        CliArgs::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn test_parse_all_flags() {
        let args = parse(&[
            "--stdio",
            "--log-level",
            "debug",
            "--log-file",
            "/tmp/dmls.log",
            "--config",
            "/tmp/custom.toml",
        ])
        .unwrap();
        assert_eq!(args.log_level.as_deref(), Some("debug"));
        assert_eq!(args.log_file, Some(PathBuf::from("/tmp/dmls.log")));
        assert_eq!(args.config, Some(PathBuf::from("/tmp/custom.toml")));
        assert!(!args.version);
    }

    #[test]
    fn test_parse_equals_form() {
        let args = parse(&["--log-level=trace", "--config=./x.toml"]).unwrap();
        assert_eq!(args.log_level.as_deref(), Some("trace"));
        assert_eq!(args.config, Some(PathBuf::from("./x.toml")));
    }

    #[test]
    fn test_parse_version_and_help() {
        assert!(parse(&["--version"]).unwrap().version);
        assert!(parse(&["-V"]).unwrap().version);
        assert!(parse(&["--help"]).unwrap().help);
    }

    #[test]
    fn test_parse_rejects_unknown_and_missing_value() {
        assert!(parse(&["--bogus"]).is_err());
        assert!(parse(&["--log-level"]).is_err());
    }

    #[test]
    fn test_parse_bench_index_and_json() {
        let args = parse(&["--bench-index", "/tmp/vault", "--json"]).unwrap();
        assert_eq!(args.bench_index, Some(PathBuf::from("/tmp/vault")));
        assert!(args.json);
    }

    #[test]
    fn test_parse_gen_corpus_takes_two_values() {
        let args = parse(&["--gen-corpus", "tiny-100", "/tmp/out"]).unwrap();
        assert_eq!(
            args.gen_corpus,
            Some(("tiny-100".to_string(), PathBuf::from("/tmp/out")))
        );
        // Missing DIR is an error.
        assert!(parse(&["--gen-corpus", "tiny-100"]).is_err());
    }
}
