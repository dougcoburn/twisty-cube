//! Corner and edge cubies, in Kociemba's order.
//!
//! `cp[i]` / `ep[i]` is the cubie sitting in position `i`.
//! `co[i]` is that corner's twist in `0..3`; `eo[i]` is that edge's flip in `0..1`.

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

    pub fn with_twist(twist: u16) -> Self {
        let mut cube = Self::solved();
        crate::coords::decode_orientation(u32::from(twist), 3, &mut cube.co);
        cube
    }

    pub fn with_flip(flip: u16) -> Self {
        let mut cube = Self::solved();
        crate::coords::decode_orientation(u32::from(flip), 2, &mut cube.eo);
        cube
    }

    pub fn with_slice(slice: u16) -> Self {
        let mut cube = Self::solved();
        cube.ep = crate::coords::edges_from_ud_slice(slice);
        cube
    }

    pub fn with_slice_sorted(idx: u16) -> Self {
        let mut cube = Self::solved();
        cube.ep = crate::coords::edges_from_slice_sorted(idx);
        cube
    }

    pub fn with_corners(corners: u16) -> Self {
        let mut cube = Self::solved();
        crate::coords::unrank_permutation(8, u32::from(corners), &mut cube.cp);
        cube
    }

    pub fn with_ud_edges(ud: u16) -> Self {
        let mut cube = Self::solved();
        cube.ep = crate::coords::edges_from_ud_edges(ud);
        cube
    }

    pub fn twist(&self) -> u16 {
        crate::coords::encode_orientation(&self.co, 3) as u16
    }

    pub fn flip(&self) -> u16 {
        crate::coords::encode_orientation(&self.eo, 2) as u16
    }

    pub fn slice(&self) -> u16 {
        crate::coords::ud_slice(&self.ep)
    }

    pub fn corners(&self) -> u16 {
        crate::coords::rank_permutation(&self.cp) as u16
    }

    /// Permutation coordinate of the eight U/D edges. Valid when those edges
    /// occupy the eight U/D slots (every position in subgroup H).
    pub fn ud_edges(&self) -> u16 {
        crate::coords::ud_edges(&self.ep)
    }

    pub fn slice_sorted(&self) -> u16 {
        crate::coords::ud_slice_sorted(&self.ep)
    }

    pub fn coordinates(&self) -> CubeCoords {
        CubeCoords {
            twist: self.twist(),
            flip: self.flip(),
            slice: self.slice(),
            corners: self.corners(),
            ud_edges: self.ud_edges(),
            slice_sorted: self.slice_sorted(),
        }
    }

    pub fn corner_parity(&self) -> u8 {
        crate::coords::permutation_parity(&self.cp)
    }

    pub fn edge_parity(&self) -> u8 {
        crate::coords::permutation_parity(&self.ep)
    }

    /// Piece identity, twist, flip, and permutation parity.
    pub fn check(&self) -> Result<(), CubeError> {
        let mut seen_c = [false; CORNER_COUNT];
        for &cubie in &self.cp {
            if cubie as usize >= CORNER_COUNT || seen_c[cubie as usize] {
                return Err(CubeError::UndefinedCubie);
            }
            seen_c[cubie as usize] = true;
        }
        if self.co.iter().any(|&o| o > 2)
            || self.co.iter().map(|&o| u32::from(o)).sum::<u32>() % 3 != 0
        {
            return Err(CubeError::Twist);
        }

        let mut seen_e = [false; EDGE_COUNT];
        for &cubie in &self.ep {
            if cubie as usize >= EDGE_COUNT || seen_e[cubie as usize] {
                return Err(CubeError::UndefinedCubie);
            }
            seen_e[cubie as usize] = true;
        }
        if self.eo.iter().any(|&o| o > 1)
            || self.eo.iter().map(|&o| u32::from(o)).sum::<u32>() % 2 != 0
        {
            return Err(CubeError::Flip);
        }
        if self.corner_parity() != self.edge_parity() {
            return Err(CubeError::Parity);
        }
        Ok(())
    }
}

/// Phase-1 and phase-2 coordinates. All six are 0 on the solved cube.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubeCoords {
    pub twist: u16,
    pub flip: u16,
    pub slice: u16,
    pub corners: u16,
    pub ud_edges: u16,
    pub slice_sorted: u16,
}

impl CubeCoords {
    pub const SOLVED: Self = Self {
        twist: 0,
        flip: 0,
        slice: 0,
        corners: 0,
        ud_edges: 0,
        slice_sorted: 0,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CubeError {
    FaceletLength(usize),
    FaceletColor,
    UndefinedCubie,
    Twist,
    Flip,
    Parity,
}

impl std::fmt::Display for CubeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CubeError::FaceletLength(n) => {
                write!(f, "facelet string has length {n}, expected 54")
            }
            CubeError::FaceletColor => {
                write!(
                    f,
                    "facelets must use URFDLB with each color exactly 9 times"
                )
            }
            CubeError::UndefinedCubie => {
                write!(f, "some corner or edge cubie is missing or duplicated")
            }
            CubeError::Twist => write!(f, "total corner twist is not divisible by 3"),
            CubeError::Flip => write!(f, "total edge flip is odd"),
            CubeError::Parity => write!(f, "corner and edge permutation parities differ"),
        }
    }
}

impl std::error::Error for CubeError {}
