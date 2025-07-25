use num_rational::Rational32;
use num_traits::ToPrimitive;
use std::fmt::{Display, Formatter, Result};
use std::ops::Div;

// Musical constants
const OCTAVE_RATIO: f32 = 2.0; // The octave ratio - frequency doubles every octave in equal temperament
const SEMITONES_PER_OCTAVE: f32 = 12.0;
const SEMITONES_PER_OCTAVE_I8: i8 = 12;

/// Musical intervals that define the distance between two notes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Unison,
    MinorSecond,
    MajorSecond,
    MinorThird,
    MajorThird,
    PerfectFourth,
    Tritone,
    PerfectFifth,
    MinorSixth,
    MajorSixth,
    MinorSeventh,
    MajorSeventh,
    Octave,
}

impl Interval {
    /// only handles one octave
    pub fn from_semitone_interval(semitone_interval: u8) -> Self {
        match semitone_interval {
            0 => Self::Unison,
            1 => Self::MinorSecond,
            2 => Self::MajorSecond,
            3 => Self::MinorThird,
            4 => Self::MajorThird,
            5 => Self::PerfectFourth,
            6 => Self::Tritone,
            7 => Self::PerfectFifth,
            8 => Self::MinorSixth,
            9 => Self::MajorSixth,
            10 => Self::MinorSeventh,
            11 => Self::MajorSeventh,
            12 => Self::Octave,
            _ => panic!("Invalid semitone interval: {semitone_interval}"),
        }
    }

    pub fn from_semitone_wrapping(semitone_interval: i8) -> Self {
        Self::from_semitone_interval(semitone_interval.rem_euclid(12) as u8)
    }

    /// Returns the just intonation ratio for this interval
    pub fn just_ratio(&self) -> Rational32 {
        match self {
            Self::Unison => Rational32::new(1, 1),
            Self::MinorSecond => Rational32::new(16, 15),
            Self::MajorSecond => Rational32::new(9, 8),
            Self::MinorThird => Rational32::new(6, 5),
            Self::MajorThird => Rational32::new(5, 4),
            Self::PerfectFourth => Rational32::new(4, 3),
            Self::Tritone => Rational32::new(45, 32),
            Self::PerfectFifth => Rational32::new(3, 2),
            Self::MinorSixth => Rational32::new(8, 5),
            Self::MajorSixth => Rational32::new(5, 3),
            Self::MinorSeventh => Rational32::new(9, 5),
            Self::MajorSeventh => Rational32::new(15, 8),
            Self::Octave => Rational32::new(2, 1),
        }
    }

    /// Returns the equal temperament ratio for this interval
    #[allow(dead_code)]
    pub fn tempered_ratio(&self) -> f32 {
        OCTAVE_RATIO.powf(self.semitones() as f32 / SEMITONES_PER_OCTAVE)
    }

    /// Returns the difference in cents between just intonation and equal temperament
    /// Positive values mean just intonation is sharper than equal temperament
    pub fn tempered_just_error_cents(&self) -> f32 {
        const CENTS_PER_OCTAVE: f32 = 1200.0;
        const CENTS_PER_SEMITONE: f32 = 100.0;

        let just_cents =
            CENTS_PER_OCTAVE * (self.just_ratio().to_f32().unwrap().ln() / OCTAVE_RATIO.ln());
        let tempered_cents = CENTS_PER_SEMITONE * self.semitones() as f32;
        just_cents - tempered_cents
    }

    /// Get the number of semitones in this interval
    fn semitones(&self) -> u8 {
        match self {
            Self::Unison => 0,
            Self::MinorSecond => 1,
            Self::MajorSecond => 2,
            Self::MinorThird => 3,
            Self::MajorThird => 4,
            Self::PerfectFourth => 5,
            Self::Tritone => 6,
            Self::PerfectFifth => 7,
            Self::MinorSixth => 8,
            Self::MajorSixth => 9,
            Self::MinorSeventh => 10,
            Self::MajorSeventh => 11,
            Self::Octave => 12,
        }
    }

    pub fn dissonance(&self) -> f32 {
        // This is appropriate for a tempered piano where equal temperament is used
        match self {
            // Perfect consonances (low dissonance)
            Self::Unison => 0.00,
            Self::Octave => 0.05,
            Self::PerfectFifth => 0.10,
            Self::PerfectFourth => 0.15,
            // Imperfect consonances (moderate dissonance)
            Self::MajorThird => 0.25,
            Self::MinorThird => 0.30,
            Self::MajorSixth => 0.35,
            Self::MinorSixth => 0.40,
            // Dissonances (high dissonance)
            Self::MajorSecond => 0.60,
            Self::MinorSeventh => 0.65,
            Self::MajorSeventh => 0.75,
            Self::MinorSecond => 0.85,
            Self::Tritone => 0.90,
        }
    }
}

impl Div for Interval {
    type Output = Self;

    /// Calculates the interval between two intervals
    ///
    /// For example, a perfect fifth divided by a major third gives a minor third
    /// (because a minor third interval separates a major third from a perfect fifth)
    fn div(self, rhs: Self) -> Self::Output {
        let left_semitones = self.semitones() as i8;
        let right_semitones = rhs.semitones() as i8;
        // we are subtracting semitones which in effect is the log of the interval
        #[expect(clippy::suspicious_arithmetic_impl)]
        let semitone_diff =
            (left_semitones - right_semitones).rem_euclid(SEMITONES_PER_OCTAVE_I8) as u8;
        Self::from_semitone_interval(semitone_diff)
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::Unison => "unison",
            Self::MinorSecond => "minor second",
            Self::MajorSecond => "major second",
            Self::MinorThird => "minor third",
            Self::MajorThird => "major third",
            Self::PerfectFourth => "perfect fourth",
            Self::Tritone => "tritone",
            Self::PerfectFifth => "perfect fifth",
            Self::MinorSixth => "minor sixth",
            Self::MajorSixth => "major sixth",
            Self::MinorSeventh => "minor seventh",
            Self::MajorSeventh => "major seventh",
            Self::Octave => "octave",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_just_tempered_error_cents() {
        // Test cases for different intervals
        // Expected values calculated based on the formula:
        // 1200 * log2(just_ratio) - 100 * semitones

        const TOLERANCE: f32 = 0.01;

        // Perfect intervals (should be close to 0 for some)
        const UNISON_CENTS_ERROR: f32 = 0.0;
        const OCTAVE_CENTS_ERROR: f32 = 0.0;
        const PERFECT_FIFTH_CENTS_ERROR: f32 = 1.96;
        const PERFECT_FOURTH_CENTS_ERROR: f32 = -1.96;
        // Major intervals
        const MAJOR_SECOND_CENTS_ERROR: f32 = 3.91;
        const MAJOR_THIRD_CENTS_ERROR: f32 = -13.69;
        const MAJOR_SIXTH_CENTS_ERROR: f32 = -15.64;
        const MAJOR_SEVENTH_CENTS_ERROR: f32 = -11.73;

        // Minor intervals
        const MINOR_SECOND_CENTS_ERROR: f32 = 11.73;
        const MINOR_THIRD_CENTS_ERROR: f32 = 15.64;
        const MINOR_SIXTH_CENTS_ERROR: f32 = 13.69;
        const MINOR_SEVENTH_CENTS_ERROR: f32 = 17.60;

        // Tritone
        const TRITONE_CENTS_ERROR: f32 = -9.77;

        // Test perfect intervals
        assert_approx_eq(
            Interval::Unison.tempered_just_error_cents(),
            UNISON_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::Octave.tempered_just_error_cents(),
            OCTAVE_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::PerfectFifth.tempered_just_error_cents(),
            PERFECT_FIFTH_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::PerfectFourth.tempered_just_error_cents(),
            PERFECT_FOURTH_CENTS_ERROR,
            TOLERANCE,
        );

        // Test major intervals
        assert_approx_eq(
            Interval::MajorSecond.tempered_just_error_cents(),
            MAJOR_SECOND_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MajorThird.tempered_just_error_cents(),
            MAJOR_THIRD_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MajorSixth.tempered_just_error_cents(),
            MAJOR_SIXTH_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MajorSeventh.tempered_just_error_cents(),
            MAJOR_SEVENTH_CENTS_ERROR,
            TOLERANCE,
        );

        // Test minor intervals
        assert_approx_eq(
            Interval::MinorSecond.tempered_just_error_cents(),
            MINOR_SECOND_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MinorThird.tempered_just_error_cents(),
            MINOR_THIRD_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MinorSixth.tempered_just_error_cents(),
            MINOR_SIXTH_CENTS_ERROR,
            TOLERANCE,
        );
        assert_approx_eq(
            Interval::MinorSeventh.tempered_just_error_cents(),
            MINOR_SEVENTH_CENTS_ERROR,
            TOLERANCE,
        );

        // Test tritone
        assert_approx_eq(
            Interval::Tritone.tempered_just_error_cents(),
            TRITONE_CENTS_ERROR,
            TOLERANCE,
        );
    }

    // Helper function to compare floating point values with tolerance
    fn assert_approx_eq(actual: f32, expected: f32, epsilon: f32) {
        assert!(
            (actual - expected).abs() < epsilon,
            "Expected {expected}, got {actual} (difference: {difference})",
            difference = (actual - expected).abs()
        );
    }

    #[test]
    fn test_interval_dissonance_ordering() {
        // ordered according to dissonance
        let intervals = [
            Interval::Unison,
            Interval::Octave,
            Interval::PerfectFifth,
            Interval::PerfectFourth,
            Interval::MajorThird,
            Interval::MinorThird,
            Interval::MajorSixth,
            Interval::MinorSixth,
            Interval::MajorSecond,
            Interval::MinorSeventh,
            Interval::MajorSeventh,
            Interval::MinorSecond,
            Interval::Tritone,
        ];

        let dissonances: Vec<(Interval, f32)> =
            intervals.iter().map(|i| (*i, i.dissonance())).collect();

        // Check that dissonance increases (or stays the same) as we go through the array
        for window in dissonances.windows(2) {
            let (current_interval, current_dissonance) = window[0];
            let (next_interval, next_dissonance) = window[1];

            assert!(
                current_dissonance <= next_dissonance,
                "Expected {current_interval} (dissonance: {current_dissonance:.2}) to be less dissonant than {next_interval} (dissonance: {next_dissonance:.2})"
            );
        }
    }

    #[test]
    fn test_most_consonant_dissonant_intervals() {
        // Check that unison is the least dissonant
        assert!(Interval::Unison.dissonance() < Interval::Octave.dissonance());

        // Check that perfect fifth is the least dissonant non-trivial interval
        let non_unison_intervals = [
            Interval::Octave,
            Interval::PerfectFifth,
            Interval::PerfectFourth,
            Interval::MajorThird,
            Interval::MinorThird,
            Interval::MajorSixth,
            Interval::MinorSixth,
            Interval::MajorSecond,
            Interval::MinorSeventh,
            Interval::MajorSeventh,
            Interval::MinorSecond,
            Interval::Tritone,
        ];

        let fifth_dissonance = Interval::PerfectFifth.dissonance();
        for interval in non_unison_intervals {
            if interval == Interval::PerfectFifth {
                continue;
            }
            if interval == Interval::Octave {
                assert!(
                    fifth_dissonance > interval.dissonance(),
                    "Perfect fifth should be more dissonant than octave"
                );
            } else {
                assert!(
                    fifth_dissonance < interval.dissonance(),
                    "Perfect fifth should be less dissonant than {interval}"
                );
            }
        }

        // Check that tritone is the most dissonant
        let tritone_dissonance = Interval::Tritone.dissonance();
        for interval in [
            Interval::Unison,
            Interval::Octave,
            Interval::PerfectFifth,
            Interval::PerfectFourth,
            Interval::MajorThird,
            Interval::MinorThird,
            Interval::MajorSixth,
            Interval::MinorSixth,
            Interval::MajorSecond,
            Interval::MinorSeventh,
            Interval::MajorSeventh,
            Interval::MinorSecond,
        ] {
            assert!(
                tritone_dissonance > interval.dissonance(),
                "Tritone should be more dissonant than {interval}"
            );
        }
    }
}
