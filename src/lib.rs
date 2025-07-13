#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::DissonanceLabApp;
mod interval;
mod interval_display;
mod midi;
mod piano_gui;
mod theme;
mod utils;
pub mod webaudio;
