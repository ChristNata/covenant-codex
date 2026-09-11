//! UNREGISTERED DRAFT: real private Job ownership; no implementation or RED yet.

#![cfg(windows)]

use super::PolicyChildStart;
use super::PolicyStdio;
use super::SuspendedJobChild;
use crate::FrozenWindowsEnvironment;
use pretty_assertions::assert_eq;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io;
use std::os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::ptr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ALREADY_EXISTS, GetHandleInformation,
    GetLastError, HANDLE, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::JobObjects::{
    IsProcessInJob, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
    JobObjectExtendedLimitInformation, QueryInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, GetExitCodeProcess, GetProcessId, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, SetEvent, TerminateProcess,
    WaitForSingleObject,
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn checked(success: i32) -> io::Result<()> {
    if success == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() { return Err(io::Error::last_os_error()); }
    // SAFETY: the caller transfers one newly created/opened handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

enum Inheritance { Observer, Stdio }

fn duplicate(handle: &impl AsRawHandle, inheritance: Inheritance) -> io::Result<OwnedHandle> {
    let mut copy = ptr::null_mut();
    // SAFETY: duplicate within this live process; copy is a valid output slot.
    unsafe {
        checked(DuplicateHandle(
            GetCurrentProcess(), handle.as_raw_handle(), GetCurrentProcess(), &mut copy,
            /*dwdesiredaccess*/ 0, i32::from(matches!(inheritance, Inheritance::Stdio)),
            DUPLICATE_SAME_ACCESS,
        ))?;
    }
    owned(copy)
}

fn event(base: &str, suffix: &str) -> io::Result<OwnedHandle> {
    let name: Vec<u16> = format!("{base}-{suffix}").encode_utf16().chain([0]).collect();
    // SAFETY: NUL-terminated name; default security; a noninherited manual event.
    let handle = unsafe {
        CreateEventW(ptr::null(), /*bmanualreset*/ 1, /*binitialstate*/ 0, name.as_ptr())
    };
    let existing = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    let handle = owned(handle)?;
    if existing { return Err(io::Error::other("fixture event already exists")); }
    Ok(handle)
}

fn wait(handle: &OwnedHandle, milliseconds: u32) -> u32 {
    // SAFETY: this independently owned handle remains live across the wait.
    unsafe { WaitForSingleObject(handle.as_raw_handle(), milliseconds) }
}

fn signal(handle: &OwnedHandle) -> io::Result<()> {
    // SAFETY: this fixture owns an event with modify-state access.
    checked(unsafe { SetEvent(handle.as_raw_handle()) })
}

struct Fixture {
    began: Instant,
    application: PathBuf,
    environment: FrozenWindowsEnvironment,
    stdio: [OwnedHandle; 3],
    started: OwnedHandle,
    tree_ready: OwnedHandle,
    release_root: OwnedHandle,
    breakaway_denied: OwnedHandle,
    _block_descendant: OwnedHandle,
}

impl Fixture {
    fn new(mode: &str) -> Result<Self> {
        let began = Instant::now();
        let application = codex_utils_cargo_bin::cargo_bin("codex-covenant-policy-fixture")?;
        assert!(application.is_absolute());
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let base = format!("Local\\CovenantJobTest-{}-{nonce}", std::process::id());
        let mut pairs = vec![
            (OsString::from("COVENANT_FIXTURE_EVENTS"), OsString::from(&base)),
            (OsString::from("COVENANT_FIXTURE_MODE"), OsString::from(mode)),
        ];
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) { pairs.push((name.into(), value)); }
        }
        // Fixture-only null stdio; no model pathname or inherited console handles.
        let null = OpenOptions::new().read(true).write(true).open("NUL")?;
        Ok(Self {
            began,
            application,
            environment: FrozenWindowsEnvironment::from_final_pairs(pairs)?,
            stdio: [
                duplicate(&null, Inheritance::Stdio)?, duplicate(&null, Inheritance::Stdio)?,
                duplicate(&null, Inheritance::Stdio)?,
            ],
            started: event(&base, "started")?,
            tree_ready: event(&base, "tree")?,
            release_root: event(&base, "release")?,
            breakaway_denied: event(&base, "denied")?,
            _block_descendant: event(&base, "block")?,
        })
    }

    fn create(&self) -> io::Result<SuspendedJobChild> {
        SuspendedJobChild::create(PolicyChildStart {
            application: &self.application,
            cwd: self.application.parent().expect("absolute fixture parent"),
            environment: &self.environment,
            stdio: PolicyStdio {
                stdin: self.stdio[0].as_handle(), stdout: self.stdio[1].as_handle(),
                stderr: self.stdio[2].as_handle(),
            },
        })
    }
}

struct Observer { job: OwnedHandle, root: OwnedHandle }

impl Observer {
    fn retain(child: &SuspendedJobChild) -> io::Result<Self> {
        Ok(Self {
            job: duplicate(&child.owned.job, Inheritance::Observer)?,
            root: duplicate(&child.owned.process, Inheritance::Observer)?,
        })
    }

    fn active(&self) -> io::Result<u32> {
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // SAFETY: class, output layout and byte length match the native API.
        checked(unsafe { QueryInformationJobObject(
            self.job.as_raw_handle(), JobObjectBasicAccountingInformation,
            (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of_val(&info) as u32, ptr::null_mut(),
        ) })?;
        Ok(info.ActiveProcesses)
    }

    fn settled(&self) -> io::Result<(u32, u32)> {
        Ok((self.active()?, wait(&self.root, /*milliseconds*/ 0)))
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        // Fixture cleanup runs AFTER assertions, never as evidence of owner cleanup.
        // Keeping this duplicate alive defeats KILL_ON_JOB_CLOSE false positives.
        unsafe {
            TerminateJobObject(self.job.as_raw_handle(), /*uexitcode*/ 91);
            // Retained exact root handle also cleans up a failed membership assertion.
            TerminateProcess(self.root.as_raw_handle(), /*uexitcode*/ 91);
            WaitForSingleObject(self.root.as_raw_handle(), /*dwmilliseconds*/ 5_000);
        }
    }
}

fn member(process: &OwnedHandle, job: &OwnedHandle) -> io::Result<bool> {
    let mut result = 0;
    checked(unsafe { IsProcessInJob(process.as_raw_handle(), job.as_raw_handle(), &mut result) })?;
    Ok(result != 0)
}

fn inherit_flag(handle: &OwnedHandle) -> io::Result<u32> {
    let mut flags = 0;
    checked(unsafe { GetHandleInformation(handle.as_raw_handle(), &mut flags) })?;
    Ok(flags & HANDLE_FLAG_INHERIT)
}

fn exit_code(handle: &OwnedHandle) -> io::Result<u32> {
    let mut code = u32::MAX;
    checked(unsafe { GetExitCodeProcess(handle.as_raw_handle(), &mut code) })?;
    Ok(code)
}

#[test]
fn covenant_native_job_suspended_then_resumed_positive() -> Result {
    let fixture = Fixture::new("exit")?;
    let suspended = fixture.create()?;
    let observer = Observer::retain(&suspended)?;
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    // SAFETY: the output struct matches JobObjectExtendedLimitInformation.
    checked(unsafe { QueryInformationJobObject(
        observer.job.as_raw_handle(), JobObjectExtendedLimitInformation,
        (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
        size_of_val(&limits) as u32, ptr::null_mut(),
    ) })?;
    assert_eq!(
        (member(&observer.root, &observer.job)?, observer.settled()?,
         wait(&fixture.started, /*milliseconds*/ 100), limits.BasicLimitInformation.LimitFlags,
         [inherit_flag(&suspended.owned.job)?, inherit_flag(&suspended.owned.process)?,
          inherit_flag(&suspended.owned.thread)?]),
        (true, (1, WAIT_TIMEOUT), WAIT_TIMEOUT, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, [0; 3]),
    );
    let running = suspended.resume()?;
    assert_eq!(wait(&fixture.started, /*milliseconds*/ 10_000), WAIT_OBJECT_0);
    signal(&fixture.release_root)?;
    assert_eq!(wait(&observer.root, /*milliseconds*/ 10_000), WAIT_OBJECT_0);
    assert_eq!(exit_code(&observer.root)?, 0);
    running.terminate_and_settle()?;
    assert_eq!(observer.settled()?, (0, WAIT_OBJECT_0));
    assert!(fixture.began.elapsed() < Duration::from_secs(/*secs*/ 20));
    Ok(())
}

#[test]
fn covenant_native_job_cancel_before_resume_never_runs_body() -> Result {
    let fixture = Fixture::new("exit")?;
    let suspended = fixture.create()?;
    let observer = Observer::retain(&suspended)?;
    suspended.terminate_and_settle()?;
    assert_eq!(
        (observer.settled()?, wait(&fixture.started, /*milliseconds*/ 0)),
        ((0, WAIT_OBJECT_0), WAIT_TIMEOUT),
    );
    assert!(fixture.began.elapsed() < Duration::from_secs(/*secs*/ 20));
    Ok(())
}

#[repr(C)]
#[derive(Default)]
struct ProcessIds { assigned: u32, listed: u32, ids: [usize; 4] }

enum Settlement { RootExited, CallerDropped }

fn tree_cleanup(settlement: Settlement) -> Result {
    let fixture = Fixture::new("tree")?;
    let suspended = fixture.create()?;
    let observer = Observer::retain(&suspended)?;
    let running = suspended.resume()?;
    assert_eq!(wait(&fixture.tree_ready, /*milliseconds*/ 10_000), WAIT_OBJECT_0);
    assert_eq!(wait(&fixture.breakaway_denied, /*milliseconds*/ 0), WAIT_OBJECT_0);
    let mut ids = ProcessIds::default();
    // SAFETY: the native header is followed by capacity for four ULONG_PTR ids.
    checked(unsafe { QueryInformationJobObject(
        observer.job.as_raw_handle(), JobObjectBasicProcessIdList,
        (&mut ids as *mut ProcessIds).cast(), size_of_val(&ids) as u32, ptr::null_mut(),
    ) })?;
    assert_eq!((ids.assigned, ids.listed, observer.active()?), (2, 2, 2));
    let root_id = unsafe { GetProcessId(observer.root.as_raw_handle()) } as usize;
    let [first, second, ..] = ids.ids;
    let descendant_id = if first == root_id { second } else { assert_eq!(second, root_id); first };
    // Both handshakes retain live processes, so the selected PID cannot be reused.
    let descendant = owned(unsafe { OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
        /*binherithandle*/ 0, u32::try_from(descendant_id)?,
    ) })?;
    assert_eq!((member(&descendant, &observer.job)?, wait(&descendant, /*milliseconds*/ 0)),
               (true, WAIT_TIMEOUT));
    assert_eq!(wait(&observer.root, /*milliseconds*/ 0), WAIT_TIMEOUT);
    match settlement {
        Settlement::RootExited => {
            signal(&fixture.release_root)?;
            assert_eq!(wait(&observer.root, /*milliseconds*/ 10_000), WAIT_OBJECT_0);
            assert_eq!(exit_code(&observer.root)?, 0);
            assert_eq!(wait(&descendant, /*milliseconds*/ 0), WAIT_TIMEOUT);
            running.terminate_and_settle()?;
        }
        Settlement::CallerDropped => drop(running),
    }
    assert_eq!((observer.settled()?, wait(&descendant, /*milliseconds*/ 0)),
               ((0, WAIT_OBJECT_0), WAIT_OBJECT_0));
    // Total assertion time precedes the helper's 30-second failure watchdog.
    // Waiting for the fixture to expire cannot substitute for explicit cleanup.
    assert!(fixture.began.elapsed() < Duration::from_secs(/*secs*/ 20));
    Ok(())
}

#[test]
fn covenant_native_job_root_exit_settles_retained_descendant() -> Result {
    tree_cleanup(Settlement::RootExited)
}

#[test]
fn covenant_native_job_drop_settles_running_tree() -> Result {
    tree_cleanup(Settlement::CallerDropped)
}
