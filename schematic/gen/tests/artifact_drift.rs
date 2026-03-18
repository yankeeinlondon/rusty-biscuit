//! Artifact drift detection tests.
//!
//! These tests verify that committed OpenAPI and Postman artifacts match
//! what the generator would produce. Run with:
//!
//! ```sh
//! cargo test -p schematic-gen artifact_drift -- --ignored
//! ```

use std::fs;
use std::path::Path;

use schematic_define::openapi::ExportOptions;
use schematic_definitions::apis_by_module;
use schematic_definitions::registry::get_registry;
use schematic_gen::openapi_output::write_openapi;
use schematic_gen::postman_output::{write_postman, write_postman_grouped};

#[test]
#[ignore] // Run explicitly or in CI
fn generated_openapi_artifacts_are_up_to_date() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let grouped = apis_by_module();

    // Map API names to registry keys
    let api_names = [
        ("Anthropic", "anthropic"),
        ("Bitbucket", "bitbucket"),
        ("OpenAI", "openai"),
        ("ElevenLabs", "elevenlabs"),
        ("Gitea", "gitea"),
        ("GitHub", "github"),
        ("GitLab", "gitlab"),
        ("HuggingFaceHub", "huggingface"),
        ("LmStudio", "lmstudio"),
        ("OllamaNative", "ollama-native"),
        ("OllamaOpenAI", "ollama-openai"),
        ("EmqxBasic", "emqx-basic"),
        ("EmqxBearer", "emqx-bearer"),
        ("Eversolo", "eversolo"),
        ("SamsungSmartTv", "samsung-smart-tv"),
        ("UnfoldedCircleCoreRest", "unfolded-circle-core-rest"),
    ];

    for (module_name, apis) in grouped.iter() {
        for api in apis {
            let api_name_lower = api.name.to_lowercase();
            let api_name = api_names
                .iter()
                .find(|(name, _)| *name == api.name)
                .map(|(_, key)| *key)
                .unwrap_or(&api_name_lower);

            // Get the registry for this API
            if let Some(registry) = get_registry(api_name) {
                let version = api.version.as_deref().unwrap_or("0.1.0");
                let options = ExportOptions::new().with_version(version);

                let generated_path = write_openapi(api, &registry, &options, temp_dir.path())
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to generate OpenAPI for {} in module {}: {}",
                            api.name, module_name, e
                        )
                    });

                let generated = fs::read_to_string(&generated_path).unwrap();

                // Check against committed file
                let committed_path =
                    Path::new("openapi").join(generated_path.file_name().unwrap());
                if committed_path.exists() {
                    let committed = fs::read_to_string(&committed_path).unwrap_or_else(|e| {
                        panic!(
                            "Failed to read committed OpenAPI for {}: {}",
                            api.name, e
                        )
                    });

                    assert_eq!(
                        generated,
                        committed,
                        "\nOpenAPI artifact drift detected for '{}' in module '{}'.\n\
                         Run `just generate` to update the committed artifacts.\n\
                         Diff: {} vs {}",
                        api.name,
                        module_name,
                        generated_path.display(),
                        committed_path.display()
                    );
                } else {
                    eprintln!(
                        "Warning: No committed OpenAPI artifact found for {} at {}",
                        api.name,
                        committed_path.display()
                    );
                }
            }
        }
    }
}

#[test]
#[ignore]
fn generated_postman_artifacts_are_up_to_date() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let grouped = apis_by_module();

    for (module_name, apis) in grouped.iter() {
        let generated_path = if apis.len() == 1 {
            write_postman(&apis[0], temp_dir.path(), false).unwrap_or_else(|e| {
                panic!(
                    "Failed to generate Postman for {} in module {}: {}",
                    apis[0].name, module_name, e
                )
            })
        } else {
            let api_refs: Vec<&_> = apis.iter().collect();
            write_postman_grouped(&api_refs, module_name, temp_dir.path(), false)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to generate grouped Postman for module {}: {}",
                        module_name, e
                    )
                })
        };

        let generated = fs::read_to_string(&generated_path).unwrap();

        let committed_path = Path::new("postman").join(generated_path.file_name().unwrap());
        if committed_path.exists() {
            let committed = fs::read_to_string(&committed_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to read committed Postman for module {}: {}",
                    module_name, e
                )
            });

            assert_eq!(
                generated,
                committed,
                "\nPostman artifact drift detected for module '{}'.\n\
                 Run `just generate` to update the committed artifacts.\n\
                 Diff: {} vs {}",
                module_name,
                generated_path.display(),
                committed_path.display()
            );
        } else {
            eprintln!(
                "Warning: No committed Postman artifact found for module {} at {}",
                module_name,
                committed_path.display()
            );
        }
    }
}
