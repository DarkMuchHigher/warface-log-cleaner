use std::path::{Path, PathBuf};

use crate::model::{Environment, Target, TargetKind};

pub fn build(env: &Environment) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut add =
        |id: &str, group: &str, root: &str, name: &str, kind, pattern: Option<&str>, caution| {
            let path = Path::new(root).join(name);
            if !path.is_absolute() {
                return;
            }
            let id = format!("{}@{}", id, root.to_lowercase());
            if targets.iter().any(|t: &Target| t.id == id) {
                return;
            }
            targets.push(Target {
                id,
                group: group.into(),
                path: path.to_string_lossy().into_owned(),
                pattern: pattern.map(str::to_owned),
                kind,
                risk: if caution { "caution" } else { "safe" }.into(),
                admin: false,
                default_on: !caution,
                exists: path.exists(),
                size: 0,
                files: 0,
                scanned: false,
                note: None,
            });
        };
    for root in &env.game_roots {
        for (id, name) in [
            ("log.game", "Game.log"),
            ("log.action_history", "action_history.log"),
            ("log.server_profile", "server_profile.txt"),
            ("log.process", "process.txt"),
            ("log.updater", "-gup-/install.log"),
        ] {
            add(id, "logs", root, name, TargetKind::File, None, false);
        }
        add(
            "log.backups",
            "logs",
            root,
            "LogBackups",
            TargetKind::Glob,
            Some("*.log"),
            false,
        );
        add(
            "crash.dumps",
            "crash",
            root,
            "",
            TargetKind::Glob,
            Some("*.dmp"),
            true,
        );
    }
    for root in &env.profile_roots {
        for (id, name) in [
            ("cache.models", "modelscache"),
            ("cache.banners", "BannersCache"),
            ("cache.heads", "headscache"),
            ("cache.query", "QueryCache"),
            ("cache.shaders", "Shaders"),
        ] {
            add(id, "caches", root, name, TargetKind::Contents, None, true);
        }
    }
    if let Some(root) = &env.gamecenter_dir {
        for (id, name) in [("gc.main", "main.log"), ("gc.chrome_log", "Chrome.log")] {
            add(id, "launcher", root, name, TargetKind::File, None, true);
        }
        add(
            "gc.dumps",
            "launcher",
            root,
            "",
            TargetKind::Glob,
            Some("sdump*.dmp"),
            true,
        );
        for (id, name) in [
            ("gc.images", "Cache/Big.Img"),
            ("gc.common", "Cache/Common"),
            ("gc.alerts", "Cache/Alerts"),
            ("gc.descriptions", "Cache/GameDescription"),
            ("gc.patches", "Cache/GamesPatchList"),
            ("gc.browser_cache", "Cache/Chrome/Cache"),
            ("gc.browser_code", "Cache/Chrome/Code Cache"),
            ("gc.browser_gpu", "Cache/Chrome/GPUCache"),
            ("gc.browser_dawn", "Cache/Chrome/DawnCache"),
            ("gc.user_cache", "Cache/ChromeUser/Cache"),
            ("gc.user_code", "Cache/ChromeUser/Code Cache"),
            ("gc.user_gpu", "Cache/ChromeUser/GPUCache"),
        ] {
            add(id, "launcher", root, name, TargetKind::Contents, None, true);
        }
        for (id, name) in [
            ("gc.config_games", "configBigGames.xml"),
            ("gc.config_repository", "configMainRepository.xml"),
            ("gc.config_mirrors", "configMirrors.xml"),
        ] {
            add(id, "launcher", root, name, TargetKind::File, None, true);
        }
    }
    if let Some(root) = &env.download_path {
        add(
            "upd.warface",
            "updates",
            root,
            "packages/warface",
            TargetKind::Contents,
            None,
            true,
        );
    }
    targets.sort_by(|a, b| a.id.cmp(&b.id));
    targets
}

pub fn permits(target: &Target, path: &Path) -> bool {
    let root = PathBuf::from(&target.path);
    let Ok(relative) = path.strip_prefix(&root) else {
        return false;
    };
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return false;
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "exe" | "dll" | "sys" | "pak" | "ini" | "cfg" | "lnk" | "bat" | "cmd" | "ps1"
    ) {
        return false;
    }
    match target.kind {
        TargetKind::File => path == root,
        TargetKind::Glob => {
            path.parent() == Some(root.as_path())
                && wildcard_match(
                    target.pattern.as_deref().unwrap_or(""),
                    &path.file_name().unwrap_or_default().to_string_lossy(),
                )
        }
        TargetKind::Contents | TargetKind::Dir => !relative.as_os_str().is_empty(),
    }
}

/// Case-insensitive wildcard match supporting `*` and `?`.
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let name: Vec<char> = name.to_ascii_lowercase().chars().collect();
    let (mut p, mut n) = (0, 0);
    let mut star = None;
    let mut mark = 0;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = n;
            p += 1;
        } else if let Some(position) = star {
            p = position + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_launcher_but_not_global_temp() {
        let env = Environment {
            game_roots: vec![r"C:\Games\Warface".into()],
            profile_roots: vec![r"C:\Users\Tester\Saved Games\My Games\Warface".into()],
            gamecenter_dir: Some(r"C:\Launcher".into()),
            download_path: Some(r"C:\Downloads".into()),
            ..Default::default()
        };
        let targets = build(&env);
        assert!(!targets.is_empty());
        assert!(targets.iter().all(|t| matches!(
            t.group.as_str(),
            "logs" | "caches" | "crash" | "launcher" | "updates"
        )));
        assert!(targets.iter().any(|t| t.id.starts_with("gc.main@")));
        assert!(targets.iter().any(|t| t.id.starts_with("upd.warface@")));
        assert!(targets
            .iter()
            .all(|t| !t.path.ends_with("GameCenter.ini") && !t.path.ends_with("Chrome")));
        assert!(targets.iter().all(|t| !t.id.starts_with("profile.")));
        assert!(targets
            .iter()
            .filter(|t| t.risk == "caution")
            .all(|t| !t.default_on));
    }

    #[test]
    fn wildcard_matching_is_anchored() {
        assert!(wildcard_match("*.log", "GAME.LOG"));
        assert!(!wildcard_match("*.log", "Game.log.exe"));
        assert!(wildcard_match("a?c*", "Abcdef"));
        assert!(!wildcard_match("a?c", "ac"));
    }

    #[test]
    fn duplicates_and_traversal_are_rejected() {
        let env = Environment {
            game_roots: vec![r"C:\WF".into(), r"C:\WF".into()],
            ..Default::default()
        };
        let targets = build(&env);
        assert_eq!(targets.len(), 7);
        let logs = targets
            .iter()
            .find(|t| t.id.starts_with("log.backups@"))
            .unwrap();
        assert!(!permits(logs, &Path::new(&logs.path).join("../other.log")));
        assert!(!permits(
            logs,
            &Path::new(&logs.path).join("nested/test.log")
        ));
        assert!(permits(logs, &Path::new(&logs.path).join("test.log")));
    }
}
