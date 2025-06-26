use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = registerProcessor)]
    fn register_processor(name: &str, processor_constructor: &JsValue);
}

// Entry point for the worklet
#[wasm_bindgen(start)]
pub fn start() {
    // Use console.log directly since we're in a worklet context
    web_sys::console::log_1(&"Audio worklet start() called".into());
    
    // Add a test to see if we're in the right context
    let global = js_sys::global();
    web_sys::console::log_1(&format!("Global object type: {:?}", global).into());
    
    // Check if AudioWorkletGlobalScope is available
    let has_register_processor = js_sys::Reflect::has(&global, &"registerProcessor".into()).unwrap_or(false);
    web_sys::console::log_1(&format!("Has registerProcessor: {}", has_register_processor).into());
    
    if !has_register_processor {
        web_sys::console::error_1(&"registerProcessor not found in global scope!".into());
        return;
    }
    
    // Create the JavaScript class that extends AudioWorkletProcessor
    let processor_class = js_sys::Function::new_with_args(
        "options",
        r#"
        class AudioProcessorWorklet extends AudioWorkletProcessor {
            constructor(options) {
                super(options);
                console.log("AudioProcessorWorklet constructor called");
                try {
                    // Import the WASM module - Trunk will make it available globally
                    console.log("Creating AudioProcessor instance");
                    this.processor = new AudioProcessor(this.port);
                    console.log("AudioProcessor instance created successfully");
                    
                    // Handle messages from the main thread
                    this.port.onmessage = (event) => {
                        this.processor.handle_message(event.data);
                    };
                } catch (error) {
                    console.error("Failed to create AudioProcessor:", error);
                    throw error;
                }
            }
            
            process(inputs, outputs, parameters) {
                if (!this.processor) {
                    console.error("AudioProcessor not initialized");
                    return true; // Keep the processor alive
                }
                // Note: logging here would be too noisy.
                return this.processor.process(inputs, outputs, parameters);
            }
        }
        return AudioProcessorWorklet;
        "#,
    );

    // Register the processor
    web_sys::console::log_1(&"Registering dissonance-processor".into());
    register_processor("dissonance-processor", &processor_class);
    web_sys::console::log_1(&"dissonance-processor registered successfully".into());
}

fn main() {
    // This main function is required for the binary
    // The actual worklet initialization happens in the start() function
    // which is called automatically by wasm-bindgen
}
