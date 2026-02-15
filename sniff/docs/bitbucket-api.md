---
title: bitbucket REST API Research
description: Endpoint coverage, common pitfalls, and Rust examples for practical bitbucket REST API usage.
prompt: |-
    Do research into the Bitbucket REST API. Document all of it's API endpoints. Discuss common gotcha's developers encounter when working with this API and how they are able to get around them. Create a Markdown table (with Markdown links) of all the key resources you've found for this API. All code examples should be written in Rust. Demonstrate the how to use the API for the following use cases:

    1. Get a list of all `README.md` filepaths (case insensitive) in the repo and then provides a means to get the content of any of these readme's that the caller wants.
    2. Get a list of all PR's for the repo with associated metadata.
    3. Get a list of all Issues for the repo along with base metadata and a means to dig into further information (if this further information requires a separate call).
    4. Get a list of all Tags from the repos including all metadata; make sure you can distinguish between a normal tag and one that is deemed a "release"

    Finally near the end, add a section which compares the Github and bitbucket API's in capability and approach. You can refer to the Github API's design in the [Github API](@sniff/docs/github.md).

    Write all your findings to @sniff/docs/bitbucket-api.md ; replace the body of this file if it already has content but retain the frontmatter. Update the the `last_updated` frontmatter property to today's date.
last_updated: 2026-02-15
model: "kimi-k2.5"
---

