//! Retained-sink observation for the Wave-1 auth scenarios.

#![cfg(windows)]

use super::backend_sink_keyring::MemoryStore;
use super::backend_sink_support::BackendCase;
use super::backend_sink_support::Fixture;
use super::backend_sink_support::Scenario;
use super::backend_sink_support::document;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::load_auth_dot_json;
use pretty_assertions::assert_eq;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::MakeWriter;

const TRACE_LIMIT: usize = 131_072;
const FILE_LIMIT: u64 = 1_048_576;
const TREE_DEPTH_LIMIT: usize = 32;
const TREE_ENTRY_LIMIT: usize = 4_096;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

#[derive(Clone)]
pub(super) struct TraceCapture {
    state: Arc<Mutex<TraceState>>,
}

#[derive(Default)]
struct TraceState {
    bytes: Vec<u8>,
    overflowed: bool,
}

impl TraceCapture {
    pub(super) fn install() -> Result<Self> {
        let capture = Self {
            state: Arc::new(Mutex::new(TraceState::default())),
        };
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_writer(capture.clone())
            .with_span_events(FmtSpan::FULL)
            .with_ansi(false)
            .without_time()
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|error| anyhow::anyhow!("failed to install trace capture: {error}"))?;
        Ok(capture)
    }

    fn snapshot(&self) -> Result<Vec<u8>> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        ensure!(!state.overflowed, "retained tracing exceeded its byte cap");
        Ok(state.bytes.clone())
    }
}

impl<'a> MakeWriter<'a> for TraceCapture {
    type Writer = TraceWriter;

    fn make_writer(&'a self) -> Self::Writer {
        TraceWriter {
            state: Arc::clone(&self.state),
        }
    }
}

pub(super) struct TraceWriter {
    state: Arc<Mutex<TraceState>>,
}

impl Write for TraceWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let remaining = TRACE_LIMIT.saturating_sub(state.bytes.len());
        let accepted = remaining.min(input.len());
        state.bytes.extend_from_slice(&input[..accepted]);
        state.overflowed |= accepted != input.len();
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct Sentinels {
    values: Vec<String>,
}

impl Sentinels {
    pub(super) fn new(fixture: &Fixture) -> Result<Self> {
        let mut values = vec![
            fixture.api_key.clone(),
            fixture.personal_access_token.clone(),
            fixture.access_seed.clone(),
            fixture.refresh_seed.clone(),
            fixture.id_payload.clone(),
        ];
        for generation in [0, 1] {
            let document = document(fixture, generation)?;
            let tokens = document.tokens.context("synthetic tokens missing")?;
            values.extend([
                tokens.access_token,
                tokens.refresh_token,
                tokens.id_token.raw_jwt,
            ]);
        }
        Ok(Self { values })
    }

    fn encoded(&self) -> Result<Vec<Vec<u8>>> {
        let mut forms = Vec::new();
        for value in &self.values {
            let raw = value.as_bytes();
            forms.push(raw.to_vec());
            forms.push(serde_json::to_string(value)?.into_bytes());
            forms.push(percent(raw, HexCase::Upper).into_bytes());
            forms.push(percent(raw, HexCase::Lower).into_bytes());
            for alphabet in [Alphabet::Standard, Alphabet::Url] {
                let padded = base64(raw, alphabet);
                forms.push(padded.as_bytes().to_vec());
                forms.push(padded.trim_end_matches('=').as_bytes().to_vec());
            }
        }
        forms.sort();
        forms.dedup();
        Ok(forms)
    }
}

pub(super) fn assert_clean_bytes(bytes: &[u8], sentinels: &Sentinels) -> Result<()> {
    assert_no_forms(bytes, &sentinels.encoded()?)
}

pub(super) fn audit_retained(
    fixture: &Fixture,
    traces: &TraceCapture,
    keyring: &Arc<Mutex<MemoryStore>>,
) -> Result<()> {
    let sentinels = Sentinels::new(fixture)?;
    assert_clean_bytes(&traces.snapshot()?, &sentinels)?;
    let keyring = keyring
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .snapshot();
    ensure!(
        !keyring.journal_overflowed,
        "retained keyring journal exceeded its entry cap"
    );
    for journal in &keyring.journal {
        assert_clean_bytes(journal, &sentinels)?;
    }
    if matches!(
        fixture.scenario,
        Scenario::Refresh(BackendCase::Direct | BackendCase::AutoDirect)
    ) {
        ensure!(
            keyring.values.len() == 1,
            "Direct backend retained an unexpected value count"
        );
        let retained = serde_json::from_slice(&keyring.values[0])?;
        let observed = load_auth_dot_json(
            &fixture.root.join("mutable"),
            match fixture.scenario {
                Scenario::Refresh(case) => case.mode(),
                Scenario::EphemeralDirectLogout
                | Scenario::EphemeralFreshProbe
                | Scenario::PersistentManagerLogout
                | Scenario::EphemeralManagerLogout
                | Scenario::ApiKeyLoginProbeLogout
                | Scenario::AccessTokenLoginProbe
                | Scenario::BrowserCallbackProbe
                | Scenario::DeviceCodeProbe
                | Scenario::RevokeSuccess
                | Scenario::RevokeFailure => unreachable!(),
            },
            codex_login::AuthKeyringBackendKind::Direct,
        )?;
        assert_eq!(observed, Some(retained));
    } else {
        for value in &keyring.values {
            assert_clean_bytes(value, &sentinels)?;
        }
    }
    scan_tree(
        &fixture.root,
        allowed_auth_file(fixture).as_deref(),
        &sentinels.encoded()?,
    )
}

fn allowed_auth_file(fixture: &Fixture) -> Option<PathBuf> {
    let allowed = match fixture.scenario {
        Scenario::Refresh(case) => case.allows_auth_file(),
        Scenario::AccessTokenLoginProbe
        | Scenario::BrowserCallbackProbe
        | Scenario::DeviceCodeProbe => true,
        Scenario::EphemeralDirectLogout
        | Scenario::EphemeralFreshProbe
        | Scenario::PersistentManagerLogout
        | Scenario::EphemeralManagerLogout
        | Scenario::ApiKeyLoginProbeLogout
        | Scenario::RevokeSuccess
        | Scenario::RevokeFailure => false,
    };
    allowed.then(|| fixture.root.join("auth/auth.json"))
}

fn scan_tree(root: &Path, allowed_file: Option<&Path>, forms: &[Vec<u8>]) -> Result<()> {
    let mut entries = 0;
    scan_tree_bounded(root, allowed_file, forms, /*depth*/ 0, &mut entries)
}

fn scan_tree_bounded(
    root: &Path,
    allowed_file: Option<&Path>,
    forms: &[Vec<u8>],
    depth: usize,
    entries: &mut usize,
) -> Result<()> {
    ensure!(
        depth <= TREE_DEPTH_LIMIT,
        "retained tree exceeded depth cap"
    );
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        *entries += 1;
        ensure!(
            *entries <= TREE_ENTRY_LIMIT,
            "retained tree exceeded entry cap"
        );
        assert_no_forms(entry.file_name().to_string_lossy().as_bytes(), forms)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
            "unexpected retained reparse point"
        );
        let file_type = metadata.file_type();
        if file_type.is_dir() {
            scan_tree_bounded(&path, allowed_file, forms, depth + 1, entries)?;
        } else if file_type.is_file() && Some(path.as_path()) != allowed_file {
            let mut bytes = Vec::new();
            fs::File::open(path)?
                .take(FILE_LIMIT + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= FILE_LIMIT as usize,
                "retained file exceeded cap"
            );
            assert_no_forms(&bytes, forms)?;
        }
    }
    Ok(())
}

fn assert_no_forms(bytes: &[u8], forms: &[Vec<u8>]) -> Result<()> {
    for (index, form) in forms.iter().enumerate() {
        ensure!(
            !contains(bytes, form),
            "retained sink contains secret encoding {index}"
        );
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[derive(Clone, Copy)]
enum Alphabet {
    Standard,
    Url,
}

fn base64(input: &[u8], alphabet: Alphabet) -> String {
    let table = match alphabet {
        Alphabet::Standard => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
        Alphabet::Url => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
    };
    let mut encoded = String::new();
    for chunk in input.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        encoded.push(table[((bits >> 18) & 63) as usize] as char);
        encoded.push(table[((bits >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            table[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            table[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

#[derive(Clone, Copy)]
enum HexCase {
    Upper,
    Lower,
}

fn percent(input: &[u8], hex_case: HexCase) -> String {
    let digits = match hex_case {
        HexCase::Upper => b"0123456789ABCDEF",
        HexCase::Lower => b"0123456789abcdef",
    };
    let mut encoded = String::new();
    for byte in input {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(digits[(byte >> 4) as usize] as char);
            encoded.push(digits[(byte & 15) as usize] as char);
        }
    }
    encoded
}
