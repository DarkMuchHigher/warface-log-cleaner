use std::path::{Path, PathBuf};

use crate::model::Environment;
use crate::winutil;

const GAME_EXE: &str = "Game.exe";
const PROTOCOLS: [&str; 3] = ["vkplay", "mailrugames", "mygames"];

pub fn detect() -> Environment {
    let mut env = Environment {
        user: current_user(),
        is_admin: winutil::is_admin(),
        ..Default::default()
    };

    let saved_games = winutil::saved_games_dir().map(display);
    match saved_games.as_ref() {
        Some(saved) => note(&mut env, format!("saved games: {}", saved)),
        None => note(&mut env, "saved games folder not found".into()),
    }
    env.saved_games = saved_games;

    let gamecenter_exe = PROTOCOLS
        .iter()
        .find_map(|protocol| winutil::protocol_exe(protocol))
        .map(display);
    if let Some(exe) = gamecenter_exe.as_ref() {
        note(&mut env, format!("gamecenter: {}", exe));
    }

    let local = dirs::data_local_dir();
    let gamecenter_dir = local
        .as_ref()
        .map(|base| base.join("GameCenter"))
        .filter(|path| path.is_dir())
        .map(display)
        .or_else(|| {
            gamecenter_exe
                .as_ref()
                .and_then(|exe| Path::new(exe).parent())
                .map(|parent| display(parent.to_path_buf()))
        });
    env.gamecenter_exe = gamecenter_exe;
    env.gamecenter_dir = gamecenter_dir;

    if let Some(ini) = env
        .gamecenter_dir
        .as_ref()
        .map(|dir| Path::new(dir).join("GameCenter.ini"))
        .filter(|path| path.is_file())
    {
        let values = parse_utf16_ini(&ini);
        let last_value = |name: &str| {
            values
                .iter()
                .rev()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.clone())
                .filter(|value| !value.trim().is_empty())
        };
        let download = last_value("DownloadPath");
        if let Some(path) = download.as_ref() {
            note(&mut env, format!("download path: {}", path));
        }
        env.download_path = download;
        env.games_install_path = last_value("GamesInstallPath");
    }

    let game_roots = detect_game_roots(&env, local.as_deref());
    for root in game_roots.iter() {
        note(&mut env, format!("game root: {}", root));
    }
    if game_roots.is_empty() {
        note(&mut env, "game installation not found".into());
    }
    env.game_roots = game_roots;

    let profile_roots = detect_profile_roots(&env);
    for root in profile_roots.iter() {
        note(&mut env, format!("profile: {}", root));
    }
    env.profile_roots = profile_roots;

    env.running = winutil::list_watched_processes();
    env.free_space = env
        .game_roots
        .first()
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .and_then(|path| winutil::free_space(&path));

    env
}

fn current_user() -> String {
    std::env::var("USERNAME")
        .ok()
        .filter(|name| !name.is_empty())
        .or_else(|| {
            dirs::home_dir().and_then(|home| {
                home.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
        })
        .unwrap_or_else(|| "unknown".into())
}

fn note(env: &mut Environment, message: String) {
    env.notes.push(message);
}

fn display(path: PathBuf) -> String {
    winutil::display_path(&path)
}

fn is_bin_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("bin") && lower.ends_with("release")
}

/// A directory is a Warface installation when it holds the launcher executable
/// or the packed game data next to the engine folder.
pub fn is_game_root(path: &Path) -> bool {
    path.join("Bin64Release").join(GAME_EXE).is_file()
        && path.join("Game").join("GameData.pak").is_file()
        && path.join("Engine").is_dir()
}

fn normalize_game_root(exe: &Path) -> Option<PathBuf> {
    let parent = exe.parent()?;
    let root = if is_bin_dir(&parent.file_name()?.to_string_lossy()) {
        parent.parent()?.to_path_buf()
    } else {
        parent.to_path_buf()
    };
    is_game_root(&root).then_some(root)
}

fn detect_game_roots(env: &Environment, local: Option<&Path>) -> Vec<String> {
    let mut roots: Vec<PathBuf> = Vec::new();

    if let Some(exe) = winutil::process_cmdline(GAME_EXE) {
        if let Some(root) = normalize_game_root(&exe) {
            roots.push(root);
        }
    }

    if let Some(saved) = env.saved_games.as_ref().map(PathBuf::from) {
        for base in [
            saved.join("Warface"),
            saved.join("My Games").join("Warface"),
        ] {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && is_game_root(&path) {
                        roots.push(path);
                    }
                }
            }
            if is_game_root(&base) {
                roots.push(base);
            }
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(install) = env.games_install_path.as_ref().map(PathBuf::from) {
        if let Ok(entries) = std::fs::read_dir(&install) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if is_game_root(&path) {
                    candidates.push(path.clone());
                }
                if let Ok(children) = std::fs::read_dir(&path) {
                    for child in children.flatten() {
                        let child_path = child.path();
                        if child_path.is_dir() && is_game_root(&child_path) {
                            candidates.push(child_path);
                        }
                    }
                }
            }
        }
    }
    if let Some(local) = local {
        candidates.push(local.join("Warface"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Warface"));
    }
    candidates.push(PathBuf::from(r"C:\Wf\Warface"));
    candidates.push(PathBuf::from(r"C:\Games\Warface"));
    candidates.push(PathBuf::from(r"C:\Warface"));
    roots.extend(candidates.into_iter().filter(|path| is_game_root(path)));

    let mut unique: Vec<PathBuf> = Vec::new();
    for root in roots {
        let canonical = root.canonicalize().unwrap_or(root);
        if !unique.iter().any(|known| known == &canonical) {
            unique.push(canonical);
        }
    }
    unique.into_iter().map(display).collect()
}

fn detect_profile_roots(env: &Environment) -> Vec<String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(saved) = env.saved_games.as_ref().map(PathBuf::from) {
        let my_games = saved.join("My Games");
        if let Ok(entries) = std::fs::read_dir(&my_games) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if name != "warface" {
                    continue;
                }
                if has_profile_data(&path) {
                    roots.push(path);
                }
            }
        }
    }
    roots.into_iter().map(display).collect()
}

fn has_profile_data(path: &Path) -> bool {
    [
        "modelscache",
        "BannersCache",
        "headscache",
        "QueryCache",
        "Shaders",
        "game.cfg",
    ]
    .iter()
    .any(|name| path.join(name).exists())
}

/// `GameCenter.ini` is UTF-16LE; keys can repeat, later values win.
fn parse_utf16_ini(path: &Path) -> Vec<(String, String)> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let text = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };

    main_paths(&text)
}

fn main_paths(text: &str) -> Vec<(String, String)> {
    let mut in_main = false;
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('[') {
                in_main = line.eq_ignore_ascii_case("[Main]");
                return None;
            }
            if !in_main || line.starts_with(';') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            let value = value.trim();
            if !matches!(
                key.to_ascii_lowercase().as_str(),
                "gamesinstallpath" | "downloadpath"
            ) || !Path::new(value).is_absolute()
            {
                return None;
            }
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ini_reads_only_main_path_keys() {
        let paths = main_paths("[Other]\nGamesInstallPath=C:\\wrong\n[Main]\nDownloadPath=C:\\downloads\nGamesInstallPath=C:\\games\nAccount=private\n[Other]\nGamesInstallPath=C:\\wrong");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[1].1, r"C:\games");
    }
    #[test]
    fn ini_rejects_relative_paths() {
        assert!(main_paths("[Main]\nGamesInstallPath=..\\other").is_empty());
    }
}
