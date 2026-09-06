//! Synthetic Auto-store fixture through public auth and keyring builder APIs.

#![cfg(windows)]

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_login::AuthCredentialsStoreMode;
use codex_login::AuthDotJson;
use codex_login::AuthKeyringBackendKind;
use codex_login::load_auth_dot_json;
use codex_login::save_auth;
use keyring::credential::Credential;
use keyring::credential::CredentialApi;
use keyring::credential::CredentialBuilderApi;
use keyring::credential::CredentialPersistence;
use serde::Deserialize;
use serde::Serialize;
use std::any::Any;
use std::collections::BTreeMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const CHILD: &str = "COVENANT_AUTH_AUTO_CHILD";
const PREFIX: &str = "COVENANT_AUTH_AUTO_REPORT ";
const LIMIT: u64 = 65_536;
const DEADLINE: Duration = Duration::from_secs(/*secs*/ 30);

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(super) enum KeyringMode {
    RejectWrites,
    AcceptWrites,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Fixture {
    pub root: PathBuf,
    pub document: AuthDotJson,
    pub keyring_mode: KeyringMode,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) enum SaveOutcome {
    Saved,
    PermissionDenied,
    OtherFailure,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct Report {
    pub outcome: SaveOutcome,
    pub loaded: Option<AuthDotJson>,
    pub attempted: Vec<AuthDotJson>,
    pub persisted: Vec<AuthDotJson>,
}

type EntryKey = (Option<String>, String, String);

#[derive(Default)]
struct Store {
    values: BTreeMap<EntryKey, Vec<u8>>,
    attempted: Vec<AuthDotJson>,
}

struct Builder {
    store: Arc<Mutex<Store>>,
    mode: KeyringMode,
}

impl CredentialBuilderApi for Builder {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        Ok(Box::new(MemoryCredential {
            store: Arc::clone(&self.store),
            key: (target.map(str::to_owned), service.into(), user.into()),
            mode: self.mode,
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

struct MemoryCredential {
    store: Arc<Mutex<Store>>,
    key: EntryKey,
    mode: KeyringMode,
}

impl CredentialApi for MemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        let document = serde_json::from_slice(secret).map_err(|_| {
            keyring::Error::Invalid("synthetic payload".into(), "invalid auth JSON".into())
        })?;
        let mut store = self.store.lock().expect("synthetic keyring store poisoned");
        store.attempted.push(document);
        match self.mode {
            KeyringMode::RejectWrites => Err(keyring::Error::Invalid(
                "synthetic store".into(),
                "write rejected by fixture".into(),
            )),
            KeyringMode::AcceptWrites => {
                store.values.insert(self.key.clone(), secret.to_vec());
                Ok(())
            }
        }
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        self.store
            .lock()
            .expect("synthetic keyring store poisoned")
            .values
            .get(&self.key)
            .cloned()
            .ok_or(keyring::Error::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        self.store
            .lock()
            .expect("synthetic keyring store poisoned")
            .values
            .remove(&self.key)
            .map(|_| ())
            .ok_or(keyring::Error::NoEntry)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(super) fn is_child(test_name: &str) -> bool {
    std::env::var(CHILD).ok().as_deref() == Some(test_name)
}

pub(super) fn run_child() -> Result<()> {
    let fixture: Fixture = serde_json::from_reader(std::io::stdin().take(LIMIT))?;
    let store = Arc::new(Mutex::new(Store::default()));
    // Installed before the first public auth call. Every Entry is test-owned,
    // shares this child-local state, and has no platform-keyring fallback.
    keyring::set_default_credential_builder(Box::new(Builder {
        store: Arc::clone(&store),
        mode: fixture.keyring_mode,
    }));
    let home = fixture.root.join("mutable");
    let outcome = match save_auth(
        &home,
        &fixture.document,
        AuthCredentialsStoreMode::Auto,
        AuthKeyringBackendKind::Direct,
    ) {
        Ok(()) => SaveOutcome::Saved,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            SaveOutcome::PermissionDenied
        }
        Err(_) => SaveOutcome::OtherFailure,
    };
    // This creates a fresh public store and fresh Entries in the same child.
    let loaded = load_auth_dot_json(
        &home,
        AuthCredentialsStoreMode::Auto,
        AuthKeyringBackendKind::Direct,
    )?;
    let store = store.lock().expect("synthetic keyring store poisoned");
    let report = Report {
        outcome,
        loaded,
        attempted: store.attempted.clone(),
        persisted: store
            .values
            .values()
            .map(|bytes| serde_json::from_slice(bytes))
            .collect::<std::result::Result<_, _>>()?,
    };
    println!("{PREFIX}{}", serde_json::to_string(&report)?);
    std::io::stdout().flush()?;
    Ok(())
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Owns only the synthetic test child, never a Cargo/Rust build process.
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

pub(super) fn save_in_child(test_name: &str, fixture: &Fixture) -> Result<Report> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test_name, "--nocapture"])
        .current_dir(&fixture.root)
        .env_clear()
        .env(CHILD, test_name)
        .env("CODEX_HOME", fixture.root.join("mutable"))
        .env("CODEX_AUTH_HOME", fixture.root.join("auth"))
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
    let mut child = OwnedChild(command.spawn()?);
    let stdout = child.0.stdout.take().context("Auto child stdout missing")?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout.take(LIMIT)).lines() {
            let Ok(line) = line else { break };
            if let Some((_, payload)) = line.split_once(PREFIX) {
                let _ = sender.send(serde_json::from_str::<Report>(payload));
            }
        }
    });
    let mut input = child.0.stdin.take().context("Auto child stdin missing")?;
    let payload = serde_json::to_vec(fixture)?;
    ensure!(
        payload.len() < LIMIT as usize,
        "Auto fixture input too large"
    );
    input.write_all(&payload)?;
    drop(input);
    let report = receiver.recv_timeout(DEADLINE)??;
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(status) = child.0.try_wait()? {
            ensure!(status.success(), "Auto child failed");
            break;
        }
        ensure!(Instant::now() < deadline, "Auto child did not settle");
        thread::sleep(Duration::from_millis(/*millis*/ 5));
    }
    reader
        .join()
        .map_err(|_| anyhow::anyhow!("Auto report reader failed"))?;
    ensure!(
        receiver.try_recv().is_err(),
        "Auto child reported more than once"
    );
    Ok(report)
}
