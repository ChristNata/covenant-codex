use super::rotation_support::Fixture;
use super::rotation_support::Method;
use super::rotation_support::child_command;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthKeyringBackendKind;
use codex_login::AuthManager;
use codex_login::test_support::transport_default_auth_route_config;
use serde::Deserialize;
use serde::Serialize;
use std::future::Future;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::process::Child;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const PREFIX: &str = "COVENANT_CANCELLATION ";
const DEADLINE: Duration = Duration::from_secs(/*secs*/ 15);

#[derive(Deserialize, Serialize, PartialEq)]
enum Event {
    Ready,
    Entered { pending: bool },
    Cancelled,
    Alive,
    Finished,
}

fn emit(event: Event) -> Result<()> {
    println!("{PREFIX}{}", serde_json::to_string(&event)?);
    std::io::stdout().flush()?;
    Ok(())
}

pub(super) fn run_child() -> Result<()> {
    let mut input = BufReader::new(std::io::stdin());
    let mut encoded = String::new();
    input.read_line(&mut encoded)?;
    let fixture: Fixture = serde_json::from_str(&encoded)
        .map_err(|_| anyhow::anyhow!("invalid synthetic cancellation fixture"))?;
    let (sender, mut commands) = tokio::sync::mpsc::unbounded_channel();
    thread::spawn(move || {
        for command in input.take(/*limit*/ 4096).lines() {
            let Ok(command) = command else { break };
            if sender.send(command).is_err() {
                break;
            }
        }
    });
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let manager = AuthManager::shared(
                fixture.root.join("mutable"),
                /*enable_codex_api_key_env*/ false,
                AuthCredentialsStoreMode::File,
                /*forced_chatgpt_workspace_id*/ None,
                /*chatgpt_base_url*/ None,
                AuthKeyringBackendKind::Direct,
                transport_default_auth_route_config(),
            )
            .await;
            ensure!(
                manager.auth_cached().is_some_and(|auth| {
                    auth.is_chatgpt_auth()
                        && auth.get_token_data().ok().as_ref() == fixture.initial.tokens.as_ref()
                }),
                "cancellation caller did not cache the starting generation"
            );
            emit(Event::Ready)?;
            ensure!(
                commands.recv().await.as_deref() == Some("go"),
                "missing start command"
            );
            let caller = tokio::spawn(async move {
                let mut refresh = Box::pin(async {
                    match fixture.method {
                        Method::Guarded => manager.refresh_token().await,
                        Method::Authority => manager.refresh_token_from_authority().await,
                        Method::Probe => unreachable!("probe cannot be cancelled"),
                    }
                });
                let mut entered = false;
                std::future::poll_fn(|context| {
                    let result = refresh.as_mut().poll(context);
                    if !entered {
                        entered = true;
                        emit(Event::Entered {
                            pending: result.is_pending(),
                        })
                        .expect("cancellation first-poll report failed");
                    }
                    result
                })
                .await
            });
            ensure!(
                commands.recv().await.as_deref() == Some("cancel"),
                "missing cancel command"
            );
            caller.abort();
            ensure!(
                caller.await.is_err_and(|error| error.is_cancelled()),
                "public refresh task was not cancelled"
            );
            emit(Event::Cancelled)?;
            // Keep this same runtime polling detached production work until the parent finishes.
            loop {
                match commands.recv().await.as_deref() {
                    Some("ping") => {
                        tokio::task::yield_now().await;
                        emit(Event::Alive)?;
                    }
                    Some("finish") => return emit(Event::Finished),
                    _ => anyhow::bail!("unexpected live-runtime command"),
                }
            }
        })
}

pub(super) struct CancelledCaller {
    child: Child,
    events: mpsc::Receiver<Event>,
}

impl CancelledCaller {
    pub fn spawn(test_name: &str, fixture: &Fixture, endpoint: &str) -> Result<Self> {
        let mut child = child_command(test_name, &fixture.root, endpoint)?.spawn()?;
        let stdout = child
            .stdout
            .take()
            .context("cancellation stdout unavailable")?;
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout.take(/*limit*/ 131_072)).lines() {
                let Ok(line) = line else { break };
                if let Some((_, payload)) = line.split_once(PREFIX)
                    && let Ok(event) = serde_json::from_str(payload)
                    && sender.send(event).is_err()
                {
                    break;
                }
            }
        });
        let mut caller = Self { child, events };
        let input = caller
            .child
            .stdin
            .as_mut()
            .context("cancellation input unavailable")?;
        input.write_all(&serde_json::to_vec(fixture)?)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(caller)
    }

    fn expect(&mut self, expected: Event) -> Result<()> {
        let event = self
            .events
            .recv_timeout(DEADLINE)
            .context("cancellation child exited or missed its barrier")?;
        ensure!(event == expected, "unexpected cancellation child event");
        Ok(())
    }

    fn send(&mut self, command: &[u8]) -> Result<()> {
        let input = self
            .child
            .stdin
            .as_mut()
            .context("cancellation input unavailable")?;
        input.write_all(command)?;
        input.flush()?;
        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        self.expect(Event::Ready)?;
        self.send(b"go\n")?;
        self.expect(Event::Entered { pending: true })
    }

    pub fn cancel(&mut self) -> Result<()> {
        self.send(b"cancel\n")?;
        self.expect(Event::Cancelled)
    }

    pub fn ping(&mut self) -> Result<()> {
        self.send(b"ping\n")?;
        self.expect(Event::Alive)?;
        ensure!(
            self.child.try_wait()?.is_none(),
            "cancelled caller runtime exited"
        );
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.send(b"finish\n")?;
        self.expect(Event::Finished)?;
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.child.try_wait()? {
                ensure!(status.success(), "cancelled caller process failed");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "cancelled caller did not exit");
            thread::sleep(Duration::from_millis(/*millis*/ 5));
        }
    }
}

impl Drop for CancelledCaller {
    fn drop(&mut self) {
        // Failure cleanup owns this synthetic child only; never a Cargo/Rust process.
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
