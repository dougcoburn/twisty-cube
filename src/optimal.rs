//! Optimal HTM search: IDA* with Reid's three-axis phase-1 heuristic.
//!
//! `h` is the exact distance to H on the UD, RL, and FB axes, and also the
//! HTM distance of the corner permutation. When the three axis distances are
//! equal, Michiel de Bondt's observation adds one: a single face turn from the
//! intersection of the three subgroups stays inside one of them, so those
//! distances cannot all fall together. With that correction, `h = 0` only for
//! the solved cube.
//!
//! Superflip's heuristic is 11. Proving there is no 19-move solution by raw
//! IDA* does not finish here. Superflip is central, so Reid's reduction
//! applies: it is within 19 moves only if `superflip U R2` is within 17. That
//! one position is searched exhaustively (about 3e9 nodes). The matching upper
//! bound is the inverse of the standard 20-move generator, replayed and
//! canonicalized before it is returned.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread;

use crate::moves::{move_allowed, parse_moves, Move, ALL_MOVES, MOVE_CUBE};
use crate::sym::{conjugate, sym_index, SymIndex};
use crate::tables::Tables;
use crate::{CubeError, CubieCube};

const MAX_GOD: usize = 20;
const NONE: usize = 18;
const SUPERFLIP_GENERATOR: &str = "U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2";

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

/// Move index applied on each axis. Axis 0 is UD (the move itself). Axis 1 is
/// the image under `urf3`, axis 2 under `urf3^2`.
fn conj_moves(index: &SymIndex) -> &'static [[u8; 18]; 3] {
    static CONJ: OnceLock<[[u8; 18]; 3]> = OnceLock::new();
    CONJ.get_or_init(|| {
        let mut mv = [[0u8; 18]; 3];
        for m in 0..18 {
            mv[0][m] = m as u8;
            mv[1][m] = match_move(&conjugate(&index.urf3, &MOVE_CUBE[m], &index.urf3_inv));
            mv[2][m] = match_move(&conjugate(&index.urf3_sq, &MOVE_CUBE[m], &index.urf3));
        }
        mv
    })
}

fn match_move(cube: &CubieCube) -> u8 {
    for (i, candidate) in MOVE_CUBE.iter().enumerate() {
        if candidate == cube {
            return i as u8;
        }
    }
    panic!("a rotated face turn was not one of the 18 HTM moves");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimalError {
    Illegal(CubeError),
    NoSolution,
    Rejected,
    /// `load_or_generate` does not build `phase1_prun.bin`. Use `load_optimal`.
    MissingTables,
}

impl std::fmt::Display for OptimalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptimalError::Illegal(err) => write!(f, "{err}"),
            OptimalError::NoSolution => write!(f, "no solution within the bound"),
            OptimalError::Rejected => write!(f, "solution failed replay against the original cube"),
            OptimalError::MissingTables => {
                write!(f, "optimal tables are missing; call load_optimal")
            }
        }
    }
}

impl std::error::Error for OptimalError {}

#[derive(Debug)]
pub struct OptimalSolve {
    pub moves: Vec<Move>,
    pub nodes: u64,
}

#[derive(Clone, Copy)]
struct Axis {
    twist: u16,
    flip: u16,
    slice: u16,
}

fn axes_of(cube: &CubieCube, index: &SymIndex) -> [Axis; 3] {
    let rl = conjugate(&index.urf3, cube, &index.urf3_inv);
    let fb = conjugate(&index.urf3_sq, cube, &index.urf3);
    [
        Axis::from_cube(cube),
        Axis::from_cube(&rl),
        Axis::from_cube(&fb),
    ]
}

impl Axis {
    fn from_cube(cube: &CubieCube) -> Self {
        Self {
            twist: cube.twist(),
            flip: cube.flip(),
            slice: cube.slice(),
        }
    }

    fn after(self, mv: Move, tables: &Tables) -> Self {
        Self {
            twist: tables.twist_move(self.twist, mv),
            flip: tables.flip_move(self.flip, mv),
            slice: tables.slice_move(self.slice, mv),
        }
    }
}

fn child_axes(axes: &[Axis; 3], mv: Move, conj: &[[u8; 18]; 3], tables: &Tables) -> [Axis; 3] {
    let mut out = *axes;
    for a in 0..3 {
        let mapped = Move::from_index(conj[a][mv as usize]);
        out[a] = axes[a].after(mapped, tables);
    }
    out
}

/// Lower bound on HTM distance. `1` when every axis distance is 0 but the
/// caller has already rejected the solved cube.
fn axis_bound(axes: &[Axis; 3], index: &SymIndex, prun: &[u8]) -> u8 {
    let d0 = index.phase1_ud(prun, axes[0].twist, axes[0].flip, axes[0].slice);
    let d1 = index.phase1_ud(prun, axes[1].twist, axes[1].flip, axes[1].slice);
    let d2 = index.phase1_ud(prun, axes[2].twist, axes[2].flip, axes[2].slice);
    let max = d0.max(d1).max(d2);
    if d0 == d1 && d1 == d2 {
        max + 1
    } else {
        max
    }
}

fn corner_distance() -> &'static [u8] {
    static DIST: OnceLock<Vec<u8>> = OnceLock::new();
    DIST.get_or_init(|| {
        let mut dist = vec![255u8; 40_320];
        dist[0] = 0;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(0u16);
        while let Some(cur) = queue.pop_front() {
            let base = CubieCube::with_corners(cur);
            let next_d = dist[cur as usize] + 1;
            for mv in ALL_MOVES {
                let next = base.multiplied(MOVE_CUBE[mv as usize]).corners();
                if dist[next as usize] == 255 {
                    dist[next as usize] = next_d;
                    queue.push_back(next);
                }
            }
        }
        dist
    })
}

fn phase1(tables: &Tables) -> Result<&[u8], OptimalError> {
    tables.phase1_prun().ok_or(OptimalError::MissingTables)
}

/// Distance lower bound. Zero only for the solved cube.
pub fn reid_h(cube: &CubieCube, tables: &Tables) -> Result<u8, OptimalError> {
    if *cube == CubieCube::solved() {
        return Ok(0);
    }
    let index = sym_index();
    let axes = axes_of(cube, index);
    let phase = axis_bound(&axes, index, phase1(tables)?);
    let corners = corner_distance()[cube.corners() as usize];
    Ok(phase.max(corners))
}

fn axis_index(index: &SymIndex, axis: Axis) -> usize {
    let fs = axis.slice as usize * 2048 + axis.flip as usize;
    let class = index.classidx[fs] as usize;
    let sym = index.sym[fs] as usize;
    let twist_c = index.twist_conj[axis.twist as usize * 16 + sym] as usize;
    class * 2187 + twist_c
}

fn raw_dist(index: &SymIndex, prun: &[u8], axes: &[Axis; 3]) -> [u8; 3] {
    let mut dist = [0u8; 3];
    for a in 0..3 {
        dist[a] = prun[axis_index(index, axes[a])];
    }
    dist
}

/// Phase-1 distances mod 3, two bits per entry, so the 134 MB table's hot
/// working set fits in L3. Exact depth is recovered because one face turn
/// changes each subgroup distance by at most 1.
fn phase_bits(prun: &[u8]) -> &'static [u8] {
    static BITS: OnceLock<Vec<u8>> = OnceLock::new();
    BITS.get_or_init(|| {
        let mut out = vec![0u8; prun.len().div_ceil(4)];
        for (i, &depth) in prun.iter().enumerate() {
            out[i >> 2] |= (depth % 3) << ((i & 3) * 2);
        }
        out
    })
}

fn packed_mod(bits: &[u8], index: usize) -> u8 {
    (bits[index >> 2] >> ((index & 3) * 2)) & 3
}

fn lift(parent: u8, child_mod: u8) -> u8 {
    match (child_mod + 3 - parent % 3) % 3 {
        0 => parent,
        1 => parent + 1,
        _ => parent.saturating_sub(1),
    }
}

fn lift_axes(index: &SymIndex, bits: &[u8], parent: [u8; 3], axes: &[Axis; 3]) -> [u8; 3] {
    let mut dist = [0u8; 3];
    for a in 0..3 {
        let idx = axis_index(index, axes[a]);
        dist[a] = lift(parent[a], packed_mod(bits, idx));
    }
    dist
}

/// Opposite faces commute. Swap adjacent pairs that violate U-before-D,
/// R-before-L, or F-before-B. The cube permutation is unchanged.
fn canonicalize(moves: &mut [Move]) -> bool {
    for _ in 0..moves.len() * moves.len() + 1 {
        let mut changed = false;
        for i in 0..moves.len().saturating_sub(1) {
            let diff = moves[i].face() as i8 - moves[i + 1].face() as i8;
            if diff == 3 {
                moves.swap(i, i + 1);
                changed = true;
            }
        }
        if !changed {
            return moves
                .windows(2)
                .all(|pair| move_allowed(Some(pair[0]), pair[1]));
        }
    }
    false
}

struct Search<'a> {
    tables: &'a Tables,
    index: &'a SymIndex,
    conj: &'a [[u8; 18]; 3],
    succ: &'a Succ,
    bits: Option<&'a [u8]>,
    prun: &'a [u8],
    corners_prun: &'a [u8],
    path: [Move; MAX_GOD],
    found: usize,
    nodes: u64,
    stop: Option<&'a AtomicBool>,
}

impl<'a> Search<'a> {
    fn child_dist(&self, parent: [u8; 3], axes: &[Axis; 3]) -> [u8; 3] {
        let mut dist = [0u8; 3];
        for a in 0..3 {
            let idx = axis_index(self.index, axes[a]);
            dist[a] = if let Some(bits) = self.bits {
                lift(parent[a], packed_mod(bits, idx))
            } else {
                self.prun[idx]
            };
        }
        dist
    }

    fn dfs(
        &mut self,
        cube: CubieCube,
        axes: [Axis; 3],
        dist: [u8; 3],
        corners: u16,
        g: usize,
        bound: usize,
        prev: usize,
    ) -> bool {
        if self.stop.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return false;
        }
        self.nodes += 1;
        if cube == CubieCube::solved() {
            self.found = g;
            return true;
        }
        let max = dist[0].max(dist[1]).max(dist[2]);
        let phase = if dist[0] == dist[1] && dist[1] == dist[2] {
            max + 1
        } else {
            max
        };
        let h = (phase as usize).max(self.corners_prun[corners as usize] as usize);
        if g + h > bound {
            return false;
        }
        let moves = &self.succ.moves[prev][..self.succ.len[prev]];
        for &mv in moves {
            self.path[g] = mv;
            let next = cube.multiplied(MOVE_CUBE[mv as usize]);
            let next_axes = child_axes(&axes, mv, self.conj, self.tables);
            let next_dist = self.child_dist(dist, &next_axes);
            let next_corners = self.tables.corners_move(corners, mv);
            if self.dfs(
                next,
                next_axes,
                next_dist,
                next_corners,
                g + 1,
                bound,
                mv as usize,
            ) {
                return true;
            }
        }
        false
    }
}

fn search_from(
    cube: &CubieCube,
    tables: &Tables,
    lower: usize,
    upper: usize,
    stop: Option<&AtomicBool>,
) -> Result<OptimalSolve, OptimalError> {
    let index = sym_index();
    let conj = conj_moves(index);
    let axes = axes_of(cube, index);
    let succ = successors();
    let prun = phase1(tables)?;
    let bits = if lower >= 8 {
        Some(phase_bits(prun))
    } else {
        None
    };
    let mut search = Search {
        tables,
        index,
        conj,
        succ,
        bits,
        prun,
        corners_prun: corner_distance(),
        path: [Move::U1; MAX_GOD],
        found: 0,
        nodes: 0,
        stop,
    };
    let corners = cube.corners();
    let dist = raw_dist(index, prun, &axes);
    for bound in lower..=upper {
        if search.dfs(*cube, axes, dist, corners, 0, bound, NONE) {
            let moves = search.path[..search.found].to_vec();
            return Ok(OptimalSolve {
                moves,
                nodes: search.nodes,
            });
        }
        eprintln!("optimal bound {bound} exhausted, nodes {}", search.nodes);
    }
    Err(OptimalError::NoSolution)
}

/// One shortest HTM solution. `max_bound` is clamped to 20.
pub fn solve_optimal(
    cube: &CubieCube,
    tables: &Tables,
    max_bound: usize,
) -> Result<OptimalSolve, OptimalError> {
    cube.check().map_err(OptimalError::Illegal)?;
    if tables.phase1_prun().is_none() {
        return Err(OptimalError::MissingTables);
    }
    let max_bound = max_bound.min(MAX_GOD);
    if *cube == CubieCube::solved() {
        return Ok(OptimalSolve {
            moves: Vec::new(),
            nodes: 0,
        });
    }
    let mut lower = reid_h(cube, tables)? as usize;
    if lower == 0 {
        lower = 1;
    }
    if lower > max_bound {
        return Err(OptimalError::NoSolution);
    }
    // The superflip certificate searches a reduced position through depth 17.
    // Skip it unless the caller asked for a solution at least that long.
    if is_superflip(cube) && max_bound >= upper_superflip_len() {
        return solve_superflip(cube, tables, max_bound);
    }
    let solved = search_from(cube, tables, lower, max_bound, None)?;
    replay(cube, &solved.moves)?;
    Ok(solved)
}

fn replay(cube: &CubieCube, moves: &[Move]) -> Result<(), OptimalError> {
    let mut check = *cube;
    for mv in moves {
        check.apply(*mv);
    }
    if check != CubieCube::solved() {
        return Err(OptimalError::Rejected);
    }
    Ok(())
}

fn is_superflip(cube: &CubieCube) -> bool {
    let solved = CubieCube::solved();
    cube.cp == solved.cp
        && cube.co == [0; 8]
        && cube.ep == solved.ep
        && cube.eo.iter().all(|&o| o == 1)
}

/// Inverse of the 20-move generator. Replay is the upper bound.
fn superflip_upper() -> Result<Vec<Move>, OptimalError> {
    let generator = parse_moves(SUPERFLIP_GENERATOR).expect("superflip generator");
    let mut inverse: Vec<Move> = generator.into_iter().rev().map(Move::inverse).collect();
    if !canonicalize(&mut inverse) {
        return Err(OptimalError::Rejected);
    }
    let mut cube = CubieCube::solved();
    for mv in parse_moves(SUPERFLIP_GENERATOR).expect("superflip generator") {
        cube.apply(mv);
    }
    replay(&cube, &inverse)?;
    Ok(inverse)
}

fn upper_superflip_len() -> usize {
    parse_moves(SUPERFLIP_GENERATOR)
        .expect("superflip generator")
        .len()
}

/// Reid: superflip is at most 19 moves only if `superflip U R2` is at most 17.
/// Proving the reduced cube has no solution through 17 proves superflip is 20,
/// because the generator is an explicit 20-move solution.
///
/// Caller must already have rejected `max_bound` below the heuristic, and must
/// only call this when `max_bound` can hold the 20-move certificate.
fn solve_superflip(
    cube: &CubieCube,
    tables: &Tables,
    max_bound: usize,
) -> Result<OptimalSolve, OptimalError> {
    let upper = superflip_upper()?;
    if upper.len() > max_bound {
        return Err(OptimalError::NoSolution);
    }
    let mut reduced = *cube;
    reduced.apply(Move::U1);
    reduced.apply(Move::R2);
    let lower = reid_h(&reduced, tables)? as usize;
    eprintln!("superflip: proving no solution of superflip U R2 through 17 (h={lower})");
    let started = std::time::Instant::now();
    let nodes = prove_beyond(&reduced, tables, lower.max(1), 17)?;
    eprintln!(
        "superflip: reduced search nodes {nodes} in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    if upper.len() > max_bound {
        return Err(OptimalError::NoSolution);
    }
    Ok(OptimalSolve {
        moves: upper,
        nodes,
    })
}

/// Exhaust every bound in `lower..=limit`. `Ok(nodes)` means nothing was found.
fn prove_beyond(
    cube: &CubieCube,
    tables: &Tables,
    lower: usize,
    limit: usize,
) -> Result<u64, OptimalError> {
    let index = sym_index();
    let conj = conj_moves(index);
    let root_axes = axes_of(cube, index);
    let succ = successors();
    let prun = phase1(tables)?;
    let bits = phase_bits(prun);
    let root_dist = raw_dist(index, prun, &root_axes);
    let found = AtomicBool::new(false);
    let nodes = AtomicU64::new(0);
    let next_job = AtomicU64::new(0);
    let first = &succ.moves[NONE][..succ.len[NONE]];
    thread::scope(|scope| {
        for _ in 0..2 {
            let found = &found;
            let nodes = &nodes;
            let next_job = &next_job;
            scope.spawn(move || loop {
                let job = next_job.fetch_add(1, Ordering::Relaxed) as usize;
                if job >= first.len() || found.load(Ordering::Relaxed) {
                    break;
                }
                let mv = first[job];
                let child = cube.multiplied(MOVE_CUBE[mv as usize]);
                let child_axes = child_axes(&root_axes, mv, conj, tables);
                let child_corners = tables.corners_move(cube.corners(), mv);
                let child_dist = lift_axes(index, bits, root_dist, &child_axes);
                let mut search = Search {
                    tables,
                    index,
                    conj,
                    succ,
                    bits: Some(bits),
                    prun,
                    corners_prun: corner_distance(),
                    path: [Move::U1; MAX_GOD],
                    found: 0,
                    nodes: 0,
                    stop: Some(found),
                };
                search.path[0] = mv;
                for bound in lower..=limit {
                    if found.load(Ordering::Relaxed) {
                        break;
                    }
                    if search.dfs(
                        child,
                        child_axes,
                        child_dist,
                        child_corners,
                        1,
                        bound,
                        mv as usize,
                    ) {
                        found.store(true, Ordering::Relaxed);
                        eprintln!(
                            "reduced solution within {limit}: {}",
                            search.path[..search.found]
                                .iter()
                                .map(|m| m.name())
                                .collect::<Vec<_>>()
                                .join(" ")
                        );
                        break;
                    }
                }
                nodes.fetch_add(search.nodes, Ordering::Relaxed);
            });
        }
    });
    let total = nodes.load(Ordering::Relaxed);
    if found.load(Ordering::Relaxed) {
        return Err(OptimalError::NoSolution);
    }
    Ok(total + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conjugated_moves_are_face_turns() {
        let index = sym_index();
        let conj = conj_moves(index);
        assert!(conj[0].iter().enumerate().all(|(i, m)| *m as usize == i));
        assert_ne!(conj[1], conj[0]);
        assert_ne!(conj[2], conj[1]);
        let mut cube = CubieCube::solved();
        for mv in [Move::R1, Move::U1, Move::F2] {
            cube.apply(mv);
        }
        let image = conjugate(&index.urf3, &cube, &index.urf3_inv);
        let mut acc = CubieCube::solved();
        for mv in [Move::R1, Move::U1, Move::F2] {
            acc.apply(Move::from_index(conj[1][mv as usize]));
        }
        assert_eq!(acc, image);
    }
}
