use std::time::{Duration, Instant};

use cube_cli::{
    cubie_from_facelets, facelets_from_cubie, format_solution, load_or_generate, parse_moves,
    solve, CubieCube, Move, SOLVED_FACELETS,
};

fn tables() -> &'static cube_cli::Tables {
    use std::sync::OnceLock;
    static LOADED: OnceLock<cube_cli::LoadedTables> = OnceLock::new();
    &LOADED
        .get_or_init(|| {
            let dir = std::env::temp_dir().join("cube-cli-solve-tables");
            load_or_generate(&dir).unwrap()
        })
        .tables
}

fn assert_solves(scrambled: &CubieCube, solution: &[Move]) {
    let mut cube = *scrambled;
    for mv in solution {
        cube.apply(*mv);
    }
    assert_eq!(cube, CubieCube::solved());
}

#[test]
fn solved_cube_is_already_done() {
    let cube = cubie_from_facelets(SOLVED_FACELETS).unwrap();
    let solution = solve(&cube, tables()).unwrap();
    assert!(solution.is_empty());
    assert_eq!(format_solution(&solution), "0");
    assert_solves(&cube, &solution);
}

#[test]
fn single_u_is_undone_by_u_prime() {
    let mut cube = CubieCube::solved();
    cube.apply(Move::U1);
    let solution = solve(&cube, tables()).unwrap();
    assert_eq!(
        solution.iter().map(|mv| mv.name()).collect::<Vec<_>>(),
        vec!["U'"]
    );
    assert_solves(&cube, &solution);

    let from_facelets = cubie_from_facelets(&facelets_from_cubie(&cube)).unwrap();
    assert_eq!(solve(&from_facelets, tables()).unwrap(), solution);

    let parsed = parse_moves("U").unwrap();
    let mut from_moves = CubieCube::solved();
    for mv in parsed {
        from_moves.apply(mv);
    }
    assert_eq!(solve(&from_moves, tables()).unwrap(), solution);
}

#[test]
fn twenty_move_scramble_solves_within_a_second() {
    let _ = tables();
    let maneuver = parse_moves("R U F D L B R' U' F' D' L' B' U2 R2 F2 D2 L2 B2 U R").unwrap();
    assert_eq!(maneuver.len(), 20);
    let mut scrambled = CubieCube::solved();
    for mv in &maneuver {
        scrambled.apply(*mv);
    }
    // One short solve so the mapped tables are faulted in before the clock starts.
    let _ = solve(&CubieCube::solved(), tables()).unwrap();

    let start = Instant::now();
    let solution = solve(&scrambled, tables()).unwrap();
    let elapsed = start.elapsed();
    assert!(
        solution.len() <= 20,
        "solution length {}, {solution:?}",
        solution.len()
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "warm solve took {elapsed:?} for {} moves",
        solution.len()
    );
    assert_solves(&scrambled, &solution);
}
