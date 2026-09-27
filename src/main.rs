//! `cube-cli gen-tables` writes or maps `./tables`. Search is session 3.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use cube_cli::load_or_generate;

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
    /// Solve a cube. Two-phase search is session 3.
    Solve {
        /// 54 facelets in URFDLB order.
        #[arg(long)]
        facelets: Option<String>,
        /// Singmaster maneuver, for example "R U R' U'".
        #[arg(long)]
        moves: Option<String>,
        /// Solver mode. Only `twophase` is planned.
        #[arg(long, default_value = "twophase")]
        mode: String,
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
            facelets: _,
            moves: _,
            mode,
        } => {
            if mode != "twophase" {
                eprintln!("unknown mode {mode:?}; only twophase is planned");
                return ExitCode::from(2);
            }
            eprintln!("solve is not implemented yet (session 3)");
            ExitCode::from(1)
        }
    }
}
