use crate::{audio::Synth, reverb::Reverb};
use log::info;
use std::f32::consts::PI;

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

    #[inline]
    fn process(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeState::Attack => {
                if let Some(rate) = self.attack_rate {
                    self.current_level += rate;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.state = EnvelopeState::Decay;
                    }
                } else {
                    self.current_level = 1.0;
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
                if self.current_level <= 0.0 {
                    self.current_level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
            EnvelopeState::Release => {
                if let Some(rate) = self.release_rate {
                    self.current_level -= rate * self.current_level;
                    if self.current_level <= 0.0 {
                        self.current_level = 0.0;
                        self.state = EnvelopeState::Idle;
                    }
                } else {
                    self.current_level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
        }

        self.current_level
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
    detuning: f32,   // Slight detuning for realism
    brightness: f32, // Controls harmonic content
}

impl PianoVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            detuned_phase: 0.0,
            phase_delta: 0.0,
            envelope: EnvelopeGenerator::new(0.01, 0.1, 0.7, 0.3, sample_rate),
            sample_rate,
            is_active: false,
            current_key: None,
            detuning: 1.003, // Slight detuning factor
            brightness: 0.8, // 0.0 to 1.0
        }
    }

    fn note_on(&mut self, key: PianoKey) {
        self.current_key = Some(key);
        self.update_phase_delta();

        // TODO: is this how it works? claude seems to think so at least
        // Calculate frequency-dependent sustain decay
        // Higher notes decay faster than lower notes
        if let Some(ref key) = self.current_key {
            // Base decay rate - will be multiplied by frequency factor
            // This value is per sample, so we need to scale it according to sample rate
            let base_decay_rate = 0.00001 * (44100.0 / self.sample_rate);

            // Scale the decay rate based on frequency
            // Higher notes (higher frequency) decay faster
            let freq = key.frequency;
            let freq_factor = (freq / 110.0).sqrt();

            let sustain_decay_rate = base_decay_rate * freq_factor;
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

        self.phase = (self.phase + self.phase_delta).rem_euclid(1.0);
        self.detuned_phase =
            (self.detuned_phase + self.phase_delta * self.detuning).rem_euclid(1.0);

        // AI generated code. sounds fairly good
        // Generate piano-like waveform using multiple harmonics
        let mut sample = 0.0;

        // Fundamental
        sample += 0.6 * (2.0 * PI * self.phase).sin();

        // Second harmonic - quite strong in pianos
        sample += 0.4 * (2.0 * 2.0 * PI * self.phase).sin();

        // Third harmonic
        sample += 0.15 * (3.0 * 2.0 * PI * self.phase).sin();

        // Fourth and fifth harmonics (controlled by brightness)
        let bright_factor = self.brightness * 0.2;
        sample += bright_factor * (4.0 * 2.0 * PI * self.phase).sin();
        sample += bright_factor * 0.7 * (5.0 * 2.0 * PI * self.phase).sin();

        // Detuned oscillator for richness
        sample += 0.1 * (2.0 * PI * self.detuned_phase).sin();

        // Reduce overall volume to prevent clipping
        sample *= 0.3;
        sample *= env_value;

        sample
    }
}

/// Piano synth managing multiple voices for polyphony
pub struct PianoSynth {
    voices: Vec<PianoVoice>,
    sample_rate: Option<u32>,
    rx: crossbeam::channel::Receiver<wmidi::MidiMessage<'static>>,
    reverb: Option<Reverb>,
}

impl PianoSynth {
    pub fn new(rx: crossbeam::channel::Receiver<wmidi::MidiMessage<'static>>) -> Self {
        Self {
            voices: Vec::new(),
            sample_rate: None,
            rx,
            reverb: None,
        }
    }

    fn note_on(&mut self, note: wmidi::Note, _velocity: wmidi::U7) {
        let key = PianoKey::new(note);
        let voice = if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active) {
            voice
        } else {
            // Simple voice stealing - just get the first one
            &mut self.voices[0]
        };
        voice.note_on(key);
    }

    fn note_off(&mut self, midi_note: wmidi::Note) {
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
        let output: f32 = self.voices.iter_mut().map(|v| v.process()).sum();
        output.clamp(-1.0, 1.0)
    }
}

impl Synth for PianoSynth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]) {
        if self.sample_rate != Some(sample_rate) {
            self.voices.clear();
            self.reverb = None;
            self.sample_rate = Some(sample_rate);
        }
        if self.voices.is_empty() {
            const NUM_VOICES: usize = 8;
            self.voices.reserve(NUM_VOICES);
            for _ in 0..NUM_VOICES {
                self.voices.push(PianoVoice::new(sample_rate as f32));
            }
        }
        loop {
            match self.rx.try_recv() {
                Ok(message) => {
                    info!("message received: {message:?}");
                    match message {
                        wmidi::MidiMessage::NoteOff(_channel, note, _velocity) => {
                            self.note_off(note);
                        }
                        wmidi::MidiMessage::NoteOn(_channel, note, velocity) => {
                            self.note_on(note, velocity);
                        }
                        wmidi::MidiMessage::PolyphonicKeyPressure(_, _, _)
                        | wmidi::MidiMessage::ControlChange(_, _, _)
                        | wmidi::MidiMessage::ProgramChange(_, _)
                        | wmidi::MidiMessage::ChannelPressure(_, _)
                        | wmidi::MidiMessage::PitchBendChange(_, _)
                        | wmidi::MidiMessage::SysEx(_)
                        | wmidi::MidiMessage::OwnedSysEx(_)
                        | wmidi::MidiMessage::MidiTimeCode(_)
                        | wmidi::MidiMessage::SongPositionPointer(_)
                        | wmidi::MidiMessage::SongSelect(_)
                        | wmidi::MidiMessage::Reserved(_)
                        | wmidi::MidiMessage::TuneRequest
                        | wmidi::MidiMessage::TimingClock
                        | wmidi::MidiMessage::Start
                        | wmidi::MidiMessage::Continue
                        | wmidi::MidiMessage::Stop
                        | wmidi::MidiMessage::ActiveSensing
                        | wmidi::MidiMessage::Reset => todo!(),
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        for out_channels in out_samples.chunks_exact_mut(num_channels) {
            let s = self.process();
            let s = self
                .reverb
                .get_or_insert_with(|| Reverb::new(sample_rate as f32))
                .process(s);
            for c in out_channels.iter_mut() {
                *c = s;
            }
        }
    }
}
