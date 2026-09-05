//! Bounded exact reply validation against a fixed monotonic attempt deadline.
//! Completion facts are caller reports; protocol acceptance is not native authority.

use std::fmt;
use std::time::Duration;
use std::time::Instant;

const ALLOW: &[u8] = br#"{"decision":"ALLOW"}"#;

pub struct DecisionReply {
    deadline: Instant,
    matched: usize,
    refusal: Option<DecisionReplyError>,
}

pub struct ProtocolAllow {
    _private: (),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdinCompletion {
    WrittenAndClosed,
    IncompleteOrFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StdoutCompletion {
    Eof,
    IncompleteOrFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessCompletion {
    Exited { code: u32 },
    RunningOrUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationCompletion {
    NotCancelled,
    Cancelled,
}

pub struct ReplyCompletion {
    pub stdin: StdinCompletion,
    pub stdout: StdoutCompletion,
    pub process: ProcessCompletion,
    pub cancellation: CancellationCompletion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionReplyError {
    InvalidOutput,
    DeadlineExpired,
    IncompleteInput,
    IncompleteOutput,
    UnsuccessfulProcess,
    Cancelled,
}

impl fmt::Display for DecisionReplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidOutput => "invalid policy output",
            Self::DeadlineExpired => "policy deadline expired",
            Self::IncompleteInput => "policy input incomplete",
            Self::IncompleteOutput => "policy output incomplete",
            Self::UnsuccessfulProcess => "policy process unsuccessful",
            Self::Cancelled => "policy attempt cancelled",
        })
    }
}

impl std::error::Error for DecisionReplyError {}

impl DecisionReply {
    pub fn begin_attempt() -> Self {
        Self {
            deadline: Instant::now() + Duration::from_millis(/*millis*/ 1000),
            matched: 0,
            refusal: None,
        }
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn push_stdout(&mut self, chunk: &[u8]) -> Result<(), DecisionReplyError> {
        self.push_stdout_at(chunk, Instant::now())
    }

    pub fn finish(self, completion: ReplyCompletion) -> Result<ProtocolAllow, DecisionReplyError> {
        self.finish_at(completion, Instant::now())
    }

    fn push_stdout_at(
        &mut self,
        chunk: &[u8],
        observed: Instant,
    ) -> Result<(), DecisionReplyError> {
        if let Some(error) = self.refusal {
            return Err(error);
        }
        if observed >= self.deadline {
            self.refusal = Some(DecisionReplyError::DeadlineExpired);
            return Err(DecisionReplyError::DeadlineExpired);
        }
        // Check the bound before inspecting bytes or calculating the slice end.
        if chunk.len() > ALLOW.len() - self.matched
            || chunk != &ALLOW[self.matched..self.matched + chunk.len()]
        {
            self.refusal = Some(DecisionReplyError::InvalidOutput);
            return Err(DecisionReplyError::InvalidOutput);
        }
        self.matched += chunk.len();
        Ok(())
    }

    fn finish_at(
        self,
        completion: ReplyCompletion,
        observed: Instant,
    ) -> Result<ProtocolAllow, DecisionReplyError> {
        if let Some(error) = self.refusal {
            return Err(error);
        }
        if completion.cancellation == CancellationCompletion::Cancelled {
            return Err(DecisionReplyError::Cancelled);
        }
        if observed >= self.deadline {
            return Err(DecisionReplyError::DeadlineExpired);
        }
        if completion.stdin != StdinCompletion::WrittenAndClosed {
            return Err(DecisionReplyError::IncompleteInput);
        }
        if completion.stdout != StdoutCompletion::Eof {
            return Err(DecisionReplyError::IncompleteOutput);
        }
        if completion.process != (ProcessCompletion::Exited { code: 0 }) {
            return Err(DecisionReplyError::UnsuccessfulProcess);
        }
        if self.matched != ALLOW.len() {
            return Err(DecisionReplyError::InvalidOutput);
        }
        Ok(ProtocolAllow { _private: () })
    }
}

#[cfg(test)]
#[path = "decision_reply_tests.rs"]
mod tests;
