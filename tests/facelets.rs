use cube_cli::{
    cubie_from_facelets, facelets_from_cubie, CubeCoords, CubeError, CubieCube, Move, ALL_MOVES,
    SOLVED_FACELETS,
};

#[test]
fn solved_facelets_are_all_zero_coordinates() {
    let cube = cubie_from_facelets(SOLVED_FACELETS).unwrap();
    assert_eq!(cube, CubieCube::solved());
    assert_eq!(cube.coordinates(), CubeCoords::SOLVED);
    assert_eq!(facelets_from_cubie(&cube), SOLVED_FACELETS);
}

#[test]
fn u_then_u_prime_is_identity() {
    let mut cube = cubie_from_facelets(SOLVED_FACELETS).unwrap();
    cube.apply(Move::U1);
    assert_ne!(cube.coordinates(), CubeCoords::SOLVED);
    assert_ne!(facelets_from_cubie(&cube), SOLVED_FACELETS);
    cube.apply(Move::U3);
    assert_eq!(cube, CubieCube::solved());
    assert_eq!(cube.coordinates(), CubeCoords::SOLVED);
    assert_eq!(facelets_from_cubie(&cube), SOLVED_FACELETS);
}

#[test]
fn every_move_roundtrips_through_facelets() {
    for &m in &ALL_MOVES {
        let mut cube = CubieCube::solved();
        cube.apply(m);
        let text = facelets_from_cubie(&cube);
        let parsed = cubie_from_facelets(&text).unwrap();
        assert_eq!(parsed, cube, "{m}");
        cube.apply(m.inverse());
        assert_eq!(facelets_from_cubie(&cube), SOLVED_FACELETS);
    }
}

#[test]
fn twisted_corner_is_rejected() {
    // Cycle URF stickers U9, R1, F3 so the U color leaves the U face.
    let mut chars: Vec<u8> = SOLVED_FACELETS.bytes().collect();
    chars[8] = b'F';
    chars[9] = b'U';
    chars[20] = b'R';
    let text = String::from_utf8(chars).unwrap();
    assert_eq!(cubie_from_facelets(&text).unwrap_err(), CubeError::Twist);
}

#[test]
fn flipped_edge_is_rejected() {
    let mut chars: Vec<u8> = SOLVED_FACELETS.bytes().collect();
    chars.swap(5, 10);
    let text = String::from_utf8(chars).unwrap();
    assert_eq!(cubie_from_facelets(&text).unwrap_err(), CubeError::Flip);
}

#[test]
fn swapped_edges_are_a_parity_error() {
    // UR side sticker R2 and UF side sticker F2 trade places: one edge transposition.
    let mut chars: Vec<u8> = SOLVED_FACELETS.bytes().collect();
    chars.swap(10, 19);
    let text = String::from_utf8(chars).unwrap();
    assert_eq!(cubie_from_facelets(&text).unwrap_err(), CubeError::Parity);
}

#[test]
fn bad_length_and_bad_color_are_errors() {
    assert_eq!(
        cubie_from_facelets("UUU").unwrap_err(),
        CubeError::FaceletLength(3)
    );
    let mut chars: Vec<u8> = SOLVED_FACELETS.bytes().collect();
    chars[0] = b'X';
    assert_eq!(
        cubie_from_facelets(&String::from_utf8(chars).unwrap()).unwrap_err(),
        CubeError::FaceletColor
    );
}
