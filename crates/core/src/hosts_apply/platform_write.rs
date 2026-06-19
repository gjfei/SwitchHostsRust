//! 平台提权写入（对齐 SwitchHosts `hosts_apply/elevation.rs`）。

use std::path::{Path, PathBuf};

use super::error::ApplyError;

fn stage_temp_file(content: &str) -> Result<PathBuf, ApplyError> {
    let path = std::env::temp_dir().join(format!(
        "swh_apply_{}.hosts",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, content)?;
    Ok(path)
}

pub fn elevated_write(path: &Path, content: &str) -> Result<(), ApplyError> {
    let tmp_path = stage_temp_file(content)?;
    let result = elevate_copy(&tmp_path, path);
    let _ = std::fs::remove_file(&tmp_path);
    result
}

// ---- macOS: Security.framework + 进程内缓存 AuthorizationRef ---------------
//
// 对齐 SwitchHosts：首次写入弹出系统密码框，同进程后续写入复用授权，不再重复询问。

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::CString;
    use std::path::Path;
    use std::sync::Mutex;

    use super::ApplyError;

    mod security_ffi {
        use std::ffi::c_void;

        pub type AuthorizationRef = *mut c_void;
        pub type OSStatus = i32;

        pub const ERR_AUTHORIZATION_SUCCESS: OSStatus = 0;
        pub const ERR_AUTHORIZATION_CANCELED: OSStatus = -60006;
        pub const ERR_AUTHORIZATION_INVALID_REF: OSStatus = -60002;
        pub const ERR_AUTHORIZATION_DENIED: OSStatus = -60005;

        pub const K_AUTH_FLAG_INTERACTION_ALLOWED: u32 = 1 << 0;
        pub const K_AUTH_FLAG_EXTEND_RIGHTS: u32 = 1 << 1;

        #[repr(C)]
        #[allow(dead_code)]
        pub struct AuthorizationItem {
            pub name: *const u8,
            pub value_length: usize,
            pub value: *mut c_void,
            pub flags: u32,
        }

        #[repr(C)]
        #[allow(dead_code)]
        pub struct AuthorizationRights {
            pub count: u32,
            pub items: *mut AuthorizationItem,
        }

        #[link(name = "Security", kind = "framework")]
        extern "C" {
            pub fn AuthorizationCreate(
                rights: *const AuthorizationRights,
                environment: *const c_void,
                flags: u32,
                authorization: *mut AuthorizationRef,
            ) -> OSStatus;

            pub fn AuthorizationFree(authorization: AuthorizationRef, flags: u32) -> OSStatus;

            pub fn AuthorizationExecuteWithPrivileges(
                authorization: AuthorizationRef,
                path_to_tool: *const u8,
                options: u32,
                arguments: *const *const u8,
                communications_pipe: *mut *mut c_void,
            ) -> OSStatus;
        }
    }

    struct CachedAuth(security_ffi::AuthorizationRef);

    // SAFETY: AuthorizationRef 为进程级 opaque 指针。
    unsafe impl Send for CachedAuth {}

    impl Drop for CachedAuth {
        fn drop(&mut self) {
            unsafe {
                security_ffi::AuthorizationFree(self.0, 0);
            }
        }
    }

    static CACHED_AUTH: Mutex<Option<CachedAuth>> = Mutex::new(None);
    static ELEVATE_LOCK: Mutex<()> = Mutex::new(());

    fn get_or_create_auth() -> Result<security_ffi::AuthorizationRef, ApplyError> {
        let mut guard = CACHED_AUTH.lock().expect("auth mutex poisoned");
        if let Some(ref cached) = *guard {
            return Ok(cached.0);
        }

        // 空 rights 创建 ref（不在此处弹窗）；首次 AEWP 时再弹出系统密码框。
        // 带 system.privilege.admin 的 Create 在非 .app 包（cargo run）下可能静默失败。
        let flags = security_ffi::K_AUTH_FLAG_INTERACTION_ALLOWED
            | security_ffi::K_AUTH_FLAG_EXTEND_RIGHTS;

        let mut auth_ref: security_ffi::AuthorizationRef = std::ptr::null_mut();
        let status = unsafe {
            security_ffi::AuthorizationCreate(std::ptr::null(), std::ptr::null(), flags, &mut auth_ref)
        };

        match status {
            security_ffi::ERR_AUTHORIZATION_SUCCESS => {
                *guard = Some(CachedAuth(auth_ref));
                Ok(auth_ref)
            }
            security_ffi::ERR_AUTHORIZATION_CANCELED => Err(ApplyError::Cancelled),
            other => Err(ApplyError::Elevation(format!(
                "AuthorizationCreate failed: OSStatus {other}"
            ))),
        }
    }

    fn invalidate_cached_auth() {
        let mut guard = CACHED_AUTH.lock().expect("auth mutex poisoned");
        *guard = None;
    }

    enum MacElevateError {
        AuthExec(security_ffi::OSStatus, String),
        Other(ApplyError),
    }

    impl From<ApplyError> for MacElevateError {
        fn from(e: ApplyError) -> Self {
            MacElevateError::Other(e)
        }
    }

    fn path_to_cstr(path: &Path) -> Result<CString, ApplyError> {
        let s = path
            .to_str()
            .ok_or_else(|| ApplyError::Elevation("path is not valid UTF-8".into()))?;
        CString::new(s).map_err(|e| ApplyError::Elevation(format!("CString from path: {e}")))
    }

    fn execute_privileged_copy(
        auth_ref: security_ffi::AuthorizationRef,
        src: &Path,
        dst: &Path,
    ) -> Result<(), MacElevateError> {
        let src_cstr = path_to_cstr(src)?;
        let dst_cstr = path_to_cstr(dst)?;

        let cp_args: [*const u8; 3] = [
            src_cstr.as_ptr() as *const u8,
            dst_cstr.as_ptr() as *const u8,
            std::ptr::null(),
        ];
        let exit = unsafe { run_privileged(auth_ref, b"/bin/cp\0".as_ptr(), cp_args.as_ptr())? };
        if exit != 0 {
            return Err(ApplyError::Elevation(format!("/bin/cp exited with status {exit}")).into());
        }

        let chmod_args: [*const u8; 3] = [
            b"644\0".as_ptr(),
            dst_cstr.as_ptr() as *const u8,
            std::ptr::null(),
        ];
        let exit =
            unsafe { run_privileged(auth_ref, b"/bin/chmod\0".as_ptr(), chmod_args.as_ptr())? };
        if exit != 0 {
            return Err(
                ApplyError::Elevation(format!("/bin/chmod exited with status {exit}")).into(),
            );
        }

        Ok(())
    }

    unsafe fn run_privileged(
        auth_ref: security_ffi::AuthorizationRef,
        tool: *const u8,
        args: *const *const u8,
    ) -> Result<i32, MacElevateError> {
        let mut pipe: *mut libc::FILE = std::ptr::null_mut();
        let status = security_ffi::AuthorizationExecuteWithPrivileges(
            auth_ref,
            tool,
            0,
            args,
            &mut pipe as *mut *mut libc::FILE as *mut *mut std::ffi::c_void,
        );
        if status != security_ffi::ERR_AUTHORIZATION_SUCCESS {
            let tool_name = std::ffi::CStr::from_ptr(tool as *const i8).to_string_lossy();
            return Err(MacElevateError::AuthExec(
                status,
                format!("AEWP({tool_name}): OSStatus {status}"),
            ));
        }

        if !pipe.is_null() {
            let mut buf = [0u8; 256];
            while libc::fread(buf.as_mut_ptr() as *mut std::ffi::c_void, 1, buf.len(), pipe) > 0 {}
            libc::fclose(pipe);
        }

        let mut wstatus: i32 = 0;
        let pid = libc::wait(&mut wstatus);
        if pid < 0 {
            return Ok(-1);
        }
        if libc::WIFEXITED(wstatus) {
            Ok(libc::WEXITSTATUS(wstatus))
        } else {
            Ok(-1)
        }
    }

    fn is_auth_stale(status: security_ffi::OSStatus) -> bool {
        status == security_ffi::ERR_AUTHORIZATION_INVALID_REF
            || status == security_ffi::ERR_AUTHORIZATION_DENIED
    }

    fn mac_elevate_to_apply_error(e: MacElevateError) -> ApplyError {
        match e {
            MacElevateError::AuthExec(status, _)
                if status == security_ffi::ERR_AUTHORIZATION_CANCELED =>
            {
                ApplyError::Cancelled
            }
            MacElevateError::AuthExec(_, msg) => ApplyError::Elevation(msg),
            MacElevateError::Other(e) => e,
        }
    }

    fn sh_single_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "'\\''"))
    }

    fn osascript_elevate_copy(src: &Path, dst: &Path) -> Result<(), ApplyError> {
        use std::process::Command;

        let from = src.display().to_string();
        let to = dst.display().to_string();
        let script = format!(
            "do shell script \"/bin/cp -f {} {} && /bin/chmod 644 {}\" with administrator privileges",
            sh_single_quote(&from),
            sh_single_quote(&to),
            sh_single_quote(&to),
        );

        let output = Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| ApplyError::Elevation(format!("无法调用 osascript：{e}")))?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.contains("-128") || stderr.to_ascii_lowercase().contains("user canceled") {
            return Err(ApplyError::Cancelled);
        }
        Err(ApplyError::Elevation(if stderr.is_empty() {
            format!("osascript 退出码 {}", output.status)
        } else {
            stderr
        }))
    }

    fn elevate_copy_security(src: &Path, dst: &Path) -> Result<(), ApplyError> {
        let _lock = ELEVATE_LOCK.lock().expect("elevate lock poisoned");
        let auth_ref = get_or_create_auth()?;

        match execute_privileged_copy(auth_ref, src, dst) {
            Ok(()) => Ok(()),
            Err(MacElevateError::AuthExec(status, msg)) if is_auth_stale(status) => {
                tracing::info!("{msg} — re-prompting");
                invalidate_cached_auth();
                let auth_ref = get_or_create_auth()?;
                execute_privileged_copy(auth_ref, src, dst).map_err(mac_elevate_to_apply_error)
            }
            Err(e) => Err(mac_elevate_to_apply_error(e)),
        }
    }

    pub fn elevate_copy(src: &Path, dst: &Path) -> Result<(), ApplyError> {
        match elevate_copy_security(src, dst) {
            Ok(()) => Ok(()),
            Err(ApplyError::Cancelled) => Err(ApplyError::Cancelled),
            Err(e) => {
                tracing::warn!("Security.framework elevation failed: {e}; falling back to osascript");
                osascript_elevate_copy(src, dst)
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn elevate_copy(src: &Path, dst: &Path) -> Result<(), ApplyError> {
    macos::elevate_copy(src, dst)
}

// ---- Linux: pkexec /bin/cp -------------------------------------------------

#[cfg(target_os = "linux")]
fn elevate_copy(src: &Path, dst: &Path) -> Result<(), ApplyError> {
    use std::process::Command;

    let output = Command::new("/usr/bin/pkexec")
        .arg("/bin/cp")
        .arg(src)
        .arg(dst)
        .output()
        .map_err(|e| ApplyError::Elevation(format!("failed to launch pkexec: {e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match code {
        Some(126) => Err(ApplyError::Cancelled),
        Some(127) => Err(ApplyError::Elevation(if stderr.is_empty() {
            "polkit refused the action".to_string()
        } else {
            stderr
        })),
        _ => Err(ApplyError::Elevation(format!(
            "pkexec exit {}: {}",
            code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            stderr
        ))),
    }
}

// ---- Windows: PowerShell UAC -----------------------------------------------

#[cfg(target_os = "windows")]
fn elevate_copy(src: &Path, dst: &Path) -> Result<(), ApplyError> {
    use std::process::Command;

    let from = src.display().to_string();
    let to = dst.display().to_string();
    let ps_script = format!(
        "Start-Process -FilePath 'cmd.exe' -ArgumentList '/c','copy','/Y','{from}','{to}' -Verb RunAs -Wait"
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| ApplyError::Elevation(format!("无法调用 PowerShell：{e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(ApplyError::Elevation(if stderr.is_empty() {
        "UAC 授权失败或被取消".to_string()
    } else {
        stderr
    }))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn elevate_copy(_src: &Path, dst: &Path) -> Result<(), ApplyError> {
    Err(ApplyError::Elevation(format!(
        "当前平台不支持自动提权写入 {}",
        dst.display()
    )))
}
