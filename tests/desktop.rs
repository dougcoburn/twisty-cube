use cube_cli::{
    apply_move, is_solved, load_or_generate, random_scramble, solve_twophase, solved, Move,
};

fn tables_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("cube-cli-solve-tables");
    load_or_generate(&dir).unwrap();
    dir
}

#[test]
fn solve_twophase_of_solved_is_empty_and_a_five_move_scramble_returns() {
    let dir = tables_dir();
    let start = solved();
    let done = solve_twophase(&start, &dir).unwrap();
    assert!(done.moves.is_empty(), "{:?}", done.moves);
    assert!(!done.optimal);
    assert!(is_solved(&start));

    let scramble = [Move::R1, Move::U1, Move::R3, Move::U3, Move::F1];
    let mut cube = start;
    for mv in scramble {
        cube = apply_move(cube, mv);
    }
    assert!(!is_solved(&cube));
    let solution = solve_twophase(&cube, &dir).unwrap();
    assert!(!solution.moves.is_empty());
    assert!(solution.moves.len() <= 20);
    for mv in solution.moves {
        cube = apply_move(cube, mv);
    }
    assert!(is_solved(&cube));
}

#[test]
fn random_scramble_is_canonical_and_length_n() {
    let moves = random_scramble(25);
    assert_eq!(moves.len(), 25);
    for pair in moves.windows(2) {
        assert!(cube_cli::move_allowed(Some(pair[0]), pair[1]));
    }
    assert!(random_scramble(0).is_empty());
}

#[test]
fn missing_tables_are_an_error_without_building_them() {
    let dir = std::env::temp_dir().join(format!(
        "cube-cli-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let err = solve_twophase(&solved(), &dir).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("gen-tables"), "{text}");
    assert!(!dir.join("manifest.bin").exists());
}

#[test]
fn desktop_dirs_check_resources_then_support_then_dev_paths() {
    let resource = std::env::temp_dir().join(format!(
        "faceturn-resource-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let dirs = cube_cli::desktop_table_dirs(&[resource.clone()]);
    let resource_at = dirs.iter().position(|dir| dir == &resource).unwrap();
    let cwd_at = dirs
        .iter()
        .position(|dir| dir == std::path::Path::new("tables"))
        .unwrap();
    let support = cube_cli::app_support_tables_dir().expect("home");
    let support_at = dirs.iter().position(|dir| dir == &support).unwrap();
    assert!(resource_at < support_at);
    assert!(support_at < cwd_at);
    let rendered = support.to_string_lossy();
    assert!(
        rendered.ends_with("com.dougcoburn.faceturn/tables"),
        "{rendered}"
    );
    assert!(!rendered.contains(".app"), "{rendered}");
}

#[test]
fn ensure_desktop_tables_reads_a_resource_dir_without_writing() {
    let dir = std::env::temp_dir().join(format!(
        "faceturn-bundle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("manifest.bin"), b"stub").unwrap();
    let before = std::fs::read(dir.join("manifest.bin")).unwrap();
    let found = cube_cli::ensure_desktop_tables(&[dir.clone()]).unwrap();
    assert_eq!(std::fs::read(dir.join("manifest.bin")).unwrap(), before);
    if std::env::var_os("CUBE_TABLES").is_none() {
        assert_eq!(found, dir);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
