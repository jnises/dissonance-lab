use js_sys::{Array, Float32Array, Object};
use shared_types::ToWorkletMessage;
use wasm_bindgen::prelude::*;
use web_sys::{AudioWorkletGlobalScope, MessagePort};

pub mod limiter;
pub mod reverb;
pub mod synth;

pub use synth::Synth;

// This is called when the module is loaded
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    
    // Use console_log just like main crate, now that we have TextDecoder shim
    #[cfg(debug_assertions)]
    console_log::init_with_level(log::Level::Debug).expect("error initializing log");
    #[cfg(not(debug_assertions))]
    console_log::init_with_level(log::Level::Info).expect("error initializing log");
}

#[wasm_bindgen]
pub struct DissonanceProcessor {
    synth: synth::PianoSynth,
    sample_rate: f32,
    port: Option<MessagePort>,
    interleaved_buffer_cache: Vec<f32>,
    channel_buffer_cache: Vec<f32>,
}

#[wasm_bindgen]
impl DissonanceProcessor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> DissonanceProcessor {
        let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
        let sample_rate = global.sample_rate();

        let processor = DissonanceProcessor {
            synth: synth::PianoSynth::new(),
            sample_rate,
            port: None,
            interleaved_buffer_cache: Vec::new(),
            channel_buffer_cache: Vec::new(),
        };

        log::debug!("DissonanceProcessor constructor initialized");
        processor
    }

    #[wasm_bindgen]
    pub fn set_port(&mut self, port: MessagePort) {
        self.port = Some(port);
        log::debug!("Port set successfully");
    }

    #[wasm_bindgen]
    pub fn handle_message(&mut self, message: JsValue) {
        let msg = serde_wasm_bindgen::from_value::<ToWorkletMessage>(message).unwrap();
        match msg {
            ToWorkletMessage::NoteOn { note, velocity } => {
                log::debug!("NoteOn: note={note}, velocity={velocity}");
                let midi_note = wmidi::Note::try_from(note).expect("Invalid MIDI note value");
                let midi_velocity = wmidi::U7::try_from(velocity).unwrap_or(wmidi::U7::MAX);
                self.synth.note_on(midi_note, midi_velocity);
            }
            ToWorkletMessage::NoteOff { note } => {
                log::debug!("NoteOff: note={note}");
                let midi_note = wmidi::Note::try_from(note).expect("Invalid MIDI note value");
                self.synth.note_off(midi_note);
            }
        }
    }

    // This is the main processing method called by the Web Audio API
    #[wasm_bindgen]
    pub fn process(&mut self, _inputs: Array, outputs: Array, _parameters: Object) -> bool {
        // Web Audio API guarantees outputs[0] exists and is an Array
        let output_array: Array = outputs.get(0).into();
        let num_channels = output_array.length() as usize;

        if num_channels > 0 {
            // Web Audio API guarantees each channel is a Float32Array
            let first_channel: Float32Array = output_array.get(0).into();
            let buffer_length = first_channel.length() as usize;

            // TODO: avoid the interleaving to fit better with the webaudio audioprocessor api

            // Create interleaved buffer for all channels
            let interleaved_len = buffer_length * num_channels;
            if self.interleaved_buffer_cache.len() != interleaved_len {
                self.interleaved_buffer_cache.resize(interleaved_len, 0f32);
            }

            // Generate audio with proper channel count
            self.synth.play(
                self.sample_rate as u32,
                num_channels,
                &mut self.interleaved_buffer_cache,
            );

            // De-interleave and copy to output channels
            for channel in 0..num_channels {
                let output_channel: Float32Array = output_array.get(channel as u32).into();
                if self.channel_buffer_cache.len() != buffer_length {
                    self.channel_buffer_cache.resize(buffer_length, 0.0);
                }

                // Extract samples for this channel from interleaved buffer
                for (frame_nr, sample) in self.channel_buffer_cache.iter_mut().enumerate() {
                    *sample = self.interleaved_buffer_cache[frame_nr * num_channels + channel];
                }

                // Copy to output
                output_channel.copy_from(&self.channel_buffer_cache);
            }
        }

        true // Continue processing
    }
}

impl Default for DissonanceProcessor {
    fn default() -> Self {
        Self::new()
    }
}
