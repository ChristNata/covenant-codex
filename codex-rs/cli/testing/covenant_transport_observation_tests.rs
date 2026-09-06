//! Ignored, unregistered observation tests; opaque TLS bytes are not a TLS session.
use super::*;
use pretty_assertions::assert_eq;
use std::io::ErrorKind;
use tokio_tungstenite::tungstenite::protocol::frame::FrameHeader;
use tokio_tungstenite::tungstenite::protocol::frame::coding::Control;
use tokio_tungstenite::tungstenite::protocol::frame::coding::Data;
use tokio_tungstenite::tungstenite::protocol::frame::coding::OpCode;

#[derive(Clone, Copy)]
enum Finality {
    Final,
    More,
}

fn tls_record(payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len()).unwrap().to_be_bytes();
    let mut bytes = vec![0x17, 0x03, 0x03, length[0], length[1]];
    bytes.extend_from_slice(payload);
    bytes
}

fn frame(opcode: OpCode, finality: Finality, payload: &[u8]) -> Vec<u8> {
    let mask = [0x12, 0x34, 0x56, 0x78];
    let header = FrameHeader {
        is_final: matches!(finality, Finality::Final),
        opcode,
        mask: Some(mask),
        ..FrameHeader::default()
    };
    let mut bytes = Vec::new();
    header.format(payload.len() as u64, &mut bytes).unwrap();
    bytes.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % 4]),
    );
    bytes
}

fn text_frame() -> Vec<u8> {
    frame(OpCode::Data(Data::Text), Finality::Final, b"owned")
}

fn facts() -> TerminalFacts {
    TerminalFacts {
        child: ChildSettlement::SuccessfulAndReaped,
        exchange: ExchangeSettlement::Complete,
        drain: DrainSettlement::NaturallyJoined,
        tls: TlsTerminal::EstablishedMissingCloseNotify,
    }
}

struct Observed {
    budget: TrafficBudget,
    tls: TlsBoundary,
    ws: WsBoundary,
}

impl Observed {
    fn new() -> Self {
        let mut budget = TrafficBudget::default();
        budget.open_connection().unwrap();
        Self {
            budget,
            tls: TlsBoundary::default(),
            ws: WsBoundary::default(),
        }
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<(), ObservationError> {
        self.tls.observe_read(
            ReadObservation {
                capacity_before: bytes.len().max(1),
                bytes,
            },
            &mut self.budget,
        )
    }

    fn complete_tls(&mut self) {
        self.raw(&tls_record(b"opaque framing fixture")).unwrap();
        self.raw(&[]).unwrap();
    }

    fn complete_ws(&mut self) {
        self.ws
            .observe_bytes(&text_frame(), &mut self.budget)
            .unwrap();
        self.ws
            .observe_delivered(DeliveredKind::Text, &mut self.budget)
            .unwrap();
    }

    fn qualify(&self) -> Result<QualifiedEof, ObservationError> {
        qualify_missing_close_notify(facts(), &self.budget, &self.tls, &self.ws)
    }
}

#[test]
fn complete_observed_records_and_delivered_message_qualify_orderly_eof() {
    let mut observed = Observed::new();
    observed.complete_tls();
    observed.complete_ws();
    assert_eq!(
        observed.qualify(),
        Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
    );
}

#[test]
fn every_tls_split_preserves_complete_record_and_real_eof() {
    let record = tls_record(b"opaque body");
    for split in 1..record.len() {
        let mut observed = Observed::new();
        observed.raw(&record[..split]).unwrap();
        observed.raw(&record[split..]).unwrap();
        observed.raw(&[]).unwrap();
        observed.complete_ws();
        assert_eq!(
            observed.qualify(),
            Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
        );
    }
}

#[test]
fn eof_refuses_every_partial_tls_header_and_payload() {
    let record = tls_record(b"opaque body");
    for end in 1..record.len() {
        let mut observed = Observed::new();
        observed.raw(&record[..end]).unwrap();
        observed.raw(&[]).unwrap();
        observed.complete_ws();
        assert_eq!(
            observed.qualify(),
            Err(ObservationError::IncompleteTlsRecord)
        );
    }
}

#[test]
fn zero_capacity_read_is_not_eof_but_positive_capacity_read_zero_is() {
    let mut observed = Observed::new();
    observed.raw(&tls_record(b"record")).unwrap();
    observed.complete_ws();
    observed
        .tls
        .observe_read(
            ReadObservation {
                capacity_before: 0,
                bytes: &[],
            },
            &mut observed.budget,
        )
        .unwrap();
    assert_eq!(observed.qualify(), Err(ObservationError::MissingTcpEof));
    observed.raw(&[]).unwrap();
    assert_eq!(
        observed.qualify(),
        Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
    );
}

#[test]
fn impossible_read_length_and_data_after_eof_are_sticky_failures() {
    let mut impossible = Observed::new();
    assert_eq!(
        impossible.tls.observe_read(
            ReadObservation {
                capacity_before: 0,
                bytes: &[0x17]
            },
            &mut impossible.budget
        ),
        Err(ObservationError::ReadLength)
    );
    assert_eq!(impossible.raw(&[]), Err(ObservationError::ReadLength));
    let mut after_eof = Observed::new();
    after_eof.complete_tls();
    after_eof.complete_ws();
    assert_eq!(after_eof.raw(&[0x17]), Err(ObservationError::ReadAfterEof));
    assert_eq!(after_eof.qualify(), Err(ObservationError::ReadAfterEof));
}

#[test]
fn raw_reset_or_unexpected_eof_never_becomes_qualified_tls_eof() {
    for kind in [
        ErrorKind::ConnectionReset,
        ErrorKind::UnexpectedEof,
        ErrorKind::BrokenPipe,
    ] {
        let mut observed = Observed::new();
        observed.raw(&tls_record(b"record")).unwrap();
        observed.complete_ws();
        assert_eq!(
            observed.tls.observe_io_error(kind, &mut observed.budget),
            Err(ObservationError::RawIo(kind))
        );
        assert_eq!(observed.raw(&[]), Err(ObservationError::RawIo(kind)));
        assert_eq!(observed.qualify(), Err(ObservationError::RawIo(kind)));
    }
}

#[test]
fn every_websocket_split_preserves_complete_message_delivery() {
    let bytes = text_frame();
    for split in 1..bytes.len() {
        let mut observed = Observed::new();
        observed.complete_tls();
        observed
            .ws
            .observe_bytes(&bytes[..split], &mut observed.budget)
            .unwrap();
        observed
            .ws
            .observe_bytes(&bytes[split..], &mut observed.budget)
            .unwrap();
        observed
            .ws
            .observe_delivered(DeliveredKind::Text, &mut observed.budget)
            .unwrap();
        assert_eq!(
            observed.qualify(),
            Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
        );
    }
}

#[test]
fn partial_websocket_header_or_payload_refuses_terminal_qualification() {
    let bytes = text_frame();
    for end in 1..bytes.len() {
        let mut observed = Observed::new();
        observed.complete_tls();
        observed
            .ws
            .observe_bytes(&bytes[..end], &mut observed.budget)
            .unwrap();
        assert_eq!(observed.qualify(), Err(ObservationError::IncompleteWsFrame));
    }
}

#[test]
fn fragment_requires_final_continuation_and_preserves_interleaved_ping_order() {
    let mut observed = Observed::new();
    observed.complete_tls();
    for bytes in [
        frame(OpCode::Data(Data::Text), Finality::More, b"part"),
        frame(OpCode::Control(Control::Ping), Finality::Final, b"ping"),
    ] {
        observed
            .ws
            .observe_bytes(&bytes, &mut observed.budget)
            .unwrap();
    }
    observed
        .ws
        .observe_delivered(DeliveredKind::Ping, &mut observed.budget)
        .unwrap();
    assert_eq!(observed.qualify(), Err(ObservationError::UnfinishedMessage));
    observed
        .ws
        .observe_bytes(
            &frame(OpCode::Data(Data::Continue), Finality::Final, b"done"),
            &mut observed.budget,
        )
        .unwrap();
    observed
        .ws
        .observe_delivered(DeliveredKind::Text, &mut observed.budget)
        .unwrap();
    assert_eq!(
        observed.qualify(),
        Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
    );
}

#[test]
fn close_mid_fragment_and_continuation_without_start_remain_errors() {
    let mut fragmented = Observed::new();
    fragmented
        .ws
        .observe_bytes(
            &frame(OpCode::Data(Data::Text), Finality::More, b"part"),
            &mut fragmented.budget,
        )
        .unwrap();
    assert_eq!(
        fragmented.ws.observe_bytes(
            &frame(OpCode::Control(Control::Close), Finality::Final, &[]),
            &mut fragmented.budget
        ),
        Err(ObservationError::InvalidFrame)
    );
    assert_eq!(fragmented.qualify(), Err(ObservationError::InvalidFrame));
    let mut orphan = Observed::new();
    assert_eq!(
        orphan.ws.observe_bytes(
            &frame(OpCode::Data(Data::Continue), Finality::Final, b"tail"),
            &mut orphan.budget
        ),
        Err(ObservationError::InvalidFrame)
    );
}

#[test]
fn read_ahead_complete_or_partial_extra_frame_cannot_disappear() {
    let mut complete = Observed::new();
    complete.complete_tls();
    complete
        .ws
        .observe_bytes(&[text_frame(), text_frame()].concat(), &mut complete.budget)
        .unwrap();
    complete
        .ws
        .observe_delivered(DeliveredKind::Text, &mut complete.budget)
        .unwrap();
    assert_eq!(
        complete.qualify(),
        Err(ObservationError::UndeliveredMessage)
    );
    let mut partial = Observed::new();
    partial.complete_tls();
    partial
        .ws
        .observe_bytes(&[text_frame(), vec![0x81]].concat(), &mut partial.budget)
        .unwrap();
    partial
        .ws
        .observe_delivered(DeliveredKind::Text, &mut partial.budget)
        .unwrap();
    assert_eq!(partial.qualify(), Err(ObservationError::IncompleteWsFrame));
}

#[test]
fn delivery_must_match_observed_kind_and_close_must_be_last() {
    let mut mismatch = Observed::new();
    mismatch
        .ws
        .observe_bytes(&text_frame(), &mut mismatch.budget)
        .unwrap();
    assert_eq!(
        mismatch
            .ws
            .observe_delivered(DeliveredKind::Pong, &mut mismatch.budget),
        Err(ObservationError::DeliveryMismatch)
    );
    assert_eq!(mismatch.qualify(), Err(ObservationError::DeliveryMismatch));
    let mut closed = Observed::new();
    closed
        .ws
        .observe_bytes(
            &frame(OpCode::Control(Control::Close), Finality::Final, &[]),
            &mut closed.budget,
        )
        .unwrap();
    closed
        .ws
        .observe_delivered(DeliveredKind::Close, &mut closed.budget)
        .unwrap();
    assert_eq!(
        closed.ws.observe_bytes(&[0x81], &mut closed.budget),
        Err(ObservationError::InvalidFrame)
    );
}

#[test]
fn terminal_facts_cannot_substitute_for_successful_joined_exchange() {
    let mut observed = Observed::new();
    observed.complete_tls();
    observed.complete_ws();
    let invalid = [
        TerminalFacts {
            child: ChildSettlement::FailedAndReaped,
            ..facts()
        },
        TerminalFacts {
            child: ChildSettlement::Unsettled,
            ..facts()
        },
        TerminalFacts {
            exchange: ExchangeSettlement::Incomplete,
            ..facts()
        },
        TerminalFacts {
            exchange: ExchangeSettlement::Invalid,
            ..facts()
        },
        TerminalFacts {
            drain: DrainSettlement::Cancelled,
            ..facts()
        },
        TerminalFacts {
            drain: DrainSettlement::TimedOut,
            ..facts()
        },
        TerminalFacts {
            drain: DrainSettlement::Unjoined,
            ..facts()
        },
        TerminalFacts {
            tls: TlsTerminal::HandshakeEof,
            ..facts()
        },
        TerminalFacts {
            tls: TlsTerminal::OtherFailure,
            ..facts()
        },
        TerminalFacts {
            tls: TlsTerminal::CleanClose,
            ..facts()
        },
    ];
    let results: Vec<_> = invalid
        .into_iter()
        .map(|value| {
            qualify_missing_close_notify(value, &observed.budget, &observed.tls, &observed.ws)
        })
        .collect();
    assert_eq!(
        results,
        vec![Err(ObservationError::UnqualifiedTerminal); 10]
    );
}

#[test]
fn empty_stream_cannot_qualify_from_reported_facts_alone() {
    let mut observed = Observed::new();
    observed.raw(&[]).unwrap();
    assert_eq!(observed.qualify(), Err(ObservationError::NoObservedTraffic));
}

#[test]
fn advertised_tls_and_frame_lengths_fail_before_payload_collection() {
    let mut tls = Observed::new();
    assert_eq!(
        tls.raw(&[0x17, 3, 3, 0x48, 0]),
        Err(ObservationError::TlsRecordTooLarge)
    );
    let header = FrameHeader {
        opcode: OpCode::Data(Data::Text),
        mask: Some([1, 2, 3, 4]),
        ..FrameHeader::default()
    };
    let mut oversized = Vec::new();
    header.format(/*length*/ 262_145, &mut oversized).unwrap();
    let mut ws = Observed::new();
    assert_eq!(
        ws.ws.observe_bytes(&oversized, &mut ws.budget),
        Err(ObservationError::FrameTooLarge)
    );
}

#[test]
fn fragmented_payload_limit_is_cumulative() {
    let mut observed = Observed::new();
    let payload = vec![b'x'; 131_073];
    observed
        .ws
        .observe_bytes(
            &frame(OpCode::Data(Data::Text), Finality::More, &payload),
            &mut observed.budget,
        )
        .unwrap();
    assert_eq!(
        observed.ws.observe_bytes(
            &frame(OpCode::Data(Data::Continue), Finality::Final, &payload),
            &mut observed.budget
        ),
        Err(ObservationError::MessageTooLarge)
    );
}

#[test]
fn connection_and_record_limits_are_shared_and_sticky() {
    let mut budget = TrafficBudget::default();
    for _ in 0..4 {
        budget.open_connection().unwrap();
    }
    assert_eq!(
        budget.open_connection(),
        Err(ObservationError::ConnectionLimit)
    );
    let mut shared = TrafficBudget::default();
    let mut first = TlsBoundary::default();
    let mut second = TlsBoundary::default();
    shared.open_connection().unwrap();
    shared.open_connection().unwrap();
    let record = tls_record(b"x");
    for observer in [&mut first, &mut second] {
        for _ in 0..256 {
            observer
                .observe_read(
                    ReadObservation {
                        capacity_before: record.len(),
                        bytes: &record,
                    },
                    &mut shared,
                )
                .unwrap();
        }
    }
    assert_eq!(
        first.observe_read(
            ReadObservation {
                capacity_before: record.len(),
                bytes: &record
            },
            &mut shared
        ),
        Err(ObservationError::TlsRecordLimit)
    );
    assert_eq!(
        second.observe_read(
            ReadObservation {
                capacity_before: 1,
                bytes: &[]
            },
            &mut shared
        ),
        Err(ObservationError::TlsRecordLimit)
    );
}

#[test]
fn raw_byte_limit_refuses_new_data_even_at_record_boundary() {
    let mut observed = Observed::new();
    let record = tls_record(&vec![0x5a; 16_384]);
    for _ in 0..255 {
        observed.raw(&record).unwrap();
    }
    assert_eq!(observed.raw(&record), Err(ObservationError::RawByteLimit));
    assert_eq!(observed.raw(&[]), Err(ObservationError::RawByteLimit));
}

#[test]
fn fragmented_empty_frames_cannot_evade_the_global_frame_limit() {
    let mut shared = TrafficBudget::default();
    let mut first = WsBoundary::default();
    let mut second = WsBoundary::default();
    shared.open_connection().unwrap();
    shared.open_connection().unwrap();
    for observer in [&mut first, &mut second] {
        observer
            .observe_bytes(
                &frame(OpCode::Data(Data::Text), Finality::More, &[]),
                &mut shared,
            )
            .unwrap();
        for _ in 0..63 {
            observer
                .observe_bytes(
                    &frame(OpCode::Data(Data::Continue), Finality::More, &[]),
                    &mut shared,
                )
                .unwrap();
        }
    }
    assert_eq!(
        first.observe_bytes(
            &frame(OpCode::Data(Data::Continue), Finality::Final, &[]),
            &mut shared
        ),
        Err(ObservationError::FrameLimit)
    );
}

#[test]
fn complete_messages_share_one_budget_across_connections() {
    let mut shared = TrafficBudget::default();
    let mut first = WsBoundary::default();
    let mut second = WsBoundary::default();
    shared.open_connection().unwrap();
    shared.open_connection().unwrap();
    for observer in [&mut first, &mut second] {
        for _ in 0..4 {
            observer.observe_bytes(&text_frame(), &mut shared).unwrap();
            observer
                .observe_delivered(DeliveredKind::Text, &mut shared)
                .unwrap();
        }
    }
    assert_eq!(
        second.observe_bytes(&text_frame(), &mut shared),
        Err(ObservationError::MessageLimit)
    );
}

#[test]
fn complete_plaintext_does_not_hide_a_partial_following_tls_record() {
    let mut observed = Observed::new();
    let mut bytes = tls_record(b"first record");
    bytes.extend_from_slice(&[0x17, 3, 3, 0, 2, 0x5a]);
    observed.raw(&bytes).unwrap();
    observed.raw(&[]).unwrap();
    observed.complete_ws();
    assert_eq!(
        observed.qualify(),
        Err(ObservationError::IncompleteTlsRecord)
    );
}

#[test]
fn invalid_mask_reserved_bits_and_fragmented_control_are_refused() {
    let mut reserved = text_frame();
    reserved[0] |= 0x40;
    let invalid = [
        vec![0x81, 0],
        reserved,
        frame(OpCode::Control(Control::Ping), Finality::More, b"ping"),
    ];
    for bytes in invalid {
        let mut observed = Observed::new();
        assert_eq!(
            observed.ws.observe_bytes(&bytes, &mut observed.budget),
            Err(ObservationError::InvalidFrame)
        );
        assert_eq!(observed.qualify(), Err(ObservationError::InvalidFrame));
    }
}

#[test]
fn unused_preconnection_can_settle_after_the_shared_exchange_completed() {
    let mut active = Observed::new();
    active.complete_tls();
    active.complete_ws();
    active.budget.open_connection().unwrap();
    let mut idle_tls = TlsBoundary::default();
    let idle_ws = WsBoundary::default();
    let record = tls_record(b"owned handshake observation");
    idle_tls
        .observe_read(
            ReadObservation {
                capacity_before: record.len(),
                bytes: &record,
            },
            &mut active.budget,
        )
        .unwrap();
    idle_tls
        .observe_read(
            ReadObservation {
                capacity_before: 1,
                bytes: &[],
            },
            &mut active.budget,
        )
        .unwrap();
    assert_eq!(
        qualify_missing_close_notify(facts(), &active.budget, &idle_tls, &idle_ws),
        Ok(QualifiedEof::ObservedOrderlyTcpEofWithoutCloseNotify)
    );
}
