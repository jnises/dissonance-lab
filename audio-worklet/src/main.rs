use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = registerProcessor)]
    fn register_processor(name: &str, processor_constructor: &JsValue);
}

fn main() {
    // This main function is required for the binary
    // The actual worklet initialization happens in the start() function
    // which is called automatically by wasm-bindgen
}
