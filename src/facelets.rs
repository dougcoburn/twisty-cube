//! 54-character facelet strings in URFDLB order.
//!
//! Each face is nine stickers, left to right and top to bottom with that face
//! toward you. Centers are U5 R5 F5 D5 L5 B5. Corner and edge facelet triples
//! are Kociemba's `cornerFacelet` / `edgeFacelet`.

use crate::{CubeError, CubieCube};

pub const FACELET_COUNT: usize = 54;
pub const SOLVED_FACELETS: &str = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";

const COLOR_CHAR: [u8; 6] = [b'U', b'R', b'F', b'D', b'L', b'B'];

/// Facelets of each corner position, in the orientation-0 color order.
const CORNER_FACELET: [[usize; 3]; 8] = [
    [8, 9, 20],
    [6, 18, 38],
    [0, 36, 47],
    [2, 45, 11],
    [29, 26, 15],
    [27, 44, 24],
    [33, 53, 42],
    [35, 17, 51],
];

/// Facelets of each edge position, in the orientation-0 color order.
const EDGE_FACELET: [[usize; 2]; 12] = [
    [5, 10],
    [7, 19],
    [3, 37],
    [1, 46],
    [32, 16],
    [28, 25],
    [30, 43],
    [34, 52],
    [23, 12],
    [21, 41],
    [50, 39],
    [48, 14],
];

/// Colors of corner cubie j, U/D color first.
const CORNER_COLOR: [[u8; 3]; 8] = [
    [0, 1, 2],
    [0, 2, 4],
    [0, 4, 5],
    [0, 5, 1],
    [3, 2, 1],
    [3, 4, 2],
    [3, 5, 4],
    [3, 1, 5],
];

const EDGE_COLOR: [[u8; 2]; 12] = [
    [0, 1],
    [0, 2],
    [0, 4],
    [0, 5],
    [3, 1],
    [3, 2],
    [3, 4],
    [3, 5],
    [2, 1],
    [2, 4],
    [5, 4],
    [5, 1],
];

pub fn cubie_from_facelets(facelets: &str) -> Result<CubieCube, CubeError> {
    if facelets.len() != FACELET_COUNT {
        return Err(CubeError::FaceletLength(facelets.len()));
    }
    let bytes = facelets.as_bytes();
    let mut color = [0u8; FACELET_COUNT];
    let mut count = [0u8; 6];
    for (i, &byte) in bytes.iter().enumerate() {
        let Some(c) = color_index(byte) else {
            return Err(CubeError::FaceletColor);
        };
        color[i] = c;
        count[c as usize] += 1;
    }
    if count != [9; 6] {
        return Err(CubeError::FaceletColor);
    }
    for face in 0..6 {
        if color[face * 9 + 4] != face as u8 {
            return Err(CubeError::Center);
        }
    }

    let mut cube = CubieCube::solved();
    cube.cp = [255; 8];
    cube.ep = [255; 12];
    cube.co = [0; 8];
    cube.eo = [0; 12];

    for i in 0..8 {
        let fac = CORNER_FACELET[i];
        let Some(ori) = (0..3).find(|&o| color[fac[o]] == 0 || color[fac[o]] == 3) else {
            return Err(CubeError::UndefinedCubie);
        };
        let ud = color[fac[ori]];
        let col1 = color[fac[(ori + 1) % 3]];
        let col2 = color[fac[(ori + 2) % 3]];
        let Some(j) = (0..8).find(|&j| {
            CORNER_COLOR[j][0] == ud && CORNER_COLOR[j][1] == col1 && CORNER_COLOR[j][2] == col2
        }) else {
            return Err(CubeError::UndefinedCubie);
        };
        cube.cp[i] = j as u8;
        cube.co[i] = ori as u8;
    }

    for i in 0..12 {
        let fac = EDGE_FACELET[i];
        let c0 = color[fac[0]];
        let c1 = color[fac[1]];
        let mut found = false;
        for j in 0..12 {
            if c0 == EDGE_COLOR[j][0] && c1 == EDGE_COLOR[j][1] {
                cube.ep[i] = j as u8;
                cube.eo[i] = 0;
                found = true;
                break;
            }
            if c0 == EDGE_COLOR[j][1] && c1 == EDGE_COLOR[j][0] {
                cube.ep[i] = j as u8;
                cube.eo[i] = 1;
                found = true;
                break;
            }
        }
        if !found {
            return Err(CubeError::UndefinedCubie);
        }
    }

    cube.check()?;
    Ok(cube)
}

pub fn facelets_from_cubie(cube: &CubieCube) -> String {
    let mut face = [b'?'; FACELET_COUNT];
    for face_i in 0..6 {
        face[face_i * 9 + 4] = COLOR_CHAR[face_i];
    }
    for i in 0..8 {
        let j = cube.cp[i] as usize;
        let ori = cube.co[i] as usize;
        for k in 0..3 {
            let slot = CORNER_FACELET[i][(k + ori) % 3];
            face[slot] = COLOR_CHAR[CORNER_COLOR[j][k] as usize];
        }
    }
    for i in 0..12 {
        let j = cube.ep[i] as usize;
        let ori = cube.eo[i] as usize;
        for k in 0..2 {
            let slot = EDGE_FACELET[i][(k + ori) % 2];
            face[slot] = COLOR_CHAR[EDGE_COLOR[j][k] as usize];
        }
    }
    face.iter().map(|&b| b as char).collect()
}

fn color_index(byte: u8) -> Option<u8> {
    COLOR_CHAR.iter().position(|&c| c == byte).map(|i| i as u8)
}
