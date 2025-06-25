use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_name = registerProcessor)]
    fn register_processor(name: &str, processor_constructor: &JsValue);
}

// Entry point for the worklet
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    log("foo");

    // Create the JavaScript class that extends AudioWorkletProcessor
    let processor_class = js_sys::Function::new_with_args(
        "options",
        r#"
        class AudioProcessorWorklet extends AudioWorkletProcessor {
            constructor(options) {
                super(options);
                // Import the WASM module - Trunk will make it available globally
                this.processor = new AudioProcessor();
                
                // Handle messages from the main thread
                this.port.onmessage = (event) => {
                    this.processor.handle_message(event.data);
                };
            }
            
            process(inputs, outputs, parameters) {
                return this.processor.process(inputs, outputs, parameters);
            }
        }
        return AudioProcessorWorklet;
        "#,
    );

    // Register the processor
    register_processor("dissonance-processor", &processor_class);
}

fn main() {
    // This main function is required for the binary
    // The actual worklet initialization happens in the start() function
    // which is called automatically by wasm-bindgen
}
