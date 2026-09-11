//! UNREGISTERED DRAFT: separately built synthetic helper, never a real decider.

#[cfg(windows)]
fn main() {
    if fixture::run().is_err() { std::process::exit(73); }
}

#[cfg(not(windows))]
fn main() { std::process::exit(74); }

#[cfg(windows)]
mod fixture {
    use std::io;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command, Stdio};
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        CREATE_BREAKAWAY_FROM_JOB, EVENT_MODIFY_STATE, OpenEventW,
        SYNCHRONIZATION_SYNCHRONIZE, SetEvent, WaitForSingleObject,
    };

    fn event(base: &str, suffix: &str) -> io::Result<OwnedHandle> {
        let name: Vec<u16> = format!("{base}-{suffix}").encode_utf16().chain([0]).collect();
        // SAFETY: a NUL-terminated test event name and a noninherited returned handle.
        let handle = unsafe { OpenEventW(
            EVENT_MODIFY_STATE | SYNCHRONIZATION_SYNCHRONIZE,
            /*binherithandle*/ 0, name.as_ptr(),
        ) };
        if handle.is_null() { return Err(io::Error::last_os_error()); }
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    fn signal(handle: &OwnedHandle) -> io::Result<()> {
        if unsafe { SetEvent(handle.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn wait(handle: &OwnedHandle) -> io::Result<()> {
        // Bounded fixture lifetime is failure cleanup, not production settlement.
        if unsafe { WaitForSingleObject(handle.as_raw_handle(), /*dwmilliseconds*/ 30_000) }
            != WAIT_OBJECT_0
        {
            return Err(io::Error::other("fixture handshake did not complete"));
        }
        Ok(())
    }

    fn command(mode: &str) -> io::Result<Command> {
        let mut command = Command::new(std::env::current_exe()?);
        command.args(["hook", "decide", "--client", "codex"])
            .env("COVENANT_FIXTURE_MODE", mode)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        Ok(command)
    }

    struct EscapedChild(Child);
    impl Drop for EscapedChild {
        fn drop(&mut self) {
            // Own only this test-created child if the breakaway assertion fails.
            if self.0.try_wait().ok().flatten().is_none() { let _ = self.0.kill(); }
            let _ = self.0.wait();
        }
    }

    pub(super) fn run() -> io::Result<()> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args != ["hook", "decide", "--client", "codex"].map(std::ffi::OsString::from) {
            return Err(io::Error::other("unexpected fixture arguments"));
        }
        let mode = std::env::var("COVENANT_FIXTURE_MODE").map_err(io::Error::other)?;
        if mode == "escape-control" { return Ok(()); }
        let base = std::env::var("COVENANT_FIXTURE_EVENTS").map_err(io::Error::other)?;
        let release = event(&base, "release")?;
        match mode.as_str() {
            "descendant" => {
                let block = event(&base, "block")?;
                signal(&event(&base, "tree")?)?;
                wait(&block)
            }
            "exit" => {
                signal(&event(&base, "started")?)?;
                wait(&release)
            }
            "tree" => {
                signal(&event(&base, "started")?)?;
                match command("escape-control")?.creation_flags(CREATE_BREAKAWAY_FROM_JOB).spawn() {
                    Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => {
                        signal(&event(&base, "denied")?)?;
                    }
                    Err(error) => return Err(error),
                    Ok(child) => {
                        drop(EscapedChild(child));
                        return Err(io::Error::other("fixture escaped its Job"));
                    }
                }
                let _descendant = command("descendant")?.spawn()?;
                // Child's ordinary Drop leaves it running after root exit.
                // The independently retained Job/descendant handles must prove cleanup.
                wait(&release)
            }
            _ => Err(io::Error::other("unknown fixture mode")),
        }
    }
}
