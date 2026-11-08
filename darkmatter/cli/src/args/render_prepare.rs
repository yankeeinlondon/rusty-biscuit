//! The render arguments after clap: classified positionals, resolved policy,
//! and the remaining options, with nothing read from disk.
//!
//! Semantic style preparation (reading `--style` files, parsing setter paths)
//! is not done here, so its failures can be reported through the run's sink.

use crate::args::policy::{self, PolicyCommand, ResolvedPolicy};
use crate::args::{Feature, RawStyleSetter, RenderOptions, classify_render_args};
use biscuit_terminal::output::OutputTarget;
use std::ffi::OsString;
use std::path::PathBuf;

/// What a render invocation asked for, whichever spelling (`md` or
/// `md render`) it used.
#[derive(Debug, Clone)]
pub struct PreparedRenderOptions {
    /// The input file, or `None` for no input.
    pub input: Option<PathBuf>,
    /// The `style…=VALUE` setters, in command-line order.
    pub setters: Vec<RawStyleSetter>,
    /// The `--style` references, in command-line order.
    pub styles: Vec<OsString>,
    pub policy: ResolvedPolicy,
    pub show: bool,
    pub indent: Option<usize>,
    pub mermaid: bool,
    pub features: Vec<Feature>,
    /// The requested target; `None` resolves to the default at dispatch.
    pub output: Option<OutputTarget>,
}

impl PreparedRenderOptions {
    /// Classifies `options`' positionals and resolves its policy.
    ///
    /// ## Errors
    ///
    /// A usage error (exit 2): more than one input, or a policy conflict.
    pub fn prepare(options: &RenderOptions) -> Result<Self, clap::Error> {
        let input = classify_render_args(&options.args, &options.literal_args)?;
        let policy = policy::resolve_for(PolicyCommand::Render, &[], &options.deny, &[], None)?;
        Ok(Self {
            input: input.input,
            setters: input.setters,
            styles: options.style.clone(),
            policy,
            show: options.show,
            indent: options.indent,
            mermaid: options.mermaid,
            features: options.features.clone(),
            output: options.output,
        })
    }

    /// The input as a data output's `file` names it: `None` for stdin.
    pub fn input_file(&self) -> Option<String> {
        self.input.as_ref().filter(|path| path.to_str() != Some("-")).map(|path| path.display().to_string())
    }
}
