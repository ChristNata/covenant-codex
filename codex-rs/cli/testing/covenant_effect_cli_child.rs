use std::fmt;
use std::process::ExitStatus;
use std::process::Stdio;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Child;
use tokio::process::ChildStdin;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task::JoinError;
use tokio::task::JoinSet;
use tokio::time::Instant;

const IO_LIMIT: usize = 65_536;

pub(super) enum ChildInput {
    Null,
    Bytes(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChildStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChildFailure {
    InputLimit,
    Spawn,
    Pipes,
    Io,
    OutputLimit(ChildStream),
    Deadline,
    Cancelled,
    Kill,
    Wait,
}

impl fmt::Display for ChildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("owned child refused")
    }
}

impl std::error::Error for ChildFailure {}

#[derive(Debug)]
pub(super) struct ChildReport {
    pub(super) pid: u32,
    pub(super) exit: Option<ExitStatus>,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) terminal: Result<(), ChildFailure>,
    pub(super) cleanup: Result<(), ChildFailure>,
}

enum PipeOutcome {
    Input(Result<(), ChildFailure>),
    Output(ChildStream, Vec<u8>, Result<(), ChildFailure>),
}

impl ChildReport {
    fn record(&mut self, joined: Result<PipeOutcome, JoinError>) {
        let result = match joined {
            Ok(PipeOutcome::Input(result)) => result,
            Ok(PipeOutcome::Output(ChildStream::Stdout, bytes, result)) => {
                self.stdout = bytes;
                result
            }
            Ok(PipeOutcome::Output(ChildStream::Stderr, bytes, result)) => {
                self.stderr = bytes;
                result
            }
            Err(_) => Err(ChildFailure::Io),
        };
        if self.terminal.is_ok() {
            self.terminal = result;
        }
    }
}

pub(super) struct OwnedChild {
    child: Child,
    pid: u32,
    input: ChildInput,
}

impl OwnedChild {
    pub(super) fn spawn(mut command: Command, input: ChildInput) -> Result<Self, ChildFailure> {
        if let ChildInput::Bytes(bytes) = &input
            && bytes.len() > IO_LIMIT
        {
            return Err(ChildFailure::InputLimit);
        }
        command
            .stdin(match &input {
                ChildInput::Null => Stdio::null(),
                ChildInput::Bytes(_) => Stdio::piped(),
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = command.spawn().map_err(|_| ChildFailure::Spawn)?;
        // Tokio returns Some until the newly spawned child is polled to completion.
        let pid = match child.id() {
            Some(pid) => pid,
            None => panic!("owned child lost native identity before wait"),
        };
        Ok(Self { child, pid, input })
    }

    pub(super) fn id(&self) -> u32 {
        self.pid
    }

    pub(super) async fn finish(
        mut self,
        deadline: Instant,
        mut cancellation: oneshot::Receiver<()>,
    ) -> ChildReport {
        let mut report = ChildReport {
            pid: self.pid,
            exit: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            terminal: Ok(()),
            cleanup: Ok(()),
        };
        // The same finish future retains self and every spawned I/O task.
        let mut pipes = JoinSet::new();
        match self.child.stdout.take() {
            Some(stdout) => {
                pipes.spawn(read_output(stdout, ChildStream::Stdout));
            }
            None => report.terminal = Err(ChildFailure::Pipes),
        }
        match self.child.stderr.take() {
            Some(stderr) => {
                pipes.spawn(read_output(stderr, ChildStream::Stderr));
            }
            None => report.terminal = Err(ChildFailure::Pipes),
        }
        // Take stdin before child.wait, which would otherwise close it itself.
        let stdin = self.child.stdin.take();
        match (std::mem::replace(&mut self.input, ChildInput::Null), stdin) {
            (ChildInput::Null, stdin) => drop(stdin),
            (ChildInput::Bytes(bytes), Some(stdin)) => {
                pipes.spawn(write_input(stdin, bytes));
            }
            (ChildInput::Bytes(_), None) => report.terminal = Err(ChildFailure::Pipes),
        }
        while report.terminal.is_ok() && (report.exit.is_none() || !pipes.is_empty()) {
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(deadline) => report.terminal = Err(ChildFailure::Deadline),
                _ = &mut cancellation => report.terminal = Err(ChildFailure::Cancelled),
                exit = self.child.wait(), if report.exit.is_none() => {
                    match exit {
                        Ok(exit) => report.exit = Some(exit),
                        Err(_) => report.terminal = Err(ChildFailure::Wait),
                    }
                }
                joined = pipes.join_next(), if !pipes.is_empty() => {
                    if let Some(joined) = joined { report.record(joined); }
                }
            }
        }
        if report.terminal.is_ok() {
            // Admit completed observations only after the deadline/cancel check.
            if Instant::now() >= deadline {
                report.terminal = Err(ChildFailure::Deadline);
            } else {
                match cancellation.try_recv() {
                    Ok(()) | Err(oneshot::error::TryRecvError::Closed) => {
                        report.terminal = Err(ChildFailure::Cancelled);
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                }
            }
        }
        if report.terminal.is_err() {
            let killed = self.child.start_kill().map_err(|_| ChildFailure::Kill);
            // Always await the actual wait, even when start_kill failed.
            report.cleanup = match self.child.wait().await {
                Ok(exit) => {
                    report.exit = Some(exit);
                    killed
                }
                Err(_) => Err(ChildFailure::Wait),
            };
        }
        // Kill/wait does not stand in for reader ownership. Preserve completed
        // bounded observations and await every task; inherited pipe handles may
        // delay this indefinitely. Such a delay never produces a settled report.
        while let Some(joined) = pipes.join_next().await {
            report.record(joined);
        }
        report
    }
}

// Dropping OwnedChild/finish uses Child's kill_on_drop and JoinSet's abort-on-drop
// only as fallback. No report or assertion of settled cleanup comes from Drop.

async fn write_input(mut stdin: ChildStdin, bytes: Vec<u8>) -> PipeOutcome {
    let result = match stdin.write_all(&bytes).await {
        Ok(()) => stdin.shutdown().await,
        Err(error) => Err(error),
    };
    // Returning drops the owned stdin handle, delivering EOF after all input.
    PipeOutcome::Input(result.map_err(|_| ChildFailure::Io))
}

async fn read_output(mut input: impl AsyncRead + Unpin, stream: ChildStream) -> PipeOutcome {
    let mut output = Vec::with_capacity(IO_LIMIT);
    let mut block = [0; 4096];
    loop {
        let remaining = IO_LIMIT - output.len();
        let allowed = (remaining + 1).min(block.len());
        let count = match input.read(&mut block[..allowed]).await {
            Ok(count) => count,
            Err(_) => return PipeOutcome::Output(stream, output, Err(ChildFailure::Io)),
        };
        if count == 0 {
            return PipeOutcome::Output(stream, output, Ok(()));
        }
        // Store only the admitted prefix, including the exact-cap portion of an
        // overflowing read. Never append the sentinel or grow beyond the cap.
        output.extend_from_slice(&block[..count.min(remaining)]);
        if count > remaining {
            return PipeOutcome::Output(stream, output, Err(ChildFailure::OutputLimit(stream)));
        }
    }
}
