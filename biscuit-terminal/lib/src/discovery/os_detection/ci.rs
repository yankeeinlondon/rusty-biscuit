use std::env;

/// Check if the process is running in a CI environment.
///
/// Detects common CI/CD platforms by checking their environment variables.
///
/// ## Supported CI Platforms
///
/// - GitHub Actions (`GITHUB_ACTIONS`)
/// - GitLab CI (`GITLAB_CI`)
/// - Travis CI (`TRAVIS`)
/// - CircleCI (`CIRCLECI`)
/// - Jenkins (`JENKINS_URL`)
/// - Azure Pipelines (`TF_BUILD`)
/// - Buildkite (`BUILDKITE`)
/// - Drone CI (`DRONE`)
/// - AppVeyor (`APPVEYOR`)
/// - Bitbucket Pipelines (`BITBUCKET_COMMIT`)
/// - Sourcehut (`SRHT_BUILD_URL`)
/// - TeamCity (`TEAMCITY_VERSION`)
/// - AWS CodeBuild (`CODEBUILD_BUILD_ID`)
/// - Generic CI (`CI`)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::os_detection::is_ci;
///
/// if is_ci() {
///     println!("Running in CI - disabling interactive features");
/// }
/// ```
pub fn is_ci() -> bool {
    const CI_ENV_VARS: &[&str] = &[
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "TRAVIS",
        "CIRCLECI",
        "JENKINS_URL",
        "TF_BUILD",
        "BUILDKITE",
        "DRONE",
        "APPVEYOR",
        "BITBUCKET_COMMIT",
        "SRHT_BUILD_URL",
        "TEAMCITY_VERSION",
        "CODEBUILD_BUILD_ID",
    ];

    CI_ENV_VARS.iter().any(|var| env::var(var).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ci_returns_bool() {
        let _ = is_ci();
    }
}
