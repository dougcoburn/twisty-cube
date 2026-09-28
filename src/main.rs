//! `cube-cli gen-tables` and `cube-cli solve`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use cube_cli::{
    cubie_from_facelets, format_report, load_optimal, load_or_generate, parse_moves, solve,
    solve_optimal, CubieCube,
};

#[derive(Parser)]
#[command(
    name = "cube-cli",
    about = "Two-phase and optimal HTM Rubik's cube solver"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate two-phase tables, or map them if they are already on disk.
    GenTables {
        /// Directory of raw table files. Created if missing.
        #[arg(long, default_value = "tables")]
        dir: PathBuf,
        /// Also build `phase1_prun.bin`, the exact table used by `--mode optimal`.
        #[arg(long)]
        optimal: bool,
    },
    /// Solve a cube. `twophase` is fast; `optimal` returns a shortest HTM solution.
    Solve {
        /// 54 facelets in URFDLB order.
        #[arg(long)]
        facelets: Option<String>,
        /// Singmaster maneuver applied to a solved cube, for example "R U R' U'".
        #[arg(long)]
        moves: Option<String>,
        /// `twophase` (default, not always shortest) or `optimal` (shortest face turns).
        /// Optimal loads `phase1_prun.bin` and builds it on first use.
        #[arg(long, default_value = "twophase")]
        mode: String,
        /// Directory of raw table files. Created if missing.
        #[arg(long, default_value = "tables")]
        dir: PathBuf,
        /// Largest length optimal search will prove. Ignored by twophase.
        #[arg(long, default_value_t = 20)]
        max_bound: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::GenTables { dir, optimal } => {
            let loaded = if optimal {
                load_optimal(&dir)
            } else {
                load_or_generate(&dir)
            };
            match loaded {
                Ok(loaded) => {
                    let verb = if loaded.mapped { "mapped" } else { "wrote" };
                    for (path, bytes) in &loaded.files {
                        println!("{verb} {path}  {bytes}");
                    }
                    println!("total {} bytes", loaded.total_bytes());
                    for (name, depth, unseen) in loaded.tables.prune_summary() {
                        println!("{name} max depth {depth}, unfilled {unseen}");
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("gen-tables: {err}");
                    ExitCode::from(1)
                }
            }
        }
        Command::Solve {
            facelets,
            moves,
            mode,
            dir,
            max_bound,
        } => match solve_cli(facelets, moves, &mode, &dir, max_bound) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(code) => code,
        },
    }
}

fn solve_cli(
    facelets: Option<String>,
    moves: Option<String>,
    mode: &str,
    dir: &std::path::Path,
    max_bound: usize,
) -> Result<String, ExitCode> {
    if mode != "twophase" && mode != "optimal" {
        eprintln!("unknown mode {mode:?}; use twophase or optimal");
        return Err(ExitCode::from(2));
    }
    let cube = match (facelets, moves) {
        (Some(facelets), None) => cubie_from_facelets(&facelets).map_err(|err| {
            eprintln!("solve: {err}");
            ExitCode::from(1)
        })?,
        (None, Some(moves)) => {
            let mut cube = CubieCube::solved();
            for mv in parse_moves(&moves).map_err(|err| {
                eprintln!("solve: {err}");
                ExitCode::from(1)
            })? {
                cube.apply(mv);
            }
            cube
        }
        (Some(_), Some(_)) => {
            eprintln!("solve: pass either --facelets or --moves, not both");
            return Err(ExitCode::from(1));
        }
        (None, None) => {
            eprintln!("solve: pass --facelets or --moves");
            return Err(ExitCode::from(1));
        }
    };
    let loaded = if mode == "optimal" {
        load_optimal(dir)
    } else {
        load_or_generate(dir)
    }
    .map_err(|err| {
        eprintln!("solve: {err}");
        ExitCode::from(1)
    })?;
    if mode == "twophase" {
        let solution = solve(&cube, &loaded.tables).map_err(|err| {
            eprintln!("solve: {err}");
            ExitCode::from(1)
        })?;
        return Ok(format_report(&solution, false));
    }
    let started = Instant::now();
    let solved = solve_optimal(&cube, &loaded.tables, max_bound).map_err(|err| {
        eprintln!("solve: {err}");
        ExitCode::from(1)
    })?;
    eprintln!(
        "nodes: {} time: {:.3}s",
        solved.nodes,
        started.elapsed().as_secs_f64()
    );
    Ok(format_report(&solved.moves, true))
}
