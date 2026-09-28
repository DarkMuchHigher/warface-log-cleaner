use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use winreg::enums::*;
use winreg::RegKey;

use crate::model::RunningProcess;

pub const WATCHED_PROCESSES: [&str; 4] = [
    "Game.exe",
    "GameCenter.exe",
    "GameCenterHelper.exe",
    "WfLauncher.exe",
];

/// True when the current process runs with an elevated token.
pub fn is_admin() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// Relaunch this executable through the UAC "runas" verb.
pub fn elevate_self() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };

    let file: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb = wide("runas");

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if result as isize > 32 {
        Ok(())
    } else {
        Err(format!(
            "ShellExecuteW(runas) failed with code {}",
            result as isize
        ))
    }
}

fn expand_env_vars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        match tail.find('%') {
            Some(end) => {
                let name = &tail[..end];
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &tail[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

fn read_string(root: RegKey, subkey: &str, value: Option<&str>) -> Option<String> {
    let key = root.open_subkey(subkey).ok()?;
    match value {
        Some(name) => key.get_value::<String, _>(name).ok(),
        None => key.get_value::<String, _>("").ok(),
    }
}

/// Resolve a protocol handler (`vkplay://`, `mailrugames://`, ...) to the executable path.
pub fn protocol_exe(protocol: &str) -> Option<PathBuf> {
    let subkey = format!("{}\\shell\\open\\command", protocol);
    let candidates = [
        (RegKey::predef(HKEY_CLASSES_ROOT), subkey.clone()),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            format!("SOFTWARE\\Classes\\{}", subkey),
        ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            format!("SOFTWARE\\WOW6432Node\\Classes\\{}", subkey),
        ),
        (
            RegKey::predef(HKEY_CURRENT_USER),
            format!("SOFTWARE\\Classes\\{}", subkey),
        ),
    ];

    for (root, path) in candidates {
        if let Some(command) = read_string(root, &path, None) {
            if let Some(exe) = parse_command_exe(&command) {
                return Some(exe);
            }
        }
    }
    None
}

fn parse_command_exe(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if let Some(rest) = command.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let cut = command
        .find(" -")
        .or_else(|| command.find(" /"))
        .unwrap_or(command.len());
    let path = PathBuf::from(command[..cut].trim());
    (!path.as_os_str().is_empty()).then_some(path)
}

/// "Saved Games" is a known folder and can be relocated (OneDrive, another drive).
pub fn saved_games_dir() -> Option<PathBuf> {
    const GUID: &str = "{4C5C32FF-BB9D-43B0-B5B4-2D72E54EAAA4}";
    if let Some(raw) = read_string(
        RegKey::predef(HKEY_CURRENT_USER),
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\User Shell Folders",
        Some(GUID),
    ) {
        let path = PathBuf::from(expand_env_vars(&raw));
        if path.is_dir() {
            return Some(path);
        }
    }
    let fallback = dirs::home_dir()?.join("Saved Games");
    fallback.is_dir().then_some(fallback)
}

fn scan_processes() -> sysinfo::System {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
}

fn matches_watched(name: &str) -> bool {
    WATCHED_PROCESSES
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

pub fn list_watched_processes() -> Vec<RunningProcess> {
    let system = scan_processes();
    system
        .processes()
        .values()
        .filter_map(|process| {
            let name = process.name().to_string_lossy().to_string();
            if !matches_watched(&name) {
                return None;
            }
            Some(RunningProcess {
                name,
                pid: process.pid().as_u32(),
                exe: process
                    .exe()
                    .map(|path| path.to_string_lossy().to_string())
                    .or_else(|| {
                        process
                            .cmd()
                            .first()
                            .map(|arg| arg.to_string_lossy().to_string())
                    }),
            })
        })
        .collect()
}

/// `canonicalize` returns verbatim paths (`\\?\C:\...`) which look alien in the UI
/// and are rejected by some shell APIs.
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text.to_string()
    }
}

pub fn reveal_in_explorer(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || !path.exists() {
        return Err("path_not_found".into());
    }
    let windows = std::env::var_os("SystemRoot").ok_or("windows_path_missing")?;
    let mut command = Command::new(PathBuf::from(windows).join("explorer.exe"));
    if path.is_file() {
        command.arg(format!("/select,{}", path.display()));
    } else {
        command.arg(path);
    }
    command.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// First command line argument of a running process, if any.
pub fn process_cmdline(exe_name: &str) -> Option<PathBuf> {
    let system = scan_processes();
    system
        .processes()
        .values()
        .filter(|process| {
            process
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case(exe_name)
        })
        .find_map(|process| {
            process
                .exe()
                .map(PathBuf::from)
                .or_else(|| process.cmd().first().map(PathBuf::from))
        })
}

/// Free space (bytes) on the volume that hosts `path`.
pub fn free_space(path: &Path) -> Option<u64> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let target = PathBuf::from(display_path(
        &path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
    ));
    let mut best: Option<(usize, u64)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if target.starts_with(mount) {
            let depth = mount.components().count();
            if best.map(|(known, _)| depth > known).unwrap_or(true) {
                best = Some((depth, disk.available_space()));
            }
        }
    }
    best.map(|(_, free)| free)
}
