//! `cube-cli gen-tables` and `cube-cli solve` — the commands exist;
//! table generation is session 2 and the search is session 3.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cube-cli", about = "Two-phase Rubik's cube solver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate move and prune tables into ./tables.
    GenTables,
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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::GenTables => {
            eprintln!("gen-tables is not implemented yet (session 2)");
            std::process::exit(1);
        }
        Command::Solve {
            facelets: _,
            moves: _,
            mode,
        } => {
            if mode != "twophase" {
                eprintln!("unknown mode {mode:?}; only twophase is planned");
                std::process::exit(2);
            }
            eprintln!("solve is not implemented yet (session 3)");
            std::process::exit(1);
        }
    }
}
