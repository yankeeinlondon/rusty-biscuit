"""In-process cases for `scripts/ci/release_path.py` and
`scripts/ci/integration_release.py`, launched by the `release_path_*` tests in
`ci_workflow_contracts.rs`, one group of cases per test.

Usage: driver.py <repository root> (--list | <case>...)

`--list` prints every case name; the Rust suite checks that its groups
together name exactly this list.

Every case drives a module's public function (or its command entry point)
against a hermetic Git repository or a real-shaped event, with Git, the forge,
and release-plz at the boundary: Git is real and isolated from host
configuration (remotes are local bare repositories), the forge is an in-memory
fake or the real `GitHubForge` against a local HTTP server on 127.0.0.1, and
release-plz is a recording fake. Nothing reaches the network. Prints one line
per named case and exits 1 on any failure or unknown name.
"""

from __future__ import annotations

import atexit
import copy
import io
import json
import os
import shutil
import subprocess
import sys
import socket
import socketserver
import tempfile
import threading
import tomllib
import traceback
from contextlib import contextmanager, redirect_stderr, redirect_stdout
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import integration_release as ir  # noqa: E402
import release_path as rp  # noqa: E402

SCRATCH = Path(tempfile.mkdtemp(prefix="release-path-driver-"))
atexit.register(shutil.rmtree, SCRATCH, True)
(SCRATCH / "empty-gitconfig").write_text("", encoding="utf-8")
os.environ.update(
    GIT_CONFIG_GLOBAL=str(SCRATCH / "empty-gitconfig"),
    GIT_CONFIG_NOSYSTEM="1",
    GIT_TERMINAL_PROMPT="0",
    GIT_AUTHOR_NAME="fixture",
    GIT_AUTHOR_EMAIL="fixture@invalid",
    GIT_COMMITTER_NAME="fixture",
    GIT_COMMITTER_EMAIL="fixture@invalid",
)
# A CI host's step summary and API URL must not receive or shape what an
# in-process command writes or requests.
for leaked in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_CONFIG_COUNT", "GITHUB_STEP_SUMMARY",
               "GITHUB_API_URL"):
    os.environ.pop(leaked, None)

CANONICAL = rp.CANONICAL_REPOSITORY
BOT = "rusty-biscuit-release[bot]"
SHIPPED_CONFIG = (ROOT / "release-plz.toml").read_text(encoding="utf-8")
# The shipped `ci.yml` title, read rather than restated, so a rename that the
# selector does not follow fails the positive preparation case.
CI_TITLE = next(line.removeprefix("name:").strip()
                for line in (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8").splitlines()
                if line.startswith("name:"))
CASES: list = []


def case(function):
    CASES.append(function)
    return function


def expect_error(path_fragment: str, function, *args, **kwargs) -> None:
    try:
        function(*args, **kwargs)
    except rp.ReleasePathError as error:
        assert path_fragment in error.path, f"error located at {error.path!r}, expected {path_fragment!r}: {error}"
        return
    raise AssertionError(f"expected an error at {path_fragment!r}")


# --- events -----------------------------------------------------------------

def workflow_run_event(**overrides) -> dict:
    event = {
        "action": "completed",
        "workflow_run": {
            "name": CI_TITLE,
            "status": "completed",
            "conclusion": "success",
            "event": "push",
            "head_branch": "main",
            "head_sha": "a" * 40,
            "repository": {"full_name": CANONICAL},
            "head_repository": {"full_name": CANONICAL},
            "id": 1,
        },
        "repository": {"full_name": CANONICAL},
    }
    event["workflow_run"].update(overrides)
    return event


def pull_request_event() -> dict:
    return {
        "action": "closed",
        "number": 7,
        "pull_request": {
            "number": 7,
            "merged": True,
            "merge_commit_sha": "b" * 40,
            "labels": [{"name": "release"}, {"name": "automated"}],
            "user": {"login": BOT, "type": "Bot"},
            "base": {"ref": "main", "repo": {"full_name": CANONICAL}},
            "head": {"ref": "release-plz-2026-11-08", "sha": "c" * 40, "repo": {"full_name": CANONICAL}},
        },
    }


def edited(event: dict, path: str, value) -> dict:
    copy_ = copy.deepcopy(event)
    node = copy_
    *parents, last = path.split(".")
    for part in parents:
        node = node[part]
    if value is rp._ABSENT:
        del node[last]
    else:
        node[last] = value
    return copy_


@case
def preparation_selects_only_a_successful_ci_push_run_on_main():
    selection = rp.select_preparation("workflow_run", CANONICAL, workflow_run_event())
    assert selection.selected and selection.outputs == {"validated_sha": "a" * 40}, selection
    for overrides, reason in [
        ({"event": "schedule"}, "schedule"),
        ({"event": "workflow_dispatch"}, "workflow_dispatch"),
        ({"event": "pull_request"}, "pull_request"),
        ({"conclusion": "failure"}, "failure"),
        ({"status": "in_progress"}, "not completed"),
        ({"head_branch": "feature"}, "feature"),
        ({"name": "nightly"}, "nightly"),
        ({"name": "Release automation"}, "Release automation"),
        # The pre-rename title: `release-plz.yml` on `main` no longer triggers
        # on it, so a run carrying it cannot be a current CI run.
        ({"name": "ci"}, "'ci'"),
    ]:
        selection = rp.select_preparation("workflow_run", CANONICAL, workflow_run_event(**overrides))
        assert not selection.selected and reason in selection.reason, (overrides, selection)
    assert not rp.select_preparation("push", CANONICAL, workflow_run_event()).selected
    assert not rp.select_preparation("workflow_run", "someone/fork", workflow_run_event()).selected


@case
def preparation_refuses_a_foreign_head_repository_and_malformed_fields():
    event = workflow_run_event(head_repository={"full_name": "someone/fork"})
    expect_error("workflow_run.head_repository.full_name", rp.select_preparation, "workflow_run", CANONICAL, event)
    expect_error("workflow_run.head_repository", rp.select_preparation, "workflow_run", CANONICAL,
                 workflow_run_event(head_repository=None))
    for value, path in [(None, "workflow_run.event"), (rp._ABSENT, "workflow_run.event"), (7, "workflow_run.event"),
                        ("", "workflow_run.event")]:
        event = edited(workflow_run_event(), "workflow_run.event", value)
        expect_error(path, rp.select_preparation, "workflow_run", CANONICAL, event)
    expect_error("workflow_run.head_sha", rp.select_preparation, "workflow_run", CANONICAL,
                 workflow_run_event(head_sha="abc123"))


@case
def publication_selects_the_release_apps_merged_proposal():
    selection = rp.select_publication("pull_request", CANONICAL, pull_request_event(), BOT)
    assert selection.selected, selection
    assert selection.outputs == {"merge_sha": "b" * 40, "head_sha": "c" * 40, "number": "7"}, selection


@case
def unrelated_pull_requests_are_no_ops():
    base = pull_request_event()
    for path, value in [
        ("pull_request.merged", False),
        ("action", "opened"),
        ("pull_request.labels", [{"name": "automated"}]),
        ("pull_request.labels", []),
        ("pull_request.base.ref", "develop"),
        ("pull_request.head.ref", "feature/release-notes"),
    ]:
        selection = rp.select_publication("pull_request", CANONICAL, edited(base, path, value), BOT)
        assert not selection.selected, (path, value, selection)
    assert not rp.select_publication("push", CANONICAL, base, BOT).selected


@case
def a_plausible_proposal_failing_identity_is_refused():
    base = pull_request_event()
    for path, value, located in [
        ("pull_request.user.login", "someone", "pull_request.user.login"),
        ("pull_request.user.type", "User", "pull_request.user.type"),
        ("pull_request.head.repo", None, "pull_request.head.repo"),
        ("pull_request.head.repo", {"full_name": "someone/fork"}, "pull_request.head.repo.full_name"),
        ("pull_request.base.repo", {"full_name": "someone/fork"}, "pull_request.base.repo.full_name"),
        ("pull_request.merge_commit_sha", None, "pull_request.merge_commit_sha"),
        ("pull_request.labels", None, "pull_request.labels"),
        ("pull_request.labels", rp._ABSENT, "pull_request.labels"),
        ("pull_request.labels", "release", "pull_request.labels"),
        ("pull_request.labels", [{"name": "release"}, 123], "pull_request.labels[1]"),
        ("pull_request.labels", [123], "pull_request.labels[0]"),
        ("pull_request.merged", "true", "pull_request.merged"),
    ]:
        expect_error(located, rp.select_publication, "pull_request", CANONICAL, edited(base, path, value), BOT)
    expect_error("RELEASE_APP_BOT_LOGIN", rp.select_publication, "pull_request", CANONICAL, base, None)


@case
def app_configuration_is_required_without_a_default_token_fallback():
    complete = {"RELEASE_APP_ID": "123", "HAS_RELEASE_APP_PRIVATE_KEY": "true", "RELEASE_APP_BOT_LOGIN": BOT}
    rp.require_app_configuration(complete, need_bot_login=True)
    for key, value, named in [
        ("RELEASE_APP_ID", "", "RELEASE_APP_ID"),
        ("HAS_RELEASE_APP_PRIVATE_KEY", "false", "RELEASE_APP_PRIVATE_KEY"),
        ("RELEASE_APP_BOT_LOGIN", "", "RELEASE_APP_BOT_LOGIN"),
        ("RELEASE_APP_BOT_LOGIN", "rusty-biscuit-release", "RELEASE_APP_BOT_LOGIN"),
    ]:
        try:
            rp.require_app_configuration({**complete, key: value}, need_bot_login=True)
        except rp.ReleasePathError as error:
            assert named in str(error), error
        else:
            raise AssertionError(f"{key}={value!r} must be refused")


@case
def the_strict_json_loader_refuses_ambiguous_documents():
    assert rp.load_json('{"a": [1, {"b": null}]}', source="x") == {"a": [1, {"b": None}]}
    for text in ['{"a": 1, "a": 2}', '{"a": NaN}', '{"a": Infinity}', '{"a": 1} trailing', '{"a": 1', ""]:
        expect_error("event.json", rp.load_json, text, source="event.json")
    path = SCRATCH / "list-event.json"
    path.write_text("[]", encoding="utf-8")
    expect_error(str(path), rp.load_event, path)


@case
def the_shipped_configuration_is_the_two_package_allowlist():
    config = rp.load_release_config(SHIPPED_CONFIG)
    assert config.enabled == rp.APPROVED_RELEASE_SET
    widened = SHIPPED_CONFIG.replace('name = "darkmatter"\n', 'name = "darkmatter"\nrelease = true\n')
    expect_error("package", rp.load_release_config, widened)
    expect_error("workspace.git_only", rp.load_release_config, SHIPPED_CONFIG.replace("git_only = true", "git_only = false"))
    expect_error("workspace.git_tag_name", rp.load_release_config,
                 SHIPPED_CONFIG.replace('git_tag_name = "{{ package }}-v{{ version }}"', 'git_tag_name = "v{{ version }}"'))
    expect_error("release-plz.toml", rp.load_release_config, SHIPPED_CONFIG + "\n[workspace]\n")
    expect_error("release-plz.toml", rp.load_release_config, SHIPPED_CONFIG + "\nnot toml at all\n")
    expect_error("package[", rp.load_release_config,
                 SHIPPED_CONFIG.replace('name = "tabby"\nrelease = false', 'name = "tabby"\nrelease = "no"'))


@case
def per_package_configuration_keeps_settings_and_disables_tags_only_on_recovery():
    config = rp.load_release_config(SHIPPED_CONFIG)
    for recovery in (False, True):
        generated = tomllib.loads(rp.package_release_config(config, "git-wt-cli", recovery=recovery))
        workspace = generated["workspace"]
        assert workspace["release"] is False and workspace["git_only"] is True and workspace["publish"] is False
        assert workspace["git_tag_name"] == "{{ package }}-v{{ version }}" and workspace["release_always"] is False
        assert workspace["git_release_enable"] is True and workspace["git_release_draft"] is False
        assert workspace["git_tag_enable"] is (not recovery), workspace
        assert generated["package"] == [
            {"name": "git-wt-cli", "release": True, "changelog_path": "worktree/cli/CHANGELOG.md"}
        ], generated["package"]
    expect_error("package darkmatter", rp.package_release_config, config, "darkmatter", recovery=False)


# --- a real merge graph -----------------------------------------------------

def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, encoding="utf-8",
                               check=False)
    assert completed.returncode == 0, f"git {args}: {completed.stderr}"
    return completed.stdout.strip()


def write(repo: Path, path: str, text: str) -> None:
    (repo / path).parent.mkdir(parents=True, exist_ok=True)
    (repo / path).write_text(text, encoding="utf-8")


def manifest(name: str, version: str) -> str:
    return f'[package]\nname = "{name}"\nversion = {version}\n'


_TEMPLATES: dict[bool, dict] = {}


def release_repository(name: str, *, extra_bump: bool = False) -> dict:
    """main: A → B, release-plz-x: A → H (bumps both CLIs), M = merge of H
    into B with a merge commit, then main advances to C, which bumps a CLI
    again; checked out detached at M. Returns the commits. Each graph is built
    once per process and copied for every later caller."""
    if extra_bump not in _TEMPLATES:
        _TEMPLATES[extra_bump] = _build_release_repository(f"template-{extra_bump}", extra_bump=extra_bump)
    template = _TEMPLATES[extra_bump]
    repo = SCRATCH / name
    shutil.copytree(template["repo"], repo, symlinks=True)
    return {**template, "repo": repo}


def _build_release_repository(name: str, *, extra_bump: bool) -> dict:
    repo = SCRATCH / name
    repo.mkdir()
    git(repo, "init", "--quiet", "--initial-branch=main")
    write(repo, "Cargo.toml", '[workspace]\nmembers = ["sniff/cli", "worktree/cli", "other"]\n'
                              '[workspace.package]\nversion = "0.1.0"\n')
    write(repo, "sniff/cli/Cargo.toml", manifest("sniff-detection-cli", '"0.1.0"'))
    write(repo, "worktree/cli/Cargo.toml", manifest("git-wt-cli", "{ workspace = true }"))
    write(repo, "other/Cargo.toml", manifest("other", '"0.1.0"'))
    # An unchanged fixture that is not valid TOML is never read.
    write(repo, "fixtures/broken/Cargo.toml", "this is [not toml\n")
    write(repo, "release-plz.toml", SHIPPED_CONFIG)
    git(repo, "add", "-A")
    git(repo, "commit", "--quiet", "-m", "A")
    a = git(repo, "rev-parse", "HEAD")
    git(repo, "switch", "--quiet", "-c", "release-plz-x")
    write(repo, "sniff/cli/Cargo.toml", manifest("sniff-detection-cli", '"0.2.0"'))
    write(repo, "worktree/cli/Cargo.toml", manifest("git-wt-cli", '"0.1.1"'))
    write(repo, "sniff/cli/CHANGELOG.md", "## [0.2.0]\n")
    if extra_bump:
        write(repo, "other/Cargo.toml", manifest("other", '"0.2.0"'))
    git(repo, "commit", "--quiet", "-am", "chore: release")
    git(repo, "add", "-A")
    git(repo, "commit", "--quiet", "--allow-empty", "-m", "changelog")
    h = git(repo, "rev-parse", "HEAD")
    git(repo, "switch", "--quiet", "main")
    write(repo, "README.md", "unrelated\n")
    git(repo, "add", "-A")
    git(repo, "commit", "--quiet", "-m", "B")
    b = git(repo, "rev-parse", "HEAD")
    git(repo, "merge", "--quiet", "--no-ff", "-m", "M", "release-plz-x")
    m = git(repo, "rev-parse", "HEAD")
    write(repo, "sniff/cli/Cargo.toml", manifest("sniff-detection-cli", '"9.9.9"'))
    git(repo, "commit", "--quiet", "-am", "C")
    c = git(repo, "rev-parse", "HEAD")
    git(repo, "checkout", "--quiet", "--detach", m)
    return {"repo": repo, "A": a, "B": b, "H": h, "M": m, "C": c}


@case
def topology_accepts_only_a_two_parent_merge_of_the_final_commit():
    graph = release_repository("topology")
    runner = rp.Runner()
    repo = graph["repo"]
    assert rp.verify_publication_topology(runner, repo, graph["M"], graph["H"]) == graph["A"]
    # A different final commit (the proposal moved after review).
    expect_error("pull_request.merge_commit_sha", rp.verify_publication_topology, runner, repo, graph["M"], graph["B"])
    # The checkout is not the event's merge commit.
    expect_error("checkout", rp.verify_publication_topology, runner, repo, graph["C"], graph["H"])
    # Squash or rebase: a single-parent commit.
    git(repo, "checkout", "--quiet", "--detach", graph["C"])
    expect_error("pull_request.merge_commit_sha", rp.verify_publication_topology, runner, repo, graph["C"], graph["H"])


@case
def candidates_come_from_the_proposal_not_a_later_main():
    graph = release_repository("candidates")
    candidates = rp.publication_candidates(rp.Runner(), graph["repo"], graph["A"], graph["H"])
    assert [(c.package, c.tag, c.manifest) for c in candidates] == [
        ("git-wt-cli", "git-wt-cli-v0.1.1", "worktree/cli/Cargo.toml"),
        ("sniff-detection-cli", "sniff-detection-cli-v0.2.0", "sniff/cli/Cargo.toml"),
    ], candidates
    extra = release_repository("candidates-extra", extra_bump=True)
    expect_error("proposal", rp.publication_candidates, rp.Runner(), extra["repo"], extra["A"], extra["H"])


# --- remote state, publication, recovery ------------------------------------

class FakeForge(rp.Forge):
    def __init__(self):
        self.tags: dict[str, str] = {}
        self.releases: dict[str, rp.ReleaseRecord] = {}
        self.failing: set[str] = set()

    def remote_tag_target(self, tag):
        if tag in self.failing:
            raise rp.ReleasePathError(f"ls-remote {tag}", "HTTP 503")
        return self.tags.get(tag)

    def release(self, tag):
        return self.releases.get(tag, rp.ReleaseRecord(exists=False))

    def user_id(self, login):
        return 4242


class FakeReleasePlz(rp.Runner):
    """Real Git; a recording release-plz that creates what its configuration
    asks for on the fake forge."""

    def __init__(self, forge: FakeForge, head: str, exit_for: dict[str, int] | None = None):
        self.forge, self.head, self.exit_for, self.calls = forge, head, exit_for or {}, []

    def run(self, argv, *, cwd, env=None):
        if argv[0] != "release-plz":
            return super().run(argv, cwd=cwd, env=env)
        config = tomllib.loads(Path(argv[argv.index("--config") + 1]).read_text(encoding="utf-8"))
        package = config["package"][0]["name"]
        version = tomllib.loads((cwd / f"{'sniff' if package.startswith('sniff') else 'worktree'}/cli/Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
        tag = f"{package}-v{version}"
        self.calls.append({
            "argv": list(argv), "package": package, "config": config,
            "head": git(cwd, "rev-parse", "HEAD"), "local_tags": git(cwd, "tag", "--list").split(),
            "origin": git(cwd, "remote", "get-url", "origin"), "env": dict(env or {}),
        })
        code = self.exit_for.get(package, 0)
        if code == 0:
            if config["workspace"].get("git_tag_enable", True):
                self.forge.tags[tag] = self.head
            self.forge.releases[tag] = rp.ReleaseRecord(exists=True, url=f"https://example.invalid/{tag}")
        return subprocess.CompletedProcess(argv, code, "", "")


def run_publication(name: str, prepare=None, exit_for=None, local_tag: str | None = None):
    graph = release_repository(name)
    if local_tag:
        git(graph["repo"], "tag", local_tag, graph["H"])
    forge = FakeForge()
    if prepare:
        prepare(forge, graph["H"])
    runner = FakeReleasePlz(forge, graph["H"], exit_for)
    candidates = rp.publication_candidates(runner, graph["repo"], graph["A"], graph["H"])
    env = rp.automation_env({"PATH": os.environ["PATH"], "GITHUB_TOKEN": "default"}, token="app-token", bot_login=BOT, bot_id=4242)
    with redirect_stdout(io.StringIO()):
        result = rp.publish(runner=runner, forge=forge, repo=graph["repo"], workdir=SCRATCH / f"{name}-work",
                            config=rp.load_release_config(SHIPPED_CONFIG), candidates=candidates,
                            head_sha=graph["H"], env=env)
    return result, runner, forge, graph


@case
def new_candidates_are_published_from_the_final_commit_one_package_each():
    result, runner, forge, graph = run_publication("publish-new")
    assert result.exit_status == 0 and not result.failures, result
    assert [call["package"] for call in runner.calls] == ["git-wt-cli", "sniff-detection-cli"]
    for call in runner.calls:
        assert call["head"] == graph["H"], call
        assert call["origin"] == f"https://github.com/{CANONICAL}.git"
        assert [p["name"] for p in call["config"]["package"]] == [call["package"]]
        assert call["config"]["workspace"].get("git_tag_enable", True) is True
        assert "--manifest-path" in call["argv"] and "--forge" in call["argv"]
        assert "GITHUB_TOKEN" not in call["env"] and call["env"]["GIT_TOKEN"] == "app-token"
        assert call["env"]["GIT_COMMITTER_EMAIL"] == f"4242+{BOT}@users.noreply.github.com"
    assert set(forge.tags.values()) == {graph["H"]} and len(forge.tags) == 2
    assert all(c.state == rp.COMPLETE for c in result.candidates)


@case
def a_complete_rerun_writes_nothing():
    def complete(forge, head):
        for tag in ("git-wt-cli-v0.1.1", "sniff-detection-cli-v0.2.0"):
            forge.tags[tag] = head
            forge.releases[tag] = rp.ReleaseRecord(exists=True, url="u")
    result, runner, _, _ = run_publication("publish-rerun", complete)
    assert result.exit_status == 0 and runner.calls == [], (result, runner.calls)


@case
def a_tag_without_its_release_is_recovered_without_moving_the_tag():
    def tag_only(forge, head):
        forge.tags["git-wt-cli-v0.1.1"] = head
    result, runner, forge, graph = run_publication("publish-recover", tag_only, local_tag="git-wt-cli-v0.1.1")
    assert result.exit_status == 0, result
    recovery = runner.calls[0]
    assert recovery["package"] == "git-wt-cli" and recovery["config"]["workspace"]["git_tag_enable"] is False
    assert "git-wt-cli-v0.1.1" not in recovery["local_tags"], recovery["local_tags"]
    assert git(graph["repo"], "rev-parse", "git-wt-cli-v0.1.1") == graph["H"], "the session's tag is untouched"
    assert forge.tags["git-wt-cli-v0.1.1"] == graph["H"]
    assert runner.calls[1]["config"]["workspace"].get("git_tag_enable", True) is True


@case
def refused_remote_states_stop_before_any_release_command():
    def wrong_target(forge, head):
        forge.tags["sniff-detection-cli-v0.2.0"] = "f" * 40

    def release_only(forge, head):
        forge.releases["git-wt-cli-v0.1.1"] = rp.ReleaseRecord(exists=True, url="u")

    def draft(forge, head):
        forge.tags["git-wt-cli-v0.1.1"] = head
        forge.releases["git-wt-cli-v0.1.1"] = rp.ReleaseRecord(exists=True, draft=True, url="u")

    def api_failure(forge, head):
        forge.failing.add("sniff-detection-cli-v0.2.0")

    for name, prepare, reason in [
        ("refuse-wrong-target", wrong_target, "never moved"),
        ("refuse-release-only", release_only, "its tag does not"),
        ("refuse-draft", draft, "draft"),
        ("refuse-api", api_failure, "HTTP 503"),
    ]:
        result, runner, _, _ = run_publication(name, prepare)
        assert result.exit_status == 1 and runner.calls == [], (name, result, runner.calls)
        assert any(reason in failure for failure in result.failures), (name, result.failures)


@case
def a_failure_after_the_first_package_keeps_its_status_and_inventory():
    result, runner, forge, graph = run_publication("publish-partial", exit_for={"sniff-detection-cli": 3})
    assert result.exit_status == 3, result
    states = {c.package: c.state for c in result.candidates}
    assert states == {"git-wt-cli": rp.COMPLETE, "sniff-detection-cli": rp.NEW}, states
    assert any("exited 3" in failure for failure in result.failures), result.failures


@case
def remote_tags_are_peeled_to_their_commit():
    commit, tag_object = "1" * 40, "2" * 40
    assert rp.peeled_tag_target(f"{commit}\trefs/tags/x-v1.0.0\n", "x-v1.0.0") == commit
    annotated = f"{tag_object}\trefs/tags/x-v1.0.0\n{commit}\trefs/tags/x-v1.0.0^{{}}\n"
    assert rp.peeled_tag_target(annotated, "x-v1.0.0") == commit
    assert rp.peeled_tag_target("", "x-v1.0.0") is None
    expect_error("ls-remote", rp.peeled_tag_target, f"{commit}\trefs/tags/x-v1.0.0-rc\n", "x-v1.0.0")


@case
def automation_env_scopes_the_token_and_identity_to_one_process():
    env = rp.automation_env({"GITHUB_TOKEN": "default", "GH_TOKEN": "x", "GIT_CONFIG_COUNT": "1",
                             "GIT_CONFIG_KEY_0": "core.autocrlf", "GIT_CONFIG_VALUE_0": "false"},
                            token="app", bot_login=BOT, bot_id=9)
    assert "GITHUB_TOKEN" not in env and "GH_TOKEN" not in env and env["GIT_TOKEN"] == "app"
    assert env["GIT_CONFIG_COUNT"] == "3" and env["GIT_CONFIG_KEY_0"] == "core.autocrlf"
    assert (env["GIT_CONFIG_KEY_1"], env["GIT_CONFIG_VALUE_1"]) == ("credential.helper", "")
    assert "${GIT_TOKEN}" in env["GIT_CONFIG_VALUE_2"] and "app" not in env["GIT_CONFIG_VALUE_2"]
    assert env["GIT_AUTHOR_EMAIL"] == f"9+{BOT}@users.noreply.github.com"


@case
def release_pr_output_names_only_approved_packages():
    assert rp.validate_release_pr_output('{"prs": []}') == []
    named = rp.validate_release_pr_output(json.dumps(
        {"prs": [{"releases": [{"package_name": "git-wt-cli", "version": "0.1.1"}]}]}))
    assert named == ["git-wt-cli"]
    expect_error("prs", rp.validate_release_pr_output,
                 json.dumps({"prs": [{"releases": [{"package_name": "darkmatter"}]}]}))
    expect_error("prs", rp.validate_release_pr_output, '{"prs": null}')


# --- integration routing ----------------------------------------------------

def release_event(tag) -> dict:
    return {"action": "published", "release": {"tag_name": tag, "draft": False}}


@case
def only_an_exact_integration_tag_selects_one_build():
    for package, directory in ir.INTEGRATIONS.items():
        for version in ("1.2.3", "0.1.0-rc.1", "2.0.0+build.5", "1.0.0-alpha-1.beta+exp.sha.5114f85"):
            selection = ir.select_integration("release", CANONICAL, release_event(f"{package}-v{version}"))
            assert selection.matrix() == {"include": [
                {"package": package, "package_dir": directory, "tag": f"{package}-v{version}"}]}, selection
    for tag in [
        "sniff-detection-cli-v0.2.0",
        "arcam-amp-integration-extra-v1.0.0",
        "xarcam-amp-integration-v1.0.0",
        "arcam-amp-integration-v1.0.0;rm -rf /",
        "arcam-amp-integration-v1.0.0\n",
        "arcam-amp-integration-v01.0.0",
        "arcam-amp-integration-v1.0",
        "arcam-amp-integration-1.0.0",
        "eversolo-integration-v$(id)",
    ]:
        selection = ir.select_integration("release", CANONICAL, release_event(tag))
        assert selection.matrix() == {"include": []}, (tag, selection)
    assert ir.select_integration("release", CANONICAL, {**release_event("eversolo-integration-v1.0.0"),
                                                         "action": "created"}).package is None
    assert ir.select_integration("release", "someone/fork", release_event("eversolo-integration-v1.0.0")).package is None
    for bad in (None, 7, ""):
        expect_error("release.tag_name", ir.select_integration, "release", CANONICAL, release_event(bad))
    expect_error("release", ir.select_integration, "release", CANONICAL, {"action": "published"})


# --- command wiring ---------------------------------------------------------

def run_cli(script: str, args: list[str], event, event_name: str, cwd: Path) -> tuple[int, str, dict]:
    event_path = SCRATCH / f"event-{len(os.listdir(SCRATCH))}.json"
    event_path.write_text(event if isinstance(event, str) else json.dumps(event), encoding="utf-8")
    output = event_path.with_suffix(".out")
    output.write_text("", encoding="utf-8")
    env = {**os.environ, "GITHUB_EVENT_PATH": str(event_path), "GITHUB_EVENT_NAME": event_name,
           "GITHUB_REPOSITORY": CANONICAL, "GITHUB_OUTPUT": str(output), "PYTHONUTF8": "1"}
    env.pop("GITHUB_STEP_SUMMARY", None)
    completed = subprocess.run([sys.executable, str(ROOT / "scripts" / "ci" / script), *args], cwd=cwd, env=env,
                               capture_output=True, text=True, encoding="utf-8", check=False)
    outputs = dict(line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines())
    return completed.returncode, completed.stdout + completed.stderr, outputs


@case
def the_select_command_writes_outputs_and_locates_errors():
    graph = release_repository("cli")
    repo = graph["repo"]
    git(repo, "checkout", "--quiet", "main")
    head = git(repo, "rev-parse", "HEAD")
    code, text, outputs = run_cli("release_path.py", ["select", "prepare"], workflow_run_event(event="schedule"),
                                  "workflow_run", repo)
    assert code == 0 and outputs == {"selected": "false"} and "::notice" in text, (code, text, outputs)
    code, text, outputs = run_cli("release_path.py", ["select", "prepare"], workflow_run_event(head_sha="d" * 40),
                                  "workflow_run", repo)
    assert code == 0 and outputs == {"selected": "false"} and "main advanced" in text, (code, text, outputs)
    code, text, outputs = run_cli("release_path.py", ["select", "prepare"], workflow_run_event(head_sha=head),
                                  "workflow_run", repo)
    assert code == 1 and "RELEASE_APP_ID" in text and outputs == {}, (code, text, outputs)
    code, text, outputs = run_cli("release_path.py", ["select", "prepare"], '{"workflow_run": {}, "workflow_run": {}}',
                                  "workflow_run", repo)
    assert code == 1 and "duplicate object key" in text and outputs == {}, (code, text, outputs)
    code, text, outputs = run_cli("integration_release.py", ["select", "--no-resolve"],
                                  release_event("sony-receiver-integration-v1.0.0"), "release", repo)
    assert code == 0 and outputs["selected"] == "true" and json.loads(outputs["matrix"])["include"][0]["package"] == \
        "sony-receiver-integration", (code, text, outputs)
    code, text, outputs = run_cli("integration_release.py", ["select", "--no-resolve"],
                                  release_event("git-wt-cli-v0.1.1"), "release", repo)
    assert code == 0 and outputs == {"selected": "false", "matrix": '{"include":[]}'}, (code, text, outputs)
    code, text, outputs = run_cli("integration_release.py", ["select", "--no-resolve"], {"action": "published"},
                                  "release", repo)
    assert code == 1 and "release" in text and outputs == {}, (code, text, outputs)


# --- shared validators, field by field -------------------------------------

def raised(function, *args, **kwargs) -> rp.ReleasePathError:
    try:
        function(*args, **kwargs)
    except rp.ReleasePathError as error:
        return error
    raise AssertionError(f"{function.__name__}{args!r} must be refused")


def at(path: str, message: str, function, *args, **kwargs) -> rp.ReleasePathError:
    """`function(...)` raises a `ReleasePathError` located exactly at `path`
    whose message contains `message`."""
    error = raised(function, *args, **kwargs)
    assert error.path == path and message in error.message, f"expected {path!r} …{message}…, found {error}"
    return error


@case
def shared_validators_locate_every_field_shape():
    sha = "0123456789abcdef" * 2 + "01234567"
    valid = {
        rp.req_str: "x",
        rp.req_bool: False,
        rp.req_int: 0,
        rp.req_repository: "owner-1/name.rs_x",
        rp.req_sha: sha,
        rp.req_object: {},
    }
    wrong_type = {
        rp.req_str: [7, True, ["x"], {}],
        rp.req_bool: [1, 0, "true", "false", []],
        rp.req_int: [True, False, "7", 7.0, []],
        rp.req_repository: [7, {"full_name": CANONICAL}],
        rp.req_sha: [int("1" * 40), ["a" * 40]],
        rp.req_object: [[], "", 0, False],
    }
    # Empty strings and format violations; an object's `{}` is valid above.
    malformed = {
        rp.req_str: [""],
        rp.req_repository: ["", "owner", "owner/name/extra", "/name", "owner/", "own er/name", "owner/na me"],
        rp.req_sha: ["", "a" * 39, "a" * 41, "A" * 40, "g" * 40, " " + "a" * 39],
    }
    for validator, value in valid.items():
        name = validator.__name__
        assert validator({"f": {"v": value}}, "f.v") == value, name
        absent = at("f.v", "is absent", validator, {"f": {}}, "f.v")
        null = at("f.v", "is null", validator, {"f": {"v": None}}, "f.v")
        assert absent.message != null.message, name
        at("f.v", "is absent", validator, {}, "f.v")
        at("f", "is null", validator, {"f": None}, "f.v")
        at("f", "must be an object", validator, {"f": 7}, "f.v")
        for wrong in wrong_type[validator]:
            at("f.v", "must be a", validator, {"f": {"v": wrong}}, "f.v")
        for bad in malformed.get(validator, []):
            at("f.v", "must", validator, {"f": {"v": bad}}, "f.v")


@case
def label_lists_are_read_whole_or_refused():
    def names(labels):
        return rp.req_label_names({"pr": {"labels": labels}}, "pr.labels")

    assert names([]) == []
    assert names([{"name": "release", "color": "fff"}, {"name": "automated"}]) == ["release", "automated"]
    at("pr.labels", "is absent", rp.req_label_names, {"pr": {}}, "pr.labels")
    at("pr.labels", "is null", rp.req_label_names, {"pr": {"labels": None}}, "pr.labels")
    for whole in ("release", {"name": "release"}, 7, True):
        at("pr.labels", "must be a list", names, whole)
    for labels, path, message in [
        ([{"name": "release"}, 7], "pr.labels[1]", "must be a label object"),
        ([{"name": "release"}, None], "pr.labels[1]", "must be a label object"),
        (["release", "automated"], "pr.labels[0]", "must be a label object"),
        ([7, 8, 9], "pr.labels[0]", "must be a label object"),
        ([{"name": "release"}, {"name": None}], "pr.labels[1].name", "is null"),
        ([{"name": "release"}, {}], "pr.labels[1].name", "is absent"),
        ([{"name": ""}], "pr.labels[0].name", "must not be empty"),
        ([{"name": 7}], "pr.labels[0].name", "must be a string"),
    ]:
        at(path, message, names, labels)


@case
def preparation_reads_every_selector_strictly():
    extra = workflow_run_event()
    extra["sender"] = {"login": "someone", "unexpected": [1, 2]}
    extra["workflow_run"].update(pull_requests=[], display_title=None, run_attempt=2)
    assert rp.select_preparation("workflow_run", CANONICAL, extra).selected, "unknown fields are ignored"
    # GitHub sends a null conclusion while a run is in progress; status decides first.
    running = edited(workflow_run_event(status="in_progress"), "workflow_run.conclusion", None)
    assert not rp.select_preparation("workflow_run", CANONICAL, running).selected
    for path, value, message in [
        ("workflow_run", None, "is null"),
        ("workflow_run.status", 7, "must be a string"),
        ("workflow_run.conclusion", None, "is null"),
        ("workflow_run.name", rp._ABSENT, "is absent"),
        ("workflow_run.event", [], "must be a string"),
        ("workflow_run.head_branch", "", "must not be empty"),
        ("workflow_run.repository", None, "is null"),
        ("workflow_run.repository.full_name", "rusty-biscuit", "owner/name"),
        ("workflow_run.head_repository.full_name", 7, "must be a string"),
        ("workflow_run.head_sha", "A" * 40, "40-digit"),
    ]:
        at(path, message, rp.select_preparation, "workflow_run", CANONICAL, edited(workflow_run_event(), path, value))
    at("workflow_run.status", "is absent", rp.select_preparation, "workflow_run", CANONICAL, {"action": "completed"})
    at("workflow_run.repository.full_name", CANONICAL, rp.select_preparation, "workflow_run", CANONICAL,
       workflow_run_event(repository={"full_name": "someone/fork"}))


@case
def publication_reads_every_selector_strictly():
    extra = pull_request_event()
    extra["sender"] = {"login": BOT}
    extra["pull_request"].update(draft=False, auto_merge=None, body=None)
    extra["pull_request"]["head"]["label"] = "yankeeinlondon:release-plz-2026-11-08"
    assert rp.select_publication("pull_request", CANONICAL, extra, BOT).selected, "unknown fields are ignored"
    # Closed without merging: GitHub sends no merge commit, and that is a no-op.
    unmerged = edited(edited(pull_request_event(), "pull_request.merged", False), "pull_request.merge_commit_sha", None)
    assert not rp.select_publication("pull_request", CANONICAL, unmerged, BOT).selected
    # An unrelated pull request from a deleted fork stays a no-op, not a crash.
    deleted_fork = edited(edited(pull_request_event(), "pull_request.labels", []), "pull_request.head.repo", None)
    assert not rp.select_publication("pull_request", CANONICAL, deleted_fork, BOT).selected
    for path, value, message in [
        ("action", 7, "must be a string"),
        ("pull_request", None, "is null"),
        ("pull_request.merged", None, "is null"),
        ("pull_request.labels", {"name": "release"}, "must be a list"),
        ("pull_request.base.ref", rp._ABSENT, "is absent"),
        ("pull_request.head.ref", 7, "must be a string"),
        ("pull_request.base.repo", None, "is null"),
        ("pull_request.base.repo.full_name", "owner", "owner/name"),
        ("pull_request.head.repo", None, "is null"),
        ("pull_request.head.repo.full_name", None, "is null"),
        ("pull_request.user", None, "is null"),
        ("pull_request.user.type", rp._ABSENT, "is absent"),
        ("pull_request.user.login", "", "must not be empty"),
        ("pull_request.merge_commit_sha", "b" * 39, "40-digit"),
        ("pull_request.head.sha", 7, "must be a string"),
        ("pull_request.number", "7", "must be an integer"),
        ("pull_request.number", True, "must be an integer"),
    ]:
        at(path, message, rp.select_publication, "pull_request", CANONICAL, edited(pull_request_event(), path, value), BOT)


# --- the release configuration loader ---------------------------------------

_WORKSPACE_CELLS = {
    "release": "false",
    "git_only": "true",
    "publish": "false",
    "release_always": "false",
    "pr_branch_prefix": '"release-plz-"',
    "git_tag_name": '"{{ package }}-v{{ version }}"',
}
_PACKAGES = ('[[package]]\nname = "git-wt-cli"\nrelease = true\n\n'
             '[[package]]\nname = "sniff-detection-cli"\nrelease = true\nchangelog_path = "sniff/cli/CHANGELOG.md"\n')


def config_text(packages: str = _PACKAGES, **cells) -> str:
    """A minimal valid configuration; a cell set to None is omitted. A
    `packages` that is a bare key (`package = ...`) goes before `[workspace]`,
    where TOML reads it as a root key rather than a workspace setting."""
    workspace = {**_WORKSPACE_CELLS, **cells}
    table = "[workspace]\n" + "".join(f"{key} = {value}\n" for key, value in workspace.items() if value is not None)
    return packages + table if packages.startswith("package =") else table + "\n" + packages


@case
def the_toml_loader_accepts_equivalent_syntax_and_refuses_ambiguity():
    canonical = rp.load_release_config(config_text())
    literal_and_comments = (
        "# leading comment\n[workspace] # trailing comment\nrelease = false\ngit_only = true # tags only\n"
        "publish = false\nrelease_always = false\npr_branch_prefix = 'release-plz-'\n"
        "git_tag_name = '{{ package }}-v{{ version }}'\n\n"
        "[[package]]\nname = 'git-wt-cli'\nrelease = true\n"
        '[[package]]\n"name" = "sniff-detection-cli"\nrelease = true\nchangelog_path = """sniff/cli/CHANGELOG.md"""\n'
    )
    dotted_and_inline = (
        "workspace.release = false\nworkspace.git_only = true\nworkspace.publish = false\n"
        "workspace.release_always = false\nworkspace.pr_branch_prefix = \"release-plz-\"\n"
        "workspace.git_tag_name = \"{{ package }}-v{{ version }}\"\n"
        'package = [\n  { name = "git-wt-cli", release = true },\n'
        '  { name = "sniff-detection-cli", release = true, changelog_path = "sniff/cli/CHANGELOG.md" },\n]\n'
    )
    for text in (literal_and_comments, dotted_and_inline):
        config = rp.load_release_config(text)
        assert (config.workspace, config.packages) == (canonical.workspace, canonical.packages), config
    for text in [
        config_text().replace("release = false\n", "release = false\nrelease = false\n", 1),
        "workspace.release = false\n" + config_text(),
        config_text() + "\n[workspace]\n",
        config_text().replace("git_only = true", "git_only = true false"),
        config_text().replace('"release-plz-"', '"release-plz-'),
        config_text() + "]]\n",
        config_text(release="null"),
    ]:
        at("release-plz.toml", "invalid TOML", rp.load_release_config, text)
    at("workspace", "is absent", rp.load_release_config, "")
    at("workspace", "must be an object", rp.load_release_config, "workspace = 7\n" + _PACKAGES)
    for packages, path, message in [
        ("package = []\n", "release-plz.toml:package", "must be exactly"),
        ("", "package", "is absent"),
        ('package = "git-wt-cli"\n', "release-plz.toml:package", "must be a list of tables"),
        ('package = ["git-wt-cli"]\n', "release-plz.toml:package[0]", "must be a table"),
        ("[[package]]\nrelease = true\n", "release-plz.toml:package[0].name", "is absent"),
        ("[[package]]\nname = 7\n", "release-plz.toml:package[0].name", "must be a string"),
        (_PACKAGES + '[[package]]\nname = "git-wt-cli"\n', "release-plz.toml:package[2].name", "duplicate"),
        (_PACKAGES.replace('"sniff/cli/CHANGELOG.md"', "7"), "release-plz.toml:package[1].changelog_path",
         "must be a string"),
        (_PACKAGES.replace('"sniff/cli/CHANGELOG.md"', '""'), "release-plz.toml:package[1].changelog_path",
         "must not be empty"),
        (_PACKAGES.replace("release = true", 'release = "true"', 1), "release-plz.toml:package[0].release",
         "must be a boolean"),
    ]:
        at(path, message, rp.load_release_config, config_text(packages=packages))


@case
def every_required_release_setting_is_typed_and_located():
    for key, wrong_type, wrong_value in [
        ("release", '"false"', "true"),
        ("git_only", "1", "false"),
        ("publish", '"no"', "true"),
        ("release_always", "0", "true"),
        ("pr_branch_prefix", "7", '"release-"'),
        ("git_tag_name", "true", '"v{{ version }}"'),
    ]:
        where = f"release-plz.toml:workspace.{key}"
        at(where, "must be a", rp.load_release_config, config_text(**{key: wrong_type}))
        at(where, "is absent", rp.load_release_config, config_text(**{key: None}))
        at(where, "must be", rp.load_release_config, config_text(**{key: wrong_value}))
    at("release-plz.toml:workspace.pr_branch_prefix", "must not be empty", rp.load_release_config,
       config_text(pr_branch_prefix='""'))
    # Optional carried settings: absent keeps release-plz's default; present must be typed.
    carried = rp.load_release_config(config_text(git_release_enable="true", git_release_name='"{{ package }}"'))
    assert carried.workspace["git_release_name"] == "{{ package }}"
    assert "git_release_draft" not in rp.load_release_config(config_text()).workspace
    rp.load_release_config(config_text(git_release_draft="false"))
    at("release-plz.toml:workspace.git_release_enable", "string or boolean", rp.load_release_config,
       config_text(git_release_enable="1"))
    at("release-plz.toml:workspace.git_release_draft", "must be false", rp.load_release_config,
       config_text(git_release_draft="true"))


# --- the GitHub forge over HTTP ---------------------------------------------

class LoopbackHTTPServer(ThreadingHTTPServer):
    """`ThreadingHTTPServer` without the startup name lookup: `HTTPServer.server_bind`
    calls `socket.getfqdn`, which blocks for tens of seconds on a host with slow DNS
    even for a numeric address."""

    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        self.server_name, self.server_port = self.server_address[:2]


@contextmanager
def no_name_resolution():
    """Make any host name lookup raise, so fixture startup that resolves names
    fails at once instead of waiting on the host's resolver."""

    def refuse(*args, **kwargs):
        raise AssertionError(f"fixture startup resolved a host name: {args!r}")

    names = ("getfqdn", "gethostbyaddr", "gethostbyname", "gethostbyname_ex", "getaddrinfo", "getnameinfo")
    saved = {name: getattr(socket, name) for name in names}
    for name in names:
        setattr(socket, name, refuse)
    try:
        yield
    finally:
        for name, original in saved.items():
            setattr(socket, name, original)


class FakeApi:
    """A local stand-in for the GitHub REST API. `routes` maps a request path,
    query included, to `(status, body)`; a str body is sent verbatim. Every
    request's path and headers are recorded."""

    def __init__(self):
        self.routes: dict[str, tuple[int, object]] = {}
        self.requests: list[dict] = []
        api = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802 - the http.server hook name
                api.requests.append({"path": self.path, "headers": self.headers})
                status, body = api.routes.get(self.path, (404, {"message": "Not Found"}))
                data = (body if isinstance(body, str) else json.dumps(body)).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args):
                pass

        with no_name_resolution():
            self.server = LoopbackHTTPServer(("127.0.0.1", 0), Handler)

    @property
    def url(self) -> str:
        return f"http://127.0.0.1:{self.server.server_address[1]}"

    def reset(self, routes: dict) -> None:
        self.routes, self.requests = routes, []


@contextmanager
def api_at(url: str):
    """`GITHUB_API_URL` set to `url` with any host proxy bypassed, restored after."""
    saved = {key: os.environ.get(key) for key in ("GITHUB_API_URL", "NO_PROXY", "no_proxy")}
    os.environ.update(GITHUB_API_URL=url, NO_PROXY="127.0.0.1", no_proxy="127.0.0.1")
    try:
        yield
    finally:
        for key, value in saved.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


@contextmanager
def serving():
    api = FakeApi()
    thread = threading.Thread(target=api.server.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True)
    thread.start()
    try:
        with api_at(api.url):
            yield api
    finally:
        api.server.shutdown()
        api.server.server_close()


def http_forge(token: str | None = "app-token", repo: Path = SCRATCH) -> rp.GitHubForge:
    return rp.GitHubForge(repository=CANONICAL, token=token, repo=repo, runner=rp.Runner(), git_env=rp._git_env())


TAG = "git-wt-cli-v0.1.1"
BY_TAG = f"/repos/{CANONICAL}/releases/tags/{TAG}"


def page(number: int) -> str:
    return f"/repos/{CANONICAL}/releases?per_page=100&page={number}"


def release_body(tag: str = TAG, **overrides) -> dict:
    return {"id": 1, "tag_name": tag, "draft": False, "html_url": f"https://example.invalid/{tag}", "assets": [],
            **overrides}


@case
def github_forge_reads_release_state_over_http():
    others = [release_body(f"other-v0.0.{n}") for n in range(100)]
    with serving() as api:
        forge = http_forge()
        api.reset({BY_TAG: (200, release_body())})
        assert forge.release(TAG) == rp.ReleaseRecord(exists=True, draft=False, url=f"https://example.invalid/{TAG}")
        assert [r["path"] for r in api.requests] == [BY_TAG], "a published Release needs no list scan"
        for request in api.requests:
            assert request["headers"]["Authorization"] == "Bearer app-token", request["headers"]
            assert request["headers"]["Accept"] == "application/vnd.github+json"
            assert request["headers"]["X-GitHub-Api-Version"] == "2022-11-28"

        # The by-tag endpoint hides drafts; the list shows them.
        api.reset({page(1): (200, [release_body("other-v1.0.0"), release_body(draft=True)])})
        assert forge.release(TAG) == rp.ReleaseRecord(exists=True, draft=True, url=f"https://example.invalid/{TAG}")
        assert [r["path"] for r in api.requests] == [BY_TAG, page(1)]

        api.reset({page(1): (200, others), page(2): (200, [release_body(draft=True)])})
        assert forge.release(TAG).draft is True, "a full page continues to the next"

        for listing in ([release_body("other-v1.0.0")], []):
            api.reset({page(1): (200, listing)})
            assert forge.release(TAG) == rp.ReleaseRecord(exists=False), listing
            assert [r["path"] for r in api.requests] == [BY_TAG, page(1)], "a short page ends the scan"

        api.reset({f"/users/{BOT.replace('[', '%5B').replace(']', '%5D')}": (200, {"id": 4242, "login": BOT})})
        assert forge.user_id(BOT) == 4242

        # A tag cannot escape its path segment.
        odd = "a/b v1?x#y"
        api.reset({})
        assert raised(forge.release, odd)
        assert api.requests[0]["path"] == f"/repos/{CANONICAL}/releases/tags/a%2Fb%20v1%3Fx%23y", api.requests

        api.reset({BY_TAG: (200, release_body())})
        http_forge(token=None).release(TAG)
        assert "Authorization" not in api.requests[0]["headers"], "no token sends no credential"


@case
def github_forge_never_reads_a_failure_or_malformed_response_as_absence():
    tag_source = f"GET releases/tags/{TAG}"
    raw_source = f"GET {BY_TAG}"
    user = f"/users/{BOT.replace('[', '%5B').replace(']', '%5D')}"
    with serving() as api:
        forge = http_forge()
        for routes, path, message in [
            ({BY_TAG: (500, {"message": "boom"})}, tag_source, "unexpected HTTP 500"),
            ({BY_TAG: (403, {"message": "rate limited"})}, tag_source, "unexpected HTTP 403"),
            ({BY_TAG: (401, {"message": "Bad credentials"})}, tag_source, "unexpected HTTP 401"),
            ({page(1): (500, [])}, "GET releases", "unexpected HTTP 500"),
            ({page(1): (200, {"releases": []})}, "GET releases", "non-list body"),
            ({page(1): (200, [7])}, "GET releases[0]", "must be an object"),
            ({page(1): (200, [{"id": 1}])}, "GET releases[0].tag_name", "is absent"),
            ({page(1): (200, [release_body(draft=None)])}, "GET releases[0].draft", "is null"),
            ({BY_TAG: (200, '{"tag_name": ')}, raw_source, "invalid JSON"),
            ({BY_TAG: (200, json.dumps(release_body())[:-1] + ', "draft": false}')}, raw_source, "duplicate"),
            ({BY_TAG: (200, "")}, raw_source, "invalid JSON"),
            ({BY_TAG: (200, json.dumps(release_body()) + " []")}, raw_source, "invalid JSON"),
            ({BY_TAG: (200, [release_body()])}, tag_source, "must be a Release object"),
            ({BY_TAG: (200, {k: v for k, v in release_body().items() if k != "draft"})}, f"{tag_source}.draft",
             "is absent"),
            ({BY_TAG: (200, release_body(draft="false"))}, f"{tag_source}.draft", "must be a boolean"),
            ({BY_TAG: (200, release_body(html_url=None))}, f"{tag_source}.html_url", "is null"),
            ({BY_TAG: (200, release_body("git-wt-cli-v0.1.2"))}, f"{tag_source}.tag_name", "expected"),
        ]:
            api.reset(routes)
            at(path, message, forge.release, TAG)
        for routes, path, message in [
            ({}, f"GET users/{BOT}", "unexpected HTTP 404"),
            ({user: (200, {"id": True})}, f"GET users/{BOT}.id", "must be an integer"),
            ({user: (200, {"id": "4242"})}, f"GET users/{BOT}.id", "must be an integer"),
            ({user: (200, {"login": BOT})}, f"GET users/{BOT}.id", "is absent"),
            ({user: (200, [{"id": 4242}])}, f"GET users/{BOT}", "must be an object"),
        ]:
            api.reset(routes)
            at(path, message, forge.user_id, BOT)
    # Nothing listens on a port that was just released.
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        closed = probe.getsockname()[1]
    with api_at(f"http://127.0.0.1:{closed}"):
        at(raw_source, "request failed", http_forge().release, TAG)


# --- tags on a real origin --------------------------------------------------

LIGHT, NEAR, ANNOTATED, MISSING = (
    "arcam-amp-integration-v1.0.0",
    "arcam-amp-integration-v1.0.0-rc.1",
    "eversolo-integration-v2.0.0-rc.1",
    "sony-receiver-integration-v9.9.9",
)


def tagged_origin(name: str) -> dict:
    """A bare `origin` holding a lightweight, a similarly named, and an
    annotated tag, and an empty consumer repository whose `origin` it is, so
    only the remote can answer."""
    source = SCRATCH / f"{name}-source"
    source.mkdir()
    git(source, "init", "--quiet", "--initial-branch=main")
    git(source, "commit", "--quiet", "--allow-empty", "-m", "one")
    first = git(source, "rev-parse", "HEAD")
    git(source, "commit", "--quiet", "--allow-empty", "-m", "two")
    second = git(source, "rev-parse", "HEAD")
    git(source, "tag", LIGHT, first)
    git(source, "tag", NEAR, second)
    git(source, "tag", "-a", "-m", "annotated", ANNOTATED, second)
    origin = SCRATCH / f"{name}-origin.git"
    git(SCRATCH, "clone", "--quiet", "--bare", str(source), str(origin))
    consumer = SCRATCH / f"{name}-consumer"
    consumer.mkdir()
    git(consumer, "init", "--quiet")
    git(consumer, "remote", "add", "origin", str(origin))
    orphan = SCRATCH / f"{name}-orphan"
    orphan.mkdir()
    git(orphan, "init", "--quiet")
    return {"consumer": consumer, "orphan": orphan, "first": first, "second": second,
            "tag_object": git(source, "rev-parse", ANNOTATED)}


@case
def remote_tag_targets_are_peeled_from_a_real_origin():
    remote = tagged_origin("forge-tags")
    forge = http_forge(repo=remote["consumer"])
    assert forge.remote_tag_target(LIGHT) == remote["first"]
    assert forge.remote_tag_target(NEAR) == remote["second"]
    assert remote["tag_object"] != remote["second"]
    assert forge.remote_tag_target(ANNOTATED) == remote["second"], "an annotated tag answers its commit"
    assert forge.remote_tag_target(MISSING) is None
    error = raised(http_forge(repo=remote["orphan"]).remote_tag_target, LIGHT)
    assert error.path.startswith("git ls-remote"), "an unreachable origin is an error, never absence"


def run_main(main, argv: list[str], event, event_name: str, env: dict | None = None, **kwargs) -> tuple[int, str, dict]:
    """A command entry point run in-process with a written event; returns its
    exit status, its output, and the step outputs it wrote."""
    event_path = SCRATCH / f"event-{len(os.listdir(SCRATCH))}.json"
    event_path.write_text(event if isinstance(event, str) else json.dumps(event), encoding="utf-8")
    output = event_path.with_suffix(".out")
    output.write_text("", encoding="utf-8")
    full_env = {**os.environ, "GITHUB_EVENT_PATH": str(event_path), "GITHUB_EVENT_NAME": event_name,
                "GITHUB_REPOSITORY": CANONICAL, "GITHUB_OUTPUT": str(output), **(env or {})}
    captured = io.StringIO()
    with redirect_stdout(captured), redirect_stderr(captured):
        code = main(argv, env=full_env, **kwargs)
    outputs = dict(line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines())
    return code, captured.getvalue(), outputs


@case
def integration_select_resolves_the_tag_commit_from_origin():
    remote = tagged_origin("integration-tags")
    consumer = ["select", "--repo", str(remote["consumer"])]
    for tag, package, commit in [(LIGHT, "arcam-amp-integration", remote["first"]),
                                 (ANNOTATED, "eversolo-integration", remote["second"])]:
        code, text, outputs = run_main(ir.main, consumer, release_event(tag), "release")
        assert code == 0, text
        assert outputs == {
            "selected": "true", "package": package, "tag": tag, "commit": commit,
            "matrix": json.dumps({"include": [{"package": package, "package_dir": ir.INTEGRATIONS[package],
                                               "tag": tag}]}, separators=(",", ":")),
        }, outputs
    code, text, outputs = run_main(ir.main, consumer, release_event(MISSING), "release")
    assert code == 1 and "does not exist on origin" in text and outputs == {}, (code, text, outputs)
    code, text, outputs = run_main(ir.main, ["select", "--repo", str(remote["orphan"])], release_event(LIGHT), "release")
    assert code == 1 and "ls-remote" in text and outputs == {}, (code, text, outputs)
    # Resolution runs only for a selected integration; an unrelated Release needs no origin.
    code, text, outputs = run_main(ir.main, ["select", "--repo", str(remote["orphan"])],
                                   release_event("git-wt-cli-v0.1.1"), "release")
    assert code == 0 and outputs == {"selected": "false", "matrix": '{"include":[]}'}, (code, text, outputs)


# --- command wiring per loader ----------------------------------------------

class RecordingRunner(rp.Runner):
    """Real Git; every command recorded; `release-plz` answered by `release_plz`."""

    def __init__(self, release_plz=None):
        self.calls: list[dict] = []
        self.release_plz = release_plz

    def run(self, argv, *, cwd, env=None):
        self.calls.append({"argv": list(argv), "cwd": cwd, "env": dict(env or {})})
        if argv[0] == "release-plz":
            assert self.release_plz is not None, f"release-plz must not run: {argv}"
            return self.release_plz(argv, cwd, env)
        return super().run(argv, cwd=cwd, env=env)


APP_ENV = {"RELEASE_APP_ID": "123", "HAS_RELEASE_APP_PRIVATE_KEY": "true", "RELEASE_APP_BOT_LOGIN": BOT}


@case
def select_commands_refuse_a_malformed_configuration_before_any_credential():
    graph = release_repository("cli-config")
    repo = graph["repo"]
    argv = ["select", "publish", "--repo", str(repo)]
    merged = edited(edited(pull_request_event(), "pull_request.merge_commit_sha", graph["M"]),
                    "pull_request.head.sha", graph["H"])
    code, text, outputs = run_main(rp.main, argv, merged, "pull_request", APP_ENV)
    assert code == 0 and outputs == {"selected": "true", "merge_sha": graph["M"], "head_sha": graph["H"],
                                     "number": "7", "base_sha": graph["A"]}, (code, text, outputs)

    # A deleted head repository on a plausible proposal: refused before Git or a credential is touched.
    runner = RecordingRunner()
    code, text, outputs = run_main(rp.main, argv, edited(merged, "pull_request.head.repo", None), "pull_request",
                                   APP_ENV, runner=runner)
    assert code == 1 and "pull_request.head.repo: is null" in text and outputs == {}, (code, text, outputs)
    assert runner.calls == [], runner.calls

    # The configuration at the proposal's final commit has a duplicate key.
    git(repo, "checkout", "--quiet", "release-plz-x")
    write(repo, "release-plz.toml", SHIPPED_CONFIG.replace("git_only = true\n", "git_only = true\ngit_only = true\n"))
    git(repo, "commit", "--quiet", "-am", "duplicate key")
    final = git(repo, "rev-parse", "HEAD")
    git(repo, "checkout", "--quiet", "--detach", graph["B"])
    git(repo, "merge", "--quiet", "--no-ff", "-m", "M2", final)
    merge = git(repo, "rev-parse", "HEAD")
    event = edited(edited(merged, "pull_request.merge_commit_sha", merge), "pull_request.head.sha", final)
    code, text, outputs = run_main(rp.main, argv, event, "pull_request", APP_ENV)
    assert code == 1 and f"{final}:release-plz.toml: invalid TOML" in text and outputs == {}, (code, text, outputs)

    # Preparation reads the checked-out `main`'s configuration.
    git(repo, "checkout", "--quiet", "main")
    head = git(repo, "rev-parse", "HEAD")
    prepare = ["select", "prepare", "--repo", str(repo)]
    code, text, outputs = run_main(rp.main, prepare, workflow_run_event(head_sha=head), "workflow_run", APP_ENV)
    assert code == 0 and outputs == {"selected": "true", "validated_sha": head}, (code, text, outputs)
    for config, located in [
        (SHIPPED_CONFIG.replace("publish = false\n", "publish = false\npublish = false\n"), "release-plz.toml: invalid TOML"),
        (SHIPPED_CONFIG.replace("git_only = true", 'git_only = "true"'), "release-plz.toml:workspace.git_only"),
    ]:
        write(repo, "release-plz.toml", config)
        code, text, outputs = run_main(rp.main, prepare, workflow_run_event(head_sha=head), "workflow_run", APP_ENV)
        assert code == 1 and located in text and outputs == {}, (code, text, outputs)


@case
def calculation_report_lists_the_approved_bumps_and_every_change():
    graph = release_repository("calculation-report")
    repo = graph["repo"]
    git(repo, "checkout", "--quiet", "main")
    # What `release-plz update` leaves: both CLIs bumped from main's 9.9.9 and
    # 0.1.1, a tracked edit sorted first, and a new changelog.
    write(repo, "README.md", "edited\n")
    write(repo, "sniff/cli/Cargo.toml", manifest("sniff-detection-cli", '"9.9.10"'))
    write(repo, "worktree/cli/Cargo.toml", manifest("git-wt-cli", '"0.1.2"'))
    write(repo, "worktree/cli/CHANGELOG.md", "## [0.1.2]\n")
    output = SCRATCH / "calculation-report" / "report.json"
    argv = ["calculation-report", "--repo", str(repo), "--output", str(output)]
    code, text, _ = run_main(rp.main, argv, {}, "workflow_dispatch")
    assert code == 0, text
    report = json.loads(output.read_text(encoding="utf-8"))
    assert report["version_changes"] == {
        "git-wt-cli": {"from": "0.1.1", "to": "0.1.2", "manifest": "worktree/cli/Cargo.toml"},
        "sniff-detection-cli": {"from": "9.9.9", "to": "9.9.10", "manifest": "sniff/cli/Cargo.toml"},
    }, report
    assert report["tracked_changes"] == [" M README.md", " M sniff/cli/Cargo.toml", " M worktree/cli/Cargo.toml"]
    assert report["untracked_not_ignored"] == ["worktree/cli/CHANGELOG.md"]
    assert report["proves_initial_release"] is True and report["unexpected_packages"] == []
    assert "`git-wt-cli`: 0.1.1 → 0.1.2" in text and "new: `worktree/cli/CHANGELOG.md`" in text, text

    # A bump outside the approved set fails the dry run and is named.
    write(repo, "other/Cargo.toml", manifest("other", '"0.2.0"'))
    code, text, _ = run_main(rp.main, argv, {}, "workflow_dispatch")
    report = json.loads(output.read_text(encoding="utf-8"))
    assert code == 1 and report["unexpected_packages"] == ["other"] and "['other']" in text, (code, text)

    # Nothing changed: reported, not an error, and not proof of the proposal.
    git(repo, "checkout", "--quiet", "--", ".")
    (repo / "worktree/cli/CHANGELOG.md").unlink()
    code, text, _ = run_main(rp.main, argv, {}, "workflow_dispatch")
    report = json.loads(output.read_text(encoding="utf-8"))
    assert code == 0 and report["proves_initial_release"] is False and report["tracked_changes"] == [], report
    assert "does not prove the initial two-package proposal" in text, text


def calculation_attributes(env: dict, *, outside: Path) -> Path:
    """The `core.attributesFile` a release-plz process received, checked while
    that process runs: the repository's PowerShell rule, outside the checkout."""
    count = int(env["GIT_CONFIG_COUNT"])
    [path] = [Path(env[f"GIT_CONFIG_VALUE_{index}"]) for index in range(count)
              if env[f"GIT_CONFIG_KEY_{index}"] == "core.attributesFile"]
    assert path.read_text(encoding="utf-8") == rp.CALCULATION_ATTRIBUTES, path
    assert not path.resolve().is_relative_to(outside.resolve()), (path, outside)
    return path


@case
def update_calculates_in_the_shared_environment_without_credentials():
    graph = release_repository("update")
    repo = graph["repo"]
    git(repo, "checkout", "--quiet", "main")
    seen: list[Path] = []

    def release_plz(argv, cwd, env):
        seen.append(calculation_attributes(env, outside=repo))
        return subprocess.CompletedProcess(argv, 0, "updated\n", "debug log\n")

    inherited = {"GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "core.autocrlf", "GIT_CONFIG_VALUE_0": "false",
                 "GITHUB_TOKEN": "default"}
    runner = RecordingRunner(release_plz)
    code, text, _ = run_main(rp.main, ["update", "--repo", str(repo)], {}, "workflow_dispatch", inherited,
                             runner=runner)
    assert code == 0 and "updated" in text and "debug log" in text, (code, text)
    [call] = runner.calls
    assert call["argv"] == ["release-plz", "update", "--config", "release-plz.toml"], call["argv"]
    assert call["cwd"] == repo.resolve(), call
    env = call["env"]
    # Appended after inherited entries; no credential is added and none is removed.
    assert env["GIT_CONFIG_COUNT"] == "2" and env["GIT_CONFIG_KEY_0"] == "core.autocrlf", env
    assert env["GIT_CONFIG_KEY_1"] == "core.attributesFile", env
    assert "GIT_TOKEN" not in env and env["GITHUB_TOKEN"] == "default"
    assert len(seen) == 1 and not seen[0].exists(), "the attributes file is removed after the run"
    assert git(repo, "status", "--porcelain", "--untracked-files=all") == ""

    runner = RecordingRunner(lambda argv, cwd, env: subprocess.CompletedProcess(argv, 1, "", "cannot find package"))
    code, text, _ = run_main(rp.main, ["update", "--repo", str(repo)], {}, "workflow_dispatch", {}, runner=runner)
    assert code == 1 and "release-plz update exited 1" in text and "cannot find package" in text, (code, text)


@case
def release_pr_refuses_incomplete_credentials_and_leaves_the_checkout_clean():
    graph = release_repository("release-pr")
    repo = graph["repo"]
    git(repo, "checkout", "--quiet", "main")
    head = git(repo, "rev-parse", "HEAD")
    proposal = json.dumps({"prs": [{"releases": [{"package_name": "git-wt-cli", "version": "0.1.1"}]}]})

    seen_attributes: list[Path] = []

    def release_plz(argv, cwd, env):
        seen_attributes.append(calculation_attributes(env, outside=repo))
        # A proposal commit on its own branch, written without touching the checkout.
        commit = git(cwd, "commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "chore: release")
        git(cwd, "update-ref", "refs/heads/release-plz-proposal", commit)
        return subprocess.CompletedProcess(argv, 0, proposal, "")

    output = SCRATCH / "release-pr-output" / "release-pr.json"
    argv = ["release-pr", "--repo", str(repo), "--output", str(output)]
    user = f"/users/{BOT.replace('[', '%5B').replace(']', '%5D')}"
    with serving() as api:
        for env, named in [({"RELEASE_APP_BOT_LOGIN": BOT}, "GIT_TOKEN"),
                           ({"GIT_TOKEN": "app-token", "RELEASE_APP_BOT_LOGIN": " "}, "RELEASE_APP_BOT_LOGIN")]:
            api.reset({user: (200, {"id": 4242})})
            runner = RecordingRunner()
            code, text, _ = run_main(rp.main, argv, {}, "workflow_run", env, runner=runner)
            assert code == 1 and f"{named}:" in text, (code, text)
            assert runner.calls == [] and api.requests == [], (runner.calls, [r["path"] for r in api.requests])

        api.reset({user: (200, {"id": 4242, "login": BOT})})
        runner = RecordingRunner(release_plz)
        code, text, _ = run_main(rp.main, argv, {}, "workflow_run",
                                 {"GIT_TOKEN": "app-token", "GITHUB_TOKEN": "default", "RELEASE_APP_BOT_LOGIN": BOT},
                                 runner=runner)
        assert code == 0 and "packages: git-wt-cli" in text, (code, text)
        [call] = runner.calls
        assert call["argv"][:2] == ["release-plz", "release-pr"] and call["cwd"] == repo.resolve(), call
        assert call["argv"][call["argv"].index("--repo-url") + 1] == f"https://github.com/{CANONICAL}"
        assert call["env"]["GIT_TOKEN"] == "app-token" and "GITHUB_TOKEN" not in call["env"]
        assert call["env"]["GIT_AUTHOR_EMAIL"] == f"4242+{BOT}@users.noreply.github.com"
        # The calculation entry comes first; the credential helper is appended after it.
        assert call["env"]["GIT_CONFIG_KEY_0"] == "core.attributesFile", call["env"]
        assert call["env"]["GIT_CONFIG_KEY_1"] == "credential.helper" and call["env"]["GIT_CONFIG_COUNT"] == "3"
        assert len(seen_attributes) == 1 and not seen_attributes[0].exists(), "removed after the run"
        assert [r["headers"]["Authorization"] for r in api.requests] == ["Bearer app-token"]
        assert output.read_text(encoding="utf-8") == proposal
        assert git(repo, "status", "--porcelain") == "" and git(repo, "rev-parse", "HEAD") == head
        assert git(repo, "rev-parse", "release-plz-proposal^") == head

        runner = RecordingRunner(lambda argv, cwd, env: subprocess.CompletedProcess(argv, 2, "", "boom"))
        code, text, _ = run_main(rp.main, argv, {}, "workflow_run", {"GIT_TOKEN": "app-token",
                                                                     "RELEASE_APP_BOT_LOGIN": BOT}, runner=runner)
        assert code == 2 and "exited 2" in text and "boom" in text, (code, text)


if sys.argv[2:] == ["--list"]:
    print("\n".join(function.__name__ for function in CASES))
    sys.exit(0)
by_name = {function.__name__: function for function in CASES}
selected = sys.argv[2:]
unknown = [name for name in selected if name not in by_name]
if not selected or unknown:
    print(f"name one or more cases; unknown: {unknown}")
    sys.exit(2)
failures = 0
for name in selected:
    try:
        by_name[name]()
    except Exception:  # noqa: BLE001 - every case failure is reported, then the run fails
        failures += 1
        print(f"FAIL {name}")
        traceback.print_exc(file=sys.stdout)
    else:
        print(f"ok   {name}")
sys.exit(1 if failures else 0)
