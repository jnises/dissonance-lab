use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub const WORKLET_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tuning {
    Equal,
    Just { reference: u8 },
}

pub fn just_ratio(semitones: u8) -> (i32, i32) {
    match semitones {
        0 => (1, 1),
        1 => (16, 15),
        2 => (9, 8),
        3 => (6, 5),
        4 => (5, 4),
        5 => (4, 3),
        6 => (45, 32),
        7 => (3, 2),
        8 => (8, 5),
        9 => (5, 3),
        10 => (9, 5),
        11 => (15, 8),
        12 => (2, 1),
        _ => panic!("Invalid interval: {semitones} semitones"),
    }
}

/// Ratio of a MIDI note's frequency to the reference's frequency, including octaves.
pub fn just_note_ratio(note: u8, reference: u8) -> (i32, i32) {
    const SEMITONES_PER_OCTAVE: i16 = 12;
    debug_assert!(note < 128 && reference < 128);
    let interval = i16::from(note) - i16::from(reference);
    let octaves = interval.div_euclid(SEMITONES_PER_OCTAVE);
    let semitones = interval.rem_euclid(SEMITONES_PER_OCTAVE) as u8;
    let (numerator, denominator) = just_ratio(semitones);
    (
        numerator << octaves.max(0),
        denominator << (-octaves).max(0),
    )
}

impl Tuning {
    pub fn frequency(self, note: u8) -> f32 {
        const SEMITONES_PER_OCTAVE: i16 = 12;
        const A4: u8 = 69;
        const A4_HZ: f32 = 440.0;

        let equal_frequency = |note| {
            A4_HZ
                * 2.0_f32.powf(
                    f32::from(i16::from(note) - i16::from(A4)) / f32::from(SEMITONES_PER_OCTAVE),
                )
        };

        match self {
            Self::Equal => equal_frequency(note),
            Self::Just { reference } => {
                let (numerator, denominator) = just_note_ratio(note, reference);
                equal_frequency(reference) * numerator as f32 / denominator as f32
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ToWorkletMessage {
    NoteOn {
        note: u8,
        velocity: u8,
        #[serde(default)]
        frequency: Option<f32>,
    },
    NoteOff {
        note: u8,
    },
    SustainPedal {
        active: bool,
    },
    Retune {
        tuning: Tuning,
    },
}

impl From<ToWorkletMessage> for JsValue {
    fn from(value: ToWorkletMessage) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FromWorkletMessage {
    InitComplete {
        #[serde(default)]
        protocol_version: u32,
    },
    InitError {
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{Tuning, just_note_ratio};

    #[test]
    fn just_tuning_preserves_reference_and_extends_across_octaves() {
        let just = Tuning::Just { reference: 60 };
        let root = Tuning::Equal.frequency(60);
        assert_eq!(just.frequency(60), root);
        assert_eq!(just.frequency(64), root * 1.25);
        assert!((just.frequency(55) - root * 0.75).abs() < 0.001);
        assert!((just.frequency(79) - root * 3.0).abs() < 0.001);
    }

    #[test]
    fn just_note_ratio_keeps_the_played_octave_and_direction() {
        let (numerator, denominator) = just_note_ratio(60, 64);
        assert_eq!(numerator * 5, denominator * 4);
        assert_eq!(just_note_ratio(60, 48), (2, 1));
        assert_eq!(just_note_ratio(48, 60), (1, 2));
    }
}
