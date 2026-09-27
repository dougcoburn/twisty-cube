//! 54-character facelet strings in URFDLB order.
//!
//! Parsing, cubie conversion, and the twist / flip / parity checks are session 1.
//! The solved coloring is fixed now so later sessions share one constant.

use crate::CubieCube;

pub const FACELET_COUNT: usize = 54;
pub const SOLVED_FACELETS: &str = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaceletError {
    /// `cubie_from_facelets` is not implemented until session 1.
    NotImplemented,
}

pub fn cubie_from_facelets(_facelets: &str) -> Result<CubieCube, FaceletError> {
    Err(FaceletError::NotImplemented)
}
