# Dissonance Lab - Audio Architecture

This project has been restructured to support WASM-only compilation with a modular audio architecture.

## Project Structure

```
dissonance-lab/
├── src/                          # Main application (UI, MIDI, etc.)
├── audio-types/                  # Shared types and traits
├── audio-engine/                 # Audio synthesis engine (synth, reverb, limiter)
├── audio-worklet/               # WebAudio AudioWorklet implementation
├── build-all.sh                 # Build script for all components
└── build-audio-worklet.sh       # Build script for just the audio worklet
```

## Architecture

### Crates

1. **`dissonance-lab`** (main): The primary application containing the UI, MIDI handling, and main audio manager
2. **`dissonance-audio-types`**: Shared types, traits, and message definitions for communication between components
3. **`dissonance-audio-engine`**: The core audio synthesis engine with piano synthesis, reverb, and limiting
4. **`dissonance-audio-worklet`**: WebAudio AudioWorklet processor for real-time audio in the browser

### Audio Flow

```
Main App (UI/MIDI) 
    ↓ MIDI messages
AudioManager 
    ↓ WebAudio messages
AudioWorklet (separate WASM binary)
    ↓ audio processing
PianoSynth + Effects
    ↓ audio output
Web Browser Audio
```

## Building

### Build All Components
```bash
./build-all.sh
```

### Build Individual Components
```bash
# Main application
trunk build --release

# Audio worklet only
./build-audio-worklet.sh
```

### Development
```bash
# Serve main application with hot reload
trunk serve

# Check all crates
cargo check --workspace --target wasm32-unknown-unknown
```

## Key Features

- **WASM-only compilation**: No native dependencies required
- **Modular architecture**: Audio engine can be used independently
- **WebAudio integration**: Uses AudioWorklet for real-time audio processing
- **Separate WASM binaries**: Audio worklet builds as independent module
- **Shared types**: Type-safe communication between components
- **MIDI support**: Maintains existing MIDI functionality in WASM

## Current Status

✅ **All crates compile for WASM**
✅ **Modular architecture established** 
✅ **Shared types and communication defined**
✅ **AudioWorklet foundation ready**
⚠️ **Full AudioWorklet integration pending** (requires async setup)

## Next Steps

1. **Complete AudioWorklet integration**: Implement async module loading and port communication
2. **Audio processing**: Connect the synthesis engine to the WebAudio pipeline
3. **Performance optimization**: Optimize for real-time audio processing
4. **Testing**: Add integration tests for the audio pipeline

## Usage

The main application continues to work as before, but now has a foundation for WebAudio-based real-time audio processing through the separate audio worklet module.
