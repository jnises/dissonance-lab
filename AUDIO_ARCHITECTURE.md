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
✅ **Full AudioWorklet integration complete**

## Features Implemented

- **Dynamic AudioWorklet loading**: The main app asynchronously loads the audio worklet WASM module
- **Port communication**: Bidirectional message passing between main app and audio worklet
- **MIDI message routing**: MIDI events are routed from the main app to the audio worklet via message ports
- **Real-time audio processing**: Audio worklet handles synthesis and audio output in real-time
- **State management**: Audio states (Uninitialized, Initializing, Setup) with proper transitions
- **Error handling**: Comprehensive error handling for AudioWorklet setup and message passing

## Next Steps

1. **Performance optimization**: Optimize for real-time audio processing and reduce latency
2. **Testing**: Add integration tests for the audio pipeline
3. **Audio buffer management**: Implement proper audio buffer streaming and management
4. **Audio effects integration**: Connect the reverb and limiter effects to the audio pipeline

## Usage

The main application continues to work as before, but now has a foundation for WebAudio-based real-time audio processing through the separate audio worklet module.
