#!/bin/bash

echo "Building audio worklet..."

# Build the audio worklet as a separate WASM module
cd audio-worklet
trunk build --release

echo "Audio worklet build complete!"
echo "Output should be in audio-worklet/dist/"
