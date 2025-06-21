use serde::{Deserialize, Serialize};

/// Trait for audio synthesis
pub trait Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, out_samples: &mut [f32]);
}

/// Configuration for the audio worklet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: f32,
    pub channels: usize,
    pub buffer_size: usize,
}

/// Simplified MIDI message for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiMsg {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    Other(Vec<u8>),
}

impl From<wmidi::MidiMessage<'_>> for MidiMsg {
    fn from(msg: wmidi::MidiMessage<'_>) -> Self {
        match msg {
            wmidi::MidiMessage::NoteOn(channel, note, velocity) => {
                MidiMsg::NoteOn {
                    channel: channel.index(),
                    note: note.into(),
                    velocity: velocity.into(),
                }
            }
            wmidi::MidiMessage::NoteOff(channel, note, velocity) => {
                MidiMsg::NoteOff {
                    channel: channel.index(),
                    note: note.into(),
                    velocity: velocity.into(),
                }
            }
            _ => MidiMsg::Other(Vec::new()), // For now, just store empty for other messages
        }
    }
}

impl TryInto<wmidi::MidiMessage<'static>> for MidiMsg {
    type Error = ();
    
    fn try_into(self) -> Result<wmidi::MidiMessage<'static>, Self::Error> {
        match self {
            MidiMsg::NoteOn { channel, note, velocity } => {
                let channel = wmidi::Channel::from_index(channel).map_err(|_| ())?;
                let note = wmidi::Note::try_from(note).map_err(|_| ())?;
                let velocity = wmidi::U7::try_from(velocity).map_err(|_| ())?;
                Ok(wmidi::MidiMessage::NoteOn(channel, note, velocity))
            }
            MidiMsg::NoteOff { channel, note, velocity } => {
                let channel = wmidi::Channel::from_index(channel).map_err(|_| ())?;
                let note = wmidi::Note::try_from(note).map_err(|_| ())?;
                let velocity = wmidi::U7::try_from(velocity).map_err(|_| ())?;
                Ok(wmidi::MidiMessage::NoteOff(channel, note, velocity))
            }
            MidiMsg::Other(_) => Err(()),
        }
    }
}

/// Messages sent to the audio worklet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkletMessage {
    Config(AudioConfig),
    MidiMessage(MidiMsg),
    RequestAudio { buffer_size: usize },
}

/// Messages sent from the audio worklet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkletResponse {
    AudioData(Vec<f32>),
    Error(String),
    Ready,
}
