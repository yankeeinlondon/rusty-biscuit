#!/usr/bin/env python3
"""Disposable Git, clone, and registration probes for Phase 1.

Run with `python3 spike_git.py`; every repository and copy lives in a temporary
directory. Assertions make unexpected Git behavior a failing probe.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def run(cwd, *args, input_bytes=None, check=True):
    result = subprocess.run(
        args, cwd=cwd, input=input_bytes, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False,
    )
    if check and result.returncode:
        raise RuntimeError(f"{args}: {result.stderr.decode(errors='replace')}")
    return result


def git(cwd, *args, input_bytes=None):
    return run(cwd, "git", *args, input_bytes=input_bytes).stdout


def write(root, name, data=b"x"):
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def registration(root, linked):
    gitdir = Path(git(linked, "rev-parse", "--absolute-git-dir").strip().decode())
    marker = gitdir / "gitdir"
    st = marker.stat()
    return {"admin": str(gitdir), "dev": st.st_dev, "ino": st.st_ino,
            "ctime_ns": st.st_ctime_ns, "mtime_ns": st.st_mtime_ns}


with tempfile.TemporaryDirectory(prefix="worktree-file-spike-") as temp:
    root = Path(temp) / "repo"
    root.mkdir()
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.name", "Spike")
    git(root, "config", "user.email", "spike@example.invalid")
    write(root, "tracked.env")
    git(root, "add", "tracked.env")
    git(root, "commit", "-qm", "initial")

    write(root, ".gitignore", b"*.env\n!revived.env\nignored/\nanchor.txt\n")
    write(root, "nested/.gitignore", b"local.secret\n")
    write(root, ".worktreeinclude", b"*.env\n**/*.secret\n/anchor.txt\n"
          b"ignored/\n!ignored/no.txt\n")
    write(root, "local.env")
    write(root, "revived.env")
    write(root, "nested/local.secret")
    write(root, "nested/note.secret")
    write(root, "ignored/deep/file.secret")
    write(root, "ignored/no.txt")
    write(root, "anchor.txt")
    write(root, "nested/anchor.txt")
    write(root, "space name.env")
    if os.name != "nt":
        write(root, "line\nbreak.env")
    info = root / ".git/info/exclude"
    info.write_bytes(b"info.secret\n")
    write(root, "info.secret")
    global_ignore = Path(temp) / "global-ignore"
    global_ignore.write_bytes(b"global.secret\n")
    write(root, "global.secret")
    git(root, "config", "core.excludesFile", str(global_ignore))

    raw = git(root, "ls-files", "--others", "--ignored",
              f"--exclude-from={root / '.worktreeinclude'}", "-z")
    candidates = set(raw.rstrip(b"\0").split(b"\0"))
    # Verbose, NUL and non-matching yield four fields per candidate.
    checked = git(root, "check-ignore", "--stdin", "-z", "--verbose",
                  "--non-matching", input_bytes=raw)
    fields = checked.split(b"\0")
    if fields[-1] == b"":
        fields.pop()
    assert len(fields) == 4 * len(candidates), (len(fields), len(candidates))
    standard = {fields[i + 3] for i in range(0, len(fields), 4)
                if fields[i] and not fields[i + 2].startswith(b"!")}
    selected = candidates & standard
    assert b"local.env" in selected
    assert b"tracked.env" not in selected
    assert b"revived.env" not in selected
    assert b"nested/note.secret" not in selected
    assert b"nested/local.secret" in selected
    assert b"info.secret" in selected
    assert b"global.secret" in selected
    assert b"ignored/deep/file.secret" in selected
    assert b"ignored/no.txt" in selected  # Negation cannot revive excluded parent.
    assert b"anchor.txt" in selected
    assert b"nested/anchor.txt" not in candidates  # Root-anchored include rule.
    assert b"space name.env" in selected
    if os.name != "nt":
        assert b"line\nbreak.env" in selected

    # An absolute fallback rules file must use the target checkout as its root.
    linked = Path(temp) / "linked"
    git(root, "worktree", "add", "-qb", "topic", str(linked))
    write(linked, ".gitignore", b"*.env\nanchor.txt\n")
    write(linked, "local.env")
    write(linked, "anchor.txt")
    write(linked, "nested/anchor.txt")
    fallback = git(linked, "ls-files", "--others", "--ignored",
                   f"--exclude-from={root / '.worktreeinclude'}", "-z")
    assert b"local.env\0" in fallback, fallback
    assert b"anchor.txt\0" in fallback, fallback
    assert b"nested/anchor.txt\0" not in fallback, fallback

    boundaries = {}
    nested_repo = root / "foreign"
    nested_repo.mkdir()
    git(nested_repo, "init", "-q")
    write(nested_repo, "file.env")
    symlink = root / "linked-dir"
    try:
        symlink.symlink_to(nested_repo, target_is_directory=True)
    except OSError:
        pass
    for name in ("foreign/file.env", "linked-dir/file.env"):
        result = git(root, "ls-files", "--others", "--ignored",
                     f"--exclude-from={root / '.worktreeinclude'}", "-z")
        boundaries[name] = name.encode() in result.split(b"\0")

    submodule = Path(temp) / "submodule-source"
    submodule.mkdir()
    git(submodule, "init", "-q", "-b", "main")
    git(submodule, "config", "user.name", "Spike")
    git(submodule, "config", "user.email", "spike@example.invalid")
    write(submodule, "tracked")
    git(submodule, "add", "tracked")
    git(submodule, "commit", "-qm", "initial")
    git(root, "-c", "protocol.file.allow=always", "submodule", "add", "-q",
        str(submodule), "module")
    write(root, "module/file.env")
    nested_linked = root / "nested-worktree"
    git(root, "worktree", "add", "-qb", "nested-topic", str(nested_linked))
    write(nested_linked, "file.env")
    result = git(root, "ls-files", "--others", "--ignored",
                 f"--exclude-from={root / '.worktreeinclude'}", "-z")
    for name in ("module/file.env", "nested-worktree/file.env"):
        boundaries[name] = name.encode() in result.split(b"\0")
    rules_file = root / ".worktreeinclude"
    original_rules = rules_file.read_bytes()
    rules_file.write_bytes(b"foreign/\nmodule/\nnested-worktree/\nlinked-dir/\n")
    directory_candidates = git(root, "ls-files", "--others", "--ignored",
                               f"--exclude-from={rules_file}", "-z").split(b"\0")
    boundaries["directory_pattern_candidates"] = [
        p.decode(errors="backslashreplace") for p in directory_candidates if p
    ]
    rules_file.write_bytes(original_rules)

    before = registration(root, linked)
    nonce_file = Path(before["admin"]) / "wt-copy-registration"
    first_nonce = os.urandom(16).hex()
    nonce_file.write_text(first_nonce)
    git(root, "worktree", "remove", "--force", str(linked))
    assert not nonce_file.exists()
    git(root, "worktree", "prune")
    git(root, "branch", "-D", "topic")
    git(root, "worktree", "add", "-qb", "topic", str(linked))
    after = registration(root, linked)
    second_nonce = os.urandom(16).hex()
    (Path(after["admin"]) / "wt-copy-registration").write_text(second_nonce)
    assert first_nonce != second_nonce
    assert before["admin"] == after["admin"]
    assert (before["dev"], before["ino"]) != (after["dev"], after["ino"]), (before, after)

    clone = {}
    src = Path(temp) / "clone-source"
    dst = Path(temp) / "clone-destination"
    src.write_bytes(b"secret")
    if os.name != "nt":
        src.chmod(0o600)
    if shutil.which("cp"):
        args = ("cp", "-c", str(src), str(dst)) if os.uname().sysname == "Darwin" else (
            "cp", "--reflink=always", str(src), str(dst))
        result = run(root, *args, check=False)
        clone = {"command": list(args[:2]), "exit": result.returncode,
                 "error": result.stderr.decode(errors="replace").strip()}
        if result.returncode == 0:
            clone["mode"] = oct(dst.stat().st_mode & 0o777)
            clone["independent"] = dst.read_bytes() == src.read_bytes()
            dst.write_bytes(b"changed")
            clone["independent"] &= src.read_bytes() == b"secret"

    print(json.dumps({"git": git(root, "--version").decode().strip(),
                      "candidates": sorted(p.decode(errors="backslashreplace") for p in candidates),
                      "selected": sorted(p.decode(errors="backslashreplace") for p in selected),
                      "boundaries": boundaries, "registration_before": before,
                      "registration_after": after, "clone": clone}, indent=2))
