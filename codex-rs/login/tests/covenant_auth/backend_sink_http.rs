//! Bounded local HTTP authority for Covenant auth tests.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);
const HTTP_LIMIT: usize = 32 * 1024;
const PROBE_DEADLINE: Duration = Duration::from_secs(/*secs*/ 5);
const TRAILING_GRACE: Duration = Duration::from_millis(/*millis*/ 100);
const AMBIGUOUS_FRAMING: &str = "ambiguous transfer framing";
const TRAILING_BYTES: &str = "trailing fixture request bytes";

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptHashCertificate2(
        algorithm: *const u16,
        flags: u32,
        reserved: *mut (),
        input: *const u8,
        input_len: u32,
        output: *mut u8,
        output_len: *mut u32,
    ) -> i32;
}

pub(super) struct Step {
    request: ExpectedRequest,
    status: u16,
    body: Vec<u8>,
    body_consumed: Option<mpsc::SyncSender<()>>,
}

struct ExpectedRequest {
    line: String,
    bearer: Option<String>,
    body: ExpectedBody,
}

pub(super) enum ExpectedBody {
    Empty,
    Json(serde_json::Value),
    Form(BTreeMap<String, String>),
    PkceForm {
        expected: BTreeMap<String, String>,
        binding: mpsc::Receiver<PkceExpectation>,
    },
}

pub(super) struct PkceExpectation {
    pub(super) redirect_uri: String,
    pub(super) code_challenge: String,
}

impl Step {
    pub(super) fn json(
        method: &'static str,
        path: &'static str,
        status: u16,
        body: serde_json::Value,
        expected_body: ExpectedBody,
    ) -> Result<Self> {
        Ok(Self {
            request: ExpectedRequest {
                line: format!("{method} {path} HTTP/1.1"),
                bearer: None,
                body: expected_body,
            },
            status,
            body: serde_json::to_vec(&body)?,
            body_consumed: None,
        })
    }

    pub(super) fn bearer(mut self, bearer: String) -> Self {
        self.request.bearer = Some(bearer);
        self
    }
}

pub(super) struct HttpFixture {
    url: String,
    port: u16,
    result: mpsc::Receiver<Result<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl HttpFixture {
    pub(super) fn start(
        steps: Vec<Step>,
        configure: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Self> {
        Self::start_dynamic(|_| Ok(steps), configure)
    }

    pub(super) fn start_dynamic(
        steps: impl FnOnce(&str) -> Result<Vec<Step>>,
        configure: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let url = format!("http://{address}");
        let steps = steps(&url)?;
        configure(&url)?;
        let (sender, result) = mpsc::channel();
        let thread = thread::spawn(move || {
            let outcome = serve_steps(listener, steps);
            let _ = sender.send(outcome);
        });
        Ok(Self {
            url,
            port: address.port(),
            result,
            thread: Some(thread),
        })
    }

    pub(super) fn url(&self) -> &str {
        &self.url
    }

    fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn finish(mut self) -> Result<()> {
        let outcome = self.result.recv_timeout(DEADLINE)?;
        let thread = self.thread.take().context("HTTP fixture thread missing")?;
        thread
            .join()
            .map_err(|_| anyhow::anyhow!("HTTP fixture panicked"))?;
        outcome
    }
}

pub(super) fn assert_rejection_probes() -> Result<()> {
    let fixture = HttpFixture::start(probe_steps(None)?, |_| Ok(()))?;
    let mut stream = probe_stream(fixture.port())?;
    write!(
        stream,
        "POST /probe HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nContent-Length: 0\r\n\r\n"
    )?;
    stream.flush()?;
    assert_fixture_rejection(fixture, AMBIGUOUS_FRAMING)?;

    let (sender, consumed) = mpsc::sync_channel(/*bound*/ 1);
    let fixture = HttpFixture::start(probe_steps(Some(sender))?, |_| Ok(()))?;
    let mut stream = probe_stream(fixture.port())?;
    write!(
        stream,
        "POST /probe HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\n\r\nx"
    )?;
    stream.flush()?;
    consumed
        .recv_timeout(PROBE_DEADLINE)
        .context("declared body consumed event missing")?;
    stream.write_all(b"G")?;
    stream.flush()?;
    assert_fixture_rejection(fixture, TRAILING_BYTES)
}

fn probe_stream(port: u16) -> Result<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_write_timeout(Some(PROBE_DEADLINE))?;
    Ok(stream)
}

fn probe_steps(body_consumed: Option<mpsc::SyncSender<()>>) -> Result<Vec<Step>> {
    let mut step = Step::json(
        "POST",
        "/probe",
        /*status*/ 200,
        serde_json::json!({}),
        ExpectedBody::Empty,
    )?;
    step.body_consumed = body_consumed;
    Ok(vec![step])
}

fn assert_fixture_rejection(fixture: HttpFixture, expected: &str) -> Result<()> {
    let error = fixture
        .finish()
        .expect_err("malformed fixture request was accepted");
    ensure!(
        error.to_string() == expected,
        "wrong fixture rejection: {error}"
    );
    Ok(())
}

pub(super) fn raw_get(port: u16, path: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(DEADLINE))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()?;
    let mut response = String::new();
    stream
        .take(2 * HTTP_LIMIT as u64)
        .read_to_string(&mut response)?;
    Ok(response)
}

fn serve_steps(listener: TcpListener, steps: Vec<Step>) -> Result<()> {
    for step in steps {
        let deadline = Instant::now() + DEADLINE;
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    ensure!(Instant::now() < deadline, "HTTP fixture timed out");
                    thread::sleep(Duration::from_millis(/*millis*/ 5));
                }
                Err(error) => return Err(error.into()),
            }
        };
        stream.set_nonblocking(false)?;
        let request = read_http_request(&mut stream, step.body_consumed.as_ref())?;
        let (request_line, headers, request_body) = request;
        ensure!(request_line == step.request.line, "wrong HTTP request line");
        let content_type = match &step.request.body {
            ExpectedBody::Empty => None,
            ExpectedBody::Json(_) => Some("application/json"),
            ExpectedBody::Form(_) | ExpectedBody::PkceForm { .. } => {
                Some("application/x-www-form-urlencoded")
            }
        };
        if headers.get("content-type").map(String::as_str) != content_type
            || headers.get("authorization") != step.request.bearer.as_ref()
        {
            anyhow::bail!("wrong HTTP headers");
        }
        match step.request.body {
            ExpectedBody::Empty => ensure!(request_body.is_empty(), "unexpected fixture body"),
            ExpectedBody::Json(expected) => ensure!(
                serde_json::from_slice::<Value>(&request_body)? == expected,
                "wrong JSON"
            ),
            ExpectedBody::Form(expected) => {
                let actual = parse_form(&request_body)?;
                ensure!(actual == expected, "wrong form")
            }
            ExpectedBody::PkceForm { expected, binding } => {
                validate_pkce_form(parse_form(&request_body)?, expected, binding)?
            }
        }
        let reason = if step.status == 200 {
            "OK"
        } else {
            "Internal Server Error"
        };
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            step.status,
            reason,
            step.body.len()
        )?;
        stream.write_all(&step.body)?;
        stream.flush()?;
    }
    let deadline = Instant::now() + Duration::from_millis(/*millis*/ 100);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok(_) => anyhow::bail!("unexpected extra HTTP fixture request"),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        thread::sleep(Duration::from_millis(/*millis*/ 5));
    }
    Ok(())
}

type HttpRequest = (String, BTreeMap<String, String>, Vec<u8>);

fn read_http_request(
    stream: &mut TcpStream,
    body_consumed: Option<&mpsc::SyncSender<()>>,
) -> Result<HttpRequest> {
    stream.set_read_timeout(Some(DEADLINE))?;
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        ensure!(bytes.len() < HTTP_LIMIT, "oversized fixture header");
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    let header = String::from_utf8(bytes)?;
    let mut lines = header[..header.len() - 4].split("\r\n");
    let request_line = lines.next().context("request line missing")?;
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').context("malformed fixture header")?;
        let name = name.to_ascii_lowercase();
        let prior = headers.insert(name, value.trim().to_string());
        ensure!(prior.is_none(), "duplicate header");
    }
    ensure!(
        !headers.contains_key("transfer-encoding"),
        AMBIGUOUS_FRAMING
    );
    let length = headers.get("content-length").map_or("0", String::as_str);
    ensure!(
        !length.is_empty() && length.bytes().all(|byte| byte.is_ascii_digit()),
        "invalid fixture content length"
    );
    let length = length.parse::<usize>()?;
    ensure!(length <= HTTP_LIMIT, "oversized fixture body");
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    if let Some(sender) = body_consumed {
        sender
            .try_send(())
            .context("declared body consumed event duplicated or late")?;
    }
    stream.set_read_timeout(Some(TRAILING_GRACE))?;
    let mut trailing = [0];
    match stream.read(&mut trailing) {
        Ok(0) => {}
        Ok(_) => anyhow::bail!(TRAILING_BYTES),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) => {}
        Err(error) => return Err(error.into()),
    }
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(DEADLINE))?;
    Ok((request_line.to_string(), headers, body))
}

fn validate_pkce_form(
    mut actual: BTreeMap<String, String>,
    expected: BTreeMap<String, String>,
    binding: mpsc::Receiver<PkceExpectation>,
) -> Result<()> {
    ensure!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")?
            == "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        "fixture S256 implementation failed RFC 7636 vector"
    );
    let expectation = binding
        .recv_timeout(PROBE_DEADLINE)
        .context("PKCE binding missing or late")?;
    ensure!(
        matches!(binding.try_recv(), Err(mpsc::TryRecvError::Disconnected)),
        "PKCE binding was duplicated or publisher remained live"
    );
    let verifier = actual
        .remove("code_verifier")
        .context("missing PKCE verifier")?;
    ensure!(
        (43..=128).contains(&verifier.len())
            && verifier.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
            }),
        "invalid PKCE verifier"
    );
    ensure!(
        pkce_challenge(&verifier)? == expectation.code_challenge,
        "PKCE verifier did not match authorization challenge"
    );
    ensure!(
        actual.remove("redirect_uri") == Some(expectation.redirect_uri),
        "wrong PKCE redirect"
    );
    ensure!(actual == expected, "wrong PKCE form");
    Ok(())
}

fn pkce_challenge(verifier: &str) -> Result<String> {
    let name: Vec<u16> = "SHA256\0".encode_utf16().collect();
    let mut digest = [0_u8; 32];
    let mut digest_len = digest.len() as u32;
    ensure!(
        unsafe {
            CryptHashCertificate2(
                name.as_ptr(),
                0,
                std::ptr::null_mut(),
                verifier.as_ptr(),
                verifier.len() as u32,
                digest.as_mut_ptr(),
                &mut digest_len,
            )
        } != 0,
        "could not compute SHA-256"
    );
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::with_capacity(43);
    for chunk in digest.chunks(3) {
        let bits = chunk
            .iter()
            .fold(0, |bits, byte| (bits << 8) | u32::from(*byte))
            << ((3 - chunk.len()) * 8);
        for shift in [18, 12, 6, 0].into_iter().take(chunk.len() + 1) {
            encoded.push(ALPHABET[((bits >> shift) & 63) as usize] as char);
        }
    }
    Ok(encoded)
}

pub(super) fn parse_form(body: &[u8]) -> Result<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();
    for pair in std::str::from_utf8(body)?.split('&') {
        let (name, value) = pair.split_once('=').context("malformed fixture form")?;
        let decode = |component: &str| -> Result<String> {
            let component = component.replace('+', " ");
            let mut parts = component.split('%');
            let mut decoded = parts.next().unwrap_or_default().as_bytes().to_vec();
            for part in parts {
                ensure!(part.len() >= 2, "malformed fixture form escape");
                decoded.push(u8::from_str_radix(
                    std::str::from_utf8(&part.as_bytes()[..2])?,
                    16,
                )?);
                decoded.extend_from_slice(&part.as_bytes()[2..]);
            }
            Ok(String::from_utf8(decoded)?)
        };
        let name = decode(name)?;
        ensure!(!name.is_empty(), "empty fixture form key");
        let prior = fields.insert(name, decode(value)?);
        ensure!(prior.is_none(), "duplicate fixture form key");
    }
    Ok(fields)
}
