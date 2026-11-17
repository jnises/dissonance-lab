use egui::{Align, Align2, Color32, FontId, Layout, RichText, pos2, vec2};
use log::error;
use std::sync::mpsc;
use web_time::{Duration, Instant};

use crate::{
    interval_display,
    midi::MidiReader,
    piano_gui::{PIANO_WIDTH, PianoGui},
    piano_state::Action,
    theme,
    webaudio::{ToWorkletMessage, WebAudio},
};

/// Width threshold for determining mobile/narrow screens
const MOBILE_BREAKPOINT_WIDTH: f32 = 480.0;
const MIDI_SUSTAIN_CONTROL: u8 = 64;
const MIDI_SUSTAIN_ACTIVE_THRESHOLD: u8 = 64;

enum AudioState {
    Uninitialized,
    Muted,
    Loading(WebAudio),
    Playing(WebAudio),
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
        assert!(matches!(
            self.audio,
            AudioState::Muted | AudioState::Uninitialized
        ));
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
            AudioState::Playing(web_audio) => (web_audio, false),
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
            let AudioState::Loading(web_audio) =
                std::mem::replace(&mut self.audio, AudioState::Uninitialized)
            else {
                unreachable!();
            };
            self.audio = AudioState::Playing(web_audio);
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
        if let AudioState::Playing(web_audio) = &self.audio {
            web_audio.ensure_running();
            web_audio.send_message(message);
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
                    self.piano_gui.external_note_on(note);
                    ToWorkletMessage::NoteOn {
                        note: u8::from(note),
                        velocity: u8::from(velocity),
                    }
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
                const STATUS_HEIGHT: f32 = 40.0;
                ui.allocate_ui(
                    vec2(PIANO_WIDTH.min(ui.available_width()), STATUS_HEIGHT),
                    |ui| {
                        const MUTE_FONT_SIZE: f32 = 16.0;
                        const STATUS_FONT_SIZE: f32 = 14.0;
                        ui.horizontal(|ui| {
                            let (playing, disabled, uninitialized) = {
                                let audio_state = &self.audio;
                                (
                                    matches!(
                                        audio_state,
                                        AudioState::Playing(_) | AudioState::Loading(_)
                                    ),
                                    matches!(audio_state, AudioState::Disabled),
                                    matches!(audio_state, AudioState::Uninitialized),
                                )
                            };

                            if playing {
                                if ui
                                    .button(RichText::new("🔈").size(MUTE_FONT_SIZE))
                                    .clicked()
                                {
                                    self.audio = AudioState::Muted;
                                }
                            } else if disabled {
                                // Show disabled audio icon with explanatory text
                                let disabled_button = ui.button(
                                    RichText::new("🔇")
                                        .size(MUTE_FONT_SIZE)
                                        .color(ui.visuals().weak_text_color())
                                        .strikethrough(),
                                );
                                disabled_button
                                    .on_hover_text("Audio not supported on this device/browser");
                            } else {
                                let mute_button_response = ui.button(
                                    RichText::new("🔇")
                                        .size(MUTE_FONT_SIZE)
                                        .color(theme::ATTENTION_TEXT),
                                );

                                if mute_button_response.clicked() {
                                    self.setup_audio();
                                }

                                // Draw custom graphic hint: rotated text with arrow pointing to mute button
                                // Only show on wider screens to avoid clutter on mobile
                                if uninitialized && ui.available_width() >= MOBILE_BREAKPOINT_WIDTH
                                {
                                    // Constants for mute button hint styling
                                    const GAMMA_BLEND_FACTOR: f32 = 0.2;
                                    const FONT_SCALING_FACTOR: f32 = 0.8;
                                    const HALF_DIVISOR: f32 = 2.0;
                                    const TEXT_POSITION_OFFSET: f32 = 2.0;
                                    const TEXT_BOTTOM_OFFSET: f32 = 10.0;
                                    const ARROW_END_OFFSET_X: f32 = -1.0;
                                    const ARROW_END_OFFSET_Y: f32 = 6.0;
                                    const CONTROL_OFFSET_X: f32 = -4.0;
                                    const CONTROL_OFFSET_Y: f32 = 4.0;
                                    const STROKE_WIDTH: f32 = 1.5;
                                    const ARROW_TIP_OFFSET: f32 = 2.0;
                                    const ARROW_HEAD_WIDTH_FACTOR: f32 = 0.5;
                                    let button_rect = mute_button_response.rect;
                                    let painter = ui.painter();

                                    // Draw rotated text "click to enable audio"
                                    let text_color = theme::ATTENTION_TEXT.lerp_to_gamma(
                                        Color32::from_black_alpha(0),
                                        GAMMA_BLEND_FACTOR,
                                    );
                                    let text_size = STATUS_FONT_SIZE * FONT_SCALING_FACTOR;
                                    let font_id = FontId::proportional(text_size);
                                    let text = "click to enable audio";

                                    // Layout the text to get its dimensions
                                    let galley = painter.layout_no_wrap(
                                        text.to_string(),
                                        font_id,
                                        text_color,
                                    );

                                    let text_x = button_rect.left() - galley.size().y;
                                    let text_bottom_y = button_rect.top() - TEXT_BOTTOM_OFFSET;

                                    // Draw the text rotated 90 degrees ccw
                                    let text_pos = pos2(text_x, text_bottom_y);

                                    painter.add(egui::epaint::Shape::Text(
                                        egui::epaint::TextShape {
                                            pos: text_pos,
                                            galley: galley.clone(),
                                            underline: egui::Stroke::NONE,
                                            fallback_color: text_color,
                                            override_text_color: None,
                                            opacity_factor: 1.0,
                                            angle: -std::f32::consts::PI / 2.0, // 90 degrees ccw
                                        },
                                    ));

                                    // Draw curved arrow from end of text toward the mute button
                                    let arrow_start = text_pos
                                        + vec2(
                                            galley.size().y / HALF_DIVISOR + TEXT_POSITION_OFFSET,
                                            TEXT_POSITION_OFFSET,
                                        );
                                    let arrow_end = button_rect.left_top()
                                        + vec2(ARROW_END_OFFSET_X, ARROW_END_OFFSET_Y);

                                    // Create control points for cubic bezier curve
                                    let control = (arrow_start + arrow_end.to_vec2())
                                        / HALF_DIVISOR
                                        + vec2(CONTROL_OFFSET_X, CONTROL_OFFSET_Y);

                                    // Draw cubic bezier curve
                                    let stroke = egui::Stroke::new(STROKE_WIDTH, text_color);
                                    painter.add(egui::epaint::Shape::CubicBezier(
                                        egui::epaint::CubicBezierShape::from_points_stroke(
                                            [arrow_start, control, control, arrow_end],
                                            false,                      // not closed
                                            egui::Color32::TRANSPARENT, // no fill
                                            stroke,
                                        ),
                                    ));

                                    // Draw custom filled arrow head
                                    const ARROW_HEAD_SIZE: f32 = 6.0;
                                    // Calculate direction from control2 to arrow_end for proper arrow orientation
                                    let arrow_direction = (arrow_end - control).normalized();
                                    let arrow_perp = vec2(-arrow_direction.y, arrow_direction.x);

                                    let arrow_tip = arrow_end + arrow_direction * ARROW_TIP_OFFSET;
                                    let arrow_base1 = arrow_tip - arrow_direction * ARROW_HEAD_SIZE
                                        + arrow_perp * ARROW_HEAD_SIZE * ARROW_HEAD_WIDTH_FACTOR;
                                    let arrow_base2 = arrow_tip
                                        - arrow_direction * ARROW_HEAD_SIZE
                                        - arrow_perp * ARROW_HEAD_SIZE * ARROW_HEAD_WIDTH_FACTOR;

                                    painter.add(egui::epaint::Shape::convex_polygon(
                                        vec![arrow_tip, arrow_base1, arrow_base2],
                                        text_color,
                                        egui::Stroke::NONE,
                                    ));
                                }
                            }

                            ui.label("|");
                            let is_connected = matches!(&self.midi, MidiState::Connected(_));
                            let midi_text = if is_connected {
                                RichText::new("MIDI ☑")
                                    .size(STATUS_FONT_SIZE)
                                    .color(ui.visuals().text_color())
                            } else {
                                RichText::new("MIDI")
                                    .size(STATUS_FONT_SIZE)
                                    .color(ui.visuals().weak_text_color())
                                    .strikethrough()
                            };

                            let response = ui.label(midi_text);
                            response.on_hover_text(match &self.midi {
                                MidiState::NotConnected { .. } => "not connected".to_string(),
                                MidiState::Connected(midi_reader) => {
                                    midi_reader.get_name().to_string()
                                }
                            });

                            // Add discreet sustain pedal polarity toggle when MIDI is connected
                            if is_connected {
                                const TOGGLE_FONT_SIZE: f32 = 10.0;
                                let polarity_icon = if self.invert_sustain_pedal {
                                    "⤴" // Up-right arrow for inverted
                                } else {
                                    "⤵" // Down-right arrow for normal
                                };
                                let toggle_button = ui.small_button(
                                    RichText::new(polarity_icon)
                                        .size(TOGGLE_FONT_SIZE)
                                        .color(ui.visuals().weak_text_color()),
                                );
                                if toggle_button.clicked() {
                                    self.invert_sustain_pedal = !self.invert_sustain_pedal;
                                    self.save_sustain_pedal_setting(frame);
                                }
                                toggle_button.on_hover_text(if self.invert_sustain_pedal {
                                    "Sustain pedal: inverted (click to use normal polarity)"
                                } else {
                                    "Sustain pedal: normal (click to invert polarity)"
                                });
                            }
                        });
                        ui.painter().text(
                            ui.max_rect().center_bottom(),
                            Align2::CENTER_BOTTOM,
                            "dissonance lab",
                            FontId::proportional(STATUS_FONT_SIZE),
                            theme::KEYBOARD_LABEL,
                        );
                        if self.piano_gui.held_keys().count_ones() <= 1 {
                            // Hide sustain label on narrow screens (mobile/phone)
                            if ui.available_width() >= MOBILE_BREAKPOINT_WIDTH {
                                let sustain_active = self.piano_gui.is_sustain_active();
                                let label_color = if sustain_active {
                                    ui.visuals().text_color() // Active: normal text color
                                } else {
                                    theme::TEXT_TERTIARY // Inactive: dimmed color
                                };
                                ui.painter().text(
                                    ui.max_rect().right_bottom(),
                                    Align2::RIGHT_BOTTOM,
                                    "⬆ sustain",
                                    FontId::proportional(STATUS_FONT_SIZE),
                                    label_color,
                                );
                            }
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

                // Show piano GUI and process actions
                let actions = interval_display::show(&mut self.piano_gui, ui);
                for action in actions {
                    let message = match action {
                        Action::Pressed(note) => ToWorkletMessage::NoteOn {
                            note: u8::from(note),
                            velocity: 64,
                        },
                        Action::Released(note) => ToWorkletMessage::NoteOff {
                            note: u8::from(note),
                        },
                        Action::SustainPedal(active) => {
                            ctx.request_repaint();
                            ToWorkletMessage::SustainPedal { active }
                        }
                    };
                    self.send_audio_message(message);
                }
            });
        });
        const REPAINT_PERIOD: Duration = Duration::from_millis(500); // 2 times per second
        ctx.request_repaint_after(REPAINT_PERIOD);
    }
}
