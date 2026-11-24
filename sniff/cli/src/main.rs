use sniff_cli::commands;

fn main() {
    // SAFETY: the Tokio runtime and its threads are built below; nothing else
    // runs yet. See the function's docs.
    unsafe { sniff::programs::git_program::name_installation_for_gix() };

    // NOTE: The CLI is designed as a single-shot command tool.  Most command
    // paths execute synchronous git/filesystem/subprocess work directly in the
    // async `run` function because there is no concurrent async work to
    // justify `tokio::task::spawn_blocking` overhead.  The Tokio runtime is
    // used here primarily for the minority of commands that perform async
    // HTTP calls (remote provider queries, dependency enrichment).  If future
    // modes introduce long-lived concurrent work (progress UI, cancellation,
    // etc.), heavyweight local detection should be moved behind
    // `spawn_blocking` only for paths that concurrently await network ops.
    // The runtime is built by hand, not by `#[tokio::main]`, because the
    // installation hint above must be set before any runtime thread exists.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    if let Err(e) = runtime.block_on(commands::run()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
