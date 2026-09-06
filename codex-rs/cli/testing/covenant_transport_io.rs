//! Observes real delegated reads; it does not replace either transport parser.
use super::observation::*;
use anyhow::Result;
use anyhow::anyhow;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::task::Context;
use std::task::Poll;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::ReadBuf;

#[derive(Default)]
enum PlaintextEnd {
    #[default]
    Unobserved,
    MissingCloseNotify,
    OtherFailure,
}

pub(super) struct ConnectionObservation {
    budget: Arc<Mutex<TrafficBudget>>,
    tls: Mutex<TlsBoundary>,
    ws: Mutex<WsBoundary>,
    plaintext_end: Mutex<PlaintextEnd>,
}

impl ConnectionObservation {
    pub fn new(budget: Arc<Mutex<TrafficBudget>>) -> Self {
        Self {
            budget,
            tls: Mutex::default(),
            ws: Mutex::default(),
            plaintext_end: Mutex::default(),
        }
    }

    pub fn delivered(&self, kind: DeliveredKind) -> Result<()> {
        let mut budget = locked(&self.budget);
        locked(&self.ws).observe_delivered(kind, &mut budget)?;
        Ok(())
    }

    pub fn missing_close_notify_observed(&self) -> bool {
        matches!(
            *locked(&self.plaintext_end),
            PlaintextEnd::MissingCloseNotify
        )
    }

    pub fn qualify(&self, facts: TerminalFacts) -> Result<QualifiedEof> {
        if !self.missing_close_notify_observed() {
            return Err(anyhow!("owned TLS reader terminal unconfirmed"));
        }
        let budget = locked(&self.budget);
        let tls = locked(&self.tls);
        let ws = locked(&self.ws);
        Ok(qualify_missing_close_notify(facts, &budget, &tls, &ws)?)
    }
}

fn locked<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy)]
enum ReadLayer {
    RawTls,
    WebSocket,
}

pub(super) struct ObservedStream<S> {
    stream: S,
    prefix: Vec<u8>,
    cursor: usize,
    layer: ReadLayer,
    observation: Arc<ConnectionObservation>,
}

impl<S> ObservedStream<S> {
    pub fn raw_tls(stream: S, observation: Arc<ConnectionObservation>) -> Self {
        Self {
            stream,
            prefix: Vec::new(),
            cursor: 0,
            layer: ReadLayer::RawTls,
            observation,
        }
    }

    pub fn websocket(stream: S, prefix: Vec<u8>, observation: Arc<ConnectionObservation>) -> Self {
        Self {
            stream,
            prefix,
            cursor: 0,
            layer: ReadLayer::WebSocket,
            observation,
        }
    }

    fn record_read(&self, capacity: usize, bytes: &[u8]) -> io::Result<()> {
        let mut budget = locked(&self.observation.budget);
        let result = match self.layer {
            ReadLayer::RawTls => locked(&self.observation.tls).observe_read(
                ReadObservation {
                    capacity_before: capacity,
                    bytes,
                },
                &mut budget,
            ),
            ReadLayer::WebSocket => locked(&self.observation.ws).observe_bytes(bytes, &mut budget),
        };
        result.map_err(io::Error::other)
    }

    fn record_error(&self, kind: io::ErrorKind, operation: IoOperation) {
        match self.layer {
            ReadLayer::RawTls => {
                let mut budget = locked(&self.observation.budget);
                let _ = locked(&self.observation.tls).observe_io_error(kind, &mut budget);
            }
            ReadLayer::WebSocket => {
                let mut end = locked(&self.observation.plaintext_end);
                if kind == io::ErrorKind::UnexpectedEof && matches!(operation, IoOperation::Read) {
                    if matches!(*end, PlaintextEnd::Unobserved) {
                        *end = PlaintextEnd::MissingCloseNotify;
                    }
                } else {
                    *end = PlaintextEnd::OtherFailure;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum IoOperation {
    Read,
    Write,
}

impl<S: AsyncRead + Unpin> AsyncRead for ObservedStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.cursor < self.prefix.len() {
            // Only the already bounded HTTP upgrade is replayed, never WS bytes.
            let count = (self.prefix.len() - self.cursor).min(output.remaining());
            let end = self.cursor + count;
            output.put_slice(&self.prefix[self.cursor..end]);
            self.cursor = end;
            return Poll::Ready(Ok(()));
        }
        let capacity = output.remaining();
        let before = output.filled().len();
        match Pin::new(&mut self.stream).poll_read(cx, output) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                Poll::Ready(self.record_read(capacity, &output.filled()[before..]))
            }
            Poll::Ready(Err(error)) => {
                self.record_error(error.kind(), IoOperation::Read);
                Poll::Ready(Err(error))
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for ObservedStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.stream).poll_write(cx, bytes);
        if let Poll::Ready(Err(error)) = &result {
            self.record_error(error.kind(), IoOperation::Write);
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.stream).poll_flush(cx);
        if let Poll::Ready(Err(error)) = &result {
            self.record_error(error.kind(), IoOperation::Write);
        }
        result
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.stream).poll_shutdown(cx);
        if let Poll::Ready(Err(error)) = &result {
            self.record_error(error.kind(), IoOperation::Write);
        }
        result
    }
}

pub(super) async fn read_header(stream: &mut (impl AsyncRead + Unpin)) -> Result<Vec<u8>> {
    let mut header = Vec::new();
    while header.len() < 16 * 1024 {
        let mut byte = [0];
        stream.read_exact(&mut byte).await?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            return Ok(header);
        }
    }
    Err(anyhow!("header cap exceeded"))
}
