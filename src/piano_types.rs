use bitvec::{BitArr, order::Msb0};
use wmidi::Note;

/// A semitone value within an octave (0-11)
/// Represents the 12 chromatic pitches: C, C#, D, D#, E, F, F#, G, G#, A, A#, B
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Semitone(u8);

impl Semitone {
    /// Named constants for all 12 semitones
    #[allow(dead_code)] // These constants are provided for convenience and future use
    pub const C: Self = Self(0);
    #[allow(dead_code)]
    pub const C_SHARP: Self = Self(1);
    #[allow(dead_code)]
    pub const D: Self = Self(2);
    #[allow(dead_code)]
    pub const D_SHARP: Self = Self(3);
    #[allow(dead_code)]
    pub const E: Self = Self(4);
    #[allow(dead_code)]
    pub const F: Self = Self(5);
    #[allow(dead_code)]
    pub const F_SHARP: Self = Self(6);
    #[allow(dead_code)]
    pub const G: Self = Self(7);
    #[allow(dead_code)]
    pub const G_SHARP: Self = Self(8);
    #[allow(dead_code)]
    pub const A: Self = Self(9);
    #[allow(dead_code)]
    pub const A_SHARP: Self = Self(10);
    #[allow(dead_code)]
    pub const B: Self = Self(11);

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

    /// Create an iterator over all 12 semitones in chromatic order (C, C#, D, D#, E, F, F#, G, G#, A, A#, B)
    pub fn iter() -> impl Iterator<Item = Semitone> {
        (0..12).map(Semitone::new)
    }

    /// Create an iterator over white key semitones (C, D, E, F, G, A, B)
    pub fn white_keys() -> impl Iterator<Item = Semitone> {
        [0, 2, 4, 5, 7, 9, 11].into_iter().map(Semitone::new)
    }

    /// Create an iterator over black key semitones (C#, D#, F#, G#, A#)
    pub fn black_keys() -> impl Iterator<Item = Semitone> {
        [1, 3, 6, 8, 10].into_iter().map(Semitone::new)
    }
}

/// Identifies a pointer (mouse or touch) in the GUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerId {
    Mouse,
    Touch(u64),
}

/// A set of keys within a single octave (12 semitones)
/// Used for tracking which keys are pressed in the piano GUI
pub type KeySet = BitArr!(for 12, in u16, Msb0);

/// A set of external MIDI keys across all octaves (128 notes)
/// Used for tracking which MIDI keys are pressed from external sources
pub type ExternalKeySet = BitArr!(for 128, in u32, Msb0);
