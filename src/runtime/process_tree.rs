//! Own the command's process tree, including descendants that outlive its parent.
use std::io;
use std::process::{Child, Command};

#[cfg(windows)]
pub(super) struct ProcessTree(std::os::windows::io::OwnedHandle);
#[cfg(unix)]
pub(super) struct ProcessTree(u32);

#[cfg(windows)]
pub(super) fn spawn(command: &mut Command) -> io::Result<(Child, ProcessTree)> {
    use std::os::windows::{io::AsRawHandle, process::CommandExt};
    use windows_sys::Win32::System::{JobObjects::*, Threading::*};

    let job = create_job()?;
    let raw = job.as_raw_handle();

    // Suspend before user code can spawn an unowned descendant or exit. The job
    // is assigned before resuming the only thread in this new process.
    command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    let mut child = command.spawn()?;
    let initialize = || {
        if unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }

        resume_primary_thread(child.id())
    };
    if let Err(error) = initialize() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }

    Ok((child, ProcessTree(job)))
}

#[cfg(windows)]
fn create_job() -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::*;

    let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }

    let job = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            raw,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }

    Ok(job)
}

#[cfg(windows)]
fn resume_primary_thread(pid: u32) -> io::Result<()> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::{Diagnostics::ToolHelp::*, Threading::*},
    };

    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while found != 0 {
        if entry.th32OwnerProcessID == pid {
            let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }

            let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
            return if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            };
        }
        found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Cannot find suspended command thread",
    ))
}

#[cfg(unix)]
pub(super) fn spawn(command: &mut Command) -> io::Result<(Child, ProcessTree)> {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
    let child = command.spawn()?;
    let tree = ProcessTree(child.id());

    Ok((child, tree))
}

impl ProcessTree {
    pub(super) fn terminate(&self) -> io::Result<()> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            if unsafe {
                windows_sys::Win32::System::JobObjects::TerminateJobObject(
                    self.0.as_raw_handle(),
                    1,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(unix)]
        if unsafe { libc::killpg(self.0 as libc::pid_t, libc::SIGKILL) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }

        Ok(())
    }
}
