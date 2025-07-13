use crate::utils::FutureData;
use js_sys::wasm_bindgen::JsValue;
pub use shared_types::{FromWorkletMessage, ToWorkletMessage};
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode, MessageEvent};

#[derive(Serialize)]
struct ProcessorOptions {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    #[serde(rename = "wasmBytes")]
    wasm_bytes: JsValue,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    #[serde(rename = "jsGlueCode")]
    js_glue_code: JsValue,
}

pub struct WebAudio {
    node: FutureData<Result<AudioNodeConnection, JsValue>>,
}

// SAFETY: we need to send messages from the midi callback. and midir requires Send. JsValue is !Send, but since we aren't using wasm threads that should not be a problem
unsafe impl Send for WebAudio {}

impl Default for WebAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl WebAudio {
    pub fn new() -> Self {
        // Load the audio worklet WASM module
        let node = FutureData::spawn(async move {
            // Load the audio worklet JavaScript wrapper
            let worklet_url = "./worklet.js";
            log::debug!("Loading audio worklet from: {worklet_url}");
            
            // Load the WASM bytes and JS glue code
            let wasm_url = "./audio-worklet_bg.wasm";
            let js_url = "./audio-worklet.js";
            
            log::debug!("Loading WASM bytes from: {wasm_url}");
            let wasm_response = JsFuture::from(
                web_sys::window().unwrap().fetch_with_str(wasm_url)
            ).await?;
            let wasm_response: web_sys::Response = wasm_response.dyn_into()?;
            let wasm_bytes = JsFuture::from(wasm_response.array_buffer()?).await?;
            
            log::debug!("Loading JS glue code from: {js_url}");
            let js_response = JsFuture::from(
                web_sys::window().unwrap().fetch_with_str(js_url)
            ).await?;
            let js_response: web_sys::Response = js_response.dyn_into()?;
            let js_glue_code = JsFuture::from(js_response.text()?).await?;
            
            let context = AudioContext::new().unwrap();
            JsFuture::from(context.audio_worklet()?.add_module(worklet_url)?).await?;

            // Create processor options with WASM data using serde
            let processor_options = ProcessorOptions {
                wasm_bytes,
                js_glue_code,
            };
            
            // Serialize the processor options to a JS object
            let processor_options_js = serde_wasm_bindgen::to_value(&processor_options)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize processor options: {e}")))?;
            
            // Convert to js_sys::Object for web-sys compatibility
            let processor_options_obj: js_sys::Object = processor_options_js.dyn_into()
                .map_err(|_| JsValue::from_str("Failed to convert processor options to Object"))?;

            // Create the AudioWorkletNode with options using new_with_options
            log::debug!("Creating AudioWorkletNode with processor 'dissonance-processor'");
            
            // Clone context for later use before it's moved
            let context_clone = context.clone();
            
            // Create AudioWorkletNodeOptions and set processor options
            let worklet_options = web_sys::AudioWorkletNodeOptions::new();
            worklet_options.set_processor_options(Some(&processor_options_obj));
            
            // Create the node with options
            let node = match AudioWorkletNode::new_with_options(&context, "dissonance-processor", &worklet_options) {
                Ok(node) => {
                    log::info!("AudioWorkletNode created successfully");
                    node
                }
                Err(e) => {
                    log::error!("Failed to create AudioWorkletNode: {e:?}");
                    return Err(e);
                }
            };

            // Connect the node to the audio context destination (speakers)
            let connection = AudioNodeConnection::new(context_clone, node);
            Ok(connection)
        });
        Self { node }
    }

    pub fn send_message(&self, message: ToWorkletMessage) {
        // it might take a while to load the worklet, so early messages might get a None from try_get
        // TODO: if we get None after a certain number of seconds after loading the module, report the error 
        if let Some(node) = self
            .node
            .try_get()
        {
            let connection = node.as_ref().expect("Audio worklet connection failed");
            connection
                .node
                .port()
                .expect("Failed to get audio worklet port")
                .post_message(&message.into())
                .expect("Failed to send message to audio worklet");
        }
    }
}

#[derive(Debug)]
struct AudioNodeConnection {
    node: AudioWorkletNode,
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
}

impl AudioNodeConnection {
    fn new(context: AudioContext, node: AudioWorkletNode) -> Self {
        let destination = context.destination();
        node.connect_with_audio_node(&destination).unwrap();

        let port = node.port().unwrap();
        let onmessage = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
            let data = event.data();
            
            // Try to get the message type first
            if data.is_object() {
                if let Ok(type_val) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
                    if let Some(type_str) = type_val.as_string() {
                        match type_str.as_str() {
                            "init-complete" => {
                                log::info!("[audio-worklet] Initialization complete");
                                return;
                            }
                            "init-error" => {
                                if let Ok(error_val) = js_sys::Reflect::get(&data, &JsValue::from_str("error")) {
                                    if let Some(error_str) = error_val.as_string() {
                                        log::error!("[audio-worklet] Initialization error: {error_str}");
                                    }
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            // Try to deserialize as FromWorkletMessage for other messages
            if let Ok(msg) = serde_wasm_bindgen::from_value::<FromWorkletMessage>(data) {
                match msg {
                    FromWorkletMessage::Log { message } => {
                        log::info!("[audio-worklet] {message}");
                    }
                }
            }
        });
        port.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        Self {
            node,
            _onmessage: onmessage,
        }
    }
}

impl Drop for AudioNodeConnection {
    fn drop(&mut self) {
        self.node.port().unwrap().set_onmessage(None);
        self.node.disconnect().unwrap();
    }
}
