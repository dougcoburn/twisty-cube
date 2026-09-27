use cube_cli::{next_move_mask, CubieCube, Move, ALL_MOVES, MOVE_CUBE};

#[test]
fn move_then_inverse_is_identity() {
    for &m in &ALL_MOVES {
        let mut cube = CubieCube::solved();
        cube.apply(m);
        cube.apply(m.inverse());
        assert_eq!(
            cube,
            CubieCube::solved(),
            "{m} then {} left a twist",
            m.inverse()
        );
    }
}

#[test]
fn group_inverse_matches_the_third_power() {
    for &m in &ALL_MOVES {
        let forward = MOVE_CUBE[m as usize];
        let backward = MOVE_CUBE[m.inverse() as usize];
        assert_eq!(forward.inverse(), backward, "{m}");
        assert_eq!(forward.multiplied(backward), CubieCube::solved());
        assert_eq!(backward.multiplied(forward), CubieCube::solved());
    }
}

#[test]
fn four_applications_are_identity() {
    for &m in &ALL_MOVES {
        let mut cube = CubieCube::solved();
        cube.apply(m);
        cube.apply(m);
        cube.apply(m);
        cube.apply(m);
        assert_eq!(cube, CubieCube::solved(), "{m}^4");
    }
}

#[test]
fn half_turns_square_to_identity_and_match_two_quarters() {
    for face in 0..6u8 {
        let quarter = Move::from_index(face * 3);
        let half = Move::from_index(face * 3 + 1);
        let mut twice = CubieCube::solved();
        twice.apply(quarter);
        twice.apply(quarter);
        assert_eq!(twice, MOVE_CUBE[half as usize]);
        assert_eq!(
            MOVE_CUBE[half as usize].multiplied(MOVE_CUBE[half as usize]),
            CubieCube::solved()
        );
    }
}

#[test]
fn face_turns_match_kociemba_basic_cycles() {
    let u = &MOVE_CUBE[Move::U1 as usize];
    assert_eq!(u.cp, [3, 0, 1, 2, 4, 5, 6, 7]);
    assert_eq!(u.ep, [3, 0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11]);
    assert_eq!(u.co, [0; 8]);
    assert_eq!(u.eo, [0; 12]);

    let f = &MOVE_CUBE[Move::F1 as usize];
    assert_eq!(f.co, [1, 2, 0, 0, 2, 1, 0, 0]);
    assert_eq!(f.eo, [0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0]);
}

#[test]
fn six_faces_and_their_inverses_cancel() {
    let mut cube = CubieCube::solved();
    let maneuver = [Move::U1, Move::R1, Move::F1, Move::D1, Move::L1, Move::B1];
    for &m in &maneuver {
        cube.apply(m);
    }
    for &m in maneuver.iter().rev() {
        cube.apply(m.inverse());
    }
    assert_eq!(cube, CubieCube::solved());
}

#[test]
fn successor_mask_drops_same_face_and_the_wrong_opposite_order() {
    assert_eq!(next_move_mask(None).count_ones(), 18);

    // U, R, F: only the same face is forbidden (15 left). D, L, B also forbid
    // the opposite face so that U-D, R-L, F-B stay in one order (12 left).
    for (face, expect) in [(0u8, 15u32), (1, 15), (2, 15), (3, 12), (4, 12), (5, 12)] {
        let prev = Move::from_index(face * 3);
        let mask = next_move_mask(Some(prev));
        assert_eq!(mask.count_ones(), expect, "{prev}");
        for power in 0..3u8 {
            let same = Move::from_index(face * 3 + power);
            assert_eq!(mask & (1 << same as u8), 0);
        }
    }

    let after_d = next_move_mask(Some(Move::D1));
    assert_eq!(after_d & (1 << Move::U1 as u8), 0);
    assert_ne!(next_move_mask(Some(Move::U1)) & (1 << Move::D1 as u8), 0);
}
