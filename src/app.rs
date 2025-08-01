use crossbeam::channel;
use egui::{Align, Align2, Color32, FontId, Layout, RichText, pos2, vec2};
use log::error;
use std::sync::{Arc, Mutex};
use web_time::{Duration, Instant};

use crate::{
    interval_display,
    midi::MidiReader,
    piano_gui::{self, PIANO_WIDTH, PianoGui},
    theme,
    webaudio::{ToWorkletMessage, WebAudio},
};

/// Width threshold for determining mobile/narrow screens
const MOBILE_BREAKPOINT_WIDTH: f32 = 480.0;

enum AudioState {
    Uninitialized,
    Muted,
    Playing(WebAudio),
    Disabled, // Audio is not supported (e.g., mobile devices without AudioWorklet)
}

enum MidiState {
    NotConnected { last_checked: Option<Instant> },
    Connected(MidiReader),
}

pub struct DissonanceLabApp {
    audio: Arc<Mutex<AudioState>>,
    piano_gui: PianoGui,
    midi: MidiState,
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
            midi_to_piano_gui_rx,
            midi_to_piano_gui_tx,
        }
    }
}

impl DissonanceLabApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        assert!(
            cfg!(target_arch = "wasm32"),
            "This application only supports WebAssembly target architecture"
        );

        // Setup custom theme instead of default dark theme
        theme::setup_custom_theme(&cc.egui_ctx);
        Default::default()
    }

    fn setup_audio(&mut self) {
        assert!(matches!(
            *self.audio.lock().unwrap(),
            AudioState::Muted | AudioState::Uninitialized
        ));
        let web_audio = WebAudio::new();
        *self.audio.lock().unwrap() = AudioState::Playing(web_audio);
    }

    /// Check if the current audio state indicates failure and update to Disabled if so
    fn check_audio_status(&mut self) {
        let mut audio_guard = self.audio.lock().unwrap();
        if let AudioState::Playing(web_audio) = &*audio_guard {
            if web_audio.is_disabled() {
                *audio_guard = AudioState::Disabled;
            }
        }
    }

    fn ensure_midi(&mut self, ctx: &egui::Context) {
        const MIDI_CHECK_PERIOD: Duration = Duration::from_secs(1);
        match &mut self.midi {
            MidiState::NotConnected { last_checked }
                if last_checked.is_none()
                    || last_checked.map(|t| t.elapsed()) > Some(MIDI_CHECK_PERIOD) =>
            {
                let to_gui_tx = self.midi_to_piano_gui_tx.clone();
                let ctx = ctx.clone();
                let audio = self.audio.clone();
                match MidiReader::new(move |message| {
                    if let AudioState::Playing(web_audio) = &*audio.lock().unwrap() {
                        match message {
                            wmidi::MidiMessage::NoteOff(_, note, _) => {
                                web_audio.send_message(ToWorkletMessage::NoteOff {
                                    note: u8::from(*note),
                                });
                            }
                            wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                                web_audio.send_message(ToWorkletMessage::NoteOn {
                                    note: u8::from(*note),
                                    velocity: u8::from(*velocity),
                                });
                            }
                            _ => {}
                        }
                    }

                    to_gui_tx.send(message.to_owned()).unwrap();
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
        self.check_audio_status();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                const STATUS_HEIGHT: f32 = 40.0;
                ui.allocate_ui(
                    vec2(PIANO_WIDTH.min(ui.available_width()), STATUS_HEIGHT),
                    |ui| {
                        const MUTE_FONT_SIZE: f32 = 16.0;
                        const STATUS_FONT_SIZE: f32 = 14.0;
                        ui.horizontal(|ui| {
                            let (playing, disabled, uninitialized) = {
                                let audio_state = &*self.audio.lock().unwrap();
                                (
                                    matches!(audio_state, AudioState::Playing(_)),
                                    matches!(audio_state, AudioState::Disabled),
                                    matches!(audio_state, AudioState::Uninitialized),
                                )
                            };

                            if playing {
                                if ui
                                    .button(RichText::new("🔈").size(MUTE_FONT_SIZE))
                                    .clicked()
                                {
                                    *self.audio.lock().unwrap() = AudioState::Muted;
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
                                #[allow(clippy::collapsible_else_if)]
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
                        });
                        ui.painter().text(
                            ui.max_rect().center_bottom(),
                            Align2::CENTER_BOTTOM,
                            "dissonance lab",
                            FontId::proportional(STATUS_FONT_SIZE),
                            theme::KEYBOARD_LABEL,
                        );
                        if self.piano_gui.pressed_keys().count_ones() <= 1 {
                            // Hide "shift for multi select" label on narrow screens (mobile/phone)
                            if ui.available_width() >= MOBILE_BREAKPOINT_WIDTH {
                                ui.painter().text(
                                    ui.max_rect().right_bottom(),
                                    Align2::RIGHT_BOTTOM,
                                    "shift for multi select",
                                    FontId::proportional(STATUS_FONT_SIZE),
                                    theme::TEXT_TERTIARY,
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
                let actions = interval_display::show(&mut self.piano_gui, ui);
                for action in actions {
                    match action {
                        piano_gui::Action::Pressed(note) => {
                            if let AudioState::Playing(web_audio) = &*self.audio.lock().unwrap() {
                                web_audio.send_message(ToWorkletMessage::NoteOn {
                                    note: u8::from(note),
                                    velocity: 64,
                                });
                            }
                        }
                        piano_gui::Action::Released(note) => {
                            if let AudioState::Playing(web_audio) = &*self.audio.lock().unwrap() {
                                web_audio.send_message(ToWorkletMessage::NoteOff {
                                    note: u8::from(note),
                                });
                            }
                        }
                    }
                }
            });
        });
        const REPAINT_PERIOD: Duration = Duration::from_secs(2);
        ctx.request_repaint_after(REPAINT_PERIOD);
    }
}
