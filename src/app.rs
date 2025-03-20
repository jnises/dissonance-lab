use std::sync::{Arc, LazyLock};

use colorgrad::{BlendMode, Gradient as _};
use egui::{
    epaint::{PathShape, PathStroke}, pos2, text::LayoutJob, vec2, Align, Align2, Color32, FontId, Layout, Pos2, Rect, Sense, ThemePreference, UiBuilder, Vec2
};
use log::{error, info, warn};
use parking_lot::Mutex;
use web_time::{Duration, Instant};

use crate::{
    audio::AudioManager,
    interval_display,
    midi::MidiReader,
    piano_gui::{self, PIANO_WIDTH, PianoGui},
    synth::PianoSynth,
    theme,
    utils::colorgrad_to_egui,
};

type MidiSender = crossbeam::channel::Sender<wmidi::MidiMessage<'static>>;

struct Audio {
    _audio: AudioManager,
    tx: MidiSender,
}

enum AudioState {
    Uninitialized,
    Muted,
    Setup(Audio),
}

enum MidiState {
    NotConnected { last_checked: Option<Instant> },
    Connected(MidiReader),
}

pub struct TheoryApp {
    audio: AudioState,
    piano_gui: PianoGui,
    midi: MidiState,
    midi_to_audio_tx: Arc<Mutex<Option<MidiSender>>>,
}

impl Default for TheoryApp {
    fn default() -> Self {
        Self {
            audio: AudioState::Uninitialized,
            piano_gui: PianoGui::new(),
            midi: MidiState::NotConnected { last_checked: None },
            midi_to_audio_tx: Arc::new(Mutex::new(None)),
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
        self.audio = AudioState::Setup(Audio {
            tx: tx.clone(),
            _audio: audio,
        });
        *self.midi_to_audio_tx.lock() = Some(tx);
    }

    fn ensure_midi(&mut self) {
        const MIDI_CHECK_PERIOD: Duration = Duration::from_secs(1);
        match &mut self.midi {
            MidiState::NotConnected { last_checked }
                if last_checked.is_none()
                    || last_checked.map(|t| t.elapsed()) > Some(MIDI_CHECK_PERIOD) =>
            {
                let tx = self.midi_to_audio_tx.clone();
                match MidiReader::new(move |message| {
                    if let Some(tx) = &*tx.lock() {
                        let _ = tx.try_send(message.to_owned());
                    }
                }) {
                    Ok(reader) => {
                        self.midi = MidiState::Connected(reader);
                    }
                    Err(e) => {
                        match e {
                            crate::midi::Error::NoMidiInterface => {}
                            crate::midi::Error::Init(_)
                            | crate::midi::Error::Connect(_)
                            | crate::midi::Error::PortInfo(_) => {
                                error!("unable to set up midi: {e:?}");
                            }
                        }
                        self.midi = MidiState::NotConnected {
                            last_checked: Some(Instant::now()),
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for TheoryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_midi();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                const STATUS_HEIGHT: f32 = 40.0;
                ui.allocate_ui(
                    vec2(PIANO_WIDTH.min(ui.available_width()), STATUS_HEIGHT),
                    |ui| {
                        ui.horizontal(|ui| {
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
                            const MIDI_TEXT: &str = "MIDI";
                            const MIDI_FONT: FontId = FontId::proportional(10.0);
                            let galley = ui.painter().layout_no_wrap(
                                MIDI_TEXT.to_string(),
                                MIDI_FONT,
                                if matches!(&self.midi, MidiState::Connected(_)) {
                                    ui.visuals().text_color()
                                } else {
                                    ui.visuals().weak_text_color()
                                },
                            );
                            let text_size = galley.size();
                            let (response, painter) =
                                ui.allocate_painter(text_size, Sense::hover());
                            painter.galley(response.rect.left_top(), galley, Color32::WHITE);
                            response.on_hover_text(match &self.midi {
                                MidiState::NotConnected { .. } => "".to_string(),
                                MidiState::Connected(midi_reader) => {
                                    midi_reader.get_name().to_string()
                                }
                            });
                        });
                        ui.painter().text(
                            ui.max_rect().center_bottom(),
                            Align2::CENTER_BOTTOM,
                            "theory",
                            FontId::proportional(12.0),
                            colorgrad_to_egui(&theme::KEYBOARD_LABEL),
                        );
                        if self.piano_gui.selected_keys().count_ones() <= 1 {
                            ui.painter().text(
                                ui.max_rect().right_bottom(),
                                Align2::RIGHT_BOTTOM,
                                "shift for multi select",
                                FontId::proportional(10.0),
                                theme::TEXT_TERTIARY,
                            );
                        } else {
                            ui.painter().text(
                                ui.max_rect().right_bottom(),
                                Align2::RIGHT_BOTTOM,
                                self.piano_gui.selected_chord_name().unwrap(),
                                FontId::monospace(10.0),
                                theme::TEXT_TERTIARY,
                            );
                        }
                    },
                );
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
        const REPAINT_PERIOD: Duration = Duration::from_secs(10);
        ctx.request_repaint_after(REPAINT_PERIOD);
    }
}
