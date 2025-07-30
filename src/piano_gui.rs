use bitvec::{BitArr, order::Msb0};
use egui::{Event, Rect, Sense, Stroke, StrokeKind, TouchPhase, Ui, pos2, vec2};
use std::collections::{HashMap, HashSet};
use wmidi::Note;

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
    // Touch state managed locally instead of using egui's ui.data() to avoid a specific
    // WASM panic that occurred with egui 0.32.0. While egui and parking_lot both support
    // WASM, ui.data() triggered a code path that caused threading-related panics.
    // This local state approach is more efficient anyway and avoids the issue entirely.
    // ~ CLAUDE
    pointer_to_key: HashMap<PointerId, usize>, // PointerId -> semitone
    key_pointers: HashMap<usize, HashSet<PointerId>>, // semitone -> set of pointers
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
}

impl PianoGui {
    pub fn new() -> Self {
        Self {
            selected_keys: Default::default(),
            external_keys: Default::default(),
            pointer_to_key: HashMap::new(),
            key_pointers: HashMap::new(),
        }
    }

    pub fn external_note_on(&mut self, note: Note) {
        self.external_keys.set(u8::from(note) as usize, true);
    }

    pub fn external_note_off(&mut self, note: Note) {
        self.external_keys.set(u8::from(note) as usize, false);
    }

    pub fn show(&mut self, ui: &mut Ui) -> (Vec<Action>, Rect) {
        let pressed_keys = self.pressed_keys();
        let mut actions = Vec::new();
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

        // Create a map of key_id -> (key_rect, semitone)
        let mut key_info = HashMap::new();

        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        const WHITE_KEY_X_POSITIONS: [f32; NUM_WHITE_KEYS] = [0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5];
        const BLACK_KEY_X_POSITIONS: [f32; NUM_BLACK_KEYS] = [1.0, 3.0, 6.0, 8.0, 10.0];
        const SEMITONES_IN_OCTAVE: f32 = 12.0;
        const BLACK_KEY_HEIGHT_RATIO: f32 = 0.6;

        #[derive(strum_macros::Display)]
        enum Color {
            White,
            Black,
        }

        // First pass: build key info map
        for color in [Color::White, Color::Black] {
            let (num_keys, x_positions, semitone_fn) = match color {
                Color::White => (
                    NUM_WHITE_KEYS,
                    &WHITE_KEY_X_POSITIONS[..],
                    white_key_to_semitone as fn(usize) -> usize,
                ),
                Color::Black => (
                    NUM_BLACK_KEYS,
                    &BLACK_KEY_X_POSITIONS[..],
                    black_key_to_semitone as fn(usize) -> usize,
                ),
            };

            for key in 0..num_keys {
                let key_id = ui.id().with(format!("{color}{key}"));
                let key_size = match color {
                    Color::White => vec2(
                        (x_positions.get(key + 1).unwrap_or(&SEMITONES_IN_OCTAVE)
                            - x_positions[key])
                            / SEMITONES_IN_OCTAVE
                            * keys_rect.width(),
                        keys_rect.height(),
                    ),
                    Color::Black => vec2(
                        keys_rect.width() / SEMITONES_IN_OCTAVE,
                        keys_rect.height() * BLACK_KEY_HEIGHT_RATIO,
                    ),
                };
                let key_rect = Rect::from_min_size(
                    pos2(
                        keys_rect.min.x
                            + x_positions[key] / SEMITONES_IN_OCTAVE * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                );
                let semitone = semitone_fn(key);
                key_info.insert(key_id, (key_rect, semitone));
            }
        }

        // Process all touch events using local state instead of egui's ui.data() system.
        // This avoids a specific WASM panic that occurred in egui 0.32.0 when ui.data()
        // triggered certain parking_lot code paths. While both egui and parking_lot support
        // WASM, this approach is more efficient and sidesteps the issue completely.
        ui.input(|i| {
            for event in &i.events {
                if let Event::Touch { id, phase, pos, .. } = event {
                    let pointer_id = PointerId::Touch(id.0);

                    match phase {
                        TouchPhase::Start | TouchPhase::Move => {
                            // Find which key this touch is over (check black keys first for proper layering)
                            let mut target_semitone = None;
                            for color in [Color::Black, Color::White] {
                                let num_keys = match color {
                                    Color::White => NUM_WHITE_KEYS,
                                    Color::Black => NUM_BLACK_KEYS,
                                };
                                for key in 0..num_keys {
                                    let key_id = ui.id().with(format!("{color}{key}"));
                                    if let Some((key_rect, semitone)) = key_info.get(&key_id) {
                                        if key_rect.contains(*pos) {
                                            target_semitone = Some(*semitone);
                                            break;
                                        }
                                    }
                                }
                                if target_semitone.is_some() {
                                    break;
                                }
                            }

                            if let Some(new_semitone) = target_semitone {
                                // Check if touch moved to a different key
                                if let Some(old_semitone) = self.pointer_to_key.get(&pointer_id) {
                                    if *old_semitone != new_semitone {
                                        // Remove from old key
                                        if let Some(pointers) =
                                            self.key_pointers.get_mut(old_semitone)
                                        {
                                            pointers.remove(&pointer_id);
                                        }
                                        // Add to new key
                                        self.pointer_to_key.insert(pointer_id, new_semitone);
                                        self.key_pointers
                                            .entry(new_semitone)
                                            .or_default()
                                            .insert(pointer_id);
                                    }
                                } else {
                                    // New touch
                                    self.pointer_to_key.insert(pointer_id, new_semitone);
                                    self.key_pointers
                                        .entry(new_semitone)
                                        .or_default()
                                        .insert(pointer_id);
                                }
                            } else {
                                // Touch moved outside all keys
                                if let Some(old_semitone) = self.pointer_to_key.remove(&pointer_id)
                                {
                                    if let Some(pointers) = self.key_pointers.get_mut(&old_semitone)
                                    {
                                        pointers.remove(&pointer_id);
                                    }
                                }
                            }
                        }
                        TouchPhase::End | TouchPhase::Cancel => {
                            // Touch ended - remove from tracking
                            if let Some(old_semitone) = self.pointer_to_key.remove(&pointer_id) {
                                if let Some(pointers) = self.key_pointers.get_mut(&old_semitone) {
                                    pointers.remove(&pointer_id);
                                }
                            }
                        }
                    }
                }
            }
        });

        // Second pass: render keys and handle mouse interactions.
        // This architecture processes all touch events globally first, then renders each key,
        // which is more efficient than the previous approach where each key processed all events.
        // The local state management also eliminates redundant event processing and ensures
        // proper multitouch handling without WASM compatibility issues.
        for color in [Color::White, Color::Black] {
            let num_keys = match color {
                Color::White => NUM_WHITE_KEYS,
                Color::Black => NUM_BLACK_KEYS,
            };
            for key in 0..num_keys {
                let key_id = ui.id().with(format!("{color}{key}"));
                let (key_rect, semitone) = key_info[&key_id];

                // Get active pointers for this key from our local state
                let touch_pointers = self
                    .key_pointers
                    .get(&semitone)
                    .cloned()
                    .unwrap_or_default();
                let mut all_pointers = touch_pointers;

                // Handle mouse interactions
                let key_response = ui.allocate_rect(key_rect, Sense::click_and_drag());
                let mouse_pressed = key_response.is_pointer_button_down_on();
                let mouse_pointer_id = PointerId::Mouse;

                // Track mouse pointer state
                if mouse_pressed {
                    all_pointers.insert(mouse_pointer_id);
                }

                let is_pressed = !all_pointers.is_empty();

                let was_pressed = pressed_keys[semitone];

                if is_pressed && !was_pressed {
                    let note = wmidi::Note::C4.step(semitone as i8).unwrap();
                    actions.push(Action::Pressed(note));
                    if !shift_pressed {
                        self.selected_keys.fill(false);
                    }
                    let key_selected = self.selected_keys[semitone];
                    self.selected_keys.set(semitone, !key_selected);
                } else if !is_pressed && was_pressed {
                    let note = wmidi::Note::C4.step(semitone as i8).unwrap();
                    actions.push(Action::Released(note));
                }

                let selected = self.selected_keys[semitone];
                let combined_selected = pressed_keys[semitone];

                let key_fill = if selected {
                    theme::selected_key()
                } else if combined_selected {
                    theme::external_selected_key()
                } else {
                    ui.visuals().panel_fill
                };
                let key_stroke = Stroke::new(2.0, theme::outlines());
                painter.rect(key_rect, 0.0, key_fill, key_stroke, StrokeKind::Middle);
                if is_pressed {
                    const HIGHLIGHT_INSET: f32 = 2.0;
                    let highlight_rect = key_rect.shrink(HIGHLIGHT_INSET);
                    painter.rect_stroke(
                        highlight_rect,
                        0.0,
                        Stroke::new(2.0, theme::selected_key()),
                        StrokeKind::Middle,
                    );
                }
            }
        }

        (actions, keys_rect)
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
