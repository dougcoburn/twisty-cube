use serde::Serialize;

use cube_cli::{
    facelets_after_moves, find_tables_dir, parse_facelets, scramble_facelets,
    solve_twophase as search_twophase, to_facelets, CubieCube,
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
pub async fn solve_twophase(facelets: String) -> Result<SolveOk, String> {
    tauri::async_runtime::spawn_blocking(move || solve_blocking(facelets))
        .await
        .map_err(|err| format!("solve task failed: {err}"))?
}

fn solve_blocking(facelets: String) -> Result<SolveOk, String> {
    let cube = parse_facelets(&facelets).map_err(|err| err.to_string())?;
    let dir = find_tables_dir().map_err(|err| err.to_string())?;
    let solved = search_twophase(&cube, &dir).map_err(|err| err.to_string())?;
    Ok(SolveOk {
        length: solved.moves.len(),
        optimal: false,
        elapsed_ms: solved.elapsed_ms,
        moves: names(&solved.moves),
    })
}
