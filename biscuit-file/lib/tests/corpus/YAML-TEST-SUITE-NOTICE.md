# YAML Test Suite notice

The `yaml_test_suite` section in `yaml_corpus.json` vendors a minimal subset of
the [YAML Test Suite](https://github.com/yaml/yaml-test-suite) release
`data-2022-01-17`, pinned to data commit
`6e6c296ae9c9d2d5c4134b4b64d01b29ac19ff6f`.

The vendored inputs are the exact contents of these release paths:

- `9FMG/in.yaml` — valid multi-level mapping
- `4H7K/in.yaml` — expected parse failure
- `2JQS/in.yaml` — duplicate empty mapping key
- `3GZX/in.yaml` — anchors and aliases
- `7ZZ5/in.yaml` — flow collections
- `G4RS/in.yaml` — quoted scalars and escapes
- `L383/in.yaml` — multi-document stream

The release does not contain a BOM case. The corpus retains BOM coverage in
its separately identified `regression` and `monorepo` cases, including BOM plus
CRLF and multi-key frontmatter.

## License

MIT License

Copyright (c) 2016-2020 Ingy döt Net

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
