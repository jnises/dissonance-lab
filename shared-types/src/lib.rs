use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ToWorkletMessage {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    SustainPedal { active: bool },
}

impl From<ToWorkletMessage> for JsValue {
    fn from(value: ToWorkletMessage) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FromWorkletMessage {
    InitComplete,
    InitError { error: String },
}
