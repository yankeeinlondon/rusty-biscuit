//! Container lifecycle discovery shares the task command approval policy.

use super::*;
use crate::composition::lifecycle::{LifecycleConfig, LifecycleSignal};
use crate::composition::lifecycle::actions::LifecycleActionKind;
use crate::composition::lifecycle::shell_template::LifecycleShellTemplate;
use darkmatter::markdown::compose::expression::Expr;

impl Loader<'_> {
    pub(super) fn prepare_container_events(
        &mut self, lifecycle: &mut LifecycleConfig, origin: &Path, label: &str,
        state: &EffectiveState, placement: Placement,
    ) -> Result<(), CompositionError> {
        for signal in LifecycleSignal::ALL {
            let placement = Placement {
                position: if signal == LifecycleSignal::Start { placement.position } else { TaskPosition::Later },
                ..placement
            };
            let mut result = Ok(());
            crate::composition::lifecycle_actions::for_each_action_mut(lifecycle.stack_mut(signal), &mut |action| {
                if result.is_err() { return; }
                let property = action.path.to_string();
                let scope = SiteScope { origin, label, property, state, placement, guard: None };
                result = (|| {
                    match &mut action.kind {
                        LifecycleActionKind::Shell(shell) => {
                            if let Expr::StringLiteral(raw) = &shell.command {
                                let command_start = self.graph.shell_commands.len();
                                let prepared = self.prepare_site(raw, ShellSiteKind::Command, TaskStage::Primary, &scope.property, &scope)?;
                                shell.authored_command = Some(raw.clone());
                                shell.template = prepared.site.template.map(|template| LifecycleShellTemplate::new(
                                    template, prepared.bytes.as_ref().map(|_| self.graph.shell_commands[command_start..].iter().map(|command| command.command.clone()).collect()),
                                ));
                                if let Some(bytes) = prepared.bytes { shell.command = Expr::StringLiteral(bytes); }
                                shell.pre_resolved = true;
                            } else {
                                return Err(CompositionError::SequenceShellResolution {
                                    command: shell.command.to_string(), task: label.into(),
                                    message: "container shell commands must be authored strings".into(), source: None,
                                });
                            }
                        }
                        LifecycleActionKind::RuntimeSet(set) => {
                            for (key, shell) in set.shell_values_mut() {
                                let command_start = self.graph.shell_commands.len();
                                let prepared = self.prepare_site(&shell.authored, ShellSiteKind::SetValue { key: key.clone() },
                                    TaskStage::Primary, &format!("{}.{}", scope.property, key), &scope)?;
                                shell.template = prepared.site.template.map(|template| LifecycleShellTemplate::new(
                                    template, prepared.bytes.as_ref().map(|_| self.graph.shell_commands[command_start..].iter().map(|command| command.command.clone()).collect()),
                                ));
                                shell.resolved = prepared.set_value.or_else(|| darkmatter::markdown::compose::ResolvedShellValue::template_only(
                                    key, &shell.authored, "container template", state.context(),
                                ));
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                })();
            });
            result?;
        }
        self.graph.container_lifecycles.push(lifecycle.clone());
        Ok(())
    }
}
