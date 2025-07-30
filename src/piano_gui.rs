use bitvec::{BitArr, order::Msb0};
use egui::{Event, Rect, Sense, Stroke, StrokeKind, TouchPhase, Ui, pos2, vec2};
use std::collections::{HashSet, HashMap};
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
    previous_pointer_keys: KeySet, // Keys that were pressed by pointers in the previous frame
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
            previous_pointer_keys: Default::default(),
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

        const NUM_WHITE_KEYS: usize = 7;
        const NUM_BLACK_KEYS: usize = 5;
        const WHITE_KEY_X_POSITIONS: [f32; NUM_WHITE_KEYS] = [0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5];
        const BLACK_KEY_X_POSITIONS: [f32; NUM_BLACK_KEYS] = [1.0, 3.0, 6.0, 8.0, 10.0];
        const SEMITONES_IN_OCTAVE: f32 = 12.0;
        const BLACK_KEY_HEIGHT_RATIO: f32 = 0.6;

        // Helper function to get key rect from semitone
        let key_rect_for_semitone = |semitone: usize| -> Rect {
            if is_black_key(semitone) {
                let black_key_index = semitone_to_black_key_index(semitone);
                let x_pos = BLACK_KEY_X_POSITIONS[black_key_index];
                let key_size = vec2(
                    keys_rect.width() / SEMITONES_IN_OCTAVE,
                    keys_rect.height() * BLACK_KEY_HEIGHT_RATIO,
                );
                Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + x_pos / SEMITONES_IN_OCTAVE * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                )
            } else {
                let white_key_index = semitone_to_white_key_index(semitone);
                let x_pos = WHITE_KEY_X_POSITIONS[white_key_index];
                let next_x_pos = WHITE_KEY_X_POSITIONS.get(white_key_index + 1).unwrap_or(&SEMITONES_IN_OCTAVE);
                let key_size = vec2(
                    (next_x_pos - x_pos) / SEMITONES_IN_OCTAVE * keys_rect.width(),
                    keys_rect.height(),
                );
                Rect::from_min_size(
                    pos2(
                        keys_rect.min.x + x_pos / SEMITONES_IN_OCTAVE * keys_rect.width(),
                        keys_rect.min.y,
                    ),
                    key_size,
                )
            }
        };

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
                            
                            // Check black keys first (they're on top)
                            for semitone in [1, 3, 6, 8, 10] {
                                let key_rect = key_rect_for_semitone(semitone);
                                if key_rect.contains(*pos) {
                                    target_semitone = Some(semitone);
                                    break;
                                }
                            }
                            
                            // If not on a black key, check white keys
                            if target_semitone.is_none() {
                                for semitone in [0, 2, 4, 5, 7, 9, 11] {
                                    let key_rect = key_rect_for_semitone(semitone);
                                    if key_rect.contains(*pos) {
                                        target_semitone = Some(semitone);
                                        break;
                                    }
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

        // Handle mouse interactions globally, similar to touch handling
        let mouse_pointer_id = PointerId::Mouse;
        let mouse_pos = ui.input(|i| i.pointer.latest_pos());
        let mouse_down = ui.input(|i| i.pointer.primary_down());

        if let Some(pos) = mouse_pos {
            if mouse_down {
                // Find which key the mouse is over (check black keys first for proper layering)
                let mut target_semitone = None;
                
                // Check black keys first (they're on top)
                for semitone in [1, 3, 6, 8, 10] {
                    let key_rect = key_rect_for_semitone(semitone);
                    if key_rect.contains(pos) {
                        target_semitone = Some(semitone);
                        break;
                    }
                }
                
                // If not on a black key, check white keys
                if target_semitone.is_none() {
                    for semitone in [0, 2, 4, 5, 7, 9, 11] {
                        let key_rect = key_rect_for_semitone(semitone);
                        if key_rect.contains(pos) {
                            target_semitone = Some(semitone);
                            break;
                        }
                    }
                }

                if let Some(new_semitone) = target_semitone {
                    // Check if mouse moved to a different key
                    if let Some(old_semitone) = self.pointer_to_key.get(&mouse_pointer_id) {
                        if *old_semitone != new_semitone {
                            // Remove from old key
                            if let Some(pointers) = self.key_pointers.get_mut(old_semitone) {
                                pointers.remove(&mouse_pointer_id);
                            }
                            // Add to new key
                            self.pointer_to_key.insert(mouse_pointer_id, new_semitone);
                            self.key_pointers
                                .entry(new_semitone)
                                .or_default()
                                .insert(mouse_pointer_id);
                        }
                    } else {
                        // New mouse press
                        self.pointer_to_key.insert(mouse_pointer_id, new_semitone);
                        self.key_pointers
                            .entry(new_semitone)
                            .or_default()
                            .insert(mouse_pointer_id);
                    }
                } else {
                    // Mouse moved outside all keys
                    if let Some(old_semitone) = self.pointer_to_key.remove(&mouse_pointer_id) {
                        if let Some(pointers) = self.key_pointers.get_mut(&old_semitone) {
                            pointers.remove(&mouse_pointer_id);
                        }
                    }
                }
            } else {
                // Mouse button not down - remove from tracking
                if let Some(old_semitone) = self.pointer_to_key.remove(&mouse_pointer_id) {
                    if let Some(pointers) = self.key_pointers.get_mut(&old_semitone) {
                        pointers.remove(&mouse_pointer_id);
                    }
                }
            }
        } else {
            // No mouse position - remove from tracking
            if let Some(old_semitone) = self.pointer_to_key.remove(&mouse_pointer_id) {
                if let Some(pointers) = self.key_pointers.get_mut(&old_semitone) {
                    pointers.remove(&mouse_pointer_id);
                }
            }
        }

        // Second pass: render keys and handle interactions.
        // This architecture processes all pointer events globally first, then renders each key,
        // which is more efficient than the previous approach where each key processed all events.
        // The local state management also eliminates redundant event processing and ensures
        // proper multitouch handling without WASM compatibility issues.
        
        // Render white keys first (so black keys appear on top)
        for semitone in [0, 2, 4, 5, 7, 9, 11] {
            let key_rect = key_rect_for_semitone(semitone);

            // Get active pointers for this key from our local state
            let all_pointers = self
                .key_pointers
                .get(&semitone)
                .cloned()
                .unwrap_or_default();

            // Allocate space for the key (needed for proper UI layout)
            ui.allocate_rect(key_rect, Sense::click_and_drag());

            let is_pressed = !all_pointers.is_empty();

            let was_pressed = self.previous_pointer_keys[semitone];

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
        
        // Render black keys on top
        for semitone in [1, 3, 6, 8, 10] {
            let key_rect = key_rect_for_semitone(semitone);

            // Get active pointers for this key from our local state
            let all_pointers = self
                .key_pointers
                .get(&semitone)
                .cloned()
                .unwrap_or_default();

            // Allocate space for the key (needed for proper UI layout)
            ui.allocate_rect(key_rect, Sense::click_and_drag());

            let is_pressed = !all_pointers.is_empty();

            let was_pressed = self.previous_pointer_keys[semitone];

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

        // Update previous pointer keys for the next frame
        self.previous_pointer_keys.fill(false);
        for &semitone in self.key_pointers.keys() {
            if !self.key_pointers[&semitone].is_empty() {
                self.previous_pointer_keys.set(semitone, true);
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

fn is_black_key(semitone: usize) -> bool {
    matches!(semitone, 1 | 3 | 6 | 8 | 10)
}

fn semitone_to_white_key_index(semitone: usize) -> usize {
    match semitone {
        0 => 0,  // C
        2 => 1,  // D
        4 => 2,  // E
        5 => 3,  // F
        7 => 4,  // G
        9 => 5,  // A
        11 => 6, // B
        _ => panic!("Invalid white key semitone: {semitone}"),
    }
}

fn semitone_to_black_key_index(semitone: usize) -> usize {
    match semitone {
        1 => 0,  // C#
        3 => 1,  // D#
        6 => 2,  // F#
        8 => 3,  // G#
        10 => 4, // A#
        _ => panic!("Invalid black key semitone: {semitone}"),
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
