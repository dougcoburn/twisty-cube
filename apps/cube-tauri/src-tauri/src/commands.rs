use serde::Serialize;
use tauri::Emitter;
use tauri::Manager;

use cube_cli::{
    desktop_tables_missing, ensure_desktop_tables, facelets_after_moves, parse_facelets,
    scramble_facelets, solve_twophase as search_twophase, to_facelets, CubieCube,
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
    let resources = resource_table_dirs(&app);
    let generating = desktop_tables_missing(&resources);
    if generating {
        let _ = app.emit(
            "tables-progress",
            "Generating two-phase tables into Application Support…",
        );
    }
    let result = tauri::async_runtime::spawn_blocking(move || solve_blocking(facelets, resources))
        .await
        .map_err(|err| format!("solve task failed: {err}"))?;
    if generating && result.is_ok() {
        let _ = app.emit("tables-done", "Two-phase tables are ready.");
    }
    result
}

fn resource_table_dirs(app: &tauri::AppHandle) -> Vec<std::path::PathBuf> {
    match app.path().resource_dir() {
        Ok(dir) => vec![dir.join("tables")],
        Err(_) => Vec::new(),
    }
}

fn solve_blocking(facelets: String, resources: Vec<std::path::PathBuf>) -> Result<SolveOk, String> {
    let cube = parse_facelets(&facelets).map_err(|err| err.to_string())?;
    let dir = ensure_desktop_tables(&resources).map_err(|err| err.to_string())?;
    let solved = search_twophase(&cube, &dir).map_err(|err| err.to_string())?;
    Ok(SolveOk {
        length: solved.moves.len(),
        optimal: false,
        elapsed_ms: solved.elapsed_ms,
        moves: names(&solved.moves),
    })
}
