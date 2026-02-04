use clap::Args;
use color_eyre::eyre::Result;

use claudine_lib::linking::{self, LinkScope};

/// Arguments for the link subcommand.
#[derive(Args)]
pub struct LinkArgs {
    /// Show what would happen without creating links.
    #[arg(long)]
    pub dry_run: bool,
    /// Filter to a specific skill name.
    #[arg(long)]
    pub provider: Option<String>,
    /// Show detailed output.
    #[arg(short, long)]
    pub verbose: bool,
    /// Replace duplicate real directories with symlinks.
    #[arg(long)]
    pub replace_duplicates: bool,
}

/// Link skills and commands across providers.
pub fn run(args: LinkArgs) -> Result<()> {
    let scope = LinkScope::User;
    let filter = args.provider.as_deref();

    let report = linking::link_skills(scope, filter, args.dry_run)?;

    if args.dry_run {
        println!("Dry run \u{2014} no changes made:");
    }

    // LinkReport implements Display via format_report()
    println!("{report}");

    Ok(())
}
