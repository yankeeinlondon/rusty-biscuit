---
$schema:
    # this type is invalid today but would be accomodated for if the "file contents directive"
    # were introduced as discussed below
    crates: crate[]
# we should make `$types` a special type too that can define schema types but by itself
# has no impact on a schema document but it's value is that the `$schema:` property can
# then utilize this type as a "native type"
$types:
    crate:
        name: string(required)
        filepath: file(required)
# currently the `$schema` property allows for you to simply type the filepath as an alternative
# to defining the schema locally and that should remain the case but there are **often** cases 
# where we will want to have a variable be the "contents" of a file or URI and offering the
# `<< <resource>` syntax feels like a clear way to indicate this.
# 
# prelinary documentation for this future feature can be found at: @darkmatter/docs/inline/file-contents-directive.md
crates: "<< ./parsing-crates.yaml@filename"
sequence: 
    - initialize:
        - message: "🏃‍♂️ starting fleet research for Rust parsing crates"
        - info: |-
            We are starting _fleet research_ on Rust **parsing** crates can/should be considered when Parsing is a requirement in Rust
    - group:
        name: Perform Research
        compute: concurrent
        tasks:
            compose: crates
    - step:
        name: Summarize Research
        
---
