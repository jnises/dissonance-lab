#!/usr/bin/env bash
# Run tests with native target instead of wasm
set -euo pipefail

# Auto-detect the host target
HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')

cargo test --target "$HOST_TARGET" "$@"