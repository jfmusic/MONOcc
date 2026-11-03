//! sextant-usb: the real-device backend over `nusb` (pure rust, no libusb).
//!
//! it implements the [`Device`] / [`DeviceHandle`] traits from `sextant-core`
//! against a physical palette / monogram device, so the controller drives real
//! hardware exactly as it drives the fake one: enumeration by `VENDOR:PRODUCT`,
//! the six-step cdc control handshake, and bulk in/out on endpoints `0x81` /
//! `0x01`.
//!
//! nusb's control transfers are blocking, but its bulk transfers are async. the
//! controller's bulk i/o is blocking-with-timeout, so bulk transfers are driven
//! on a persistent queue and parked on the calling thread until they complete or
//! the deadline passes. a timed-out read is left in flight and reused on the next
//! call, so no input is dropped.
//!
//! this is the one backend that needs a device to fully validate; everything
//! above it is exercised through `sextant-fake`.

use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

use nusb::transfer::{
    Completion, Control, ControlType, Direction, EndpointType, Queue, Recipient, RequestBuffer,
    ResponseBuffer, TransferError,
};
use sextant_core::device::{
    self, device_class, ControlData, Device, DeviceClass, DeviceHandle, UsbError,
};

const CONTROL_TIMEOUT: Duration = Duration::from_secs(1);
const BULK_WRITE_TIMEOUT: Duration = Duration::from_secs(1);
// cdc interface classes: the data interface carries the bulk endpoints, the
// communications interface receives the line-coding control handshake.
const CDC_COMM_CLASS: u8 = 0x02;
const CDC_DATA_CLASS: u8 = 0x0a;

/// one endpoint of an interface, reduced to what endpoint selection needs.
struct EndpointDesc {
    address: u8,
    bulk: bool,
    input: bool,
}

/// one interface's class and endpoints, reduced for endpoint selection.
struct InterfaceDesc {
    number: u8,
    class: u8,
    endpoints: Vec<EndpointDesc>,
}

impl InterfaceDesc {
    fn bulk(&self, input: bool) -> Option<u8> {
        self.endpoints
            .iter()
            .find(|e| e.bulk && e.input == input)
            .map(|e| e.address)
    }
}

/// the interfaces and endpoints to drive a device, discovered from descriptors.
#[derive(Debug, PartialEq, Eq)]
struct Endpoints {
    data_interface: u8,
    control_interface: u8,
    bulk_in: u8,
    bulk_out: u8,
}

/// pick the bulk data interface (its in/out pair) and the control interface for
/// the line-coding handshake. palette hardware revisions wire these onto
/// different interfaces and endpoints, so they are discovered, not hardcoded.
fn select_endpoints(interfaces: &[InterfaceDesc]) -> Option<Endpoints> {
    let has_bulk_pair = |i: &&InterfaceDesc| i.bulk(true).is_some() && i.bulk(false).is_some();
    let data = interfaces
        .iter()
        .filter(|i| i.class == CDC_DATA_CLASS)
        .find(has_bulk_pair)
        .or_else(|| interfaces.iter().find(has_bulk_pair))?;
    let control = interfaces
        .iter()
        .find(|i| i.class == CDC_COMM_CLASS)
        .map_or(data.number, |i| i.number);
    Some(Endpoints {
        data_interface: data.number,
        control_interface: control,
        bulk_in: data.bulk(true)?,
        bulk_out: data.bulk(false)?,
    })
}

/// a real palette / monogram device addressed by `VENDOR:PRODUCT` and index.
pub struct UsbDevice {
    vendor_id: u16,
    product_id: u16,
    device_index: usize,
    class: DeviceClass,
}

impl UsbDevice {
    /// parse `VENDOR:PRODUCT` and select the device class (which fixes the wire
    /// format). unknown ids are an error.
    pub fn new(device_id: &str, device_index: usize) -> Result<UsbDevice, UsbError> {
        let class = device_class(device_id)
            .ok_or_else(|| UsbError::Io(format!("unknown device id {device_id}")))?;
        let (vendor, product) = device_id
            .split_once(':')
            .ok_or_else(|| UsbError::Io(format!("malformed device id {device_id}")))?;
        let parse = |s: &str| {
            u16::from_str_radix(s, 16).map_err(|_| UsbError::Io(format!("bad hex in {device_id}")))
        };
        Ok(UsbDevice {
            vendor_id: parse(vendor)?,
            product_id: parse(product)?,
            device_index,
            class,
        })
    }
}

/// a connected device sextant recognizes, for the `devices` listing.
pub struct DeviceInfo {
    pub device_id: String,
    pub class: DeviceClass,
    pub bus: u8,
    pub address: u8,
    pub product: Option<String>,
}

/// enumerate connected usb devices sextant recognizes, tagged with their class.
pub fn list_devices() -> Result<Vec<DeviceInfo>, UsbError> {
    let devices = nusb::list_devices().map_err(|e| UsbError::Io(e.to_string()))?;
    Ok(devices
        .filter_map(|d| {
            let device_id = format!("{:04X}:{:04X}", d.vendor_id(), d.product_id());
            device_class(&device_id).map(|class| DeviceInfo {
                device_id,
                class,
                bus: d.bus_number(),
                address: d.device_address(),
                product: d.product_string().map(str::to_string),
            })
        })
        .collect())
}

/// pick a device from the recognized listing: filter by an explicit `--device`
/// id when given, then take the `--device-index`th match. returns the chosen
/// `VENDOR:PRODUCT` and the offset of that device among ones sharing its id,
/// which is what [`UsbDevice::open`] re-derives via `nth` at enumeration time.
pub fn select_device(
    devices: &[DeviceInfo],
    device: Option<&str>,
    index: usize,
) -> Option<(String, usize)> {
    let matches = |d: &DeviceInfo| device.is_none_or(|want| d.device_id.eq_ignore_ascii_case(want));
    let pos = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| matches(d))
        .map(|(i, _)| i)
        .nth(index)?;
    let id = &devices[pos].device_id;
    let offset = devices[..pos].iter().filter(|d| &d.device_id == id).count();
    Some((id.clone(), offset))
}

impl Device for UsbDevice {
    fn class(&self) -> DeviceClass {
        self.class
    }

    fn open(&mut self) -> Result<Arc<dyn DeviceHandle>, UsbError> {
        let info = nusb::list_devices()
            .map_err(io_error)?
            .filter(|d| d.vendor_id() == self.vendor_id && d.product_id() == self.product_id)
            .nth(self.device_index)
            .ok_or_else(|| UsbError::Io("device not found, check cable and permissions".into()))?;
        let usb_device = info.open().map_err(io_error)?;
        let endpoints = discover_endpoints(&usb_device);
        // detach the kernel cdc-acm driver from the control interface so the
        // line-coding handshake reaches it, then claim the data interface (which
        // carries the bulk endpoints).
        let _ = usb_device.detach_kernel_driver(endpoints.control_interface);
        let interface = usb_device
            .detach_and_claim_interface(endpoints.data_interface)
            .or_else(|_| usb_device.claim_interface(endpoints.data_interface))
            .map_err(io_error)?;
        let bulk_in = interface.bulk_in_queue(endpoints.bulk_in);
        let bulk_out = interface.bulk_out_queue(endpoints.bulk_out);
        Ok(Arc::new(UsbHandle {
            usb_device,
            interface,
            control_interface: endpoints.control_interface,
            bulk_in: Mutex::new(bulk_in),
            bulk_out: Mutex::new(bulk_out),
        }))
    }
}

/// discover the bulk + control interfaces from the device descriptors, falling
/// back to the legacy interface 0 / ep 0x81,0x01 layout if introspection fails.
fn discover_endpoints(usb_device: &nusb::Device) -> Endpoints {
    usb_device
        .active_configuration()
        .ok()
        .and_then(|config| {
            let interfaces: Vec<InterfaceDesc> = config
                .interfaces()
                .filter_map(|group| {
                    let alt = group.alt_settings().next()?;
                    Some(InterfaceDesc {
                        number: group.interface_number(),
                        class: alt.class(),
                        endpoints: alt
                            .endpoints()
                            .map(|e| EndpointDesc {
                                address: e.address(),
                                bulk: e.transfer_type() == EndpointType::Bulk,
                                input: e.direction() == Direction::In,
                            })
                            .collect(),
                    })
                })
                .collect();
            select_endpoints(&interfaces)
        })
        .unwrap_or(Endpoints {
            data_interface: 0,
            control_interface: 0,
            bulk_in: device::ENDPOINT_BULK_IN,
            bulk_out: device::ENDPOINT_BULK_OUT,
        })
}

/// an opened device handle: the claimed data interface, the control interface
/// the line-coding handshake targets, and the persistent bulk queues.
pub struct UsbHandle {
    usb_device: nusb::Device,
    interface: nusb::Interface,
    control_interface: u8,
    bulk_in: Mutex<Queue<RequestBuffer>>,
    bulk_out: Mutex<Queue<Vec<u8>>>,
}

impl DeviceHandle for UsbHandle {
    fn control(
        &self,
        _request_type: u8,
        request: u8,
        data: ControlData<'_>,
    ) -> Result<(), UsbError> {
        // every handshake step is a cdc class request to the communications
        // interface; the direction is implied by whether we read or write.
        let control = Control {
            control_type: ControlType::Class,
            recipient: Recipient::Interface,
            request,
            value: 0,
            index: self.control_interface as u16,
        };
        match data {
            ControlData::In(len) => {
                let mut buffer = vec![0u8; len];
                self.interface
                    .control_in_blocking(control, &mut buffer, CONTROL_TIMEOUT)
                    .map_err(transfer_error)?;
            }
            ControlData::Out(bytes) => {
                self.interface
                    .control_out_blocking(control, bytes, CONTROL_TIMEOUT)
                    .map_err(transfer_error)?;
            }
        }
        Ok(())
    }

    fn bulk_write(&self, _endpoint: u8, data: &[u8]) -> Result<usize, UsbError> {
        let mut queue = self.bulk_out.lock().unwrap();
        queue.submit(data.to_vec());
        match park_until(&mut *queue, BULK_WRITE_TIMEOUT) {
            Some(completion) => completion
                .into_result()
                .map(|response| response.actual_length())
                .map_err(transfer_error),
            None => {
                queue.cancel_all();
                Err(UsbError::Timeout)
            }
        }
    }

    fn bulk_read(&self, _endpoint: u8, max: usize, timeout: Duration) -> Result<Vec<u8>, UsbError> {
        let mut queue = self.bulk_in.lock().unwrap();
        // reuse an in-flight read left pending by a prior timeout so no device
        // input is lost; otherwise start a fresh transfer.
        if queue.pending() == 0 {
            queue.submit(RequestBuffer::new(max));
        }
        match park_until(&mut *queue, timeout) {
            Some(completion) => completion.into_result().map_err(transfer_error),
            None => Err(UsbError::Timeout),
        }
    }

    fn reset(&self) {
        let _ = self.usb_device.reset();
    }

    fn release(&self) {
        // the interface is released when this handle is dropped.
    }
}

/// a waker that unparks the thread that is blocking on a transfer.
struct ThreadWaker(thread::Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// the queue's response type differs by direction, so the parking loop is
/// generated for each concrete queue.
macro_rules! park_fn {
    ($name:ident, $request:ty, $response:ty) => {
        fn $name(queue: &mut Queue<$request>, timeout: Duration) -> Option<Completion<$response>> {
            let deadline = Instant::now() + timeout;
            let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
            let mut cx = Context::from_waker(&waker);
            loop {
                match queue.poll_next(&mut cx) {
                    Poll::Ready(completion) => return Some(completion),
                    Poll::Pending => {
                        let now = Instant::now();
                        if now >= deadline {
                            return None;
                        }
                        thread::park_timeout(deadline - now);
                    }
                }
            }
        }
    };
}

park_fn!(park_in, RequestBuffer, Vec<u8>);
park_fn!(park_out, Vec<u8>, ResponseBuffer);

/// block on the next completion of a queued transfer, or `None` on timeout.
fn park_until<Q: QueueExt>(queue: &mut Q, timeout: Duration) -> Option<Completion<Q::Response>> {
    Q::park_next(queue, timeout)
}

/// bridge so `bulk_read` / `bulk_write` share one parking call site despite the
/// queue response type differing by direction.
trait QueueExt {
    type Response;
    fn park_next(queue: &mut Self, timeout: Duration) -> Option<Completion<Self::Response>>;
}

impl QueueExt for Queue<RequestBuffer> {
    type Response = Vec<u8>;
    fn park_next(queue: &mut Self, timeout: Duration) -> Option<Completion<Self::Response>> {
        park_in(queue, timeout)
    }
}

impl QueueExt for Queue<Vec<u8>> {
    type Response = ResponseBuffer;
    fn park_next(queue: &mut Self, timeout: Duration) -> Option<Completion<Self::Response>> {
        park_out(queue, timeout)
    }
}

fn io_error(error: nusb::Error) -> UsbError {
    UsbError::Io(error.to_string())
}

fn transfer_error(error: TransferError) -> UsbError {
    match error {
        TransferError::Disconnected => UsbError::Disconnected,
        other => UsbError::Io(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_ids_and_class() {
        let palette = UsbDevice::new("16D0:09F8", 0).unwrap();
        assert_eq!((0x16d0, 0x09f8), (palette.vendor_id, palette.product_id));
        assert!(palette.legacy());

        let monogram = UsbDevice::new("0483:a2c5", 0).unwrap();
        assert!(!monogram.legacy());

        assert!(UsbDevice::new("nope", 0).is_err());
        assert!(UsbDevice::new("16D0:zzzz", 0).is_err());
    }

    fn info(id: &str) -> DeviceInfo {
        DeviceInfo {
            device_id: id.to_string(),
            class: device_class(id).unwrap(),
            bus: 0,
            address: 0,
            product: None,
        }
    }

    #[test]
    fn device_index_selects_across_the_recognized_listing() {
        // two different devices: --device-index 0 is the first listed row,
        // index 1 the second, regardless of their differing ids.
        let devices = vec![info("16D0:09F8"), info("0483:A2C5")];
        assert_eq!(
            Some(("16D0:09F8".to_string(), 0)),
            select_device(&devices, None, 0)
        );
        assert_eq!(
            Some(("0483:A2C5".to_string(), 0)),
            select_device(&devices, None, 1)
        );
    }

    #[test]
    fn device_index_offsets_within_a_repeated_id() {
        // two palettes then a monogram: the index walks the listing, and the
        // returned offset is the device's position among ones sharing its id.
        let devices = vec![info("16D0:09F8"), info("16D0:09F8"), info("0483:A2C5")];
        assert_eq!(
            Some(("16D0:09F8".to_string(), 1)),
            select_device(&devices, None, 1)
        );
        assert_eq!(
            Some(("0483:A2C5".to_string(), 0)),
            select_device(&devices, None, 2)
        );
    }

    #[test]
    fn explicit_device_filters_then_indexes() {
        // --device pins the id; --device-index then counts only matching rows.
        let devices = vec![info("16D0:09F8"), info("0483:A2C5"), info("16D0:09F8")];
        assert_eq!(
            Some(("16D0:09F8".to_string(), 1)),
            select_device(&devices, Some("16D0:09F8"), 1)
        );
        assert_eq!(None, select_device(&devices, Some("0483:A2C5"), 1));
    }

    #[test]
    fn device_index_past_the_end_is_none() {
        let devices = vec![info("16D0:09F8")];
        assert_eq!(None, select_device(&devices, None, 1));
    }

    fn endpoint(address: u8, bulk: bool, input: bool) -> EndpointDesc {
        EndpointDesc {
            address,
            bulk,
            input,
        }
    }

    #[test]
    fn selects_cdc_data_interface_and_bulk_endpoints() {
        // this user's palette gear: bulk data on interface 1 (ep 0x84 in /
        // 0x05 out), control on interface 0 -- not the legacy 0x81/0x01.
        let interfaces = vec![
            InterfaceDesc {
                number: 0,
                class: CDC_COMM_CLASS,
                endpoints: vec![endpoint(0x83, false, true)],
            },
            InterfaceDesc {
                number: 1,
                class: CDC_DATA_CLASS,
                endpoints: vec![endpoint(0x05, true, false), endpoint(0x84, true, true)],
            },
            InterfaceDesc {
                number: 2,
                class: 0x03,
                endpoints: vec![endpoint(0x81, false, true)],
            },
        ];
        assert_eq!(
            Some(Endpoints {
                data_interface: 1,
                control_interface: 0,
                bulk_in: 0x84,
                bulk_out: 0x05,
            }),
            select_endpoints(&interfaces)
        );
    }

    #[test]
    fn selects_legacy_bulk_pair_when_data_interface_is_on_zero() {
        // a revision exposing the bulk pair on interface 0 (ep 0x81 / 0x01).
        let interfaces = vec![InterfaceDesc {
            number: 0,
            class: CDC_DATA_CLASS,
            endpoints: vec![endpoint(0x81, true, true), endpoint(0x01, true, false)],
        }];
        assert_eq!(
            Some(Endpoints {
                data_interface: 0,
                control_interface: 0,
                bulk_in: 0x81,
                bulk_out: 0x01,
            }),
            select_endpoints(&interfaces)
        );
    }

    #[test]
    fn selects_nothing_without_a_bulk_pair() {
        let interfaces = vec![InterfaceDesc {
            number: 0,
            class: CDC_COMM_CLASS,
            endpoints: vec![endpoint(0x83, false, true)],
        }];
        assert_eq!(None, select_endpoints(&interfaces));
    }

    #[test]
    fn open_missing_device_reports_not_found() {
        // a device index past anything that could be attached: enumeration
        // succeeds but the nth() selection finds nothing, so open() errors --
        // independent of whether real hardware is plugged in.
        let mut device = UsbDevice::new("16D0:09F8", 9999).unwrap();
        assert!(device.open().is_err());
    }

    #[test]
    #[ignore = "diagnostic: dumps the live usb device list"]
    fn dump_devices() {
        match nusb::list_devices() {
            Ok(devices) => {
                for d in devices {
                    eprintln!(
                        "{:04x}:{:04x} bus={} addr={} {:?}",
                        d.vendor_id(),
                        d.product_id(),
                        d.bus_number(),
                        d.device_address(),
                        d.product_string()
                    );
                }
            }
            Err(error) => eprintln!("list_devices error: {error}"),
        }
    }
}
