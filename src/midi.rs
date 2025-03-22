use log::error;
use midir::{MidiInput, MidiInputConnection};
use std::convert::TryFrom;
use wmidi::MidiMessage;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No MIDI interface available")]
    NoMidiInterface,
    #[error("Failed to initialize MIDI: {0}")]
    Init(#[from] midir::InitError),
    #[error("Failed to connect to MIDI device: {0}")]
    Connect(#[from] midir::ConnectError<midir::MidiInput>),
    #[error("Failed to get port info: {0}")]
    PortInfo(#[from] midir::PortInfoError),
}

pub struct MidiReader {
    _connection: MidiInputConnection<()>,
    name: String,
}

impl MidiReader {
    pub fn new(callback: impl Fn(&MidiMessage<'_>) + Send + 'static) -> Result<Self, Error> {
        let midi = MidiInput::new("dissonance-lab")?;
        let ports = midi.ports();
        if let Some(port) = ports.first() {
            let name = midi.port_name(port)?;
            let connection = midi.connect(
                port,
                &name,
                move |_time_ms, message, _| match wmidi::MidiMessage::try_from(message) {
                    Ok(message) => {
                        callback(&message);
                    }
                    Err(e) => {
                        error!("error parsing midi event {}", e);
                    }
                },
                (),
            )?;
            Ok(Self {
                _connection: connection,
                name,
            })
        } else {
            Err(Error::NoMidiInterface)
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
