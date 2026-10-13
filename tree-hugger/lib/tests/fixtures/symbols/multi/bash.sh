#!/usr/bin/env bash
# Multi-file fixture: Bash.

DEFAULT_TARGET="build"

build_target() {
    local target="$1"
    echo "building ${target:-$DEFAULT_TARGET}"
}

function clean_target {
    echo "cleaning"
}
