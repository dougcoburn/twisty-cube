//! Two-phase IDA*. Phase 1 searches into H. Phase 2 stays in H.
//! The first solution of length at most 20 is returned; it is not proved optimal.

use std::sync::OnceLock;

use crate::moves::{move_allowed, Move, ALL_MOVES, PHASE2_MOVES};
use crate::tables::Tables;
use crate::{CubeError, CubieCube};

const MAX_LEN: usize = 20;
const NONE: usize = 18;

struct Succ {
    phase1: [[Move; 18]; 19],
    phase1_len: [usize; 19],
    phase2: [[Move; 10]; 19],
    phase2_len: [usize; 19],
}

fn successors() -> &'static Succ {
    static SUCC: OnceLock<Succ> = OnceLock::new();
    SUCC.get_or_init(|| {
        let mut succ = Succ {
            phase1: [[Move::U1; 18]; 19],
            phase1_len: [0; 19],
            phase2: [[Move::U1; 10]; 19],
            phase2_len: [0; 19],
        };
        for prev_i in 0..19 {
            let prev = if prev_i == NONE {
                None
            } else {
                Some(Move::from_index(prev_i as u8))
            };
            for mv in ALL_MOVES {
                if move_allowed(prev, mv) {
                    let n = succ.phase1_len[prev_i];
                    succ.phase1[prev_i][n] = mv;
                    succ.phase1_len[prev_i] = n + 1;
                }
            }
            for mv in PHASE2_MOVES {
                if move_allowed(prev, mv) {
                    let n = succ.phase2_len[prev_i];
                    succ.phase2[prev_i][n] = mv;
                    succ.phase2_len[prev_i] = n + 1;
                }
            }
        }
        succ
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    Illegal(CubeError),
    NoSolution,
}

impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolveError::Illegal(err) => write!(f, "{err}"),
            SolveError::NoSolution => write!(f, "no solution of length <= {MAX_LEN}"),
        }
    }
}

impl std::error::Error for SolveError {}

struct Search<'a> {
    tables: &'a Tables,
    succ: &'a Succ,
    start: CubieCube,
    path: [Move; MAX_LEN],
    found: usize,
}

impl<'a> Search<'a> {
    fn phase1(
        &mut self,
        twist: u16,
        flip: u16,
        slice_sorted: u16,
        depth: usize,
        target: usize,
    ) -> bool {
        if depth == target {
            if twist == 0 && flip == 0 && slice_sorted < 24 {
                return self.finish_phase2(depth);
            }
            return false;
        }
        let remain = target - depth;
        let slice = slice_sorted / 24;
        let bound = self
            .tables
            .prune_twist_slice(twist, slice)
            .max(self.tables.prune_flip_slice(flip, slice));
        if bound as usize > remain {
            return false;
        }
        let prev = if depth == 0 {
            NONE
        } else {
            self.path[depth - 1] as usize
        };
        let moves = &self.succ.phase1[prev][..self.succ.phase1_len[prev]];
        for &mv in moves {
            self.path[depth] = mv;
            let next_twist = self.tables.twist_move(twist, mv);
            let next_flip = self.tables.flip_move(flip, mv);
            let next_slice = self.tables.slice_sorted_move(slice_sorted, mv);
            if self.phase1(next_twist, next_flip, next_slice, depth + 1, target) {
                return true;
            }
        }
        false
    }

    fn finish_phase2(&mut self, phase1_len: usize) -> bool {
        let mut cube = self.start;
        for mv in &self.path[..phase1_len] {
            cube.apply(*mv);
        }
        let corners = cube.corners();
        let ud = cube.ud_edges();
        let slice = cube.slice_sorted();
        debug_assert!(slice < 24);
        let budget = MAX_LEN - phase1_len;
        let dist = self
            .tables
            .prune_corners(corners, slice)
            .max(self.tables.prune_ud_edges(ud, slice)) as usize;
        if dist > budget {
            return false;
        }
        for togo in dist..=budget {
            if self.phase2(corners, ud, slice, phase1_len, phase1_len + togo) {
                return true;
            }
        }
        false
    }

    fn phase2(&mut self, corners: u16, ud: u16, slice: u16, depth: usize, limit: usize) -> bool {
        if corners == 0 && ud == 0 && slice == 0 {
            self.found = depth;
            return true;
        }
        if depth == limit {
            return false;
        }
        let remain = (limit - depth) as u8;
        let bound = self
            .tables
            .prune_corners(corners, slice)
            .max(self.tables.prune_ud_edges(ud, slice));
        if bound > remain {
            return false;
        }
        let prev = if depth == 0 {
            NONE
        } else {
            self.path[depth - 1] as usize
        };
        let moves = &self.succ.phase2[prev][..self.succ.phase2_len[prev]];
        for &mv in moves {
            self.path[depth] = mv;
            let next_corners = self.tables.corners_move(corners, mv);
            let next_ud = self.tables.ud_edges_move(ud, mv);
            let next_slice = self.tables.slice_sorted_move(slice, mv);
            if self.phase2(next_corners, next_ud, next_slice, depth + 1, limit) {
                return true;
            }
        }
        false
    }
}

/// Maneuver of length at most 20 that takes `cube` to solved.
pub fn solve(cube: &CubieCube, tables: &Tables) -> Result<Vec<Move>, SolveError> {
    cube.check().map_err(SolveError::Illegal)?;
    let twist = cube.twist();
    let flip = cube.flip();
    let slice_sorted = cube.slice_sorted();
    let slice = slice_sorted / 24;
    let lower = tables
        .prune_twist_slice(twist, slice)
        .max(tables.prune_flip_slice(flip, slice)) as usize;
    let succ = successors();
    let mut search = Search {
        tables,
        succ,
        start: *cube,
        path: [Move::U1; MAX_LEN],
        found: 0,
    };
    for target in lower..=MAX_LEN {
        if search.phase1(twist, flip, slice_sorted, 0, target) {
            return Ok(search.path[..search.found].to_vec());
        }
    }
    Err(SolveError::NoSolution)
}

/// Space-separated names, or `0` when the cube is already solved.
pub fn format_solution(moves: &[Move]) -> String {
    if moves.is_empty() {
        "0".to_string()
    } else {
        moves
            .iter()
            .map(|mv| mv.name())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
