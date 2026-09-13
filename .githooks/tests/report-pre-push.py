#!/usr/bin/env python3
"""Publish the shell harness's measured results as JUnit and a GitHub summary."""

import csv
import html
import os
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET


def main():
    directory = Path(sys.argv[1])
    with (directory / "results.tsv").open() as source:
        rows = list(csv.reader(source, delimiter="\t"))
    if not rows:
        raise ValueError("the hook harness recorded no test results")
    failures = sum(row[2] == "fail" for row in rows)
    suite = ET.Element("testsuite", name="pre-push-hook", tests=str(len(rows)),
                       failures=str(failures), errors="0", skipped="0",
                       time=str(sum(int(row[3]) for row in rows)))
    summary = ["## Pre-push hook tests\n",
               f"**{len(rows) - failures} passed, {failures} failed, {len(rows)} total.**\n",
               "These test temporary repositories and fake workloads; they are not product test cells.\n",
               "| Test | Result | Seconds |", "|---|---|---:|"]
    for case_id, name, status, duration in rows:
        if status not in {"pass", "fail"}:
            raise ValueError(f"unknown test status: {status}")
        case = ET.SubElement(suite, "testcase", classname="pre_push_hook",
                             name=name, time=duration)
        log = (directory / f"{case_id}.log").read_text(errors="replace")
        log = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", log)
        log = re.sub(r"[\x00-\x08\x0b\x0c\x0e-\x1f]", "", log)
        if status == "fail":
            ET.SubElement(case, "failure", message=name).text = log
            if os.environ.get("GITHUB_ACTIONS") == "true":
                title = name.replace("%", "%25").replace(",", "%2C").replace(":", "%3A")
                print(f"::error title={title}::Hook regression failed; see the job summary and hook-test-results artifact.")
        ET.SubElement(case, "system-out").text = log
        summary.append(f"| {html.escape(name).replace('|', '&#124;')} | {'PASS' if status == 'pass' else '**FAIL**'} | {duration} |")
    ET.ElementTree(suite).write(directory / "junit.xml", encoding="utf-8", xml_declaration=True)
    summary.append("\nFailure logs, including captured hook stderr, are in the hook-test-results artifact.\n")
    for case_id, name, status, _ in rows:
        if status == "fail":
            log = (directory / f"{case_id}.log").read_text(errors="replace")
            summary.append(f"<details><summary>{html.escape(name)}</summary><pre>{html.escape(log[-12000:])}</pre></details>\n")
    document = "\n".join(summary)
    (directory / "summary.md").write_text(document)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write(document)


if __name__ == "__main__":
    main()
