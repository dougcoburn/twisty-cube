//! `cube-cli gen-tables` and `cube-cli solve`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use cube_cli::{
    cubie_from_facelets, format_solution, load_or_generate, parse_moves, solve, CubieCube,
};

#[derive(Parser)]
#[command(name = "cube-cli", about = "Two-phase Rubik's cube solver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate move and prune tables, or map them if they are already on disk.
    GenTables {
        /// Directory of raw table files. Created if missing.
        #[arg(long, default_value = "tables")]
        dir: PathBuf,
    },
    /// Solve a cube with the two-phase algorithm. Stops at the first solution of length ≤20.
    Solve {
        /// 54 facelets in URFDLB order.
        #[arg(long)]
        facelets: Option<String>,
        /// Singmaster maneuver applied to a solved cube, for example "R U R' U'".
        #[arg(long)]
        moves: Option<String>,
        /// Solver mode. Only `twophase` is implemented.
        #[arg(long, default_value = "twophase")]
        mode: String,
        /// Directory of raw table files. Created if missing.
        #[arg(long, default_value = "tables")]
        dir: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::GenTables { dir } => match load_or_generate(&dir) {
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
        },
        Command::Solve {
            facelets,
            moves,
            mode,
            dir,
        } => {
            if mode != "twophase" {
                eprintln!("unknown mode {mode:?}; only twophase is implemented");
                return ExitCode::from(2);
            }
            match solve_cli(facelets, moves, &dir) {
                Ok(line) => {
                    println!("{line}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("solve: {err}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

fn solve_cli(
    facelets: Option<String>,
    moves: Option<String>,
    dir: &std::path::Path,
) -> Result<String, String> {
    let cube = match (facelets, moves) {
        (Some(facelets), None) => cubie_from_facelets(&facelets).map_err(|err| err.to_string())?,
        (None, Some(moves)) => {
            let mut cube = CubieCube::solved();
            for mv in parse_moves(&moves)? {
                cube.apply(mv);
            }
            cube
        }
        (Some(_), Some(_)) => return Err("pass either --facelets or --moves, not both".to_string()),
        (None, None) => return Err("pass --facelets or --moves".to_string()),
    };
    let loaded = load_or_generate(dir).map_err(|err| err.to_string())?;
    let solution = solve(&cube, &loaded.tables).map_err(|err| err.to_string())?;
    Ok(format_solution(&solution))
}
