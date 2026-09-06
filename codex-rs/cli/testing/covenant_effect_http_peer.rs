use super::FixtureFailure;
use super::HttpLimits;
use super::HttpRequest;
use super::McpExchange;
use super::connection::RequestWork;
use super::connection::serve_connection;
use std::process::ExitStatus;
use std::task::Poll;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio::time::Instant;

pub(super) enum PeerProtocol {
    Mcp { target: String },
}

#[derive(Debug)]
pub(super) struct PeerReport {
    pub(super) accepted: usize,
    pub(super) requests: Vec<HttpRequest>,
    pub(super) terminal: Result<(), FixtureFailure>,
}

struct Finish {
    terminal: Result<(), FixtureFailure>,
    deadline: Instant,
}

pub(super) struct HttpPeer {
    control: Option<oneshot::Sender<Finish>>,
    worker: JoinHandle<PeerReport>,
}

impl HttpPeer {
    pub(super) fn start(listener: TcpListener, protocol: PeerProtocol) -> Self {
        let (control, command) = oneshot::channel();
        let worker = tokio::spawn(serve(listener, protocol, command));
        Self {
            control: Some(control),
            worker,
        }
    }

    pub(super) async fn finish_after_child(
        self,
        status: ExitStatus,
        deadline: Instant,
    ) -> PeerReport {
        let terminal = if status.success() {
            Ok(())
        } else {
            Err(FixtureFailure::Child)
        };
        self.settle(Finish { terminal, deadline }).await
    }

    pub(super) async fn abort(self, deadline: Instant) -> PeerReport {
        self.settle(Finish {
            terminal: Err(FixtureFailure::Cancelled),
            deadline,
        })
        .await
    }

    async fn settle(mut self, finish: Finish) -> PeerReport {
        if let Some(control) = self.control.take() {
            // A closed receiver means the owned supervisor already found an error.
            let _ = control.send(finish);
        }
        // The handle remains in self across the await. A caller may pin and await this
        // same future after cancelling an outer borrowed wait; dropping it yields no report.
        let result = (&mut self.worker).await;
        match result {
            Ok(report) => report,
            Err(_) => panic!("owned effect fixture lost task ownership"),
        }
    }
}

impl Drop for HttpPeer {
    fn drop(&mut self) {
        // Defensive cancellation only. Drop cannot certify the accepted tasks joined.
        // Aborting an already completed supervisor after an awaited report is harmless.
        self.worker.abort();
    }
}

fn admit_connection(
    stream: TcpStream,
    limits: HttpLimits,
    requests: &mpsc::Sender<RequestWork>,
    tasks: &mut JoinSet<Result<(), FixtureFailure>>,
    accepted: &mut usize,
) -> Result<(), FixtureFailure> {
    *accepted += 1;
    if *accepted > 4 {
        return Err(FixtureFailure::Limit);
    }
    tasks.spawn(serve_connection(stream, limits, requests.clone()));
    Ok(())
}

async fn serve(
    listener: TcpListener,
    protocol: PeerProtocol,
    mut command: oneshot::Receiver<Finish>,
) -> PeerReport {
    let PeerProtocol::Mcp { target } = protocol;
    let mut exchange = McpExchange::new(target);
    let limits = HttpLimits {
        headers: 8192,
        body: 32768,
        total: 32768,
    };
    let mut listener = Some(listener);
    let (requests, mut received) = mpsc::channel::<RequestWork>(/*buffer*/ 4);
    let mut tasks = JoinSet::new();
    let mut accepted = 0;
    let mut failure = None;
    let mut deadline = None;
    loop {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            failure.get_or_insert(FixtureFailure::Deadline);
        }
        if failure.is_some() || (deadline.is_some() && tasks.is_empty()) {
            break;
        }
        tokio::select! {
            finish = &mut command, if deadline.is_none() => {
                match finish {
                    Ok(finish) => {
                        deadline = Some(finish.deadline);
                        if let Err(error) = finish.terminal {
                            failure = Some(error);
                        } else if Instant::now() >= finish.deadline {
                            failure = Some(FixtureFailure::Deadline);
                        } else if let Some(open) = &listener {
                            loop {
                                let ready = std::future::poll_fn(|cx| match open.poll_accept(cx) {
                                    Poll::Ready(value) => Poll::Ready(Some(value)),
                                    Poll::Pending => Poll::Ready(None),
                                }).await;
                                match ready {
                                    Some(Ok((stream, _))) => {
                                        if let Err(error) = admit_connection(stream, limits, &requests, &mut tasks, &mut accepted) {
                                            failure = Some(error);
                                            break;
                                        }
                                    }
                                    Some(Err(_)) => { failure = Some(FixtureFailure::Transport); break; }
                                    None => break,
                                }
                            }
                        }
                    }
                    Err(_) => failure = Some(FixtureFailure::Cancelled),
                }
                drop(listener.take());
            }
            incoming = async {
                match &listener {
                    Some(listener) => listener.accept().await,
                    None => std::future::pending().await,
                }
            }, if listener.is_some() => {
                match incoming {
                    Ok((stream, _)) => {
                        if let Err(error) = admit_connection(stream, limits, &requests, &mut tasks, &mut accepted) {
                            failure = Some(error);
                        }
                    }
                    Err(_) => failure = Some(FixtureFailure::Transport),
                }
            }
            work = received.recv(), if !tasks.is_empty() => {
                match work {
                    Some(work) => {
                        let response = exchange.respond(work.request);
                        match response {
                            Ok(response) => {
                                if work.reply.send(response).is_err() {
                                    failure = Some(FixtureFailure::Transport);
                                }
                            }
                            Err(error) => failure = Some(error),
                        }
                    }
                    None => failure = Some(FixtureFailure::Transport),
                }
            }
            result = tasks.join_next(), if !tasks.is_empty() => {
                match result {
                    Some(Ok(Ok(()))) => {}
                    Some(Ok(Err(error))) => failure = Some(error),
                    Some(Err(_)) | None => failure = Some(FixtureFailure::Transport),
                }
            }
            _ = async {
                match deadline {
                    Some(limit) => tokio::time::sleep_until(limit).await,
                    None => std::future::pending().await,
                }
            } => failure = Some(FixtureFailure::Deadline),
        }
    }
    drop(listener.take());
    received.close();
    drop(received);
    drop(requests);
    if failure.is_some() {
        tasks.abort_all();
    }
    // Never time out this join loop and return a partial report. The deadline initiated
    // cancellation; retained ownership may outlast it. An outer watchdog is only failure.
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                failure.get_or_insert(error);
            }
            Err(error) => {
                if !error.is_cancelled() {
                    failure.get_or_insert(FixtureFailure::Transport);
                }
            }
        }
    }
    let requests = exchange.requests().to_vec();
    PeerReport {
        accepted,
        requests,
        terminal: failure.map_or(Ok(()), Err),
    }
}
