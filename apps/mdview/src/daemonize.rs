//! Platform-specific daemonisation helpers.
//!
//! On the default (non-terminal, non-nvim) invocation, `mdview FILE.md` must
//! return control to the shell immediately while a detached child keeps the
//! Tauri event loop alive.
//!
//! * Windows: re-exec `self` with `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`
//!   if the current process isn't already the detached child (detected via the
//!   `MDVIEW_DETACHED=1` env var). The parent exits 0.
//! * Unix:   `fork` + `setsid` + close stdio. Parent exits 0.
//!
//! `unsafe` is allowed in this module (see `mod daemonize` in `main.rs`) because
//! `fork`, `setsid`, and `CreateProcessW` have no safe wrappers in std.

use anyhow::Result;

pub const MARKER_ENV: &str = "MDVIEW_DETACHED";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spawned {
    /// The current process is the detached child; continue running.
    Child,
    /// The current process spawned a detached child; caller should exit.
    Parent,
}

pub fn is_detached_child() -> bool {
    std::env::var_os(MARKER_ENV).is_some()
}

#[cfg(windows)]
pub fn daemonize() -> Result<Spawned> {
    use std::ffi::OsString;
    use std::mem::zeroed;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT,
        DETACHED_PROCESS, PROCESS_INFORMATION, STARTUPINFOW,
    };

    if is_detached_child() {
        return Ok(Spawned::Child);
    }

    let exe = std::env::current_exe()?;
    let mut cmdline: Vec<u16> = Vec::new();
    cmdline.push(b'"' as u16);
    cmdline.extend(OsString::from(exe.as_os_str()).encode_wide());
    cmdline.push(b'"' as u16);
    for arg in std::env::args_os().skip(1) {
        cmdline.push(b' ' as u16);
        cmdline.push(b'"' as u16);
        cmdline.extend(arg.encode_wide());
        cmdline.push(b'"' as u16);
    }
    cmdline.push(0);

    let mut env_block = build_env_with_marker();

    let flags =
        DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT;

    // Safety: CreateProcessW requires a mutable wide command-line buffer and a
    // (for Unicode) mutable env block; both are owned `Vec<u16>` with proper
    // terminators. The STARTUPINFO and PROCESS_INFORMATION structs are
    // zero-initialised per MSDN.
    let ok = unsafe {
        let mut si: STARTUPINFOW = zeroed();
        let mut pi: PROCESS_INFORMATION = zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let ok = CreateProcessW(
            std::ptr::null(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            env_block.as_mut_ptr().cast(),
            std::ptr::null(),
            &si,
            &mut pi,
        );
        if ok != 0 {
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        }
        ok
    };

    if ok == 0 {
        anyhow::bail!("CreateProcessW failed: {}", std::io::Error::last_os_error());
    }
    Ok(Spawned::Parent)
}

#[cfg(windows)]
fn build_env_with_marker() -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut out: Vec<u16> = Vec::new();
    let mut marker_seen = false;
    for (k, v) in std::env::vars_os() {
        if k.to_string_lossy().eq_ignore_ascii_case(MARKER_ENV) {
            marker_seen = true;
            push_pair(&mut out, MARKER_ENV, "1");
        } else {
            let mut pair = std::ffi::OsString::new();
            pair.push(&k);
            pair.push("=");
            pair.push(&v);
            out.extend(pair.encode_wide());
            out.push(0);
        }
    }
    if !marker_seen {
        push_pair(&mut out, MARKER_ENV, "1");
    }
    out.push(0);
    out
}

#[cfg(windows)]
fn push_pair(out: &mut Vec<u16>, key: &str, val: &str) {
    use std::os::windows::ffi::OsStrExt;
    let mut s = std::ffi::OsString::from(key);
    s.push("=");
    s.push(val);
    out.extend(s.encode_wide());
    out.push(0);
}

#[cfg(unix)]
pub fn daemonize() -> Result<Spawned> {
    if is_detached_child() {
        return Ok(Spawned::Child);
    }

    // Safety: fork/setsid/dup2/open have no safe wrappers. After a successful
    // fork the child continues on its own path; the parent returns
    // `Spawned::Parent` so `main` can exit normally.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        anyhow::bail!("fork failed: {}", std::io::Error::last_os_error());
    }
    if pid > 0 {
        return Ok(Spawned::Parent);
    }

    unsafe {
        if libc::setsid() < 0 {
            anyhow::bail!("setsid failed: {}", std::io::Error::last_os_error());
        }
        let devnull = std::ffi::CString::new("/dev/null").unwrap();
        let fd = libc::open(devnull.as_ptr(), libc::O_RDWR);
        if fd >= 0 {
            libc::dup2(fd, libc::STDIN_FILENO);
            libc::dup2(fd, libc::STDOUT_FILENO);
            libc::dup2(fd, libc::STDERR_FILENO);
            if fd > libc::STDERR_FILENO {
                libc::close(fd);
            }
        }
        // Post-fork single-threaded context; set_var is safe here.
        std::env::set_var(MARKER_ENV, "1");
    }
    Ok(Spawned::Child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_env_round_trip() {
        let prev = std::env::var_os(MARKER_ENV);
        std::env::remove_var(MARKER_ENV);
        assert!(!is_detached_child());
        std::env::set_var(MARKER_ENV, "1");
        assert!(is_detached_child());
        std::env::remove_var(MARKER_ENV);
        if let Some(v) = prev {
            std::env::set_var(MARKER_ENV, v);
        }
    }
}
