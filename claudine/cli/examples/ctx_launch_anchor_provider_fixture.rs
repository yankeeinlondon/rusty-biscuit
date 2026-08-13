use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const MODE_ENV: &str = "CLAUDINE_PROVIDER_FIXTURE_MODE";
const STDIN_FILE_ENV: &str = "CLAUDINE_STDIN_FILE";
const COUNT_FILE_ENV: &str = "CLAUDINE_COUNT_FILE";
const ARGS_FILE_ENV: &str = "CLAUDINE_PROVIDER_FIXTURE_ARGS_FILE";

fn required_path(name: &str) -> Result<PathBuf, String> {
    env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} is required"))
}

fn capture_args(args: &[String]) -> Result<(), String> {
    let Some(path) = env::var_os(ARGS_FILE_ENV).map(PathBuf::from) else {
        return Ok(());
    };
    let encoded = serde_json::to_vec(args).map_err(|error| error.to_string())?;
    fs::write(path, encoded).map_err(|error| error.to_string())
}

fn read_stdin() -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn run() -> Result<(), String> {
    let mode = env::var(MODE_ENV).map_err(|_| format!("{MODE_ENV} is required"))?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    capture_args(&args)?;
    let stdin = read_stdin()?;
    let stdin_file = required_path(STDIN_FILE_ENV)?;

    match mode.as_str() {
        "codex" => fs::write(stdin_file, stdin).map_err(|error| error.to_string()),
        "goose" => {
            let mut capture = args.join("\n").into_bytes();
            if !capture.is_empty() && !stdin.is_empty() {
                capture.push(b'\n');
            }
            capture.extend_from_slice(&stdin);
            fs::write(stdin_file, capture).map_err(|error| error.to_string())?;
            io::stdout()
                .write_all(b"Generated body\n")
                .map_err(|error| error.to_string())
        }
        "counting-codex" => {
            let count_file = required_path(COUNT_FILE_ENV)?;
            let count = fs::read_to_string(&count_file)
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())
                .unwrap_or(0)
                + 1;
            fs::write(count_file, count.to_string()).map_err(|error| error.to_string())?;
            fs::write(stdin_file, stdin).map_err(|error| error.to_string())
        }
        _ => Err(format!("unknown fixture mode: {mode}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
