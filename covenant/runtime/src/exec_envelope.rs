//! One bounded policy event retaining its exact native-facing input facts.
//!
//! Lexical validation does not establish filesystem identity or launch authority.

#[path = "windows_command_line.rs"]
mod windows_command_line;

use crate::DecideV1;
use crate::FrozenWindowsEnvironment;
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::path::Prefix;

const MAX_PATH_UNITS: usize = 32_766;
const MAX_ARGV_ENTRIES: usize = 1_024;
const MAX_ARGV_UNITS: usize = 32_766;
const MAX_SANDBOX_BYTES: usize = 128;
const MAX_EVENT_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkAccess {
    Denied,
    Allowed,
}

pub struct FinalExecInput {
    pub program: PathBuf,
    pub argument_tail: Vec<OsString>,
    pub cwd: PathBuf,
    pub environment: FrozenWindowsEnvironment,
    pub sandbox: String,
    pub network: NetworkAccess,
}

pub struct ExecEvent {
    program: PathBuf,
    argv: Vec<String>,
    native_command_line: Vec<u16>,
    cwd: PathBuf,
    environment: FrozenWindowsEnvironment,
    sandbox: String,
    network: NetworkAccess,
    stdin: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecEventError;

impl fmt::Display for ExecEventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid final exec event")
    }
}

impl std::error::Error for ExecEventError {}

impl ExecEvent {
    pub fn from_final(input: FinalExecInput) -> Result<Self, ExecEventError> {
        let count = input
            .argument_tail
            .len()
            .checked_add(1)
            .filter(|count| *count <= MAX_ARGV_ENTRIES)
            .ok_or(ExecEventError)?;
        if input.sandbox.is_empty()
            || input.sandbox.len() > MAX_SANDBOX_BYTES
            || input.sandbox.contains('\0')
        {
            return Err(ExecEventError);
        }
        let (program_text, program_units) = path_text(&input.program)?;
        if input.program.file_name().is_none() || program_text.ends_with(['\\', '/']) {
            return Err(ExecEventError);
        }
        let (cwd_text, _) = path_text(&input.cwd)?;
        let mut argv_units = program_units
            .checked_add(1)
            .filter(|units| *units <= MAX_ARGV_UNITS)
            .ok_or(ExecEventError)?;
        let mut argv = Vec::with_capacity(count);
        argv.push(program_text.to_owned());
        for argument in &input.argument_tail {
            let (text, units) = bounded_text(argument, MAX_ARGV_UNITS)?;
            argv_units = argv_units
                .checked_add(units)
                .and_then(|units| units.checked_add(1))
                .filter(|units| *units <= MAX_ARGV_UNITS)
                .ok_or(ExecEventError)?;
            argv.push(text.to_owned());
        }
        let native_command_line = windows_command_line::encode(&argv)?;

        let decide = {
            let bytes = bounded_json(&ExecRequest {
                version: 1,
                kind: "exec",
                exec: ExecFacts {
                    program: program_text,
                    argv: &argv,
                    cwd: cwd_text,
                    env: input.environment.as_json(),
                    sandbox: &input.sandbox,
                    network: input.network == NetworkAccess::Allowed,
                    tty: false,
                },
            })?;
            DecideV1::decode(&bytes).map_err(|_| ExecEventError)?
        };
        // The inner encoded buffer is gone before allocating the outer one.
        let stdin = bounded_json(&Event {
            hook_event_name: "Exec",
            cwd: cwd_text,
            decide_v1: &decide,
        })?;
        Ok(Self {
            program: input.program,
            argv,
            native_command_line,
            cwd: input.cwd,
            environment: input.environment,
            sandbox: input.sandbox,
            network: input.network,
            stdin,
        })
    }

    pub fn as_stdin(&self) -> &[u8] {
        &self.stdin
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// CRT-style representation including its final NUL, bound to this event's argv.
    pub fn native_command_line(&self) -> &[u16] {
        &self.native_command_line
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn environment(&self) -> &FrozenWindowsEnvironment {
        &self.environment
    }

    pub fn sandbox(&self) -> &str {
        &self.sandbox
    }

    pub fn network(&self) -> NetworkAccess {
        self.network
    }
}

fn bounded_text(value: &OsStr, maximum_units: usize) -> Result<(&str, usize), ExecEventError> {
    let observed_limit = maximum_units.checked_add(1).ok_or(ExecEventError)?;
    let units = value.encode_wide().take(observed_limit).count();
    if units > maximum_units {
        return Err(ExecEventError);
    }
    let text = value.to_str().ok_or(ExecEventError)?;
    if text.contains('\0') {
        return Err(ExecEventError);
    }
    Ok((text, units))
}

fn path_text(path: &Path) -> Result<(&str, usize), ExecEventError> {
    let text = bounded_text(path.as_os_str(), MAX_PATH_UNITS)?;
    let mut components = path.components();
    if !matches!(
        components.next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) || !matches!(components.next(), Some(Component::RootDir))
        || components.any(|component| component == Component::ParentDir)
    {
        return Err(ExecEventError);
    }
    Ok(text)
}

#[derive(Serialize)]
struct ExecRequest<'a> {
    version: u8,
    kind: &'static str,
    exec: ExecFacts<'a>,
}

#[derive(Serialize)]
struct ExecFacts<'a> {
    program: &'a str,
    argv: &'a [String],
    cwd: &'a str,
    env: &'a BTreeMap<String, String>,
    sandbox: &'a str,
    network: bool,
    tty: bool,
}

#[derive(Serialize)]
struct Event<'a> {
    hook_event_name: &'static str,
    cwd: &'a str,
    decide_v1: &'a DecideV1,
}

struct EventBuffer(Vec<u8>);

impl Write for EventBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .len()
            .checked_add(bytes.len())
            .filter(|length| *length <= MAX_EVENT_BYTES)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn bounded_json(value: &impl Serialize) -> Result<Vec<u8>, ExecEventError> {
    let mut buffer = EventBuffer(Vec::with_capacity(MAX_EVENT_BYTES));
    serde_json::to_writer(&mut buffer, value).map_err(|_| ExecEventError)?;
    Ok(buffer.0)
}

#[cfg(test)]
#[path = "exec_envelope_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "windows_command_line_admission_tests.rs"]
mod windows_command_line_admission_tests;

#[cfg(test)]
#[path = "windows_command_line_tests.rs"]
mod windows_command_line_tests;
