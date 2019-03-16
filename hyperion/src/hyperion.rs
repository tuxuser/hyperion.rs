//! Definition of the Hyperion data model

use futures::sync::mpsc;
use futures::{Async, Poll, Future, Stream};

/// Definition of the Led type
mod led;
pub use led::*;

/// Definition of the Device type
mod device;
pub use device::*;

/// State update messages for the Hyperion service
#[derive(Debug)]
pub enum StateUpdate {
    ClearAll,
}

/// A configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    devices: Vec<Device>,
}

/// Hyperion service state
pub struct Hyperion {
    /// Configured state of LED devices
    configuration: Configuration,
    /// Receiver for update messages
    receiver: mpsc::UnboundedReceiver<StateUpdate>,
}

impl Hyperion {
    pub fn new(configuration: Configuration) -> (Self, mpsc::UnboundedSender<StateUpdate>) {
        // TODO: check channel capacity
        let (sender, receiver) = mpsc::unbounded();
        (Self { configuration, receiver }, sender)
    }
}

#[derive(Debug, Fail)]
pub enum HyperionError {
    #[fail(display = "failed to receive update from channel")]
    ChannelReceiveFailed,
}

impl Future for Hyperion {
    type Item = ();
    type Error = HyperionError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.receiver.poll() {
            Ok(value) => Ok(match value {
                Async::Ready(value) => match value {
                    Some(state_update) => {
                        debug!("got state update: {:?}", state_update);
                        Async::NotReady
                    }
                    None => Async::NotReady,
                },
                Async::NotReady => Async::NotReady,
            }),
            Err(_) => Err(HyperionError::ChannelReceiveFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn deserialize_full_config() {
        let config: Configuration = serde_json::from_str(r#"
{
    "devices": [
        {
            "name": "Stdout dummy",
            "endpoint": {
                "method": "stdout"
            },
            "leds": [ 
                { "index": 0, "hscan": { "minimum": 0.0, "maximum": 0.5 },
                              "vscan": { "minimum": 0.0, "maximum": 0.5 } }
            ]
        },
        {
            "name": "Remote UDP",
            "endpoint": {
                "method": "udp",
                "target": {
                    "address": "127.0.0.1:20446"
                }
            },
            "leds": [ 
                { "index": 0, "hscan": { "minimum": 0.5, "maximum": 1.0 },
                              "vscan": { "minimum": 0.5, "maximum": 1.0 } }
            ]
        }
    ]
}
        "#).unwrap();

        println!("{:?}", config);
    }
}


