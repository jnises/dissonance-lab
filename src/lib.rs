#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::DissonanceLabApp;
mod audio;
mod interval;
mod interval_display;
mod limiter;
mod midi;
mod piano_gui;
mod reverb;
mod synth;
mod theme;
mod utils;
