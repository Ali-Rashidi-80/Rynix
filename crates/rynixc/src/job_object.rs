//! Windows Job Object wrapper for clang link subprocess (Phase 31 / ADR-0022 amend).

#![cfg(windows)]

use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Command, ExitStatus};
use std::ptr;

use std::os::windows::raw::HANDLE;

type BOOL = i32;
type DWORD = u32;
type SizeT = usize;

const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x0000_2000;
const JOB_OBJECT_LIMIT_PROCESS_MEMORY: DWORD = 0x0000_0100;
const JOB_OBJECT_LIMIT_ACTIVE_PROCESS: DWORD = 0x0000_0008;
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;

#[repr(C)]
struct IoCounters {
    read_op: u64,
    write_op: u64,
    other_op: u64,
    read_x: u64,
    write_x: u64,
    other_x: u64,
}

#[repr(C)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: DWORD,
    minimum_working_set_size: SizeT,
    maximum_working_set_size: SizeT,
    active_process_limit: DWORD,
    affinity: usize,
    priority_class: DWORD,
    scheduling_class: DWORD,
}

#[repr(C)]
struct JobObjectExtendedLimitInformation {
    basic: JobObjectBasicLimitInformation,
    io: IoCounters,
    process_memory_limit: SizeT,
    job_memory_limit: SizeT,
    peak_process_memory: SizeT,
    peak_job_memory: SizeT,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attrs: *mut std::ffi::c_void, name: *const u16) -> HANDLE;
    fn SetInformationJobObject(
        job: HANDLE,
        info_class: u32,
        info: *mut std::ffi::c_void,
        len: DWORD,
    ) -> BOOL;
    fn AssignProcessToJobObject(job: HANDLE, process: HANDLE) -> BOOL;
    fn CloseHandle(handle: HANDLE) -> BOOL;
    fn GetLastError() -> DWORD;
}

/// Run `cmd` assigned to a Job Object: kill-on-close, 1 GiB process memory, max 32 processes.
///
/// Assign-after-spawn (no CREATE_SUSPENDED): small race window before assign is
/// accepted for Phase 31 smoke; clang link is the primary child.
pub fn run_in_job(mut cmd: Command) -> io::Result<ExitStatus> {
    // SAFETY: kernel32 Job Object APIs; job handle closed on all paths.
    unsafe {
        let job = CreateJobObjectW(ptr::null_mut(), ptr::null());
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut info: JobObjectExtendedLimitInformation = std::mem::zeroed();
        info.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        info.basic.active_process_limit = 32;
        info.process_memory_limit = 1024 * 1024 * 1024; // 1 GiB

        if SetInformationJobObject(
            job,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
            &mut info as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of_val(&info) as DWORD,
        ) == 0
        {
            let e = io::Error::from_raw_os_error(GetLastError() as i32);
            CloseHandle(job);
            return Err(e);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                CloseHandle(job);
                return Err(e);
            }
        };

        if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0 {
            let e = io::Error::from_raw_os_error(GetLastError() as i32);
            let _ = child.kill();
            let _ = child.wait();
            CloseHandle(job);
            return Err(e);
        }

        let status = child.wait();
        CloseHandle(job);
        status
    }
}
