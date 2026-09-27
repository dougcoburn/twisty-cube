//! Eighteen half-turn-metric face turns, and the successor mask used by IDA*.
//!
//! Move index `3 * face + power` with faces U R F D L B and power 0, 1, 2
//! meaning 90°, 180°, 270° clockwise. A successor of face `f` is forbidden
//! when `prev_face - f` is 0 (same face) or 3 (opposite face, wrong order).
//! That keeps U before D, R before L, and F before B.

use crate::cubie::CubieCube;

pub const MOVE_COUNT: usize = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Move {
    U1 = 0,
    U2 = 1,
    U3 = 2,
    R1 = 3,
    R2 = 4,
    R3 = 5,
    F1 = 6,
    F2 = 7,
    F3 = 8,
    D1 = 9,
    D2 = 10,
    D3 = 11,
    L1 = 12,
    L2 = 13,
    L3 = 14,
    B1 = 15,
    B2 = 16,
    B3 = 17,
}

pub const ALL_MOVES: [Move; MOVE_COUNT] = [
    Move::U1,
    Move::U2,
    Move::U3,
    Move::R1,
    Move::R2,
    Move::R3,
    Move::F1,
    Move::F2,
    Move::F3,
    Move::D1,
    Move::D2,
    Move::D3,
    Move::L1,
    Move::L2,
    Move::L3,
    Move::B1,
    Move::B2,
    Move::B3,
];

const MOVE_NAMES: [&str; MOVE_COUNT] = [
    "U", "U2", "U'", "R", "R2", "R'", "F", "F2", "F'", "D", "D2", "D'", "L", "L2", "L'", "B", "B2",
    "B'",
];

impl Move {
    pub const fn from_index(i: u8) -> Self {
        ALL_MOVES[i as usize]
    }

    pub const fn face(self) -> u8 {
        self as u8 / 3
    }

    /// 0, 1, or 2 for 90°, 180°, 270°.
    pub const fn power(self) -> u8 {
        self as u8 % 3
    }

    pub const fn inverse(self) -> Self {
        let face = self.face();
        let inv_power = (3 - 1 - self.power()) % 3;
        Self::from_index(face * 3 + inv_power)
    }

    pub const fn name(self) -> &'static str {
        MOVE_NAMES[self as usize]
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Bit `m` is set when move `m` may follow `prev`. `None` allows every move.
pub const fn next_move_mask(prev: Option<Move>) -> u32 {
    let Some(prev) = prev else {
        return (1u32 << MOVE_COUNT) - 1;
    };
    let prev_face = prev.face() as i8;
    let mut mask = 0u32;
    let mut m = 0i8;
    while m < MOVE_COUNT as i8 {
        let diff = prev_face - (m / 3);
        if diff != 0 && diff != 3 {
            mask |= 1u32 << m;
        }
        m += 1;
    }
    mask
}

pub const fn move_allowed(prev: Option<Move>, next: Move) -> bool {
    next_move_mask(prev) & (1u32 << next as u8) != 0
}

const fn basic_move(face: usize) -> CubieCube {
    // cp, co, ep, eo copied from Kociemba's basic face turns.
    const CP: [[u8; 8]; 6] = [
        [3, 0, 1, 2, 4, 5, 6, 7],
        [4, 1, 2, 0, 7, 5, 6, 3],
        [1, 5, 2, 3, 0, 4, 6, 7],
        [0, 1, 2, 3, 5, 6, 7, 4],
        [0, 2, 6, 3, 4, 1, 5, 7],
        [0, 1, 3, 7, 4, 5, 2, 6],
    ];
    const CO: [[u8; 8]; 6] = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 1, 1, 0, 0, 2],
        [1, 2, 0, 0, 2, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 1, 2, 0, 0, 2, 1, 0],
        [0, 0, 1, 2, 0, 0, 2, 1],
    ];
    const EP: [[u8; 12]; 6] = [
        [3, 0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11],
        [8, 1, 2, 3, 11, 5, 6, 7, 4, 9, 10, 0],
        [0, 9, 2, 3, 4, 8, 6, 7, 1, 5, 10, 11],
        [0, 1, 2, 3, 5, 6, 7, 4, 8, 9, 10, 11],
        [0, 1, 10, 3, 4, 5, 9, 7, 8, 2, 6, 11],
        [0, 1, 2, 11, 4, 5, 6, 10, 8, 9, 3, 7],
    ];
    const EO: [[u8; 12]; 6] = [
        [0; 12],
        [0; 12],
        [0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0],
        [0; 12],
        [0; 12],
        [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1],
    ];
    CubieCube {
        cp: CP[face],
        co: CO[face],
        ep: EP[face],
        eo: EO[face],
    }
}

/// The 18 face turns, built as the first, second, and third power of each basic move.
pub const MOVE_CUBE: [CubieCube; MOVE_COUNT] = {
    let mut out = [CubieCube::SOLVED; MOVE_COUNT];
    let mut face = 0;
    while face < 6 {
        let basic = basic_move(face);
        let mut acc = CubieCube::SOLVED;
        let mut k = 0;
        while k < 3 {
            acc = acc.multiplied(basic);
            out[face * 3 + k] = acc;
            k += 1;
        }
        face += 1;
    }
    out
};
