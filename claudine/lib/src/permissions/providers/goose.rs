use async_trait::async_trait;
use biscuit_file::serde_yaml_ng;
use tokio::fs;

use crate::error::{ClaudineError, PolicyParseCause, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalPolicy, CanonicalRuleProvenance, PathProtectionRule, PolicyMode, PolicyWarning,
};
use crate::permissions::change::PolicyChange;
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::mutation::PolicyMutationPlan;
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};
use crate::provider::Provider;

#[derive(Debug, Clone)]
enum GooseLayerData {
    Config(serde_yaml_ng::Value),
    Permission(serde_yaml_ng::Value),
}

#[derive(Debug, Clone, Default)]
struct GooseCliOverrides;

#[derive(Debug, Clone)]
struct GooseState {
    layers: Vec<(PolicySource, GooseLayerData)>,
}

#[derive(Debug, Default)]
pub(crate) struct GoosePolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for GoosePolicyBackend {
    fn provider(&self) -> Provider {
        Provider::Goose
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
            for (id, rel, precedence) in [
                ("goose-config", ".config/goose/config.yaml", 40),
                ("goose-permissions", ".config/goose/permission.yaml", 41),
            ] {
                let path = home.join(rel);
                if fs::try_exists(&path).await.unwrap_or(false) {
                    sources.push(PolicySource {
                        id: id.to_owned(),
                        kind: PolicySourceKind::UserConfig,
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
                    "Goose source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path).await?;
            let parsed: serde_yaml_ng::Value =
                serde_yaml_ng::from_str(&content).map_err(|error| {
                    ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                        source: Some(PolicyParseCause::Yaml(error)),
                    }
                })?;
            let payload = if source.id.contains("permission") {
                GooseLayerData::Permission(parsed)
            } else {
                GooseLayerData::Config(parsed)
            };
            layers.push(NativePolicyLayer::new(source.clone(), payload));
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
                Provider::Goose,
                GooseCliOverrides,
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::Goose {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::Goose,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                        source: None,
                    });
                }
                Ok(ProviderCliOverrides::new(
                    Provider::Goose,
                    GooseCliOverrides,
                ))
            }
            CliPolicyInput::Argv(_) => Ok(ProviderCliOverrides::new(
                Provider::Goose,
                GooseCliOverrides,
            )),
        }
    }

    fn compose_native_policy(
        &self,
        _ctx: &PolicyContext,
        layers: &[NativePolicyLayer],
        _cli: Option<&ProviderCliOverrides>,
    ) -> Result<NativeEffectivePolicy> {
        let typed_layers = layers
            .iter()
            .map(|layer| {
                let payload = layer.payload::<GooseLayerData>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Goose layer payload type mismatch".to_owned(),
                        source: None,
                    }
                })?;
                Ok((layer.source.clone(), payload))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(NativeEffectivePolicy::new(
            Provider::Goose,
            typed_layers
                .iter()
                .map(|(source, _)| source.clone())
                .collect(),
            GooseState {
                layers: typed_layers,
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
                .payload::<GooseState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "goose-effective".to_owned(),
                    message: "Goose effective policy payload type mismatch".to_owned(),
                    source: None,
                })?;
        let mut policy = CanonicalPolicy::empty(Provider::Goose, PolicyMode::Configured);
        policy.warnings.push(PolicyWarning {
            code: "goose_partial_support".to_owned(),
            message:
                "Goose policy support is partial; only obvious config discovery and protected-path reporting are modeled."
                    .to_owned(),
            source_id: None,
        });
        for (source, layer) in &state.layers {
            let kind = match layer {
                GooseLayerData::Config(value) => {
                    let _ = value;
                    "config.yaml"
                }
                GooseLayerData::Permission(value) => {
                    let _ = value;
                    "permission.yaml"
                }
            };
            if let Some(path) = &source.path {
                policy
                    .axes
                    .filesystem
                    .protected_config_paths
                    .push(PathProtectionRule {
                        pattern: path.to_string_lossy().into_owned(),
                        description: format!("Goose {kind} file"),
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            kind,
                            "Goose config files are exposed as protected paths until deeper policy modeling exists.",
                        ),
                    });
            }
        }
        Ok(policy)
    }

    async fn plan_change(
        &self,
        _ctx: &PolicyContext,
        _current: &NativeEffectivePolicy,
        _change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        Ok(PolicyMutationPlan::unsupported(
            Provider::Goose,
            "Goose mutation planning is deferred until the native permission model is researched further.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn goose_backend_returns_partial_snapshot_instead_of_missing_backend() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        tokio::fs::create_dir_all(home.join(".config/goose"))
            .await
            .unwrap();
        tokio::fs::write(home.join(".config/goose/config.yaml"), "provider: openai\n")
            .await
            .unwrap();

        let ctx = PolicyContext::new(dir.path().join("repo")).with_home_dir(home);
        let backend = GoosePolicyBackend;
        let sources = backend.discover_sources(&ctx).await.unwrap();
        let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
        let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
        let canonical = backend.canonicalize(&ctx, &native).await.unwrap();

        assert_eq!(canonical.provider, Provider::Goose);
        assert!(!canonical.warnings.is_empty());
        assert!(!canonical.axes.filesystem.protected_config_paths.is_empty());
    }
}
