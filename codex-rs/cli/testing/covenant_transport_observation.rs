//! Bounded framing observations; rustls and tungstenite still validate the transport.
use std::collections::VecDeque;
use std::io::Cursor;
use std::io::ErrorKind;
use tokio_tungstenite::tungstenite::protocol::frame::FrameHeader;
use tokio_tungstenite::tungstenite::protocol::frame::coding::Control;
use tokio_tungstenite::tungstenite::protocol::frame::coding::Data;
use tokio_tungstenite::tungstenite::protocol::frame::coding::OpCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObservationError {
    ConnectionLimit,
    ReadLength,
    ReadAfterEof,
    RawIo(ErrorKind),
    TlsRecordTooLarge,
    TlsRecordLimit,
    RawByteLimit,
    InvalidFrame,
    FrameTooLarge,
    MessageTooLarge,
    FrameLimit,
    MessageLimit,
    PlaintextByteLimit,
    DeliveryMismatch,
    MissingTcpEof,
    IncompleteTlsRecord,
    IncompleteWsFrame,
    UnfinishedMessage,
    UndeliveredMessage,
    NoObservedTraffic,
    UnqualifiedTerminal,
}

impl std::fmt::Display for ObservationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("owned transport observation refused")
    }
}

impl std::error::Error for ObservationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildSettlement {
    SuccessfulAndReaped,
    FailedAndReaped,
    Unsettled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExchangeSettlement {
    Complete,
    Incomplete,
    Invalid,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DrainSettlement {
    NaturallyJoined,
    Cancelled,
    TimedOut,
    Unjoined,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TlsTerminal {
    EstablishedMissingCloseNotify,
    HandshakeEof,
    OtherFailure,
    CleanClose,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalFacts {
    pub child: ChildSettlement,
    pub exchange: ExchangeSettlement,
    pub drain: DrainSettlement,
    pub tls: TlsTerminal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QualifiedEof {
    ObservedOrderlyTcpEofWithoutCloseNotify,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeliveredKind {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

#[derive(Default)]
pub(super) struct TrafficBudget {
    connections: usize,
    records: usize,
    raw_bytes: usize,
    frames: usize,
    messages: usize,
    delivered: usize,
    plaintext_bytes: usize,
    error: Option<ObservationError>,
}

impl TrafficBudget {
    pub fn open_connection(&mut self) -> Result<(), ObservationError> {
        self.check()?;
        let result = add(
            &mut self.connections,
            1,
            4,
            ObservationError::ConnectionLimit,
        );
        self.record(result)
    }

    fn check(&self) -> Result<(), ObservationError> {
        self.error.map_or(Ok(()), Err)
    }

    fn record(&mut self, result: Result<(), ObservationError>) -> Result<(), ObservationError> {
        if let Err(error) = result {
            self.error.get_or_insert(error);
        }
        self.check()
    }
}

fn add(
    value: &mut usize,
    amount: usize,
    limit: usize,
    error: ObservationError,
) -> Result<(), ObservationError> {
    let next = value
        .checked_add(amount)
        .filter(|next| *next <= limit)
        .ok_or(error)?;
    *value = next;
    Ok(())
}

pub(super) struct ReadObservation<'a> {
    pub capacity_before: usize,
    pub bytes: &'a [u8],
}

#[derive(Default)]
pub(super) struct TlsBoundary {
    header: [u8; 5],
    used: usize,
    remaining: usize,
    records: usize,
    eof: bool,
}

impl TlsBoundary {
    pub fn observe_read(
        &mut self,
        read: ReadObservation<'_>,
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        budget.check()?;
        let result = self.read(read, budget);
        budget.record(result)
    }

    pub fn observe_io_error(
        &mut self,
        error: ErrorKind,
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        budget.record(Err(ObservationError::RawIo(error)))
    }

    fn read(
        &mut self,
        read: ReadObservation<'_>,
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        if read.bytes.len() > read.capacity_before {
            return Err(ObservationError::ReadLength);
        }
        if read.bytes.is_empty() {
            self.eof |= read.capacity_before > 0;
            return Ok(());
        }
        if self.eof {
            return Err(ObservationError::ReadAfterEof);
        }
        add(
            &mut budget.raw_bytes,
            read.bytes.len(),
            4 * 1024 * 1024,
            ObservationError::RawByteLimit,
        )?;
        let mut bytes = read.bytes;
        while !bytes.is_empty() {
            if self.remaining > 0 {
                let count = self.remaining.min(bytes.len());
                self.remaining -= count;
                bytes = &bytes[count..];
                if self.remaining == 0 {
                    add(&mut self.records, 1, 512, ObservationError::TlsRecordLimit)?;
                }
            } else {
                let count = (self.header.len() - self.used).min(bytes.len());
                self.header[self.used..self.used + count].copy_from_slice(&bytes[..count]);
                self.used += count;
                bytes = &bytes[count..];
                if self.used == self.header.len() {
                    let length = usize::from(u16::from_be_bytes([self.header[3], self.header[4]]));
                    if length >= 18_432 {
                        return Err(ObservationError::TlsRecordTooLarge);
                    }
                    add(
                        &mut budget.records,
                        1,
                        512,
                        ObservationError::TlsRecordLimit,
                    )?;
                    self.used = 0;
                    self.remaining = length;
                    if length == 0 {
                        add(&mut self.records, 1, 512, ObservationError::TlsRecordLimit)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct WsBoundary {
    header: [u8; 14],
    used: usize,
    remaining: usize,
    fragment: Option<(DeliveredKind, usize)>,
    completing: Option<DeliveredKind>,
    pending: VecDeque<DeliveredKind>,
    closed: bool,
}

impl WsBoundary {
    pub fn observe_bytes(
        &mut self,
        bytes: &[u8],
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        budget.check()?;
        let result = self.bytes(bytes, budget);
        budget.record(result)
    }

    pub fn observe_delivered(
        &mut self,
        kind: DeliveredKind,
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        budget.check()?;
        if self.pending.front() != Some(&kind) {
            return budget.record(Err(ObservationError::DeliveryMismatch));
        }
        self.pending.pop_front();
        let result = add(&mut budget.delivered, 1, 8, ObservationError::MessageLimit);
        budget.record(result)
    }

    fn bytes(
        &mut self,
        mut bytes: &[u8],
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        add(
            &mut budget.plaintext_bytes,
            bytes.len(),
            8 * 256 * 1024 + 128 * 14,
            ObservationError::PlaintextByteLimit,
        )?;
        while !bytes.is_empty() {
            if self.closed {
                return Err(ObservationError::InvalidFrame);
            }
            if self.remaining > 0 {
                let count = self.remaining.min(bytes.len());
                self.remaining -= count;
                bytes = &bytes[count..];
                if self.remaining == 0 {
                    self.complete_frame(budget)?;
                }
                continue;
            }
            if self.used == self.header.len() {
                return Err(ObservationError::InvalidFrame);
            }
            self.header[self.used] = bytes[0];
            self.used += 1;
            bytes = &bytes[1..];
            let parsed = FrameHeader::parse(&mut Cursor::new(&self.header[..self.used]))
                .map_err(|_| ObservationError::InvalidFrame)?;
            if let Some((header, length)) = parsed {
                if length > 256 * 1024 {
                    return Err(ObservationError::FrameTooLarge);
                }
                self.start_frame(header, length as usize, budget)?;
                self.used = 0;
                self.remaining = length as usize;
                if self.remaining == 0 {
                    self.complete_frame(budget)?;
                }
            }
        }
        Ok(())
    }

    fn start_frame(
        &mut self,
        header: FrameHeader,
        length: usize,
        budget: &mut TrafficBudget,
    ) -> Result<(), ObservationError> {
        if header.mask.is_none() || header.rsv1 || header.rsv2 || header.rsv3 {
            return Err(ObservationError::InvalidFrame);
        }
        add(&mut budget.frames, 1, 128, ObservationError::FrameLimit)?;
        self.completing = match header.opcode {
            OpCode::Data(Data::Text | Data::Binary) => {
                if self.fragment.is_some() {
                    return Err(ObservationError::InvalidFrame);
                }
                let kind = if header.opcode == OpCode::Data(Data::Text) {
                    DeliveredKind::Text
                } else {
                    DeliveredKind::Binary
                };
                if header.is_final {
                    Some(kind)
                } else {
                    self.fragment = Some((kind, length));
                    None
                }
            }
            OpCode::Data(Data::Continue) => {
                let (kind, mut size) = self.fragment.ok_or(ObservationError::InvalidFrame)?;
                add(
                    &mut size,
                    length,
                    256 * 1024,
                    ObservationError::MessageTooLarge,
                )?;
                if header.is_final {
                    self.fragment = None;
                    Some(kind)
                } else {
                    self.fragment = Some((kind, size));
                    None
                }
            }
            OpCode::Control(control) => {
                if !header.is_final || length > 125 {
                    return Err(ObservationError::InvalidFrame);
                }
                match control {
                    Control::Close => {
                        if self.fragment.is_some() || length == 1 {
                            return Err(ObservationError::InvalidFrame);
                        }
                        Some(DeliveredKind::Close)
                    }
                    Control::Ping => Some(DeliveredKind::Ping),
                    Control::Pong => Some(DeliveredKind::Pong),
                    Control::Reserved(_) => return Err(ObservationError::InvalidFrame),
                }
            }
            OpCode::Data(Data::Reserved(_)) => return Err(ObservationError::InvalidFrame),
        };
        Ok(())
    }

    fn complete_frame(&mut self, budget: &mut TrafficBudget) -> Result<(), ObservationError> {
        if let Some(kind) = self.completing.take() {
            add(&mut budget.messages, 1, 8, ObservationError::MessageLimit)?;
            self.pending.push_back(kind);
            self.closed = kind == DeliveredKind::Close;
        }
        Ok(())
    }
}

pub(super) fn qualify_missing_close_notify(
    facts: TerminalFacts,
    budget: &TrafficBudget,
    tls: &TlsBoundary,
    ws: &WsBoundary,
) -> Result<QualifiedEof, ObservationError> {
    budget.check()?;
    if facts
        != (TerminalFacts {
            child: ChildSettlement::SuccessfulAndReaped,
            exchange: ExchangeSettlement::Complete,
            drain: DrainSettlement::NaturallyJoined,
            tls: TlsTerminal::EstablishedMissingCloseNotify,
        })
    {
        return Err(ObservationError::UnqualifiedTerminal);
    }
    if !tls.eof {
        return Err(ObservationError::MissingTcpEof);
    }
    if tls.used != 0 || tls.remaining != 0 {
        return Err(ObservationError::IncompleteTlsRecord);
    }
    if ws.used != 0 || ws.remaining != 0 {
        return Err(ObservationError::IncompleteWsFrame);
    }
    if ws.fragment.is_some() {
        return Err(ObservationError::UnfinishedMessage);
    }
    if !ws.pending.is_empty() {
        return Err(ObservationError::UndeliveredMessage);
    }
    if tls.records == 0 || budget.delivered == 0 {
        return Err(ObservationError::NoObservedTraffic);
    }
    Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
}

#[cfg(test)]
#[path = "covenant_transport_observation_tests.rs"]
mod tests;
