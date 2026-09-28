mod catalog;
mod clean;
mod env;
mod model;
mod scan;
mod winutil;

use model::{CleanReport, Environment, Preview, ProgressEvent, ScanSummary, Target};
use scan::Guard;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct AppState(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    environment: Environment,
    targets: Vec<Target>,
    plan: Option<clean::Plan>,
    revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    environment: Environment,
    targets: Vec<Target>,
}

#[tauri::command]
async fn snapshot(app: AppHandle, game_path: Option<String>) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut inner = state.0.try_lock().map_err(|_| "busy")?;
        let mut environment = env::detect();
        if let Some(path) = game_path.filter(|p| !p.trim().is_empty()) {
            let path = std::path::Path::new(path.trim());
            if !env::is_game_root(path) {
                return Err("invalid_game_path".into());
            }
            let path = path.canonicalize().map_err(|e| e.to_string())?;
            let path = winutil::display_path(&path);
            if !environment.game_roots.contains(&path) {
                environment.game_roots.push(path);
            }
        }
        inner.targets = catalog::build(&environment);
        inner.environment = environment;
        inner.plan = None;
        Ok(Snapshot {
            environment: inner.environment.clone(),
            targets: inner.targets.clone(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scan_targets(app: AppHandle) -> Result<ScanSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut inner = state.0.try_lock().map_err(|_| "busy")?;
        inner.plan = None;
        let guard = Guard::new(&inner.environment);
        Ok(scan::scan(&app, &mut inner.targets, &guard))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn preview_targets(app: AppHandle, ids: Vec<String>) -> Result<Preview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut inner = state.0.try_lock().map_err(|_| "busy")?;
        inner.plan = None;
        inner.revision += 1;
        let plan = clean::Plan::prepare(
            inner.revision,
            &inner.targets,
            &ids,
            &Guard::new(&inner.environment),
        )?;
        let preview = plan.preview();
        inner.plan = Some(plan);
        Ok(preview)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clean_targets(
    app: AppHandle,
    plan_id: u64,
    confirmed: bool,
) -> Result<CleanReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !confirmed {
            return Err("confirmation_required".into());
        }
        let state = app.state::<AppState>();
        let mut inner = state.0.try_lock().map_err(|_| "busy")?;
        if !winutil::list_watched_processes().is_empty() {
            return Err("close_game_and_launcher".into());
        }
        if inner.plan.as_ref().map(|p| p.id) != Some(plan_id) {
            return Err("preview_expired".into());
        }
        let plan = inner.plan.take().ok_or("preview_expired")?;
        let guard = Guard::new(&inner.environment);
        let mut report = plan.execute(confirmed, &guard, |index, total| {
            if index % 100 == 0 || index == total {
                let _ = app.emit(
                    "wf://progress",
                    ProgressEvent {
                        phase: "clean".into(),
                        target_id: String::new(),
                        path: String::new(),
                        bytes: 0,
                        files: index as u64,
                        index,
                        total,
                        done: index == total,
                        error: None,
                    },
                );
            }
        })?;
        report.targets = scan::scan(&app, &mut inner.targets, &guard).targets;
        Ok(report)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn reveal(app: AppHandle, target_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let inner = state.0.try_lock().map_err(|_| "busy")?;
    let target = inner
        .targets
        .iter()
        .find(|t| t.id == target_id)
        .ok_or("unknown_target")?;
    winutil::reveal_in_explorer(std::path::Path::new(&target.path))
}

#[tauri::command]
fn elevate(app: AppHandle) -> Result<(), String> {
    winutil::elevate_self()?;
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            scan_targets,
            preview_targets,
            clean_targets,
            reveal,
            elevate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
