use bitvec::{BitArr, order::Msb0};
use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::theme;

pub const PIANO_WIDTH: f32 = 600.0;
pub const PIANO_HEIGHT: f32 = 200.0;

pub type KeySet = BitArr!(for 12, in u16, Msb0);

pub struct PianoGui {
    selected_keys: KeySet,
}

impl PianoGui {
    pub fn new() -> Self {
        Self {
            selected_keys: Default::default(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> (Option<Action>, Rect) {
        let mut action = None;
        let mut piano_size = vec2(PIANO_WIDTH, PIANO_HEIGHT);
        if piano_size.x > ui.available_width() {
            piano_size *= (ui.available_width() / piano_size.x).max(0.5);
        }
        let (response, painter) =
            ui.allocate_painter(piano_size, Sense::empty());
        let rect = response.rect;
        painter.rect_filled(rect, 1.0, ui.visuals().panel_fill);
        const MARGIN: f32 = 2.0;
        let keys_rect = rect.shrink(MARGIN);
        let shift_pressed = ui.input(|i| i.modifiers.shift);
        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        #[derive(strum_macros::Display)]
        enum Color {
            White,
            Black,
        }
        for color in [Color::White, Color::Black] {
            let num_keys = match color {
                Color::White => NUM_WHITE_KEYS,
                Color::Black => NUM_BLACK_KEYS,
            };
            let x = match color {
                Color::White => vec![0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5],
                Color::Black => vec![1.0, 3.0, 6.0, 8.0, 10.0],
            };
            for key in 0..num_keys {
                let key_id = ui.id().with(format!("{color}{key}"));
                let key_size = match color {
                    Color::White => vec2(
                        (x.get(key + 1).unwrap_or(&12.0) - x[key]) / 12.0 * keys_rect.width(),
                        keys_rect.height(),
                    ),
                    Color::Black => vec2(keys_rect.width() / 12.0, keys_rect.height() * 0.6),
                };
                let key_rect = Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + x[key] / 12.0 * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                );
                let semitone = match color {
                    Color::White => white_key_to_semitone(key),
                    Color::Black => black_key_to_semitone(key),
                };
                let selected = self.selected_keys[semitone];
                let note = wmidi::Note::C4.step(semitone as i8).unwrap();
                painter.rect(
                    key_rect,
                    0.0,
                    if selected {
                        theme::selected_key()
                    } else {
                        ui.visuals().panel_fill
                    },
                    Stroke::new(2.0, theme::outlines()),
                    StrokeKind::Middle,
                );
                let key_response = ui.interact(key_rect, key_id, Sense::click());
                let mouse_pressed = ui.data(|r| r.get_temp::<bool>(key_id).unwrap_or(false));
                if key_response.is_pointer_button_down_on() && !mouse_pressed {
                    ui.data_mut(|r| r.insert_temp(key_id, true));
                    debug_assert!(action.is_none());
                    action = Some(Action::Pressed(note));
                    if !shift_pressed {
                        self.selected_keys.fill(false);
                    }
                    let key_selected = self.selected_keys[semitone];
                    self.selected_keys.set(semitone, !key_selected);
                } else if !key_response.is_pointer_button_down_on() && mouse_pressed {
                    ui.data_mut(|r| r.insert_temp(key_id, false));
                    debug_assert!(action.is_none());
                    action = Some(Action::Released(note));
                }
            }
        }
        (action, keys_rect)
    }

    pub fn selected_keys(&self) -> &KeySet {
        &self.selected_keys
    }

    pub fn selected_chord_name(&self) -> Option<String> {
        // AI generated. But seems mostly sensible
        let mut selected_semitones: Vec<usize> = self.selected_keys.iter_ones().collect();
        if selected_semitones.is_empty() {
            return None;
        }

        // Sort semitones to normalize chord representation
        selected_semitones.sort();

        // Try all rotations of the chord (all possible roots)
        for rotation in 0..selected_semitones.len() {
            let root_semitone = selected_semitones[rotation];
            let root = semitone_name(root_semitone);

            // Calculate intervals relative to this potential root
            let mut intervals: Vec<usize> = Vec::new();
            for &semitone in selected_semitones.iter() {
                if semitone != root_semitone {
                    intervals
                        .push((semitone as i32 - root_semitone as i32).rem_euclid(12) as usize);
                }
            }
            intervals.sort();

            // Check for standard chord types
            let chord_type = match (intervals.as_slice(), selected_semitones.len()) {
                ([4, 7], 3) => "maj",      // Major triad
                ([3, 7], 3) => "min",      // Minor triad
                ([3, 6], 3) => "dim",      // Diminished triad
                ([4, 8], 3) => "aug",      // Augmented triad
                ([4, 7, 11], 4) => "maj7", // Major seventh
                ([3, 7, 10], 4) => "min7", // Minor seventh
                ([4, 7, 10], 4) => "7",    // Dominant seventh
                ([3, 6, 9], 4) => "dim7",  // Diminished seventh
                ([3, 6, 10], 4) => "m7b5", // Half-diminished seventh
                _ => "",                   // Unknown chord type
            };

            if !chord_type.is_empty() {
                return Some(format!("{root}{chord_type}"));
            }
        }

        // If we can't identify the chord, list the notes
        if selected_semitones.len() == 1 {
            return Some(semitone_name(selected_semitones[0]).to_string());
        } else {
            let notes: Vec<String> = selected_semitones
                .iter()
                .map(|&semitone| semitone_name(semitone).to_string())
                .collect();
            Some(notes.join("/"))
        }
    }
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
}

fn white_key_to_semitone(key: usize) -> usize {
    match key {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 5,
        4 => 7,
        5 => 9,
        6 => 11,
        _ => panic!("Invalid white key index"),
    }
}

fn black_key_to_semitone(key: usize) -> usize {
    match key {
        0 => 1,
        1 => 3,
        2 => 6,
        3 => 8,
        4 => 10,
        _ => panic!("Invalid black key index"),
    }
}

fn semitone_name(semitone: usize) -> &'static str {
    match semitone {
        0 => "C",
        1 => "C#",
        2 => "D",
        3 => "D#",
        4 => "E",
        5 => "F",
        6 => "F#",
        7 => "G",
        8 => "G#",
        9 => "A",
        10 => "A#",
        11 => "B",
        _ => panic!("Invalid semitone"),
    }
}
