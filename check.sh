#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')

cargo check --quiet --workspace --all-targets
cargo check --quiet --workspace --all-features --lib --target wasm32-unknown-unknown
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --quiet --workspace --all-features --target "$HOST_TARGET"
cargo test --quiet --workspace --doc
trunk build
