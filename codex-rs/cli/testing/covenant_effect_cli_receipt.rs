use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const RAW_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(super) enum ReceiptName {
    OrdinaryHookFirst,
    OrdinaryHookExistingMarker,
    OrdinaryMcpFirst,
    CovenantAbsenceFirst,
}

impl ReceiptName {
    fn basename(self) -> &'static str {
        match self {
            Self::OrdinaryHookFirst => "ordinary_hook_effect_is_observed.first.json",
            Self::OrdinaryHookExistingMarker => {
                "ordinary_hook_effect_is_observed.existing-marker.json"
            }
            Self::OrdinaryMcpFirst => "ordinary_mcp_requests_are_observed.first.json",
            Self::CovenantAbsenceFirst => "covenant_hook_mcp_effects_are_absent.first.json",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReceiptError {
    Root,
    Exists,
    Limit,
    Encode,
    Decode,
    Io,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("owned receipt refused")
    }
}

impl std::error::Error for ReceiptError {}

pub(super) struct ReceiptRoot {
    path: PathBuf,
}

impl ReceiptRoot {
    pub(super) fn open(path: &Path) -> Result<Self, ReceiptError> {
        if !path.is_absolute() {
            return Err(ReceiptError::Root);
        }
        if !path.metadata().map_err(|_| ReceiptError::Root)?.is_dir() {
            return Err(ReceiptError::Root);
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub(super) fn write_new<T: Serialize + ?Sized>(
        &self,
        name: ReceiptName,
        observation: &T,
    ) -> Result<PathBuf, ReceiptError> {
        let path = self.path.join(name.basename());
        // CreateNew precedes all serializer work, including on failed artifacts.
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    ReceiptError::Exists
                } else {
                    ReceiptError::Io
                }
            })?;
        let mut writer = ReceiptWriter {
            file,
            written: 0,
            failure: None,
        };
        let encoded = serde_json::to_writer(&mut writer, observation);
        // A serializer cannot turn a swallowed writer refusal into success.
        if let Some(error) = writer.failure {
            return Err(error);
        }
        encoded.map_err(|_| ReceiptError::Encode)?;
        writer.flush().map_err(|_| ReceiptError::Io)?;
        writer.file.sync_all().map_err(|_| ReceiptError::Io)?;
        drop(writer);
        Ok(path)
    }

    pub(super) fn read<T: DeserializeOwned>(&self, name: ReceiptName) -> Result<T, ReceiptError> {
        let mut file = File::open(self.path.join(name.basename())).map_err(|_| ReceiptError::Io)?;
        let metadata = file.metadata().map_err(|_| ReceiptError::Io)?;
        if !metadata.is_file() {
            return Err(ReceiptError::Io);
        }
        if metadata.len() > RAW_LIMIT as u64 {
            return Err(ReceiptError::Limit);
        }
        // Fixed storage, independent of metadata. One sentinel detects growth.
        let mut bytes = vec![0; RAW_LIMIT + 1];
        let mut filled = 0;
        loop {
            let count = match file.read(&mut bytes[filled..]) {
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(ReceiptError::Io),
            };
            if count == 0 {
                break;
            }
            filled += count;
            if filled > RAW_LIMIT {
                return Err(ReceiptError::Limit);
            }
        }
        drop(file);
        serde_json::from_slice(&bytes[..filled]).map_err(|_| ReceiptError::Decode)
    }
}

struct ReceiptWriter {
    file: File,
    written: usize,
    failure: Option<ReceiptError>,
}

impl Write for ReceiptWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(error) = self.failure {
            return Err(io::Error::other(error));
        }
        if bytes.len() > RAW_LIMIT - self.written {
            self.failure = Some(ReceiptError::Limit);
            return Err(io::Error::other(ReceiptError::Limit));
        }
        match self.file.write(bytes) {
            Ok(0) if !bytes.is_empty() => {
                self.failure = Some(ReceiptError::Io);
                Err(io::Error::other(ReceiptError::Io))
            }
            Ok(count) => {
                self.written += count;
                Ok(count)
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Err(error),
            Err(_) => {
                self.failure = Some(ReceiptError::Io);
                Err(io::Error::other(ReceiptError::Io))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(error) = self.failure {
            return Err(io::Error::other(error));
        }
        self.file.flush().map_err(|_| {
            self.failure = Some(ReceiptError::Io);
            io::Error::other(ReceiptError::Io)
        })
    }
}
