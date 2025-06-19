use anyhow::{Result, anyhow};
use web_sys::{AudioContext, AudioWorkletNode};
use std::sync::Arc;
use crossbeam::atomic::AtomicCell;
use log::warn;
use dissonance_audio_types::{Synth, WorkletMessage, MidiMsg};
use serde_wasm_bindgen::to_value;

pub struct AudioManager {
    context: Option<AudioContext>,
    worklet_node: Option<AudioWorkletNode>,
    buffer_size: Arc<AtomicCell<u32>>,
    error_callback: Arc<Box<dyn Fn(String) + Send + Sync>>,
    synth: Option<Box<dyn Synth + Send + Sync>>,
}

impl AudioManager {
    pub fn new<U>(synth: Box<dyn Synth + Send + Sync>, error_callback: U) -> Self
    where
        U: Fn(String) + Send + Sync + 'static,
    {
        let mut s = Self {
            context: None,
            worklet_node: None,
            buffer_size: Arc::new(AtomicCell::new(128)),
            error_callback: Arc::new(Box::new(error_callback)),
            synth: Some(synth),
        };
        s.setup();
        s
    }

    fn setup(&mut self) {
        let r = (|| -> Result<_> {
            // Create AudioContext
            let context = AudioContext::new().map_err(|e| anyhow!("Failed to create AudioContext: {:?}", e))?;
            
            // For now, just store the context
            // The worklet loading would be async and more complex
            self.context = Some(context);
            
            warn!("WebAudio context created. Full AudioWorklet integration requires async setup.");
            Ok(())
        })();
        
        if let Err(e) = r {
            (self.error_callback)(format!("WebAudio setup error: {:?}", e));
        }
    }

    // Method to send MIDI messages to the worklet (when implemented)
    pub fn send_midi_message(&self, message: wmidi::MidiMessage<'static>) {
        if let Some(ref _worklet_node) = self.worklet_node {
            let midi_msg = MidiMsg::from(message);
            if let Ok(_msg) = to_value(&WorkletMessage::MidiMessage(midi_msg)) {
                // Send message to worklet port
                // This would require proper port setup
                warn!("MIDI message ready to send to worklet");
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_name(&self) -> Option<String> {
        Some("WebAudio".to_string())
    }
}
