#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

cargo check --quiet --workspace --all-targets
# TODO: why do we need this one?
cargo check-wasm --quiet --workspace --all-features --lib
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --quiet --workspace --all-features
cargo test --quiet --workspace --doc
trunk build
