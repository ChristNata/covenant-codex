//! One validated, scrubbed environment shared by policy JSON and native creation.

#[path = "secret_policy.rs"]
mod secret_policy;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::path::Component;
use std::path::Path;
use std::path::Prefix;
use windows_sys::Win32::Globalization::CSTR_EQUAL;
use windows_sys::Win32::Globalization::CSTR_GREATER_THAN;
use windows_sys::Win32::Globalization::CSTR_LESS_THAN;
use windows_sys::Win32::Globalization::CompareStringOrdinal;

const MAX_ENTRIES: usize = 2048;
const MAX_NAME_UNITS: usize = 1024;
const MAX_VALUE_UNITS: usize = 32766;
const MAX_BLOCK_UNITS: usize = 65536;

struct Entry {
    name: String,
    value: String,
    name_units: Vec<u16>,
    value_units: Vec<u16>,
}

pub struct FrozenWindowsEnvironment {
    json: BTreeMap<String, String>,
    native: Vec<u16>,
}

#[derive(Debug)]
pub struct WindowsEnvironmentError;

impl fmt::Display for WindowsEnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid Windows environment")
    }
}

impl std::error::Error for WindowsEnvironmentError {}

impl FrozenWindowsEnvironment {
    pub fn from_final_pairs(
        pairs: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<Self, WindowsEnvironmentError> {
        let mut entries: Vec<Entry> = Vec::new();
        // Each nonempty block has one final NUL beyond its entry terminators.
        let mut raw_units = 1_usize;
        for (index, (name, value)) in pairs.into_iter().enumerate() {
            if index >= MAX_ENTRIES {
                return Err(WindowsEnvironmentError);
            }
            let (name, name_units) = bounded_text(name, MAX_NAME_UNITS)?;
            let (value, value_units) = bounded_text(value, MAX_VALUE_UNITS)?;
            raw_units = raw_units
                .checked_add(name_units.len() + value_units.len() + 2)
                .filter(|units| *units <= MAX_BLOCK_UNITS)
                .ok_or(WindowsEnvironmentError)?;
            if name.starts_with('=') {
                let [b'=', drive, b':'] = name.as_bytes() else {
                    return Err(WindowsEnvironmentError);
                };
                if !drive.is_ascii_alphabetic() {
                    return Err(WindowsEnvironmentError);
                }
                let path = Path::new(&value);
                let Some(Component::Prefix(prefix)) = path.components().next() else {
                    return Err(WindowsEnvironmentError);
                };
                let (Prefix::Disk(value_drive) | Prefix::VerbatimDisk(value_drive)) = prefix.kind()
                else {
                    return Err(WindowsEnvironmentError);
                };
                if !path.is_absolute()
                    || compare_names(&[u16::from(*drive)], &[u16::from(value_drive)])?
                        != Ordering::Equal
                {
                    return Err(WindowsEnvironmentError);
                }
            } else if name.is_empty() || name.contains('=') {
                return Err(WindowsEnvironmentError);
            }

            // Insertion keeps comparison fallible; an infallible Ord/sort adapter
            // must not hide an OS comparison failure or erase duplicate evidence.
            let mut left = 0;
            let mut right = entries.len();
            while left < right {
                let middle = left + (right - left) / 2;
                match compare_names(&name_units, &entries[middle].name_units)? {
                    Ordering::Less => right = middle,
                    Ordering::Equal => return Err(WindowsEnvironmentError),
                    Ordering::Greater => left = middle + 1,
                }
            }
            entries.insert(
                left,
                Entry {
                    name,
                    value,
                    name_units,
                    value_units,
                },
            );
        }

        // Scrubbing begins only after every raw entry and collision was validated.
        let mut json = BTreeMap::new();
        let mut native = Vec::new();
        for entry in entries {
            if secret_policy::should_scrub(&entry.name_units)? {
                continue;
            }
            native.extend(entry.name_units);
            native.push(u16::from(b'='));
            native.extend(entry.value_units);
            native.push(0);
            json.insert(entry.name, entry.value);
        }
        if native.is_empty() {
            native.push(0);
        }
        native.push(0);
        Ok(Self { json, native })
    }

    pub fn as_json(&self) -> &BTreeMap<String, String> {
        &self.json
    }

    pub fn as_native_block(&self) -> &[u16] {
        &self.native
    }
}

fn bounded_text(
    text: OsString,
    maximum_units: usize,
) -> Result<(String, Vec<u16>), WindowsEnvironmentError> {
    let units: Vec<u16> = text.encode_wide().take(maximum_units + 1).collect();
    if units.len() > maximum_units || units.contains(&0) {
        return Err(WindowsEnvironmentError);
    }
    let text = text.into_string().map_err(|_| WindowsEnvironmentError)?;
    Ok((text, units))
}

fn compare_names(left: &[u16], right: &[u16]) -> Result<Ordering, WindowsEnvironmentError> {
    let left_length = i32::try_from(left.len()).map_err(|_| WindowsEnvironmentError)?;
    let right_length = i32::try_from(right.len()).map_err(|_| WindowsEnvironmentError)?;
    // SAFETY: both UTF-16 slices remain live for the explicit bounded lengths.
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left_length,
            right.as_ptr(),
            right_length,
            /*bignorecase*/ 1,
        )
    };
    match result {
        CSTR_LESS_THAN => Ok(Ordering::Less),
        CSTR_EQUAL => Ok(Ordering::Equal),
        CSTR_GREATER_THAN => Ok(Ordering::Greater),
        _ => Err(WindowsEnvironmentError),
    }
}

#[cfg(test)]
#[path = "windows_environment_tests.rs"]
mod tests;
