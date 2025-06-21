use anyhow::{Result, anyhow};
use web_sys::{AudioContext, AudioWorkletNode, MessagePort};
use std::sync::Arc;
use crossbeam::atomic::AtomicCell;
use log::{warn, info, error};
use dissonance_audio_types::Synth;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct AudioManager {
    inner: Arc<Mutex<AudioManagerInner>>,
}

struct AudioManagerInner {
    context: Option<AudioContext>,
    worklet_node: Option<AudioWorkletNode>,
    worklet_port: Option<MessagePort>,
    buffer_size: Arc<AtomicCell<u32>>,
    error_callback: Arc<Box<dyn Fn(String) + Send + Sync>>,
    synth: Option<Box<dyn Synth + Send + Sync>>,
    ready: bool,
}

impl AudioManager {
    pub fn new<U>(synth: Box<dyn Synth + Send + Sync>, error_callback: U) -> Self
    where
        U: Fn(String) + Send + Sync + 'static,
    {
        let inner = AudioManagerInner {
            context: None,
            worklet_node: None,
            worklet_port: None,
            buffer_size: Arc::new(AtomicCell::new(128)),
            error_callback: Arc::new(Box::new(error_callback)),
            synth: Some(synth),
            ready: false,
        };
        
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn initialize(&self) {
        let mut inner = self.inner.lock();
        let r = (|| -> Result<_> {
            let context = AudioContext::new().map_err(|e| anyhow!("Failed to create AudioContext: {:?}", e))?;
            inner.context = Some(context);
            info!("WebAudio context created.");
            Ok(())
        })();
        
        if let Err(e) = r {
            (inner.error_callback)(format!("WebAudio context setup error: {:?}", e));
        }
    }

    pub async fn setup_worklet(&self) -> Result<()> {
        let context = {
            let inner = self.inner.lock();
            inner.context.as_ref()
                .ok_or_else(|| anyhow!("AudioContext not initialized"))?
                .clone()
        };

        // Ensure AudioContext is running (try to resume if suspended)
        info!("Attempting to resume AudioContext...");
        let resume_promise = context.resume()
            .map_err(|e| anyhow!("Failed to get resume promise: {:?}", e))?;
        JsFuture::from(resume_promise).await
            .map_err(|e| anyhow!("Failed to resume AudioContext: {:?}", e))?;
        info!("AudioContext resume completed");

        // Add the audio worklet module
        let worklet_url = "./ultra-minimal-processor.js";
        let audioworklet = context.audio_worklet()
            .map_err(|e| anyhow!("AudioWorklet not supported: {:?}", e))?;
        
        let add_module_promise = audioworklet.add_module(worklet_url)
            .map_err(|e| anyhow!("Failed to get add_module promise: {:?}", e))?;

        info!("About to await add_module promise for: {}", worklet_url);
        let result = JsFuture::from(add_module_promise).await;
        match result {
            Ok(_) => {
                info!("AudioWorklet module loaded successfully");
            }
            Err(e) => {
                error!("Failed to load AudioWorklet module '{}': {:?}", worklet_url, e);
                return Err(anyhow!("Failed to load AudioWorklet module: {:?}", e));
            }
        }

        // Create the AudioWorkletNode
        let worklet_node = AudioWorkletNode::new(&context, "ultra-minimal-processor")
            .map_err(|e| anyhow!("Failed to create AudioWorkletNode: {:?}", e))?;

        // Connect to destination
        worklet_node.connect_with_audio_node(&context.destination())
            .map_err(|e| anyhow!("Failed to connect AudioWorkletNode: {:?}", e))?;

        // Get the message port for communication
        let port = worklet_node.port()
            .map_err(|e| anyhow!("Failed to get message port: {:?}", e))?;

        // Set up message listener for responses from the worklet
        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                warn!("Received message from AudioWorklet: {:?}", data);
            }
        }) as Box<dyn FnMut(_)>);

        port.set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Keep the closure alive

        // Update the inner state
        {
            let mut inner = self.inner.lock();
            inner.worklet_node = Some(worklet_node);
            inner.worklet_port = Some(port);
            inner.ready = true;
        }

        info!("AudioWorklet setup complete!");
        Ok(())
    }

    // Method to send MIDI messages to the worklet
    pub fn send_midi_message(&self, message: wmidi::MidiMessage<'static>) {
        let inner = self.inner.lock();
        if let Some(ref worklet_port) = inner.worklet_port {
            // Convert wmidi message to simple format for JavaScript processor
            let js_message = js_sys::Object::new();
            
            unsafe {
                js_sys::Reflect::set(&js_message, &"type".into(), &"midi".into()).unwrap();
                
                let midi_data = js_sys::Object::new();
                
                match message {
                    wmidi::MidiMessage::NoteOn(ch, note, vel) => {
                        js_sys::Reflect::set(&midi_data, &"status".into(), &(144u8 + ch.index()).into()).unwrap();
                        js_sys::Reflect::set(&midi_data, &"data1".into(), &u8::from(note).into()).unwrap();
                        js_sys::Reflect::set(&midi_data, &"data2".into(), &u8::from(vel).into()).unwrap();
                    }
                    wmidi::MidiMessage::NoteOff(ch, note, vel) => {
                        js_sys::Reflect::set(&midi_data, &"status".into(), &(128u8 + ch.index()).into()).unwrap();
                        js_sys::Reflect::set(&midi_data, &"data1".into(), &u8::from(note).into()).unwrap();
                        js_sys::Reflect::set(&midi_data, &"data2".into(), &u8::from(vel).into()).unwrap();
                    }
                    _ => {
                        warn!("Unsupported MIDI message type");
                        return;
                    }
                }
                
                js_sys::Reflect::set(&js_message, &"data".into(), &midi_data).unwrap();
            }
            
            if let Err(e) = worklet_port.post_message(&js_message) {
                warn!("Failed to send MIDI message to worklet: {:?}", e);
            }
        } else {
            warn!("AudioWorklet not initialized, cannot send MIDI message");
        }
    }

    pub fn is_ready(&self) -> bool {
        self.inner.lock().ready
    }

    #[allow(dead_code)]
    pub fn get_name(&self) -> Option<String> {
        Some("WebAudio".to_string())
    }
}
