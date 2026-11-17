use crate::{
    inharmonicity::{InharmonicityModel, PianoStringParameters},
    limiter::Limiter,
    reverb::Reverb,
};
use shared_types::Tuning;
use std::{cmp::Ordering, f32::consts::PI};

mod envelope;
use envelope::{EnvelopeGenerator, EnvelopeState};

/// Synth trait for audio synthesis
pub trait Synth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]);
}

struct PitchGlide {
    target: f32,
    step: f32,
    remaining: u32,
}

/// Piano voice with oscillator and envelope
struct PianoVoice {
    // TODO: couldn't these be part of the partial_phases array?
    phase: f32,
    detuned_phase: f32,
    phase_delta: f32,
    pitch_glide: Option<PitchGlide>,
    envelope: EnvelopeGenerator,
    sample_rate: f32,
    is_active: bool,
    release_deferred: bool,
    current_note: Option<wmidi::Note>,
    // Piano-specific parameters
    detuning: f32,     // Slight detuning for realism
    brightness: f32,   // Controls harmonic content
    velocity: f32,     // Normalized velocity (0.0 to 1.0)
    attack_phase: f32, // Tracks progress through attack portion (0.0 to 1.0)
    note_phase: f32,
    // Individual phase accumulators for inharmonic partials
    partial_phases: [f32; 7], // Phases for partials 2-8 (partial 1 uses main phase)
    partial_multipliers: [f32; 7],
    // Cached phase deltas for inharmonic partials to avoid recalculation in hot path
    partial_phase_deltas: [f32; 7], // Phase deltas for partials 2-8 (7 partials)
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
            pitch_glide: None,
            envelope: EnvelopeGenerator::new(
                ATTACK_TIME,
                DECAY_TIME,
                SUSTAIN_LEVEL,
                RELEASE_TIME,
                sample_rate,
            ),
            sample_rate,
            is_active: false,
            release_deferred: false,
            current_note: None,
            detuning: DETUNING,
            brightness: BRIGHTNESS,
            velocity: 1.0,     // Default full velocity
            attack_phase: 0.0, // Initialize attack phase
            note_phase: 0.0,
            partial_phases: [0.0; 7], // Initialize all partial phases to 0
            partial_multipliers: [0.0; 7],
            partial_phase_deltas: [0.0; 7], // Initialize all partial phase deltas to 0
        }
    }

    fn note_on(&mut self, note: wmidi::Note, velocity: wmidi::U7, frequency: f32) {
        const VELOCITY_POWER_CURVE: f32 = 0.8;
        const MIDI_VELOCITY_MAX: f32 = 127.0;

        const BASE_DECAY_RATE_HZ: f32 = 44100.0;
        // Much slower sustain decay than a normal piano, we want to allow the user to hear many notes at once.
        const BASE_DECAY_RATE: f32 = 0.000001;

        const FREQUENCY_DECAY_REFERENCE_HZ: f32 = 110.0;

        const VELOCITY_DECAY_FACTOR: f32 = 0.3;

        debug_assert!(frequency.is_finite() && frequency > 0.0);
        self.current_note = Some(note);
        self.release_deferred = false;
        self.pitch_glide = None;

        // Keep the string's overtone spacing when its fundamental is retuned.
        let inharmonicity =
            InharmonicityModel::from(PianoStringParameters::for_midi_note(note.into()));
        self.partial_multipliers =
            std::array::from_fn(|index| inharmonicity.partial_frequency(1.0, index as u32 + 2));
        self.set_phase_delta(frequency / self.sample_rate);

        // Power curve provides more natural dynamic response than linear mapping
        let normalized_velocity = u8::from(velocity) as f32 / MIDI_VELOCITY_MAX;
        self.velocity = normalized_velocity.powf(VELOCITY_POWER_CURVE);

        self.envelope.set_velocity(self.velocity);
        self.attack_phase = 0.0; // Reset attack phase on new note
        self.note_phase = 0.0;

        // Don't reset partial_phases to maintain legato playing consistency with main phases

        // Model frequency-dependent decay behavior of real piano strings
        // Physics: higher frequency strings have less mass and dissipate energy faster
        let base_decay_rate = BASE_DECAY_RATE * (BASE_DECAY_RATE_HZ / self.sample_rate);
        let freq_factor = (frequency / FREQUENCY_DECAY_REFERENCE_HZ).sqrt();
        let velocity_factor = 1.0 - (self.velocity * VELOCITY_DECAY_FACTOR);
        self.envelope
            .set_sustain_decay_rate(base_decay_rate * freq_factor * velocity_factor);

        self.envelope.trigger();
        self.is_active = true;
    }

    fn note_off(&mut self) {
        self.release_deferred = false;
        self.envelope.release();
    }

    fn set_phase_delta(&mut self, phase_delta: f32) {
        self.phase_delta = phase_delta;
        self.partial_phase_deltas = self.partial_multipliers.map(|ratio| ratio * phase_delta);
    }

    fn retune(&mut self, frequency: f32) {
        const GLIDE_SECONDS: f32 = 0.020;
        let target = frequency / self.sample_rate;
        let remaining = (self.sample_rate * GLIDE_SECONDS).round().max(1.0) as u32;
        self.pitch_glide = (target != self.phase_delta).then_some(PitchGlide {
            target,
            step: (target - self.phase_delta) / remaining as f32,
            remaining,
        });
    }

    fn advance_pitch_glide(&mut self) {
        if let Some(glide) = &mut self.pitch_glide {
            glide.remaining -= 1;
            let phase_delta = if glide.remaining == 0 {
                let target = glide.target;
                self.pitch_glide = None;
                target
            } else {
                self.phase_delta + glide.step
            };
            self.set_phase_delta(phase_delta);
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

        self.advance_pitch_glide();

        const MAX_PHASE: f32 = 1.0;
        self.phase = (self.phase + self.phase_delta).rem_euclid(MAX_PHASE);
        self.detuned_phase =
            (self.detuned_phase + self.phase_delta * self.detuning).rem_euclid(MAX_PHASE);
        self.note_phase += self.phase_delta;

        // Update individual phase accumulators for inharmonic partials using cached phase deltas
        for (partial_index, &cached_phase_delta) in self.partial_phase_deltas.iter().enumerate() {
            self.partial_phases[partial_index] =
                (self.partial_phases[partial_index] + cached_phase_delta).rem_euclid(MAX_PHASE);
        }

        let mut sample = 0.0;

        // Calculate attack intensity - strongest at the beginning
        let attack_intensity = (MAX_ATTACK_PHASE - self.attack_phase) * self.velocity;

        const FUNDAMENTAL_AMPLITUDE: f32 = 0.6;
        const SECOND_HARMONIC_AMPLITUDE: f32 = 0.4;
        const THIRD_HARMONIC_AMPLITUDE: f32 = 0.15;
        const TWO_PI: f32 = 2.0 * PI;

        // Fundamental (always exactly 1.0 multiplier, uses main phase)
        sample += FUNDAMENTAL_AMPLITUDE * (TWO_PI * self.phase).sin();

        // Second partial - inharmonic (uses partial_phases[0])
        sample += SECOND_HARMONIC_AMPLITUDE * (TWO_PI * self.partial_phases[0]).sin();

        // Third partial - inharmonic (uses partial_phases[1])
        sample += THIRD_HARMONIC_AMPLITUDE * (TWO_PI * self.partial_phases[1]).sin();

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

        sample += dynamic_brightness
            * FOURTH_HARMONIC_AMPLITUDE
            * attack_harmonic_boost
            * (TWO_PI * self.partial_phases[2]).sin(); // 4th partial
        sample += dynamic_brightness
            * FIFTH_HARMONIC_AMPLITUDE
            * attack_harmonic_boost
            * (TWO_PI * self.partial_phases[3]).sin(); // 5th partial

        const ATTACK_INTENSITY_THRESHOLD: f32 = 0.01;

        const SIXTH_HARMONIC_AMPLITUDE: f32 = 0.05;
        const SEVENTH_HARMONIC_AMPLITUDE: f32 = 0.03;
        const EIGHTH_HARMONIC_AMPLITUDE: f32 = 0.02;

        // Add even higher harmonics during attack for hammer "ping"
        if attack_intensity > ATTACK_INTENSITY_THRESHOLD {
            sample += dynamic_brightness
                * SIXTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (TWO_PI * self.partial_phases[4]).sin(); // 6th partial
            sample += dynamic_brightness
                * SEVENTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (TWO_PI * self.partial_phases[5]).sin(); // 7th partial
            sample += dynamic_brightness
                * EIGHTH_HARMONIC_AMPLITUDE
                * attack_intensity
                * (TWO_PI * self.partial_phases[6]).sin(); // 8th partial
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

        // +6dB gain
        const FINAL_AMPLITUDE_SCALING: f32 = 0.6;

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
    sustain_pedal_active: bool,
}

impl Default for PianoSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl PianoSynth {
    pub fn new() -> Self {
        // Used in tests where sample_rate will be set via play() before use.
        Self {
            voices: Vec::new(),
            sample_rate: None,
            reverb: None,
            limiter: None,
            sustain_pedal_active: false,
        }
    }

    /// Construct with a known sample rate (used by the audio processor) so voices are ready
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        let mut s = Self::new();
        s.sample_rate = Some(sample_rate);
        s.allocate_voices_if_needed();
        s
    }

    fn allocate_voices_if_needed(&mut self) {
        if self.voices.is_empty()
            && let Some(sr) = self.sample_rate
        {
            const NUM_VOICES: usize = 8;
            self.voices.reserve(NUM_VOICES);
            for _ in 0..NUM_VOICES {
                self.voices.push(PianoVoice::new(sr as f32));
            }
        }
    }

    pub fn note_on(&mut self, note: wmidi::Note, velocity: wmidi::U7, frequency: f32) {
        // Ensure voices exist (messages can arrive before first process callback when audio auto-starts)
        self.allocate_voices_if_needed();
        debug_assert!(
            self.sample_rate.is_some(),
            "sample_rate should be set before note_on"
        );
        // First try to find an inactive voice
        let voice = if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active) {
            voice
        } else {
            // Voice stealing - prioritize voices based on envelope state
            // Find the voice furthest along in its envelope cycle
            self.find_voice_to_steal()
        };

        voice.note_on(note, velocity, frequency);
    }

    pub fn retune(&mut self, tuning: Tuning) {
        for voice in &mut self.voices {
            if voice.is_active
                && let Some(note) = voice.current_note
            {
                voice.retune(tuning.frequency(note.into()));
            }
        }
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
                        .current_level()
                        .partial_cmp(&b.envelope.current_level())
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
                            .current_level()
                            .partial_cmp(&b.envelope.current_level())
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
                                .current_level()
                                .partial_cmp(&b.envelope.current_level())
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
        for voice in &mut self.voices {
            if voice.is_active
                && voice.envelope.state != EnvelopeState::Release
                && voice.current_note == Some(midi_note)
            {
                if self.sustain_pedal_active {
                    voice.release_deferred = true;
                } else {
                    voice.note_off();
                }
            }
        }
    }

    /// Set the sustain pedal state
    pub fn set_sustain_pedal(&mut self, active: bool) {
        if self.sustain_pedal_active && !active {
            // A re-pressed key can have a held voice alongside its older sustained voice.
            for voice in &mut self.voices {
                if voice.release_deferred {
                    voice.note_off();
                }
            }
        }
        self.sustain_pedal_active = active;
    }

    #[inline]
    fn process(&mut self) -> f32 {
        self.voices.iter_mut().map(|v| v.process()).sum()
    }
}

impl Synth for PianoSynth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]) {
        if self.sample_rate != Some(sample_rate) {
            // Sample rate changed or first-time setup.
            self.voices.clear();
            self.reverb = None;
            self.limiter = None;
            self.sample_rate = Some(sample_rate);
        }
        self.allocate_voices_if_needed();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retuning_glides_sustained_voices_without_resetting_the_attack() {
        const SAMPLE_RATE: u32 = 44_100;
        let mut synth = PianoSynth::with_sample_rate(SAMPLE_RATE);
        let note = wmidi::Note::E4;
        let equal = Tuning::Equal.frequency(note.into());
        let just = Tuning::Just { reference: 60 };
        synth.set_sustain_pedal(true);
        synth.note_on(note, wmidi::U7::MAX, equal);
        for _ in 0..SAMPLE_RATE / 4 {
            synth.process();
        }
        synth.note_off(note);

        let voice = &synth.voices[0];
        let phase = voice.phase;
        let partial_phases = voice.partial_phases;
        let phase_delta = voice.phase_delta;
        assert_eq!(voice.envelope.state, EnvelopeState::Sustain);
        let envelope_level = voice.envelope.current_level();
        let attack_phase = voice.attack_phase;
        synth.retune(just);
        let voice = &mut synth.voices[0];
        assert_eq!(voice.phase, phase);
        assert_eq!(voice.partial_phases, partial_phases);
        assert_eq!(voice.envelope.state, EnvelopeState::Sustain);
        assert_eq!(voice.envelope.current_level(), envelope_level);
        assert_eq!(voice.attack_phase, attack_phase);
        assert!(voice.release_deferred);

        voice.process();
        let target_delta = just.frequency(note.into()) / SAMPLE_RATE as f32;
        assert!(voice.phase_delta < phase_delta && voice.phase_delta > target_delta);
        while voice.pitch_glide.is_some() {
            voice.process();
        }
        assert_eq!(voice.phase_delta, target_delta);
        assert_eq!(
            voice.partial_phase_deltas,
            voice.partial_multipliers.map(|ratio| ratio * target_delta)
        );
        assert_eq!(voice.envelope.state, EnvelopeState::Sustain);
    }

    #[test]
    fn pedal_up_releases_only_the_previous_voice_of_a_repressed_key() {
        let mut synth = PianoSynth::with_sample_rate(44_100);
        let note = wmidi::Note::C4;
        synth.set_sustain_pedal(true);
        synth.note_on(note, wmidi::U7::MAX, note.to_freq_f32());
        synth.note_off(note);
        synth.note_on(note, wmidi::U7::MAX, note.to_freq_f32());

        synth.set_sustain_pedal(false);

        assert_eq!(synth.voices[0].envelope.state, EnvelopeState::Release);
        assert_eq!(synth.voices[1].envelope.state, EnvelopeState::Attack);
    }

    #[test]
    fn test_inharmonic_phase_continuity() {
        // Test that the actual synth implementation doesn't create discontinuities
        let mut synth = PianoSynth::new();
        let sample_rate = 44100;
        let num_channels = 1;

        // Initialize synth
        let mut init_buffer = vec![0.0f32; num_channels];
        synth.play(sample_rate, num_channels, &mut init_buffer);

        // Play a note
        let note = wmidi::Note::A4;
        let velocity = wmidi::U7::try_from(100).unwrap();
        synth.note_on(note, velocity, note.to_freq_f32());

        // Generate audio and check for large discontinuities
        const BUFFER_SIZE: usize = 1024;
        let mut buffer = vec![0.0f32; BUFFER_SIZE * num_channels];
        let mut previous_sample = 0.0f32;
        let mut max_discontinuity = 0.0f32;

        // Process several buffers to test phase wrapping
        for _ in 0..100 {
            synth.play(sample_rate, num_channels, &mut buffer);

            for &sample in buffer.iter().step_by(num_channels) {
                let diff = (sample - previous_sample).abs();
                max_discontinuity = max_discontinuity.max(diff);
                previous_sample = sample;
            }
        }

        // Allow for normal audio signal variation but catch phase jumps
        // Piano signals can have sharp attacks, so be reasonable with threshold
        // Actual measured discontinuity is ~0.027, so 0.1 provides good safety margin
        const MAX_ALLOWED_DISCONTINUITY: f32 = 0.1;
        assert!(
            max_discontinuity < MAX_ALLOWED_DISCONTINUITY,
            "Audio discontinuity detected: {max_discontinuity} exceeds threshold {MAX_ALLOWED_DISCONTINUITY}"
        );
    }

    #[test]
    fn test_voice_phase_accumulator_independence() {
        // Test that each partial's phase accumulator works independently
        let mut voice = PianoVoice::new(44100.0);
        let note = wmidi::Note::A4;
        let velocity = wmidi::U7::try_from(100).unwrap();

        voice.note_on(note, velocity, note.to_freq_f32());

        // Process several samples and verify partial phases are different
        for _ in 0..1000 {
            voice.process();
        }

        // Check that partial phases have different values (they advance at different rates)
        let mut all_same = true;
        let first_phase = voice.partial_phases[0];
        for &phase in &voice.partial_phases[1..] {
            if (phase - first_phase).abs() > 0.01 {
                all_same = false;
                break;
            }
        }

        assert!(
            !all_same,
            "All partial phases are the same - inharmonic advancement not working"
        );

        // Verify all phases are in valid range [0.0, 1.0)
        for (i, &phase) in voice.partial_phases.iter().enumerate() {
            assert!(
                (0.0..1.0).contains(&phase),
                "Partial {partial_index} phase {phase} out of range [0.0, 1.0)",
                partial_index = i + 2
            );
        }
    }

    #[test]
    fn test_partial_phase_rem_euclid_no_discontinuities() {
        // Test that rem_euclid phase wrapping doesn't create audio discontinuities
        let mut voice = PianoVoice::new(44100.0);
        let note = wmidi::Note::A4;
        let velocity = wmidi::U7::try_from(100).unwrap();

        voice.note_on(note, velocity, note.to_freq_f32());

        // Track previous sample output and look for discontinuities during phase wrapping
        let mut previous_sample = voice.process();
        let mut max_discontinuity = 0.0f32;
        let mut phase_wraps_detected = 0;

        // Process enough samples to ensure multiple phase wraps for high frequency partials
        // A4 (440Hz) with 8th harmonic (~3520Hz) at 44100Hz sample rate
        // will wrap roughly every 12.5 samples, so 10000 samples ensures many wraps
        for _ in 0..10000 {
            // Store phase values before processing
            let phases_before = voice.partial_phases;

            let current_sample = voice.process();

            // Store phase values after processing
            let phases_after = voice.partial_phases;

            // Check for phase wraps (when phase goes from high value to low value)
            for i in 0..phases_before.len() {
                const WRAP_THRESHOLD: f32 = 0.8; // If phase drops by more than this, it wrapped
                if phases_before[i] > WRAP_THRESHOLD && phases_after[i] < (1.0 - WRAP_THRESHOLD) {
                    phase_wraps_detected += 1;
                }
            }

            // Check for audio discontinuities
            let diff = (current_sample - previous_sample).abs();
            max_discontinuity = max_discontinuity.max(diff);
            previous_sample = current_sample;
        }

        // Verify that we actually detected phase wraps (test is working)
        assert!(
            phase_wraps_detected > 0,
            "No phase wraps detected - test may not be running long enough"
        );

        // Verify that phase wraps don't cause large audio discontinuities
        // Piano signals can have legitimate sharp transients during attack, but
        // phase wrapping artifacts would be much larger
        const MAX_ALLOWED_DISCONTINUITY: f32 = 0.15;
        assert!(
            max_discontinuity < MAX_ALLOWED_DISCONTINUITY,
            "Phase wrapping caused audio discontinuity: {max_discontinuity} exceeds threshold {MAX_ALLOWED_DISCONTINUITY} (detected {phase_wraps_detected} phase wraps)"
        );
    }

    #[test]
    fn test_partial_phase_delta_caching() {
        // Test that partial phase deltas are correctly cached when a note is played
        let mut voice = PianoVoice::new(44100.0);
        let note = wmidi::Note::A4;
        let velocity = wmidi::U7::try_from(100).unwrap();

        // Initially, all cached phase deltas should be 0
        for &delta in &voice.partial_phase_deltas {
            assert_eq!(delta, 0.0, "Initial partial phase delta should be 0");
        }

        voice.note_on(note, velocity, note.to_freq_f32());

        // After note_on, cached phase deltas should be non-zero and different
        let mut all_zero = true;
        let mut all_same = true;
        let first_delta = voice.partial_phase_deltas[0];

        for &delta in &voice.partial_phase_deltas {
            if delta != 0.0 {
                all_zero = false;
            }
            if (delta - first_delta).abs() > 0.0001 {
                all_same = false;
            }
        }

        assert!(
            !all_zero,
            "Cached phase deltas should be non-zero after note_on"
        );
        assert!(
            !all_same,
            "Cached phase deltas should be different for different partials"
        );

        // Verify that cached values are reasonable (positive and less than 1.0 for A4)
        for (i, &delta) in voice.partial_phase_deltas.iter().enumerate() {
            assert!(
                delta > 0.0 && delta < 1.0,
                "Partial {partial_num} phase delta {delta} should be in range (0.0, 1.0)",
                partial_num = i + 2
            );
        }
    }
}
