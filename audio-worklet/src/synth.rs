use crate::{limiter::Limiter, reverb::Reverb};
use std::{cmp::Ordering, f32::consts::PI};

/// Synth trait for audio synthesis
pub trait Synth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]);
}

/// Represents a piano key with associated frequency
#[derive(Copy, Clone)]
struct PianoKey {
    frequency: f32,
    midi_note: wmidi::Note,
}

impl PianoKey {
    fn new(midi_note: wmidi::Note) -> Self {
        let frequency = midi_note.to_freq_f32();
        Self {
            frequency,
            midi_note,
        }
    }
}

/// Envelope generator for ADSR (Attack, Decay, Sustain, Release)
struct EnvelopeGenerator {
    sustain_level: f32, // 0.0 to 1.0
    current_level: f32,
    state: EnvelopeState,
    sustain_decay_rate: f32,   // Piano-like sustain decay
    attack_rate: Option<f32>,  // Precalculated attack rate
    decay_rate: Option<f32>,   // Precalculated decay rate
    release_rate: Option<f32>, // Precalculated release rate
    velocity_level: f32,       // Velocity scaling factor (0.0 to 1.0)
}

#[derive(PartialEq, Eq, Debug)]
enum EnvelopeState {
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
    fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: f32) -> Self {
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

    fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        // Don't reset level to 0 to allow legato playing
    }

    fn release(&mut self) {
        self.state = EnvelopeState::Release;
    }

    fn set_sustain_decay_rate(&mut self, rate: f32) {
        self.sustain_decay_rate = rate;
    }

    fn set_velocity(&mut self, velocity: f32) {
        self.velocity_level = velocity;
    }

    #[inline]
    fn process(&mut self) -> f32 {
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
                const MIN_ENVELOPE_LEVEL: f32 = 0.0;
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

    fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle
    }
}

/// Piano voice with oscillator and envelope
struct PianoVoice {
    phase: f32,
    detuned_phase: f32,
    phase_delta: f32,
    envelope: EnvelopeGenerator,
    sample_rate: f32,
    is_active: bool,
    current_key: Option<PianoKey>,
    // Piano-specific parameters
    detuning: f32,     // Slight detuning for realism
    brightness: f32,   // Controls harmonic content
    velocity: f32,     // Normalized velocity (0.0 to 1.0)
    attack_phase: f32, // Tracks progress through attack portion (0.0 to 1.0)
    note_phase: f32,
}

impl PianoVoice {
    fn new(sample_rate: f32) -> Self {
        const ATTACK_TIME: f32 = 0.01;
        const DECAY_TIME: f32 = 0.1;
        const SUSTAIN_LEVEL: f32 = 0.7;
        const RELEASE_TIME: f32 = 0.3;

        const DETUNING: f32 = 1.003; // Creates chorus-like effect for richer tone
        const BRIGHTNESS: f32 = 0.8; // Controls higher harmonic content

        Self {
            phase: 0.0,
            detuned_phase: 0.0,
            phase_delta: 0.0,
            envelope: EnvelopeGenerator::new(
                ATTACK_TIME,
                DECAY_TIME,
                SUSTAIN_LEVEL,
                RELEASE_TIME,
                sample_rate,
            ),
            sample_rate,
            is_active: false,
            current_key: None,
            detuning: DETUNING,
            brightness: BRIGHTNESS,
            velocity: 1.0,     // Default full velocity
            attack_phase: 0.0, // Initialize attack phase
            note_phase: 0.0,
        }
    }

    fn note_on(&mut self, key: PianoKey, velocity: wmidi::U7) {
        const VELOCITY_POWER_CURVE: f32 = 0.8;
        const MIDI_VELOCITY_MAX: f32 = 127.0;

        const BASE_DECAY_RATE_HZ: f32 = 44100.0;
        const BASE_DECAY_RATE: f32 = 0.00001;

        const FREQUENCY_DECAY_REFERENCE_HZ: f32 = 110.0;

        const VELOCITY_DECAY_FACTOR: f32 = 0.3;

        self.current_key = Some(key);
        self.update_phase_delta();

        // Power curve provides more natural dynamic response than linear mapping
        let normalized_velocity = u8::from(velocity) as f32 / MIDI_VELOCITY_MAX;
        self.velocity = normalized_velocity.powf(VELOCITY_POWER_CURVE);

        self.envelope.set_velocity(self.velocity);
        self.attack_phase = 0.0; // Reset attack phase on new note
        self.note_phase = 0.0;

        // Model frequency-dependent decay behavior of real piano strings
        // Physics: higher frequency strings have less mass and dissipate energy faster
        if let Some(ref key) = self.current_key {
            // Scale relative to a reference sample rate to maintain consistent behavior across sample rates
            let base_decay_rate = BASE_DECAY_RATE * (BASE_DECAY_RATE_HZ / self.sample_rate);

            // Scale the decay rate based on frequency
            // Higher notes (higher frequency) decay faster
            let freq = key.frequency;
            let freq_factor = (freq / FREQUENCY_DECAY_REFERENCE_HZ).sqrt();

            // Also scale by velocity - higher velocity notes decay slightly slower
            let velocity_factor = 1.0 - (self.velocity * VELOCITY_DECAY_FACTOR); // 0.7 to 1.0 range

            let sustain_decay_rate = base_decay_rate * freq_factor * velocity_factor;
            self.envelope.set_sustain_decay_rate(sustain_decay_rate);
        }

        self.envelope.trigger();
        self.is_active = true;
    }

    fn note_off(&mut self) {
        self.envelope.release();
    }

    fn update_phase_delta(&mut self) {
        if let Some(key) = &self.current_key {
            self.phase_delta = key.frequency / self.sample_rate;
        }
    }

    #[inline]
    fn process(&mut self) -> f32 {
        if !self.is_active && !self.envelope.is_active() {
            return 0.0;
        }

        let env_value = self.envelope.process();
        if !self.envelope.is_active() {
            self.is_active = false;
            return 0.0;
        }

        const ATTACK_TRANSIENT_HZ: f32 = 50.0; // 1/0.02s = 50
        const MAX_ATTACK_PHASE: f32 = 1.0;

        // Update attack phase tracking (0.0 to 1.0) in a sample rate independent way
        // The attack transient should take ~20ms regardless of sample rate
        if self.envelope.state == EnvelopeState::Attack || self.attack_phase < MAX_ATTACK_PHASE {
            // Calculate rate based on sample rate - equivalent to 20ms attack transient
            let attack_rate = ATTACK_TRANSIENT_HZ / self.sample_rate;

            // Non-linear curve: faster at start, slower near end
            self.attack_phase += attack_rate * (MAX_ATTACK_PHASE - self.attack_phase);

            if self.attack_phase > MAX_ATTACK_PHASE {
                self.attack_phase = MAX_ATTACK_PHASE;
            }
        }

        const MAX_PHASE: f32 = 1.0;
        self.phase = (self.phase + self.phase_delta).rem_euclid(MAX_PHASE);
        self.detuned_phase =
            (self.detuned_phase + self.phase_delta * self.detuning).rem_euclid(MAX_PHASE);
        self.note_phase += self.phase_delta;

        let mut sample = 0.0;

        // Calculate attack intensity - strongest at the beginning
        let attack_intensity = (MAX_ATTACK_PHASE - self.attack_phase) * self.velocity;

        const FUNDAMENTAL_AMPLITUDE: f32 = 0.6;
        const SECOND_HARMONIC_AMPLITUDE: f32 = 0.4;
        const THIRD_HARMONIC_AMPLITUDE: f32 = 0.15;
        const TWO_PI: f32 = 2.0 * PI;

        // Fundamental
        sample += FUNDAMENTAL_AMPLITUDE * (TWO_PI * self.phase).sin();

        // Second harmonic - quite strong in pianos
        const SECOND_HARMONIC_MULTIPLIER: f32 = 2.0;
        sample +=
            SECOND_HARMONIC_AMPLITUDE * (SECOND_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();

        // Third harmonic
        const THIRD_HARMONIC_MULTIPLIER: f32 = 3.0;
        sample +=
            THIRD_HARMONIC_AMPLITUDE * (THIRD_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();

        const DYNAMIC_BRIGHTNESS_BASE: f32 = 0.7;
        const DYNAMIC_BRIGHTNESS_VELOCITY_FACTOR: f32 = 0.3;

        // Higher harmonics with brightness control and dynamic attack
        let dynamic_brightness = self.brightness
            * (DYNAMIC_BRIGHTNESS_BASE + DYNAMIC_BRIGHTNESS_VELOCITY_FACTOR * self.velocity);

        const ATTACK_HARMONIC_BOOST_FACTOR: f32 = 2.0;

        // 4th and 5th harmonics are stronger during attack phase
        let attack_harmonic_boost =
            MAX_ATTACK_PHASE + (attack_intensity * ATTACK_HARMONIC_BOOST_FACTOR);

        const FOURTH_HARMONIC_AMPLITUDE: f32 = 0.2;
        const FIFTH_HARMONIC_AMPLITUDE: f32 = 0.14;
        const FOURTH_HARMONIC_MULTIPLIER: f32 = 4.0;
        const FIFTH_HARMONIC_MULTIPLIER: f32 = 5.0;

        sample += dynamic_brightness
            * FOURTH_HARMONIC_AMPLITUDE
            * attack_harmonic_boost
            * (FOURTH_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();
        sample += dynamic_brightness
            * FIFTH_HARMONIC_AMPLITUDE
            * attack_harmonic_boost
            * (FIFTH_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();

        const ATTACK_INTENSITY_THRESHOLD: f32 = 0.01;

        const SIXTH_HARMONIC_AMPLITUDE: f32 = 0.05;
        const SEVENTH_HARMONIC_AMPLITUDE: f32 = 0.03;
        const EIGHTH_HARMONIC_AMPLITUDE: f32 = 0.02;
        const SIXTH_HARMONIC_MULTIPLIER: f32 = 6.0;
        const SEVENTH_HARMONIC_MULTIPLIER: f32 = 7.0;
        const EIGHTH_HARMONIC_MULTIPLIER: f32 = 8.0;

        // Add even higher harmonics during attack for hammer "ping"
        if attack_intensity > ATTACK_INTENSITY_THRESHOLD {
            sample += dynamic_brightness
                * SIXTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (SIXTH_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();
            sample += dynamic_brightness
                * SEVENTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (SEVENTH_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();
            sample += dynamic_brightness
                * EIGHTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (EIGHTH_HARMONIC_MULTIPLIER * TWO_PI * self.phase).sin();
        }

        const DETUNED_OSCILLATOR_AMPLITUDE: f32 = 0.1;

        // Detuned oscillator for richness
        sample += DETUNED_OSCILLATOR_AMPLITUDE * (TWO_PI * self.detuned_phase).sin();

        const NOISE1_FREQ: f32 = 3.71;
        const NOISE2_FREQ: f32 = 5.83;
        const NOISE3_FREQ: f32 = 8.91;

        const NOISE3_NOTE_PHASE_FACTOR: f32 = 0.5;
        const NOISE3_ATTACK_INTENSITY_FACTOR: f32 = 0.5;

        const HAMMER_NOISE_AMPLITUDE: f32 = 0.2;

        // Add hammer noise/transient during attack
        if attack_intensity > ATTACK_INTENSITY_THRESHOLD {
            // Use attack_intensity as base phase for noise to create evolving hammer sound
            // This ensures a continuous noise transition that doesn't repeat with the waveform cycle

            // Create noise elements using attack_intensity as the primary phase source
            // attack_intensity smoothly goes from 1.0 to 0.0, creating evolving hammer strike sound
            let noise1 = (TWO_PI * attack_intensity * NOISE1_FREQ).sin();
            let noise2 = (TWO_PI * attack_intensity * NOISE2_FREQ).cos();

            // Add some phase and detuned phase influence to create more complex sound
            // The phase component adds string harmonic characteristics
            let noise3 = (TWO_PI
                * (self.note_phase * NOISE3_NOTE_PHASE_FACTOR
                    + attack_intensity * NOISE3_ATTACK_INTENSITY_FACTOR)
                * NOISE3_FREQ)
                .sin();

            let noise = noise1 * noise2 * noise3;

            sample += noise * attack_intensity * self.velocity * HAMMER_NOISE_AMPLITUDE;

            const THUMP_AMPLITUDE: f32 = 0.5;
            const THUMP_FREQ: f32 = 5.0;

            // Add initial "thump" of hammer hitting string - brief low-mid frequency component
            sample += attack_intensity
                * self.velocity
                * THUMP_AMPLITUDE
                * (TWO_PI * attack_intensity * THUMP_FREQ).sin(); // Lower frequency thump
        }

        const FINAL_AMPLITUDE_SCALING: f32 = 0.3;

        // Reduce overall volume to prevent clipping
        sample *= FINAL_AMPLITUDE_SCALING;
        sample *= env_value;

        sample
    }
}

/// Piano synth managing multiple voices for polyphony
pub struct PianoSynth {
    voices: Vec<PianoVoice>,
    sample_rate: Option<u32>,
    reverb: Option<Reverb>,
    limiter: Option<Limiter>,
}

impl Default for PianoSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl PianoSynth {
    pub fn new() -> Self {
        Self {
            voices: Vec::new(),
            sample_rate: None,
            reverb: None,
            limiter: None,
        }
    }

    pub fn note_on(&mut self, note: wmidi::Note, velocity: wmidi::U7) {
        let key = PianoKey::new(note);

        // First try to find an inactive voice
        let voice = if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active) {
            voice
        } else {
            // Voice stealing - prioritize voices based on envelope state
            // Find the voice furthest along in its envelope cycle
            self.find_voice_to_steal()
        };

        voice.note_on(key, velocity);
    }

    // Helper method to find the best voice to steal
    fn find_voice_to_steal(&mut self) -> &mut PianoVoice {
        // Strategy: find index first, then get the voice by index
        let voice_index = {
            // First check for voices in release state (already note-off)
            let release_index = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.envelope.state == EnvelopeState::Release)
                .min_by(|(_, a), (_, b)| {
                    a.envelope
                        .current_level
                        .partial_cmp(&b.envelope.current_level)
                        .unwrap_or(Ordering::Equal)
                })
                .map(|(idx, _)| idx);

            if let Some(idx) = release_index {
                idx
            } else {
                // Then check for voices in sustain state
                let sustain_index = self
                    .voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.envelope.state == EnvelopeState::Sustain)
                    .min_by(|(_, a), (_, b)| {
                        a.envelope
                            .current_level
                            .partial_cmp(&b.envelope.current_level)
                            .unwrap_or(Ordering::Equal)
                    })
                    .map(|(idx, _)| idx);

                if let Some(idx) = sustain_index {
                    idx
                } else {
                    // Last resort: take the voice with the lowest current envelope level
                    self.voices
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            a.envelope
                                .current_level
                                .partial_cmp(&b.envelope.current_level)
                                .unwrap_or(Ordering::Equal)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap()
                }
            }
        };

        &mut self.voices[voice_index]
    }

    pub fn note_off(&mut self, midi_note: wmidi::Note) {
        for voice in self.voices.iter_mut() {
            if let Some(key) = &voice.current_key {
                if key.midi_note == midi_note {
                    voice.note_off();
                }
            }
        }
    }

    #[inline]
    fn process(&mut self) -> f32 {
        self.voices.iter_mut().map(|v| v.process()).sum()
    }
}

impl Synth for PianoSynth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]) {
        if self.sample_rate != Some(sample_rate) {
            self.voices.clear();
            self.reverb = None;
            self.sample_rate = Some(sample_rate);
            self.limiter = None;
        }
        if self.voices.is_empty() {
            const NUM_VOICES: usize = 8;
            self.voices.reserve(NUM_VOICES);
            for _ in 0..NUM_VOICES {
                self.voices.push(PianoVoice::new(sample_rate as f32));
            }
        }

        for out_channels in out_samples.chunks_exact_mut(num_channels) {
            let s = self.process();
            let s = self
                .reverb
                .get_or_insert_with(|| Reverb::new(sample_rate as f32))
                .process(s);
            let s = self
                .limiter
                .get_or_insert_with(|| Limiter::new(sample_rate as f32))
                .process(s);
            for c in out_channels.iter_mut() {
                *c = s;
            }
        }
    }
}
