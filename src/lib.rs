//! Cubie-level cube group, facelets, coordinates, tables, and solvers.

mod coords;
mod cubie;
mod desktop;
mod facelets;
mod moves;
mod optimal;
mod solve;
mod sym;
mod tables;

pub use coords::{
    binomial, decode_orientation, edges_from_slice_sorted, edges_from_ud_edges,
    edges_from_ud_slice, encode_orientation, permutation_parity, rank_permutation, ud_edges,
    ud_slice, ud_slice_sorted, unrank_permutation,
};
pub use cubie::{CubeCoords, CubeError, CubieCube};
pub use desktop::{
    app_support_tables_dir, apply_move, apply_moves, desktop_table_dirs, desktop_tables_missing,
    ensure_desktop_tables, facelets_after_moves, find_tables_dir, is_solved, parse_facelets,
    random_scramble, scramble_facelets, solve_twophase, solve_twophase_default, solved,
    tables_candidates, to_facelets, DesktopError, TwophaseSolution,
};
pub use facelets::{cubie_from_facelets, facelets_from_cubie, FACELET_COUNT, SOLVED_FACELETS};
pub use moves::{
    move_allowed, next_move_mask, parse_moves, phase2_move, Move, ALL_MOVES, MOVE_COUNT, MOVE_CUBE,
    PHASE2_MOVES,
};
pub use optimal::{reid_h, solve_optimal, OptimalError, OptimalSolve};
pub use solve::{format_report, format_solution, solve, SolveError};
pub use tables::{
    load_optimal, load_or_generate, LoadedTables, Tables, N_CORNERS, N_FLIP, N_SLICE, N_SLICE_PERM,
    N_SLICE_SORTED, N_TWIST, N_UD_EDGES, TABLE_BUDGET, UD_EDGES_INVALID,
};
