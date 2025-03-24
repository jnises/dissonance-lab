use itertools::Itertools as _;
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
        2.0_f32.powf(self.semitones() as f32 / 12.0)
    }

    /// Returns the difference in cents between just intonation and equal temperament
    /// Positive values mean just intonation is sharper than equal temperament
    pub fn tempered_just_error_cents(&self) -> f32 {
        let just_cents = 1200.0 * (self.just_ratio().to_f32().unwrap().ln() / 2.0_f32.ln());
        let tempered_cents = 100.0 * self.semitones() as f32;
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

    /// Dissonance of an interval between 0 and 1
    /// Since we only care about a single octave we also take the inversion into account
    pub fn dissonance(&self) -> f32 {
        // AI generated
        // Calculate dissonance based on frequency ratio complexity and tuning error
        // Lower values = more consonant
        let semitones = self.semitones();
        assert!(semitones < 12);
        // Calculate dissonance for the given interval
        let ratio = self.just_ratio();
        let numerator = ratio.numer();
        let denominator = ratio.denom();
        
        // Base dissonance from ratio complexity (simpler ratios are more consonant)
        let complexity = (numerator * denominator) as f32;
        let base_dissonance = (complexity.ln() / 20.0).min(1.0);
        
        // Effect of tuning error (increases dissonance)
        let tuning_error = self.tempered_just_error_cents().abs() / 20.0;
        let tuning_factor = 0.3 * (tuning_error / 15.0);
        
        // Consider inversion by comparing with inverted interval's dissonance
        let inverted = Interval::from_semitone_interval(12 - semitones);
        let inverted_ratio = inverted.just_ratio();
        let inverted_complexity = (inverted_ratio.numer() * inverted_ratio.denom()) as f32;
        let inverted_dissonance = (inverted_complexity.ln() / 20.0).min(1.0);
        let inverted_tuning_error = inverted.tempered_just_error_cents().abs() / 20.0;
        let inverted_tuning_factor = 0.3 * (inverted_tuning_error / 15.0);

        // Use the more consonant of the interval or its inversion
        let direct = base_dissonance + tuning_factor;
        let inverted = inverted_dissonance + inverted_tuning_factor;
        direct.min(inverted)
    }

    /// Calculates the average dissonance between all the intervals in a chord
    pub fn chord_dissonance(intervals: impl Iterator<Item = Self>) -> f32 {
        // TODO: is there a better way to calculate the dissonance of a chord?
        let (sum, count) = intervals
            .combinations(2)
            .map(|pair| {
                let between_interval = pair[1] / pair[0];
                between_interval.dissonance()
            })
            .fold((0.0, 0), |acc, dissonance| (acc.0 + dissonance, acc.1 + 1));
        sum / count as f32
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
        let semitone_diff = (left_semitones - right_semitones).rem_euclid(12) as u8;
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
        write!(f, "{}", s)
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
        assert_approx_eq(Interval::Unison.tempered_just_error_cents(), 0.0, 0.01);
        assert_approx_eq(
            Interval::PerfectFifth.tempered_just_error_cents(),
            1.96,
            0.01,
        );
        assert_approx_eq(
            Interval::PerfectFourth.tempered_just_error_cents(),
            -1.96,
            0.01,
        );
        assert_approx_eq(Interval::Octave.tempered_just_error_cents(), 0.0, 0.01);

        // Major intervals
        assert_approx_eq(
            Interval::MajorSecond.tempered_just_error_cents(),
            3.91,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorThird.tempered_just_error_cents(),
            -13.69,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorSixth.tempered_just_error_cents(),
            -15.64,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorSeventh.tempered_just_error_cents(),
            -11.73,
            0.01,
        );

        // Minor intervals
        assert_approx_eq(
            Interval::MinorSecond.tempered_just_error_cents(),
            11.73,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorThird.tempered_just_error_cents(),
            15.64,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorSixth.tempered_just_error_cents(),
            13.69,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorSeventh.tempered_just_error_cents(),
            17.60,
            0.01,
        );

        // Tritone
        assert_approx_eq(Interval::Tritone.tempered_just_error_cents(), -9.77, 0.01);
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
