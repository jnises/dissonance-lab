#!/bin/bash

echo "Building all WASM targets..."

# Build the audio worklet first
echo "Building audio worklet..."
cd audio-worklet
trunk build --release

# Create a stable symlink for the audio worklet JS file
WORKLET_JS=$(ls dist/dissonance-audio-worklet-*.js | head -1)
if [ -f "$WORKLET_JS" ]; then
    ln -sf "$(basename "$WORKLET_JS")" dist/audio-worklet.js
    echo "Created stable symlink: audio-worklet.js -> $(basename "$WORKLET_JS")"
fi

cd ..

# Build the main application
echo "Building main application..."
trunk build --release

# Copy audio worklet files to main dist directory
echo "Copying audio worklet files to main dist..."
cp audio-worklet/dist/audio-worklet.js dist/
cp audio-worklet/dist/audio-worklet-processor.js dist/
cp audio-worklet/dist/dissonance-audio-worklet-*.js dist/
cp audio-worklet/dist/dissonance-audio-worklet-*_bg.wasm dist/

echo ""
echo "Build complete!"
echo "Main app: dist/"
echo "Audio worklet: audio-worklet/dist/"
echo "Audio worklet files copied to main dist/"
echo ""
echo "To serve the application, run: trunk serve"
