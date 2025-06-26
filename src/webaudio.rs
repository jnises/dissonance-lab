use crate::utils::FutureData;
use js_sys::wasm_bindgen::JsValue;
pub use shared_types::{FromWorkletMessage, ToWorkletMessage};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode, MessageEvent};

pub struct WebAudio {
    //context: AudioContext,
    node: Option<FutureData<Result<AudioNodeConnection, JsValue>>>,
    //node: AudioWorkletNode,
}

// SAFETY: we need to send messages from the midi callback. and midir requires Send. JsValue is !Send, but since we aren't using wasm threads that should not be a problem
unsafe impl Send for WebAudio {}

impl WebAudio {
    pub fn new() -> Self {
        let mut s = Self { node: None };
        // Load the audio worklet WASM instead of JavaScript
        s.set_audio_worklet_wasm();
        s
    }

    pub fn send_message(&self, message: ToWorkletMessage) {
        // it might take a while to load the worklet, so early messages might get a None from try_get
        if let Some(node) = self
            .node
            .as_ref()
            .expect("Audio worklet node not initialized")
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

    pub fn set_audio_worklet_wasm(&mut self) {
        self.node = None;
        let context = AudioContext::new().unwrap();

        // Load the audio worklet WASM module
        let node = FutureData::spawn(async move {
            // Load the audio worklet built by Trunk
            let worklet_url = "./audio-worklet.js";
            log::info!("Loading audio worklet from: {}", worklet_url);
            
            match JsFuture::from(context.audio_worklet()?.add_module(worklet_url)?).await {
                Ok(_) => {
                    log::info!("Audio worklet module loaded successfully");
                    // Add a small delay to ensure the processor is registered
                    let delay_promise = js_sys::Promise::new(&mut |resolve, _| {
                        let callback = Closure::once_into_js(move || resolve.call0(&JsValue::NULL));
                        web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                            callback.as_ref().unchecked_ref(), 
                            50  // 50ms delay
                        ).unwrap();
                    });
                    let _ = JsFuture::from(delay_promise).await;
                }
                Err(e) => {
                    log::error!("Failed to load audio worklet module: {:?}", e);
                    return Err(e);
                }
            }

            // Create the AudioWorkletNode
            log::info!("Creating AudioWorkletNode with processor 'dissonance-processor'");
            let node = match AudioWorkletNode::new(&context, "dissonance-processor") {
                Ok(node) => {
                    log::info!("AudioWorkletNode created successfully");
                    node
                }
                Err(e) => {
                    log::error!("Failed to create AudioWorkletNode: {:?}", e);
                    return Err(e);
                }
            };

            // Connect the node to the audio context destination (speakers)
            let connection = AudioNodeConnection::new(context, node);
            Ok(connection)
        });
        self.node = Some(node);
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
            let msg = serde_wasm_bindgen::from_value(event.data())
                .expect("Failed to deserialize message from audio worklet");
            match msg {
                FromWorkletMessage::Log(msg) => {
                    log::info!("[audio-worklet] {}", msg);
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
