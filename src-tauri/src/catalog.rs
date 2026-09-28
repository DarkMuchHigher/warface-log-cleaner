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
        add(
            "upd.gup",
            "updates",
            root,
            "-gup-",
            TargetKind::Contents,
            None,
            true,
        );
        add(
            "cache.shader_pak",
            "caches",
            root,
            "Engine/ShaderCache.pak",
            TargetKind::File,
            None,
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
        add(
            "account.user_cfg",
            "account",
            root,
            "",
            TargetKind::Glob,
            Some("user_*.cfg"),
            true,
        );
        add(
            "account.room_config",
            "account",
            root,
            "pvp_game_room_config.xml",
            TargetKind::File,
            None,
            true,
        );
        add(
            "account.profiles",
            "account",
            root,
            "Profiles",
            TargetKind::Contents,
            None,
            true,
        );
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
            ("gc.time_spent", "Cache/GamesTimeSpent"),
            ("gc.play_games", "Cache/PlayGamesGet"),
            ("gc.cef", "Chrome"),
        ] {
            add(id, "launcher", root, name, TargetKind::Contents, None, true);
        }
        add(
            "gc.avatar",
            "launcher",
            root,
            "Cache/CurrentAvatar.png",
            TargetKind::File,
            None,
            true,
        );
        for (id, name) in [
            ("gc.config_games", "configBigGames.xml"),
            ("gc.config_repository", "configMainRepository.xml"),
            ("gc.config_mirrors", "configMirrors.xml"),
        ] {
            add(id, "launcher", root, name, TargetKind::File, None, true);
        }
        add(
            "account.launcher_ini",
            "account",
            root,
            "GameCenter.ini",
            TargetKind::File,
            None,
            true,
        );
        add(
            "account.launcher_plays",
            "account",
            root,
            "",
            TargetKind::Glob,
            Some("configPlays*.dat"),
            true,
        );
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
        add(
            "upd.packages",
            "updates",
            root,
            "packages",
            TargetKind::Contents,
            None,
            true,
        );
        add(
            "upd.torrents",
            "updates",
            root,
            "torrents",
            TargetKind::Contents,
            None,
            true,
        );
    }
    if let Some(local) = dirs::data_local_dir() {
        let crash = local.join("CrashRpt").join("UnsentCrashReports");
        if crash.is_dir() {
            add(
                "crash.crashrpt",
                "crash",
                &local.to_string_lossy(),
                "CrashRpt/UnsentCrashReports",
                TargetKind::Contents,
                None,
                true,
            );
        }
    }
    targets.sort_by(|a, b| a.id.cmp(&b.id));
    targets
}

/// Targets whose payload legitimately contains otherwise blocked extensions.
fn extension_override(target_id: &str) -> Option<&'static [&'static str]> {
    match target_id.split('@').next().unwrap_or_default() {
        "cache.shader_pak" => Some(&["pak"]),
        "account.user_cfg" => Some(&["cfg"]),
        "account.launcher_ini" => Some(&["ini"]),
        "gc.cef" => Some(&[
            "exe", "dll", "pak", "bin", "dat", "json", "ini", "cfg", "so",
        ]),
        _ => None,
    }
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
    let overridden =
        extension_override(&target.id).is_some_and(|list| list.contains(&extension.as_str()));
    if !overridden
        && matches!(
            extension.as_str(),
            "exe" | "dll" | "sys" | "pak" | "ini" | "cfg" | "lnk" | "bat" | "cmd" | "ps1"
        )
    {
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
            "logs" | "caches" | "crash" | "launcher" | "updates" | "account"
        )));
        assert!(targets
            .iter()
            .any(|t| t.id.starts_with("account.user_cfg@")));
        assert!(targets
            .iter()
            .any(|t| t.id.starts_with("cache.shader_pak@")));
        assert!(targets.iter().any(|t| t.id.starts_with("upd.packages@")));
        assert!(targets.iter().any(|t| t.id.starts_with("gc.cef@")));
        assert!(targets.iter().any(|t| t.id.starts_with("gc.main@")));
        assert!(targets.iter().any(|t| t.id.starts_with("upd.warface@")));
        let ini = targets
            .iter()
            .find(|t| t.id.starts_with("account.launcher_ini@"))
            .unwrap();
        assert!(!ini.default_on && ini.risk == "caution");
        assert!(targets.iter().all(|t| !t.path.contains("Local Storage")
            && !t.path.contains("Session Storage")
            && !t.path.contains("IndexedDB")
            && !t.path.contains("Network")));
        assert!(targets.iter().all(|t| !t.id.starts_with("profile.")));
        assert!(targets
            .iter()
            .filter(|t| t.risk == "caution")
            .all(|t| !t.default_on));
    }

    #[test]
    fn blocked_extensions_open_only_for_their_target() {
        let env = Environment {
            game_roots: vec![r"C:\WF".into()],
            profile_roots: vec![r"C:\Users\Tester\Saved Games\My Games\Warface".into()],
            ..Default::default()
        };
        let targets = build(&env);
        let find = |prefix: &str| targets.iter().find(|t| t.id.starts_with(prefix)).unwrap();
        let shader = find("cache.shader_pak@");
        assert!(permits(shader, &PathBuf::from(&shader.path)));
        let backups = find("log.backups@");
        assert!(!permits(
            backups,
            &Path::new(&backups.path).join("payload.pak")
        ));
        assert!(!permits(
            backups,
            &Path::new(&backups.path).join("payload.dll")
        ));
        let cfg = find("account.user_cfg@");
        assert!(permits(cfg, &Path::new(&cfg.path).join("user_1.cfg")));
        assert!(!permits(cfg, &Path::new(&cfg.path).join("user_1.exe")));
        let shaders = find("cache.shaders@");
        assert!(!permits(
            shaders,
            &Path::new(&shaders.path).join("Engine.dll")
        ));
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
        let own: Vec<_> = targets
            .iter()
            .filter(|t| Path::new(&t.path).starts_with(r"C:\WF"))
            .collect();
        assert_eq!(own.len(), 8);
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
