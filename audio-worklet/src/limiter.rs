// AI generated

/// Audio peak limiter
/// Prevents audio signals from exceeding a specified threshold
pub struct Limiter {
    // Limiter parameters
    threshold: f32,   // Threshold level in dB (negative value)
    attack: f32,      // Attack time in seconds
    release: f32,     // Release time in seconds
    makeup_gain: f32, // Makeup gain in dB

    // Internal state
    envelope: f32,       // Current envelope level
    gain_reduction: f32, // Current gain reduction in linear scale
    sample_rate: f32,    // Sample rate for time constant calculations
    attack_coef: f32,    // Attack coefficient
    release_coef: f32,   // Release coefficient
}

impl Limiter {
    pub fn new(sample_rate: f32) -> Self {
        let attack = 0.005; // 5ms default attack
        let release = 0.050; // 50ms default release

        let mut limiter = Limiter {
            threshold: -3.0, // -3 dB default threshold
            attack,
            release,
            makeup_gain: 0.0, // 0 dB default makeup gain
            envelope: 0.0,
            gain_reduction: 1.0, // No reduction initially (1.0 = 0dB)
            sample_rate,
            attack_coef: 0.0,  // Will be set in update_coefficients
            release_coef: 0.0, // Will be set in update_coefficients
        };

        limiter.update_coefficients();
        limiter
    }

    #[allow(dead_code)]
    pub fn set_threshold(&mut self, threshold_db: f32) {
        self.threshold = threshold_db.clamp(-60.0, 0.0);
    }

    #[allow(dead_code)]
    pub fn set_attack(&mut self, attack_time: f32) {
        self.attack = attack_time.clamp(0.001, 1.0);
        self.update_coefficients();
    }

    #[allow(dead_code)]
    pub fn set_release(&mut self, release_time: f32) {
        self.release = release_time.clamp(0.001, 3.0);
        self.update_coefficients();
    }

    #[allow(dead_code)]
    pub fn set_makeup_gain(&mut self, makeup_gain_db: f32) {
        self.makeup_gain = makeup_gain_db.clamp(0.0, 30.0);
    }

    fn update_coefficients(&mut self) {
        self.attack_coef = (-1.0 / (self.sample_rate * self.attack)).exp();
        self.release_coef = (-1.0 / (self.sample_rate * self.release)).exp();
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        // Convert threshold from dB to linear
        let threshold_linear = 10.0_f32.powf(self.threshold / 20.0);

        let input_abs = input.abs();

        // Envelope detection (peak detection)
        if input_abs > self.envelope {
            // Attack phase: envelope rises quickly
            self.envelope = self.attack_coef * (self.envelope - input_abs) + input_abs;
        } else {
            // Release phase: envelope falls more slowly
            self.envelope = self.release_coef * (self.envelope - input_abs) + input_abs;
        }

        if self.envelope > threshold_linear {
            // Calculate gain reduction in linear scale
            self.gain_reduction = threshold_linear / self.envelope;
        } else {
            self.gain_reduction = 1.0;
        }

        let makeup_gain_linear = 10.0_f32.powf(self.makeup_gain / 20.0);
        input * self.gain_reduction * makeup_gain_linear
    }

    // Get the current gain reduction in dB (useful for metering)
    #[allow(dead_code)]
    #[inline]
    pub fn get_gain_reduction_db(&self) -> f32 {
        20.0 * self.gain_reduction.log10()
    }
}
