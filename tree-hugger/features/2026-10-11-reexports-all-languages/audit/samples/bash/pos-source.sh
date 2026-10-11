#!/usr/bin/env bash
source ./lib.sh
. ./other.sh
source "$DIR/x.sh"
[ -f a.sh ] && source a.sh
if true; then . ./cond.sh; fi
