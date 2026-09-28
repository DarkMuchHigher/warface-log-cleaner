use std::os::windows::io::AsRawHandle;
use std::time::{Duration, Instant};
use windows_sys::Win32::Storage::FileSystem::{
    FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
};

use crate::model::{CleanError, CleanReport, PlannedFile, Preview, Target};
use crate::scan::{self, Entry, Guard};

pub struct Plan {
    pub id: u64,
    created: Instant,
    entries: Vec<Entry>,
    errors: Vec<CleanError>,
}

pub fn validate_selection(targets: &[Target], ids: &[String]) -> Result<Vec<Target>, String> {
    if ids.is_empty() {
        return Err("empty_selection".into());
    }
    if ids.iter().any(|id| !targets.iter().any(|t| &t.id == id)) {
        return Err("unknown_target".into());
    }
    Ok(targets
        .iter()
        .filter(|t| ids.contains(&t.id))
        .cloned()
        .collect())
}

impl Plan {
    pub fn prepare(
        id: u64,
        targets: &[Target],
        ids: &[String],
        guard: &Guard,
    ) -> Result<Self, String> {
        let selected = validate_selection(targets, ids)?;
        let (entries, errors) = scan::inventory(&selected, guard);
        Ok(Self {
            id,
            created: Instant::now(),
            entries,
            errors,
        })
    }

    pub fn preview(&self) -> Preview {
        Preview {
            plan_id: self.id,
            bytes: self.entries.iter().map(|e| e.identity.size).sum(),
            files: self
                .entries
                .iter()
                .map(|e| PlannedFile {
                    path: e.path.to_string_lossy().into_owned(),
                    size: e.identity.size,
                    target_id: e.target.id.clone(),
                })
                .collect(),
            errors: self.errors.clone(),
        }
    }

    pub fn execute(
        self,
        confirmed: bool,
        guard: &Guard,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<CleanReport, String> {
        if !confirmed {
            return Err("confirmation_required".into());
        }
        if self.created.elapsed() > Duration::from_secs(300) {
            return Err("preview_expired".into());
        }
        let started = Instant::now();
        let mut report = CleanReport::default();
        let total = self.entries.len();
        report.skipped = self.errors.len() as u64;
        report.errors = self.errors;
        for (index, entry) in self.entries.into_iter().enumerate() {
            match delete_file(&entry, guard) {
                Ok(()) => {
                    report.bytes += entry.identity.size;
                    report.files += 1;
                }
                Err(message) => {
                    report.skipped += 1;
                    report.errors.push(CleanError {
                        path: entry.path.to_string_lossy().into_owned(),
                        message,
                    });
                }
            }
            progress(index + 1, total);
        }
        report.duration_ms = started.elapsed().as_millis() as u64;
        Ok(report)
    }
}

fn delete_file(entry: &Entry, guard: &Guard) -> Result<(), String> {
    guard.ensure_deletable(&entry.target, &entry.path)?;
    let (_parents, file) = scan::open_locked(&entry.path, true)?;
    if scan::identity(&file)? != entry.identity {
        return Err("file_changed".into());
    }
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    if unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog, model::Environment};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        targets: Vec<Target>,
        guard: Guard,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "wlc-test-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(root.join("LogBackups")).unwrap();
            std::fs::write(root.join("Game.log"), b"log").unwrap();
            std::fs::write(root.join("game.cfg"), b"settings").unwrap();
            let env = Environment {
                game_roots: vec![root.to_string_lossy().into_owned()],
                ..Default::default()
            };
            Self {
                targets: catalog::build(&env),
                guard: Guard::new(&env),
                root,
            }
        }
        fn prepare(&self) -> Plan {
            Plan::prepare(
                1,
                &self.targets,
                &[self
                    .targets
                    .iter()
                    .find(|t| t.id.starts_with("log.game@"))
                    .unwrap()
                    .id
                    .clone()],
                &self.guard,
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn empty_and_unknown_selection_fail_closed() {
        let f = Fixture::new();
        assert!(Plan::prepare(1, &f.targets, &[], &f.guard).is_err());
        assert!(Plan::prepare(1, &f.targets, &["temp.user".into()], &f.guard).is_err());
    }
    #[test]
    fn preview_does_not_delete_and_requires_confirmation() {
        let f = Fixture::new();
        let plan = f.prepare();
        assert_eq!(plan.preview().bytes, 3);
        assert!(plan.execute(false, &f.guard, |_, _| {}).is_err());
        assert!(f.root.join("Game.log").exists());
    }
    #[test]
    fn confirmed_plan_preserves_settings_and_new_files() {
        let f = Fixture::new();
        let plan = f.prepare();
        std::fs::write(f.root.join("LogBackups/new.log"), b"new").unwrap();
        let report = plan.execute(true, &f.guard, |_, _| {}).unwrap();
        assert_eq!(report.files, 1);
        assert!(!f.root.join("Game.log").exists());
        assert!(f.root.join("game.cfg").exists());
        assert!(f.root.join("LogBackups/new.log").exists());
    }
    #[test]
    fn changed_file_is_skipped() {
        let f = Fixture::new();
        let plan = f.prepare();
        std::fs::write(f.root.join("Game.log"), b"changed").unwrap();
        let report = plan.execute(true, &f.guard, |_, _| {}).unwrap();
        assert_eq!(report.files, 0);
        assert_eq!(report.skipped, 1);
    }
    #[test]
    fn hard_links_are_not_planned() {
        let f = Fixture::new();
        std::fs::hard_link(f.root.join("Game.log"), f.root.join("other.log")).unwrap();
        assert!(f.prepare().preview().files.is_empty());
    }
    #[test]
    fn locked_file_is_not_planned() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = Fixture::new();
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(f.root.join("Game.log"))
            .unwrap();
        assert!(f.prepare().preview().files.is_empty());
    }
    #[test]
    fn junction_cannot_redirect_cleanup() {
        let f = Fixture::new();
        let outside = Fixture::new();
        let junction = f.root.join("LogBackups").join("redirect");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside.root)
            .output()
            .unwrap();
        assert!(status.status.success());
        assert!(scan::open_locked(&junction.join("Game.log"), false).is_err());
        assert!(outside.root.join("Game.log").exists());
        std::fs::remove_dir(junction).unwrap();
    }
    #[test]
    fn preview_expiration_preserves_files() {
        let f = Fixture::new();
        let mut plan = f.prepare();
        plan.created = Instant::now() - Duration::from_secs(301);
        assert!(plan.execute(true, &f.guard, |_, _| {}).is_err());
        assert!(f.root.join("Game.log").exists());
    }
    #[test]
    fn executable_in_cache_is_excluded() {
        let f = Fixture::new();
        std::fs::create_dir(f.root.join("modelscache")).unwrap();
        std::fs::write(f.root.join("modelscache/keep.exe"), b"keep").unwrap();
        std::fs::write(f.root.join("modelscache/mesh.cac"), b"cache").unwrap();
        let env = Environment {
            profile_roots: vec![f.root.to_string_lossy().into_owned()],
            ..Default::default()
        };
        let targets = catalog::build(&env);
        let ids = targets
            .iter()
            .filter(|t| PathBuf::from(&t.path).starts_with(&f.root))
            .map(|t| t.id.clone())
            .collect::<Vec<_>>();
        let plan = Plan::prepare(1, &targets, &ids, &Guard::new(&env)).unwrap();
        assert_eq!(plan.preview().files.len(), 1);
        assert!(plan.preview().files[0].path.ends_with("mesh.cac"));
    }
    #[test]
    fn replaced_file_is_skipped() {
        let f = Fixture::new();
        let plan = f.prepare();
        std::fs::rename(f.root.join("Game.log"), f.root.join("old.log")).unwrap();
        std::fs::write(f.root.join("Game.log"), b"log").unwrap();
        assert_eq!(plan.execute(true, &f.guard, |_, _| {}).unwrap().files, 0);
    }
}
