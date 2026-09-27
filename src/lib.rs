//! Cubie-level cube group. Coordinates, facelets, and search land in later sessions.

mod coords;
mod cubie;
mod facelets;
mod moves;

pub use coords::{decode_orientation, encode_orientation, rank_permutation, unrank_permutation};
pub use cubie::CubieCube;
pub use facelets::{cubie_from_facelets, FaceletError, FACELET_COUNT, SOLVED_FACELETS};
pub use moves::{move_allowed, next_move_mask, Move, ALL_MOVES, MOVE_COUNT, MOVE_CUBE};
