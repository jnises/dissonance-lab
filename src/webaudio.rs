use crate::utils::FutureData;
use js_sys::wasm_bindgen::JsValue;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContext, AudioWorkletNode};

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

    pub fn send_message(&self, message: Message) {
        if let Some(node) = self.node.as_ref().unwrap().try_get() {
            if let Ok(connection) = node.as_ref() {
                connection
                    .node
                    .port()
                    .unwrap()
                    .post_message(&message.into())
                    .unwrap();
            }
        }
    }

    pub fn set_audio_worklet_wasm(&mut self) {
        self.node = None;
        let context = AudioContext::new().unwrap();

        // Load the audio worklet WASM module
        let node = FutureData::spawn(async move {
            // Load the audio worklet built by Trunk
            let worklet_url = "./audio-worklet.js";
            JsFuture::from(context.audio_worklet()?.add_module(worklet_url)?).await?;

            // Create the AudioWorkletNode
            let node = AudioWorkletNode::new(&context, "dissonance-processor")?;

            // Connect the node to the audio context destination (speakers)
            let connection = AudioNodeConnection::new(context, node);
            Ok(connection)
        });
        self.node = Some(node);
    }
}

#[derive(Debug)]
struct AudioNodeConnection {
    //context: AudioContext,
    node: AudioWorkletNode,
}

impl AudioNodeConnection {
    fn new(context: AudioContext, node: AudioWorkletNode) -> Self {
        let destination = context.destination();
        node.connect_with_audio_node(&destination).unwrap();
        Self { node } //, context
    }
}

impl Drop for AudioNodeConnection {
    fn drop(&mut self) {
        self.node.disconnect().unwrap();
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

impl From<Message> for JsValue {
    fn from(value: Message) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}
