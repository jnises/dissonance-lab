use egui::{Align, Frame, Label, Layout, RichText, Sense, Stroke, Ui, UiBuilder, vec2};
use log::error;
use shared_types::Tuning;
use std::sync::mpsc;
use web_time::{Duration, Instant};
use wmidi::Note;

use crate::{
    control_icon::{CONTROL_HEIGHT, Icon},
    interval_display,
    midi::MidiReader,
    piano_gui::{PIANO_WIDTH, PianoGui},
    piano_state::Action,
    piano_types::Semitone,
    theme,
    webaudio::{ToWorkletMessage, WebAudio},
};

const MIDI_SUSTAIN_CONTROL: u8 = 64;
const MIDI_SUSTAIN_ACTIVE_THRESHOLD: u8 = 64;
const CONTROL_FONT_SIZE: f32 = 12.0;
const DEFAULT_NOTE_VELOCITY: u8 = 64;

enum AudioState {
    Uninitialized,
    Loading(WebAudio),
    Playing { audio: WebAudio, muted: bool },
    Disabled, // Audio is not supported (e.g., mobile devices without AudioWorklet)
}

enum MidiState {
    NotConnected { last_checked: Option<Instant> },
    Connected(MidiReader),
}

pub struct DissonanceLabApp {
    audio: AudioState,
    piano_gui: PianoGui,
    midi: MidiState,
    midi_rx: mpsc::Receiver<wmidi::MidiMessage<'static>>,
    midi_tx: mpsc::Sender<wmidi::MidiMessage<'static>>,
    just_tuning: bool,
    invert_sustain_pedal: bool,
    // Whether the user has explicitly attempted to enable audio (clicked the button)
    user_audio_attempted: bool,
}

impl Default for DissonanceLabApp {
    fn default() -> Self {
        let (midi_tx, midi_rx) = mpsc::channel();
        Self {
            audio: AudioState::Uninitialized,
            piano_gui: PianoGui::new(),
            midi: MidiState::NotConnected { last_checked: None },
            midi_rx,
            midi_tx,
            just_tuning: false,
            invert_sustain_pedal: false,
            user_audio_attempted: false,
        }
    }
}

impl DissonanceLabApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            _ = cc;
            panic!("This application only supports WebAssembly target architecture");
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Setup custom theme instead of default dark theme
            theme::setup_custom_theme(&cc.egui_ctx);

            let mut app = Self::default();
            // Load sustain pedal polarity setting from local storage
            app.load_sustain_pedal_setting(cc);
            // Try to eagerly initialize audio once at startup in case the browser allows it without user gesture.
            // Some browsers (notably Safari / iOS) will reject or suspend AudioContext creation until a user gesture.
            // If initialization ultimately fails we will revert the state back to Uninitialized so the user can click the audio enable/unmute button in the UI.
            app.try_startup_audio_once();
            app
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_sustain_pedal_setting(&mut self, cc: &eframe::CreationContext<'_>) {
        if let Some(storage) = cc.storage
            && let Some(invert_sustain) = storage.get_string("invert_sustain_pedal")
        {
            self.invert_sustain_pedal = invert_sustain == "true";
        }
    }

    fn save_sustain_pedal_setting(&self, frame: &mut eframe::Frame) {
        if let Some(storage) = frame.storage_mut() {
            storage.set_string(
                "invert_sustain_pedal",
                self.invert_sustain_pedal.to_string(),
            );
        }
    }

    fn setup_audio(&mut self) {
        assert!(matches!(self.audio, AudioState::Uninitialized));
        let web_audio = WebAudio::new();
        self.audio = AudioState::Loading(web_audio);
        self.user_audio_attempted = true;
    }

    /// Attempt to start audio during startup (no user gesture yet).
    /// If the worklet later reports a disable state, revert to Uninitialized so the user can try again manually.
    #[cfg(target_arch = "wasm32")]
    fn try_startup_audio_once(&mut self) {
        // Only attempt if currently uninitialized – don't override an explicit user mute choice.
        if matches!(self.audio, AudioState::Uninitialized) {
            self.audio = AudioState::Loading(WebAudio::new());
        }
    }

    fn check_audio_status(&mut self) {
        let (web_audio, loading) = match &self.audio {
            AudioState::Loading(web_audio) => (web_audio, true),
            AudioState::Playing { audio, .. } => (audio, false),
            _ => return,
        };
        if web_audio.is_disabled() {
            if !self.user_audio_attempted {
                // Automatic attempt failed / unsupported. Revert to Uninitialized so user can try enabling manually.
                self.audio = AudioState::Uninitialized;
            } else {
                // User explicitly tried and it still failed; mark Disabled so we can show more explicit UI.
                self.audio = AudioState::Disabled;
            }
        } else if loading && web_audio.is_ready() {
            // Pedal events may have arrived before this worklet was ready to receive them.
            web_audio.send_message(ToWorkletMessage::SustainPedal {
                active: self.piano_gui.is_sustain_active(),
            });
            if let Some(reference) = self.piano_gui.tuning_reference() {
                for (note, sustained) in self.piano_gui.sounding_notes() {
                    // Only held state survives startup. Recreate it at the GUI's default velocity.
                    web_audio.send_message(self.note_on(note, reference, DEFAULT_NOTE_VELOCITY));
                    if sustained {
                        web_audio.send_message(ToWorkletMessage::NoteOff { note: note.into() });
                    }
                }
            }
            let AudioState::Loading(web_audio) =
                std::mem::replace(&mut self.audio, AudioState::Uninitialized)
            else {
                unreachable!();
            };
            self.audio = AudioState::Playing {
                audio: web_audio,
                muted: false,
            };
        }
    }

    fn ensure_midi(&mut self, ctx: &egui::Context) {
        const MIDI_CHECK_PERIOD: Duration = Duration::from_secs(1);
        match &mut self.midi {
            MidiState::NotConnected { last_checked }
                if last_checked.is_none()
                    || last_checked.map(|t| t.elapsed()) > Some(MIDI_CHECK_PERIOD) =>
            {
                let midi_tx = self.midi_tx.clone();
                let ctx = ctx.clone();
                match MidiReader::new(move |message| {
                    // The receiver can disappear when the app closes.
                    if midi_tx.send(message.to_owned()).is_ok() {
                        ctx.request_repaint();
                    }
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

    fn send_audio_message(&self, message: ToWorkletMessage) {
        if let AudioState::Playing { audio, muted } = &self.audio {
            if !muted {
                audio.ensure_running();
            }
            audio.send_message(message);
        }
    }

    fn tuning(&self, reference: Option<Note>) -> Tuning {
        match (self.just_tuning, reference) {
            (true, Some(reference)) => Tuning::Just {
                reference: u8::from(reference),
            },
            _ => Tuning::Equal,
        }
    }

    fn set_just_tuning(&mut self, just: bool) {
        if self.just_tuning != just {
            self.just_tuning = just;
            self.send_audio_message(ToWorkletMessage::Retune {
                tuning: self.tuning(self.piano_gui.tuning_reference()),
            });
        }
    }

    fn note_on(&self, note: Note, reference: Note, velocity: u8) -> ToWorkletMessage {
        let note = u8::from(note);
        ToWorkletMessage::NoteOn {
            note,
            velocity,
            frequency: Some(self.tuning(Some(reference)).frequency(note)),
        }
    }

    fn send_piano_actions(&self, actions: Vec<Action>) {
        for action in actions {
            let message = match action {
                Action::Pressed { note, reference } => {
                    self.note_on(note, reference, DEFAULT_NOTE_VELOCITY)
                }
                Action::Released(note) => ToWorkletMessage::NoteOff {
                    note: u8::from(note),
                },
                Action::SustainPedal(active) => ToWorkletMessage::SustainPedal { active },
            };
            self.send_audio_message(message);
        }
    }

    fn process_midi(&mut self) {
        use wmidi::MidiMessage;

        let mut sustain_actions = Vec::new();
        // Notes and pedal changes must reach the synth in the order received.
        for message in self.midi_rx.try_iter() {
            let message = match message {
                MidiMessage::NoteOff(_, note, _) | MidiMessage::NoteOn(_, note, wmidi::U7::MIN) => {
                    self.piano_gui.external_note_off(note);
                    ToWorkletMessage::NoteOff {
                        note: u8::from(note),
                    }
                }
                MidiMessage::NoteOn(_, note, velocity) => {
                    let reference = self.piano_gui.external_note_on(note);
                    self.note_on(note, reference, u8::from(velocity))
                }
                MidiMessage::ControlChange(_, control, value)
                    if u8::from(control) == MIDI_SUSTAIN_CONTROL =>
                {
                    let active = u8::from(value) >= MIDI_SUSTAIN_ACTIVE_THRESHOLD;
                    self.piano_gui.set_external_sustain(
                        active ^ self.invert_sustain_pedal,
                        &mut sustain_actions,
                    );
                    for action in sustain_actions.drain(..) {
                        let Action::SustainPedal(active) = action else {
                            unreachable!("Unexpected external sustain action: {action:?}");
                        };
                        self.send_audio_message(ToWorkletMessage::SustainPedal { active });
                    }
                    continue;
                }
                _ => continue,
            };
            self.send_audio_message(message);
        }
    }

    fn show_speaker(&mut self, ui: &mut Ui) {
        let (muted, hint) = match &self.audio {
            AudioState::Playing {
                audio,
                muted: false,
            } if audio.is_running() => (false, "Mute sound"),
            AudioState::Uninitialized | AudioState::Playing { .. } => (true, "Enable sound"),
            AudioState::Loading(_) => (true, "Loading sound"),
            AudioState::Disabled => (true, "Audio unavailable"),
        };
        let enabled = !matches!(self.audio, AudioState::Disabled | AudioState::Loading(_));
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                Icon::Speaker { muted }.button(ui, hint, None, None)
            })
            .inner
            .on_hover_text(hint)
            .on_disabled_hover_text(hint);
        if response.clicked() {
            match &mut self.audio {
                AudioState::Playing {
                    audio,
                    muted: current,
                } => {
                    let mute = !muted;
                    match audio.set_muted(mute) {
                        Ok(()) => *current = mute,
                        Err(error) => log::error!("Could not change output gain: {error:?}"),
                    }
                    if !mute {
                        audio.ensure_running();
                    }
                }
                AudioState::Uninitialized => self.setup_audio(),
                AudioState::Disabled | AudioState::Loading(_) => unreachable!(),
            }
        }
    }

    fn show_footer(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let width = PIANO_WIDTH.min(ui.available_width());
        ui.allocate_ui_with_layout(
            vec2(width, CONTROL_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_min_size(vec2(width, CONTROL_HEIGHT));
                self.show_speaker(ui);

                let midi_name = match &self.midi {
                    MidiState::Connected(reader) => Some(reader.get_name()),
                    MidiState::NotConnected { .. } => None,
                };
                let midi_color = if midi_name.is_some() {
                    theme::outlines()
                } else {
                    theme::TEXT_TERTIARY
                };
                ui.label(
                    RichText::new("MIDI")
                        .size(CONTROL_FONT_SIZE)
                        .color(midi_color),
                )
                .on_hover_text(midi_name.unwrap_or("MIDI disconnected"));
                if midi_name.is_some() {
                    let polarity = if self.invert_sustain_pedal {
                        "Sustain pedal · inverted"
                    } else {
                        "Sustain pedal · normal"
                    };
                    if (Icon::Polarity {
                        inverted: self.invert_sustain_pedal,
                    })
                    .button(
                        ui,
                        "Invert sustain pedal",
                        None,
                        Some(self.invert_sustain_pedal),
                    )
                    .on_hover_text(polarity)
                    .clicked()
                    {
                        self.invert_sustain_pedal = !self.invert_sustain_pedal;
                        self.save_sustain_pedal_setting(frame);
                    }
                }

                if self.piano_gui.held_keys().count_ones() > 1
                    && let Some(chord) = self.piano_gui.selected_chord_name()
                {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add(
                            Label::new(RichText::new(&chord).monospace().size(CONTROL_FONT_SIZE))
                                .truncate(),
                        )
                        .on_hover_text(chord);
                    });
                }
            },
        );
    }

    fn show_tuning_strip(&mut self, ui: &mut Ui, width: f32) {
        // Keep the piano and interval rows independent of hover and selection paint.
        let (rect, _) = ui.allocate_exact_size(vec2(width, CONTROL_HEIGHT), Sense::hover());
        let mut strip = ui.new_child(
            UiBuilder::new()
                .id_salt("tuning_strip")
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        self.show_tuning_choices(&mut strip);
        strip.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let held = self.piano_gui.hold_active();
            let hint = if held {
                "Release held notes"
            } else {
                "Hold notes after key release"
            };
            if (Icon::Hold {
                active: self.piano_gui.is_sustain_active(),
            })
            .button(ui, "Hold notes", Some("Hold"), Some(held))
            .on_hover_text(hint)
            .clicked()
            {
                let mut actions = Vec::new();
                self.piano_gui.set_hold(!held, &mut actions);
                self.send_piano_actions(actions);
                ui.ctx().request_repaint();
            }

            let shift = ui.input(|input| input.modifiers.shift);
            let outline = theme::outlines();
            let fill = if shift {
                outline.gamma_multiply(0.12)
            } else {
                egui::Color32::TRANSPARENT
            };
            Frame::new()
                .corner_radius(3.0)
                .inner_margin(egui::Margin::symmetric(4, 1))
                .stroke(Stroke::new(1.0, outline.gamma_multiply(0.4)))
                .fill(fill)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Shift")
                            .monospace()
                            .size(CONTROL_FONT_SIZE)
                            .color(if shift {
                                outline
                            } else {
                                theme::KEYBOARD_LABEL
                            }),
                    );
                })
                .response
                .on_hover_text("Hold Shift for momentary sustain");
        });
    }

    fn show_tuning_choices(&mut self, ui: &mut Ui) {
        for (just, icon, hint) in [
            (
                false,
                Icon::EqualTuning,
                "Equal temperament · 12 equal steps",
            ),
            (
                true,
                Icon::JustTuning,
                "Just intervals · ratios from the reference note",
            ),
        ] {
            if icon
                .button(ui, hint, None, Some(self.just_tuning == just))
                .on_hover_text(hint)
                .clicked()
            {
                self.set_just_tuning(just);
                ui.ctx().request_repaint();
            }
        }

        let reference = self.piano_gui.tuning_reference();
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            Icon::Reference.indicator(ui, reference.is_some());
            if let Some(note) = reference {
                const SEMITONES_PER_OCTAVE: u8 = 12;
                let midi = u8::from(note);
                let name = Semitone::new(midi % SEMITONES_PER_OCTAVE).name();
                let octave = i16::from(midi / SEMITONES_PER_OCTAVE) - 1;
                ui.label(
                    RichText::new(format!("{name}{octave}"))
                        .monospace()
                        .size(CONTROL_FONT_SIZE),
                );
            }
        })
        .response
        .on_hover_text(if reference.is_some() {
            "Tuning reference · fixed until all notes are released"
        } else {
            "Play a note to set the tuning reference"
        });
    }
}

impl eframe::App for DissonanceLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Ensure dark mode remains forced, reapply custom theme if needed
        if !ui.visuals().dark_mode {
            theme::setup_custom_theme(&ctx);
        }

        self.ensure_midi(&ctx);
        self.check_audio_status();
        self.process_midi();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                self.show_footer(ui, frame);

                let (actions, piano_rect) = self.piano_gui.show(ui);
                self.send_piano_actions(actions);
                self.show_tuning_strip(ui, piano_rect.width());
                interval_display::show(&self.piano_gui, ui, piano_rect, self.just_tuning);
            });
        });
        const REPAINT_PERIOD: Duration = Duration::from_millis(500); // 2 times per second
        ctx.request_repaint_after(REPAINT_PERIOD);
    }
}
