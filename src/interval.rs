use num_rational::Rational32;
use num_traits::ToPrimitive;
use std::fmt::{Display, Formatter, Result};
use std::ops::Div;

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
        const RATIO_POWER_BASE: f32 = 2.0;
        const SEMITONES_IN_OCTAVE: f32 = 12.0;
        RATIO_POWER_BASE.powf(self.semitones() as f32 / SEMITONES_IN_OCTAVE)
    }

    /// Returns the difference in cents between just intonation and equal temperament
    /// Positive values mean just intonation is sharper than equal temperament
    pub fn tempered_just_error_cents(&self) -> f32 {
        const CENTS_IN_OCTAVE: f32 = 1200.0;
        const RATIO_POWER_BASE: f32 = 2.0;
        let just_cents =
            CENTS_IN_OCTAVE * (self.just_ratio().to_f32().unwrap().ln() / RATIO_POWER_BASE.ln());
        const CENTS_IN_SEMITONE: f32 = 100.0;
        let tempered_cents = CENTS_IN_SEMITONE * self.semitones() as f32;
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
        // AI generated
        // TODO: perhaps this is overcomplicated. better to just use the base_dissonance directly?

        // Factor 1: Ratio complexity - simpler ratios are less dissonant
        let just = self.just_ratio();
        let numer = just.numer().abs() as f32;
        let denom = just.denom().abs() as f32;
        const COMPLEXITY_FACTOR: f32 = 0.7;
        const COMPLEXITY_DENOMINATOR_OFFSET: f32 = 1.0;
        let complexity = 1.0
            - 1.0 / (COMPLEXITY_FACTOR * (numer + denom - COMPLEXITY_DENOMINATOR_OFFSET)).sqrt();

        // Factor 2: Just/tempered error in cents
        const CENTS_ERROR_NORMALIZATION_FACTOR: f32 = 20.0;
        let cents_error =
            self.tempered_just_error_cents().abs() / CENTS_ERROR_NORMALIZATION_FACTOR; // Normalize
        const MAX_ERROR_FACTOR: f32 = 1.0;
        let error_factor = cents_error.min(MAX_ERROR_FACTOR);

        // Factor 3: Perceptual/cultural base dissonance
        const UNISON_DISSONANCE: f32 = 0.00;
        const OCTAVE_DISSONANCE: f32 = 0.05;
        const PERFECT_FIFTH_DISSONANCE: f32 = 0.10;
        const PERFECT_FOURTH_DISSONANCE: f32 = 0.15;
        const MAJOR_THIRD_DISSONANCE: f32 = 0.25;
        const MINOR_THIRD_DISSONANCE: f32 = 0.30;
        const MAJOR_SIXTH_DISSONANCE: f32 = 0.35;
        const MINOR_SIXTH_DISSONANCE: f32 = 0.40;
        const MAJOR_SECOND_DISSONANCE: f32 = 0.60;
        const MINOR_SEVENTH_DISSONANCE: f32 = 0.65;
        const MAJOR_SEVENTH_DISSONANCE: f32 = 0.75;
        const MINOR_SECOND_DISSONANCE: f32 = 0.85;
        const TRITONE_DISSONANCE: f32 = 0.90;
        let base_dissonance = match self {
            Self::Unison => UNISON_DISSONANCE,
            Self::Octave => OCTAVE_DISSONANCE,
            Self::PerfectFifth => PERFECT_FIFTH_DISSONANCE,
            Self::PerfectFourth => PERFECT_FOURTH_DISSONANCE,
            Self::MajorThird => MAJOR_THIRD_DISSONANCE,
            Self::MinorThird => MINOR_THIRD_DISSONANCE,
            Self::MajorSixth => MAJOR_SIXTH_DISSONANCE,
            Self::MinorSixth => MINOR_SIXTH_DISSONANCE,
            Self::MajorSecond => MAJOR_SECOND_DISSONANCE,
            Self::MinorSeventh => MINOR_SEVENTH_DISSONANCE,
            Self::MajorSeventh => MAJOR_SEVENTH_DISSONANCE,
            Self::MinorSecond => MINOR_SECOND_DISSONANCE,
            Self::Tritone => TRITONE_DISSONANCE,
        };

        // Weighted combination of all factors
        const BASE_DISSONANCE_WEIGHT: f32 = 0.5;
        const COMPLEXITY_WEIGHT: f32 = 0.3;
        const ERROR_FACTOR_WEIGHT: f32 = 0.2;
        BASE_DISSONANCE_WEIGHT * base_dissonance
            + COMPLEXITY_WEIGHT * complexity
            + ERROR_FACTOR_WEIGHT * error_factor
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
        const SEMITONES_IN_OCTAVE: i8 = 12;
        let semitone_diff = (left_semitones - right_semitones).rem_euclid(SEMITONES_IN_OCTAVE) as u8;
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

        // Perfect intervals (should be close to 0 for some)
        const UNISON_CENTS_ERROR: f32 = 0.0;
        const TOLERANCE: f32 = 0.01;
        assert_approx_eq(
            Interval::Unison.tempered_just_error_cents(),
            UNISON_CENTS_ERROR,
            TOLERANCE,
        );
        const PERFECT_FIFTH_CENTS_ERROR: f32 = 1.96;
        assert_approx_eq(
            Interval::PerfectFifth.tempered_just_error_cents(),
            PERFECT_FIFTH_CENTS_ERROR,
            TOLERANCE,
        );
        const PERFECT_FOURTH_CENTS_ERROR: f32 = -1.96;
        assert_approx_eq(
            Interval::PerfectFourth.tempered_just_error_cents(),
            PERFECT_FOURTH_CENTS_ERROR,
            TOLERANCE,
        );
        const OCTAVE_CENTS_ERROR: f32 = 0.0;
        assert_approx_eq(
            Interval::Octave.tempered_just_error_cents(),
            OCTAVE_CENTS_ERROR,
            TOLERANCE,
        );

        // Major intervals
        const MAJOR_SECOND_CENTS_ERROR: f32 = 3.91;
        assert_approx_eq(
            Interval::MajorSecond.tempered_just_error_cents(),
            MAJOR_SECOND_CENTS_ERROR,
            TOLERANCE,
        );
        const MAJOR_THIRD_CENTS_ERROR: f32 = -13.69;
        assert_approx_eq(
            Interval::MajorThird.tempered_just_error_cents(),
            MAJOR_THIRD_CENTS_ERROR,
            TOLERANCE,
        );
        const MAJOR_SIXTH_CENTS_ERROR: f32 = -15.64;
        assert_approx_eq(
            Interval::MajorSixth.tempered_just_error_cents(),
            MAJOR_SIXTH_CENTS_ERROR,
            TOLERANCE,
        );
        const MAJOR_SEVENTH_CENTS_ERROR: f32 = -11.73;
        assert_approx_eq(
            Interval::MajorSeventh.tempered_just_error_cents(),
            MAJOR_SEVENTH_CENTS_ERROR,
            TOLERANCE,
        );

        // Minor intervals
        const MINOR_SECOND_CENTS_ERROR: f32 = 11.73;
        assert_approx_eq(
            Interval::MinorSecond.tempered_just_error_cents(),
            MINOR_SECOND_CENTS_ERROR,
            TOLERANCE,
        );
        const MINOR_THIRD_CENTS_ERROR: f32 = 15.64;
        assert_approx_eq(
            Interval::MinorThird.tempered_just_error_cents(),
            MINOR_THIRD_CENTS_ERROR,
            TOLERANCE,
        );
        const MINOR_SIXTH_CENTS_ERROR: f32 = 13.69;
        assert_approx_eq(
            Interval::MinorSixth.tempered_just_error_cents(),
            MINOR_SIXTH_CENTS_ERROR,
            TOLERANCE,
        );
        const MINOR_SEVENTH_CENTS_ERROR: f32 = 17.60;
        assert_approx_eq(
            Interval::MinorSeventh.tempered_just_error_cents(),
            MINOR_SEVENTH_CENTS_ERROR,
            TOLERANCE,
        );

        // Tritone
        const TRITONE_CENTS_ERROR: f32 = -9.77;
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
            "Expected {expected}, got {actual} (difference: {})",
            (actual - expected).abs()
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
