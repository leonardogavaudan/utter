use std::ffi::c_void;
use std::ptr;

use accessibility_sys::{
    AXError, AXUIElementCopyAttributeValue, AXUIElementCopyParameterizedAttributeValue,
    AXUIElementCreateApplication, AXUIElementGetTypeID, AXUIElementRef,
    AXUIElementSetAttributeValue, AXUIElementSetMessagingTimeout, AXValueCreate, AXValueGetTypeID,
    AXValueGetValue, AXValueRef, kAXChildrenAttribute, kAXErrorSuccess, kAXPositionAttribute,
    kAXSizeAttribute, kAXStringForRangeParameterizedAttribute, kAXValueTypeCFRange,
    kAXValueTypeCGPoint, kAXValueTypeCGSize,
};
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFRange, CFRelease, CFRetain, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::string::{CFStringGetCharacters, CFStringGetLength};
use utter_core::screen::Frame;

const MESSAGING_TIMEOUT_SECONDS: f32 = 0.25;

pub(crate) struct Element(AXUIElementRef);

impl Drop for Element {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0 as CFTypeRef) }
    }
}

impl Element {
    pub(crate) fn application(pid: i32) -> Element {
        Element::with_timeout(unsafe { AXUIElementCreateApplication(pid) })
    }

    pub(crate) fn element(&self, attribute: &str) -> Option<Element> {
        Element::retained_from(&self.copy_attribute(attribute)?)
    }

    pub(crate) fn children(&self) -> Vec<Element> {
        let Some(value) = self.copy_attribute(kAXChildrenAttribute) else {
            return Vec::new();
        };
        if value.type_of() != CFArray::<CFType>::type_id() {
            return Vec::new();
        }
        let children: CFArray<CFType> =
            unsafe { CFArray::wrap_under_get_rule(value.as_CFTypeRef() as CFArrayRef) };
        children
            .iter()
            .filter_map(|child| Element::retained_from(&child))
            .collect()
    }

    pub(crate) fn frame(&self) -> Option<Frame> {
        let [x, y] = self.pair(kAXPositionAttribute, kAXValueTypeCGPoint)?;
        let [width, height] = self.pair(kAXSizeAttribute, kAXValueTypeCGSize)?;
        Some(Frame {
            x,
            y,
            width,
            height,
        })
    }

    pub(crate) fn string(&self, attribute: &str) -> Option<String> {
        self.copy_attribute(attribute)?
            .downcast::<CFString>()
            .map(|text| utf16_lossy(&text))
    }

    pub(crate) fn integer(&self, attribute: &str) -> Option<i64> {
        self.copy_attribute(attribute)?
            .downcast::<CFNumber>()?
            .to_i64()
    }

    pub(crate) fn range(&self, attribute: &str) -> Option<CFRange> {
        let value = self.copy_attribute(attribute)?;
        if value.type_of() != unsafe { AXValueGetTypeID() } {
            return None;
        }
        let mut range = CFRange::init(0, 0);
        let read = unsafe {
            AXValueGetValue(
                value.as_CFTypeRef() as AXValueRef,
                kAXValueTypeCFRange,
                (&raw mut range).cast::<c_void>(),
            )
        };
        read.then_some(range)
    }

    pub(crate) fn string_for_range(&self, range: CFRange) -> Option<String> {
        let parameter =
            unsafe { AXValueCreate(kAXValueTypeCFRange, (&raw const range).cast::<c_void>()) };
        if parameter.is_null() {
            return None;
        }
        let parameter = unsafe { CFType::wrap_under_create_rule(parameter as CFTypeRef) };
        let attribute = CFString::from_static_string(kAXStringForRangeParameterizedAttribute);
        let mut value: CFTypeRef = ptr::null();
        let status = unsafe {
            AXUIElementCopyParameterizedAttributeValue(
                self.0,
                attribute.as_concrete_TypeRef(),
                parameter.as_CFTypeRef(),
                &mut value,
            )
        };
        owned_value(status, value)?
            .downcast::<CFString>()
            .map(|text| utf16_lossy(&text))
    }

    pub(crate) fn set_flag(&self, attribute: &str, enabled: bool) -> bool {
        let attribute = CFString::new(attribute);
        let value = if enabled {
            CFBoolean::true_value()
        } else {
            CFBoolean::false_value()
        };
        succeeded(unsafe {
            AXUIElementSetAttributeValue(
                self.0,
                attribute.as_concrete_TypeRef(),
                value.as_CFTypeRef(),
            )
        })
    }

    fn retained_from(value: &CFType) -> Option<Element> {
        if value.type_of() != unsafe { AXUIElementGetTypeID() } {
            return None;
        }
        let raw = value.as_CFTypeRef();
        unsafe { CFRetain(raw) };
        Some(Element::with_timeout(raw as AXUIElementRef))
    }

    fn pair(&self, attribute: &str, kind: u32) -> Option<[f64; 2]> {
        let value = self.copy_attribute(attribute)?;
        if value.type_of() != unsafe { AXValueGetTypeID() } {
            return None;
        }
        let mut pair = [0.0_f64; 2];
        let read = unsafe {
            AXValueGetValue(
                value.as_CFTypeRef() as AXValueRef,
                kind,
                pair.as_mut_ptr().cast::<c_void>(),
            )
        };
        read.then_some(pair)
    }

    fn with_timeout(raw: AXUIElementRef) -> Element {
        let element = Element(raw);
        let _ = unsafe { AXUIElementSetMessagingTimeout(element.0, MESSAGING_TIMEOUT_SECONDS) };
        element
    }

    fn copy_attribute(&self, attribute: &str) -> Option<CFType> {
        let attribute = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let status = unsafe {
            AXUIElementCopyAttributeValue(self.0, attribute.as_concrete_TypeRef(), &mut value)
        };
        owned_value(status, value)
    }
}

fn utf16_lossy(text: &CFString) -> String {
    let length = unsafe { CFStringGetLength(text.as_concrete_TypeRef()) };
    let mut units = vec![0u16; usize::try_from(length).unwrap_or(0)];
    unsafe {
        CFStringGetCharacters(
            text.as_concrete_TypeRef(),
            CFRange::init(0, length),
            units.as_mut_ptr(),
        );
    }
    String::from_utf16_lossy(&units)
}

fn succeeded(status: AXError) -> bool {
    status == kAXErrorSuccess
}

fn owned_value(status: AXError, value: CFTypeRef) -> Option<CFType> {
    (succeeded(status) && !value.is_null())
        .then(|| unsafe { CFType::wrap_under_create_rule(value) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_strings_with_unpaired_surrogates_without_panicking() {
        let units: Vec<u16> = "ok 👋".encode_utf16().collect();
        let half_emoji = &units[..units.len() - 1];
        let text = unsafe {
            CFString::wrap_under_create_rule(
                core_foundation_sys::string::CFStringCreateWithCharacters(
                    ptr::null(),
                    half_emoji.as_ptr(),
                    half_emoji.len() as _,
                ),
            )
        };
        assert_eq!(utf16_lossy(&text), "ok \u{FFFD}");
        assert_eq!(utf16_lossy(&CFString::new("héllo 👋")), "héllo 👋");
    }
}
