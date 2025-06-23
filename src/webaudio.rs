use crate::utils::{FutureData, Task};
use crossbeam::channel::Receiver;
use js_sys::wasm_bindgen::JsValue;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};
use web_sys::{AudioContext, AudioWorkletNode, Blob, BlobPropertyBag};

pub struct WebAudio {
    //context: AudioContext,
    node: Option<FutureData<Result<AudioNodeConnection, JsValue>>>,
    //node: AudioWorkletNode,
}

// SAFETY: we need to send messages from the midi callback. and midir requires Send. JsValue is !Send, but since we aren't using wasm threads that should not be a problem
unsafe impl Send for WebAudio {}

impl WebAudio {
    pub fn new(code: &str) -> Self {
        let mut s = Self { node: None };
        s.set_audio_worklet(code);
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

    pub fn set_audio_worklet(&mut self, code: &str) {
        self.node = None;
        let context = AudioContext::new().unwrap();
        let blob_options = BlobPropertyBag::new();
        blob_options.set_type("application/javascript");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(
            &js_sys::Array::of1(&code.into()),
            &blob_options,
        )
        .unwrap();
        let url = BlobUrl::new(&blob);
        let node = FutureData::spawn(async move {
            JsFuture::from(context.audio_worklet()?.add_module(url.url())?).await?;
            // Create the AudioWorkletNode
            let node = AudioWorkletNode::new(&context, "sine-processor")?;

            // Connect the node to the audio context destination (speakers)
            let connection = AudioNodeConnection::new(context, node);
            // let destination = context.destination();
            // node.connect_with_audio_node(&destination)?;
            // Ok(node)
            Ok(connection)
        });
        self.node = Some(node);
    }

    pub fn error(&self) -> Option<JsValue> {
        self.node
            .as_ref()
            .unwrap()
            .try_get()?
            .as_ref()
            .err()
            .cloned()
    }
}

struct BlobUrl {
    url: String,
}

impl BlobUrl {
    fn new(blob: &Blob) -> Self {
        Self {
            url: web_sys::Url::create_object_url_with_blob(blob).unwrap(),
        }
    }

    fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for BlobUrl {
    fn drop(&mut self) {
        web_sys::Url::revoke_object_url(&self.url).unwrap();
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
        Self { node } //, context}
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
