use std::sync::{Arc, LazyLock};

use colorgrad::{BlendMode, Gradient as _};
use egui::{
    Color32, Pos2, Rect, Sense, ThemePreference, Vec2,
    epaint::{PathShape, PathStroke},
    pos2,
};
use log::{info, warn};

use crate::{audio::AudioManager, piano_gui::{self, PianoGui}, synth::PianoSynth, theory::is_key_black};

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
    pressed: Option<usize>,
    audio: AudioState,
    piano_gui: PianoGui,
}

impl Default for TheoryApp {
    fn default() -> Self {
        Self {
            pressed: None,
            audio: AudioState::Uninitialized,
            piano_gui: PianoGui::new(),
        }
    }
}

impl TheoryApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(ThemePreference::Dark);
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
                const KEY_SIZE: Vec2 = Vec2::new(50f32, 140f32);

                // Add a frame around the piano
                egui::Frame::group(ui.style())
                    .fill(oklab(0.5, 0.0, 0.0, 1.0))
                    .stroke(egui::Stroke::new(
                        2.0,
                        ui.visuals().widgets.noninteractive.fg_stroke.color,
                    ))
                    .show(ui, |ui| {
                        match self.piano_gui.draw(ui, ctx) {
                            None => {},
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

                        // TODO: cache this
                        // let (min, max) = self.piano_gui.get_bounding_box();
                        // //let piano_rect = Rect::from_min_max(min.into(), max.into());
                        // let (piano_id, piano_rect) = ui.allocate_space(max.into());
                        // let r = ui.interact(piano_rect, piano_id, Sense::click());
                        // let painter = ui.painter_at(piano_rect);
                        // for key in &self.piano_gui.keys {
                        //     painter.add(PathShape::closed_line(
                        //         key.shape
                        //             .points
                        //             .iter()
                        //             .map(|[x, y]| pos2(*x, *y) + piano_rect.min.to_vec2())
                        //             .collect(),
                        //         PathStroke::new(2f32, match key.key_type {
                        //             piano_gui::KeyType::White => Color32::WHITE,
                        //             piano_gui::KeyType::Black => Color32::BLACK,
                        //         }),
                        //     ));
                        // }
                        // ui.horizontal(|ui| {
                        //     for note in 0..12 {
                        //         if ui.available_width() <= 0f32 {
                        //             break;
                        //         }
                        //         // Calculate semitone difference (if any pressed note exists)
                        //         let semi_diff_from_pressed = self.pressed.map(|pressed_note| {
                        //             // Use rem_euclid which properly handles negative numbers
                        //             // and always returns a positive remainder
                        //             u8::try_from((note as i32 - pressed_note as i32).rem_euclid(12))
                        //                 .unwrap()
                        //         });

                        //         let diff_interval = semi_diff_from_pressed.map(|diff| {
                        //             crate::theory::Interval::from_semitone_interval(diff)
                        //         });

                        //         let just_interval = diff_interval.map(|diff| diff.just_ratio());

                        //         let cent_error =
                        //             diff_interval.map(|diff| diff.just_tempered_error_cents());

                        //         // Use this value later if needed for display or logic
                        //         let this_pressed = Some(note) == self.pressed;
                        //         let (key_id, key_rect) = ui.allocate_space(KEY_SIZE);

                        //         let interact = ui.interact(key_rect, key_id, Sense::click());
                        //         let painter = ui.painter();
                        //         painter.rect(
                        //             key_rect,
                        //             5f32,
                        //             if this_pressed {
                        //                 ui.style().visuals.selection.bg_fill
                        //             } else {
                        //                 egui::Color32::TRANSPARENT
                        //             },
                        //             egui::Stroke::new(
                        //                 4f32,
                        //                 if is_key_black(note) {
                        //                     egui::Color32::BLACK
                        //                 } else {
                        //                     egui::Color32::WHITE
                        //                 },
                        //             ),
                        //             egui::StrokeKind::Middle,
                        //         );

                        //         if let (true, Some(just), Some(cents)) =
                        //             (!this_pressed, just_interval, cent_error)
                        //         {
                        //             static DENOMINATOR_GRADIENT: LazyLock<
                        //                 colorgrad::BasisGradient,
                        //             > = LazyLock::new(|| {
                        //                 colorgrad::GradientBuilder::new()
                        //                     .colors(&[
                        //                         colorgrad::Color::from_oklaba(1.0, 0.0, 0.0, 1.0),
                        //                         colorgrad::Color::from_oklaba(0.8, 0.0, 0.25, 1.0),
                        //                         colorgrad::Color::from_oklaba(
                        //                             0.8, 0.217, 0.125, 1.0,
                        //                         ),
                        //                     ])
                        //                     .domain(&[2.0, 5.0, 20.0])
                        //                     .mode(BlendMode::Oklab)
                        //                     .build()
                        //                     .unwrap()
                        //             });
                        //             // Draw the just ratio
                        //             painter.text(
                        //                 key_rect.center_top() + Vec2::new(0.0, 50.0),
                        //                 egui::Align2::CENTER_CENTER,
                        //                 format!("{:.2}", just),
                        //                 egui::FontId::default(),
                        //                 colorgrad_to_egui(
                        //                     DENOMINATOR_GRADIENT.at(*just.denom() as f32),
                        //                 ),
                        //             );

                        //             static CENT_ERROR_GRADIENT: LazyLock<colorgrad::BasisGradient> =
                        //                 LazyLock::new(|| {
                        //                     colorgrad::GradientBuilder::new()
                        //                         .colors(&[
                        //                             colorgrad::Color::from_oklaba(
                        //                                 1.0, 0.0, 0.0, 1.0,
                        //                             ),
                        //                             colorgrad::Color::from_oklaba(
                        //                                 0.8, 0.0, 0.25, 1.0,
                        //                             ),
                        //                             colorgrad::Color::from_oklaba(
                        //                                 0.8, 0.217, 0.125, 1.0,
                        //                             ),
                        //                         ])
                        //                         .domain(&[5.0, 10.0, 20.0])
                        //                         .mode(BlendMode::Oklab)
                        //                         .build()
                        //                         .unwrap()
                        //                 });

                        //             // Draw the cents error
                        //             painter.text(
                        //                 key_rect.center_top() + Vec2::new(0.0, 80.0),
                        //                 egui::Align2::CENTER_CENTER,
                        //                 format!("{:.1}¢", cents),
                        //                 egui::FontId::default(),
                        //                 {
                        //                     // Get color based on absolute cent error value
                        //                     let abs_cents = cents.abs();
                        //                     let color = CENT_ERROR_GRADIENT.at(abs_cents);
                        //                     colorgrad_to_egui(color)
                        //                 },
                        //             );
                        //         }

                        //         if interact.is_pointer_button_down_on()
                        //             && self.pressed != Some(note)
                        //         {
                        //             self.pressed = Some(note);
                        //             if matches!(self.audio, AudioState::Uninitialized) {
                        //                 self.setup_audio();
                        //             }
                        //             if let AudioState::Setup(audio) = &self.audio {
                        //                 audio
                        //                     .tx
                        //                     .send(wmidi::MidiMessage::NoteOn(
                        //                         wmidi::Channel::Ch1,
                        //                         wmidi::Note::C4
                        //                             .step(i8::try_from(note).unwrap())
                        //                             .unwrap(),
                        //                         wmidi::Velocity::MAX,
                        //                     ))
                        //                     .unwrap();
                        //             }
                        //         }

                        //         // Check for button release
                        //         if interact.drag_stopped()
                        //             || (interact.hovered()
                        //                 && ctx.input(|i| i.pointer.any_released()))
                        //         {
                        //             // This detects when we release the button while hovering over this element
                        //             if Some(note) == self.pressed {
                        //                 if let AudioState::Setup(audio) = &self.audio {
                        //                     audio
                        //                         .tx
                        //                         .send(wmidi::MidiMessage::NoteOff(
                        //                             wmidi::Channel::Ch1,
                        //                             wmidi::Note::C4
                        //                                 .step(i8::try_from(note).unwrap())
                        //                                 .unwrap(),
                        //                             wmidi::Velocity::MAX,
                        //                         ))
                        //                         .unwrap();
                        //                 }
                        //             }
                        //         }
                        //     }
                        // });
                    });
            });
        });
    }
}

/// Convert a color from colorgrad to egui's Color32
fn colorgrad_to_egui(color: colorgrad::Color) -> egui::Color32 {
    let [r, g, b, a] = color.to_rgba8();
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> egui::Color32 {
    colorgrad_to_egui(colorgrad::Color::from_oklaba(l, a, b, alpha))
}
