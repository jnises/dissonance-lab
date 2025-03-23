# Dissonance Lab

![screenshot](docs/screenshot.webp)

Test at https://jnises.github.io/dissonance-lab/

Small gui to explore the dissonance of different intervals and chords on a piano.

Includes a simple piano synth and midi input.

The colorful rows above the piano show the interval for each other key when one or more is pressed.

## Requirements
* Rust toolchain ([rustup.rs](https://rustup.rs/))
* Trunk `cargo install trunk`

## Running

### Wasm
```
trunk serve --release
```

### Native
```
cargo run -r
```