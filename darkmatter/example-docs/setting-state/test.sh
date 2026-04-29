#!/usr/bin/env bash

echo
md compose ./test.md --set '{ name: "Bob", iteration: 1, example: "using JSON5 and --set" }' | md
echo
echo
md compose ./test.md name="Bob" iteration=2 example="using key='value' syntax" | md
echo
