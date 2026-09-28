use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningProcess {
    pub name: String,
    pub pid: u32,
    pub exe: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub user: String,
    pub is_admin: bool,
    pub saved_games: Option<String>,
    pub game_roots: Vec<String>,
    pub profile_roots: Vec<String>,
    pub gamecenter_dir: Option<String>,
    pub gamecenter_exe: Option<String>,
    pub download_path: Option<String>,
    pub games_install_path: Option<String>,
    pub running: Vec<RunningProcess>,
    pub free_space: Option<u64>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    /// Single file.
    File,
    /// Directory removed together with its contents.
    Dir,
    /// Contents of a directory are removed, the directory itself stays.
    Contents,
    /// Every entry of a directory matching `pattern` (files and directories).
    Glob,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub group: String,
    /// For `Glob` this is the directory to look into, otherwise the path itself.
    pub path: String,
    pub pattern: Option<String>,
    pub kind: TargetKind,
    /// `safe` or `caution`.
    pub risk: String,
    pub admin: bool,
    pub default_on: bool,
    pub exists: bool,
    pub size: u64,
    pub files: u64,
    pub scanned: bool,
    /// Extra context for the UI (which game root this belongs to, etc).
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: String,
    pub target_id: String,
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    pub index: usize,
    pub total: usize,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub targets: Vec<Target>,
    pub total_bytes: u64,
    pub total_files: u64,
    pub duration_ms: u64,
    pub errors: Vec<CleanError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub dry_run: bool,
    pub bytes: u64,
    pub files: u64,
    pub dirs: u64,
    pub skipped: u64,
    pub killed: Vec<RunningProcess>,
    pub errors: Vec<CleanError>,
    pub targets: Vec<Target>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub path: String,
    pub size: u64,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub plan_id: u64,
    pub files: Vec<PlannedFile>,
    pub bytes: u64,
    pub errors: Vec<CleanError>,
}
