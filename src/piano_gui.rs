use bitvec::{BitArr, order::Msb0};
use egui::{Event, Rect, Sense, TouchPhase, Ui, pos2, vec2};
use std::collections::{HashMap, HashSet};
use wmidi::Note;

use crate::theme;

/// A semitone value within an octave (0-11)
/// Represents the 12 chromatic pitches: C, C#, D, D#, E, F, F#, G, G#, A, A#, B
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Semitone(u8);

impl Semitone {
    /// Create a new Semitone from a u8 value (0-11)
    pub const fn new(value: u8) -> Self {
        debug_assert!(value < 12, "Semitone value must be in range 0-11");
        Self(value)
    }

    /// Create a new Semitone from a usize value (0-11)
    pub const fn from_usize(value: usize) -> Self {
        debug_assert!(value < 12, "Semitone value must be in range 0-11");
        Self(value as u8)
    }

    /// Get the value as usize for compatibility with existing code
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Convert to an array index (same as as_usize but more explicit about intent)
    pub const fn as_index(self) -> usize {
        self.0 as usize
    }

    /// Check if this semitone represents a black key on a piano
    pub const fn is_black_key(self) -> bool {
        matches!(self.0, 1 | 3 | 6 | 8 | 10)
    }

    /// Get the white key index (0-6) for this semitone
    /// Panics if this is not a white key semitone
    pub fn white_key_index(self) -> usize {
        debug_assert!(!self.is_black_key(), "Semitone must be a white key");
        match self.0 {
            0 => 0,  // C
            2 => 1,  // D
            4 => 2,  // E
            5 => 3,  // F
            7 => 4,  // G
            9 => 5,  // A
            11 => 6, // B
            _ => panic!("Invalid white key semitone: {}", self.0),
        }
    }

    /// Get the black key index (0-4) for this semitone
    /// Panics if this is not a black key semitone
    pub fn black_key_index(self) -> usize {
        debug_assert!(self.is_black_key(), "Semitone must be a black key");
        match self.0 {
            1 => 0,  // C#
            3 => 1,  // D#
            6 => 2,  // F#
            8 => 3,  // G#
            10 => 4, // A#
            _ => panic!("Invalid black key semitone: {}", self.0),
        }
    }

    /// Get the note name for this semitone
    pub const fn name(self) -> &'static str {
        match self.0 {
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

    /// Convert this semitone to a Note in the specified octave
    pub fn to_note_in_octave(self, octave: u8) -> Note {
        debug_assert!(
            octave <= 9,
            "Octave must be in range 0-9 for valid MIDI notes"
        );

        // Calculate the MIDI note number: (octave + 1) * 12 + semitone
        // The +1 is because MIDI octave numbering starts at -1, so C4 = 60
        let midi_note = (octave as usize + 1) * 12 + self.as_usize();
        debug_assert!(midi_note <= 127, "MIDI note number must be <= 127");

        Note::try_from(midi_note as u8).unwrap()
    }

    /// Convert a Note to its semitone representation (0-11) within its octave
    pub fn from_note(note: Note) -> Self {
        Self::new(u8::from(note) % 12)
    }
}

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

    key_held_by_pointer: HashMap<PointerId, wmidi::Note>,
    pointers_holding_key: HashMap<wmidi::Note, HashSet<PointerId>>,
    previous_pointer_keys: KeySet, // Keys that were pressed by pointers in the previous frame

    /// The octave that this piano GUI displays (default: 4, meaning C4-B4)
    octave: u8,
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
}

impl PianoGui {
    pub fn new() -> Self {
        const DEFAULT_OCTAVE: u8 = 4;

        Self {
            selected_keys: Default::default(),
            external_keys: Default::default(),
            key_held_by_pointer: HashMap::new(),
            pointers_holding_key: HashMap::new(),
            previous_pointer_keys: Default::default(),
            octave: DEFAULT_OCTAVE, // Default to 4th octave (C4-B4)
        }
    }

    pub fn external_note_on(&mut self, note: Note) {
        let note_value = u8::from(note) as usize;
        debug_assert!(note_value < 128, "MIDI note value must be < 128");
        self.external_keys.set(note_value, true);
    }

    pub fn external_note_off(&mut self, note: Note) {
        let note_value = u8::from(note) as usize;
        debug_assert!(note_value < 128, "MIDI note value must be < 128");
        self.external_keys.set(note_value, false);
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
        // keys_rect represents the inner rectangle of the piano widget, after applying a margin.
        // All piano key positions and sizes are calculated relative to this area.
        let keys_rect = rect.shrink(MARGIN);
        let shift_pressed = ui.input(|i| i.modifiers.shift);

        // Process all pointer events (touch and mouse) using local state instead of egui's ui.data() system.
        // This avoids a specific WASM panic that occurred in egui 0.32.0 when ui.data()
        // triggered certain parking_lot code paths. While both egui and parking_lot support
        // WASM, this approach is more efficient and sidesteps the issue completely.

        // Handle touch events
        ui.input(|i| {
            for event in &i.events {
                if let Event::Touch { id, phase, pos, .. } = event {
                    let pointer_id = PointerId::Touch(id.0);

                    match phase {
                        TouchPhase::Start | TouchPhase::Move => {
                            let target_note = self.find_key_at_position(*pos, keys_rect);
                            self.handle_pointer_move(pointer_id, target_note);
                        }
                        TouchPhase::End | TouchPhase::Cancel => {
                            self.handle_pointer_release(pointer_id);
                        }
                    }
                }
            }
        });

        // Handle mouse interactions
        let mouse_pointer_id = PointerId::Mouse;
        let mouse_pos = ui.input(|i| i.pointer.latest_pos());
        let mouse_down = ui.input(|i| i.pointer.primary_down());

        if let Some(pos) = mouse_pos {
            if mouse_down {
                let target_note = self.find_key_at_position(pos, keys_rect);
                self.handle_pointer_move(mouse_pointer_id, target_note);
            } else {
                self.handle_pointer_release(mouse_pointer_id);
            }
        } else {
            self.handle_pointer_release(mouse_pointer_id);
        }

        // Second pass: render keys and handle interactions.
        // This architecture processes all pointer events globally first, then renders each key,
        // which is more efficient than the previous approach where each key processed all events.
        // The local state management also eliminates redundant event processing and ensures
        // proper multitouch handling without WASM compatibility issues.

        // Render white keys first (so black keys appear on top)
        for semitone in [0, 2, 4, 5, 7, 9, 11] {
            self.render_key(
                Semitone::new(semitone),
                ui,
                &painter,
                keys_rect,
                &pressed_keys,
                shift_pressed,
                &mut actions,
            );
        }

        // Render black keys on top
        for semitone in [1, 3, 6, 8, 10] {
            self.render_key(
                Semitone::new(semitone),
                ui,
                &painter,
                keys_rect,
                &pressed_keys,
                shift_pressed,
                &mut actions,
            );
        }

        // Update previous pointer keys for the next frame
        self.previous_pointer_keys.fill(false);
        for &note in self.pointers_holding_key.keys() {
            if !self.pointers_holding_key[&note].is_empty() {
                let semitone = Semitone::from_note(note);
                self.previous_pointer_keys.set(semitone.as_index(), true);
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
            let root = Semitone::from_usize(root_semitone).name();

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
            Some(
                Semitone::from_usize(selected_semitones[0])
                    .name()
                    .to_string(),
            )
        } else {
            let notes: Vec<String> = selected_semitones
                .iter()
                .map(|&semitone| Semitone::from_usize(semitone).name().to_string())
                .collect();
            Some(notes.join("/"))
        }
    }

    /// Render a single piano key with all associated logic
    ///
    /// This method takes 8 arguments, which exceeds clippy's default limit of 7.
    /// However, this is justified because:
    /// 1. The method extracts duplicated rendering logic from two loops (white/black keys)
    /// 2. All arguments are necessary for key rendering and interaction handling
    /// 3. Grouping them into a struct would be artificial since they represent different concerns
    ///    (UI state, rendering context, input state, and output collection)
    /// 4. The method significantly reduces code duplication (eliminated ~80 lines of repetition)
    #[allow(clippy::too_many_arguments)]
    fn render_key(
        &mut self,
        semitone: Semitone,
        ui: &mut Ui,
        painter: &egui::Painter,
        keys_rect: Rect,
        pressed_keys: &KeySet,
        shift_pressed: bool,
        actions: &mut Vec<Action>,
    ) {
        let note = semitone.to_note_in_octave(self.octave);
        let key_rect = key_rect_for_semitone(semitone, keys_rect);

        // Get active pointers for this key from our local state
        let all_pointers = self
            .pointers_holding_key
            .get(&note)
            .cloned()
            .unwrap_or_default();

        // Allocate space for the key (needed for proper UI layout)
        ui.allocate_rect(key_rect, Sense::click_and_drag());

        let is_pressed = !all_pointers.is_empty();
        let was_pressed = self.previous_pointer_keys[semitone.as_index()];

        if is_pressed && !was_pressed {
            actions.push(Action::Pressed(note));
            if !shift_pressed {
                self.selected_keys.fill(false);
            }
            let key_selected = self.selected_keys[semitone.as_index()];
            self.selected_keys.set(semitone.as_index(), !key_selected);
        } else if !is_pressed && was_pressed {
            actions.push(Action::Released(note));
        }

        let selected = self.selected_keys[semitone.as_index()];
        let combined_selected = pressed_keys[semitone.as_index()];

        let key_fill = if selected {
            theme::selected_key()
        } else if combined_selected {
            theme::external_selected_key()
        } else {
            ui.visuals().panel_fill
        };
        let key_stroke = egui::Stroke::new(2.0, theme::outlines());
        painter.rect(
            key_rect,
            0.0,
            key_fill,
            key_stroke,
            egui::StrokeKind::Middle,
        );
        if is_pressed {
            const HIGHLIGHT_INSET: f32 = 2.0;
            let highlight_rect = key_rect.shrink(HIGHLIGHT_INSET);
            painter.rect_stroke(
                highlight_rect,
                0.0,
                egui::Stroke::new(2.0, theme::selected_key()),
                egui::StrokeKind::Middle,
            );
        }
    }

    /// Find which key is at the given position, checking black keys first for proper layering
    fn find_key_at_position(&self, pos: egui::Pos2, keys_rect: Rect) -> Option<wmidi::Note> {
        debug_assert!(
            keys_rect.is_positive(),
            "Keys rect must have positive dimensions"
        );

        // Check black keys first (they're on top)
        for semitone in [1, 3, 6, 8, 10] {
            let semitone = Semitone::new(semitone);
            let key_rect = key_rect_for_semitone(semitone, keys_rect);
            if key_rect.contains(pos) {
                return Some(semitone.to_note_in_octave(self.octave));
            }
        }

        // If not on a black key, check white keys
        for semitone in [0, 2, 4, 5, 7, 9, 11] {
            let semitone = Semitone::new(semitone);
            let key_rect = key_rect_for_semitone(semitone, keys_rect);
            if key_rect.contains(pos) {
                return Some(semitone.to_note_in_octave(self.octave));
            }
        }

        None
    }

    /// Handle a pointer moving to a new key (or moving off all keys)
    fn handle_pointer_move(&mut self, pointer_id: PointerId, target_note: Option<wmidi::Note>) {
        if let Some(new_note) = target_note {
            // Check if pointer moved to a different key
            if let Some(old_note) = self.key_held_by_pointer.get(&pointer_id) {
                let old_note_val = *old_note;
                if old_note_val != new_note {
                    // Move to the new key
                    self.move_pointer_to_key(pointer_id, new_note);
                }
            } else {
                // New pointer press
                self.add_pointer_to_key(pointer_id, new_note);
            }
        } else {
            // Pointer moved outside all keys
            self.remove_pointer_from_current_key(pointer_id);
        }
    }

    /// Handle a pointer being released or ending
    fn handle_pointer_release(&mut self, pointer_id: PointerId) {
        self.remove_pointer_from_current_key(pointer_id);
    }

    /// Add a pointer to a key, updating both tracking data structures
    fn add_pointer_to_key(&mut self, pointer_id: PointerId, note: wmidi::Note) {
        // Update the reverse mapping (pointer -> key)
        self.key_held_by_pointer.insert(pointer_id, note);

        // Update the forward mapping (key -> pointers)
        let was_inserted = self
            .pointers_holding_key
            .entry(note)
            .or_default()
            .insert(pointer_id);

        debug_assert!(
            was_inserted,
            "Pointer should not already be in the key's set when adding"
        );
    }

    /// Remove a pointer from its current key, updating both tracking data structures
    /// Returns the note that the pointer was removed from, if any
    fn remove_pointer_from_current_key(&mut self, pointer_id: PointerId) -> Option<wmidi::Note> {
        if let Some(old_note) = self.key_held_by_pointer.remove(&pointer_id) {
            if let Some(pointers) = self.pointers_holding_key.get_mut(&old_note) {
                let was_removed = pointers.remove(&pointer_id);
                debug_assert!(
                    was_removed,
                    "Pointer should have been in the key's set when removed"
                );
            }
            Some(old_note)
        } else {
            None
        }
    }

    /// Move a pointer from its current key to a new key, updating both tracking data structures
    fn move_pointer_to_key(&mut self, pointer_id: PointerId, new_note: wmidi::Note) {
        // Remove from current key (if any)
        self.remove_pointer_from_current_key(pointer_id);

        // Add to new key
        self.add_pointer_to_key(pointer_id, new_note);
    }
}

/// Returns the rectangle for a piano key.
/// * `semitone` - The semitone index (0-11) representing the key within the octave. Determines which piano key's rectangle to compute.
/// * `rect` - The bounding rectangle of the entire piano area. All key positions and sizes are calculated relative to this rectangle.
fn key_rect_for_semitone(semitone: Semitone, rect: Rect) -> Rect {
    debug_assert!(
        rect.is_positive(),
        "Piano rect must have positive dimensions"
    );

    const WHITE_KEY_X_POSITIONS: [f32; 7] = [0.0, 1.5, 3.5, 5.0, 6.5, 8.5, 10.5];
    const BLACK_KEY_X_POSITIONS: [f32; 5] = [1.0, 3.0, 6.0, 8.0, 10.0];
    const SEMITONES_IN_OCTAVE: f32 = 12.0;
    const BLACK_KEY_HEIGHT_RATIO: f32 = 0.6;

    if semitone.is_black_key() {
        let black_key_index = semitone.black_key_index();
        debug_assert!(
            black_key_index < BLACK_KEY_X_POSITIONS.len(),
            "Black key index out of bounds"
        );
        let x_pos = BLACK_KEY_X_POSITIONS[black_key_index];
        let key_size = vec2(
            rect.width() / SEMITONES_IN_OCTAVE,
            rect.height() * BLACK_KEY_HEIGHT_RATIO,
        );
        Rect::from_min_size(
            pos2(
                rect.min.x + x_pos / SEMITONES_IN_OCTAVE * rect.width(),
                rect.min.y,
            ),
            key_size,
        )
    } else {
        let white_key_index = semitone.white_key_index();
        debug_assert!(
            white_key_index < WHITE_KEY_X_POSITIONS.len(),
            "White key index out of bounds"
        );
        let x_pos = WHITE_KEY_X_POSITIONS[white_key_index];
        let next_x_pos = WHITE_KEY_X_POSITIONS
            .get(white_key_index + 1)
            .unwrap_or(&SEMITONES_IN_OCTAVE);
        let key_size = vec2(
            (next_x_pos - x_pos) / SEMITONES_IN_OCTAVE * rect.width(),
            rect.height(),
        );
        Rect::from_min_size(
            pos2(
                rect.min.x + x_pos / SEMITONES_IN_OCTAVE * rect.width(),
                rect.min.y,
            ),
            key_size,
        )
    }
}
