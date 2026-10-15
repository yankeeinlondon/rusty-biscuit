//! Encoding of a [`RequestBody`] into bytes.
//!
//! Every body is encoded into owned bytes before the first hop, so a 307 or
//! 308 redirect can resend exactly the same bytes.

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::{BufMut, Bytes, BytesMut};

use crate::endpoint::{FormPart, RequestBody};

/// An encoded body and its `Content-Type`.
#[derive(Debug, Clone)]
pub(crate) struct EncodedBody {
    pub(crate) content_type: String,
    pub(crate) bytes: Bytes,
}

/// Encodes `body`; `None` for [`RequestBody::Empty`].
pub(crate) fn encode(body: &RequestBody) -> Option<EncodedBody> {
    match body {
        RequestBody::Empty => None,
        RequestBody::Json(json) => Some(EncodedBody {
            content_type: "application/json".to_string(),
            bytes: Bytes::from(json.clone().into_bytes()),
        }),
        RequestBody::UrlEncoded(pairs) => Some(EncodedBody {
            content_type: "application/x-www-form-urlencoded".to_string(),
            bytes: Bytes::from(
                url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(pairs)
                    .finish()
                    .into_bytes(),
            ),
        }),
        RequestBody::Multipart(parts) => Some(multipart(parts)),
        RequestBody::Raw { content_type, bytes } => Some(EncodedBody {
            content_type: content_type.clone(),
            bytes: bytes.clone(),
        }),
    }
}

fn multipart(parts: &[FormPart]) -> EncodedBody {
    let boundary = boundary_for(parts);
    let mut out = BytesMut::new();
    for part in parts {
        out.put_slice(format!("--{boundary}\r\n").as_bytes());
        match part {
            FormPart::Text { name, value } => {
                put_disposition(&mut out, name, None);
                out.put_slice(b"\r\n");
                out.put_slice(value.as_bytes());
            }
            FormPart::Json { name, value } => {
                put_disposition(&mut out, name, None);
                out.put_slice(b"Content-Type: application/json\r\n\r\n");
                out.put_slice(value.as_bytes());
            }
            FormPart::File {
                name,
                file_name,
                mime,
                bytes,
            } => {
                put_disposition(&mut out, name, Some(file_name));
                let mime = mime.as_deref().unwrap_or("application/octet-stream");
                out.put_slice(format!("Content-Type: {mime}\r\n\r\n").as_bytes());
                out.put_slice(bytes);
            }
        }
        out.put_slice(b"\r\n");
    }
    out.put_slice(format!("--{boundary}--\r\n").as_bytes());
    EncodedBody {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        bytes: out.freeze(),
    }
}

fn put_disposition(out: &mut BytesMut, name: &str, file_name: Option<&str>) {
    out.put_slice(format!("Content-Disposition: form-data; name=\"{}\"", quote(name)).as_bytes());
    if let Some(file_name) = file_name {
        out.put_slice(format!("; filename=\"{}\"", quote(file_name)).as_bytes());
    }
    out.put_slice(b"\r\n");
}

/// Escapes a `Content-Disposition` parameter the way browsers do.
fn quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "%22").replace('\r', "%0D").replace('\n', "%0A")
}

/// A boundary that occurs in no part's content.
fn boundary_for(parts: &[FormPart]) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    loop {
        let candidate = format!(
            "schematic-{:016x}-{:08x}",
            std::process::id() as u64 ^ 0x5c4e_6d61_7469_6300,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let collides = parts.iter().any(|part| {
            let content: &[u8] = match part {
                FormPart::Text { value, .. } | FormPart::Json { value, .. } => value.as_bytes(),
                FormPart::File { bytes, .. } => bytes,
            };
            content.windows(candidate.len()).any(|window| window == candidate.as_bytes())
        });
        if !collides {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_multipart_body_frames_every_part_with_one_boundary() {
        let parts = [
            FormPart::Text {
                name: "model".into(),
                value: "m1".into(),
            },
            FormPart::File {
                name: "file".into(),
                file_name: "a\"b.txt".into(),
                mime: None,
                bytes: Bytes::from_static(b"hello"),
            },
        ];
        let encoded = multipart(&parts);
        let boundary = encoded.content_type.strip_prefix("multipart/form-data; boundary=").expect("boundary");
        let expected = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nm1\r\n\
             --{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a%22b.txt\"\r\n\
             Content-Type: application/octet-stream\r\n\r\nhello\r\n--{boundary}--\r\n"
        );
        assert_eq!(String::from_utf8(encoded.bytes.to_vec()).expect("utf-8"), expected);
    }

    #[test]
    fn a_boundary_never_occurs_in_the_content() {
        let parts = [FormPart::Text {
            name: "a".into(),
            value: "schematic-".repeat(4),
        }];
        let boundary = boundary_for(&parts);
        assert!(!"schematic-".repeat(4).contains(&boundary));
    }
}
