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
Navigate to http://127.0.0.1:8080/

Note that you need to manually unmute by clicking the 🔇 button. This is due to the browser autoplay blocking feature.

## Testing
```
cargo test
```

Tests run as native binaries by default.

## Local development
```
trunk serve
```
Then open http://127.0.0.1:8080/#dev
The #dev disables the pwa cache so that we get the latest version of the page.

When you want to deploy to production you should make sure to update `cacheName` in `sw.js` to invalidate the cache.
