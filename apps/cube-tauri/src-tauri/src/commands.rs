use serde::Serialize;
use tauri::Emitter;

use cube_cli::{
    desktop_tables_missing, ensure_desktop_tables, ensure_tables_at, facelets_after_moves,
    parse_facelets, scramble_facelets, solve_twophase as search_twophase, to_facelets, CubieCube,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CubeState {
    facelets: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrambleOk {
    moves: Vec<String>,
    facelets: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveOk {
    moves: Vec<String>,
    length: usize,
    optimal: bool,
    elapsed_ms: u128,
}

fn names(moves: &[cube_cli::Move]) -> Vec<String> {
    moves.iter().map(|mv| mv.name().to_string()).collect()
}

#[tauri::command]
pub fn get_solved() -> CubeState {
    CubeState {
        facelets: to_facelets(&CubieCube::solved()),
    }
}

#[tauri::command]
pub fn apply_moves(facelets: String, moves: String) -> Result<CubeState, String> {
    let facelets = facelets_after_moves(&facelets, &moves).map_err(|err| err.to_string())?;
    Ok(CubeState { facelets })
}

#[tauri::command]
pub fn validate_facelets(facelets: String) -> Result<(), String> {
    parse_facelets(&facelets)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn scramble(n: u32) -> ScrambleOk {
    let (moves, facelets) = scramble_facelets(n);
    ScrambleOk {
        moves: names(&moves),
        facelets,
    }
}

#[tauri::command]
pub async fn solve_twophase(app: tauri::AppHandle, facelets: String) -> Result<SolveOk, String> {
    let packaged = packaged_tables(&app)?;
    let generating = match &packaged {
        Some(dir) => !dir.join("manifest.bin").is_file(),
        None => desktop_tables_missing(),
    };
    if generating {
        let message = match &packaged {
            Some(dir) => format!("Generating two-phase tables into {}…", dir.display()),
            None => "Generating two-phase tables into Application Support…".to_string(),
        };
        let _ = app.emit("tables-progress", message);
    }
    let result = tauri::async_runtime::spawn_blocking(move || solve_blocking(facelets, packaged))
        .await
        .map_err(|err| format!("solve task failed: {err}"))?;
    if generating && result.is_ok() {
        let _ = app.emit("tables-done", "Two-phase tables are ready.");
    }
    result
}

/// Packaged builds ignore `$CUBE_TABLES` and the compile-time repo path.
/// `cargo tauri dev` keeps those and returns `None`.
#[cfg(dev)]
fn packaged_tables(_app: &tauri::AppHandle) -> Result<Option<std::path::PathBuf>, String> {
    Ok(None)
}

#[cfg(not(dev))]
fn packaged_tables(app: &tauri::AppHandle) -> Result<Option<std::path::PathBuf>, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("app data directory: {err}"))?
        .join("tables");
    Ok(Some(dir))
}

fn solve_blocking(
    facelets: String,
    packaged: Option<std::path::PathBuf>,
) -> Result<SolveOk, String> {
    let cube = parse_facelets(&facelets).map_err(|err| err.to_string())?;
    let dir = match packaged {
        Some(dir) => ensure_tables_at(&dir).map_err(|err| err.to_string())?,
        None => ensure_desktop_tables().map_err(|err| err.to_string())?,
    };
    let solved = search_twophase(&cube, &dir).map_err(|err| err.to_string())?;
    Ok(SolveOk {
        length: solved.moves.len(),
        optimal: false,
        elapsed_ms: solved.elapsed_ms,
        moves: names(&solved.moves),
    })
}
