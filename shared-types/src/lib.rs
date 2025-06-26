use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ToWorkletMessage {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

impl From<ToWorkletMessage> for JsValue {
    fn from(value: ToWorkletMessage) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum FromWorkletMessage {
    Log(String),
}

impl From<FromWorkletMessage> for JsValue {
    fn from(value: FromWorkletMessage) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}