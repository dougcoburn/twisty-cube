use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::Instant;

use cube_cli::{
    format_report, load_optimal, load_or_generate, move_allowed, parse_moves, reid_h, solve,
    solve_optimal, CubieCube, Move, OptimalError, ALL_MOVES,
};

fn tables() -> &'static cube_cli::Tables {
    static LOADED: OnceLock<cube_cli::LoadedTables> = OnceLock::new();
    &LOADED
        .get_or_init(|| {
            let dir = std::env::temp_dir().join("cube-cli-solve-tables");
            load_optimal(&dir).expect("tables")
        })
        .tables
}

fn apply_all(moves: &str) -> CubieCube {
    let mut cube = CubieCube::solved();
    for mv in parse_moves(moves).unwrap() {
        cube.apply(mv);
    }
    cube
}

fn assert_solves(start: &CubieCube, moves: &[Move]) {
    let mut cube = *start;
    for mv in moves {
        cube.apply(*mv);
    }
    assert_eq!(cube, CubieCube::solved());
}

#[test]
fn heuristic_is_zero_only_on_solved_and_one_on_a_single_turn() {
    let tables = tables();
    assert_eq!(reid_h(&CubieCube::solved(), tables).unwrap(), 0);
    let mut turned = CubieCube::solved();
    turned.apply(Move::U1);
    assert_eq!(reid_h(&turned, tables).unwrap(), 1);
    for &mv in &ALL_MOVES {
        let mut cube = CubieCube::solved();
        cube.apply(mv);
        let h = reid_h(&cube, tables).unwrap();
        if mv.power() == 1 {
            assert!(h <= 1, "{mv} h={h}");
        } else {
            assert_eq!(h, 1, "{mv}");
        }
        let solution = solve_optimal(&cube, tables, 20).unwrap();
        assert_eq!(solution.moves, vec![mv.inverse()]);
        assert_solves(&cube, &solution.moves);
    }

    let mut seen = std::collections::HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((CubieCube::solved(), 0u8, None));
    while let Some((cube, depth, prev)) = queue.pop_front() {
        let h = reid_h(&cube, tables).unwrap();
        assert!(h as u8 <= depth || depth == 0);
        if depth == 3 || seen.len() > 8_000 {
            continue;
        }
        for &mv in &ALL_MOVES {
            if !move_allowed(prev, mv) {
                continue;
            }
            let mut next = cube;
            next.apply(mv);
            if seen.insert(next) {
                queue.push_back((next, depth + 1, Some(mv)));
            }
        }
    }

    let superflip = apply_all("U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2");
    assert!(superflip.cp == CubieCube::solved().cp);
    assert!(superflip.co == CubieCube::solved().co);
    assert!(superflip.eo.iter().all(|&o| o == 1));
    let h = reid_h(&superflip, tables).unwrap();
    assert!(h >= 8 && h < 20, "superflip h = {h}");
}

#[test]
fn optimal_solved_and_u_are_immediate() {
    let tables = tables();
    let solved = solve_optimal(&CubieCube::solved(), tables, 20).unwrap();
    assert!(solved.moves.is_empty());
    assert_eq!(
        format_report(&solved.moves, true),
        "length: 0\noptimal: true\n"
    );

    let mut u = CubieCube::solved();
    u.apply(Move::U1);
    let start = Instant::now();
    let solution = solve_optimal(&u, tables, 20).unwrap();
    assert!(start.elapsed().as_millis() < 500, "{:?}", start.elapsed());
    assert_eq!(
        solution
            .moves
            .iter()
            .map(|mv| mv.name())
            .collect::<Vec<_>>(),
        vec!["U'"]
    );
    assert_solves(&u, &solution.moves);
    assert_eq!(
        format_report(&solution.moves, true),
        "U'\nlength: 1\noptimal: true\n"
    );

    let two = solve(&u, tables).unwrap();
    assert_eq!(two, solution.moves);
    assert_eq!(
        format_report(&two, false),
        "U'\nlength: 1\noptimal: false\n"
    );
}

#[test]
fn three_move_scramble_is_distance_three() {
    let tables = tables();
    let cube = apply_all("R U F");
    let solution = solve_optimal(&cube, tables, 20).unwrap();
    assert_eq!(solution.moves.len(), 3);
    assert_solves(&cube, &solution.moves);
    let fast = solve(&cube, tables).unwrap();
    assert_solves(&cube, &fast);
    assert!(fast.len() <= 20);
    assert_eq!(
        format_report(&fast, false).lines().last(),
        Some("optimal: false")
    );
}

#[test]
fn superflip_heuristic_is_at_least_8_and_the_generator_is_20() {
    let tables = tables();
    let maneuver = "U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2";
    assert_eq!(parse_moves(maneuver).unwrap().len(), 20);
    let superflip = apply_all(maneuver);
    assert_eq!(superflip.cp, CubieCube::solved().cp);
    assert_eq!(superflip.co, [0; 8]);
    assert!(superflip.eo.iter().all(|&flip| flip == 1));
    let h = reid_h(&superflip, tables).unwrap();
    assert!(h >= 8 && h < 20, "superflip h = {h}");
    let mut undone = superflip;
    for mv in parse_moves(maneuver).unwrap().into_iter().rev() {
        undone.apply(mv.inverse());
    }
    assert_eq!(undone, CubieCube::solved());
}

#[test]
fn superflip_below_the_heuristic_returns_immediately() {
    let tables = tables();
    let superflip = apply_all("U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2");
    let h = reid_h(&superflip, tables).unwrap() as usize;
    assert!(h >= 8);
    let start = Instant::now();
    let too_short = solve_optimal(&superflip, tables, 0).unwrap_err();
    let below = solve_optimal(&superflip, tables, h - 1).unwrap_err();
    assert!(start.elapsed().as_millis() < 500, "{:?}", start.elapsed());
    assert_eq!(too_short, OptimalError::NoSolution);
    assert_eq!(below, OptimalError::NoSolution);
}

#[test]
fn optimal_without_phase1_returns_missing_tables() {
    let dir = std::env::temp_dir().join(format!("cube-cli-nophase1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let loaded = load_or_generate(&dir).unwrap();
    assert!(loaded.tables.phase1_prun().is_none());
    let mut turned = CubieCube::solved();
    turned.apply(Move::U1);
    assert_eq!(
        solve_optimal(&turned, &loaded.tables, 20).unwrap_err(),
        OptimalError::MissingTables
    );
    assert_eq!(
        solve_optimal(&CubieCube::solved(), &loaded.tables, 20).unwrap_err(),
        OptimalError::MissingTables
    );
    assert_eq!(
        reid_h(&turned, &loaded.tables).unwrap_err(),
        OptimalError::MissingTables
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "exhaustive superflip proof searches about 3e9 nodes"]
fn superflip_optimal_length_is_20() {
    let tables = tables();
    let _ = reid_h(&CubieCube::solved(), tables).unwrap();
    let superflip = apply_all("U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2");
    let start = Instant::now();
    let solution = solve_optimal(&superflip, tables, 20).unwrap();
    let elapsed = start.elapsed();
    eprintln!(
        "superflip {} moves, {} nodes, {elapsed:.3?}",
        solution.moves.len(),
        solution.nodes
    );
    assert_eq!(solution.moves.len(), 20);
    assert_solves(&superflip, &solution.moves);
    for pair in solution.moves.windows(2) {
        assert!(move_allowed(Some(pair[0]), pair[1]));
    }
    let names: Vec<_> = solution.moves.iter().map(|mv| mv.name()).collect();
    for name in &names {
        assert!(matches!(
            *name,
            "U" | "U2"
                | "U'"
                | "R"
                | "R2"
                | "R'"
                | "F"
                | "F2"
                | "F'"
                | "D"
                | "D2"
                | "D'"
                | "L"
                | "L2"
                | "L'"
                | "B"
                | "B2"
                | "B'"
        ));
    }
}
