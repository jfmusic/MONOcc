//! minimal reader for usbmon `.pcapng` captures (linktype 220), enough to pull
//! the host<->device bulk payloads `inspect` analyzes. it walks pcapng blocks,
//! takes each enhanced-packet block's usbmon record, and returns the bulk data:
//! host->device rides the submit (`S`), device->host the callback (`C`).

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

const BLOCK_ENHANCED_PACKET: u32 = 0x0000_0006;
const USBMON_HEADER_LEN: usize = 64;
const TRANSFER_TYPE_BULK: u8 = 3;
const EPB_MIN_LEN: usize = 32; // 8 header + 20 fixed fields + 4 trailing length

/// read host<->device bulk payloads from a usbmon `.pcapng`, in capture order.
/// each item is `(device_to_host, bytes)`.
pub fn read_bulk_payloads(path: &Path) -> Result<Vec<(bool, Vec<u8>)>> {
    let data = fs::read(path)?;
    let mut payloads = Vec::new();
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let block_type = u32le(&data, offset);
        let block_len = u32le(&data, offset + 4) as usize;
        if block_len < 12 || offset + block_len > data.len() {
            break;
        }
        if block_type == BLOCK_ENHANCED_PACKET && block_len >= EPB_MIN_LEN {
            let captured = u32le(&data, offset + 20) as usize;
            let start = offset + 28;
            if let Some(packet) = data.get(start..start + captured) {
                if let Some(item) = usbmon_bulk(packet) {
                    payloads.push(item);
                }
            }
        }
        offset += block_len;
    }
    if payloads.is_empty() {
        bail!("no usb bulk payloads found (is this a usbmon .pcapng capture?)");
    }
    Ok(payloads)
}

/// the bulk data payload of one usbmon packet, if it carries one. only the
/// data-bearing side of a transfer is kept: the submit for an out endpoint,
/// the callback for an in endpoint.
fn usbmon_bulk(packet: &[u8]) -> Option<(bool, Vec<u8>)> {
    if packet.len() < USBMON_HEADER_LEN {
        return None;
    }
    let event_type = packet[8]; // 'S' submit / 'C' callback
    let transfer_type = packet[9];
    let endpoint = packet[10];
    if transfer_type != TRANSFER_TYPE_BULK {
        return None;
    }
    let data_len = u32le(packet, 36) as usize;
    let data = packet.get(USBMON_HEADER_LEN..USBMON_HEADER_LEN + data_len)?;
    let device_to_host = endpoint & 0x80 != 0;
    let carries_data = if device_to_host {
        event_type == b'C'
    } else {
        event_type == b'S'
    };
    if !carries_data || data.is_empty() {
        return None;
    }
    Some((device_to_host, data.to_vec()))
}

fn u32le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// build a one-packet usbmon `.pcapng` (section header + interface + a
    /// single enhanced-packet block wrapping a bulk usbmon record).
    fn synth_pcapng(endpoint: u8, event_type: u8, data: &[u8]) -> Vec<u8> {
        let mut usbmon = vec![0u8; USBMON_HEADER_LEN];
        usbmon[8] = event_type;
        usbmon[9] = TRANSFER_TYPE_BULK;
        usbmon[10] = endpoint;
        usbmon[36..40].copy_from_slice(&(data.len() as u32).to_le_bytes());
        usbmon.extend_from_slice(data);
        while !usbmon.len().is_multiple_of(4) {
            usbmon.push(0);
        }

        let mut out = Vec::new();
        // section header block (declares little-endian byte order).
        out.extend_from_slice(&0x0A0D_0D0Au32.to_le_bytes());
        out.extend_from_slice(&28u32.to_le_bytes());
        out.extend_from_slice(&0x1A2B_3C4Du32.to_le_bytes());
        out.extend_from_slice(&[0xFF; 12]); // version (4) + section length (8)
        out.extend_from_slice(&28u32.to_le_bytes());
        // interface description block, linktype 220 (usbmon).
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&20u32.to_le_bytes());
        out.extend_from_slice(&220u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&20u32.to_le_bytes());
        // enhanced packet block carrying the usbmon record.
        let body_len = 20 + usbmon.len();
        let block_len = (body_len + 12) as u32;
        out.extend_from_slice(&BLOCK_ENHANCED_PACKET.to_le_bytes());
        out.extend_from_slice(&block_len.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // interface id
        out.extend_from_slice(&0u32.to_le_bytes()); // ts high
        out.extend_from_slice(&0u32.to_le_bytes()); // ts low
        out.extend_from_slice(&(usbmon.len() as u32).to_le_bytes()); // captured len
        out.extend_from_slice(&(usbmon.len() as u32).to_le_bytes()); // original len
        out.extend_from_slice(&usbmon);
        out.extend_from_slice(&block_len.to_le_bytes());
        out
    }

    #[test]
    fn reads_out_submit_and_in_callback_bulk_payloads() {
        let dir = std::env::temp_dir();
        let out = dir.join("sextant_pcap_out.pcapng");
        fs::write(&out, synth_pcapng(0x01, b'S', &[0x7e, 0x91, 0x01, 0x7e])).unwrap();
        assert_eq!(
            vec![(false, vec![0x7e, 0x91, 0x01, 0x7e])],
            read_bulk_payloads(&out).unwrap()
        );

        let inn = dir.join("sextant_pcap_in.pcapng");
        fs::write(&inn, synth_pcapng(0x81, b'C', &[0x7e, 0x92, 0x7e])).unwrap();
        assert_eq!(
            vec![(true, vec![0x7e, 0x92, 0x7e])],
            read_bulk_payloads(&inn).unwrap()
        );
    }

    #[test]
    fn skips_the_non_data_side_of_a_transfer() {
        // an out callback ('C') and an in submit ('S') carry no data.
        let dir = std::env::temp_dir();
        let path = dir.join("sextant_pcap_nodata.pcapng");
        fs::write(&path, synth_pcapng(0x01, b'C', &[0x91, 0x01])).unwrap();
        assert!(read_bulk_payloads(&path).is_err());
    }
}
