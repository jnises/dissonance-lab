use num_rational::Rational32;
use num_traits::ToPrimitive;

pub fn is_key_black(note: usize) -> bool {
    [
        false, true, false, true, false, false, true, false, true, false, true, false,
    ][note % 12]
}

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
    pub fn tempered_ratio(&self) -> f32 {
        2.0_f32.powf(self.semitones() as f32 / 12.0)
    }

    /// Returns the difference in cents between just intonation and equal temperament
    /// Positive values mean just intonation is higher than equal temperament
    pub fn just_tempered_error_cents(&self) -> f32 {
        // Convert the frequency ratios to cents
        let just_cents = 1200.0 * (self.just_ratio().to_f32().unwrap().ln() / 2.0_f32.ln());
        let tempered_cents = 100.0 * self.semitones() as f32;

        // Return the difference
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

    /// dissonance based on just interval and how large the just/tempered error is  
    pub fn compound_dissonance(&self) -> f32 {
        // TODO: calculate this directly instead
        match self {
            Interval::Unison => 0.0,
            Interval::MinorSecond => 0.7852,
            Interval::MajorSecond => 0.6117,
            Interval::MinorThird => 0.3969,
            Interval::MajorThird => 0.3411,
            Interval::PerfectFourth => 0.2059,
            Interval::Tritone => 0.8793,
            Interval::PerfectFifth => 0.1059,
            Interval::MinorSixth => 0.4911,
            Interval::MajorSixth => 0.4469,
            Interval::MinorSeventh => 0.7617,
            Interval::MajorSeventh => 0.8352,
            Interval::Octave => 0.0,
        }
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
        assert_approx_eq(Interval::Unison.just_tempered_error_cents(), 0.0, 0.01);
        assert_approx_eq(
            Interval::PerfectFifth.just_tempered_error_cents(),
            1.96,
            0.01,
        );
        assert_approx_eq(
            Interval::PerfectFourth.just_tempered_error_cents(),
            -1.96,
            0.01,
        );
        assert_approx_eq(Interval::Octave.just_tempered_error_cents(), 0.0, 0.01);

        // Major intervals
        assert_approx_eq(
            Interval::MajorSecond.just_tempered_error_cents(),
            3.91,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorThird.just_tempered_error_cents(),
            -13.69,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorSixth.just_tempered_error_cents(),
            -15.64,
            0.01,
        );
        assert_approx_eq(
            Interval::MajorSeventh.just_tempered_error_cents(),
            -11.73,
            0.01,
        );

        // Minor intervals
        assert_approx_eq(
            Interval::MinorSecond.just_tempered_error_cents(),
            11.73,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorThird.just_tempered_error_cents(),
            15.64,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorSixth.just_tempered_error_cents(),
            13.69,
            0.01,
        );
        assert_approx_eq(
            Interval::MinorSeventh.just_tempered_error_cents(),
            -17.60,
            0.01,
        );

        // Tritone
        assert_approx_eq(Interval::Tritone.just_tempered_error_cents(), -9.77, 0.01);
    }

    // Helper function to compare floating point values with tolerance
    fn assert_approx_eq(actual: f32, expected: f32, epsilon: f32) {
        assert!(
            (actual - expected).abs() < epsilon,
            "Expected {expected}, got {actual} (difference: {})",
            (actual - expected).abs()
        );
    }
}
