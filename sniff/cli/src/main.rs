mod args;
mod commands;
mod output;

#[tokio::main]
async fn main() {
    if let Err(e) = commands::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
