use super::FixtureFailure;
use super::HttpLimits;
use super::HttpRequest;

pub(super) struct HttpDecoder {
    limits: HttpLimits,
    header: Vec<u8>,
    request: Option<HttpRequest>,
    body_length: usize,
    received: usize,
    emitted: bool,
    failure: Option<FixtureFailure>,
}

impl HttpDecoder {
    pub(super) fn new(limits: HttpLimits) -> Self {
        Self {
            limits,
            header: Vec::new(),
            request: None,
            body_length: 0,
            received: 0,
            emitted: false,
            failure: None,
        }
    }

    pub(super) fn feed(&mut self, bytes: &[u8]) -> Result<Option<HttpRequest>, FixtureFailure> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let result = self.consume(bytes);
        if let Err(error) = result {
            self.failure = Some(error);
        }
        result
    }

    fn consume(&mut self, mut bytes: &[u8]) -> Result<Option<HttpRequest>, FixtureFailure> {
        if self.emitted {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(FixtureFailure::Framing)
            };
        }
        self.received = self
            .received
            .checked_add(bytes.len())
            .filter(|size| *size <= self.limits.total)
            .ok_or(FixtureFailure::Limit)?;
        while self.request.is_none() && !bytes.is_empty() {
            if self.header.len() >= self.limits.headers {
                return Err(FixtureFailure::Limit);
            }
            self.header.push(bytes[0]);
            bytes = &bytes[1..];
            if self.header.ends_with(b"\r\n\r\n") {
                let (request, body_length) = parse_header(&self.header, self.limits)?;
                self.request = Some(request);
                self.body_length = body_length;
            }
        }
        let Some(request) = self.request.as_mut() else {
            return Ok(None);
        };
        if bytes.len() > self.body_length - request.body.len() {
            return Err(FixtureFailure::Framing);
        }
        request.body.extend_from_slice(bytes);
        if request.body.len() == self.body_length {
            self.emitted = true;
            Ok(self.request.take())
        } else {
            Ok(None)
        }
    }

    pub(super) fn finish_eof(&mut self) -> Result<(), FixtureFailure> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.emitted {
            Ok(())
        } else {
            self.failure = Some(FixtureFailure::Incomplete);
            Err(FixtureFailure::Incomplete)
        }
    }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

fn parse_header(bytes: &[u8], limits: HttpLimits) -> Result<(HttpRequest, usize), FixtureFailure> {
    let text = std::str::from_utf8(bytes).map_err(|_| FixtureFailure::Framing)?;
    let mut lines = text
        .strip_suffix("\r\n\r\n")
        .ok_or(FixtureFailure::Framing)?
        .split("\r\n");
    let mut start = lines.next().ok_or(FixtureFailure::Framing)?.split(' ');
    let method = start.next().ok_or(FixtureFailure::Framing)?;
    let target = start.next().ok_or(FixtureFailure::Framing)?;
    if !token(method)
        || target.is_empty()
        || !target.bytes().all(|byte| byte.is_ascii_graphic())
        || start.next() != Some("HTTP/1.1")
        || start.next().is_some()
    {
        return Err(FixtureFailure::Framing);
    }
    let mut headers = Vec::new();
    let mut host = false;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(FixtureFailure::Framing)?;
        if !token(name) || !value.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
            return Err(FixtureFailure::Framing);
        }
        let name = name.to_ascii_lowercase();
        let value = value.trim_matches(' ');
        match name.as_str() {
            "host" => {
                if host || value.is_empty() {
                    return Err(FixtureFailure::Framing);
                }
                host = true;
            }
            "content-length" => {
                if length.is_some()
                    || value.is_empty()
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(FixtureFailure::Framing);
                }
                length = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| FixtureFailure::Framing)?,
                );
            }
            "transfer-encoding" => return Err(FixtureFailure::Framing),
            _ => {}
        }
        headers.push((name, value.to_owned()));
    }
    if !host || (method == "POST" && length.is_none()) {
        return Err(FixtureFailure::Framing);
    }
    let length = length.unwrap_or(0);
    if length > limits.body
        || bytes
            .len()
            .checked_add(length)
            .is_none_or(|size| size > limits.total)
    {
        return Err(FixtureFailure::Limit);
    }
    Ok((
        HttpRequest {
            method: method.to_owned(),
            target: target.to_owned(),
            headers,
            body: Vec::with_capacity(length),
        },
        length,
    ))
}
