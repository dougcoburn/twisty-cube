//! Optimal HTM search: IDA* with Reid's three-axis phase-1 heuristic.
//!
//! `h` is the exact distance to H on the UD, RL, and FB axes. A half turn can
//! sit in all three subgroups, so `h = 0` does not by itself mean solved.
//! The search returns one shortest solution, not every one.

use std::sync::OnceLock;

use crate::moves::{move_allowed, Move, ALL_MOVES, MOVE_CUBE};
use crate::sym::{conjugate, sym_index, SymIndex};
use crate::tables::Tables;
use crate::{CubeError, CubieCube};

const MAX_GOD: usize = 20;
const NONE: usize = 18;

struct Succ {
    moves: [[Move; 18]; 19],
    len: [usize; 19],
}

fn successors() -> &'static Succ {
    static SUCC: OnceLock<Succ> = OnceLock::new();
    SUCC.get_or_init(|| {
        let mut succ = Succ {
            moves: [[Move::U1; 18]; 19],
            len: [0; 19],
        };
        for prev_i in 0..19 {
            let prev = if prev_i == NONE {
                None
            } else {
                Some(Move::from_index(prev_i as u8))
            };
            for mv in ALL_MOVES {
                if move_allowed(prev, mv) {
                    let n = succ.len[prev_i];
                    succ.moves[prev_i][n] = mv;
                    succ.len[prev_i] = n + 1;
                }
            }
        }
        succ
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimalError {
    Illegal(CubeError),
    NoSolution,
    Rejected,
}

impl std::fmt::Display for OptimalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimalError::Illegal(err) => write!(f, "{err}"),
            OptimalError::NoSolution => write!(f, "no solution within the bound"),
            OptimalError::Rejected => write!(f, "solution failed replay against the original cube"),
        }
    }
}

impl std::error::Error for OptimalError {}

pub struct OptimalSolve {
    pub moves: Vec<Move>,
    pub nodes: u64,
}

/// Exact distance to the UD phase-1 subgroup, then the same after rotating
/// the RL and FB axes into UD. Zero only for the solved cube.
pub fn reid_h(cube: &CubieCube, tables: &Tables) -> u8 {
    let index = sym_index();
    axis_h(cube, tables, index)
}

fn axis_h(cube: &CubieCube, tables: &Tables, index: &SymIndex) -> u8 {
    let prun = tables.phase1_prun();
    let h_ud = index.phase1_ud(prun, cube.twist(), cube.flip(), cube.slice());
    let rl = conjugate(&index.urf3, cube, &index.urf3_inv);
    let h_rl = index.phase1_ud(prun, rl.twist(), rl.flip(), rl.slice());
    let fb = conjugate(&index.urf3_sq, cube, &index.urf3);
    let h_fb = index.phase1_ud(prun, fb.twist(), fb.flip(), fb.slice());
    h_ud.max(h_rl).max(h_fb)
}

struct Search<'a> {
    tables: &'a Tables,
    index: &'a SymIndex,
    succ: &'a Succ,
    path: [Move; MAX_GOD],
    found: usize,
    nodes: u64,
}

impl<'a> Search<'a> {
    fn dfs(&mut self, cube: CubieCube, g: usize, bound: usize, prev: usize) -> bool {
        self.nodes += 1;
        if cube == CubieCube::solved() {
            self.found = g;
            return true;
        }
        let h = axis_h(&cube, self.tables, self.index) as usize;
        if g + h > bound || g == bound {
            return false;
        }
        let moves = &self.succ.moves[prev][..self.succ.len[prev]];
        for &mv in moves {
            self.path[g] = mv;
            let next = cube.multiplied(MOVE_CUBE[mv as usize]);
            if self.dfs(next, g + 1, bound, mv as usize) {
                return true;
            }
        }
        false
    }
}

/// One shortest HTM solution. `max_bound` is clamped to 20.
pub fn solve_optimal(
    cube: &CubieCube,
    tables: &Tables,
    max_bound: usize,
) -> Result<OptimalSolve, OptimalError> {
    cube.check().map_err(OptimalError::Illegal)?;
    let max_bound = max_bound.min(MAX_GOD);
    if *cube == CubieCube::solved() {
        return Ok(OptimalSolve {
            moves: Vec::new(),
            nodes: 0,
        });
    }
    let index = sym_index();
    let mut lower = axis_h(cube, tables, index) as usize;
    // A half turn can lie in every axis subgroup, so h can be 0 on an unsolved cube.
    if lower == 0 {
        lower = 1;
    }
    if lower > max_bound {
        return Err(OptimalError::NoSolution);
    }
    let succ = successors();
    let mut search = Search {
        tables,
        index,
        succ,
        path: [Move::U1; MAX_GOD],
        found: 0,
        nodes: 0,
    };
    for bound in lower..=max_bound {
        if search.dfs(*cube, 0, bound, NONE) {
            let moves = search.path[..search.found].to_vec();
            let mut check = *cube;
            for mv in &moves {
                check.apply(*mv);
            }
            if check != CubieCube::solved() {
                return Err(OptimalError::Rejected);
            }
            return Ok(OptimalSolve {
                moves,
                nodes: search.nodes,
            });
        }
    }
    Err(OptimalError::NoSolution)
}
