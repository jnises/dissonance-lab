use std::f32::consts::PI;

use log::info;

use crate::audio::Synth;

// Piano note frequencies in Hz (A4 = 440Hz)
const A4_FREQ: f32 = 440.0;
const A4_MIDI_NOTE: u8 = 69; // A4 is MIDI note 69

/// Represents a piano key with associated frequency
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
    attack_time: f32,   // seconds
    decay_time: f32,    // seconds
    sustain_level: f32, // 0.0 to 1.0
    release_time: f32,  // seconds
    current_level: f32,
    state: EnvelopeState,
    sample_rate: f32,
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
    fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: f32) -> Self {
        Self {
            attack_time: attack,
            decay_time: decay,
            sustain_level: sustain,
            release_time: release,
            current_level: 0.0,
            state: EnvelopeState::Idle,
            sample_rate,
        }
    }

    fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        // Don't reset level to 0 to allow legato playing
    }

    fn release(&mut self) {
        self.state = EnvelopeState::Release;
    }

    fn process(&mut self) -> f32 {
        // Use a small epsilon value to avoid division by near-zero
        const EPSILON: f32 = 0.000001;
        
        let attack_rate = if self.attack_time > EPSILON {
            1.0 / (self.sample_rate * self.attack_time)
        } else {
            1.0 // Immediate attack
        };
        
        let decay_rate = if self.decay_time > EPSILON {
            (1.0 - self.sustain_level) / (self.sample_rate * self.decay_time)
        } else {
            1.0 // Immediate decay
        };
        
        let release_rate = if self.release_time > EPSILON {
            self.current_level / (self.sample_rate * self.release_time)
        } else {
            1.0 // Immediate release
        };

        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeState::Attack => {
                self.current_level += attack_rate;
                if self.current_level >= 1.0 {
                    self.current_level = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.current_level -= decay_rate;
                if self.current_level <= self.sustain_level {
                    self.current_level = self.sustain_level;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                self.current_level = self.sustain_level;
            }
            EnvelopeState::Release => {
                self.current_level -= release_rate;
                if self.current_level <= 0.0 {
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
    hardness: f32,   // Attack characteristic
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
            hardness: 0.5,   // 0.0 to 1.0
        }
    }

    fn note_on(&mut self, key: PianoKey) {
        self.current_key = Some(key);
        self.update_phase_delta();
        self.envelope.trigger();
        self.is_active = true;
    }

    fn note_off(&mut self) {
        self.envelope.release();
    }

    fn update_phase_delta(&mut self) {
        if let Some(key) = &self.current_key {
            // Calculate phase increment from frequency
            self.phase_delta = key.frequency / self.sample_rate;
        }
    }

    fn process(&mut self) -> f32 {
        if !self.is_active && !self.envelope.is_active() {
            return 0.0;
        }

        // Process envelope
        let env_value = self.envelope.process();
        if !self.envelope.is_active() {
            self.is_active = false;
            return 0.0;
        }

        // Advance phase
        self.phase += self.phase_delta;
        self.phase = self.phase.rem_euclid(1.0);
        self.detuned_phase = (self.detuned_phase + self.phase_delta * self.detuning).rem_euclid(1.0);

        // Generate piano-like waveform using multiple harmonics
        let mut sample = 0.0;

        // Fundamental
        sample += 0.6 * (2.0 * PI * self.phase).sin();

        // Second harmonic (octave) - quite strong in pianos
        sample += 0.4 * (2.0 * 2.0 * PI * self.phase).sin();

        // Third harmonic
        sample += 0.15 * (3.0 * 2.0 * PI * self.phase).sin();

        // Fourth and fifth harmonics (controlled by brightness)
        let bright_factor = self.brightness * 0.2;
        sample += bright_factor * (4.0 * 2.0 * PI * self.phase).sin();
        sample += bright_factor * 0.7 * (5.0 * 2.0 * PI * self.phase).sin();

        // Detuned oscillator for richness
        sample += 0.1 * (2.0 * PI * self.detuned_phase).sin();

        // Normalize and apply envelope
        sample = sample * 0.3; // Reduce overall volume to prevent clipping
        sample *= env_value;

        // Apply characteristic piano attack based on hardness
        let attack_mod = (1.0 - self.hardness).max(0.1);
        if self.envelope.state == EnvelopeState::Attack {
            sample *= attack_mod + (1.0 - attack_mod) * self.envelope.current_level;
        }

        sample
    }
}

/// Piano synth managing multiple voices for polyphony
pub struct PianoSynth {
    voices: Vec<PianoVoice>,
    sample_rate: Option<u32>,
    rx: crossbeam::channel::Receiver<wmidi::MidiMessage<'static>>,
}

impl PianoSynth {
    pub fn new(rx: crossbeam::channel::Receiver<wmidi::MidiMessage<'static>>) -> Self {
        Self {
            voices: Vec::new(),
            sample_rate: None,
            rx,
        }
    }

    fn note_on(&mut self, note: wmidi::Note, _velocity: wmidi::U7) {
        let key = PianoKey::new(note);

        // Find free voice or steal the oldest one
        let voice = if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active) {
            voice
        } else {
            // Simple voice stealing - just get the first one
            &mut self.voices[0]
        };

        voice.note_on(key);
    }

    fn note_off(&mut self, midi_note: wmidi::Note) {
        // Release all voices playing this note
        for voice in self.voices.iter_mut() {
            if let Some(key) = &voice.current_key {
                if key.midi_note == midi_note {
                    voice.note_off();
                }
            }
        }
    }

    // TODO: buffer this all the way
    fn process(&mut self) -> f32 {
        // Mix all active voices
        let mut output = 0.0;
        for voice in self.voices.iter_mut() {
            output += voice.process();
        }

        // Simple limiter to prevent clipping
        output = output.clamp(-1.0, 1.0);

        output
    }
}

impl Synth for PianoSynth {
    fn play(&mut self, sample_rate: u32, num_channels: usize, out_samples: &mut [f32]) {
        if self.sample_rate != Some(sample_rate) {
            self.voices.clear();
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
        for channels in out_samples.chunks_exact_mut(num_channels) {
            let s = self.process();
            for c in channels {
                *c = s;
            }
        }
    }
}
