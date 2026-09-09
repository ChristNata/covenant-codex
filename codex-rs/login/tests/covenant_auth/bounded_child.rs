//! Bounded child-process ownership for Covenant login integration fixtures.

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::ensure;
use std::io::Read;
use std::path::Path;
use std::process::Child;
use std::process::ChildStdin;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

const POLL_INTERVAL: Duration = Duration::from_millis(/*millis*/ 5);
const READER_CLOSURE_DEADLINE: Duration = Duration::from_secs(/*secs*/ 5);
const TIMEOUT_CLEANUP_DEADLINE: Duration = Duration::from_secs(/*secs*/ 5);
const DROP_CLEANUP_DEADLINE: Duration = Duration::from_secs(/*secs*/ 1);

type ReaderResult = Result<(Vec<u8>, usize)>;
type ReaderHandle = JoinHandle<ReaderResult>;

pub(super) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

pub(super) fn isolated_test_command(
    test_name: &str,
    child_selector: &str,
    root: &Path,
    auth_home: &Path,
) -> Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test_name, "--nocapture"])
        .current_dir(root)
        .env_clear()
        .env(child_selector, test_name)
        .env("CODEX_HOME", root.join("mutable"))
        .env("CODEX_AUTH_HOME", auth_home)
        .env("TEMP", root)
        .env("TMP", root)
        .env("USERPROFILE", root)
        .env("HOME", root)
        .env("LOCALAPPDATA", root)
        .env("APPDATA", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    Ok(command)
}

#[derive(Debug)]
pub(super) enum Settlement {
    Exited(ExitStatus),
    Killed(ExitStatus),
}

pub(super) struct CapturedOutput {
    pub(super) settlement: Settlement,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) readiness_count: usize,
}

pub(super) struct BoundedChild {
    child: Child,
    stdout: Reader,
    stderr: Reader,
    readiness: mpsc::Receiver<()>,
    settled: bool,
}

impl BoundedChild {
    pub(super) fn capture(
        mut child: Child,
        output_limit: usize,
        readiness_line: Option<&str>,
    ) -> Result<Self> {
        let stdout = child.stdout.take().context("child stdout unavailable")?;
        let stderr = child.stderr.take().context("child stderr unavailable")?;
        let (readiness_tx, readiness) = mpsc::sync_channel(/*bound*/ 1);
        let stdout_result = spawn_reader(
            stdout,
            output_limit,
            readiness_line.map(str::as_bytes).map(<[u8]>::to_vec),
            Some(readiness_tx),
        );
        Ok(Self {
            child,
            stdout: stdout_result,
            stderr: spawn_reader(stderr, output_limit, None, None),
            readiness,
            settled: false,
        })
    }

    pub(super) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.stdin.take()
    }

    pub(super) fn wait_for_readiness(&mut self, deadline: Duration) -> Result<bool> {
        self.readiness
            .recv_timeout(deadline)
            .context("child readiness was not observed before its deadline")?;
        Ok(self.child.try_wait()?.is_none())
    }

    pub(super) fn wait_for_exit(&mut self, deadline: Duration) -> Result<CapturedOutput> {
        if let Some(status) = self.poll_for_exit(deadline)? {
            return self.finish(Ok(Settlement::Exited(status)));
        }
        let settlement = self.terminate(TIMEOUT_CLEANUP_DEADLINE);
        match self.finish(settlement) {
            Ok(output) => Err(anyhow!(
                "child exceeded its exit deadline; cleanup={:?}, stdout={} bytes, stderr={} bytes, readiness={}",
                output.settlement,
                output.stdout.len(),
                output.stderr.len(),
                output.readiness_count
            )),
            Err(error) => Err(error).context("child exceeded its exit deadline and cleanup failed"),
        }
    }

    pub(super) fn kill_and_reap(&mut self, deadline: Duration) -> Result<CapturedOutput> {
        let settlement = self.terminate(deadline);
        self.finish(settlement)
    }

    fn terminate(&mut self, deadline: Duration) -> Result<Settlement> {
        if let Some(status) = self.child.try_wait()? {
            return Ok(Settlement::Exited(status));
        }
        if let Err(kill_error) = self.child.kill() {
            return match self.child.try_wait()? {
                Some(status) => Ok(Settlement::Exited(status)),
                None => Err(kill_error).context("failed to kill live child"),
            };
        }
        self.poll_for_exit(deadline)?
            .map(Settlement::Killed)
            .context("killed child was not reaped before its deadline")
    }

    fn poll_for_exit(&mut self, deadline: Duration) -> Result<Option<ExitStatus>> {
        let deadline = Instant::now() + deadline;
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(Some(status));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            thread::sleep(POLL_INTERVAL.min(remaining));
        }
    }

    fn finish(&mut self, settlement: Result<Settlement>) -> Result<CapturedOutput> {
        let deadline = Instant::now() + READER_CLOSURE_DEADLINE;
        let stdout = self.stdout.finish_before(deadline, "stdout reader");
        let stderr = self.stderr.finish_before(deadline, "stderr reader");
        match (settlement, stdout, stderr) {
            (Ok(settlement), Ok((stdout, readiness_count)), Ok((stderr, _))) => {
                self.settled = true;
                Ok(CapturedOutput {
                    settlement,
                    stdout,
                    stderr,
                    readiness_count,
                })
            }
            (settlement, stdout, stderr) => Err(anyhow!(
                "bounded child settlement failed: process={}; stdout={}; stderr={}",
                result_state(&settlement),
                result_state(&stdout),
                result_state(&stderr)
            )),
        }
    }
}

impl Drop for BoundedChild {
    fn drop(&mut self) {
        if !self.settled {
            let settlement = match self.child.try_wait() {
                Ok(Some(status)) => Ok(Settlement::Exited(status)),
                Ok(None) => self.terminate(DROP_CLEANUP_DEADLINE),
                Err(error) => Err(error.into()),
            };
            let _ = self.finish(settlement);
        }
    }
}

struct Reader {
    completion: mpsc::Receiver<()>,
    completion_observed: bool,
    handle: Option<ReaderHandle>,
}

impl Reader {
    fn finish_before(&mut self, deadline: Instant, name: &str) -> Result<(Vec<u8>, usize)> {
        if !self.completion_observed {
            self.completion
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .with_context(|| format!("{name} did not signal bounded completion"))?;
            self.completion_observed = true;
        }
        let handle = self.handle.as_ref().context("reader was already settled")?;
        while !handle.is_finished() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(
                !remaining.is_zero(),
                "{name} did not finish before its deadline"
            );
            thread::sleep(POLL_INTERVAL.min(remaining));
        }
        self.handle
            .take()
            .context("reader handle missing after completion")?
            .join()
            .map_err(|_| anyhow!("{name} panicked after signaling completion"))?
    }
}

fn spawn_reader(
    input: impl Read + Send + 'static,
    limit: usize,
    readiness_line: Option<Vec<u8>>,
    readiness: Option<mpsc::SyncSender<()>>,
) -> Reader {
    let (completion_tx, completion) = mpsc::sync_channel(/*bound*/ 1);
    let handle = thread::spawn(move || {
        let result = read_stream(input, limit, readiness_line.as_deref(), readiness.as_ref());
        let _ = completion_tx.try_send(());
        result
    });
    Reader {
        completion,
        completion_observed: false,
        handle: Some(handle),
    }
}

fn read_stream(
    mut input: impl Read,
    limit: usize,
    readiness_line: Option<&[u8]>,
    readiness: Option<&mpsc::SyncSender<()>>,
) -> Result<(Vec<u8>, usize)> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut line_start = 0;
    let mut readiness_count = 0_usize;
    loop {
        let read = input.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        ensure!(bytes.len() <= limit, "child stream exceeded limit");
        while let Some(offset) = bytes[line_start..].iter().position(|byte| *byte == b'\n') {
            let line_end = line_start + offset;
            let line = bytes[line_start..line_end]
                .strip_suffix(b"\r")
                .unwrap_or(&bytes[line_start..line_end]);
            if readiness_line == Some(line) {
                readiness_count = readiness_count.saturating_add(1);
                if readiness_count == 1
                    && let Some(readiness) = readiness
                {
                    let _ = readiness.try_send(());
                }
            }
            line_start = line_end + 1;
        }
    }
    Ok((bytes, readiness_count))
}

fn result_state<T>(result: &Result<T>) -> String {
    match result {
        Ok(_) => "settled".to_string(),
        Err(error) => format!("error: {error:#}"),
    }
}
