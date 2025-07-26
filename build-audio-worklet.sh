#!/bin/bash

# Build script for audio-worklet
# Called from a trunk hook, no need to run manually

set -e

echo "Building audio-worklet..."

cd audio-worklet

echo "Building WASM module..."

# Check if TRUNK_PROFILE is set to debug
if [ "$TRUNK_PROFILE" = "debug" ] || [ "$1" = "debug" ]; then
    echo "Building in debug mode..."
    wasm-pack build --dev --no-typescript --target no-modules --out-dir pkg --out-name audio-worklet
else
    echo "Building in release mode..."
    wasm-pack build --release --no-typescript --target no-modules --out-dir pkg --out-name audio-worklet
fi

echo "Audio worklet build complete!"
