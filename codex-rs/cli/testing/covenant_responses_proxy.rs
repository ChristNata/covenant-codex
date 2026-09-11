//! Test-owned CONNECT/TLS/WebSocket peer; it never connects to an upstream server.
#[path = "covenant_responses_connection.rs"]
mod connection;
#[path = "covenant_transport_observation.rs"]
mod observation;
#[path = "covenant_transport_io.rs"]
mod transport;

use anyhow::Result;
use anyhow::anyhow;
use observation::ChildSettlement;
use observation::DrainSettlement;
use observation::ExchangeSettlement;
use observation::TerminalFacts;
use observation::TlsTerminal;
use observation::TrafficBudget;
use rcgen::BasicConstraints;
use rcgen::CertificateParams;
use rcgen::CertifiedIssuer;
use rcgen::ExtendedKeyUsagePurpose;
use rcgen::IsCa;
use rcgen::KeyPair;
use rcgen::KeyUsagePurpose;
use rcgen::PKCS_ECDSA_P256_SHA256;
use serde_json::Value;
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::task::Poll;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls;
use transport::ConnectionObservation;

pub(super) const API_KEY: &str = "covenant-sc5-synthetic-noncredential";
pub(super) const PROMPT: &str = "Return the owned SC5 completion marker.";
pub(super) const MARKER: &str = "COVENANT_SC5_TEXT_COMPLETE";
const WARMUP_ID: &str = "resp_covenant_owned_warmup";
const INFERENCE_ID: &str = "resp_covenant_owned_inference";
const MESSAGE_LIMIT: usize = 256 * 1024;

#[derive(Clone, Default)]
pub(super) struct Capture {
    pub connections: usize,
    pub connects: Vec<Vec<u8>>,
    pub handshakes: Vec<(String, Vec<(String, String)>)>,
    pub requests: Vec<Value>,
    pub inference_input: Vec<Value>,
    pub warmup_completed: bool,
    pub inference_completed: bool,
    pub messages: usize,
    pub qualified_eof_connections: usize,
    pub failure: Option<&'static str>,
}

pub(super) struct Proxy {
    pub address: String,
    stop: oneshot::Sender<ChildSettlement>,
    task: JoinHandle<()>,
    capture: Arc<Mutex<Capture>>,
}

#[cfg(test)]
#[path = "covenant_responses_proxy_tests.rs"]
mod tests;

impl Proxy {
    pub async fn start(ca_path: &Path) -> Result<Self> {
        // Same pinned rcgen APIs as http-client/tests/ca_env.rs:224-261.
        let mut ca_params = CertificateParams::default();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca = CertifiedIssuer::self_signed(
            ca_params,
            KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?,
        )?;
        let mut leaf_params = CertificateParams::new(vec!["api.openai.com".to_string()])?;
        leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        leaf_params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        let leaf = leaf_params.signed_by(&key, &ca)?;
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            rustls::pki_types::PrivateKeyDer::from(key),
        )?;
        std::fs::write(ca_path, ca.pem())?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = format!("http://{}", listener.local_addr()?);
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let capture = Arc::new(Mutex::new(Capture::default()));
        let (stop, receiver) = oneshot::channel();
        let task = tokio::spawn(serve(listener, acceptor, Arc::clone(&capture), receiver));
        Ok(Self {
            address,
            stop,
            task,
            capture,
        })
    }

    pub async fn finish_after_child(self, status: ExitStatus) -> Result<Capture> {
        let child = if status.success() {
            ChildSettlement::SuccessfulAndReaped
        } else {
            ChildSettlement::FailedAndReaped
        };
        self.finish_owned(child).await
    }

    pub async fn abort(self) -> Result<Capture> {
        self.finish_owned(ChildSettlement::Unsettled).await
    }

    async fn finish_owned(self, child: ChildSettlement) -> Result<Capture> {
        if self.stop.send(child).is_err() {
            captured(&self.capture)
                .failure
                .get_or_insert("owned proxy stop unconfirmed");
        }
        self.task
            .await
            .map_err(|_| anyhow!("owned proxy task failed"))?;
        Ok(captured(&self.capture).clone())
    }
}

fn captured(capture: &Mutex<Capture>) -> MutexGuard<'_, Capture> {
    capture
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

type ConnectionResult = Result<Arc<ConnectionObservation>>;

fn spawn_connection(
    stream: TcpStream,
    acceptor: &TlsAcceptor,
    capture: &Arc<Mutex<Capture>>,
    budget: &Arc<Mutex<TrafficBudget>>,
    tasks: &mut JoinSet<ConnectionResult>,
) -> Result<()> {
    {
        let mut state = captured(capture);
        state.connections = state
            .connections
            .checked_add(1)
            .ok_or_else(|| anyhow!("connection count refused"))?;
    }
    budget
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .open_connection()?;
    tasks.spawn(connection::connection(
        stream,
        acceptor.clone(),
        Arc::clone(capture),
        Arc::clone(budget),
    ));
    Ok(())
}

fn collect_connection(
    result: std::result::Result<ConnectionResult, JoinError>,
    completed: &mut Vec<Arc<ConnectionObservation>>,
    capture: &Mutex<Capture>,
) {
    match result {
        Ok(Ok(observation)) => completed.push(observation),
        Ok(Err(_)) => {
            captured(capture)
                .failure
                .get_or_insert("owned connection refused");
        }
        Err(error) if error.is_cancelled() => {}
        Err(_) => {
            captured(capture)
                .failure
                .get_or_insert("owned connection task failed");
        }
    }
}

async fn serve(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    capture: Arc<Mutex<Capture>>,
    mut stop: oneshot::Receiver<ChildSettlement>,
) {
    let budget = Arc::new(Mutex::new(TrafficBudget::default()));
    let mut tasks = JoinSet::new();
    let mut completed = Vec::new();
    let child = loop {
        tokio::select! {
            biased;
            child = &mut stop => break child.unwrap_or(ChildSettlement::Unsettled),
            accepted = listener.accept() => {
                let result = accepted.map_err(anyhow::Error::from).and_then(|stream| {
                    spawn_connection(stream.0, &acceptor, &capture, &budget, &mut tasks)
                });
                if result.is_err() {
                    captured(&capture).failure.get_or_insert("owned accept or connection cap refused");
                    break ChildSettlement::Unsettled;
                }
            }
            Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                collect_connection(result, &mut completed, &capture);
            }
        }
    };
    let mut natural =
        child == ChildSettlement::SuccessfulAndReaped && captured(&capture).failure.is_none();
    if natural {
        // The actual child has settled. Account for every accept already observable now.
        loop {
            let accepted = std::future::poll_fn(|cx| match listener.poll_accept(cx) {
                Poll::Ready(result) => Poll::Ready(Some(result)),
                Poll::Pending => Poll::Ready(None),
            })
            .await;
            let Some(accepted) = accepted else { break };
            let result = accepted.map_err(anyhow::Error::from).and_then(|stream| {
                spawn_connection(stream.0, &acceptor, &capture, &budget, &mut tasks)
            });
            if result.is_err() {
                captured(&capture)
                    .failure
                    .get_or_insert("owned pending accept refused");
                natural = false;
                break;
            }
        }
    }
    drop(listener);
    if natural {
        let drained = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(result) = tasks.join_next().await {
                collect_connection(result, &mut completed, &capture);
            }
        })
        .await;
        if drained.is_err() {
            captured(&capture)
                .failure
                .get_or_insert("owned natural drain timed out");
            natural = false;
        }
    }
    if !natural {
        captured(&capture)
            .failure
            .get_or_insert("owned capture did not naturally settle");
        tasks.abort_all();
        let joined = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(result) = tasks.join_next().await {
                collect_connection(result, &mut completed, &capture);
            }
        })
        .await;
        if joined.is_err() {
            captured(&capture)
                .failure
                .get_or_insert("owned failure cleanup unjoined");
        }
        return;
    }
    let exchange = {
        let state = captured(&capture);
        if state.failure.is_some() {
            ExchangeSettlement::Invalid
        } else if state.inference_completed {
            ExchangeSettlement::Complete
        } else {
            ExchangeSettlement::Incomplete
        }
    };
    // All accepted connection tasks have naturally returned; no cancellation is success evidence.
    for observation in completed {
        let facts = TerminalFacts {
            child,
            exchange,
            drain: DrainSettlement::NaturallyJoined,
            tls: TlsTerminal::EstablishedMissingCloseNotify,
        };
        if observation.qualify(facts).is_ok() {
            captured(&capture).qualified_eof_connections += 1;
        } else {
            captured(&capture)
                .failure
                .get_or_insert("owned terminal qualification refused");
        }
    }
}
