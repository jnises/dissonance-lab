# Dissonance Lab

![screenshot](docs/screenshot.webp)

Test at https://jnises.github.io/dissonance-lab/

Small gui to explore the dissonance of different intervals and chords on a piano.
Includes midi input and a simple piano synth implemented as a webaudio worklet.

The colorful rows above the piano show the interval for each other key when one or more is pressed.
The pressed keys are considered the root of each interval even when it isn't the lower note.


## Requirements
* Rust toolchain ([rustup.rs](https://rustup.rs/))
* Trunk `cargo install trunk`
* wasm-pack `cargo install wasm-pack`

## Running
```
trunk serve --release
```
Navigate to http://127.0.0.1:8080/#dev

Note that you need to manually unmute by clicking the 🔇 button. This is due to the browser autoplay blocking feature.

### Development Environment

#### Quick Start
For the best development experience, use the included development tools that start both the frontend and log server:

```bash
cargo xtask dev
```

This single command will:
- Start the backend HTTP log server on port 3001
- Start the Trunk development server on port 8080
- Enable automatic builds with hot-reloading
- Forward frontend console logs to your terminal

Navigate to http://127.0.0.1:8080/#dev

When you're done, press `Ctrl+C` in the terminal to shut everything down gracefully.

#### Manual Setup (Alternative)
If you prefer to run the components manually:

1. **Start the log server** (in one terminal):
   ```bash
   cargo run -p dev-log-server
   ```

2. **Start Trunk** (in another terminal):
   ```bash
   trunk serve
   ```

#### Build Commands
For standalone builds, use Trunk directly:

```bash
# Build for development (with debug logging)
trunk build

# Build for release (optimized, no debug logging)
trunk build --release
```

## Development Tools

The project includes several development utilities via `cargo xtask`:

```bash
# Run comprehensive checks (build, format, clippy, tests, trunk build), mostly equivalent to CI
cargo xtask check

# Skip formatting check during development
cargo xtask check --skip-fmt

# Start development environment (frontend + log server)
cargo xtask dev

```

## Testing
```
cargo test
```
Tests run as native binaries by default.

## Development Notes
The project includes:
- **Frontend log forwarding**: Console logs from the browser are forwarded to the terminal during development
- **Audio worklet processing**: Real-time audio synthesis using WebAssembly

When you want to deploy to production you should make sure to update `cacheName` in `sw.js` to invalidate the cache.
