//! the usb device abstraction the controller runs against.
//!
//! both the real backend (`sextant-usb`, over nusb) and the in-memory
//! `sextant-fake` implement [`Device`] and [`DeviceHandle`], so the controller
//! is identical for hardware and tests. this is the rust replacement for the
//! python original's duck typing and its sed-generated `_simulator` variants.

use std::sync::Arc;
use std::time::Duration;

/// the two device generations, distinguished by their wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceClass {
    PaletteGear,
    MonogramCc,
}

impl DeviceClass {
    /// a lowercase display name for the `devices` listing.
    pub fn name(self) -> &'static str {
        match self {
            DeviceClass::PaletteGear => "palette gear",
            DeviceClass::MonogramCc => "monogram cc",
        }
    }
}

/// known `VENDOR:PRODUCT` ids and their device class.
pub fn device_class(device_id: &str) -> Option<DeviceClass> {
    match device_id.to_uppercase().as_str() {
        "16D0:09F8" => Some(DeviceClass::PaletteGear),
        "0483:A2C5" => Some(DeviceClass::MonogramCc),
        _ => None,
    }
}

// cdc control handshake and bulk transfer constants. these describe the device
// protocol, so they live with the device abstraction rather than the controller.
pub const HANDSHAKE_BYTES: [u8; 8] = [0xe0, 0x93, 0x04, 0x00, 0x00, 0x00, 0x00, 0x08];
// the usb backend discovers the bulk endpoints from the device descriptors
// (palette hardware revisions differ); these are only the fallback addresses.
pub const ENDPOINT_BULK_IN: u8 = 0x81;
pub const ENDPOINT_BULK_OUT: u8 = 0x01;
pub const REQUEST_TYPE_IN: u8 = 0xa1;
pub const REQUEST_TYPE_OUT: u8 = 0x21;
pub const SETUP_REQUEST_HANDSHAKE: u8 = 0x20;
pub const SETUP_REQUEST_EMPTY_IN: u8 = 0x21;
pub const SETUP_REQUEST_EMPTY_OUT: u8 = 0x22;
pub const BULK_READ_SIZE_BYTES: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum UsbError {
    #[error("usb read timed out")]
    Timeout,
    #[error("device disconnected")]
    Disconnected,
    #[error("usb error: {0}")]
    Io(String),
}

/// the payload of a control transfer: a length to read (IN) or bytes (OUT).
pub enum ControlData<'a> {
    In(usize),
    Out(&'a [u8]),
}

/// the live i/o surface of an opened device. shared across the controller's
/// reader and writer threads, so all methods take `&self`.
pub trait DeviceHandle: Send + Sync {
    /// one step of the cdc control handshake.
    fn control(&self, request_type: u8, request: u8, data: ControlData<'_>)
        -> Result<(), UsbError>;
    /// write a framed message buffer to the bulk out endpoint.
    fn bulk_write(&self, endpoint: u8, data: &[u8]) -> Result<usize, UsbError>;
    /// read up to `max` bytes from the bulk in endpoint, blocking up to `timeout`.
    fn bulk_read(&self, endpoint: u8, max: usize, timeout: Duration) -> Result<Vec<u8>, UsbError>;
    fn reset(&self);
    fn release(&self);
}

/// the cdc control handshake: six control transfers that put the palette /
/// monogram bridge into its bulk-streaming mode. shared by the controller and
/// the write-rate probe so both bring a device up identically.
pub fn control_handshake(handle: &dyn DeviceHandle) -> Result<(), UsbError> {
    handle.control(REQUEST_TYPE_IN, SETUP_REQUEST_EMPTY_IN, ControlData::In(7))?;
    handle.control(
        REQUEST_TYPE_OUT,
        SETUP_REQUEST_HANDSHAKE,
        ControlData::Out(&HANDSHAKE_BYTES),
    )?;
    handle.control(REQUEST_TYPE_IN, SETUP_REQUEST_EMPTY_IN, ControlData::In(7))?;
    handle.control(
        REQUEST_TYPE_OUT,
        SETUP_REQUEST_EMPTY_OUT,
        ControlData::Out(&[]),
    )?;
    handle.control(
        REQUEST_TYPE_OUT,
        SETUP_REQUEST_HANDSHAKE,
        ControlData::Out(&HANDSHAKE_BYTES),
    )?;
    handle.control(REQUEST_TYPE_IN, SETUP_REQUEST_EMPTY_IN, ControlData::In(7))?;
    Ok(())
}

/// an enumerable, openable device.
pub trait Device {
    /// the device class (palettegear vs monogram). this is the single source of
    /// truth for device-profile differences (wire format, led / screen output).
    fn class(&self) -> DeviceClass;
    /// whether this device speaks the legacy palettegear json protocol.
    fn legacy(&self) -> bool {
        self.class() == DeviceClass::PaletteGear
    }
    /// claim the interface and return the live handle.
    fn open(&mut self) -> Result<Arc<dyn DeviceHandle>, UsbError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_classes_resolve_case_insensitively() {
        assert_eq!(Some(DeviceClass::PaletteGear), device_class("16d0:09f8"));
        assert_eq!(Some(DeviceClass::MonogramCc), device_class("0483:A2C5"));
        assert_eq!(None, device_class("dead:beef"));
    }

    #[test]
    fn device_class_names_are_display_ready() {
        assert_eq!("palette gear", DeviceClass::PaletteGear.name());
        assert_eq!("monogram cc", DeviceClass::MonogramCc.name());
    }
}
