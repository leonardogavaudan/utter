use std::sync::mpsc;
use std::time::Duration;

use accessibility_sys::{AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt};
use block2::RcBlock;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use objc2::runtime::Bool;
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

use crate::permissions::MicrophoneAccess;

const MICROPHONE_PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) fn accessibility_granted(prompt: bool) -> bool {
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let value = if prompt {
        CFBoolean::true_value()
    } else {
        CFBoolean::false_value()
    };
    let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
}

pub(crate) fn microphone_access() -> MicrophoneAccess {
    let Some(audio) = (unsafe { AVMediaTypeAudio }) else {
        return MicrophoneAccess::Denied;
    };
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(audio) } {
        AVAuthorizationStatus::Authorized => MicrophoneAccess::Granted,
        AVAuthorizationStatus::NotDetermined => MicrophoneAccess::NotDetermined,
        _ => MicrophoneAccess::Denied,
    }
}

pub(crate) fn request_microphone() -> MicrophoneAccess {
    let Some(audio) = (unsafe { AVMediaTypeAudio }) else {
        return MicrophoneAccess::Denied;
    };
    if microphone_access() != MicrophoneAccess::NotDetermined {
        return microphone_access();
    }
    let (sender, answered) = mpsc::channel();
    let handler = RcBlock::new(move |granted: Bool| {
        let _ = sender.send(granted.as_bool());
    });
    unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(audio, &handler) };
    let _ = answered.recv_timeout(MICROPHONE_PROMPT_TIMEOUT);
    microphone_access()
}
