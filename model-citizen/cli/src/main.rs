//! Model Citizen CLI - Local LLM model management.
//!
//! Manage local LLM models across Ollama, LM Studio, and Llama.cpp.

mod args;
mod commands;
pub mod output;

use args::{Cli, Commands};
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use color_eyre::eyre::Result;
use commands::run::RunOptions;
use inquire::InquireError;

pub const COMPLETIONS_HELP: &str = r#"
SHELL COMPLETIONS

Enable dynamic shell completions for model.

Examples:
  # Bash - add to ~/.bashrc or ~/.bash_profile
  echo 'source <(COMPLETE=bash model)' >> ~/.bashrc

  # Zsh - add to ~/.zshrc
  echo 'source <(COMPLETE=zsh model)' >> ~/.zshrc

  # Fish - add to config
  echo 'COMPLETE=fish model | source' >> ~/.config/fish/config.fish

  # Disable completions
  COMPLETE=0
"#;

#[tokio::main]
async fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();

    color_eyre::install()?;

    let cli = Cli::parse();

    let command_result = match cli.command {
        Commands::List {
            filter,
            runner,
            verbose,
            app,
            size,
        } => commands::list::run(filter, runner, cli.json, verbose, app, size).await,
        Commands::Info { model } => commands::info::run(&model, cli.json).await,
        Commands::Search {
            query,
            limit,
            sort,
            verbose,
        } => {
            let query = if query.is_empty() {
                None
            } else {
                Some(query.join(" "))
            };
            commands::search::run(query.as_deref(), limit, sort.into(), cli.json, verbose).await
        }
        Commands::Download {
            query,
            limit,
            sort,
            verbose,
            output,
            remove_partial,
        } => {
            let query = if query.is_empty() {
                None
            } else {
                Some(query.join(" "))
            };
            commands::download::run(
                query.as_deref(),
                limit,
                sort.into(),
                verbose,
                output.as_deref(),
                remove_partial,
            )
            .await
        }
        Commands::Remove {
            model,
            runner,
            force,
        } => commands::remove::run(&model, runner.as_deref(), force).await,
        Commands::Run {
            model,
            runner,
            host,
            port,
            ctx_size,
            threads,
            n_gpu_layers,
            api_key,
            llama_server_bin,
            no_browser,
            dry_run,
        } => {
            commands::run::run(RunOptions {
                model,
                runner,
                host,
                port,
                ctx_size,
                threads,
                n_gpu_layers,
                api_key,
                llama_server_bin,
                no_browser,
                dry_run,
            })
            .await
        }
        Commands::Completions => {
            print!("{}", COMPLETIONS_HELP.trim_start());
            Ok(())
        }
    };

    if let Err(err) = command_result {
        if is_prompt_interrupt(&err) {
            println!("Cancelled.");
            return Ok(());
        }
        return Err(err);
    }

    Ok(())
}

fn is_prompt_interrupt(err: &color_eyre::Report) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<InquireError>()
            .is_some_and(|inquire_err| {
                matches!(
                    inquire_err,
                    InquireError::OperationCanceled | InquireError::OperationInterrupted
                )
            })
    })
}
