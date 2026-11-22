use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Connection {
    BuiltIn,
    Bluetooth,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Microphone {
    pub id: String,
    pub name: String,
    pub connection: Connection,
    pub is_default: bool,
}

pub fn choose_microphone<'a>(
    microphones: &'a [Microphone],
    preferred_id: Option<&str>,
) -> Option<&'a Microphone> {
    if let Some(preferred) =
        preferred_id.and_then(|id| microphones.iter().find(|microphone| microphone.id == id))
    {
        return Some(preferred);
    }
    let default = microphones.iter().find(|microphone| microphone.is_default);
    match default {
        Some(default) if default.connection != Connection::Bluetooth => Some(default),
        _ => microphones
            .iter()
            .find(|microphone| microphone.connection == Connection::BuiltIn)
            .or_else(|| {
                microphones
                    .iter()
                    .find(|microphone| microphone.connection != Connection::Bluetooth)
            })
            .or(default)
            .or_else(|| microphones.first()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn microphone(id: &str, connection: Connection, is_default: bool) -> Microphone {
        Microphone {
            id: id.to_owned(),
            name: id.to_owned(),
            connection,
            is_default,
        }
    }

    #[test]
    fn avoids_a_bluetooth_headset_mic_when_a_built_in_one_exists() {
        let microphones = [
            microphone("usb-webcam", Connection::Other, false),
            microphone("airpods", Connection::Bluetooth, true),
            microphone("macbook", Connection::BuiltIn, false),
        ];
        assert_eq!(
            choose_microphone(&microphones, None).map(|m| m.id.as_str()),
            Some("macbook")
        );
    }

    #[test]
    fn falls_back_to_another_wired_mic_then_to_the_headset() {
        let without_built_in = [
            microphone("airpods", Connection::Bluetooth, true),
            microphone("usb-webcam", Connection::Other, false),
        ];
        assert_eq!(
            choose_microphone(&without_built_in, None).map(|m| m.id.as_str()),
            Some("usb-webcam")
        );
        let only_headset = [microphone("airpods", Connection::Bluetooth, true)];
        assert_eq!(
            choose_microphone(&only_headset, None).map(|m| m.id.as_str()),
            Some("airpods")
        );
    }

    #[test]
    fn keeps_a_wired_default_and_honours_an_explicit_choice() {
        let microphones = [
            microphone("studio", Connection::Other, true),
            microphone("macbook", Connection::BuiltIn, false),
            microphone("airpods", Connection::Bluetooth, false),
        ];
        assert_eq!(
            choose_microphone(&microphones, None).map(|m| m.id.as_str()),
            Some("studio")
        );
        assert_eq!(
            choose_microphone(&microphones, Some("airpods")).map(|m| m.id.as_str()),
            Some("airpods")
        );
    }

    #[test]
    fn ignores_a_chosen_mic_that_is_unplugged() {
        let microphones = [microphone("macbook", Connection::BuiltIn, true)];
        assert_eq!(
            choose_microphone(&microphones, Some("unplugged")).map(|m| m.id.as_str()),
            Some("macbook")
        );
        assert_eq!(choose_microphone(&[], None), None);
    }
}
