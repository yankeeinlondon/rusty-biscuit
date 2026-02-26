//! Run command - launch a local GGUF model with llama-server.

use clap::ValueEnum;
use clap_complete::engine::CompletionCandidate;
use color_eyre::eyre::{Result, WrapErr, eyre};
use inquire::Select;
use model_citizen::{
    Config, ModelFormat, ModelRegistry, UnifiedModel, gguf,
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
};
use std::ffi::OsStr;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Child;
use tokio::time::{sleep, timeout};

const READINESS_TIMEOUT: Duration = Duration::from_secs(15);
const READINESS_INTERVAL: Duration = Duration::from_millis(300);
const PROBE_TIMEOUT: Duration = Duration::from_millis(600);

/// Runner filter for selecting model sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RunnerFilter {
    Ollama,
    Lmstudio,
    Llamacpp,
}

impl RunnerFilter {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Lmstudio => "lmstudio",
            Self::Llamacpp => "llamacpp",
        }
    }
}

/// Options for the `model run` command.
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub model: Option<String>,
    pub runner: Option<RunnerFilter>,
    pub host: String,
    pub port: u16,
    pub ctx_size: Option<u32>,
    pub threads: Option<usize>,
    pub n_gpu_layers: Option<i32>,
    pub api_key: Option<String>,
    pub llama_server_bin: Option<PathBuf>,
    pub no_browser: bool,
    pub dry_run: bool,
}

/// Completion candidates for `model run [model]`.
///
/// Returns runnable GGUF models across enabled scanners. Candidates include
/// unique model names, model IDs, and file names when available.
pub fn run_model_candidates() -> Vec<CompletionCandidate> {
    let config = match Config::load() {
        Ok(config) => config,
        Err(_) => return Vec::new(),
    };

    let registry = match build_registry(&config, None) {
        Ok(registry) => registry,
        Err(_) => return Vec::new(),
    };

    let models = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .map(|runtime| runtime.block_on(async move { registry.scan_all().await }))
            .unwrap_or_default()
    })
    .join()
    .unwrap_or_default();

    let runnable = filter_runnable_models(models);
    build_model_completion_candidates(&runnable)
}

fn build_model_completion_candidates(models: &[UnifiedModel]) -> Vec<CompletionCandidate> {
    use std::collections::{BTreeSet, HashMap};

    let mut candidates = Vec::new();
    let mut seen_values = BTreeSet::new();
    let mut name_counts = HashMap::new();

    for model in models {
        *name_counts
            .entry(model.name.to_lowercase())
            .or_insert(0usize) += 1;
    }

    for model in models {
        let help = format!(
            "{} | {} | {} | {}",
            model.source.display_name(),
            model.quantization.as_str(),
            model.size_display(),
            model.path.display()
        );
        let help_text: clap::builder::StyledStr = help.clone().into();

        if seen_values.insert(model.id.clone()) {
            candidates
                .push(CompletionCandidate::new(model.id.clone()).help(Some(help_text.clone())));
        }

        if name_counts
            .get(&model.name.to_lowercase())
            .is_some_and(|count| *count == 1)
            && seen_values.insert(model.name.clone())
        {
            candidates
                .push(CompletionCandidate::new(model.name.clone()).help(Some(help_text.clone())));
        }

        if let Some(file_name) = model.path.file_name().and_then(|name| name.to_str()) {
            let file_name = file_name.to_string();
            if seen_values.insert(file_name.clone()) {
                candidates.push(CompletionCandidate::new(file_name).help(Some(help_text)));
            }
        }
    }

    candidates
}

/// Executes `model run`.
pub async fn run(options: RunOptions) -> Result<()> {
    let config = Config::load()?;
    let registry = build_registry(&config, options.runner)?;

    let runnable_models = filter_runnable_models(registry.scan_all().await);
    if runnable_models.is_empty() {
        return Err(eyre!(
            "No runnable GGUF models found. Download a GGUF model first with `model download`, or verify model paths."
        ));
    }

    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let selected = resolve_model_to_run(&runnable_models, options.model.as_deref(), interactive)?;

    let path_env = std::env::var_os("PATH");
    let binary =
        resolve_llama_server_binary(options.llama_server_bin.as_deref(), path_env.as_deref())?;

    let capabilities = probe_llama_server_capabilities(&binary);
    let args = build_llama_server_args(&ServerLaunchConfig {
        model_path: selected.path.clone(),
        host: options.host.clone(),
        port: options.port,
        ctx_size: options.ctx_size,
        threads: options.threads,
        n_gpu_layers: options.n_gpu_layers,
        api_key: options.api_key.clone(),
        enable_webui: capabilities.has_webui_flag,
    });

    if options.dry_run {
        println!(
            "Selected model: {} ({}, {}, {})",
            selected.name,
            selected.size_display(),
            selected.quantization.as_str(),
            selected.source.as_str()
        );
        println!("Model path: {}", selected.path.display());
        println!("Command: {}", format_command_for_display(&binary, &args));
        return Ok(());
    }

    ensure_port_available(&options.host, options.port)?;

    let url = format!("http://{}:{}", options.host, options.port);
    println!("Starting llama-server with model: {}", selected.name);
    println!("Model path: {}", selected.path.display());
    println!("Server URL: {}", url);

    let mut child = spawn_llama_server(&binary, &args)?;

    let readiness_host = options.host.clone();
    let readiness_url = url.clone();
    let no_browser = options.no_browser;
    let readiness_task = tokio::spawn(async move {
        report_readiness_and_maybe_open_browser(
            &readiness_host,
            options.port,
            &readiness_url,
            no_browser,
        )
        .await;
    });

    let outcome = tokio::select! {
        status = child.wait() => {
            if !readiness_task.is_finished() {
                readiness_task.abort();
            }

            let status = status.wrap_err("Failed while waiting on llama-server")?;
            if status.success() {
                Ok(())
            } else {
                Err(eyre!("llama-server exited with status {status}"))
            }
        }
        ctrl_c = tokio::signal::ctrl_c() => {
            ctrl_c.wrap_err("Failed to listen for Ctrl-C")?;
            eprintln!("\nCtrl-C received. Stopping llama-server...");
            terminate_child(&mut child).await?;

            if !readiness_task.is_finished() {
                readiness_task.abort();
            }

            Ok(())
        }
    };

    outcome
}

fn build_registry(config: &Config, runner_filter: Option<RunnerFilter>) -> Result<ModelRegistry> {
    let mut registry = ModelRegistry::new();
    let mut added = false;

    if runner_filter.is_none() || runner_filter == Some(RunnerFilter::Ollama) {
        if config.scanners.ollama.enabled {
            registry.add_scanner(OllamaScanner::new(config));
            added = true;
        }
    }

    if runner_filter.is_none() || runner_filter == Some(RunnerFilter::Lmstudio) {
        if config.scanners.lmstudio.enabled {
            registry.add_scanner(LmStudioScanner::new(config));
            added = true;
        }
    }

    if runner_filter.is_none() || runner_filter == Some(RunnerFilter::Llamacpp) {
        if config.scanners.llamacpp.enabled {
            registry.add_scanner(LlamaCppScanner::new(config));
            added = true;
        }
    }

    if !added {
        if let Some(filter) = runner_filter {
            return Err(eyre!(
                "Runner '{}' is disabled in config. Enable the scanner or use a different runner.",
                filter.as_str()
            ));
        }
        return Err(eyre!(
            "No scanners are enabled. Enable at least one scanner in config.toml."
        ));
    }

    Ok(registry)
}

fn filter_runnable_models(models: Vec<UnifiedModel>) -> Vec<UnifiedModel> {
    let mut runnable: Vec<UnifiedModel> = models
        .into_iter()
        .filter(|model| model.format == ModelFormat::Gguf)
        .filter(|model| model.path.is_file())
        .filter(|model| is_runnable_gguf_path(&model.path))
        .collect();

    runnable.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    runnable
}

fn is_runnable_gguf_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        || gguf::is_gguf_file(path)
}

fn resolve_model_to_run(
    models: &[UnifiedModel],
    model_query: Option<&str>,
    interactive: bool,
) -> Result<UnifiedModel> {
    match model_query {
        None => {
            if models.len() == 1 {
                return Ok(models[0].clone());
            }

            if !interactive {
                return Err(eyre!(
                    "Multiple runnable GGUF models found ({}). Specify one: `model run <model>`.",
                    models.len()
                ));
            }

            select_model_interactively(models, "Select a model to run:")
        }
        Some(query) => {
            let matches = find_model_matches(models, query);
            if matches.is_empty() {
                return Err(eyre!("No runnable model matches '{}'", query));
            }

            if matches.len() == 1 {
                return Ok(matches[0].clone());
            }

            if let Some(exact) = find_exact_match(&matches, query) {
                return Ok(exact.clone());
            }

            if !interactive {
                let names = matches
                    .iter()
                    .map(|m| format!("{} ({})", m.name, m.source.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(eyre!(
                    "Ambiguous model '{}' matched {} models: {}",
                    query,
                    matches.len(),
                    names
                ));
            }

            select_model_interactively(&matches, &format!("'{}' matches multiple models:", query))
        }
    }
}

fn find_model_matches(models: &[UnifiedModel], query: &str) -> Vec<UnifiedModel> {
    let query_lower = query.to_lowercase();

    let mut matches: Vec<UnifiedModel> = models
        .iter()
        .filter(|m| {
            m.name.to_lowercase().contains(&query_lower)
                || m.id.to_lowercase().contains(&query_lower)
                || m.path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|filename| filename.to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect();

    matches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    matches
}

fn find_exact_match<'a>(matches: &'a [UnifiedModel], query: &str) -> Option<&'a UnifiedModel> {
    let query_lower = query.to_lowercase();

    let mut exact = matches.iter().filter(|m| {
        m.name.eq_ignore_ascii_case(query)
            || m.id.eq_ignore_ascii_case(query)
            || m.path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|filename| filename.eq_ignore_ascii_case(query))
            || m.name.to_lowercase() == query_lower
    });

    let first = exact.next()?;
    if exact.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn select_model_interactively(models: &[UnifiedModel], prompt: &str) -> Result<UnifiedModel> {
    let options: Vec<String> = models
        .iter()
        .map(|model| {
            format!(
                "{} ({}, {}, {}, {})",
                model.name,
                model.size_display(),
                model.quantization.as_str(),
                model.source.display_name(),
                model.path.display()
            )
        })
        .collect();

    let selection = Select::new(prompt, options.clone()).prompt()?;

    let index = options
        .iter()
        .position(|opt| opt == &selection)
        .ok_or_else(|| eyre!("Invalid model selection"))?;

    Ok(models[index].clone())
}

fn resolve_llama_server_binary(
    override_path: Option<&Path>,
    path_env: Option<&OsStr>,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        if is_executable(path) {
            return Ok(path.to_path_buf());
        }

        return Err(eyre!(
            "--llama-server-bin points to a missing or non-executable file: {}",
            path.display()
        ));
    }

    find_in_path("llama-server", path_env).ok_or_else(|| {
        eyre!(
            "Could not find `llama-server` on PATH. Install llama.cpp or pass --llama-server-bin <path>."
        )
    })
}

fn find_in_path(executable: &str, path_env: Option<&OsStr>) -> Option<PathBuf> {
    let path_env = path_env?;

    for dir in std::env::split_paths(path_env) {
        let direct = dir.join(executable);
        if is_executable(&direct) {
            return Some(direct);
        }

        #[cfg(windows)]
        {
            let has_ext = direct.extension().is_some();
            if !has_ext {
                let path_ext =
                    std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
                for ext in path_ext.split(';').filter(|s| !s.is_empty()) {
                    let ext = ext.trim_start_matches('.');
                    let candidate = dir.join(format!("{executable}.{ext}"));
                    if is_executable(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(meta) = std::fs::metadata(path) {
            return meta.permissions().mode() & 0o111 != 0;
        }

        false
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[derive(Debug, Default)]
struct LlamaServerCapabilities {
    has_webui_flag: bool,
}

fn probe_llama_server_capabilities(binary: &Path) -> LlamaServerCapabilities {
    let output = StdCommand::new(binary).arg("--help").output();
    let Ok(output) = output else {
        return LlamaServerCapabilities::default();
    };

    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    LlamaServerCapabilities {
        has_webui_flag: help.contains("--webui"),
    }
}

#[derive(Debug)]
struct ServerLaunchConfig {
    model_path: PathBuf,
    host: String,
    port: u16,
    ctx_size: Option<u32>,
    threads: Option<usize>,
    n_gpu_layers: Option<i32>,
    api_key: Option<String>,
    enable_webui: bool,
}

fn build_llama_server_args(config: &ServerLaunchConfig) -> Vec<String> {
    let mut args = vec![
        "--model".to_string(),
        config.model_path.to_string_lossy().to_string(),
        "--host".to_string(),
        config.host.clone(),
        "--port".to_string(),
        config.port.to_string(),
    ];

    if config.enable_webui {
        args.push("--webui".to_string());
    }

    if let Some(ctx_size) = config.ctx_size {
        args.push("--ctx-size".to_string());
        args.push(ctx_size.to_string());
    }

    if let Some(threads) = config.threads {
        args.push("--threads".to_string());
        args.push(threads.to_string());
    }

    if let Some(n_gpu_layers) = config.n_gpu_layers {
        args.push("--n-gpu-layers".to_string());
        args.push(n_gpu_layers.to_string());
    }

    if let Some(api_key) = &config.api_key {
        args.push("--api-key".to_string());
        args.push(api_key.clone());
    }

    args
}

fn format_command_for_display(binary: &Path, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_escape(&binary.to_string_lossy()));
    parts.extend(args.iter().map(|arg| shell_escape(arg)));
    parts.join(" ")
}

fn shell_escape(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':' | '='))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn ensure_port_available(host: &str, port: u16) -> Result<()> {
    let bind_addr = format!("{}:{}", host, port);

    match std::net::TcpListener::bind(&bind_addr) {
        Ok(listener) => {
            drop(listener);
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => Err(eyre!(
            "Port {} is already in use on host '{}'. Try a different --port.",
            port,
            host
        )),
        Err(err) => Err(eyre!(
            "Could not bind to {} before launch: {}",
            bind_addr,
            err
        )),
    }
}

fn spawn_llama_server(binary: &Path, args: &[String]) -> Result<Child> {
    let mut command = tokio::process::Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command.spawn().wrap_err_with(|| {
        format!(
            "Failed to start llama-server using binary '{}'",
            binary.display()
        )
    })
}

async fn terminate_child(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let _ = StdCommand::new("kill")
                .arg("-INT")
                .arg(pid.to_string())
                .status();
        }
    }

    match timeout(Duration::from_secs(3), child.wait()).await {
        Ok(wait_result) => {
            let _ = wait_result?;
            Ok(())
        }
        Err(_) => {
            child
                .start_kill()
                .wrap_err("Failed to terminate llama-server")?;
            let _ = child.wait().await;
            Ok(())
        }
    }
}

async fn report_readiness_and_maybe_open_browser(
    host: &str,
    port: u16,
    url: &str,
    no_browser: bool,
) {
    match wait_for_server_readiness(host, port, READINESS_TIMEOUT).await {
        ServerReadiness::HealthOk => {
            println!("llama-server is ready: {url}");
            if no_browser {
                println!("Browser auto-open disabled (--no-browser).");
            } else if let Err(err) = open_browser(url) {
                eprintln!("Warning: failed to open browser automatically: {err}");
            } else {
                println!("Opened browser: {url}");
            }
        }
        ServerReadiness::Reachable(status) => {
            println!("llama-server is reachable at {url} (status {status} on /health)");
            if no_browser {
                println!("Browser auto-open disabled (--no-browser).");
            } else if let Err(err) = open_browser(url) {
                eprintln!("Warning: failed to open browser automatically: {err}");
            } else {
                println!("Opened browser: {url}");
            }
        }
        ServerReadiness::TcpReachable => {
            println!("llama-server is reachable on {url}");
            if no_browser {
                println!("Browser auto-open disabled (--no-browser).");
            } else if let Err(err) = open_browser(url) {
                eprintln!("Warning: failed to open browser automatically: {err}");
            } else {
                println!("Opened browser: {url}");
            }
        }
        ServerReadiness::TimedOut => {
            eprintln!(
                "Warning: llama-server did not become reachable at {} within {} seconds.",
                url,
                READINESS_TIMEOUT.as_secs()
            );
            eprintln!("You can open it manually when it is ready.");
        }
    }
}

enum ServerReadiness {
    HealthOk,
    Reachable(u16),
    TcpReachable,
    TimedOut,
}

async fn wait_for_server_readiness(
    host: &str,
    port: u16,
    timeout_window: Duration,
) -> ServerReadiness {
    let deadline = tokio::time::Instant::now() + timeout_window;

    loop {
        if let Some(status) = probe_http_status(host, port, "/health").await {
            if status == 200 {
                return ServerReadiness::HealthOk;
            }
            return ServerReadiness::Reachable(status);
        }

        if probe_tcp_port(host, port).await {
            return ServerReadiness::TcpReachable;
        }

        if tokio::time::Instant::now() >= deadline {
            return ServerReadiness::TimedOut;
        }

        sleep(READINESS_INTERVAL).await;
    }
}

async fn probe_tcp_port(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(addr))
        .await
        .is_ok_and(|result| result.is_ok())
}

async fn probe_http_status(host: &str, port: u16, path: &str) -> Option<u16> {
    let addr = format!("{}:{}", host, port);
    let mut stream = timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(addr))
        .await
        .ok()?
        .ok()?;

    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");

    timeout(PROBE_TIMEOUT, stream.write_all(request.as_bytes()))
        .await
        .ok()?
        .ok()?;

    let mut buffer = [0_u8; 512];
    let bytes_read = timeout(PROBE_TIMEOUT, stream.read(&mut buffer))
        .await
        .ok()?
        .ok()?;

    if bytes_read == 0 {
        return None;
    }

    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    let status_line = response.lines().next()?;
    parse_status_code(status_line)
}

fn parse_status_code(status_line: &str) -> Option<u16> {
    let mut parts = status_line.split_whitespace();
    let _http = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        StdCommand::new("open")
            .arg(url)
            .spawn()
            .wrap_err("Failed to execute `open`")?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        StdCommand::new("xdg-open")
            .arg(url)
            .spawn()
            .wrap_err("Failed to execute `xdg-open`")?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        StdCommand::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .wrap_err("Failed to execute `start`")?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(eyre!("Browser auto-open is not supported on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model_citizen::{ModelArchitecture, ModelFormat, ModelSource, QuantizationType};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "model-citizen-run-tests-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn make_model(
        name: &str,
        format: ModelFormat,
        source: ModelSource,
        path: &Path,
    ) -> UnifiedModel {
        UnifiedModel::new(
            name,
            1_000,
            QuantizationType::Q4Km,
            ModelArchitecture::Llama,
            source,
            format,
            path.to_path_buf(),
        )
    }

    #[test]
    fn command_argument_builder_includes_expected_flags() {
        let config = ServerLaunchConfig {
            model_path: PathBuf::from("/models/test.gguf"),
            host: "127.0.0.1".to_string(),
            port: 8080,
            ctx_size: Some(8192),
            threads: Some(8),
            n_gpu_layers: Some(42),
            api_key: Some("secret".to_string()),
            enable_webui: true,
        };

        let args = build_llama_server_args(&config);

        assert!(
            args.windows(2)
                .any(|w| w[0] == "--model" && w[1] == "/models/test.gguf")
        );
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--host" && w[1] == "127.0.0.1")
        );
        assert!(args.windows(2).any(|w| w[0] == "--port" && w[1] == "8080"));
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--ctx-size" && w[1] == "8192")
        );
        assert!(args.windows(2).any(|w| w[0] == "--threads" && w[1] == "8"));
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--n-gpu-layers" && w[1] == "42")
        );
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--api-key" && w[1] == "secret")
        );
        assert!(args.iter().any(|arg| arg == "--webui"));
    }

    #[test]
    fn runnable_filter_excludes_non_runnable_entries() {
        let dir = test_dir();

        let ext_gguf = dir.join("good.gguf");
        write_file(&ext_gguf, b"not-a-real-gguf-but-extension-counts");

        let magic_no_ext = dir.join("good-no-ext");
        write_file(&magic_no_ext, b"GGUF\0\0\0\0");

        let manifest_like = dir.join("manifest");
        write_file(&manifest_like, b"{\"schemaVersion\":2}");

        let safetensors = dir.join("model.safetensors");
        write_file(&safetensors, b"safetensors");

        let missing = dir.join("missing.gguf");

        let models = vec![
            make_model(
                "good-ext",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &ext_gguf,
            ),
            make_model(
                "good-header",
                ModelFormat::Gguf,
                ModelSource::LmStudio,
                &magic_no_ext,
            ),
            make_model(
                "manifest",
                ModelFormat::Gguf,
                ModelSource::Ollama,
                &manifest_like,
            ),
            make_model(
                "wrong-format",
                ModelFormat::Safetensors,
                ModelSource::LmStudio,
                &safetensors,
            ),
            make_model(
                "missing",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &missing,
            ),
        ];

        let runnable = filter_runnable_models(models);
        let names: Vec<String> = runnable.into_iter().map(|m| m.name).collect();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"good-ext".to_string()));
        assert!(names.contains(&"good-header".to_string()));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn model_matching_handles_exact_single_multiple_and_none() {
        let dir = test_dir();
        let phi2_path = dir.join("phi2.gguf");
        let phi2_mini_path = dir.join("phi2-mini.gguf");
        let llama_path = dir.join("llama3.gguf");

        write_file(&phi2_path, b"GGUF\0\0\0\0");
        write_file(&phi2_mini_path, b"GGUF\0\0\0\0");
        write_file(&llama_path, b"GGUF\0\0\0\0");

        let models = vec![
            make_model("phi2", ModelFormat::Gguf, ModelSource::LmStudio, &phi2_path),
            make_model(
                "phi2-mini",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &phi2_mini_path,
            ),
            make_model(
                "llama3",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &llama_path,
            ),
        ];

        let exact = resolve_model_to_run(&models, Some("phi2"), false).unwrap();
        assert_eq!(exact.name, "phi2");

        let single = resolve_model_to_run(&models, Some("mini"), false).unwrap();
        assert_eq!(single.name, "phi2-mini");

        let ambiguous = resolve_model_to_run(&models, Some("phi"), false).unwrap_err();
        assert!(ambiguous.to_string().contains("Ambiguous model"));

        let none = resolve_model_to_run(&models, Some("does-not-exist"), false).unwrap_err();
        assert!(none.to_string().contains("No runnable model matches"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn binary_resolution_prefers_explicit_override_over_path_lookup() {
        let dir = test_dir();
        let path_binary = dir.join("llama-server");
        let override_binary = dir.join("custom-llama-server");

        write_file(&path_binary, b"#!/bin/sh\n");
        write_file(&override_binary, b"#!/bin/sh\n");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path_binary, perms).unwrap();

            let mut perms = fs::metadata(&override_binary).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&override_binary, perms).unwrap();
        }

        let fake_path = std::env::join_paths([dir.clone()]).unwrap();

        let explicit =
            resolve_llama_server_binary(Some(&override_binary), Some(fake_path.as_os_str()))
                .unwrap();
        assert_eq!(explicit, override_binary);

        let from_path = resolve_llama_server_binary(None, Some(fake_path.as_os_str())).unwrap();
        assert_eq!(from_path, path_binary);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn completion_candidates_prefer_unique_names_and_always_include_ids() {
        let dir = test_dir();
        let unique_path = dir.join("unique.gguf");
        let dup_a_path = dir.join("dup-a.gguf");
        let dup_b_path = dir.join("dup-b.gguf");
        write_file(&unique_path, b"GGUF\0\0\0\0");
        write_file(&dup_a_path, b"GGUF\0\0\0\0");
        write_file(&dup_b_path, b"GGUF\0\0\0\0");

        let models = vec![
            make_model(
                "unique-model",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &unique_path,
            ),
            make_model(
                "dup-model",
                ModelFormat::Gguf,
                ModelSource::LlamaCpp,
                &dup_a_path,
            ),
            make_model(
                "dup-model",
                ModelFormat::Gguf,
                ModelSource::LmStudio,
                &dup_b_path,
            ),
        ];

        let candidates = build_model_completion_candidates(&models);
        let values: Vec<String> = candidates
            .iter()
            .filter_map(|c| c.get_value().to_str().map(ToString::to_string))
            .collect();

        assert!(values.contains(&"unique-model".to_string()));
        assert!(values.contains(&models[0].id));

        assert!(values.contains(&models[1].id));
        assert!(values.contains(&models[2].id));
        assert_eq!(
            values.iter().filter(|value| *value == "dup-model").count(),
            0
        );

        let _ = fs::remove_dir_all(dir);
    }
}
