//! Bounded CRT-style representation of the envelope's already validated argv.
//!
//! This buffer does not establish image identity, child delivery or launch authority.

use super::ExecEventError;

const MAX_COMMAND_LINE_UNITS: usize = 32_767;
const QUOTE: u16 = b'"' as u16;
const BACKSLASH: u16 = b'\\' as u16;

pub(super) fn encode(argv: &[String]) -> Result<Vec<u16>, ExecEventError> {
    let (program, arguments) = argv.split_first().ok_or(ExecEventError)?;
    if program.contains('"') {
        return Err(ExecEventError);
    }
    let mut command = CommandLine(Vec::with_capacity(MAX_COMMAND_LINE_UNITS));
    // CRT argv[0] has distinct quoting rules: its backslashes stay literal.
    let quoted_program = needs_quotes(program);
    if quoted_program {
        command.push(QUOTE)?;
    }
    command.extend(program)?;
    if quoted_program {
        command.push(QUOTE)?;
    }
    for argument in arguments {
        command.push(u16::from(b' '))?;
        if !needs_quotes(argument) {
            command.extend(argument)?;
            continue;
        }
        command.push(QUOTE)?;
        let mut backslashes = 0_usize;
        for unit in argument.encode_utf16() {
            if unit == BACKSLASH {
                backslashes = backslashes.checked_add(1).ok_or(ExecEventError)?;
                continue;
            }
            let escaped = if unit == QUOTE {
                backslashes
                    .checked_mul(2)
                    .and_then(|count| count.checked_add(1))
                    .ok_or(ExecEventError)?
            } else {
                backslashes
            };
            command.backslashes(escaped)?;
            command.push(unit)?;
            backslashes = 0;
        }
        command.backslashes(backslashes.checked_mul(2).ok_or(ExecEventError)?)?;
        command.push(QUOTE)?;
    }
    // The same bound includes exactly one terminator; argv was validated NUL-free.
    command.push(/*unit*/ 0)?;
    Ok(command.0)
}

fn needs_quotes(argument: &str) -> bool {
    argument.is_empty() || argument.contains([' ', '\t', '\r', '\n', '"'])
}

struct CommandLine(Vec<u16>);

impl CommandLine {
    fn push(&mut self, unit: u16) -> Result<(), ExecEventError> {
        if self.0.len() >= MAX_COMMAND_LINE_UNITS {
            return Err(ExecEventError);
        }
        self.0.push(unit);
        Ok(())
    }

    fn extend(&mut self, value: &str) -> Result<(), ExecEventError> {
        for unit in value.encode_utf16() {
            self.push(unit)?;
        }
        Ok(())
    }

    fn backslashes(&mut self, count: usize) -> Result<(), ExecEventError> {
        let length = self
            .0
            .len()
            .checked_add(count)
            .filter(|length| *length <= MAX_COMMAND_LINE_UNITS)
            .ok_or(ExecEventError)?;
        self.0.resize(length, BACKSLASH);
        Ok(())
    }
}
