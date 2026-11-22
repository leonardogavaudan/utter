use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{self, NonNull};

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
    kAudioDevicePropertyTransportType, kAudioDeviceTransportTypeBluetooth,
    kAudioDeviceTransportTypeBluetoothLE, kAudioDeviceTransportTypeBuiltIn,
    kAudioHardwarePropertyTranslateUIDToDevice, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
};
use utter_core::microphone::Connection;

pub(crate) fn connection(uid: &str) -> Connection {
    let Some(transport) = device_for_uid(uid).and_then(transport_type) else {
        return Connection::Other;
    };
    if transport == kAudioDeviceTransportTypeBuiltIn {
        Connection::BuiltIn
    } else if transport == kAudioDeviceTransportTypeBluetooth
        || transport == kAudioDeviceTransportTypeBluetoothLE
    {
        Connection::Bluetooth
    } else {
        Connection::Other
    }
}

fn device_for_uid(uid: &str) -> Option<AudioObjectID> {
    let uid = CFString::new(uid);
    let uid_ref: CFStringRef = uid.as_concrete_TypeRef();
    let mut device: AudioObjectID = 0;
    read_property(
        kAudioObjectSystemObject as AudioObjectID,
        kAudioHardwarePropertyTranslateUIDToDevice,
        Some((
            (&raw const uid_ref).cast::<c_void>(),
            size_of::<CFStringRef>() as u32,
        )),
        &mut device,
    )?;
    (device != 0).then_some(device)
}

fn transport_type(device: AudioObjectID) -> Option<u32> {
    let mut transport: u32 = 0;
    read_property(
        device,
        kAudioDevicePropertyTransportType,
        None,
        &mut transport,
    )?;
    Some(transport)
}

fn read_property<T>(
    object: AudioObjectID,
    selector: u32,
    qualifier: Option<(*const c_void, u32)>,
    out: &mut T,
) -> Option<()> {
    let address = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size = size_of::<T>() as u32;
    let (qualifier_data, qualifier_size) = qualifier.unwrap_or((ptr::null(), 0));
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            qualifier_size,
            qualifier_data,
            NonNull::from(&mut size),
            NonNull::from(out).cast::<c_void>(),
        )
    };
    (status == 0).then_some(())
}
