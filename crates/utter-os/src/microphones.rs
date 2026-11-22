use cpal::traits::{DeviceTrait, HostTrait};
use utter_core::microphone::{Connection, Microphone};

pub(crate) struct InputDevice {
    pub(crate) microphone: Microphone,
    pub(crate) device: cpal::Device,
}

pub fn list_microphones() -> Vec<Microphone> {
    input_devices()
        .into_iter()
        .map(|input| input.microphone)
        .collect()
}

pub(crate) fn input_devices() -> Vec<InputDevice> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|device| device.id().ok());
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?;
            let name = device
                .description()
                .map(|description| description.name().to_owned())
                .unwrap_or_else(|_| id.id().to_owned());
            Some(InputDevice {
                microphone: Microphone {
                    id: id.id().to_owned(),
                    name,
                    connection: connection(id.id()),
                    is_default: default_id.as_ref() == Some(&id),
                },
                device,
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn connection(id: &str) -> Connection {
    crate::macos::audio::connection(id)
}

#[cfg(not(target_os = "macos"))]
fn connection(_id: &str) -> Connection {
    Connection::Other
}
