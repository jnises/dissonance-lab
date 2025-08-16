use crate::utils::FutureData;
use js_sys::wasm_bindgen::JsValue;
use serde::Serialize;
pub use shared_types::{FromWorkletMessage, ToWorkletMessage};
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
    message_attempt_count: std::cell::Cell<u32>,
    init_failure_logged: std::cell::Cell<bool>,
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
            let worklet_url = "./dissonance_worklet_processor.js";
            log::debug!("Loading audio worklet from: {worklet_url}");

            // Load the WASM bytes and JS glue code
            let wasm_url = "./audio-worklet_bg.wasm";
            let js_url = "./audio-worklet.js";

            log::debug!("Loading WASM bytes from: {wasm_url}");
            let wasm_response =
                JsFuture::from(web_sys::window().unwrap().fetch_with_str(wasm_url)).await?;
            let wasm_response: web_sys::Response = wasm_response.dyn_into()?;
            let wasm_bytes = JsFuture::from(wasm_response.array_buffer()?).await?;

            log::debug!("Loading JS glue code from: {js_url}");
            let js_response =
                JsFuture::from(web_sys::window().unwrap().fetch_with_str(js_url)).await?;
            let js_response: web_sys::Response = js_response.dyn_into()?;
            let js_glue_code = JsFuture::from(js_response.text()?).await?;

            let context = AudioContext::new()
                .map_err(|e| JsValue::from_str(&format!("Failed to create AudioContext: {e:?}")))?;

            // Check if AudioWorklet is supported by checking if the property exists
            let worklet_js = js_sys::Reflect::get(&context, &JsValue::from_str("audioWorklet"))
                .map_err(|_| JsValue::from_str("Failed to check AudioWorklet support"))?;

            if worklet_js.is_undefined() || worklet_js.is_null() {
                return Err(JsValue::from_str(
                    "AudioWorklet is not supported in this browser. This is common on mobile devices or older browsers. The visual interface will work, but audio playback is disabled.",
                ));
            }

            // Now try to get the audio worklet
            let audio_worklet = context
                .audio_worklet()
                .map_err(|e| JsValue::from_str(&format!("Failed to get AudioWorklet: {e:?}")))?;

            JsFuture::from(audio_worklet.add_module(worklet_url)?)
                .await
                .map_err(|e| {
                    JsValue::from_str(&format!("Failed to load audio worklet module: {e:?}"))
                })?;

            // Create processor options with WASM data using serde
            let processor_options = ProcessorOptions {
                wasm_bytes,
                js_glue_code,
            };

            let processor_options_js =
                serde_wasm_bindgen::to_value(&processor_options).map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize processor options: {e}"))
                })?;

            // Convert to js_sys::Object for web-sys compatibility
            let processor_options_obj: js_sys::Object = processor_options_js
                .dyn_into()
                .map_err(|_| JsValue::from_str("Failed to convert processor options to Object"))?;

            log::debug!("Creating AudioWorkletNode with processor 'dissonance-processor'");

            let worklet_options = web_sys::AudioWorkletNodeOptions::new();
            worklet_options.set_processor_options(Some(&processor_options_obj));

            let node = match AudioWorkletNode::new_with_options(
                &context,
                "dissonance-processor",
                &worklet_options,
            ) {
                Ok(node) => {
                    log::debug!("AudioWorkletNode created successfully");
                    node
                }
                Err(e) => {
                    log::error!("Failed to create AudioWorkletNode: {e:?}");
                    return Err(e);
                }
            };

            // Connect the node to the audio context destination (speakers)
            let connection = AudioNodeConnection::new(context, node);
            Ok(connection)
        });
        Self {
            node,
            message_attempt_count: std::cell::Cell::new(0),
            init_failure_logged: std::cell::Cell::new(false),
        }
    }

    pub fn send_message(&self, message: ToWorkletMessage) {
        // it might take a while to load the worklet, so early messages might get a None from try_get
        if let Some(node) = self.node.try_get() {
            match node.as_ref() {
                Ok(connection) => {
                    match connection.node.port() {
                        Ok(port) => {
                            if let Err(e) = port.post_message(&message.into()) {
                                log::error!("Failed to send message to audio worklet: {e:?}");
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to get audio worklet port: {e:?}");
                        }
                    }
                    self.message_attempt_count.set(0);
                }
                Err(e) => {
                    // AudioWorklet failed to initialize, log the error once and stop trying
                    if !self.init_failure_logged.get() {
                        log::info!("Audio worklet initialization failed: {e:?}");
                        log::info!(
                            "Audio functionality is disabled. The visual interface remains fully functional."
                        );
                    }
                    self.init_failure_logged.set(true);
                }
            }
        } else {
            let count = self.message_attempt_count.get();
            self.message_attempt_count.set(count + 1);

            // Log warning after 100 attempts (arbitrary threshold)
            if count == 100 {
                log::warn!(
                    "Audio worklet still not ready after {count} message attempts. This may indicate a loading problem."
                );
            } else if count > 100 && count % 50 == 0 {
                log::warn!("Audio worklet still not ready after {count} message attempts.");
            }
        }
    }

    /// Check if audio is disabled (AudioWorklet failed to initialize)
    pub fn is_disabled(&self) -> bool {
        if let Some(node) = self.node.try_get() {
            node.is_err()
        } else {
            false // Still loading
        }
    }

    /// Check if the audio worklet finished initializing successfully
    ///
    /// Returns true only once the underlying Future has resolved Ok
    pub fn is_ready(&self) -> bool {
        if let Some(node) = self.node.try_get() {
            node.is_ok()
        } else {
            false // Still loading
        }
    }

    /// Ensure the underlying AudioContext is in a running state.
    ///
    /// Browsers may start the context suspended until a user gesture occurs. When we
    /// auto-initialize audio at startup this can result in a "playing" state with no sound
    /// until the user clicks mute/unmute. Calling this before sending note messages will
    /// until the user performs a gesture (such as clicking mute/unmute or pressing a piano key).
    /// Calling this before sending note messages will resume the context once a gesture has occurred
    /// (e.g., piano key press, mute/unmute click, or any other user interaction).
    pub fn ensure_running(&self) {
        if let Some(node) = self.node.try_get()
            && let Ok(connection) = node.as_ref()
        {
            // We don't have access to AudioContext.state via web-sys yet on all targets, so just attempt resume.
            // Browsers ignore resume() if already running.
            if let Err(e) = connection.context.resume() {
                // Ignored: can fail prior to a valid user gesture; we'll try again next event.
                log::debug!("AudioContext resume attempt failed or deferred: {e:?}");
            }
        }
    }
}

#[derive(Debug)]
struct AudioNodeConnection {
    context: AudioContext,
    node: AudioWorkletNode,
    // needs to be kept alive
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
            if data.is_object()
                && let Ok(type_val) = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
                && let Some(type_str) = type_val.as_string()
            {
                match type_str.as_str() {
                    "init-complete" => {
                        log::debug!("[audio-worklet] Initialization complete");
                        return;
                    }
                    "init-error" => {
                        if let Ok(error_val) =
                            js_sys::Reflect::get(&data, &JsValue::from_str("error"))
                            && let Some(error_str) = error_val.as_string()
                        {
                            log::error!("[audio-worklet] Initialization error: {error_str}");
                        }
                        return;
                    }
                    _ => {}
                }
            }

            // Try to deserialize as FromWorkletMessage for other messages
            if let Ok(msg) = serde_wasm_bindgen::from_value::<FromWorkletMessage>(data) {
                match msg {
                    // no messages sent back from the worklet currently
                }
            }
        });
        port.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        Self {
            context,
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
