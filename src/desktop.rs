//! Facelet, scramble, and two-phase helpers for the desktop shell.
//!
//! The solver itself stays in [`crate::solve`]. This module only adapts it:
//! string in, string out, and a hard error when `tables/manifest.bin` is absent.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::moves::{move_allowed, parse_moves, Move, ALL_MOVES};
use crate::solve::{solve, SolveError};
use crate::tables::load_or_generate;
use crate::{cubie_from_facelets, facelets_from_cubie, CubeError, CubieCube};

/// Two-phase result. `optimal` is always false; this is not a shortest-path proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwophaseSolution {
    pub moves: Vec<Move>,
    pub elapsed_ms: u128,
    pub optimal: bool,
}

#[derive(Debug)]
pub enum DesktopError {
    Illegal(CubeError),
    Moves(String),
    NoSolution,
    Rejected,
    /// `manifest.bin` is missing, or the files on disk could not be mapped.
    MissingTables(String),
}

impl std::fmt::Display for DesktopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopError::Illegal(err) => write!(f, "{err}"),
            DesktopError::Moves(err) => write!(f, "{err}"),
            DesktopError::NoSolution => write!(f, "no solution of length <= 20"),
            DesktopError::Rejected => write!(f, "solution failed replay against the original cube"),
            DesktopError::MissingTables(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DesktopError {}

impl From<SolveError> for DesktopError {
    fn from(err: SolveError) -> Self {
        match err {
            SolveError::Illegal(err) => DesktopError::Illegal(err),
            SolveError::NoSolution => DesktopError::NoSolution,
            SolveError::Rejected => DesktopError::Rejected,
        }
    }
}

pub fn parse_facelets(text: &str) -> Result<CubieCube, CubeError> {
    cubie_from_facelets(text)
}

pub fn to_facelets(cube: &CubieCube) -> String {
    facelets_from_cubie(cube)
}

pub fn solved() -> CubieCube {
    CubieCube::solved()
}

pub fn is_solved(cube: &CubieCube) -> bool {
    *cube == CubieCube::solved()
}

pub fn apply_move(cube: CubieCube, mv: Move) -> CubieCube {
    let mut cube = cube;
    cube.apply(mv);
    cube
}

pub fn apply_moves(cube: &CubieCube, moves: &[Move]) -> CubieCube {
    let mut cube = *cube;
    for mv in moves {
        cube.apply(*mv);
    }
    cube
}

/// Random HTM scramble. Successive moves obey the same canonical mask as search:
/// no repeated face, and an opposite face only in U-D, R-L, F-B order.
pub fn random_scramble(n: u32) -> Vec<Move> {
    let n = n.min(100) as usize;
    let mut out = Vec::with_capacity(n);
    let mut prev = None;
    let mut guard = 0u32;
    while out.len() < n {
        guard = guard.wrapping_add(1);
        if guard > 10_000 {
            break;
        }
        let mv = ALL_MOVES[(random_u32() as usize) % ALL_MOVES.len()];
        if move_allowed(prev, mv) {
            out.push(mv);
            prev = Some(mv);
        }
    }
    out
}

/// Directories the CLI checks for `manifest.bin`, in order.
///
/// `CARGO_MANIFEST_DIR/tables` is the cube-cli crate root, so `cargo tauri dev`
/// finds `./tables` next to the solver even when the shell's cwd is `src-tauri`.
pub fn tables_candidates() -> Vec<PathBuf> {
    dev_table_dirs()
}

/// Desktop search order.
///
/// `resource_dirs` are bundle resource paths (`resource_dir()/tables`). They are
/// read-only inside a Mac App Store `.app`. Nothing here is a place to write.
pub fn desktop_table_dirs(resource_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = cube_tables_env() {
        dirs.push(dir);
    }
    dirs.extend(resource_dirs.iter().cloned());
    if let Some(dir) = app_support_tables_dir() {
        dirs.push(dir);
    }
    dirs.extend(dev_table_dirs_without_env());
    dirs
}

/// `true` when every desktop candidate is missing `manifest.bin`.
pub fn desktop_tables_missing(resource_dirs: &[PathBuf]) -> bool {
    !desktop_table_dirs(resource_dirs)
        .iter()
        .any(|dir| dir.join("manifest.bin").is_file())
}

/// Writable tables directory for a packaged app. Never a path inside the `.app`.
pub fn app_support_tables_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let base = PathBuf::from(home);
    #[cfg(target_os = "macos")]
    let dir = base.join("Library/Application Support/com.dougcoburn.faceturn/tables");
    #[cfg(not(target_os = "macos"))]
    let dir = base.join(".local/share/com.dougcoburn.faceturn/tables");
    Some(dir)
}

pub fn find_tables_dir() -> Result<PathBuf, DesktopError> {
    first_tables_dir(&tables_candidates()).ok_or_else(|| missing_tables(&tables_candidates(), None))
}

/// Use an existing tables directory, or generate into Application Support.
///
/// Does not write next to the executable or into a bundle resource directory.
pub fn ensure_desktop_tables(resource_dirs: &[PathBuf]) -> Result<PathBuf, DesktopError> {
    let dirs = desktop_table_dirs(resource_dirs);
    if let Some(dir) = first_tables_dir(&dirs) {
        return Ok(dir);
    }
    let Some(support) = app_support_tables_dir() else {
        return Err(missing_tables(&dirs, None));
    };
    if let Err(err) = std::fs::create_dir_all(&support) {
        return Err(missing_tables(
            &dirs,
            Some(format!("could not create {}: {err}", support.display())),
        ));
    }
    match load_or_generate(&support) {
        Ok(_) => Ok(support),
        Err(err) => Err(missing_tables(
            &dirs,
            Some(format!(
                "first-run generate into {} failed: {err}",
                support.display()
            )),
        )),
    }
}

fn cube_tables_env() -> Option<PathBuf> {
    let dir = std::env::var("CUBE_TABLES").ok()?;
    if dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(dir))
    }
}

fn dev_table_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = cube_tables_env() {
        dirs.push(dir);
    }
    dirs.extend(dev_table_dirs_without_env());
    dirs
}

fn dev_table_dirs_without_env() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("tables"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tables"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("tables"));
        }
    }
    dirs
}

fn first_tables_dir(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .find(|dir| dir.join("manifest.bin").is_file())
        .cloned()
}

fn missing_tables(dirs: &[PathBuf], detail: Option<String>) -> DesktopError {
    let listed = dirs
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let extra = detail.map(|text| format!(" {text}.")).unwrap_or_default();
    DesktopError::MissingTables(format!(
        "two-phase tables not found (no manifest.bin in {listed}).{extra} Reinstall the app or run `cube-cli gen-tables`."
    ))
}

/// Solve with tables already generated in `dir`. Does not build them.
pub fn solve_twophase(cube: &CubieCube, dir: &Path) -> Result<TwophaseSolution, DesktopError> {
    if !dir.join("manifest.bin").is_file() {
        return Err(DesktopError::MissingTables(format!(
            "two-phase tables not found at {} (missing manifest.bin). Run `cube-cli gen-tables --dir {}` first.",
            dir.display(),
            dir.display()
        )));
    }
    let loaded = crate::tables::load_existing(dir).map_err(|err| {
        DesktopError::MissingTables(format!(
            "could not load tables at {}: {err}. Run `cube-cli gen-tables --dir {}` first.",
            dir.display(),
            dir.display()
        ))
    })?;
    let started = Instant::now();
    let moves = solve(cube, &loaded.tables)?;
    Ok(TwophaseSolution {
        moves,
        elapsed_ms: started.elapsed().as_millis(),
        optimal: false,
    })
}

/// `solve_twophase` using the first directory from [`find_tables_dir`].
pub fn solve_twophase_default(cube: &CubieCube) -> Result<TwophaseSolution, DesktopError> {
    let dir = find_tables_dir()?;
    solve_twophase(cube, &dir)
}

pub fn facelets_after_moves(facelets: &str, moves: &str) -> Result<String, DesktopError> {
    let cube = parse_facelets(facelets).map_err(DesktopError::Illegal)?;
    let moves = parse_moves(moves).map_err(DesktopError::Moves)?;
    Ok(to_facelets(&apply_moves(&cube, &moves)))
}

pub fn scramble_facelets(n: u32) -> (Vec<Move>, String) {
    let moves = random_scramble(n);
    let cube = apply_moves(&CubieCube::solved(), &moves);
    (moves, to_facelets(&cube))
}

fn random_u32() -> u32 {
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let mut buf = [0u8; 4];
        if file.read_exact(&mut buf).is_ok() {
            return u32::from_ne_bytes(buf);
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    (nanos ^ nanos >> 33) as u32
}
