#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::TheoryApp;
mod theory;
mod audio;
mod synth;
mod reverb;
mod piano_gui;
mod theme;
mod interval_display;
mod utils;