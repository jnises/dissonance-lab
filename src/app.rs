use crossbeam::channel;
use egui::{Align, Align2, Color32, FontId, Layout, RichText, Sense, vec2};
use log::{error, warn};
use parking_lot::Mutex;
use std::sync::Arc;
use web_time::{Duration, Instant};

use crate::{
    interval_display,
    midi::MidiReader,
    piano_gui::{self, PIANO_WIDTH, PianoGui},
    //synth::PianoSynth,
    theme,
    webaudio::{self, WebAudio},
};

type MidiSender = channel::Sender<wmidi::MidiMessage<'static>>;

struct Audio {
    //_audio: AudioManager,
    tx: MidiSender,
}

enum AudioState {
    Uninitialized,
    Muted,
    Playing(WebAudio),
}

enum MidiState {
    NotConnected { last_checked: Option<Instant> },
    Connected(MidiReader),
}

pub struct DissonanceLabApp {
    audio: Arc<Mutex<AudioState>>,
    piano_gui: PianoGui,
    midi: MidiState,
    midi_to_audio_tx: Arc<Mutex<Option<MidiSender>>>,
    midi_to_piano_gui_rx: channel::Receiver<wmidi::MidiMessage<'static>>,
    midi_to_piano_gui_tx: channel::Sender<wmidi::MidiMessage<'static>>,
}

impl Default for DissonanceLabApp {
    fn default() -> Self {
        let (midi_to_piano_gui_tx, midi_to_piano_gui_rx) = channel::unbounded();
        Self {
            audio: Arc::new(Mutex::new(AudioState::Uninitialized)),
            piano_gui: PianoGui::new(),
            midi: MidiState::NotConnected { last_checked: None },
            midi_to_audio_tx: Arc::new(Mutex::new(None)),
            midi_to_piano_gui_rx,
            midi_to_piano_gui_tx,
        }
    }
}

impl DissonanceLabApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup custom theme instead of default dark theme
        theme::setup_custom_theme(&cc.egui_ctx);
        Default::default()
    }

    fn setup_audio(&mut self) {
        assert!(matches!(
            *self.audio.lock(),
            AudioState::Muted | AudioState::Uninitialized
        ));
        *self.audio.lock() =
            AudioState::Playing(WebAudio::new(include_str!("audio_worklet_processor.js")));
        // let (tx, rx) = crossbeam::channel::unbounded();
        // let synth = Box::new(PianoSynth::new(rx));
        // // let audio = AudioManager::new(synth, |message| {
        // //     warn!("{message}");
        // // });
        // self.audio = AudioState::Setup(Audio {
        //     tx: tx.clone(),
        //     //_audio: audio,
        // });
        // *self.midi_to_audio_tx.lock() = Some(tx);
    }

    fn ensure_midi(&mut self, ctx: &egui::Context) {
        const MIDI_CHECK_PERIOD: Duration = Duration::from_secs(1);
        match &mut self.midi {
            MidiState::NotConnected { last_checked }
                if last_checked.is_none()
                    || last_checked.map(|t| t.elapsed()) > Some(MIDI_CHECK_PERIOD) =>
            {
                // let to_synth_tx = self.midi_to_audio_tx.clone();
                let to_gui_tx = self.midi_to_piano_gui_tx.clone();
                let ctx = ctx.clone();
                let audio = self.audio.clone();
                match MidiReader::new(move |message| {
                    if let AudioState::Playing(web_audio) = &*audio.lock() {
                        match message {
                            wmidi::MidiMessage::NoteOff(_, note, _) => {
                                web_audio.send_message(webaudio::Message::NoteOff {
                                    note: u8::from(*note),
                                });
                            }
                            wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                                web_audio.send_message(webaudio::Message::NoteOn {
                                    note: u8::from(*note),
                                    velocity: u8::from(*velocity),
                                });
                            }
                            _ => {}
                        }
                    }
                    // if let Some(tx) = &*to_synth_tx.lock() {
                    //     let _ = tx.try_send(message.to_owned());
                    // }
                    let _ = to_gui_tx.try_send(message.to_owned());
                    ctx.request_repaint();
                }) {
                    Ok(reader) => {
                        self.midi = MidiState::Connected(reader);
                    }
                    Err(e) => {
                        match e {
                            crate::midi::Error::NoMidiInterface | crate::midi::Error::Init(_) => {}
                            crate::midi::Error::Connect(_) | crate::midi::Error::PortInfo(_) => {
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

impl eframe::App for DissonanceLabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_midi(ctx);
        // don't need to start muted if in native mode
        #[cfg(not(target_arch = "wasm32"))]
        if let AudioState::Uninitialized = self.audio {
            self.setup_audio();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                const STATUS_HEIGHT: f32 = 40.0;
                ui.allocate_ui(
                    vec2(PIANO_WIDTH.min(ui.available_width()), STATUS_HEIGHT),
                    |ui| {
                        const MUTE_FONT_SIZE: f32 = 16.0;
                        const STATUS_FONT_SIZE: f32 = 14.0;
                        ui.horizontal(|ui| {
                            let playing = match *self.audio.lock() {
                                AudioState::Playing(_) => true,
                                AudioState::Uninitialized | AudioState::Muted => false,
                            };
                            if playing {
                                if ui
                                    .button(RichText::new("🔈").size(MUTE_FONT_SIZE))
                                    .clicked()
                                {
                                    *self.audio.lock() = AudioState::Muted;
                                }
                            } else {
                                #[allow(clippy::collapsible_else_if)]
                                if ui
                                    .button(
                                        RichText::new("🔇")
                                            .size(MUTE_FONT_SIZE)
                                            .color(theme::ATTENTION_TEXT),
                                    )
                                    .clicked()
                                {
                                    self.setup_audio();
                                }
                            }

                            ui.label("|");
                            const MIDI_TEXT: &str = "MIDI";
                            const MIDI_FONT: FontId = FontId::proportional(STATUS_FONT_SIZE);
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
                                MidiState::NotConnected { .. } => "not connected".to_string(),
                                MidiState::Connected(midi_reader) => {
                                    midi_reader.get_name().to_string()
                                }
                            });
                        });
                        ui.painter().text(
                            ui.max_rect().center_bottom(),
                            Align2::CENTER_BOTTOM,
                            "dissonance lab",
                            FontId::proportional(STATUS_FONT_SIZE),
                            theme::KEYBOARD_LABEL,
                        );
                        if self.piano_gui.pressed_keys().count_ones() <= 1 {
                            ui.painter().text(
                                ui.max_rect().right_bottom(),
                                Align2::RIGHT_BOTTOM,
                                "shift for multi select",
                                FontId::proportional(STATUS_FONT_SIZE),
                                theme::TEXT_TERTIARY,
                            );
                        } else {
                            ui.painter().text(
                                ui.max_rect().right_bottom(),
                                Align2::RIGHT_BOTTOM,
                                self.piano_gui.selected_chord_name().unwrap(),
                                FontId::monospace(STATUS_FONT_SIZE),
                                ui.visuals().text_color(),
                            );
                        }
                    },
                );
                for message in self.midi_to_piano_gui_rx.try_iter() {
                    match message {
                        wmidi::MidiMessage::NoteOff(_channel, note, _) => {
                            self.piano_gui.external_note_off(note);
                        }
                        wmidi::MidiMessage::NoteOn(_channel, note, _) => {
                            self.piano_gui.external_note_on(note);
                        }
                        _ => {}
                    }
                }
                match interval_display::show(&mut self.piano_gui, ui) {
                    None => {}
                    Some(piano_gui::Action::Pressed(note)) => {
                        if let AudioState::Playing(web_audio) = &*self.audio.lock() {
                            web_audio.send_message(webaudio::Message::NoteOn {
                                note: u8::from(note),
                                velocity: u8::from(64),
                            });
                        }
                    }
                    Some(piano_gui::Action::Released(note)) => {
                        if let AudioState::Playing(web_audio) = &*self.audio.lock() {
                            web_audio.send_message(webaudio::Message::NoteOff {
                                note: u8::from(note),
                            });
                        }
                    }
                }
            });
        });
        const REPAINT_PERIOD: Duration = Duration::from_secs(2);
        ctx.request_repaint_after(REPAINT_PERIOD);
    }
}
