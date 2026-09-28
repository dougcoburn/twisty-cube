# Cube Bench

Two-phase and optimal HTM solver, plus a desktop shell that shows the cube.

The solver crate is already a library (`cube-cli`). `cargo run --bin cube-cli` is the same CLI as before. `apps/cube-tauri` is a Tauri 2 window: orbit the cube, turn faces, run two-phase, and play the solution back.

## Tables

Two-phase search reads `manifest.bin` and the ten table files next to it. It does not build them on a solve, and it does not load `phase1_prun.bin`.

```bash
cargo run --release --bin cube-cli -- gen-tables
```

That writes `./tables` (gitignored). The desktop shell looks for `manifest.bin` in this order:

1. `$CUBE_TABLES`
2. `./tables` from the current working directory
3. `tables/` beside this crate (`CARGO_MANIFEST_DIR`)
4. `tables/` beside the executable

If none of those exist, Solve returns an error that names the paths it tried and tells you to run `cube-cli gen-tables`. Optimal search is unchanged on the CLI (`--mode optimal`) and is not wired in the window yet.

## Desktop app

macOS is the target. Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (Xcode command line tools), then:

```bash
cd apps/cube-tauri
npm install
cargo tauri dev
```

The window talks to the solver with `get_solved`, `apply_moves`, `validate_facelets`, `scramble`, and `solve_twophase`. The solve command runs on a blocking thread so the webview stays responsive.

## What to click

1. Reset shows six solid faces. Drag to orbit. The cube does not spin on its own.
2. U (button or the U key) turns the white face clockwise. Shift+U is U'. Press 2, then a face, for a half turn.
3. Shuffle 25, Solve, Play. The cube should end solved. The status line shows `length`, `optimal false`, and elapsed milliseconds.
4. Paste a move string and Apply moves, then Solve and Play.
5. A 53-character facelet string, or any illegal coloring, shows an error and leaves the window up.

Playback keeps the facelets from when you pressed Solve. Step back animates the inverse. The slider jumps to that prefix without replaying the earlier moves.

## Tests

```bash
cargo test
```

`solve_twophase` on a solved cube is empty, and applying a two-phase solution to a 5-move scramble returns to solved. The exhaustive superflip proof stays `#[ignore]`.
