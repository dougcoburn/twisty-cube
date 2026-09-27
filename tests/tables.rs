use std::fs;
use std::path::PathBuf;

use cube_cli::{
    load_or_generate, CubieCube, Move, ALL_MOVES, N_FLIP, N_SLICE, N_SLICE_SORTED, N_TWIST,
    PHASE2_MOVES, TABLE_BUDGET, UD_EDGES_INVALID,
};

fn scratch() -> PathBuf {
    let path = std::env::temp_dir().join(format!("cube-cli-tables-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

#[test]
fn tables_generate_then_mmap_and_match_cubie_moves() {
    let dir = scratch();
    let generated = load_or_generate(&dir).unwrap();
    assert!(!generated.mapped);
    assert!(generated.total_bytes() < TABLE_BUDGET);
    assert!(generated.total_bytes() > 1_000_000);

    let tables = &generated.tables;
    for (name, depth, unseen) in tables.prune_summary() {
        assert_eq!(unseen, 0, "{name}");
        assert!(depth >= 1, "{name}");
    }
    let summary = tables.prune_summary();
    assert!(summary[0].1 <= 12, "twist×slice depth {}", summary[0].1);
    assert!(summary[1].1 <= 12, "flip×slice depth {}", summary[1].1);
    assert!(summary[2].1 <= 18, "corners depth {}", summary[2].1);
    assert!(summary[3].1 <= 18, "ud-edges depth {}", summary[3].1);

    assert_eq!(tables.prune_twist_slice(0, 0), 0);
    assert_eq!(tables.prune_flip_slice(0, 0), 0);
    assert_eq!(tables.prune_corners(0, 0), 0);
    assert_eq!(tables.prune_ud_edges(0, 0), 0);

    let f = apply_move(CubieCube::solved(), Move::F1);
    assert_eq!(tables.prune_twist_slice(f.twist(), f.slice()), 1);
    assert_eq!(tables.twist_move(0, Move::U1), 0);
    assert_eq!(tables.slice_move(0, Move::U1), 0);
    assert_ne!(tables.corners_move(0, Move::U1), 0);

    for twist in 0..N_TWIST as u16 {
        let cube = CubieCube::with_twist(twist);
        for &mv in &ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            assert_eq!(tables.twist_move(twist, mv), next.twist());
        }
    }
    for flip in (0..N_FLIP as u16).step_by(3) {
        let cube = CubieCube::with_flip(flip);
        for &mv in &ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            assert_eq!(tables.flip_move(flip, mv), next.flip());
        }
    }
    for slice in 0..N_SLICE as u16 {
        let cube = CubieCube::with_slice(slice);
        for &mv in &ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            assert_eq!(tables.slice_move(slice, mv), next.slice());
        }
    }
    for idx in (0..N_SLICE_SORTED as u16).step_by(17) {
        let cube = CubieCube::with_slice_sorted(idx);
        for &mv in &ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            assert_eq!(tables.slice_sorted_move(idx, mv), next.slice_sorted());
            if idx < 24 && PHASE2_MOVES.contains(&mv) {
                assert!(tables.slice_sorted_move(idx, mv) < 24);
            }
        }
    }

    let mut state = 1u32;
    for _ in 0..200 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let corners = (state % 40_320) as u16;
        let cube = CubieCube::with_corners(corners);
        for &mv in &ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            assert_eq!(tables.corners_move(corners, mv), next.corners());
        }
        let ud = ((state >> 8) % 40_320) as u16;
        let cube = CubieCube::with_ud_edges(ud);
        for &mv in &ALL_MOVES {
            if PHASE2_MOVES.contains(&mv) {
                let mut next = cube;
                next.apply(mv);
                assert_eq!(tables.ud_edges_move(ud, mv), next.ud_edges());
            } else {
                assert_eq!(tables.ud_edges_move(ud, mv), UD_EDGES_INVALID);
            }
        }
    }

    let stamp = fs::metadata(dir.join("twist_move.bin"))
        .unwrap()
        .modified()
        .unwrap();
    drop(generated);
    // The filesystem timestamp resolution can miss a same-second rewrite.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let mapped = load_or_generate(&dir).unwrap();
    assert!(mapped.mapped);
    assert_eq!(
        mapped.total_bytes(),
        mapped.files.iter().map(|(_, n)| n).sum()
    );
    let stamp_after = fs::metadata(dir.join("twist_move.bin"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(stamp, stamp_after);
    assert_eq!(mapped.tables.twist_move(0, Move::F1), f.twist());
    assert_eq!(mapped.tables.prune_twist_slice(f.twist(), f.slice()), 1);

    let _ = fs::remove_dir_all(&dir);
}

fn apply_move(mut cube: CubieCube, mv: Move) -> CubieCube {
    cube.apply(mv);
    cube
}
