#!/bin/bash

# Build script for audio-worklet
set -e

echo "Building audio-worklet..."

# Navigate to the audio-worklet directory
cd audio-worklet

# Build the WASM module
echo "Building WASM module..."
#rm -rf pkg
wasm-pack build --target no-modules --out-dir pkg --out-name audio-worklet

# Copy the worklet JavaScript wrapper
# echo "Copying worklet wrapper..."
# cp worklet.js pkg/

echo "Audio worklet build complete!"
