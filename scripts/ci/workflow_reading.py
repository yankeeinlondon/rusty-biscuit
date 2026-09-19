#!/usr/bin/env python3
"""One indentation-based reader for the shipped GitHub workflow YAML.

`scripts/ci` carries no third-party dependency, which is what lets the
pre-push hook run these suites on any developer machine. The stdlib has no
YAML parser, so the suites read the workflows by indentation instead: a job
header at two spaces, a step at six, a step's keys at eight, a block scalar's
body at ten. This module is the only place that assumption lives.

Every degradation is loud. A workflow with no jobs, a job that is not there,
a job with no `run:` steps, a step with no `run:` -- each raises
`WorkflowLayoutError`. A silently truncated step list would let a suite
execute a shorter job and still pass, which is the hazard this module exists
to remove (`reviews/2026-09-15-python-test-code/spike-1-results.md`).

A restructured workflow is therefore a test failure here, not a quiet loss of
coverage. Whichever layout the workflows adopt, change it in this file.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Sequence, Union

#: A workflow as a path to read, its whole text, or its already-split lines.
Source = Union[Path, str, Sequence[str]]

JOB_INDENT = 2
JOB_KEY_INDENT = 4
STEP_INDENT = 6
KEY_INDENT = 8
BLOCK_INDENT = 10

_STEP_OPENER = " " * STEP_INDENT + "- "
_JOB_HEADER = re.compile(r"^ {%d}([A-Za-z0-9_-]+):[ \t]*$" % JOB_INDENT)
#: Any other job-level line: indented exactly `JOB_INDENT`, not a comment.
#: A quoted (`"build":`) or flow-style job id lands here, and reading it as
#: part of the job above would hand a caller a job with two jobs' steps.
_JOB_LEVEL = re.compile(r"^ {%d}[^ #]" % JOB_INDENT)


class WorkflowLayoutError(AssertionError):
    """A workflow does not match the layout this reader assumes.

    An `AssertionError` so a layout drift reads as the failed contract it is
    rather than as an unrelated crash in a fixture.
    """


class JobStep:
    """One `run:` step of a job: its name, script, and whether failure is fatal."""

    def __init__(self, name: str, script: str, continue_on_error: bool) -> None:
        self.name = name
        self.script = script
        self.continue_on_error = continue_on_error


def _lines(source: Source) -> List[str]:
    if isinstance(source, Path):
        return source.read_text(encoding="utf-8").splitlines()
    if isinstance(source, str):
        return source.splitlines()
    return list(source)


def _text(source: Source) -> str:
    if isinstance(source, Path):
        return source.read_text(encoding="utf-8")
    if isinstance(source, str):
        return source
    return "\n".join(source) + "\n"


def _describe(source: Source) -> str:
    return str(source) if isinstance(source, Path) else "<workflow text>"


def job_blocks(source: Source) -> Dict[str, str]:
    """Every job id in a workflow, mapped to its block as text.

    The block excludes the job's own header line and keeps every following
    line at its original indentation, so the key indents above still apply.
    """
    body = _text(source).partition("\njobs:\n")[2]
    if not body:
        raise WorkflowLayoutError(
            "{}: no `jobs:` section at the top level".format(_describe(source))
        )
    blocks: Dict[str, str] = {}
    current = ""
    for line in body.splitlines(keepends=True):
        header = _JOB_HEADER.match(line)
        if header:
            current = header.group(1)
            blocks[current] = ""
        elif _JOB_LEVEL.match(line):
            raise WorkflowLayoutError(
                "{}: {!r} is at job level but is not a job header this reader "
                "can read".format(_describe(source), line.rstrip("\n"))
            )
        elif current:
            blocks[current] += line
    if not blocks:
        raise WorkflowLayoutError(
            "{}: `jobs:` declares no job at {} spaces".format(
                _describe(source), JOB_INDENT
            )
        )
    return blocks


def job_names(source: Source) -> List[str]:
    """Every job id in a workflow, in declaration order."""
    return list(job_blocks(source))


def job_block(source: Source, job: str) -> str:
    """One job's block as text."""
    blocks = job_blocks(source)
    if job not in blocks:
        raise WorkflowLayoutError(
            "{}: no job {!r}; declares {}".format(
                _describe(source), job, ", ".join(blocks) or "nothing"
            )
        )
    return blocks[job]


def _steps_body(block: str, described: str, job: str) -> List[str]:
    """The lines under a job's `steps:` key, and only those.

    A job's `needs:` is also a sequence at `STEP_INDENT`, so scanning the
    whole block for `- ` would read `- validation` as a step.
    """
    lines = block.splitlines()
    key = " " * JOB_KEY_INDENT + "steps:"
    if key not in lines:
        raise WorkflowLayoutError(
            "{}: job {!r} has no `steps:` at {} spaces".format(
                described, job, JOB_KEY_INDENT
            )
        )
    start = lines.index(key)
    end = next(
        (
            index
            for index in range(start + 1, len(lines))
            if lines[index].strip()
            and not lines[index].lstrip().startswith("#")
            and len(lines[index]) - len(lines[index].lstrip()) <= JOB_KEY_INDENT
        ),
        len(lines),
    )
    return lines[start + 1 : end]


def _step_blocks(lines: Sequence[str]) -> Iterator[List[str]]:
    """Each step of a `steps:` body, at the workflow's own indentation.

    The opening `- ` is rewritten to spaces so a key written on the opener
    (`- name: x`) sits at the same indent as the keys below it.
    """
    starts = [index for index, line in enumerate(lines) if line.startswith(_STEP_OPENER)]
    for position, start in enumerate(starts):
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        opener = " " * KEY_INDENT + lines[start][len(_STEP_OPENER):]
        yield [opener] + lines[start + 1 : end]


def _block_scalar_body(lines: Sequence[str], indent: int) -> str:
    """A block scalar's body, dedented, terminated by the first shallower line.

    A blank line belongs to the scalar: YAML keeps it, and a shell script's
    paragraph breaks depend on it.
    """
    body: List[str] = []
    for line in lines:
        if line.strip() and len(line) - len(line.lstrip()) < indent:
            break
        body.append(line[indent:])
    return "\n".join(body) + "\n"


def _step_key(step: Sequence[str], key: str) -> Optional[str]:
    prefix = " " * KEY_INDENT + key + ": "
    for line in step:
        if line.startswith(prefix):
            return line[len(prefix):]
    return None


def _declares(step: Sequence[str], key: str) -> bool:
    prefix = " " * KEY_INDENT + key + ":"
    return any(line.startswith(prefix) for line in step)


def job_run_steps(source: Source, job: str) -> List[JobStep]:
    """Every `run:` step of one job, in order, as standalone scripts.

    `uses:` steps have no script and are dropped. Running a whole job rather
    than one step is what lets a test see a toolchain step that precedes the
    decision under test.
    """
    described = _describe(source)
    steps: List[JobStep] = []
    for step in _step_blocks(_steps_body(job_block(source, job), described, job)):
        script = _step_run_script(step)
        if script is None:
            # Every Actions step carries one or the other. A step with
            # neither is a step this reader failed to read -- a flow-style
            # mapping, say -- and dropping it would shorten the job the
            # caller is about to run.
            if not _declares(step, "uses"):
                raise WorkflowLayoutError(
                    "{}: job {!r} has a step declaring neither `run:` nor "
                    "`uses:` at {} spaces: {!r}".format(
                        described, job, KEY_INDENT, step[0]
                    )
                )
            continue
        steps.append(
            JobStep(
                name=_step_key(step, "name") or "",
                script=script,
                continue_on_error=" " * KEY_INDENT + "continue-on-error: true" in step,
            )
        )
    if not steps:
        raise WorkflowLayoutError(
            "{}: job {!r} yields no `run:` step".format(described, job)
        )
    return steps


def _step_run_script(step: Sequence[str]) -> Optional[str]:
    """One step's script: a `run: |` body dedented, or a one-line `run:`."""
    literal = " " * KEY_INDENT + "run: |"
    prefix = " " * KEY_INDENT + "run: "
    for position, line in enumerate(step):
        if line == literal:
            return _block_scalar_body(step[position + 1 :], BLOCK_INDENT)
        if line.startswith(prefix):
            value = line[len(prefix):]
            if value.lstrip()[:1] in (">", "|", "*", "&"):
                raise WorkflowLayoutError(
                    "step {!r}: `run: {}` needs YAML this reader does not "
                    "implement; only `run: |` and a plain one-line `run:` "
                    "are supported".format(_step_key(step, "name") or "<unnamed>", value)
                )
            return value + "\n"
    return None


def step_script(source: Source, step_name: str) -> str:
    """The `run: |` body of one named step, dedented into a standalone script.

    Reaching the next step before `run: |` fails loudly rather than borrowing
    a neighbour's script.
    """
    step = _named_step(source, step_name)
    literal = " " * KEY_INDENT + "run: |"
    for position, line in enumerate(step):
        if line == literal:
            return _block_scalar_body(step[position + 1 :], BLOCK_INDENT)
        if line.startswith(" " * KEY_INDENT + "run: "):
            break
    raise WorkflowLayoutError(
        "{}: step {!r} has no `run: |` block".format(_describe(source), step_name)
    )


def step_run_lines(source: Source, step_name: str) -> List[str]:
    """One named step's `run:` scalar, verbatim and still indented.

    Verbatim because the caller compares two revisions of the same step for
    byte identity; a dedent would hide a whitespace-only change.
    """
    step = _named_step(source, step_name)
    literal = " " * KEY_INDENT + "run: |"
    prefix = " " * KEY_INDENT + "run:"
    for position, line in enumerate(step):
        if not line.startswith(prefix):
            continue
        if line != literal:
            return [line]
        body = [line]
        for following in step[position + 1 :]:
            if following and not following.startswith(" " * BLOCK_INDENT):
                break
            body.append(following)
        return body
    raise WorkflowLayoutError(
        "{}: step {!r} has no `run:`".format(_describe(source), step_name)
    )


def _named_step(source: Source, step_name: str) -> List[str]:
    """The first step of a workflow whose `name:` is `step_name`, job-agnostic.

    Job-agnostic because a step name is unique across the shipped workflows
    and the callers name one without naming its job.
    """
    lines = _lines(source)
    opener = _STEP_OPENER + "name: " + step_name
    starts = [index for index, line in enumerate(lines) if line.startswith(_STEP_OPENER)]
    for position, start in enumerate(starts):
        if lines[start] != opener:
            continue
        end = starts[position + 1] if position + 1 < len(starts) else len(lines)
        return [" " * KEY_INDENT + lines[start][len(_STEP_OPENER):]] + lines[start + 1 : end]
    raise WorkflowLayoutError(
        "{}: no step named {!r}".format(_describe(source), step_name)
    )
