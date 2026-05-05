use claudine::mcp::defaults;
use color_eyre::eyre::{Result, eyre};
use serde_json::json;

use crate::log;

use super::{DefaultArgs, current_repo_root};

pub(super) fn run_default(args: DefaultArgs, json_output: bool) -> Result<()> {
    if args.repo {
        let repo_root = current_repo_root()?.ok_or_else(|| eyre!("failed to resolve repo root"))?;
        defaults::set_repo_defaults(&repo_root, args.ids.clone())?;
        if json_output {
            log::data(&serde_json::to_string_pretty(&json!({
                "scope": "repo",
                "repo_root": repo_root,
                "defaults": args.ids,
            }))?);
        } else {
            log::data(&format!("Repo defaults set: {}", args.ids.join(", ")));
        }
    } else {
        defaults::set_user_defaults(args.ids.clone())?;
        if json_output {
            log::data(&serde_json::to_string_pretty(&json!({
                "scope": "user",
                "defaults": args.ids,
            }))?);
        } else {
            log::data(&format!("User defaults set: {}", args.ids.join(", ")));
        }
    }
    Ok(())
}
