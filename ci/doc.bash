#!/usr/bin/bash

# fail fast
#
set -e

# print each command before it's executed
#
set -x

cargo doc --all-features
cargo test --doc
