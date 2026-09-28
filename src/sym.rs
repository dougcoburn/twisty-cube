//! The 16 UD-preserving cube symmetries (D4h) and the 120° rotation that
//! cycles the UD, RL, and FB axes.
//!
//! Phase-1 distance is stored once, for the UD axis, on symmetry classes of
//! the flip-slice coordinate. There are 64430 classes, so the exact distance
//! table is `64430 * 2187` bytes (about 134 MB). Another axis is the same
//! lookup after conjugation by `ROT_URF3` or its square.
//!
//! Corner orientations of 3 or more mark a mirrored cubie, matching
//! Kociemba's multiply. Legal face turns never use that branch.

use std::sync::OnceLock;

use crate::coords::{decode_orientation, encode_orientation, ud_slice};
use crate::CubieCube;

pub const N_SYM: usize = 16;
pub const N_FLIPSLICE_CLASS: usize = 64_430;
pub const N_FLIPSLICE: usize = 2048 * 495;
pub const N_PHASE1: usize = N_FLIPSLICE_CLASS * 2187;

const URF: u8 = 0;
const UFL: u8 = 1;
const ULB: u8 = 2;
const UBR: u8 = 3;
const DFR: u8 = 4;
const DLF: u8 = 5;
const DBL: u8 = 6;
const DRB: u8 = 7;

const UR: u8 = 0;
const UF: u8 = 1;
const UL: u8 = 2;
const UB: u8 = 3;
const DR: u8 = 4;
const DF: u8 = 5;
const DL: u8 = 6;
const DB: u8 = 7;
const FR: u8 = 8;
const FL: u8 = 9;
const BL: u8 = 10;
const BR: u8 = 11;

#[derive(Clone, Copy)]
struct Sym {
    cp: [u8; 8],
    co: [u8; 8],
    ep: [u8; 12],
    eo: [u8; 12],
}

impl Sym {
    const fn id() -> Self {
        Self {
            cp: [0, 1, 2, 3, 4, 5, 6, 7],
            co: [0; 8],
            ep: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            eo: [0; 12],
        }
    }

    fn mul(self, b: Self) -> Self {
        let mut cp = [0u8; 8];
        let mut co = [0u8; 8];
        for i in 0..8 {
            let from = b.cp[i] as usize;
            cp[i] = self.cp[from];
            co[i] = corner_ori(self.co[from], b.co[i]);
        }
        let mut ep = [0u8; 12];
        let mut eo = [0u8; 12];
        for i in 0..12 {
            let from = b.ep[i] as usize;
            ep[i] = self.ep[from];
            eo[i] = (b.eo[i] + self.eo[from]) & 1;
        }
        Self { cp, co, ep, eo }
    }

    fn twist(self) -> u16 {
        encode_orientation(&self.co, 3) as u16
    }

    fn flip(self) -> u16 {
        encode_orientation(&self.eo, 2) as u16
    }

    fn slice(self) -> u16 {
        ud_slice(&self.ep)
    }
}

fn corner_ori(ori_a: u8, ori_b: u8) -> u8 {
    if ori_a < 3 && ori_b < 3 {
        let ori = ori_a + ori_b;
        if ori >= 3 {
            ori - 3
        } else {
            ori
        }
    } else if ori_a < 3 && ori_b >= 3 {
        let ori = ori_a + ori_b;
        if ori >= 6 {
            ori - 3
        } else {
            ori
        }
    } else if ori_a >= 3 && ori_b < 3 {
        let mut ori = i16::from(ori_a) - i16::from(ori_b);
        if ori < 3 {
            ori += 3;
        }
        ori as u8
    } else {
        let mut ori = i16::from(ori_a) - i16::from(ori_b);
        if ori < 0 {
            ori += 3;
        }
        ori as u8
    }
}

fn basic_syms() -> [Sym; 4] {
    let urf3 = Sym {
        cp: [URF, DFR, DLF, UFL, UBR, DRB, DBL, ULB],
        co: [1, 2, 1, 2, 2, 1, 2, 1],
        ep: [UF, FR, DF, FL, UB, BR, DB, BL, UR, DR, DL, UL],
        eo: [1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1],
    };
    let f2 = Sym {
        cp: [DLF, DFR, DRB, DBL, UFL, URF, UBR, ULB],
        co: [0; 8],
        ep: [DL, DF, DR, DB, UL, UF, UR, UB, FL, FR, BR, BL],
        eo: [0; 12],
    };
    let u4 = Sym {
        cp: [UBR, URF, UFL, ULB, DRB, DFR, DLF, DBL],
        co: [0; 8],
        ep: [UB, UR, UF, UL, DB, DR, DF, DL, BR, FR, FL, BL],
        eo: [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
    };
    let mirr = Sym {
        cp: [UFL, URF, UBR, ULB, DLF, DFR, DRB, DBL],
        co: [3; 8],
        ep: [UL, UF, UR, UB, DL, DF, DR, DB, FL, FR, BR, BL],
        eo: [0; 12],
    };
    [urf3, f2, u4, mirr]
}

fn d4h() -> [Sym; N_SYM] {
    let [_urf3, f2, u4, mirr] = basic_syms();
    let mut cc = Sym::id();
    let mut out = [Sym::id(); N_SYM];
    let mut n = 0;
    // The first 16 symmetries of the 48-element loop. They preserve the UD axis.
    for _f2 in 0..2 {
        for _u4 in 0..4 {
            for _lr2 in 0..2 {
                out[n] = cc;
                n += 1;
                cc = cc.mul(mirr);
            }
            cc = cc.mul(u4);
        }
        cc = cc.mul(f2);
    }
    debug_assert_eq!(n, N_SYM);
    out
}

fn inverse_index(syms: &[Sym; N_SYM]) -> [usize; N_SYM] {
    let mut inv = [0; N_SYM];
    for j in 0..N_SYM {
        let mut found = false;
        for i in 0..N_SYM {
            let product = syms[j].mul(syms[i]);
            if product.cp[0] == URF && product.cp[1] == UFL && product.cp[2] == ULB {
                inv[j] = i;
                found = true;
                break;
            }
        }
        assert!(found, "symmetry {j} has no inverse in D4h");
    }
    inv
}

fn flip_slice_cube(flip: u16, slice: u16) -> Sym {
    let mut cube = Sym::id();
    cube.ep = crate::coords::edges_from_ud_slice(slice);
    decode_orientation(u32::from(flip), 2, &mut cube.eo);
    cube
}

/// Symmetry reduction of flip-slice, plus twist conjugation by D4h.
pub struct SymIndex {
    pub classidx: Vec<u16>,
    pub sym: Vec<u8>,
    pub twist_conj: Vec<u16>,
    pub stab: Vec<u16>,
    pub rep_flip: Vec<u16>,
    pub rep_slice: Vec<u16>,
    pub urf3: CubieCube,
    pub urf3_inv: CubieCube,
    pub urf3_sq: CubieCube,
}

pub fn sym_index() -> &'static SymIndex {
    static INDEX: OnceLock<SymIndex> = OnceLock::new();
    INDEX.get_or_init(build_sym_index)
}

fn build_sym_index() -> SymIndex {
    let syms = d4h();
    let inv = inverse_index(&syms);
    let mut classidx = vec![u16::MAX; N_FLIPSLICE];
    let mut sym_of = vec![0u8; N_FLIPSLICE];
    let mut rep_flip = Vec::new();
    let mut rep_slice = Vec::new();

    for slice in 0..495u16 {
        for flip in 0..2048u16 {
            let idx = slice as usize * 2048 + flip as usize;
            if classidx[idx] != u16::MAX {
                continue;
            }
            let class = rep_flip.len() as u16;
            classidx[idx] = class;
            sym_of[idx] = 0;
            rep_flip.push(flip);
            rep_slice.push(slice);
            let cc = flip_slice_cube(flip, slice);
            for s in 0..N_SYM {
                let image = syms[inv[s]].mul(cc).mul(syms[s]);
                let idx_new = image.slice() as usize * 2048 + image.flip() as usize;
                if classidx[idx_new] == u16::MAX {
                    classidx[idx_new] = class;
                    sym_of[idx_new] = s as u8;
                }
            }
        }
    }
    assert_eq!(
        rep_flip.len(),
        N_FLIPSLICE_CLASS,
        "D4h flip-slice class count"
    );
    assert!(classidx.iter().all(|&c| c != u16::MAX));

    let mut twist_conj = vec![0u16; 2187 * N_SYM];
    for twist in 0..2187u16 {
        let mut twist_cube = Sym::id();
        decode_orientation(u32::from(twist), 3, &mut twist_cube.co);
        for s in 0..N_SYM {
            let image = syms[s].mul(twist_cube).mul(syms[inv[s]]);
            twist_conj[twist as usize * N_SYM + s] = image.twist();
        }
    }

    let mut stab = vec![0u16; N_FLIPSLICE_CLASS];
    for class in 0..N_FLIPSLICE_CLASS {
        let cc = flip_slice_cube(rep_flip[class], rep_slice[class]);
        let mut mask = 0u16;
        for s in 0..N_SYM {
            let image = syms[s].mul(cc).mul(syms[inv[s]]);
            if image.slice() == rep_slice[class] && image.flip() == rep_flip[class] {
                mask |= 1 << s;
            }
        }
        stab[class] = mask;
    }

    let urf3 = cubie_from_sym(basic_syms()[0]);
    let urf3_inv = urf3.inverse();
    let urf3_sq = urf3.multiplied(urf3);
    SymIndex {
        classidx,
        sym: sym_of,
        twist_conj,
        stab,
        rep_flip,
        rep_slice,
        urf3,
        urf3_inv,
        urf3_sq,
    }
}

fn cubie_from_sym(sym: Sym) -> CubieCube {
    CubieCube {
        cp: sym.cp,
        co: sym.co,
        ep: sym.ep,
        eo: sym.eo,
    }
}

impl SymIndex {
    /// Exact distance to H_UD. `prun` is `phase1_prun.bin`.
    pub fn phase1_ud(&self, prun: &[u8], twist: u16, flip: u16, slice: u16) -> u8 {
        let fs = slice as usize * 2048 + flip as usize;
        let class = self.classidx[fs] as usize;
        let sym = self.sym[fs] as usize;
        let twist_c = self.twist_conj[twist as usize * N_SYM + sym] as usize;
        prun[class * 2187 + twist_c]
    }
}

/// `s * cube * s_inv`: the cube written in the rotated frame.
pub fn conjugate(s: &CubieCube, cube: &CubieCube, s_inv: &CubieCube) -> CubieCube {
    s.multiplied(*cube).multiplied(*s_inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_have_finite_order_and_d4h_has_64430_classes() {
        let [urf3, f2, u4, mirr] = basic_syms();
        assert_eq!(urf3.mul(urf3).mul(urf3).cp, Sym::id().cp);
        assert_eq!(f2.mul(f2).cp, Sym::id().cp);
        assert_eq!(u4.mul(u4).mul(u4).mul(u4).cp, Sym::id().cp);
        assert_eq!(mirr.mul(mirr).cp, Sym::id().cp);
        assert_eq!(mirr.mul(mirr).co, Sym::id().co);
        let index = sym_index();
        assert_eq!(index.rep_flip.len(), N_FLIPSLICE_CLASS);
        assert_eq!(index.classidx[0], 0);
        assert_eq!(index.twist_conj[0], 0);
        assert_ne!(index.stab[0], 0);
    }

    #[test]
    fn urf3_sends_r_and_f_into_the_ud_subgroup() {
        use crate::moves::{Move, MOVE_CUBE};
        let urf3 = cubie_from_sym(basic_syms()[0]);
        let urf3_inv = urf3.inverse();
        let urf3_sq = urf3.multiplied(urf3);
        assert_eq!(urf3.multiplied(urf3).multiplied(urf3), CubieCube::solved());
        let in_h = |c: &CubieCube| c.twist() == 0 && c.flip() == 0 && c.slice() == 0;
        let image_r = conjugate(&urf3, &MOVE_CUBE[Move::R1 as usize], &urf3_inv);
        assert!(in_h(&image_r));
        let image_f = conjugate(&urf3_sq, &MOVE_CUBE[Move::F1 as usize], &urf3);
        assert!(in_h(&image_f));
        let image_u = conjugate(&urf3, &MOVE_CUBE[Move::U1 as usize], &urf3_inv);
        assert!(!in_h(&image_u));
    }
}
