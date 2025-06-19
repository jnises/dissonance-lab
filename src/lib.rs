#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::DissonanceLabApp;
mod audio;
mod interval;
mod interval_display;
mod midi;
mod piano_gui;
mod theme;
mod utils;
