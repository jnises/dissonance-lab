use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct AudioProcessor;

#[wasm_bindgen]
impl AudioProcessor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AudioProcessor {
        AudioProcessor
    }

    #[wasm_bindgen]
    pub fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) -> bool {
        // Process audio here
        for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
            *output = *input; // Pass-through for now
        }
        true
    }
}

fn main() {
    // Set up console panic hook for better error reporting in worklets
    console_error_panic_hook::set_once();
    
    // This function is required for the crate to compile as a binary
    // but won't be called in a WebAudio worklet context
}