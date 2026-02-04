use clap::Args;
use color_eyre::eyre::Result;

/// Arguments for shell completion generation.
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

/// Generate shell completions.
pub fn run(args: CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    let mut cmd = super::super::Cli::command();
    clap_complete::generate(args.shell, &mut cmd, "claudine", &mut std::io::stdout());
    Ok(())
}
