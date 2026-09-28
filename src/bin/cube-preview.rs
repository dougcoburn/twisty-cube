//! JSON adapter so a webview can call the solver without linking Tauri.
//!
//! `cube-preview <op> [args…]` writes one JSON object to stdout.
//! Failures go to stderr and exit 1. This binary is not the CLI.

use std::env;
use std::process::ExitCode;

use cube_cli::{
    facelets_after_moves, find_tables_dir, parse_facelets, scramble_facelets, solve_twophase,
    CubieCube, Move, SOLVED_FACELETS,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(op) = args.next() else {
        eprintln!("usage: cube-preview <get-solved|validate|apply-moves|scramble|solve> …");
        return ExitCode::from(2);
    };
    let result = match op.as_str() {
        "get-solved" => Ok(format!(r#"{{"facelets":"{SOLVED_FACELETS}"}}"#)),
        "validate" => {
            let facelets = args.next().unwrap_or_default();
            match parse_facelets(&facelets) {
                Ok(_) => Ok(r#"{"ok":true}"#.to_string()),
                Err(err) => Err(err.to_string()),
            }
        }
        "apply-moves" => {
            let facelets = args.next().unwrap_or_default();
            let moves = args.collect::<Vec<_>>().join(" ");
            match facelets_after_moves(&facelets, &moves) {
                Ok(next) => Ok(format!(r#"{{"facelets":"{next}"}}"#)),
                Err(err) => Err(err.to_string()),
            }
        }
        "scramble" => {
            let n: u32 = args
                .next()
                .unwrap_or_else(|| "25".into())
                .parse()
                .unwrap_or(25);
            let (moves, facelets) = scramble_facelets(n);
            Ok(format!(
                r#"{{"moves":{},"facelets":"{facelets}"}}"#,
                json_string_array(&moves)
            ))
        }
        "solve" => {
            let facelets = args.next().unwrap_or_default();
            match solve_facelets(&facelets) {
                Ok(json) => Ok(json),
                Err(err) => Err(err),
            }
        }
        other => Err(format!("unknown op {other}")),
    };
    match result {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn solve_facelets(facelets: &str) -> Result<String, String> {
    let cube = if facelets.is_empty() {
        CubieCube::solved()
    } else {
        parse_facelets(facelets).map_err(|err| err.to_string())?
    };
    let dir = find_tables_dir().map_err(|err| err.to_string())?;
    let solved = solve_twophase(&cube, &dir).map_err(|err| err.to_string())?;
    Ok(format!(
        r#"{{"moves":{},"length":{},"optimal":false,"elapsedMs":{}}}"#,
        json_string_array(&solved.moves),
        solved.moves.len(),
        solved.elapsed_ms
    ))
}

fn json_string_array(moves: &[Move]) -> String {
    let body = moves
        .iter()
        .map(|mv| format!("\"{}\"", mv.name().replace('"', "")))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}
