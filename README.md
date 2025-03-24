# Dissonance Lab

![screenshot](docs/screenshot.webp)

Test at https://jnises.github.io/dissonance-lab/

Small gui to explore the dissonance of different intervals and chords on a piano.
Includes a simple piano synth and midi input.

The colorful rows above the piano show the interval for each other key when one or more is pressed.
The pressed keys are considered the root of each interval even when it isn't the lower note.

## Requirements
* Rust toolchain ([rustup.rs](https://rustup.rs/))
* Trunk `cargo install trunk`

## Running

### Wasm
```
trunk serve --release
```
Navigate to http://127.0.0.1:8080/

Note that the we version requires you to manually unmute by clickin the 🔇 button. This is due to the browser autoplay blocking feature.

### Native
```
cargo run -r
```