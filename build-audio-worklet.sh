#!/bin/bash

# Build script for audio-worklet

set -e

echo "Building audio-worklet..."

cd audio-worklet

echo "Building WASM module..."
wasm-pack build --release --no-typescript --target no-modules --out-dir pkg --out-name audio-worklet

echo "Audio worklet build complete!"
