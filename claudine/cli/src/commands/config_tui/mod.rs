use clap::Args;
use color_eyre::eyre::Result;

#[derive(Debug, Args)]
pub struct ConfigArgs {}

pub async fn run(_args: ConfigArgs) -> Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    if !config_path.exists() {
        return super::init_wizard::run_initialization().await;
    }

    eprintln!(
        "Config TUI not yet implemented. Edit {} directly.",
        config_path.display()
    );
    eprintln!("Use `claudine config` to launch the TUI in a future release.");
    Ok(())
}
