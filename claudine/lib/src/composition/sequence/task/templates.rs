//! Template membership and current-state filling for sequence tasks.

use super::*;
use super::super::preflight::{PreparedShellSite, SiteMode, ShellSiteKind};
use crate::composition::lifecycle::shell_template::LifecycleShellTemplate;
use darkmatter::markdown::compose::shell_expansion::template::{
    FilledChain, LateValues, LookupEvaluator, fill_template,
};
use darkmatter::markdown::compose::{EffectiveStateBuilder, ResolvedShellValue, ShellCommandOrigin, ShellExpansionError};
use darkmatter::markdown::compose::expression::EvaluationSession;

impl TaskExecution<'_> {
    fn template_error(&self, site: &PreparedShellSite, error: ShellExpansionError) -> CompositionError {
        CompositionError::SequenceShellResolution {
            command: site.authored.clone(),
            task: self.label(),
            message: error.to_string(),
            source: Some(Box::new(MarkdownError::ShellExpansion(Box::new(error)))),
        }
    }

    fn authorized_template(&self, site: &PreparedShellSite, late: &LateValues) -> Result<LifecycleShellTemplate, CompositionError> {
        let template = site.template.as_ref().expect("template-mode sites carry a template");
        let Some(policy_root) = self.graph.template_authorization(template) else {
            return Err(CompositionError::SequenceShellResolution {
                command: site.authored.clone(), task: self.label(),
                message: "the authored template was not approved at sequence preflight".to_string(), source: None,
            });
        };
        let approved = LifecycleShellTemplate::approved(template.clone(), policy_root);
        Ok(match late.site(&site.id) {
            Some(values) => approved.with_late_values(values.clone()),
            None => approved,
        })
    }

    pub(super) fn prepare_primary_template(&self, index: usize, late: &LateValues) -> Result<Option<FilledChain>, CompositionError> {
        let Some(site) = self.task.shell_sites.iter().find(|site| site.id == format!("primary[{index}]") && site.mode == SiteMode::Template) else {
            return Ok(None);
        };
        let approved = self.authorized_template(site, late)?;
        let mut data = self.state.data().clone();
        if let Some(runtime) = self.runtime {
            let snapshot = runtime.snapshot();
            data.extend(snapshot.mutations);
        }
        let state = EffectiveStateBuilder::new().with_shared_frontmatter(data)
            .with_context(self.state.context().clone()).with_allow_ctx_override(true).build()
            .map_err(|error| CompositionError::SequenceShellResolution {
                command: site.authored.clone(), task: self.label(), message: error.to_string(), source: Some(Box::new(MarkdownError::CtxMerge(error))),
            })?;
        let resolution = super::super::super::document_expression_resolution_context(
            &self.task.origin_path, self.stack.prepared_context, self.stack.file_resolution_context, self.stack.ctx_base_dir,
        )?;
        let seq = self.sequence_binding();
        let (view, globals) = runtime_bindings_in_loop(LifecycleScope::TaskSetup, LifecycleValues {
            err: None, timing: self.stack.timing, group: self.stack.group,
            seq: Some(&seq),
        }, self.stack.loop_values);
        let session = EvaluationSession::associate(view, &state, globals)
            .expect("task bindings agree with their scope").with_resolution_context(resolution);
        let context = biscuit_terminal::errors::SourceContext::new(site.source_file.clone(), site.source_file.clone(), site.authored.clone());
        let origin = ShellCommandOrigin::Frontmatter { key: site.property.clone(), line: None };
        let chain = fill_template(approved.template(), &LookupEvaluator::new(&session), approved.late_values())
            .map_err(|error| self.template_error(site, error.into_shell_error(&context, &site.authored, &origin)))?;
        let policy_root = self.graph.template_authorization(approved.template()).flatten();
        let blacklist = crate::harness::shell::user_blacklist(policy_root.as_deref());
        if let Some(reason) = darkmatter::markdown::compose::shell_expansion::check_filled_chain(&chain, &blacklist) {
            return Err(self.template_error(site, ShellExpansionError::Blacklisted {
                ctx: Box::new(context), command: site.authored.clone(), reason, origin,
            }));
        }
        Ok(Some(chain))
    }

    pub(super) fn verify_template_authorizations(&self) -> Result<(), CompositionError> {
        for site in self.task.shell_sites.iter().filter(|site| site.mode == SiteMode::Template) {
            self.authorized_template(site, &LateValues::default())?;
        }
        Ok(())
    }

    pub(super) fn install_template_sites(&self, stacks: &mut ParsedStacks, late: &LateValues) -> Result<(), CompositionError> {
        for (stage, items) in [(TaskStage::Setup, &mut stacks.setup), (TaskStage::Teardown, &mut stacks.teardown)] {
            let Some(items) = items else { continue };
            let mut result: Result<(), CompositionError> = Ok(());
            for_each_action_mut(items, &mut |action| {
                if result.is_err() { return; }
                result = (|| match &mut action.kind {
                    LifecycleActionKind::Shell(shell) => {
                        let site = self.task.shell_sites.iter().find(|site| site.stage == stage && site.mode == SiteMode::Template
                            && matches!(site.kind, ShellSiteKind::Command)
                            && action_value_to_expr(&Value::String(site.authored.clone())).is_ok_and(|expr| expr == shell.command));
                        if let Some(site) = site {
                            shell.template = Some(self.authorized_template(site, late)?);
                            shell.pre_resolved = false;
                        }
                        Ok(())
                    }
                    LifecycleActionKind::RuntimeSet(set) => {
                        for (key, shell) in set.shell_values_mut() {
                            let authored = darkmatter::markdown::compose::authored_shell_template(
                                &shell.authored, key,
                                &biscuit_terminal::errors::SourceContext::new(self.task.origin_path.clone(), self.task.origin_path.clone(), shell.authored.clone()),
                            ).map(|(authored, _)| authored);
                            let site = self.task.shell_sites.iter().find(|site| site.stage == stage && site.mode == SiteMode::Template
                                && matches!(&site.kind, ShellSiteKind::SetValue { key: target } if target == key)
                                && authored.as_deref() == Some(site.authored.as_str()));
                            if let Some(site) = site {
                                let context = self.state.context();
                                shell.resolved = ResolvedShellValue::template_only(key, &shell.authored, "template-mode sequence value", context);
                                shell.template = Some(self.authorized_template(site, late)?);
                            }
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                })();
            });
            result?;
        }
        Ok(())
    }
}
