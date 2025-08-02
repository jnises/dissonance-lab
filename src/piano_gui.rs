use egui::{Event, Rect, Sense, TouchPhase, Ui, pos2, vec2};
use std::collections::{HashMap, HashSet};
use wmidi::Note;

mod types;
use types::{Semitone, PointerId, KeySet, ExternalKeySet};
use crate::theme;

pub const PIANO_WIDTH: f32 = 600.0;
pub const PIANO_HEIGHT: f32 = 200.0;

pub struct PianoGui {
    /// Keys that had active GUI pointers (mouse/touch) in the previous frame.
    /// Used to detect press/release transitions for action generation.
    /// Does NOT include sustained keys or keys held via external MIDI input.
    previous_pressed_keys: KeySet,

    /// Keys that were pressed via GUI while sustain was active, but have since been released.
    /// These keys will remain active until the sustain pedal is released.
    sustained_keys: KeySet,

    /// Keys that are currently pressed via external MIDI input.
    /// Tracks all 128 MIDI notes, not just the current octave.
    external_pressed_keys: ExternalKeySet,

    /// Keys that are sustained due to sustain pedal being active when they were released.
    /// These keys will remain active until the sustain pedal is released.
    external_sustained_keys: ExternalKeySet,

    /// Maps each active pointer (mouse/touch) to the note it's currently pressing.
    /// Used for reverse lookup: given a pointer, what key is it on?
    key_held_by_pointer: HashMap<PointerId, wmidi::Note>,

    /// Maps each note to the set of pointers currently pressing it.
    /// Enables multi-touch: multiple fingers can press the same key simultaneously.
    pointers_holding_key: HashMap<wmidi::Note, HashSet<PointerId>>,

    /// The octave that this piano GUI displays (default: 4, meaning C4-B4)
    octave: u8,

    /// Whether the shift key was active in the previous frame
    previous_shift_sustain_active: bool,

    /// Whether the shift key is currently active
    shift_sustain_active: bool,

    /// Whether external MIDI sustain is currently active
    external_sustain_active: bool,
}

pub enum Action {
    Pressed(wmidi::Note),
    Released(wmidi::Note),
    SustainPedal(bool),
}

impl PianoGui {
    pub fn new() -> Self {
        const DEFAULT_OCTAVE: u8 = 4;

        Self {
            sustained_keys: Default::default(),
            external_pressed_keys: Default::default(),
            external_sustained_keys: Default::default(),
            key_held_by_pointer: HashMap::new(),
            pointers_holding_key: HashMap::new(),
            previous_pressed_keys: Default::default(),
            octave: DEFAULT_OCTAVE, // Default to 4th octave (C4-B4)
            previous_shift_sustain_active: false,
            shift_sustain_active: false,
            external_sustain_active: false,
        }
    }

    pub fn external_note_on(&mut self, note: Note) {
        let note_value = u8::from(note) as usize;
        debug_assert!(note_value < 128, "MIDI note value must be < 128");
        self.external_pressed_keys.set(note_value, true);
        // If this note was sustained, remove it from sustained set since it's now actively pressed
        self.external_sustained_keys.set(note_value, false);
    }

    pub fn external_note_off(&mut self, note: Note) {
        let note_value = u8::from(note) as usize;
        debug_assert!(note_value < 128, "MIDI note value must be < 128");

        if self.is_sustain_active() {
            // If sustain is active, move the key to sustained set instead of turning it off
            self.external_sustained_keys.set(note_value, true);
        } else {
            // Normal note off - turn off the key immediately
            self.external_pressed_keys.set(note_value, false);
        }
    }

    /// Set external sustain pedal state (from MIDI input)
    pub fn set_external_sustain(&mut self, active: bool) {
        self.external_sustain_active = active;
        if !active {
            // When sustain is released, clear all sustained external keys
            self.handle_sustain_release_for_external_keys();
        }
    }

    /// Check if sustain is currently active (either from Shift key or MIDI)
    /// Check if sustain is currently active (either from Shift key or MIDI)
    pub fn is_sustain_active(&self) -> bool {
        self.shift_sustain_active || self.external_sustain_active
    }

    pub fn show(&mut self, ui: &mut Ui) -> (Vec<Action>, Rect) {
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

        // Process all pointer events (touch and mouse)

        // Handle touch events
        let mut has_active_touches = false;
        ui.input(|i| {
            for event in &i.events {
                if let Event::Touch { id, phase, pos, .. } = event {
                    has_active_touches = true;
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

        // Handle mouse interactions only if there are no active touches
        // This prevents mouse events from interfering with multitouch on touch devices
        if !has_active_touches {
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
        }

        // Update current shift state
        self.shift_sustain_active = shift_pressed;

        // Generate actions based on key state changes
        self.generate_actions_for_all_keys(&mut actions);

        // Check for shift key state change specifically
        if shift_pressed != self.previous_shift_sustain_active {
            actions.push(Action::SustainPedal(shift_pressed));
            self.previous_shift_sustain_active = shift_pressed;

            if !shift_pressed {
                // Handle sustain pedal release for external keys
                self.handle_sustain_release_for_external_keys();
            }
        }

        // Update previous pointer keys for the next frame
        self.previous_pressed_keys.fill(false);
        for &note in self.pointers_holding_key.keys() {
            if !self.pointers_holding_key[&note].is_empty() {
                let semitone = Semitone::from_note(note);
                self.previous_pressed_keys.set(semitone.as_index(), true);
            }
        }

        // Render white keys first (so black keys appear on top)
        for semitone in Semitone::white_keys() {
            self.render_key(semitone, ui, &painter, keys_rect);
        }

        // Render black keys on top
        for semitone in Semitone::black_keys() {
            self.render_key(semitone, ui, &painter, keys_rect);
        }

        (actions, keys_rect)
    }

    /// All keys currently held in some way, from gui or from midi, actively pressed or sustained
    pub fn held_keys(&self) -> KeySet {
        let mut keys = self.pressed_keys();

        // TODO: is all this iterating expensive? should we cache?
        // Add sustained GUI keys
        for sustained_key in self.sustained_keys.iter_ones() {
            keys.set(sustained_key, true);
        }

        // Add currently active external keys
        for external_key in self.external_pressed_keys.iter_ones() {
            keys.set(external_key % 12, true);
        }

        // Add sustained external keys
        for sustained_key in self.external_sustained_keys.iter_ones() {
            keys.set(sustained_key % 12, true);
        }

        keys
    }

    /// Get keys currently pressed via GUI pointers (computed from pointers_holding_key)
    fn pressed_keys(&self) -> KeySet {
        let mut keys = KeySet::default();
        for (&note, pointers) in &self.pointers_holding_key {
            if !pointers.is_empty() {
                let semitone = Semitone::from_note(note);
                keys.set(semitone.as_index(), true);
            }
        }
        keys
    }

    pub fn selected_chord_name(&self) -> Option<String> {
        // AI generated. But seems mostly sensible
        let mut selected_semitones: Vec<usize> = self.held_keys().iter_ones().collect();
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

    /// Generate actions for all keys based on state changes.
    ///
    /// This method checks each key for state changes (pressed/released) and generates
    /// the appropriate actions. It also handles key selection logic including sustain pedal behavior.
    fn generate_actions_for_all_keys(&mut self, actions: &mut Vec<Action>) {
        for semitone in Semitone::iter() {
            let note = semitone.to_note_in_octave(self.octave);

            // Get active pointers for this key from our local state
            let is_pressed = self
                .pointers_holding_key
                .get(&note)
                .is_some_and(|pointers| !pointers.is_empty());
            let was_pressed = self.previous_pressed_keys[semitone.as_index()];

            if is_pressed && !was_pressed {
                actions.push(Action::Pressed(note));
                // If this key was sustained, remove it from sustained set since it's now actively pressed
                self.sustained_keys.set(semitone.as_index(), false);
            } else if !is_pressed && was_pressed {
                if self.is_sustain_active() {
                    // If sustain is active, move to sustained but don't generate Action::Released
                    self.sustained_keys.set(semitone.as_index(), true);
                } else {
                    // If sustain is not active, generate release action and clear sustained
                    actions.push(Action::Released(note));
                    self.sustained_keys.set(semitone.as_index(), false);
                }
            }
        }

        if !self.shift_sustain_active && self.previous_shift_sustain_active {
            for semitone in Semitone::iter() {
                let note = semitone.to_note_in_octave(self.octave);
                let is_currently_pressed = self
                    .pointers_holding_key
                    .get(&note)
                    .is_some_and(|pointers| !pointers.is_empty());

                // Clear sustained selection if key is not currently being pressed
                if !is_currently_pressed && self.sustained_keys[semitone.as_index()] {
                    self.sustained_keys.set(semitone.as_index(), false);
                    actions.push(Action::Released(note));
                }
            }
        }
    }

    /// Render a single piano key (pure rendering, no action generation).
    fn render_key(
        &mut self,
        semitone: Semitone,
        ui: &mut Ui,
        painter: &egui::Painter,
        keys_rect: Rect,
    ) {
        let note = semitone.to_note_in_octave(self.octave);
        let key_rect = key_rect_for_semitone(semitone, keys_rect);

        // Allocate space for the key (needed for proper UI layout)
        ui.allocate_rect(key_rect, Sense::click_and_drag());

        let is_pressed = self
            .pointers_holding_key
            .get(&note)
            .is_some_and(|pointers| !pointers.is_empty());
        let selected = is_pressed; // pressed_keys is now computed from pointers_holding_key
        let sustained_selected = self.sustained_keys[semitone.as_index()];
        // Check all octaves for this semitone in external key sets
        let mut external_selected = false;
        let mut sustained_external = false;
        let semitone_index = semitone.as_index();
        // MIDI notes range from 0 to 127
        for midi_note in (semitone_index..128).step_by(12) {
            if self.external_pressed_keys[midi_note] {
                external_selected = true;
            }
            if self.external_sustained_keys[midi_note] {
                sustained_external = true;
            }
            // Early exit if both are true
            if external_selected && sustained_external {
                break;
            }
        }

        let key_fill = if selected {
            // Currently pressed via GUI
            theme::pressed_key()
        } else if sustained_selected {
            // Sustained GUI keys (were pressed while sustain was active, now released)
            theme::sustained_key()
        } else if external_selected {
            // Currently pressed via external MIDI
            theme::external_key()
        } else if sustained_external {
            // Sustained external keys (were pressed via MIDI while sustain was active, now released)
            theme::external_sustained_key()
        } else if is_pressed {
            // Show actively pressed keys even when sustain is off
            theme::pressed_key()
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
                egui::Stroke::new(2.0, theme::pressed_key()),
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
        for semitone in Semitone::black_keys() {
            let key_rect = key_rect_for_semitone(semitone, keys_rect);
            if key_rect.contains(pos) {
                return Some(semitone.to_note_in_octave(self.octave));
            }
        }

        // If not on a black key, check white keys
        for semitone in Semitone::white_keys() {
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

    /// Handle sustain pedal release for external (MIDI) keys
    /// Clear all sustained external keys when sustain is released
    fn handle_sustain_release_for_external_keys(&mut self) {
        // Clear all sustained external keys - they should no longer be sustained
        self.external_sustained_keys.fill(false);
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
