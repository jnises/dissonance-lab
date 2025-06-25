use wasm_bindgen::prelude::*;
use js_sys::{Array, Float32Array, Object};

pub mod limiter;
pub mod reverb;
pub mod synth;

pub use synth::Synth;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct AudioProcessor {
    // You can add internal state here if needed
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
        console_error_panic_hook::set_once();
        
        AudioProcessor {}
    }

    // This is the main processing method called by the Web Audio API
    #[wasm_bindgen]
    pub fn process(
        &mut self,
        inputs: Array,
        outputs: Array,
        _parameters: Object,
    ) -> bool {
        // Get the first input and output
        if let (Some(input_array), Some(output_array)) = (
            inputs.get(0).dyn_into::<Array>().ok(),
            outputs.get(0).dyn_into::<Array>().ok(),
        ) {
            // Process each channel
            for channel in 0..input_array.length().min(output_array.length()) {
                if let (Some(input_channel), Some(output_channel)) = (
                    input_array.get(channel).dyn_into::<Float32Array>().ok(),
                    output_array.get(channel).dyn_into::<Float32Array>().ok(),
                ) {
                    // Get the buffer length
                    let buffer_length = input_channel.length() as usize;
                    
                    // Create a temporary buffer for processing
                    let mut temp_buffer = vec![0.0f32; buffer_length];
                    
                    // Copy input data to temp buffer
                    input_channel.copy_to(&mut temp_buffer);
                    
                    // Process the audio (pass-through for now)
                    // This is where you would add your audio processing logic
                    // For example: apply effects, synthesis, etc.
                    
                    // Copy processed data back to output
                    output_channel.copy_from(&temp_buffer);
                }
            }
        }
        
        true // Continue processing
    }
}

// JavaScript class that extends AudioWorkletProcessor
// This will be created dynamically in the binary's start function

