/// Envelope generator for ADSR (Attack, Decay, Sustain, Release)
pub struct EnvelopeGenerator {
    sustain_level: f32, // 0.0 to 1.0
    current_level: f32,
    pub state: EnvelopeState,
    sustain_decay_rate: f32,   // Piano-like sustain decay
    attack_rate: Option<f32>,  // Precalculated attack rate
    decay_rate: Option<f32>,   // Precalculated decay rate
    release_rate: Option<f32>, // Precalculated release rate
    velocity_level: f32,       // Velocity scaling factor (0.0 to 1.0)
}

#[derive(PartialEq, Eq, Debug)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl EnvelopeGenerator {
    /// Create a new envelope generator with given ADSR parameters
    /// - `attack`: Attack time in seconds
    /// - `decay`: Decay time in seconds
    /// - `sustain`: Sustain level (0.0 to 1.0)
    /// - `release`: Release time in seconds
    /// - `sample_rate`: Sample rate in Hz
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: f32) -> Self {
        const EPSILON: f32 = 0.000001;

        let attack_rate = if attack > EPSILON {
            Some(1.0 / (sample_rate * attack))
        } else {
            None // Immediate attack
        };

        let decay_rate = if decay > EPSILON {
            Some((1.0 - sustain) / (sample_rate * decay))
        } else {
            None // Immediate decay
        };

        let release_rate = if release > EPSILON {
            Some(1.0 / (sample_rate * release))
        } else {
            None // Immediate release
        };

        Self {
            sustain_level: sustain,
            current_level: 0.0,
            state: EnvelopeState::Idle,
            // Will be set based on note frequency
            sustain_decay_rate: 0.0,
            attack_rate,
            decay_rate,
            release_rate,
            velocity_level: 1.0, // Default full velocity
        }
    }

    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        // Don't reset level to 0 to allow legato playing
    }

    pub fn release(&mut self) {
        self.state = EnvelopeState::Release;
    }

    pub fn set_sustain_decay_rate(&mut self, rate: f32) {
        self.sustain_decay_rate = rate;
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity_level = velocity;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        const MIN_ENVELOPE_LEVEL: f32 = 0.0;
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = MIN_ENVELOPE_LEVEL;
            }
            EnvelopeState::Attack => {
                const MAX_ENVELOPE_LEVEL: f32 = 1.0;
                if let Some(rate) = self.attack_rate {
                    self.current_level += rate;
                    if self.current_level >= MAX_ENVELOPE_LEVEL {
                        self.current_level = MAX_ENVELOPE_LEVEL;
                        self.state = EnvelopeState::Decay;
                    }
                } else {
                    self.current_level = MAX_ENVELOPE_LEVEL;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                if let Some(rate) = self.decay_rate {
                    self.current_level -= rate;
                    if self.current_level <= self.sustain_level {
                        self.current_level = self.sustain_level;
                        self.state = EnvelopeState::Sustain;
                    }
                } else {
                    self.current_level = self.sustain_level;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                // Piano-like sustain: gradually decays instead of holding steady
                self.current_level -= self.sustain_decay_rate;
                if self.current_level <= MIN_ENVELOPE_LEVEL {
                    self.current_level = MIN_ENVELOPE_LEVEL;
                    self.state = EnvelopeState::Idle;
                }
            }
            EnvelopeState::Release => {
                if let Some(rate) = self.release_rate {
                    // Piano strings don't follow simple exponential decay
                    // They have a more complex release with initially faster decay
                    // followed by a longer tail

                    const RELEASE_THRESHOLD: f32 = 0.0001; // -80dB
                    const INITIAL_DECAY_FACTOR: f32 = 2.5; // Initial decay is faster
                    const INITIAL_DECAY_THRESHOLD: f32 = 0.1;

                    self.current_level -= rate
                        * self.current_level
                        * if self.current_level > INITIAL_DECAY_THRESHOLD {
                            INITIAL_DECAY_FACTOR
                        } else {
                            1.0
                        };

                    if self.current_level <= RELEASE_THRESHOLD {
                        self.current_level = MIN_ENVELOPE_LEVEL;
                        self.state = EnvelopeState::Idle;
                    }
                } else {
                    self.current_level = MIN_ENVELOPE_LEVEL;
                    self.state = EnvelopeState::Idle;
                }
            }
        }

        // Apply velocity scaling to the envelope output
        self.current_level * self.velocity_level
    }

    pub fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle
    }

    pub fn current_level(&self) -> f32 {
        self.current_level
    }
}
