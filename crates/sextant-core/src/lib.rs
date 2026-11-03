//! sextant-core: the host-side core library for palette / monogram controllers.
//!
//! this crate holds the wire protocol (json + msgpack framing), the component
//! data model, the behavior engine, configuration parsing, the screen-image
//! codec, and embedded resources. it is free of gui and real-usb dependencies
//! so the controller can be driven by either the real usb backend
//! (`sextant-usb`) or the in-memory `sextant-fake` device through a shared
//! device trait (added in phase 2).

pub mod component;
pub mod configuration;
pub mod controller;
pub mod coordinates;
pub mod device;
pub mod flags;
pub mod image;
pub mod inspect;
pub mod message;
pub mod resources;

pub use component::{
    Behavior, BehaviorKind, Component, ComponentLayout, DispatchValue, Invocation,
};
pub use configuration::{Configuration, Profile};
pub use controller::{Controller, Led, RenderComponent};
pub use device::{Device, DeviceHandle, UsbError};
pub use flags::Flags;
pub use inspect::{InspectAnalysis, InspectInputRow, InspectOutputRow, InspectRegisterRow};
pub use message::MessageFormat;
