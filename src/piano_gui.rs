use bitvec::{BitArr, order::Msb0};
use egui::{Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2, Id, Event, TouchPhase};
use wmidi::Note;
use std::collections::{HashMap, HashSet};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PointerId {
    Mouse,
    Touch(u64),
}

pub const PIANO_WIDTH: f32 = 600.0;
pub const PIANO_HEIGHT: f32 = 200.0;

pub type KeySet = BitArr!(for 12, in u16, Msb0);
type ExternalKeySet = BitArr!(for 128, in u32, Msb0);

pub struct PianoGui {
    selected_keys: KeySet,
    external_keys: ExternalKeySet,
}

impl PianoGui {
    pub fn new() -> Self {
        Self {
            selected_keys: Default::default(),
            external_keys: Default::default(),
        }
    }

    pub fn external_note_on(&mut self, note: Note) {
        self.external_keys.set(u8::from(note) as usize, true);
    }

    pub fn external_note_off(&mut self, note: Note) {
        self.external_keys.set(u8::from(note) as usize, false);
    }

    pub fn show(&mut self, ui: &mut Ui) -> (Option<Action>, Rect) {
        let pressed_keys = self.pressed_keys();
        let mut action = None;
        let mut piano_size = vec2(PIANO_WIDTH, PIANO_HEIGHT);
        if piano_size.x > ui.available_width() {
            const MIN_PIANO_SCALE: f32 = 0.5;
            piano_size *= (ui.available_width() / piano_size.x).max(MIN_PIANO_SCALE);
        }
        let (response, painter) = ui.allocate_painter(piano_size, Sense::empty());
        let rect = response.rect;
        const PIANO_RECT_CORNER_RADIUS: f32 = 1.0;
        painter.rect_filled(rect, PIANO_RECT_CORNER_RADIUS, ui.visuals().panel_fill);
        const MARGIN: f32 = 2.0;
        let keys_rect = rect.shrink(MARGIN);
        let shift_pressed = ui.input(|i| i.modifiers.shift);
        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        const WHITE_KEY_X_POSITIONS: [f32; NUM_WHITE_KEYS] = [0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5];
        const BLACK_KEY_X_POSITIONS: [f32; NUM_BLACK_KEYS] = [1.0, 3.0, 6.0, 8.0, 10.0];
        const SEMITONES_IN_OCTAVE: f32 = 12.0;

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
                Color::White => WHITE_KEY_X_POSITIONS.to_vec(),
                Color::Black => BLACK_KEY_X_POSITIONS.to_vec(),
            };
            for key in 0..num_keys {
                let key_id = ui.id().with(format!("{color}{key}"));
                let key_size = match color {
                    Color::White => vec2(
                        (x.get(key + 1).unwrap_or(&SEMITONES_IN_OCTAVE) - x[key])
                            / SEMITONES_IN_OCTAVE
                            * keys_rect.width(),
                        keys_rect.height(),
                    ),
                    Color::Black => {
                        const BLACK_KEY_HEIGHT_RATIO: f32 = 0.6;
                        vec2(
                            keys_rect.width() / SEMITONES_IN_OCTAVE,
                            keys_rect.height() * BLACK_KEY_HEIGHT_RATIO,
                        )
                    }
                };
                let key_rect = Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + x[key] / SEMITONES_IN_OCTAVE * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                );
                let semitone = match color {
                    Color::White => white_key_to_semitone(key),
                    Color::Black => black_key_to_semitone(key),
                };
                let selected = self.selected_keys[semitone];
                let combined_selected = pressed_keys[semitone];
                let note = wmidi::Note::C4.step(semitone as i8).unwrap();
                const KEY_RECT_CORNER_RADIUS: f32 = 0.0;
                const KEY_OUTLINE_STROKE_WIDTH: f32 = 2.0;
                painter.rect(
                    key_rect,
                    KEY_RECT_CORNER_RADIUS,
                    if selected {
                        theme::selected_key()
                    } else if combined_selected {
                        theme::external_selected_key()
                    } else {
                        ui.visuals().panel_fill
                    },
                    Stroke::new(KEY_OUTLINE_STROKE_WIDTH, theme::outlines()),
                    StrokeKind::Middle,
                );
                let key_response = ui.interact(key_rect, key_id, Sense::click());
                
                // Get multi-pointer tracking data
                let mut active_pointers = ui.data(|r| {
                    r.get_temp::<HashSet<PointerId>>(key_id).unwrap_or_default()
                });
                let mut pointer_to_key = ui.data(|r| {
                    r.get_temp::<HashMap<PointerId, Id>>(ui.id().with("pointer_to_key")).unwrap_or_default()
                });
                
                let was_pressed = !active_pointers.is_empty();
                
                // Handle mouse input
                if key_response.is_pointer_button_down_on() {
                    if !active_pointers.contains(&PointerId::Mouse) {
                        // Mouse press started
                        active_pointers.insert(PointerId::Mouse);
                        pointer_to_key.insert(PointerId::Mouse, key_id);
                    }
                } else {
                    if active_pointers.contains(&PointerId::Mouse) {
                        // Mouse press ended
                        active_pointers.remove(&PointerId::Mouse);
                        pointer_to_key.remove(&PointerId::Mouse);
                    }
                }
                
                // Process touch events
                ui.input(|i| {
                    for event in &i.events {
                        if let Event::Touch { id, phase, pos, .. } = event {
                            let pointer_id = PointerId::Touch(id.0);
                            match phase {
                                TouchPhase::Start | TouchPhase::Move => {
                                    if key_rect.contains(*pos) {
                                        if !active_pointers.contains(&pointer_id) {
                                            // New touch on this key
                                            if let Some(old_key) = pointer_to_key.get(&pointer_id) {
                                                // Touch moved from another key, remove from old key
                                                if let Some(mut old_pointers) = ui.data(|r| r.get_temp::<HashSet<PointerId>>(*old_key)) {
                                                    old_pointers.remove(&pointer_id);
                                                    ui.data_mut(|r| r.insert_temp(*old_key, old_pointers));
                                                }
                                            }
                                            active_pointers.insert(pointer_id);
                                            pointer_to_key.insert(pointer_id, key_id);
                                        }
                                    } else {
                                        // Touch moved away from this key
                                        if active_pointers.contains(&pointer_id) {
                                            active_pointers.remove(&pointer_id);
                                            pointer_to_key.remove(&pointer_id);
                                        }
                                    }
                                }
                                TouchPhase::End | TouchPhase::Cancel => {
                                    // Touch ended
                                    if active_pointers.contains(&pointer_id) {
                                        active_pointers.remove(&pointer_id);
                                        pointer_to_key.remove(&pointer_id);
                                    }
                                }
                            }
                        }
                    }
                });
                
                let is_pressed = !active_pointers.is_empty();
                
                // Generate actions based on state changes
                if is_pressed && !was_pressed {
                    debug_assert!(action.is_none());
                    action = Some(Action::Pressed(note));
                    if !shift_pressed {
                        self.selected_keys.fill(false);
                    }
                    let key_selected = self.selected_keys[semitone];
                    self.selected_keys.set(semitone, !key_selected);
                } else if !is_pressed && was_pressed {
                    debug_assert!(action.is_none());
                    action = Some(Action::Released(note));
                }
                
                // Store updated tracking data
                ui.data_mut(|r| {
                    r.insert_temp(key_id, active_pointers);
                    r.insert_temp(ui.id().with("pointer_to_key"), pointer_to_key);
                });
            }
        }
        (action, keys_rect)
    }

    pub fn pressed_keys(&self) -> KeySet {
        let mut keys = self.selected_keys;
        for external_key in self.external_keys.iter_ones() {
            keys.set(external_key % 12, true);
        }
        keys
    }

    pub fn selected_chord_name(&self) -> Option<String> {
        // AI generated. But seems mostly sensible
        let mut selected_semitones: Vec<usize> = self.pressed_keys().iter_ones().collect();
        if selected_semitones.is_empty() {
            return None;
        }

        // Sort semitones to normalize chord representation
        selected_semitones.sort();

        // Try all rotations of the chord (all possible roots)
        for rotation in 0..selected_semitones.len() {
            let root_semitone = selected_semitones[rotation];
            let root = semitone_name(root_semitone);

            let mut intervals: Vec<usize> = Vec::new();
            for &semitone in selected_semitones.iter() {
                if semitone != root_semitone {
                    intervals
                        .push((semitone as i32 - root_semitone as i32).rem_euclid(12) as usize);
                }
            }
            intervals.sort();

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

        if selected_semitones.len() == 1 {
            Some(semitone_name(selected_semitones[0]).to_string())
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
