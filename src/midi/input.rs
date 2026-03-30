//! MIDI input handling via midir

use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use parking_lot::Mutex;

use super::parser::{parse_midi, MidiMessage};
use crate::{Error, Result};

/// MIDI input manager
pub struct MidiInputManager {
    input: Option<MidiInput>,
    connection: Option<MidiInputConnection<()>>,
    receiver: Option<Receiver<MidiMessage>>,
    sender: Option<Arc<Mutex<Sender<MidiMessage>>>>,
}

impl MidiInputManager {
    /// Create new MIDI input manager
    pub fn new() -> Result<Self> {
        let input = MidiInput::new("synthesis")
            .map_err(|e| Error::Midi(e.to_string()))?;
        
        Ok(Self {
            input: Some(input),
            connection: None,
            receiver: None,
            sender: None,
        })
    }

    /// List available MIDI input ports
    pub fn list_ports(&self) -> Result<Vec<String>> {
        let input = self.input.as_ref()
            .ok_or_else(|| Error::Midi("MIDI input not initialized".to_string()))?;
        
        let ports: Vec<String> = input
            .ports()
            .iter()
            .filter_map(|p| input.port_name(p).ok())
            .collect();
        
        Ok(ports)
    }

    /// Find port by name (partial match)
    pub fn find_port(&self, name: &str) -> Result<MidiInputPort> {
        let input = self.input.as_ref()
            .ok_or_else(|| Error::Midi("MIDI input not initialized".to_string()))?;
        
        let name_lower = name.to_lowercase();
        
        for port in input.ports() {
            if let Ok(port_name) = input.port_name(&port) {
                if port_name.to_lowercase().contains(&name_lower) {
                    return Ok(port);
                }
            }
        }
        
        Err(Error::Midi(format!("MIDI port not found: {}", name)))
    }

    /// Connect to a MIDI input port by name
    pub fn connect(&mut self, port_name: &str) -> Result<()> {
        // Close existing connection
        self.disconnect();
        
        let input = self.input.take()
            .ok_or_else(|| Error::Midi("MIDI input not initialized".to_string()))?;
        
        let port = {
            let name_lower = port_name.to_lowercase();
            let mut found_port = None;
            
            for port in input.ports() {
                if let Ok(pname) = input.port_name(&port) {
                    if pname.to_lowercase().contains(&name_lower) {
                        found_port = Some(port);
                        break;
                    }
                }
            }
            
            found_port.ok_or_else(|| Error::Midi(format!("Port not found: {}", port_name)))?
        };

        let (sender, receiver) = channel();
        let sender = Arc::new(Mutex::new(sender));
        let sender_clone = Arc::clone(&sender);

        let connection = input
            .connect(
                &port,
                "synthesis-input",
                move |_timestamp, data, _| {
                    let msg = parse_midi(data);
                    if let Ok(s) = sender_clone.lock().send(msg) {
                        let _ = s;
                    }
                },
                (),
            )
            .map_err(|e| Error::Midi(e.to_string()))?;

        self.connection = Some(connection);
        self.receiver = Some(receiver);
        self.sender = Some(sender);

        Ok(())
    }

    /// Disconnect from current port
    pub fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            let (input, _) = conn.close();
            self.input = Some(input);
        }
        self.receiver = None;
        self.sender = None;
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Poll for incoming MIDI messages (non-blocking)
    pub fn poll(&self) -> Option<MidiMessage> {
        self.receiver.as_ref().and_then(|rx| {
            match rx.try_recv() {
                Ok(msg) => Some(msg),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => None,
            }
        })
    }

    /// Poll all available messages
    pub fn poll_all(&self) -> Vec<MidiMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.poll() {
            messages.push(msg);
        }
        messages
    }
}

impl Default for MidiInputManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            input: None,
            connection: None,
            receiver: None,
            sender: None,
        })
    }
}

impl Drop for MidiInputManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// List available MIDI input devices
pub fn list_midi_inputs() -> Result<Vec<String>> {
    let input = MidiInput::new("synthesis-list")
        .map_err(|e| Error::Midi(e.to_string()))?;
    
    let ports: Vec<String> = input
        .ports()
        .iter()
        .filter_map(|p| input.port_name(p).ok())
        .collect();
    
    Ok(ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_input_new() {
        // May fail on systems without MIDI support
        let _ = MidiInputManager::new();
    }

    #[test]
    fn test_list_midi_inputs() {
        // Should not panic even without MIDI devices
        let _ = list_midi_inputs();
    }

    #[test]
    fn test_midi_input_list_ports() {
        if let Ok(manager) = MidiInputManager::new() {
            let _ = manager.list_ports();
        }
    }

    #[test]
    fn test_midi_input_not_connected() {
        if let Ok(manager) = MidiInputManager::new() {
            assert!(!manager.is_connected());
        }
    }

    #[test]
    fn test_midi_input_poll_empty() {
        if let Ok(manager) = MidiInputManager::new() {
            assert!(manager.poll().is_none());
        }
    }

    #[test]
    fn test_midi_input_poll_all_empty() {
        if let Ok(manager) = MidiInputManager::new() {
            assert!(manager.poll_all().is_empty());
        }
    }
}
