# Binary distribution checks (`.github/workflows/distribute-binaries.yml`).
#
# `distribution-test` is also what CI runs: it is `repo-deps`'s `distribution`
# companion suite in `scripts/ci/affected_scope.py`'s `SUITE_REGISTRY`, so the
# command a developer runs is the command CI runs.

# Run the distribution Python suites in a temporary environment holding exactly the pinned requirements
distribution-test *args:
    bash scripts/distribution/pinned-python.sh bash -c '"${PYTHON}" scripts/distribution/run_suites.py "$@"' run_suites "$@"

# The Python suites, then the workflow contracts (`distribution_*` in test-toolkit's `ci_workflow_contracts`)
distribution-check:
    just distribution-test
    cargo nextest run --locked -p test-toolkit --test ci_workflow_contracts -E 'test(/^distribution_/)'

# Nonpublishing acceptance with the real pinned tools: `just distribution-verify OUT --artifacts DIR` (see scripts/distribution/verify.sh)
distribution-verify out *args:
    bash scripts/distribution/pinned-python.sh bash scripts/distribution/verify.sh "$@"
