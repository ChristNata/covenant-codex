//! Unregistered draft: real byte framing and retained request data, no sockets.
use super::FixtureFailure;
use super::HttpDecoder;
use super::HttpLimits;
use super::HttpRequest;
use pretty_assertions::assert_eq;

fn limits() -> HttpLimits {
    HttpLimits {
        headers: 128,
        body: 16,
        total: 144,
    }
}

fn wire(body: &[u8]) -> Vec<u8> {
    let mut bytes = format!(
        "POST /mcp/nonce HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn request(body: &[u8]) -> HttpRequest {
    HttpRequest {
        method: "POST".to_owned(),
        target: "/mcp/nonce".to_owned(),
        headers: vec![
            ("host".to_owned(), "localhost".to_owned()),
            ("content-length".to_owned(), body.len().to_string()),
        ],
        body: body.to_vec(),
    }
}

#[test]
fn framing_preserves_complete_request_for_every_split() {
    let bytes = wire(b"{\"id\":7}");
    for split in 0..=bytes.len() {
        let mut decoder = HttpDecoder::new(limits());
        let first = decoder.feed(&bytes[..split]).unwrap();
        let second = decoder.feed(&bytes[split..]).unwrap();
        let records: Vec<_> = first.into_iter().chain(second).collect();
        assert_eq!(records, vec![request(b"{\"id\":7}")]);
        assert_eq!(decoder.finish_eof(), Ok(()));
    }
}

#[test]
fn framing_requires_complete_headers_and_declared_body_at_eof() {
    let bytes = wire(b"12345");
    for end in 0..bytes.len() {
        let mut decoder = HttpDecoder::new(limits());
        assert_eq!(decoder.feed(&bytes[..end]), Ok(None));
        assert_eq!(decoder.finish_eof(), Err(FixtureFailure::Incomplete));
    }
}

#[test]
fn framing_enforces_inclusive_body_header_and_total_limits() {
    let body = [b'x'; 16];
    let bytes = wire(&body);
    let header_size = bytes.len() - body.len();
    let exact = HttpLimits {
        headers: header_size,
        body: 16,
        total: bytes.len(),
    };
    let mut decoder = HttpDecoder::new(exact);
    assert_eq!(decoder.feed(&bytes).unwrap(), Some(request(&body)));
    assert_eq!(decoder.finish_eof(), Ok(()));
    for bounds in [
        HttpLimits {
            headers: header_size - 1,
            body: 16,
            total: bytes.len(),
        },
        HttpLimits {
            headers: header_size,
            body: 15,
            total: bytes.len(),
        },
        HttpLimits {
            headers: header_size,
            body: 16,
            total: bytes.len() - 1,
        },
    ] {
        let mut decoder = HttpDecoder::new(bounds);
        assert_eq!(decoder.feed(&bytes), Err(FixtureFailure::Limit));
        assert_eq!(decoder.feed(b""), Err(FixtureFailure::Limit));
    }
}

#[test]
fn framing_rejects_ambiguous_lengths_and_unsupported_transfer_encoding() {
    for header in [
        "Content-Length: 0\r\nContent-Length: 0\r\n",
        "Content-Length: 0\r\ncOnTeNt-LeNgTh: 1\r\n",
        "Content-Length: +1\r\n",
        "Content-Length: -1\r\n",
        "Content-Length: 1, 1\r\n",
        "Content-Length: 184467440737095516160\r\n",
        "Transfer-Encoding: chunked\r\nContent-Length: 0\r\n",
        "Transfer-Encoding: identity\r\n",
    ] {
        let bytes = format!("POST /mcp/nonce HTTP/1.1\r\nHost: local\r\n{header}\r\n");
        let mut decoder = HttpDecoder::new(limits());
        assert_eq!(decoder.feed(bytes.as_bytes()), Err(FixtureFailure::Framing));
    }
}

#[test]
fn framing_rejects_missing_post_length_bad_lines_and_duplicate_host() {
    for bytes in [
        "POST /mcp/nonce HTTP/1.1\r\nHost: local\r\n\r\n",
        "GET /mcp/nonce HTTP/1.1\r\n\r\n",
        "GET /mcp/nonce HTTP/1.1\nHost: local\n\n",
        "GET /mcp/nonce HTTP/1.1\r\n folded: value\r\n\r\n",
        "GET /mcp/nonce HTTP/1.1\r\nHost: a\r\nhost: b\r\n\r\n",
        "GET /mcp/nonce HTTP/1.1\r\nHost: local\r\nX-Test: a\0b\r\n\r\n",
    ] {
        let mut decoder = HttpDecoder::new(limits());
        let fed = decoder.feed(bytes.as_bytes());
        assert!(
            fed.is_err() || decoder.finish_eof().is_err(),
            "accepted {bytes:?}"
        );
    }
}

#[test]
fn framing_does_not_discard_coalesced_or_later_read_ahead() {
    for tail in [
        b"G".as_slice(),
        b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".as_slice(),
    ] {
        let mut bytes = wire(b"{}");
        bytes.extend_from_slice(tail);
        let mut combined = HttpDecoder::new(HttpLimits {
            headers: 128,
            body: 16,
            total: 256,
        });
        assert_eq!(combined.feed(&bytes), Err(FixtureFailure::Framing));
        let mut separate = HttpDecoder::new(limits());
        assert_eq!(separate.feed(&wire(b"{}")), Ok(Some(request(b"{}"))));
        assert_eq!(separate.feed(tail), Err(FixtureFailure::Framing));
        assert_eq!(separate.finish_eof(), Err(FixtureFailure::Framing));
    }
}

#[test]
fn framing_preserves_repeated_non_framing_headers_and_empty_get_body() {
    let bytes = b"GET /mcp/nonce HTTP/1.1\r\nHost: local\r\nX-Test: one\r\nX-Test: two\r\n\r\n";
    let expected = HttpRequest {
        method: "GET".to_owned(),
        target: "/mcp/nonce".to_owned(),
        headers: vec![
            ("host".to_owned(), "local".to_owned()),
            ("x-test".to_owned(), "one".to_owned()),
            ("x-test".to_owned(), "two".to_owned()),
        ],
        body: Vec::new(),
    };
    let mut decoder = HttpDecoder::new(limits());
    assert_eq!(decoder.feed(bytes), Ok(Some(expected)));
    assert_eq!(decoder.finish_eof(), Ok(()));
}
