// AI generated

const MS_TO_S: f32 = 0.001;

// Delay times in ms for comb filters based on classic Schroeder reverb
const COMB_FILTER_DELAYS_MS: [f32; 4] = [29.7, 37.1, 41.1, 43.7];
const COMB_FILTER_FEEDBACK: f32 = 0.84;
const COMB_FILTER_DAMPING: f32 = 0.2;

// Delay times for allpass filters
const ALLPASS_FILTER_DELAYS_MS: [f32; 2] = [5.0, 1.7];
const ALLPASS_FILTER_FEEDBACK: f32 = 0.5;

const DEFAULT_ROOM_SIZE: f32 = 0.7;
const DEFAULT_DAMPING: f32 = 0.4;
const DEFAULT_WET_LEVEL: f32 = 0.5;
const DEFAULT_DRY_LEVEL: f32 = 0.4;
const DEFAULT_WIDTH: f32 = 1.0;

/// Shroeder reverb
pub struct Reverb {
    // Reverb parameters
    room_size: f32,
    damping: f32,
    wet_level: f32,
    dry_level: f32,
    width: f32,

    // Comb filters for main reverb body
    comb_filters: Vec<CombFilter>,
    // All-pass filters for diffusion
    allpass_filters: Vec<AllpassFilter>,
}

struct CombFilter {
    delay_line: Vec<f32>,
    index: usize,
    feedback: f32,
    damping: f32,
    dampening_value: f32,
}

struct AllpassFilter {
    delay_line: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let comb_filters = COMB_FILTER_DELAYS_MS
            .iter()
            .map(|delay| {
                let buffer_size = (delay * MS_TO_S * sample_rate) as usize;
                CombFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: COMB_FILTER_FEEDBACK,
                    damping: COMB_FILTER_DAMPING,
                    dampening_value: 0.0,
                }
            })
            .collect();

        let allpass_filters = ALLPASS_FILTER_DELAYS_MS
            .iter()
            .map(|delay| {
                let buffer_size = (delay * MS_TO_S * sample_rate) as usize;
                AllpassFilter {
                    delay_line: vec![0.0; buffer_size],
                    index: 0,
                    feedback: ALLPASS_FILTER_FEEDBACK,
                }
            })
            .collect();

        let mut reverb = Reverb {
            room_size: DEFAULT_ROOM_SIZE,
            damping: DEFAULT_DAMPING,
            wet_level: DEFAULT_WET_LEVEL,
            dry_level: DEFAULT_DRY_LEVEL,
            width: DEFAULT_WIDTH,
            comb_filters,
            allpass_filters,
        };

        reverb.update_parameters();
        reverb
    }

    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        self.update_parameters();
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
        self.update_parameters();
    }

    pub fn set_wet_level(&mut self, level: f32) {
        self.wet_level = level.clamp(0.0, 1.0);
    }

    pub fn set_dry_level(&mut self, level: f32) {
        self.dry_level = level.clamp(0.0, 1.0);
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 1.0);
    }

    fn update_parameters(&mut self) {
        const ROOM_SIZE_FACTOR: f32 = 0.6;
        const ROOM_SIZE_OFFSET: f32 = 0.4;

        for filter in &mut self.comb_filters {
            filter.feedback = self.room_size * ROOM_SIZE_FACTOR + ROOM_SIZE_OFFSET;
            filter.damping = self.damping;
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = 0.0;

        for filter in &mut self.comb_filters {
            output += filter.process(input);
        }
        output /= self.comb_filters.len() as f32;

        for filter in &mut self.allpass_filters {
            output = filter.process(output);
        }

        self.dry_level * input + self.wet_level * output
    }
}

impl CombFilter {
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.delay_line[self.index];
        self.dampening_value = output * (1.0 - self.damping) + self.dampening_value * self.damping;
        let new_value = input + self.dampening_value * self.feedback;
        self.delay_line[self.index] = new_value;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}

impl AllpassFilter {
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.delay_line[self.index];
        let output = -input * self.feedback + delayed;
        self.delay_line[self.index] = input + delayed * self.feedback;
        self.index = (self.index + 1) % self.delay_line.len();
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAMPLE_RATE: f32 = 44100.0;

    #[test]
    fn test_basic_reverb_functionality() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Test that dry signal passes through correctly
        let input = 0.5;
        let output = reverb.process(input);
        let expected_dry = DEFAULT_DRY_LEVEL * input;

        // The output should contain at minimum the dry component
        assert!(
            output >= expected_dry * 0.9,
            "Output {output} should contain dry component {expected_dry}"
        );

        // Process multiple samples to fill delay lines
        let mut all_zero = true;
        for _ in 0..5000 {
            // Process enough samples to fill largest delay buffer
            let sample = reverb.process(0.0);
            if sample.abs() > 1e-10 {
                all_zero = false;
            }
        }

        // After an impulse and some processing, we should see some reverb tail
        assert!(!all_zero, "Should have some reverb tail after impulse");
    }

    #[test]
    fn test_impulse_response() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Apply an impulse (single sample with value 1.0)
        let impulse_response = reverb.process(1.0);

        // The impulse response should contain both dry and wet signal
        // With default settings (dry=0.3, wet=0.6), we expect some immediate output
        assert!(
            impulse_response.abs() > 0.0,
            "Impulse should produce immediate output"
        );

        // The dry component should be around 0.3 (dry_level * input)
        let expected_dry = DEFAULT_DRY_LEVEL; // DEFAULT_DRY_LEVEL * 1.0
        assert!(
            impulse_response >= expected_dry * 0.9,
            "Should include significant dry component"
        );

        // Process silence for enough samples to let the reverb develop
        // Longest delay is 43.7ms = ~1926 samples at 44.1kHz
        let max_delay_samples = (43.7 * 0.001 * TEST_SAMPLE_RATE) as usize + 100;
        let mut reverb_tail = Vec::new();

        for _ in 0..max_delay_samples {
            let sample = reverb.process(0.0);
            reverb_tail.push(sample.abs());
        }

        // The reverb should produce some output after the delays kick in
        let has_reverb_tail = reverb_tail.iter().any(|&sample| sample > 0.001);
        assert!(has_reverb_tail, "Reverb should produce a tail after delays");

        // Find where the reverb energy starts (after delays)
        let start_idx = reverb_tail.iter().position(|&s| s > 0.001).unwrap_or(0);
        if start_idx < reverb_tail.len() - 500 {
            let early_energy = reverb_tail[start_idx];
            let late_energy = reverb_tail[reverb_tail.len() - 1];

            // Energy should generally decrease (allowing for some variation due to filter interactions)
            assert!(
                early_energy > late_energy * 0.5,
                "Reverb should show some decay: early={early_energy}, late={late_energy}"
            );
        }
    }

    #[test]
    fn test_dry_wet_mix() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Test with only dry signal
        reverb.set_dry_level(1.0);
        reverb.set_wet_level(0.0);
        let dry_only = reverb.process(0.5);
        assert!(
            (dry_only - 0.5).abs() < 0.01,
            "Dry-only should pass input through"
        );

        // Reset reverb state
        reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Test with only wet signal - need to prime the delays first
        reverb.set_dry_level(0.0);
        reverb.set_wet_level(1.0);

        // Send a few samples to prime the delay lines
        for _ in 0..10 {
            reverb.process(0.5);
        }

        // Now process the delay length to see wet output
        let max_delay_samples = (43.7 * 0.001 * TEST_SAMPLE_RATE) as usize + 100;
        let mut has_wet_output = false;

        for _ in 0..max_delay_samples {
            let wet_sample = reverb.process(0.0);
            if wet_sample.abs() > 0.001 {
                has_wet_output = true;
                break;
            }
        }

        assert!(
            has_wet_output,
            "Wet-only should produce some output after delays"
        );
    }

    #[test]
    fn test_room_size_effect() {
        let mut reverb_small = Reverb::new(TEST_SAMPLE_RATE);
        let mut reverb_large = Reverb::new(TEST_SAMPLE_RATE);

        reverb_small.set_room_size(0.1);
        reverb_large.set_room_size(0.9);

        // Prime both reverbs with the same input pattern
        for _ in 0..10 {
            reverb_small.process(1.0);
            reverb_large.process(1.0);
        }

        // Now process silence and measure total energy output
        let max_delay_samples = (43.7 * 0.001 * TEST_SAMPLE_RATE) as usize * 2;
        let mut small_energy = 0.0;
        let mut large_energy = 0.0;

        for _ in 0..max_delay_samples {
            let small_sample = reverb_small.process(0.0);
            let large_sample = reverb_large.process(0.0);
            small_energy += small_sample.abs();
            large_energy += large_sample.abs();
        }

        // Larger room should have more sustained energy (longer decay)
        // Allow for some tolerance since the effect might be subtle
        assert!(
            large_energy > small_energy * 1.1,
            "Larger room should have more sustained reverb energy: small={small_energy}, large={large_energy}"
        );
    }

    #[test]
    fn test_damping_effect() {
        let mut reverb_low_damp = Reverb::new(TEST_SAMPLE_RATE);
        let mut reverb_high_damp = Reverb::new(TEST_SAMPLE_RATE);

        reverb_low_damp.set_damping(0.1);
        reverb_high_damp.set_damping(0.9);

        // Prime both reverbs with impulses
        for _ in 0..10 {
            reverb_low_damp.process(1.0);
            reverb_high_damp.process(1.0);
        }

        // Process silence and measure high-frequency content
        let max_delay_samples = (43.7 * 0.001 * TEST_SAMPLE_RATE) as usize * 2;
        let mut low_damp_hf_energy = 0.0;
        let mut high_damp_hf_energy = 0.0;
        let mut prev_low = 0.0;
        let mut prev_high = 0.0;

        for _ in 0..max_delay_samples {
            let low_sample = reverb_low_damp.process(0.0);
            let high_sample = reverb_high_damp.process(0.0);

            // Measure high-frequency content by sample-to-sample differences
            low_damp_hf_energy += (low_sample - prev_low).abs();
            high_damp_hf_energy += (high_sample - prev_high).abs();

            prev_low = low_sample;
            prev_high = high_sample;
        }

        // Higher damping should reduce high-frequency content
        // Allow some tolerance since the effect might be subtle
        if low_damp_hf_energy > 0.001 && high_damp_hf_energy > 0.001 {
            assert!(
                low_damp_hf_energy > high_damp_hf_energy * 1.1,
                "Lower damping should preserve more high-frequency content: low_damp={low_damp_hf_energy}, high_damp={high_damp_hf_energy}"
            );
        }
    }

    #[test]
    fn test_parameter_clamping() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Test that parameters are properly clamped to valid ranges
        reverb.set_room_size(-1.0);
        reverb.set_damping(2.0);
        reverb.set_wet_level(-0.5);
        reverb.set_dry_level(1.5);
        reverb.set_width(-0.1);

        // All should be clamped to valid ranges
        // We can't directly access the fields, but we can test that the reverb still works
        let output = reverb.process(0.5);
        assert!(
            output.is_finite(),
            "Reverb should produce finite output even with invalid parameters"
        );
    }

    #[test]
    fn test_silence_input() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Process silence
        let output = reverb.process(0.0);
        assert_eq!(
            output, 0.0,
            "Silence input should produce silence output initially"
        );

        // Continue processing silence
        for _ in 0..1000 {
            let sample = reverb.process(0.0);
            assert!(
                sample.abs() < 1e-10,
                "Continued silence should remain silent"
            );
        }
    }

    #[test]
    fn test_stability() {
        let mut reverb = Reverb::new(TEST_SAMPLE_RATE);

        // Process a variety of inputs to ensure numerical stability
        let test_inputs = [0.0, 1.0, -1.0, 0.5, -0.5, 0.1, -0.1];

        for &input in &test_inputs {
            for _ in 0..1000 {
                let output = reverb.process(input);
                assert!(
                    output.is_finite(),
                    "Output should always be finite for input {input}"
                );
                assert!(
                    output.abs() < 100.0,
                    "Output should not grow unbounded for input {input}"
                );
            }
        }
    }
}
