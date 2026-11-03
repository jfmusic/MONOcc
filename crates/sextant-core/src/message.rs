//! wire-protocol framing for the two device generations.
//!
//! palettegear (legacy) speaks minified json with no delimiter between objects;
//! monogram cc speaks msgpack objects framed by `0x7e` (`~`). a `screen_write`
//! json header carries a trailing binary blob that must be split off before the
//! json is parsed. both directions are kept byte-compatible with the original
//! python implementation, including its known fragilities (see `decode_msgpack`).

use serde_json::Value;

const FRAME: u8 = b'~';
const SCREEN_WRITE_PREFIX: &[u8] = b"{\"screen_write\":";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageFormat {
    Json,
    Msgpack,
}

/// encode a batch of messages with optional trailing binary data.
pub fn encode(messages: &[Value], format: MessageFormat, extra: &[u8]) -> Vec<u8> {
    match format {
        MessageFormat::Msgpack => encode_msgpack(messages, extra),
        MessageFormat::Json => encode_json(messages, extra),
    }
}

/// decode a buffer into messages plus any trailing binary data.
pub fn decode(data: &[u8], format: MessageFormat) -> (Vec<Value>, Vec<u8>) {
    match format {
        MessageFormat::Msgpack => decode_msgpack(data),
        MessageFormat::Json => decode_json(data),
    }
}

fn encode_json(messages: &[Value], extra: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for message in messages {
        // serde_json is compact by default; preserve_order keeps key order.
        serde_json::to_writer(&mut out, message).expect("json message is serializable");
    }
    out.extend_from_slice(extra);
    out
}

fn decode_json(data: &[u8]) -> (Vec<Value>, Vec<u8>) {
    let mut extra: Vec<u8> = Vec::new();
    let mut body = data;

    if data.starts_with(SCREEN_WRITE_PREFIX) {
        if let Some(end) = data.iter().position(|&b| b == b'}') {
            let split = end + 1;
            extra = data[split..].to_vec();
            body = &data[..split];
        }
    }

    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(_) => return (Vec::new(), extra),
    };

    // the upstream parser concatenates objects and naively rewrites `}{` into
    // `},{` so a run of objects parses as a json array. preserved verbatim,
    // including the documented failure on a literal `}{` inside a string.
    let joined = format!("[{}]", text.trim().replace('\n', ",").replace("}{", "},{"));
    let messages = serde_json::from_str::<Vec<Value>>(&joined).unwrap_or_default();
    (messages, extra)
}

// The official CoreV2 codec escapes 0x7d and 0x7e with 0x7d, byte XOR 0x20.
fn escape_payload(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for &byte in bytes {
        if matches!(byte, 0x7d | 0x7e) {
            out.extend_from_slice(&[0x7d, byte ^ 0x20]);
        } else {
            out.push(byte);
        }
    }
    out
}

fn unescape_payload(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut escaped = false;
    for &byte in bytes {
        if escaped {
            out.push(byte ^ 0x20);
            escaped = false;
        } else if byte == 0x7d {
            escaped = true;
        } else {
            out.push(byte);
        }
    }
    out
}

fn encode_msgpack(messages: &[Value], extra: &[u8]) -> Vec<u8> {
    let mut out = vec![FRAME];
    for (i, message) in messages.iter().enumerate() {
        if i > 0 {
            out.push(FRAME);
            out.push(FRAME);
        }
        let mut payload = Vec::new();
        rmpv::encode::write_value(&mut payload, &json_to_rmpv(message))
            .expect("msgpack message is serializable");
        out.extend(escape_payload(&payload));
    }
    out.push(FRAME);
    if !extra.is_empty() {
        out.extend_from_slice(extra);
        out.push(FRAME);
    }
    out
}

fn decode_msgpack(data: &[u8]) -> (Vec<Value>, Vec<u8>) {
    // Split wire frames before unescaping, so literal payload 0x7e is preserved.
    let parts: Vec<&[u8]> = data.split(|&b| b == FRAME).collect();
    if parts.len() < 2 {
        return (Vec::new(), Vec::new());
    }

    let tail = parts[parts.len() - 1];
    let extra = if tail.is_empty() {
        Vec::new()
    } else {
        let mut e = vec![FRAME];
        e.extend_from_slice(tail);
        e
    };

    let mut messages = Vec::new();
    for part in &parts[1..parts.len() - 1] {
        let payload = unescape_payload(part);
        let mut rd: &[u8] = &payload;
        if let Ok(value) = rmpv::decode::read_value(&mut rd) {
            messages.push(rmpv_to_json(&value));
        }
    }
    (messages, extra)
}

/// encode a `write_display_slow` strip -- a monogram core-screen rectangle from
/// `(x0, y0)` to `(x1, y1)` filled with `pixels` (rgb565) -- framed for the bulk
/// endpoint. the pixels are a msgpack **binary**, which the json message path
/// cannot express, so the rmpv value is built directly.
pub fn encode_display_strip(
    more: bool,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    pixels: &[u8],
) -> Vec<u8> {
    use rmpv::Value as R;
    let message = R::Map(vec![(
        R::String("write_display_slow".into()),
        R::Array(vec![
            R::Boolean(more),
            R::Integer(x0.into()),
            R::Integer(y0.into()),
            R::Integer(x1.into()),
            R::Integer(y1.into()),
            R::Binary(pixels.to_vec()),
        ]),
    )]);
    let mut payload = Vec::new();
    rmpv::encode::write_value(&mut payload, &message).expect("display strip is serializable");
    let mut out = vec![FRAME];
    out.extend(escape_payload(&payload));
    out.push(FRAME);
    out
}

/// decode a `0x7e`-framed msgpack byte stream length-aware: msgpack is
/// self-delimiting, so we skip frame delimiters and read one value at a time.
/// unlike [`decode`], this tolerates a frame byte inside a payload (an input
/// `v` register can legitimately be `126`), so it is the decoder used for
/// offline capture analysis where correctness matters more than byte-fidelity.
pub fn decode_msgpack_lenient(data: &[u8]) -> Vec<Value> {
    let mut messages = Vec::new();
    let payload = unescape_payload(data);
    let mut rd: &[u8] = &payload;
    while !rd.is_empty() {
        // a real message is a map or array, never a bare 0x7e, so a leading
        // frame byte is always a delimiter to skip.
        if rd[0] == FRAME {
            rd = &rd[1..];
            continue;
        }
        match rmpv::decode::read_value(&mut rd) {
            Ok(value) => messages.push(rmpv_to_json(&value)),
            Err(_) => break,
        }
    }
    messages
}

fn json_to_rmpv(value: &Value) -> rmpv::Value {
    use rmpv::Value as R;
    match value {
        Value::Null => R::Nil,
        Value::Bool(b) => R::Boolean(*b),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                R::Integer(u.into())
            } else if let Some(i) = n.as_i64() {
                R::Integer(i.into())
            } else {
                R::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => R::String(s.as_str().into()),
        Value::Array(a) => R::Array(a.iter().map(json_to_rmpv).collect()),
        Value::Object(o) => R::Map(
            o.iter()
                .map(|(k, v)| (R::String(k.as_str().into()), json_to_rmpv(v)))
                .collect(),
        ),
    }
}

fn rmpv_to_json(value: &rmpv::Value) -> Value {
    use rmpv::Value as R;
    match value {
        R::Nil => Value::Null,
        R::Boolean(b) => Value::Bool(*b),
        R::Integer(i) => {
            if let Some(u) = i.as_u64() {
                Value::Number(u.into())
            } else if let Some(s) = i.as_i64() {
                Value::Number(s.into())
            } else {
                Value::Null
            }
        }
        R::F32(f) => serde_json::json!(*f),
        R::F64(f) => serde_json::json!(*f),
        R::String(s) => Value::String(s.as_str().unwrap_or_default().to_string()),
        R::Binary(b) => Value::Array(b.iter().map(|&x| Value::Number(x.into())).collect()),
        R::Array(a) => Value::Array(a.iter().map(rmpv_to_json).collect()),
        R::Map(m) => Value::Object(
            m.iter()
                .map(|(k, v)| {
                    let key = match k {
                        R::String(s) => s.as_str().unwrap_or_default().to_string(),
                        other => other.to_string(),
                    };
                    (key, rmpv_to_json(v))
                })
                .collect(),
        ),
        R::Ext(_, _) => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn official_escaped_slider_frame_preserves_channels() {
        // Wire fixture: center slider at 126, its neighbors remain at 255.
        let wire = [
            0x7e, 0x81, 0xa2, b'i', b'n', 0x91, 0x82, 0xa1, b'i', 1, 0xa1, b'v', 0x98, 0, 0xcc,
            255, 0x7d, 0x5e, 0xcc, 255, 0, 0, 0, 0, 0x7e,
        ];
        let expected = json!({"in": [{"i": 1, "v": [0,255,126,255,0,0,0,0]}]});
        assert_eq!(
            vec![expected.clone()],
            decode(&wire, MessageFormat::Msgpack).0
        );
        assert_eq!(vec![expected], decode_msgpack_lenient(&wire));
        for value in [125, 126] {
            let message = json!({"v": [value, 255]});
            let encoded = encode(std::slice::from_ref(&message), MessageFormat::Msgpack, &[]);
            assert!(encoded.windows(2).any(|p| p == [0x7d, value as u8 ^ 0x20]));
            assert_eq!(vec![message], decode(&encoded, MessageFormat::Msgpack).0);
        }
    }

    #[test]
    fn display_strip_carries_a_binary_payload() {
        // two rgb565 pixels (red, green) in a 0,4..0,8 strip.
        let pixels = vec![0xF8, 0x00, 0x07, 0xE0];
        let encoded = encode_display_strip(true, 0, 4, 240, 8, &pixels);
        let decoded = decode_msgpack_lenient(&encoded);
        assert_eq!(1, decoded.len());
        let strip = &decoded[0]["write_display_slow"];
        assert_eq!(json!(true), strip[0]);
        assert_eq!(json!(4), strip[2]);
        assert_eq!(json!(240), strip[3]);
        assert_eq!(json!(8), strip[4]);
        // the binary decodes to an array of bytes (rmpv has no json bin type).
        assert_eq!(json!([248, 0, 7, 224]), strip[5]);
    }

    #[test]
    fn lenient_msgpack_decode_tolerates_frame_byte_in_payload() {
        // an input frame whose v register holds 126 (== the 0x7e frame byte):
        // the naive splitter corrupts it, the length-aware decoder does not.
        let message = json!({"in": [{"i": 2, "v": [0, 0, 126, 0, 0, 0, 0, 0]}]});
        let encoded = encode(std::slice::from_ref(&message), MessageFormat::Msgpack, &[]);
        assert!(encoded.contains(&FRAME));

        let lenient = decode_msgpack_lenient(&encoded);
        assert_eq!(vec![message], lenient);

        // two framed messages decode back to both.
        let two = encode(
            &[json!({"a": 1}), json!({"b": 126})],
            MessageFormat::Msgpack,
            &[],
        );
        assert_eq!(
            vec![json!({"a": 1}), json!({"b": 126})],
            decode_msgpack_lenient(&two)
        );
    }

    #[test]
    fn encode_json_concatenates_without_delimiter() {
        let data = encode(
            &[json!({"check": 1}), json!({"screen_string": "TEST"})],
            MessageFormat::Json,
            &[],
        );
        assert_eq!(b"{\"check\":1}{\"screen_string\":\"TEST\"}".to_vec(), data);
    }

    #[test]
    fn decode_json_splits_screen_write_blob() {
        let (messages, extra) = decode(b"{\"screen_write\":23}EXTRA_DATA", MessageFormat::Json);
        assert_eq!(vec![json!({"screen_write": 23})], messages);
        assert_eq!(b"EXTRA_DATA".to_vec(), extra);
    }

    #[test]
    fn decode_json_runs_of_objects() {
        let (messages, _) = decode(b"{\"a\":1}{\"b\":2}", MessageFormat::Json);
        assert_eq!(vec![json!({"a": 1}), json!({"b": 2})], messages);
    }

    #[test]
    fn msgpack_frames_round_trip() {
        let messages = vec![json!({"check": 1}), json!({"led": [255, 0, 0]})];
        let encoded = encode(&messages, MessageFormat::Msgpack, &[]);
        assert_eq!(Some(&FRAME), encoded.first());
        assert_eq!(Some(&FRAME), encoded.last());
        let (decoded, extra) = decode(&encoded, MessageFormat::Msgpack);
        assert_eq!(messages, decoded);
        assert!(extra.is_empty());
    }

    #[test]
    fn msgpack_decode_recovers_trailing_blob() {
        // a device frames messages then appends a raw blob with no closing
        // frame; decode returns it (re-prefixed with the frame byte) as extra.
        let mut buf = encode(&[json!({"screen_write": 1})], MessageFormat::Msgpack, &[]);
        buf.extend_from_slice(b"BLOB");
        let (_, extra) = decode(&buf, MessageFormat::Msgpack);
        assert_eq!(b"~BLOB".to_vec(), extra);
    }
}
