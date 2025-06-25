use js_sys::{Array, Float32Array, Object};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub mod limiter;
pub mod reverb;
pub mod synth;

pub use synth::Synth;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

#[wasm_bindgen]
pub struct AudioProcessor {
    synth: synth::PianoSynth,
    sample_rate: f32,
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl AudioProcessor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AudioProcessor {
        // Set up console panic hook for better error reporting in worklets
        // console_error_panic_hook::set_once();

        // Create a channel for MIDI messages (though we won't use the sender in this context)
        let (_tx, rx) = crossbeam::channel::unbounded();

        AudioProcessor {
            synth: synth::PianoSynth::new(rx),
            sample_rate: 44100.0, // Default sample rate, will be updated when available
        }
    }

    #[wasm_bindgen]
    pub fn handle_message(&mut self, message: JsValue) {
        if let Ok(msg) = serde_wasm_bindgen::from_value::<Message>(message) {
            match msg {
                Message::NoteOn { note, velocity } => {
                    if let Ok(midi_note) = wmidi::Note::try_from(note) {
                        let midi_velocity = wmidi::U7::try_from(velocity).unwrap_or(wmidi::U7::MAX);
                        self.synth.note_on(midi_note, midi_velocity);
                    }
                }
                Message::NoteOff { note } => {
                    if let Ok(midi_note) = wmidi::Note::try_from(note) {
                        self.synth.note_off(midi_note);
                    }
                }
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
            let mut interleaved_buffer = vec![0.0f32; buffer_length * num_channels];
            
            // Generate audio with proper channel count
            self.synth.play(
                self.sample_rate as u32, 
                num_channels, 
                &mut interleaved_buffer
            );
            
            // De-interleave and copy to output channels
            for channel in 0..num_channels {
                let output_channel: Float32Array = output_array.get(channel as u32).into();
                let mut channel_buffer = vec![0.0f32; buffer_length];
                
                // Extract samples for this channel from interleaved buffer
                for frame in 0..buffer_length {
                    channel_buffer[frame] = interleaved_buffer[frame * num_channels + channel];
                }
                
                // Copy to output
                output_channel.copy_from(&channel_buffer);
            }
        }

        true // Continue processing
    }
}

// JavaScript class that extends AudioWorkletProcessor
// This will be created dynamically in the binary's start function
