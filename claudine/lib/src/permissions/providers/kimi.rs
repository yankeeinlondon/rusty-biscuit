use async_trait::async_trait;
use tokio::fs;

use crate::error::{ClaudineError, PolicyParseCause, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, PathProtectionRule,
    PolicyMode, PolicyWarning, TernaryState,
};
use crate::permissions::change::PolicyChange;
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::mutation::PolicyMutationPlan;
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};
use crate::provider::Provider;

#[derive(Debug, Clone, Default)]
struct KimiConfig {
    permission_mode: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct KimiCliOverrides {
    yolo: bool,
}

#[derive(Debug, Clone)]
struct KimiState {
    layers: Vec<(PolicySource, KimiConfig)>,
    cli: KimiCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct KimiPolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for KimiPolicyBackend {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::Partial,
            filesystem_queries: false,
            command_queries: false,
            network_queries: false,
            mcp_queries: false,
            agent_queries: false,
            persistent_mutations: false,
            one_shot_mutations: false,
        }
    }

    async fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();
        if let Some(home) = &ctx.home_dir {
            for (id, rel, precedence, kind) in [
                (
                    "kimi-config",
                    ".kimi/config.toml",
                    40,
                    PolicySourceKind::UserConfig,
                ),
                (
                    "kimi-mcp",
                    ".kimi/mcp.json",
                    41,
                    PolicySourceKind::UserConfig,
                ),
            ] {
                let path = home.join(rel);
                if fs::try_exists(&path).await.unwrap_or(false) {
                    sources.push(PolicySource {
                        id: id.to_owned(),
                        kind,
                        path: Some(path),
                        precedence,
                        writable: true,
                    });
                }
            }
        }
        sources.sort_by_key(|source| source.precedence);
        Ok(sources)
    }

    async fn load_native_layers(
        &self,
        _ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>> {
        let mut layers = Vec::new();
        for source in sources {
            let path = source.path.as_ref().ok_or_else(|| {
                ClaudineError::PolicySourceDiscovery(format!(
                    "Kimi source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path).await?;
            let config = if path.extension().and_then(|value| value.to_str()) == Some("toml") {
                let value: toml::Value =
                    toml::from_str(&content).map_err(|error| ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                        source: Some(PolicyParseCause::TomlDe(Box::new(error))),
                    })?;
                KimiConfig {
                    permission_mode: value
                        .get("permission_mode")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| {
                            value
                                .get("permissions")
                                .and_then(toml::Value::as_table)
                                .and_then(|table| table.get("mode"))
                                .and_then(toml::Value::as_str)
                                .map(str::to_owned)
                        }),
                }
            } else {
                KimiConfig::default()
            };
            layers.push(NativePolicyLayer::new(source.clone(), config));
        }
        Ok(layers)
    }

    fn parse_cli_overrides(
        &self,
        _ctx: &PolicyContext,
        input: CliPolicyInput<'_>,
    ) -> Result<ProviderCliOverrides> {
        match input {
            CliPolicyInput::None => Ok(ProviderCliOverrides::new(
                Provider::KimiCode,
                KimiCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::KimiCode {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::KimiCode,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                        source: None,
                    });
                }
                let typed = parsed
                    .payload::<KimiCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::KimiCode,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                        source: None,
                    })?;
                Ok(ProviderCliOverrides::new(Provider::KimiCode, typed))
            }
            CliPolicyInput::Argv(argv) => Ok(ProviderCliOverrides::new(
                Provider::KimiCode,
                KimiCliOverrides {
                    yolo: argv.iter().any(|arg| arg == "--yolo" || arg == "--print"),
                },
            )),
        }
    }

    fn compose_native_policy(
        &self,
        _ctx: &PolicyContext,
        layers: &[NativePolicyLayer],
        cli: Option<&ProviderCliOverrides>,
    ) -> Result<NativeEffectivePolicy> {
        let typed_layers = layers
            .iter()
            .map(|layer| {
                let config = layer.payload::<KimiConfig>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Kimi layer payload type mismatch".to_owned(),
                        source: None,
                    }
                })?;
                Ok((layer.source.clone(), config))
            })
            .collect::<Result<Vec<_>>>()?;
        let cli = cli
            .and_then(|value| value.payload::<KimiCliOverrides>().cloned())
            .unwrap_or_default();
        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if cli.yolo {
            sources.push(PolicySource {
                id: "kimi-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }
        Ok(NativeEffectivePolicy::new(
            Provider::KimiCode,
            sources,
            KimiState {
                layers: typed_layers,
                cli,
            },
        ))
    }

    async fn canonicalize(
        &self,
        _ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy> {
        let state =
            native
                .payload::<KimiState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "kimi-effective".to_owned(),
                    message: "Kimi effective policy payload type mismatch".to_owned(),
                    source: None,
                })?;
        let effective_mode = state
            .layers
            .iter()
            .find_map(|(_, config)| config.permission_mode.clone())
            .unwrap_or_else(|| "prompted".to_owned());
        let mut policy = CanonicalPolicy::empty(
            Provider::KimiCode,
            if state.cli.yolo {
                PolicyMode::Effective
            } else {
                PolicyMode::Configured
            },
        );
        policy.warnings.push(PolicyWarning {
            code: "kimi_partial_support".to_owned(),
            message:
                "Kimi policy support is partial; only high-confidence approval-mode signals and protected config paths are modeled."
                    .to_owned(),
            source_id: None,
        });
        for source in &native.sources {
            if let Some(path) = &source.path {
                policy
                    .axes
                    .filesystem
                    .protected_config_paths
                    .push(PathProtectionRule {
                        pattern: path.to_string_lossy().into_owned(),
                        description: "Kimi configuration file".to_owned(),
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "kimi-config",
                            "Kimi config files are exposed as protected paths until deeper policy modeling exists.",
                        ),
                    });
            }
        }
        policy.axes.runtime.approval_mode = Some(if state.cli.yolo || effective_mode == "yolo" {
            CanonicalApprovalMode::AutoApprove
        } else {
            CanonicalApprovalMode::AlwaysAsk
        });
        policy.axes.runtime.can_bypass_permissions = if state.cli.yolo || effective_mode == "yolo" {
            TernaryState::Yes
        } else {
            TernaryState::No
        };
        Ok(policy)
    }

    async fn plan_change(
        &self,
        _ctx: &PolicyContext,
        _current: &NativeEffectivePolicy,
        _change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        Ok(PolicyMutationPlan::unsupported(
            Provider::KimiCode,
            "Kimi mutation planning is deferred until the native config model is researched further.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kimi_backend_returns_partial_snapshot_with_runtime_mode() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        tokio::fs::create_dir_all(home.join(".kimi")).await.unwrap();
        tokio::fs::write(
            home.join(".kimi/config.toml"),
            "permission_mode = \"prompted\"\n",
        )
        .await
        .unwrap();

        let ctx = PolicyContext::new(dir.path().join("repo")).with_home_dir(home);
        let backend = KimiPolicyBackend;
        let sources = backend.discover_sources(&ctx).await.unwrap();
        let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
        let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
        let canonical = backend.canonicalize(&ctx, &native).await.unwrap();

        assert_eq!(
            canonical.axes.runtime.approval_mode,
            Some(CanonicalApprovalMode::AlwaysAsk)
        );
        assert!(!canonical.axes.filesystem.protected_config_paths.is_empty());
        assert!(!canonical.warnings.is_empty());
    }
}
