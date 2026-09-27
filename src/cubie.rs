//! Corner and edge cubies, in Kociemba's order.
//!
//! `cp[i]` / `ep[i]` is the cubie sitting in position `i`.
//! `co[i]` is that corner's twist in `0..3`; `eo[i]` is that edge's flip in `0..2`.

/// Corners: URF UFL ULB UBR DFR DLF DBL DRB.
pub const CORNER_COUNT: usize = 8;
/// Edges: UR UF UL UB DR DF DL DB FR FL BL BR.
pub const EDGE_COUNT: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubieCube {
    pub cp: [u8; CORNER_COUNT],
    pub co: [u8; CORNER_COUNT],
    pub ep: [u8; EDGE_COUNT],
    pub eo: [u8; EDGE_COUNT],
}

impl CubieCube {
    pub const SOLVED: Self = Self {
        cp: [0, 1, 2, 3, 4, 5, 6, 7],
        co: [0; CORNER_COUNT],
        ep: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        eo: [0; EDGE_COUNT],
    };

    pub const fn solved() -> Self {
        Self::SOLVED
    }

    /// `self * b` in Kociemba's order: `b` is applied to the current cube.
    pub const fn multiplied(self, b: Self) -> Self {
        let mut cp = [0u8; CORNER_COUNT];
        let mut co = [0u8; CORNER_COUNT];
        let mut i = 0;
        while i < CORNER_COUNT {
            let from = b.cp[i] as usize;
            cp[i] = self.cp[from];
            co[i] = (self.co[from] + b.co[i]) % 3;
            i += 1;
        }

        let mut ep = [0u8; EDGE_COUNT];
        let mut eo = [0u8; EDGE_COUNT];
        let mut e = 0;
        while e < EDGE_COUNT {
            let from = b.ep[e] as usize;
            ep[e] = self.ep[from];
            eo[e] = (b.eo[e] + self.eo[from]) & 1;
            e += 1;
        }

        Self { cp, co, ep, eo }
    }

    pub const fn inverse(self) -> Self {
        let mut cp = [0u8; CORNER_COUNT];
        let mut ep = [0u8; EDGE_COUNT];
        let mut i = 0;
        while i < CORNER_COUNT {
            cp[self.cp[i] as usize] = i as u8;
            i += 1;
        }
        i = 0;
        while i < EDGE_COUNT {
            ep[self.ep[i] as usize] = i as u8;
            i += 1;
        }

        let mut co = [0u8; CORNER_COUNT];
        let mut eo = [0u8; EDGE_COUNT];
        i = 0;
        while i < CORNER_COUNT {
            let ori = self.co[cp[i] as usize] % 3;
            co[i] = (3 - ori) % 3;
            i += 1;
        }
        i = 0;
        while i < EDGE_COUNT {
            eo[i] = self.eo[ep[i] as usize] & 1;
            i += 1;
        }

        Self { cp, co, ep, eo }
    }

    pub fn apply(&mut self, m: crate::moves::Move) {
        *self = self.multiplied(crate::moves::MOVE_CUBE[m as usize]);
    }
}
