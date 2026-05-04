mod api;
mod daemon;

use std::net::SocketAddr;
use std::sync::Arc;

use biscuit_clipboard::{History, Storage};
use clap::Parser;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

use api::AppState;

#[derive(Parser, Debug)]
#[command(name = "clipper", about = "Biscuit clipboard background service")]
struct Args {
    #[arg(long, default_value = "0")]
    port: u16,

    #[arg(long)]
    pid_file: Option<String>,

    #[arg(long)]
    port_file: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let daemon_files = create_daemon_files(&args);

    if daemon_files.is_already_running() {
        eprintln!(
            "clipper is already running (PID {:?})",
            daemon_files.read_pid()
        );
        std::process::exit(1);
    }

    let _lock = match daemon_files.acquire_lock() {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("Failed to acquire lock: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = daemon_files.write_pid() {
        eprintln!("Failed to write PID file: {e}");
        std::process::exit(1);
    }

    let storage = match Storage::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize storage: {e}");
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState {
        history: Mutex::new(History::new()),
        storage,
    });

    let app = api::router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {addr}: {e}");
            daemon_files.cleanup().ok();
            std::process::exit(1);
        }
    };

    let actual_port = listener.local_addr().unwrap().port();
    if let Err(e) = daemon_files.write_port(actual_port) {
        eprintln!("Failed to write port file: {e}");
    }

    println!("clipper listening on 127.0.0.1:{actual_port}");

    let df_sigint = daemon_files.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\nShutting down clipper...");
        df_sigint.cleanup().ok();
        std::process::exit(0);
    });

    #[cfg(unix)]
    {
        let df_sigterm = daemon_files.clone();
        tokio::spawn(async move {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .unwrap()
                .recv()
                .await;
            println!("Received SIGTERM, shutting down...");
            df_sigterm.cleanup().ok();
            std::process::exit(0);
        });
    }

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {e}");
    }

    daemon_files.cleanup().ok();
}

fn create_daemon_files(args: &Args) -> daemon::DaemonFiles {
    if let Some(pid_path) = &args.pid_file {
        let runtime_dir = std::path::PathBuf::from(pid_path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        daemon::DaemonFiles::with_runtime_dir(runtime_dir)
    } else {
        daemon::DaemonFiles::new().unwrap_or_else(|e| {
            eprintln!("Failed to initialize daemon files: {e}");
            std::process::exit(1);
        })
    }
}
