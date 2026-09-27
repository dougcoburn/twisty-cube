//! Cubie-level cube group, facelets, coordinates, and two-phase tables.

mod coords;
mod cubie;
mod facelets;
mod moves;
mod tables;

pub use coords::{
    binomial, decode_orientation, edges_from_slice_sorted, edges_from_ud_edges,
    edges_from_ud_slice, encode_orientation, permutation_parity, rank_permutation, ud_edges,
    ud_slice, ud_slice_sorted, unrank_permutation,
};
pub use cubie::{CubeCoords, CubeError, CubieCube};
pub use facelets::{cubie_from_facelets, facelets_from_cubie, FACELET_COUNT, SOLVED_FACELETS};
pub use moves::{
    move_allowed, next_move_mask, phase2_move, Move, ALL_MOVES, MOVE_COUNT, MOVE_CUBE, PHASE2_MOVES,
};
pub use tables::{
    load_or_generate, LoadedTables, Tables, N_CORNERS, N_FLIP, N_SLICE, N_SLICE_PERM,
    N_SLICE_SORTED, N_TWIST, N_UD_EDGES, TABLE_BUDGET, UD_EDGES_INVALID,
};
