//! Bounded child ownership and barriers for mutation-race tests.

use super::mutation_race_fixture as fixture;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR;
use fixture::Event;
use fixture::Fixture;
use fixture::Outcome;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

pub(super) struct Process {
    child: OwnedChild,
    events: mpsc::Receiver<Event>,
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_some() || self.0.kill().is_err() {
            return;
        }
        let deadline = Instant::now() + fixture::DEADLINE;
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(std::time::Duration::from_millis(/*millis*/ 5));
        }
    }
}

impl Process {
    pub(super) fn spawn(test_name: &str, fixture: &Fixture) -> Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(["--exact", test_name, "--nocapture"])
            .current_dir(&fixture.root)
            .env_clear()
            .env(super::mutation_race_fixture::CHILD, test_name)
            .env("CODEX_HOME", fixture.root.join("mutable"))
            .env("CODEX_AUTH_HOME", fixture.root.join("auth"))
            .env(
                REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR,
                &fixture.refresh_endpoint,
            )
            .env(fixture::AGENT_ENDPOINT, &fixture.agent_endpoint)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for name in [
            "TEMP",
            "TMP",
            "USERPROFILE",
            "HOME",
            "LOCALAPPDATA",
            "APPDATA",
        ] {
            command.env(name, &fixture.root);
        }
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let (sender, events) = mpsc::channel();
        let mut process = Self {
            child: OwnedChild(command.spawn()?),
            events,
        };
        let stdout = process
            .child
            .0
            .stdout
            .take()
            .context("child stdout unavailable")?;
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout.take(/*limit*/ 131_073));
            for line in (&mut reader).lines() {
                let Ok(line) = line else { break };
                if let Some(payload) = line.strip_prefix(fixture::PREFIX)
                    && let Ok(event) = serde_json::from_str(payload)
                    && sender.send(event).is_err()
                {
                    return;
                }
            }
            let event = if reader.get_ref().limit() == 0 {
                Event::OutputLimitExceeded
            } else {
                Event::Closed
            };
            let _ = sender.send(event);
        });
        let encoded = serde_json::to_vec(fixture)?;
        ensure!(
            encoded.len() <= fixture::MAX_CHILD_INPUT_BYTES,
            "oversized bounded child fixture"
        );
        let stdin = process
            .child
            .0
            .stdin
            .as_mut()
            .context("child stdin unavailable")?;
        stdin.write_all(&encoded)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        process.wait_for(|event| matches!(event, Event::Ready))?;
        Ok(process)
    }

    fn wait_for(&self, predicate: impl Fn(&Event) -> bool) -> Result<Event> {
        loop {
            let event = self.events.recv_timeout(fixture::DEADLINE)?;
            ensure!(
                !matches!(event, Event::OutputLimitExceeded),
                "bounded child stdout limit exceeded"
            );
            ensure!(
                !matches!(event, Event::Closed),
                "bounded child closed early"
            );
            if predicate(&event) {
                return Ok(event);
            }
        }
    }

    pub(super) fn release(&mut self) -> Result<()> {
        let stdin = self
            .child
            .0
            .stdin
            .as_mut()
            .context("child stdin unavailable")?;
        stdin.write_all(b"go\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub(super) fn wait_entered(&self) -> Result<()> {
        self.wait_for(|event| matches!(event, Event::Entered))?;
        Ok(())
    }

    pub(super) fn wait_done(&self) -> Result<fixture::Report> {
        match self.wait_for(|event| matches!(event, Event::Done(_)))? {
            Event::Done(report) => Ok(report),
            Event::Ready
            | Event::Entered
            | Event::Phase(_)
            | Event::OutputLimitExceeded
            | Event::Closed => {
                unreachable!()
            }
        }
    }

    pub(super) fn wait_phase(&self) -> Result<fixture::Report> {
        match self.wait_for(|event| matches!(event, Event::Phase(_)))? {
            Event::Phase(report) => Ok(report),
            Event::Ready
            | Event::Entered
            | Event::Done(_)
            | Event::OutputLimitExceeded
            | Event::Closed => unreachable!(),
        }
    }

    pub(super) fn finish(mut self) -> Result<()> {
        let deadline = Instant::now() + fixture::DEADLINE;
        let mut status = None;
        let mut stdout_closed = false;
        loop {
            if status.is_none() {
                status = self.child.0.try_wait()?;
            }
            match self.events.try_recv() {
                Ok(Event::OutputLimitExceeded) => {
                    anyhow::bail!("bounded child stdout limit exceeded")
                }
                Ok(Event::Closed) => stdout_closed = true,
                Ok(Event::Ready | Event::Entered | Event::Phase(_) | Event::Done(_)) => {
                    anyhow::bail!("unexpected terminal child event")
                }
                Err(mpsc::TryRecvError::Disconnected) if !stdout_closed => {
                    anyhow::bail!("bounded child event channel closed")
                }
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => {}
            }
            if let Some(status) = status
                && stdout_closed
            {
                ensure!(status.success(), "bounded child failed");
                return Ok(());
            }
            if Instant::now() >= deadline {
                anyhow::bail!("bounded child completion timed out");
            }
            thread::sleep(std::time::Duration::from_millis(/*millis*/ 5));
        }
    }
}

pub(super) fn run_writer(test_name: &str, fixture: Fixture, expected: Outcome) -> Result<()> {
    let mut process = Process::spawn(test_name, &fixture)?;
    process.release()?;
    process.wait_entered()?;
    ensure!(
        process.wait_done()?.passed(expected),
        "public writer produced an unexpected bounded result"
    );
    process.finish()
}
