"""Probe the installed passive schema parser, not the proposed function engine."""

import json
import os
from pathlib import Path
import subprocess
import tempfile


binary = os.environ.get("DARKMATTER_SPIKE_MD", "md")
fixture = Path(__file__).with_name("function-declarations.yaml")
definitions = [
    "number(required)",
    "number(integer; required)",
    "file(required)",
    "file(required)[](required)",
    "string(required)[](required)",
    "object(required)",
    "boolean(required)",
    "ip-address(required)",
    "literal(unstable; required)",
    "any(required)",
    "unknown(required)",
    "null(required)",
]
shape = (
    "{ name: string(required), category: string, order: number, description: string, "
    "evaluation: string, prototype_only: boolean, supplied_by: string, runtime_note: string, "
    "overloads: { parameters: { name: string(required), value: any(required), "
    "presence: enum(required, optional, variadic; required), conversion: string, "
    "on_unsupported: { diagnostic: string, result: boolean }, "
    "on_invalid_conversion: { diagnostic_when_statically_known: string, result: boolean }, "
    "on_unknown_type: { diagnostic: string, enabled_by_default: boolean } "
    "}[](required), returns: { value: any(required), fallible: boolean(required) }(required), "
    "true_facts: { argument: string(required), suitable_for: string(required) }[], "
    "runtime_note: string, example: { expression: string(required), result: string(required), "
    "verification: enum(executable, display-only; required), reason: string } "
    "}[](min(1); required) }[](min(1); required)"
)
print(subprocess.check_output([binary, "--version"], text=True).strip())
with tempfile.TemporaryDirectory(prefix="darkmatter-function-schema-") as directory:
    cases = [("candidate-envelope", "$schema:\n  functions: " + json.dumps(shape)
              + "\n" + fixture.read_text(), True)]
    cases.extend((definition, "$schema:\n  definition: type-definition(required)\n"
                  + "definition: " + json.dumps(definition) + "\n",
                  definition not in ("unknown(required)", "null(required)", "ip-address(required)"))
                 for definition in definitions)
    for index, (name, contents, expected_valid) in enumerate(cases):
        document = Path(directory) / f"case-{index}.md"
        document.write_text("---\n" + contents + "---\n")
        result = subprocess.run(
            [binary, "schema", "validate", "--no-trigger-schemas", "--format", "json", str(document)],
            cwd=directory, text=True, capture_output=True,
        )
        report = json.loads(result.stdout)
        print(json.dumps({"case": name, "exit_code": result.returncode,
                          "valid": report["valid"],
                          "messages": [problem["message"] for problem in report["problems"]]}))
        assert report["valid"] == expected_valid, (name, result.stdout, result.stderr)
        assert result.returncode == (0 if expected_valid else 1), (name, result.returncode)
