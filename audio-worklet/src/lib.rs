use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use js_sys::Float32Array;
use serde_wasm_bindgen::from_value;
use dissonance_audio_types::{WorkletMessage, Synth};
use dissonance_audio_engine::PianoSynth;
use crossbeam::channel;

macro_rules! console_log {
    ($($t:tt)*) => {
        web_sys::console::log_1(&format!($($t)*).into());
    };
}

#[wasm_bindgen]
pub struct AudioProcessor {
    sample_rate: f32,
    channels: usize,
    buffer_size: usize,
    synth: Option<PianoSynth>,
    midi_tx: Option<channel::Sender<wmidi::MidiMessage<'static>>>,
    midi_rx: Option<channel::Receiver<wmidi::MidiMessage<'static>>>,
}

#[wasm_bindgen]
impl AudioProcessor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_log!("AudioProcessor created");
        let (midi_tx, midi_rx) = channel::unbounded();
        Self {
            sample_rate: 44100.0,
            channels: 2,
            buffer_size: 128,
            synth: None,
            midi_tx: Some(midi_tx),
            midi_rx: Some(midi_rx),
        }
    }

    #[wasm_bindgen]
    pub fn process(&mut self, _inputs: &js_sys::Array, outputs: &js_sys::Array) -> bool {
        let output = outputs.get(0);
        if output.is_undefined() {
            return true;
        }

        let output_array = js_sys::Array::from(&output);
        if output_array.length() == 0 {
            return true;
        }

        let channel_data = Float32Array::from(output_array.get(0));
        let samples_per_channel = channel_data.length() as usize;

        // Initialize synth if not already done
        if self.synth.is_none() {
            if let Some(rx) = self.midi_rx.take() {
                self.synth = Some(PianoSynth::new(rx));
            }
        }

        // Generate audio samples
        let mut samples = vec![0.0f32; samples_per_channel * self.channels];
        
        if let Some(ref mut synth) = self.synth {
            synth.play(self.sample_rate as u32, self.channels, &mut samples);
        }

        // Copy samples to output
        for ch in 0..self.channels.min(output_array.length() as usize) {
            let channel_data = Float32Array::from(output_array.get(ch as u32));
            for i in 0..samples_per_channel.min(channel_data.length() as usize) {
                channel_data.set_index(i as u32, samples[i * self.channels + ch]);
            }
        }

        true
    }

    #[wasm_bindgen]
    pub fn handle_message(&mut self, event: &MessageEvent) {
        if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
            if let Ok(message) = from_value::<WorkletMessage>(data.into()) {
                match message {
                    WorkletMessage::Config(config) => {
                        console_log!("Received config: {:?}", config);
                        self.sample_rate = config.sample_rate;
                        self.channels = config.channels;
                        self.buffer_size = config.buffer_size;
                    }
                    WorkletMessage::MidiMessage(midi_msg) => {
                        console_log!("Received MIDI message: {:?}", midi_msg);
                        // Convert MidiMsg back to wmidi::MidiMessage and send to synth
                        if let Ok(wmidi_msg) = midi_msg.try_into() {
                            if let Some(ref tx) = self.midi_tx {
                                let _ = tx.try_send(wmidi_msg);
                            }
                        }
                    }
                    WorkletMessage::RequestAudio { buffer_size: _ } => {
                        // Generate audio data and send back
                        // This will be integrated with the actual synthesizer
                    }
                }
            }
        }
    }
}
