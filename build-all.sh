#!/bin/bash

echo "Building all WASM targets..."

# Build the main application
echo "Building main application..."
trunk build --release

# Build the audio worklet
echo "Building audio worklet..."
cd audio-worklet
trunk build --release
cd ..

echo ""
echo "Build complete!"
echo "Main app: dist/"
echo "Audio worklet: audio-worklet/dist/"
echo ""
echo "To serve the application, run: trunk serve"
