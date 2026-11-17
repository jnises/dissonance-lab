use crate::utils::FutureData;
use js_sys::wasm_bindgen::JsValue;
use serde::Serialize;
use shared_types::FromWorkletMessage;
pub use shared_types::ToWorkletMessage;
use std::{cell::Cell, rc::Rc};
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
    init_failure_logged: Cell<bool>,
}

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
            init_failure_logged: Cell::new(false),
        }
    }

    pub fn send_message(&self, message: ToWorkletMessage) {
        if let Some(node) = self.node.try_get()
            && let Ok(connection) = node
            && connection.status.get() == WorkletStatus::Ready
            && let Err(e) = connection
                .node
                .port()
                .and_then(|port| port.post_message(&message.into()))
        {
            log::error!("Failed to send message to audio worklet: {e:?}");
        }
    }

    pub fn is_disabled(&self) -> bool {
        match self.node.try_get() {
            Some(Err(e)) => {
                if !self.init_failure_logged.replace(true) {
                    log::error!("Audio worklet initialization failed: {e:?}");
                }
                true
            }
            Some(Ok(connection)) => connection.status.get() == WorkletStatus::Failed,
            None => false,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(
            self.node.try_get(),
            Some(Ok(connection)) if connection.status.get() == WorkletStatus::Ready
        )
    }

    /// Try to resume a context suspended by the browser's autoplay policy.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkletStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug)]
struct AudioNodeConnection {
    context: AudioContext,
    node: AudioWorkletNode,
    status: Rc<Cell<WorkletStatus>>,
    // These callbacks must live as long as their JS event handlers.
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onprocessorerror: Closure<dyn FnMut()>,
}

impl AudioNodeConnection {
    fn new(context: AudioContext, node: AudioWorkletNode) -> Self {
        let destination = context.destination();
        node.connect_with_audio_node(&destination).unwrap();

        let port = node.port().unwrap();
        let status = Rc::new(Cell::new(WorkletStatus::Loading));
        let message_status = Rc::clone(&status);
        let onmessage = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
            match serde_wasm_bindgen::from_value(event.data()) {
                Ok(FromWorkletMessage::InitComplete) => {
                    if message_status.get() == WorkletStatus::Loading {
                        message_status.set(WorkletStatus::Ready);
                        log::debug!("[audio-worklet] Initialization complete");
                    }
                }
                Ok(FromWorkletMessage::InitError { error }) => {
                    message_status.set(WorkletStatus::Failed);
                    log::error!("[audio-worklet] Initialization error: {error}");
                }
                Err(error) => log::warn!("Unexpected audio worklet message: {error}"),
            }
        });
        port.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        let processor_status = Rc::clone(&status);
        let onprocessorerror = Closure::<dyn FnMut()>::new(move || {
            processor_status.set(WorkletStatus::Failed);
            log::error!("[audio-worklet] Browser reported a processor error");
        });
        node.set_onprocessorerror(Some(onprocessorerror.as_ref().unchecked_ref()));

        Self {
            context,
            node,
            status,
            _onmessage: onmessage,
            _onprocessorerror: onprocessorerror,
        }
    }
}

impl Drop for AudioNodeConnection {
    fn drop(&mut self) {
        self.node.port().unwrap().set_onmessage(None);
        self.node.set_onprocessorerror(None);
        self.node.disconnect().unwrap();
    }
}
