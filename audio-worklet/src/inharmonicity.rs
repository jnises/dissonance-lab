use std::f32::consts::PI;

/// Piano string inharmonicity model
///
/// Models the deviation of piano string overtones from perfect harmonics
/// due to string stiffness. Real piano strings produce partials that are
/// slightly sharp compared to integer multiples of the fundamental.
pub struct InharmonicityModel {
    /// Inharmonicity coefficient B for this string
    coefficient: f32,
}

impl InharmonicityModel {
    /// Create a new inharmonicity model for a piano string
    ///
    /// # Parameters
    /// - `string_diameter`: String diameter in meters
    /// - `string_length`: String length in meters
    /// - `string_tension`: String tension in Newtons
    pub fn new(string_diameter: f32, string_length: f32, string_tension: f32) -> Self {
        let coefficient = Self::calculate_inharmonicity_coefficient(
            string_diameter,
            string_length,
            string_tension,
        );

        Self { coefficient }
    }

    /// Create inharmonicity model from precomputed coefficient
    pub fn from_coefficient(coefficient: f32) -> Self {
        Self { coefficient }
    }

    /// Calculate the inharmonicity coefficient B
    ///
    /// Formula: B = (π³ * d⁴ * E) / (64 * T * L²)
    /// Where:
    /// - d = string diameter (m)
    /// - E = Young's modulus of steel (~200 GPa)
    /// - T = string tension (N)
    /// - L = string length (m)
    fn calculate_inharmonicity_coefficient(diameter: f32, length: f32, tension: f32) -> f32 {
        // Young's modulus of steel in Pa (200 GPa)
        const YOUNGS_MODULUS_STEEL: f32 = 2e11;

        let numerator = PI.powi(3) * diameter.powi(4) * YOUNGS_MODULUS_STEEL;
        let denominator = 64.0 * tension * length.powi(2);

        numerator / denominator
    }

    /// Calculate the frequency of the nth partial including inharmonicity
    ///
    /// Formula: f_n = n * f₀ * √(1 + B * n²)
    ///
    /// # Parameters
    /// - `fundamental_freq`: The fundamental frequency f₀
    /// - `partial_number`: The partial number n (1 for fundamental, 2 for first overtone, etc.)
    ///
    /// # Returns
    /// The inharmonic frequency of the nth partial
    pub fn partial_frequency(&self, fundamental_freq: f32, partial_number: u32) -> f32 {
        if partial_number == 1 {
            // The fundamental (first partial) is always exactly the fundamental frequency
            return fundamental_freq;
        }

        let n = partial_number as f32;
        let inharmonicity_factor = self.coefficient.mul_add(n * n, 1.0).sqrt();
        n * fundamental_freq * inharmonicity_factor
    }

    /// Get the inharmonicity coefficient B
    pub fn coefficient(&self) -> f32 {
        self.coefficient
    }
}

/// Piano string parameters for different register ranges
pub struct PianoStringParameters {
    /// String diameter in meters
    pub diameter: f32,
    /// String length in meters
    pub length: f32,
    /// String tension in Newtons
    pub tension: f32,
}

impl PianoStringParameters {
    /// Get approximate string parameters for a given MIDI note
    ///
    /// This uses simplified models based on typical grand piano construction.
    /// Real pianos have more complex string scaling with wound bass strings, etc.
    pub fn for_midi_note(midi_note: u8) -> Self {
        // Approximate parameters based on typical grand piano scaling
        // These are simplified - real pianos have more complex scaling laws

        // Piano range constants
        const MIDI_NOTE_MIN: f32 = 21.0; // A0
        const MIDI_NOTE_MAX: f32 = 108.0; // C8
        let note_ratio = (midi_note as f32 - MIDI_NOTE_MIN) / (MIDI_NOTE_MAX - MIDI_NOTE_MIN);

        // String length decreases linearly from bass to treble
        const LENGTH_BASS: f32 = 2.0; // Bass strings ~2m
        const LENGTH_SCALING_FACTOR: f32 = 0.95; // Treble strings ~0.1m (2.0 * (1.0 - 0.95) = 0.1m)
        let length = LENGTH_BASS * (1.0 - LENGTH_SCALING_FACTOR * note_ratio);

        // Bass strings are thicker AND wound strings have additional mass/stiffness
        // More realistic scaling: bass strings ~1.5mm, treble strings ~0.8mm
        const DIAMETER_SCALING_FACTOR: f32 = 0.47;
        const DIAMETER_CONVERSION: f32 = 0.001; // Convert mm to meters
        let base_diameter = (1.0 - DIAMETER_SCALING_FACTOR * note_ratio) * DIAMETER_CONVERSION;

        // Add effective stiffness from wound bass strings
        const BASS_REGISTER_THRESHOLD: f32 = 0.3; // Notes below this ratio are considered bass
        const WOUND_STRING_STIFFNESS_FACTOR: f32 = 1.5; // Wound strings behave effectively stiffer
        const PLAIN_STRING_FACTOR: f32 = 1.0;
        
        let winding_factor = if note_ratio < BASS_REGISTER_THRESHOLD {
            // Bass register
            WOUND_STRING_STIFFNESS_FACTOR
        } else {
            PLAIN_STRING_FACTOR
        };

        let diameter = base_diameter * winding_factor;

        // Tension scaling: bass strings have lower tension to avoid excessive force
        // Linear increase from bass to treble: 100N to 200N range
        const TENSION_BASS: f32 = 100.0; // Newtons
        const TENSION_RANGE: f32 = 100.0; // Newtons (200N - 100N)
        let tension = TENSION_BASS + TENSION_RANGE * note_ratio;

        Self {
            diameter,
            length,
            tension,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inharmonicity_coefficient_calculation() {
        // Test with typical piano string parameters
        let diameter = 0.001; // 1mm
        let length = 1.0; // 1m
        let tension = 150.0; // 150N

        let coefficient =
            InharmonicityModel::calculate_inharmonicity_coefficient(diameter, length, tension);

        // Should be a small positive value
        assert!(coefficient > 0.0);
        assert!(coefficient < 0.01); // Typical range for piano strings
    }

    #[test]
    fn test_partial_frequency_calculation() {
        // Test with a known coefficient
        let model = InharmonicityModel::from_coefficient(0.001);
        let fundamental = 440.0; // A4

        // First partial (fundamental) should be unchanged
        let first_partial = model.partial_frequency(fundamental, 1);
        assert!((first_partial - fundamental).abs() < 0.1);

        // Second partial should be slightly sharp
        let second_partial = model.partial_frequency(fundamental, 2);
        let expected_harmonic = 2.0 * fundamental;
        assert!(second_partial > expected_harmonic);

        // Higher partials should be progressively more sharp
        let third_partial = model.partial_frequency(fundamental, 3);
        let fourth_partial = model.partial_frequency(fundamental, 4);

        let second_deviation = second_partial - 2.0 * fundamental;
        let third_deviation = third_partial - 3.0 * fundamental;
        let fourth_deviation = fourth_partial - 4.0 * fundamental;

        assert!(third_deviation > second_deviation);
        assert!(fourth_deviation > third_deviation);
    }

    #[test]
    fn test_string_parameters_scaling() {
        // Test that bass notes have different parameters than treble
        let bass_params = PianoStringParameters::for_midi_note(21); // A0
        let treble_params = PianoStringParameters::for_midi_note(84); // C6 (more realistic high note)

        // Bass strings should be longer, thicker
        assert!(bass_params.length > treble_params.length);
        assert!(bass_params.diameter > treble_params.diameter);

        // Bass strings should have lower tension (corrected expectation)
        assert!(bass_params.tension < treble_params.tension);

        // Most importantly: bass should have higher inharmonicity
        let bass_model = InharmonicityModel::new(
            bass_params.diameter,
            bass_params.length,
            bass_params.tension,
        );
        let treble_model = InharmonicityModel::new(
            treble_params.diameter,
            treble_params.length,
            treble_params.tension,
        );

        assert!(
            bass_model.coefficient() > treble_model.coefficient(),
            "Bass inharmonicity ({:.6}) should be higher than treble ({:.6})",
            bass_model.coefficient(),
            treble_model.coefficient()
        );
    }

    #[test]
    fn test_inharmonic_vs_harmonic_frequency_deviation() {
        // Test that inharmonic partials are actually different from harmonic ones
        let model = InharmonicityModel::from_coefficient(0.001);
        let fundamental_freq = 440.0;

        for partial_num in 2..=8 {
            let inharmonic_freq = model.partial_frequency(fundamental_freq, partial_num as u32);
            let harmonic_freq = partial_num as f32 * fundamental_freq;

            // Inharmonic frequency should be higher than harmonic
            assert!(
                inharmonic_freq > harmonic_freq,
                "Partial {partial_num} should be sharp: inharmonic {inharmonic_freq} vs harmonic {harmonic_freq}"
            );

            // The deviation should be reasonable (not too extreme)
            let deviation_ratio = inharmonic_freq / harmonic_freq;
            assert!(
                deviation_ratio < 1.05, // Less than 5% sharp (higher partials can be quite sharp)
                "Partial {partial_num} deviation too large: ratio = {deviation_ratio}"
            );
        }
    }

    #[test]
    fn test_higher_partials_more_inharmonic() {
        // Test that higher partials deviate more from harmonic than lower ones
        let model = InharmonicityModel::from_coefficient(0.001);
        let fundamental_freq = 440.0;

        let mut previous_deviation = 0.0;

        for partial_num in 2..=8 {
            let inharmonic_freq = model.partial_frequency(fundamental_freq, partial_num as u32);
            let harmonic_freq = partial_num as f32 * fundamental_freq;
            let deviation = inharmonic_freq - harmonic_freq;

            // Higher partials should have larger absolute deviation
            assert!(
                deviation > previous_deviation,
                "Partial {} deviation ({}) should be larger than partial {} deviation ({})",
                partial_num,
                deviation,
                partial_num - 1,
                previous_deviation
            );

            previous_deviation = deviation;
        }
    }

    #[test]
    fn test_phase_delta_calculation_consistency() {
        // Test that the frequency calculations are mathematically sound
        // This ensures the phase deltas will be correct when used in the synth
        let model = InharmonicityModel::from_coefficient(0.001);
        let fundamental_freq = 440.0;
        let sample_rate = 44100.0;

        for partial_num in 2..=8 {
            let partial_freq = model.partial_frequency(fundamental_freq, partial_num as u32);
            let phase_delta = partial_freq / sample_rate;

            // Phase delta should be reasonable (not too large that it would alias)
            assert!(
                phase_delta < 0.5,
                "Partial {partial_num} phase delta {phase_delta} too large, would alias"
            );

            // Phase delta should be positive and non-zero
            assert!(
                phase_delta > 0.0,
                "Partial {partial_num} phase delta should be positive"
            );

            // For a 1-second period at this sample rate, we should get the right frequency
            let cycles_per_second = phase_delta * sample_rate;
            let expected_freq = partial_freq;
            assert!(
                (cycles_per_second - expected_freq).abs() < 0.01,
                "Partial {partial_num} frequency calculation inconsistent: expected {expected_freq}, got {cycles_per_second}"
            );
        }
    }
}
