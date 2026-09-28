use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::os::windows::{
    fs::{MetadataExt, OpenOptionsExt},
    io::AsRawHandle,
};
use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;
use windows_sys::Win32::Storage::FileSystem::*;

use crate::catalog;
use crate::model::{CleanError, Environment, ProgressEvent, ScanSummary, Target, TargetKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub volume: u32,
    pub index: u64,
    pub size: u64,
    pub modified: u64,
}

pub fn identity(file: &File) -> Result<Identity, String> {
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let info = unsafe { info.assume_init() };
    if info.nNumberOfLinks != 1 {
        return Err("hard_link".into());
    }
    Ok(Identity {
        volume: info.dwVolumeSerialNumber,
        index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        modified: (u64::from(info.ftLastWriteTime.dwHighDateTime) << 32)
            | u64::from(info.ftLastWriteTime.dwLowDateTime),
    })
}

pub fn open_locked(path: &Path, delete: bool) -> Result<(Vec<File>, File), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err("invalid_path".into());
    }
    if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_)))
    {
        return Err("local_paths_only".into());
    }
    let mut parents = Vec::new();
    let mut ancestors: Vec<_> = path.ancestors().skip(1).collect();
    ancestors.reverse();
    for parent in ancestors {
        let handle = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(parent)
            .map_err(|e| e.to_string())?;
        if handle
            .metadata()
            .map_err(|e| e.to_string())?
            .file_attributes()
            & FILE_ATTRIBUTE_REPARSE_POINT
            != 0
        {
            return Err("reparse_point".into());
        }
        parents.push(handle);
    }
    let file = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ | if delete { DELETE } else { 0 })
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("not_regular_file".into());
    }
    Ok((parents, file))
}

/// Top-level entries a target is responsible for.
pub fn collect_items(target: &Target) -> Result<Vec<PathBuf>, String> {
    let path = PathBuf::from(&target.path);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
        Ok(meta) if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => {
            return Err("reparse_point".into())
        }
        _ => {}
    }
    if target.kind == TargetKind::File {
        return Ok(vec![path]);
    }
    let depth = if target.kind == TargetKind::Glob {
        1
    } else {
        24
    };
    let mut items = Vec::new();
    for entry in WalkDir::new(path)
        .follow_links(false)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|e| {
            e.metadata()
                .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0)
        })
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() && catalog::permits(target, entry.path()) {
            items.push(entry.into_path());
        }
        if items.len() > 100_000 {
            return Err("too_many_files".into());
        }
    }
    items.sort();
    Ok(items)
}

/// Size in bytes and number of files (directories are not counted).
pub fn measure_path(path: &Path) -> Result<Identity, String> {
    let (_parents, file) = open_locked(path, false)?;
    identity(&file)
}

#[derive(Clone)]
pub struct Entry {
    pub target: Target,
    pub path: PathBuf,
    pub identity: Identity,
}

pub fn inventory(targets: &[Target], guard: &Guard) -> (Vec<Entry>, Vec<CleanError>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for target in targets {
        let items = match collect_items(target) {
            Ok(items) => items,
            Err(message) => {
                errors.push(CleanError {
                    path: target.path.clone(),
                    message,
                });
                continue;
            }
        };
        for path in items {
            if !seen.insert(path.to_string_lossy().to_lowercase()) {
                continue;
            }
            match guard
                .ensure_deletable(target, &path)
                .and_then(|()| measure_path(&path))
            {
                Ok(identity) => files.push(Entry {
                    target: target.clone(),
                    path,
                    identity,
                }),
                Err(message) => errors.push(CleanError {
                    path: path.to_string_lossy().into_owned(),
                    message,
                }),
            }
        }
    }
    (files, errors)
}

pub fn scan(app: &AppHandle, targets: &mut [Target], guard: &Guard) -> ScanSummary {
    let started = std::time::Instant::now();
    let (entries, errors) = inventory(targets, guard);
    let total = targets.len();
    for (index, target) in targets.iter_mut().enumerate() {
        let matching: Vec<_> = entries
            .iter()
            .filter(|e| e.target.id == target.id)
            .collect();
        target.size = matching.iter().map(|e| e.identity.size).sum();
        target.files = matching.len() as u64;
        target.exists = target.files > 0;
        target.scanned = true;
        let _ = app.emit(
            "wf://progress",
            ProgressEvent {
                phase: "scan".into(),
                target_id: target.id.clone(),
                path: target.path.clone(),
                bytes: target.size,
                files: target.files,
                index: index + 1,
                total,
                done: true,
                error: None,
            },
        );
    }
    ScanSummary {
        total_bytes: entries.iter().map(|e| e.identity.size).sum(),
        total_files: entries.len() as u64,
        targets: targets.to_vec(),
        duration_ms: started.elapsed().as_millis() as u64,
        errors,
    }
}

/// Paths that must never be deleted, even if a catalog entry points at them.
pub struct Guard {
    allowed: Vec<Target>,
}

impl Guard {
    pub fn new(env: &Environment) -> Self {
        Self {
            allowed: catalog::build(env),
        }
    }

    pub fn ensure_deletable(&self, target: &Target, path: &Path) -> Result<(), String> {
        if !self.allowed.iter().any(|t| {
            t.id == target.id
                && t.path == target.path
                && t.kind == target.kind
                && t.pattern == target.pattern
        }) || !catalog::permits(target, path)
        {
            return Err("outside_allowlist".into());
        }
        Ok(())
    }
}
