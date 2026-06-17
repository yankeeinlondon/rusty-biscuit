use sniff_cli::commands;

#[tokio::main]
async fn main() {
    // NOTE: The CLI is designed as a single-shot command tool.  Most command
    // paths execute synchronous git/filesystem/subprocess work directly in the
    // async `run` function because there is no concurrent async work to
    // justify `tokio::task::spawn_blocking` overhead.  The Tokio runtime is
    // used here primarily for the minority of commands that perform async
    // HTTP calls (remote provider queries, dependency enrichment).  If future
    // modes introduce long-lived concurrent work (progress UI, cancellation,
    // etc.), heavyweight local detection should be moved behind
    // `spawn_blocking` only for paths that concurrently await network ops.
    if let Err(e) = commands::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
