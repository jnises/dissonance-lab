use std::sync::{Arc, LazyLock};

use colorgrad::{BlendMode, Gradient as _};
use egui::{
    Color32, Pos2, Rect, Sense, ThemePreference, Vec2,
    epaint::{PathShape, PathStroke},
    pos2,
};
use log::{info, warn};

use crate::{
    audio::AudioManager, interval_display, piano_gui::{self, PianoGui}, synth::PianoSynth, theme, theory::is_key_black
};

struct Audio {
    _audio: AudioManager,
    tx: crossbeam::channel::Sender<wmidi::MidiMessage<'static>>,
}

enum AudioState {
    Uninitialized,
    Muted,
    Setup(Audio),
}

pub struct TheoryApp {
    audio: AudioState,
    piano_gui: PianoGui,
}

impl Default for TheoryApp {
    fn default() -> Self {
        Self {
            audio: AudioState::Uninitialized,
            piano_gui: PianoGui::new(),
        }
    }
}

impl TheoryApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom theme instead of default dark theme
        theme::setup_custom_theme(&cc.egui_ctx);
        Default::default()
    }

    fn setup_audio(&mut self) {
        assert!(matches!(
            self.audio,
            AudioState::Uninitialized | AudioState::Muted
        ));
        let (tx, rx) = crossbeam::channel::unbounded();
        let synth = Box::new(PianoSynth::new(rx));
        let audio = AudioManager::new(synth, |message| {
            warn!("{message}");
        });
        info!("audio initialized: {:?}", audio.get_name());
        self.audio = AudioState::Setup(Audio { tx, _audio: audio });
    }
}

impl eframe::App for TheoryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Audio controls at the top
                match self.audio {
                    AudioState::Uninitialized | AudioState::Setup(_) => {
                        if ui.button("🔈").clicked() {
                            self.audio = AudioState::Muted;
                        }
                    }
                    AudioState::Muted => {
                        if ui.button("🔇").clicked() {
                            self.setup_audio();
                        }
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    match interval_display::show(&mut self.piano_gui, ui) {
                        None => {}
                        Some(piano_gui::Action::Pressed(note)) => {
                            if matches!(self.audio, AudioState::Uninitialized) {
                                self.setup_audio();
                            }
                            if let AudioState::Setup(audio) = &self.audio {
                                audio
                                    .tx
                                    .send(wmidi::MidiMessage::NoteOn(
                                        wmidi::Channel::Ch1,
                                        note,
                                        wmidi::Velocity::MAX,
                                    ))
                                    .unwrap();
                            }
                        }
                        Some(piano_gui::Action::Released(note)) => {
                            if matches!(self.audio, AudioState::Uninitialized) {
                                self.setup_audio();
                            }
                            if let AudioState::Setup(audio) = &self.audio {
                                audio
                                    .tx
                                    .send(wmidi::MidiMessage::NoteOff(
                                        wmidi::Channel::Ch1,
                                        note,
                                        wmidi::Velocity::MAX,
                                    ))
                                    .unwrap();
                            }
                        }
                    }
                });
            });
        });
    }
}
